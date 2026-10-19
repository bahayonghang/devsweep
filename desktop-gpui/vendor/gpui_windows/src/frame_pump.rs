use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::{Condvar, Mutex};
use windows::Win32::Foundation::HWND;

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
}

impl FramePump {
    pub(crate) fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
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
}
