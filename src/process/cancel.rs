use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Observes cooperative cancellation without owning the caller's token type.
pub trait CancelObserver: Send + Sync {
    fn is_cancel_requested(&self) -> bool;
}

/// Default observer that never requests cancellation.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCancelObserver;

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
    pub fn new() -> Self {
        Self {
            flag: AtomicBool::new(false),
        }
    }

    pub fn request_cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn shared(self: &Arc<Self>) -> ArcCancelObserver {
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
#[derive(Debug, Clone)]
pub struct ArcCancelObserver {
    inner: Arc<FlagCancelObserver>,
}

impl CancelObserver for ArcCancelObserver {
    fn is_cancel_requested(&self) -> bool {
        self.inner.is_cancel_requested()
    }
}
