use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use gpui::FrameRequestSource;
use gpui_util::ResultExt;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND, WAIT_OBJECT_0},
    Graphics::Dwm::{DWM_TIMING_INFO, DwmGetCompositionTimingInfo},
    System::{
        Performance::QueryPerformanceFrequency,
        Threading::{
            CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateWaitableTimerExW, INFINITE,
            SetWaitableTimer, TIMER_ALL_ACCESS, WaitForSingleObject,
        },
    },
};

static QPC_TICKS_PER_SECOND: LazyLock<u64> = LazyLock::new(|| {
    let mut frequency = 0;
    // On systems that run Windows XP or later, the function will always succeed and
    // will thus never return zero.
    unsafe { QueryPerformanceFrequency(&mut frequency).unwrap() };
    frequency as u64
});

const VSYNC_INTERVAL_THRESHOLD: Duration = Duration::from_millis(1);
const DEFAULT_VSYNC_INTERVAL: Duration = Duration::from_micros(16_666); // ~60Hz
// Relative due time is negative 100 ns units. 16 ms keeps the measured fallback
// at or under 16.7 ms without raising the system timer resolution.
const FALLBACK_WAIT_MICROS: i64 = 16_000;
const POST_WAIT_RESERVE_MAX: Duration = Duration::from_micros(1_500);

pub(crate) struct VSyncProvider {
    #[allow(dead_code)]
    interval: Duration,
    next_deadline: Option<Instant>,
    // One high-resolution timer for every queued-frame remainder. Created on
    // the first wait and closed in Drop. Idle gaps do not start it.
    timer: Option<HANDLE>,
    /// Post-wait work measured after the previous Present, clamped to 1500 µs.
    post_wait_reserve: Duration,
}

impl VSyncProvider {
    pub(crate) fn new() -> Self {
        let interval = get_dwm_interval()
            .context("Failed to get DWM interval")
            .log_err()
            .unwrap_or(DEFAULT_VSYNC_INTERVAL);
        Self {
            interval,
            next_deadline: None,
            timer: None,
            post_wait_reserve: Duration::ZERO,
        }
    }

    /// `reserve` is the clamped post-wait duration from the last Present.
    /// Values above 1500 µs are clamped. The idle path does not read it.
    pub(crate) fn set_post_wait_reserve(&mut self, reserve: Duration) {
        self.post_wait_reserve = reserve.min(POST_WAIT_RESERVE_MAX);
    }

    /// Wait before present. The deadline carries across queued frames so work
    /// after this call reduces the next remainder. Never waits after present.
    pub(crate) fn wait_before_present(&mut self, now: Instant) -> FrameRequestSource {
        let budget = Duration::from_micros(FALLBACK_WAIT_MICROS as u64);
        match self.next_deadline {
            None => {
                // Arm only. Do not wait, and do not start the timer while no
                // deadline exists yet.
                self.next_deadline = Some(now + budget);
            }
            Some(deadline) if now < deadline => {
                // Shorten the sleep by the last post-wait cost. The deadline
                // still advances by a full budget, so Present lands on it.
                let remainder = deadline.saturating_duration_since(now);
                self.wait_remainder(remainder.saturating_sub(self.post_wait_reserve));
                self.next_deadline = Some(deadline + budget);
            }
            Some(deadline) => {
                let late = now.saturating_duration_since(deadline);
                if late > budget {
                    // Idle gap after the frame-request wait blocked. Re-arm
                    // from now and do not start the timer.
                    self.next_deadline = Some(now + budget);
                } else {
                    let mut next = deadline + budget;
                    if next < now {
                        next = now + budget;
                    }
                    self.next_deadline = Some(next);
                }
            }
        }
        FrameRequestSource::LocalSchedule
    }

    fn wait_remainder(&mut self, remainder: Duration) {
        // Due time is negative 100 ns units. A zero remainder must not call
        // SetWaitableTimer. DwmFlush is not used: a successful flush measured
        // 16.8 ms median and misses the 16.7 ms budget.
        let Ok(micros) = i64::try_from(remainder.as_micros()) else {
            return;
        };
        if micros == 0 {
            return;
        }
        let due_100ns = -(micros * 10);
        let timer = if let Some(timer) = self.timer {
            timer
        } else {
            let timer = unsafe {
                CreateWaitableTimerExW(
                    None,
                    windows::core::PCWSTR(std::ptr::null()),
                    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                    TIMER_ALL_ACCESS.0,
                )
                .expect("CreateWaitableTimerExW")
            };
            self.timer = Some(timer);
            timer
        };
        unsafe {
            SetWaitableTimer(timer, &due_100ns, 0, None, None, false).expect("SetWaitableTimer");
            let status = WaitForSingleObject(timer, INFINITE);
            assert_eq!(status, WAIT_OBJECT_0, "WaitForSingleObject");
        }
    }
}

