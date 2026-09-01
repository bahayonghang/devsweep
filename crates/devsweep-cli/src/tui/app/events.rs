use crossterm::event::KeyEvent;

use crate::{
    execution::{ExecutionReport, ExecutionTargetStatus},
    i18n::Locale,
    inventory::InventoryReport,
    model::{CleanupPlan, ScanHealth, TargetId},
    scan::ScanPhase,
};
use devsweep_core::{
    analysis::{AnalyzeProgressV1, AnalyzeRunOutcome},
    optimize::{
        MaintenanceActionOutcomeV1, MaintenanceCatalogueEntryV1, MaintenanceExecutionReportV1,
        MaintenancePlanV1, MaintenancePreviewV1, OptimizeAuditRecordV1,
    },
    software::{
        SoftwareExecutionReportV1, SoftwareInventoryV1, SoftwarePreviewV1, SoftwareSelectionPlanV1,
    },
    status::{StatusEventV1, StatusSnapshotV1},
};

pub(in crate::tui) type JobId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum UiEvent {
    Key(KeyEvent),
    Worker(WorkerEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum Effect {
    StartAnalyze {
        job_id: JobId,
    },
    StartScan {
        job_id: JobId,
    },
    StartInventory {
        job_id: JobId,
    },
    StartSoftwareInventory {
        job_id: JobId,
    },
    StartSoftwarePreview {
        job_id: JobId,
        inventory: SoftwareInventoryV1,
        selected_ids: Vec<String>,
    },
    StartSoftwareUninstall {
        job_id: JobId,
        plan: SoftwareSelectionPlanV1,
        preview_digest: String,
    },
    StartSoftwareAudit {
        job_id: JobId,
    },
    StartOptimizeList {
        job_id: JobId,
    },
    StartOptimizePreview {
        job_id: JobId,
        catalogue_id: String,
    },
    StartOptimizeRun {
        job_id: JobId,
        plan: MaintenancePlanV1,
        preview_digest: String,
    },
    StartOptimizeAudit {
        job_id: JobId,
    },
    StartStatusSnapshot {
        job_id: JobId,
    },
    StartStatusLive {
        job_id: JobId,
        interval_ms: u32,
        process_limit: u32,
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
    SavePresentationLanguage {
        request_id: u64,
        locale: Locale,
    },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum WorkerEvent {
    PresentationLanguageSaved {
        request_id: u64,
        locale: Locale,
    },
    PresentationLanguageSaveFailed {
        request_id: u64,
    },
    ScanStarted {
        job_id: JobId,
    },
    InventoryStarted {
        job_id: JobId,
    },
    AnalyzeStarted {
        job_id: JobId,
    },
    AnalyzeProgress {
        job_id: JobId,
        progress: AnalyzeProgressV1,
    },
    AnalyzeFinished {
        job_id: JobId,
        outcome: AnalyzeRunOutcome,
    },
    SoftwareInventoryFinished {
        job_id: JobId,
        inventory: SoftwareInventoryV1,
    },
    SoftwarePreviewFinished {
        job_id: JobId,
        plan: SoftwareSelectionPlanV1,
        preview: SoftwarePreviewV1,
    },
    SoftwareUninstallFinished {
        job_id: JobId,
        report: SoftwareExecutionReportV1,
    },
    SoftwareAuditFinished {
        job_id: JobId,
        report: SoftwareExecutionReportV1,
    },
    OptimizeListFinished {
        job_id: JobId,
        entries: Vec<MaintenanceCatalogueEntryV1>,
    },
    OptimizePreviewFinished {
        job_id: JobId,
        plan: MaintenancePlanV1,
        preview: MaintenancePreviewV1,
    },
    OptimizeRunFinished {
        job_id: JobId,
        report: MaintenanceExecutionReportV1,
    },
    OptimizeAuditFinished {
        job_id: JobId,
        recovered: Vec<MaintenanceActionOutcomeV1>,
        records: Vec<OptimizeAuditRecordV1>,
    },
    StatusSnapshotFinished {
        job_id: JobId,
        snapshot: Box<StatusSnapshotV1>,
    },
    StatusLiveEvent {
        job_id: JobId,
        event: Box<StatusEventV1>,
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
