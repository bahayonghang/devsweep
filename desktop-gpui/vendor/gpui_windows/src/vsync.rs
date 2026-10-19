use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use gpui::FrameRequestSource;
use gpui_util::ResultExt;
use windows::Win32::{
    Foundation::HWND,
    Graphics::Dwm::{DWM_TIMING_INFO, DwmGetCompositionTimingInfo},
    System::Performance::QueryPerformanceFrequency,
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
// Deadline step for idle-gap detection. Not a sleep. Present(1) paces queued frames.
const FALLBACK_WAIT_MICROS: i64 = 16_000;
const POST_WAIT_RESERVE_MAX: Duration = Duration::from_micros(1_500);

pub(crate) struct VSyncProvider {
    #[allow(dead_code)]
    interval: Duration,
    next_deadline: Option<Instant>,
    /// Clamped post-wait duration from the last Present. The wait does not read it.
    #[allow(dead_code)]
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
            post_wait_reserve: Duration::ZERO,
        }
    }

    /// `reserve` is the clamped post-wait duration from the last Present.
    /// Values above 1500 µs are clamped. `wait_before_present` does not read it.
    pub(crate) fn set_post_wait_reserve(&mut self, reserve: Duration) {
        self.post_wait_reserve = reserve.min(POST_WAIT_RESERVE_MAX);
    }

    /// Advance the queued-frame deadline. This does not sleep. `Present(1)` is
    /// the only queued-frame pacer. The 16 ms budget still moves `next_deadline`
    /// so an idle gap is detected.
    pub(crate) fn wait_before_present(&mut self, now: Instant) -> FrameRequestSource {
        let budget = Duration::from_micros(FALLBACK_WAIT_MICROS as u64);
        match self.next_deadline {
            None => {
                // Arm only. Do not wait.
                self.next_deadline = Some(now + budget);
            }
            Some(deadline) if now < deadline => {
                // Present(1) paces this frame. Advance the deadline and do not sleep.
                // Do not subtract `post_wait_reserve`.
                self.next_deadline = Some(deadline + budget);
            }
            Some(deadline) => {
                let late = now.saturating_duration_since(deadline);
                if late > budget {
                    // Idle gap after the frame-request wait blocked. Re-arm
                    // with the 16 ms budget and do not sleep.
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
    fn successive_queued_waits_each_return_under_8ms() {
        let mut provider = VSyncProvider::new();
        for _ in 0..30 {
            let started = Instant::now();
            assert_eq!(
                provider.wait_before_present(Instant::now()),
                FrameRequestSource::LocalSchedule
            );
            let elapsed = started.elapsed();
            assert!(
                elapsed < Duration::from_micros(8_000),
                "queued wait {elapsed:?} was not under 8 ms"
            );
        }
    }

    #[test]
    fn early_branch_does_not_sleep() {
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
            waited < Duration::from_micros(8_000),
            "early branch waited {waited:?}"
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
    fn queued_second_wait_returns_under_8ms() {
        let mut provider = VSyncProvider::new();
        let t0 = Instant::now();
        assert_eq!(
            provider.wait_before_present(t0),
            FrameRequestSource::LocalSchedule
        );

        let started = Instant::now();
        assert_eq!(
            provider.wait_before_present(t0 + Duration::from_micros(1_000)),
            FrameRequestSource::LocalSchedule
        );
        let waited = started.elapsed();
        assert!(
            waited < Duration::from_micros(8_000),
            "second wait {waited:?} was not under 8 ms"
        );
    }
}
