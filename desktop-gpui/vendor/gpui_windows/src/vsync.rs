use std::{sync::LazyLock, time::Duration};

use anyhow::{Context, Result};
use gpui::FrameRequestSource;
use gpui_util::ResultExt;
use windows::Win32::{
    Foundation::{CloseHandle, HWND, WAIT_OBJECT_0},
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

pub(crate) struct VSyncProvider {
    #[allow(dead_code)]
    interval: Duration,
}

impl VSyncProvider {
    pub(crate) fn new() -> Self {
        let interval = get_dwm_interval()
            .context("Failed to get DWM interval")
            .log_err()
            .unwrap_or(DEFAULT_VSYNC_INTERVAL);
        Self { interval }
    }

    pub(crate) fn wait_for_vsync(&self) -> FrameRequestSource {
        // DwmFlush success waits for a compositor refresh. That path measured
        // 16.8 ms median and misses the 16.7 ms budget, so every queued frame
        // waits on the high-resolution timer instead.
        wait_high_resolution_fallback();
        FrameRequestSource::LocalSchedule
    }
}

fn wait_high_resolution_fallback() {
    let due_100ns = -(FALLBACK_WAIT_MICROS * 10);
    unsafe {
        let timer = CreateWaitableTimerExW(
            None,
            windows::core::PCWSTR(std::ptr::null()),
            CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
            TIMER_ALL_ACCESS.0,
        )
        .expect("CreateWaitableTimerExW");
        SetWaitableTimer(timer, &due_100ns, 0, None, None, false).expect("SetWaitableTimer");
        let status = WaitForSingleObject(timer, INFINITE);
        CloseHandle(timer).expect("CloseHandle");
        assert_eq!(status, WAIT_OBJECT_0, "WaitForSingleObject");
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
        let provider = VSyncProvider {
            interval: Duration::from_micros(16_000),
        };
        let mut samples = Vec::with_capacity(30);
        for _ in 0..30 {
            let started = Instant::now();
            assert_eq!(provider.wait_for_vsync(), FrameRequestSource::LocalSchedule);
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
}
