use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};

use parking_lot::{Condvar, Mutex};
use windows::Win32::Foundation::HWND;

/// Measured draw work after `wait_before_present` returns. The next paced wait
/// subtracts this, and a longer sample is clamped so one spike cannot skip a frame.
const POST_WAIT_RESERVE_MAX_MICROS: u64 = 1_500;

/// `HWND` is an opaque handle stored as a raw pointer, so the windows crate does not
/// implement `Send`. The vsync thread only passes it to `RedrawWindow`.
#[derive(Clone, Copy, PartialEq, Eq)]
struct QueuedHwnd(HWND);

// SAFETY: `HWND` is an opaque Win32 handle, not a Rust reference. `SafeHwnd` in this
// crate makes the same claim so the vsync thread can call `RedrawWindow`.
unsafe impl Send for QueuedHwnd {}

/// HWND values that asked for a frame, plus the condvar the vsync thread waits on.
///
/// The queue holds each HWND at most once until it is drained. An empty drain
/// is not a frame: the thread must not call `DwmFlush` or `RedrawWindow` for it.
pub(crate) struct FramePump {
    pending: Mutex<Vec<QueuedHwnd>>,
    wake: Condvar,
    stop: AtomicBool,
    /// Instant when `wait_before_present` returned. Consumed by the next Present.
    wait_returned: Mutex<Option<Instant>>,
    /// Microseconds of post-wait work, clamped to `0..=POST_WAIT_RESERVE_MAX_MICROS`.
    reserve_micros: AtomicU64,
}

impl FramePump {
    pub(crate) fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
            wait_returned: Mutex::new(None),
            reserve_micros: AtomicU64::new(0),
        }
    }

    /// Record `hwnd` once. A second request while the first is unconsumed does not add another entry.
    pub(crate) fn enqueue(&self, hwnd: HWND) {
        let queued = QueuedHwnd(hwnd);
        let mut pending = self.pending.lock();
        if !pending.contains(&queued) {
            pending.push(queued);
        }
        self.wake.notify_one();
    }

    /// Wake the vsync thread so it can exit without waiting for a frame.
    pub(crate) fn stop(&self) {
        let _pending = self.pending.lock();
        self.stop.store(true, Ordering::Release);
        self.wake.notify_one();
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    /// HWNDs to invalidate now. Empty when nobody has asked for a frame.
    pub(crate) fn take_invalidate_list(&self) -> Vec<HWND> {
        std::mem::take(&mut *self.pending.lock())
            .into_iter()
            .map(|queued| queued.0)
            .collect()
    }

    /// Block while the queue is empty and the pump is running.
    ///
    /// Returns the HWNDs for this frame. Requests that arrive after the return
    /// stay queued for the next loop. `None` means the thread should exit, and
    /// the caller must not call `DwmFlush` or `RedrawWindow`.
    pub(crate) fn wait_for_frame_requests(&self) -> Option<Vec<HWND>> {
        {
            let mut pending = self.pending.lock();
            while pending.is_empty() && !self.stop.load(Ordering::Acquire) {
                self.wake.wait(&mut pending);
            }
            if self.stop.load(Ordering::Acquire) || pending.is_empty() {
                return None;
            }
        }
        let batch = self.take_invalidate_list();
        if batch.is_empty() { None } else { Some(batch) }
    }

    /// Called on the vsync thread when `wait_before_present` returns.
    pub(crate) fn record_wait_returned(&self, at: Instant) {
        *self.wait_returned.lock() = Some(at);
    }

    /// Called on the window thread after a successful Present.
    ///
    /// A missing wait timestamp stores 0. The duration is clamped to
    /// `0..=1500` µs.
    pub(crate) fn record_present(&self, presented_at: Instant) {
        let started = self.wait_returned.lock().take();
        let micros = match started {
            Some(started) => presented_at
                .saturating_duration_since(started)
                .as_micros()
                .min(u128::from(POST_WAIT_RESERVE_MAX_MICROS)) as u64,
            None => 0,
        };
        self.reserve_micros.store(micros, Ordering::Release);
    }

    pub(crate) fn post_wait_reserve(&self) -> Duration {
        Duration::from_micros(self.reserve_micros.load(Ordering::Acquire))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hwnd(value: usize) -> HWND {
        HWND(value as *mut std::ffi::c_void)
    }

    #[test]
    fn empty_queue_yields_nothing_and_duplicate_enqueue_yields_once() {
        let pump = FramePump::new();
        assert!(pump.take_invalidate_list().is_empty());

        let hwnd = test_hwnd(1);
        pump.enqueue(hwnd);
        pump.enqueue(hwnd);

        assert_eq!(pump.take_invalidate_list(), vec![hwnd]);
    }

    #[test]
    fn present_reserve_clamps_and_missing_timestamp_is_zero() {
        let pump = FramePump::new();
        pump.record_present(Instant::now());
        assert_eq!(pump.post_wait_reserve(), Duration::ZERO);

        let started = Instant::now();
        pump.record_wait_returned(started);
        pump.record_present(started + Duration::from_micros(5_000));
        assert_eq!(
            pump.post_wait_reserve(),
            Duration::from_micros(POST_WAIT_RESERVE_MAX_MICROS)
        );

        pump.record_wait_returned(started);
        pump.record_present(started + Duration::from_micros(400));
        assert_eq!(pump.post_wait_reserve(), Duration::from_micros(400));
    }
}
