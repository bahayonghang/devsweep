use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use std::sync::Arc;

/// Observes cooperative cancellation without owning the caller's token type.
pub trait CancelObserver: Send + Sync {
    /// Returns whether cooperative cancellation has been requested.
    fn is_cancel_requested(&self) -> bool;
}

/// Default observer that never requests cancellation.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NoopCancelObserver;

impl CancelObserver for NoopCancelObserver {
    fn is_cancel_requested(&self) -> bool {
        false
    }
}

/// Shared flag used by scan, inventory, execution, and TUI jobs.
#[derive(Debug, Default)]
pub struct FlagCancelObserver {
    flag: AtomicBool,
}

impl FlagCancelObserver {
    /// Creates a cancellation flag in the non-canceled state.
    pub fn new() -> Self {
        Self {
            flag: AtomicBool::new(false),
        }
    }

    /// Requests cooperative cancellation for all observers of this flag.
    pub fn request_cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(super) fn shared(self: &Arc<Self>) -> ArcCancelObserver {
        ArcCancelObserver {
            inner: Arc::clone(self),
        }
    }
}

impl CancelObserver for FlagCancelObserver {
    fn is_cancel_requested(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Arc-backed observer for consumers that need an owned adapter.
#[cfg(test)]
#[derive(Debug, Clone)]
pub(super) struct ArcCancelObserver {
    inner: Arc<FlagCancelObserver>,
}

#[cfg(test)]
impl CancelObserver for ArcCancelObserver {
    fn is_cancel_requested(&self) -> bool {
        self.inner.is_cancel_requested()
    }
}
