//! Bounded Analyze domain. The walker and snapshot types never create cleanup
//! authority. `actions` rebuilds node paths from a retained snapshot for the
//! Explorer reveal and the confirmed `analyze.trash` Recycle Bin move.

mod actions;
mod model;
mod observe;
mod saved;
mod walker;

pub use actions::{
    ANALYZE_TRASH_RULE_ID, ANALYZE_TRASH_VERSION, AnalyzeActionError, AnalyzeTrashExecutionRequest,
    AnalyzeTrashItemV1, AnalyzeTrashPreviewV1, AnalyzeTrashRefusalCode, AnalyzeTrashRefusalV1,
    AnalyzeTrashReportV1, analyze_node_path, default_analyze_root, execute_analyze_trash,
    preview_analyze_trash, reveal_analyze_node,
};
pub use model::{
    ANALYZE_SNAPSHOT_VERSION, ANALYZE_WORKERS, AnalyzeCompleteness, AnalyzeEvidence,
    AnalyzeNodeKind, AnalyzeNodeV1, AnalyzeProgressV1, AnalyzeRootIdentity, AnalyzeRunOutcome,
    AnalyzeRunStats, AnalyzeSnapshotV1, AnalyzeWarningClass, AnalyzeWarningV1, MAX_LOGICAL_DEPTH,
    MAX_OWNED_BYTES, MAX_STORED_NODES, MAX_WARNINGS, MemoryBudget, PROGRESS_INTERVAL_MS,
    PROGRESS_NODE_BATCH, PROGRESS_QUEUE_CAP,
};
pub use observe::{
    ANALYZE_LIST_DEFAULT_LIMIT, ANALYZE_LIST_MAX_LIMIT, ANALYZE_OBSERVE_VERSION, AnalyzeClass,
    AnalyzeClassEvidence, AnalyzeExplainV1, AnalyzeListChildV1, AnalyzeListV1, AnalyzeLockOwnerV1,
    AnalyzeLockReportV1, AnalyzeObserveError, AnalyzeRootErrorCode, AnalyzeRootRecordV1,
    AnalyzeRootsV1, explain_analyze_node, list_analyze_children, list_analyze_roots,
};
pub use saved::{
    ANALYZE_TRASH_PLAN_MAX_BYTES, ANALYZE_TRASH_PLAN_VERSION, AnalyzeSavedError,
    AnalyzeSelectedBindingV1, AnalyzeTrashPlanV1, AnalyzeTrashStrictPreviewV1,
    AnalyzeTrashStrictReportV1, decode_analyze_trash_plan, execute_analyze_trash_plan,
    plan_analyze_trash, preview_analyze_trash_plan, reveal_saved_analyze_node,
};
pub use walker::analyze_path;

#[cfg(test)]
pub(crate) use walker::analyze_with_fs_and_stats;
#[cfg(test)]
pub(crate) use walker::{Fake250k, MapFs, dir_entry, file_entry, reparse_entry};

#[cfg(test)]
mod tests;