impl Drop for VSyncProvider {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            unsafe {
                CloseHandle(timer).expect("CloseHandle");
            }
        }
    }
}

fn get_dwm_interval() -> Result<Duration> {
    let mut timing_info = DWM_TIMING_INFO {
        cbSize: std::mem::size_of::<DWM_TIMING_INFO>() as u32,
        ..Default::default()
    };
    unsafe { DwmGetCompositionTimingInfo(HWND::default(), &mut timing_info) }?;
    let interval = retrieve_duration(timing_info.qpcRefreshPeriod, *QPC_TICKS_PER_SECOND);
    // Check for interval values that are impossibly low. A 29 microsecond
    // interval was seen (from a qpcRefreshPeriod of 60).
    if interval < VSYNC_INTERVAL_THRESHOLD {
        Ok(retrieve_duration(
            timing_info.rateRefresh.uiDenominator as u64,
            timing_info.rateRefresh.uiNumerator as u64,
        ))
    } else {
        Ok(interval)
    }
}

#[inline]
fn retrieve_duration(counts: u64, ticks_per_second: u64) -> Duration {
    let ticks_per_microsecond = ticks_per_second / 1_000_000;
    Duration::from_micros(counts / ticks_per_microsecond)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn fallback_wait_p95_stays_at_or_under_16_7ms() {
        let mut provider = VSyncProvider::new();
        let mut samples = Vec::with_capacity(30);
        for _ in 0..30 {
            let started = Instant::now();
            assert_eq!(
                provider.wait_before_present(Instant::now()),
                FrameRequestSource::LocalSchedule
            );
            samples.push(started.elapsed());
        }
        samples.sort();
        let n = samples.len();
        // Nearest-rank p95: ceil(0.95 * n) - 1.
        let index = (n * 95).div_ceil(100) - 1;
        let p95 = samples[index];
        assert!(
            p95 <= Duration::from_micros(16_700),
            "p95 {p95:?} exceeded 16.7 ms"
        );
    }

    #[test]
    fn carried_deadline_shortens_the_next_wait() {
        let mut provider = VSyncProvider::new();
        let t0 = Instant::now();
        let started = Instant::now();
        assert_eq!(
            provider.wait_before_present(t0),
            FrameRequestSource::LocalSchedule
        );
        let armed = started.elapsed();
        assert!(
            armed < Duration::from_micros(8_000),
            "arming the deadline waited {armed:?}"
        );

        let started = Instant::now();
        assert_eq!(
            provider.wait_before_present(t0 + Duration::from_micros(3_000)),
            FrameRequestSource::LocalSchedule
        );
        let waited = started.elapsed();
        assert!(
            waited >= Duration::from_micros(8_000),
            "carried wait {waited:?} was under 8 ms"
        );
        assert!(
            waited < Duration::from_micros(15_000),
            "carried wait {waited:?} was not under 15 ms"
        );
    }

    #[test]
    fn idle_gap_does_not_wait() {
        let mut provider = VSyncProvider::new();
        let t0 = Instant::now();
        assert_eq!(
            provider.wait_before_present(t0),
            FrameRequestSource::LocalSchedule
        );
        let started = Instant::now();
        assert_eq!(
            provider.wait_before_present(t0 + Duration::from_micros(40_000)),
            FrameRequestSource::LocalSchedule
        );
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_micros(8_000),
            "idle gap waited {elapsed:?}"
        );
    }

    #[test]
    fn post_wait_reserve_returns_sooner_than_zero_reserve() {
        let mut reserved_total = Duration::ZERO;
        let mut plain_total = Duration::ZERO;
        for _ in 0..4 {
            let mut reserved = VSyncProvider::new();
            let mut plain = VSyncProvider::new();
            let t0 = Instant::now();
            assert_eq!(
                reserved.wait_before_present(t0),
                FrameRequestSource::LocalSchedule
            );
            assert_eq!(
                plain.wait_before_present(t0),
                FrameRequestSource::LocalSchedule
            );
            reserved.set_post_wait_reserve(Duration::from_micros(1_500));

            let now = t0 + Duration::from_micros(1_000);
            let started = Instant::now();
            assert_eq!(
                plain.wait_before_present(now),
                FrameRequestSource::LocalSchedule
            );
            plain_total += started.elapsed();

            let started = Instant::now();
            assert_eq!(
                reserved.wait_before_present(now),
                FrameRequestSource::LocalSchedule
            );
            reserved_total += started.elapsed();
        }
        assert!(
            reserved_total + Duration::from_millis(2) < plain_total,
            "reserve wait {reserved_total:?} was not sooner than {plain_total:?}"
        );
    }
}
