use crossterm::event::KeyEvent;

use crate::{
    execution::{ExecutionReport, ExecutionTargetStatus},
    inventory::InventoryReport,
    model::{CleanupPlan, ScanHealth, TargetId},
    scan::ScanPhase,
};

pub(in crate::tui) type JobId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum UiEvent {
    Key(KeyEvent),
    Worker(WorkerEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum Effect {
    StartScan {
        job_id: JobId,
    },
    StartInventory {
        job_id: JobId,
    },
    StartClean {
        job_id: JobId,
        plan: CleanupPlan,
        selected: Vec<TargetId>,
        plan_digest: String,
    },
    CancelJob {
        job_id: JobId,
    },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum WorkerEvent {
    ScanStarted {
        job_id: JobId,
    },
    InventoryStarted {
        job_id: JobId,
    },
    JobProgress {
        job_id: JobId,
        message: String,
    },
    ScanProgress {
        job_id: JobId,
        phase: ScanPhase,
        message: String,
        plan: Option<CleanupPlan>,
    },
    ScanFinished {
        job_id: JobId,
        plan: CleanupPlan,
        health: ScanHealth,
    },
    InventoryFinished {
        job_id: JobId,
        report: Box<InventoryReport>,
    },
    CleanProgress {
        job_id: JobId,
        target_id: TargetId,
        status: ExecutionTargetStatus,
        message: String,
        detail: String,
        completed: usize,
        total: usize,
    },
    CleanFinished {
        job_id: JobId,
        report: ExecutionReport,
    },
    JobFailed {
        job_id: JobId,
        message: String,
    },
    #[allow(dead_code)] // Retained for the true-cancellation worker handoff.
    JobCanceled {
        job_id: JobId,
    },
}
