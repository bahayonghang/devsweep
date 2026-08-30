//! Read-only bounded Analyze domain. This module never creates cleanup authority.

mod model;
mod walker;

pub use model::{
    ANALYZE_SNAPSHOT_VERSION, ANALYZE_WORKERS, AnalyzeCompleteness, AnalyzeEvidence,
    AnalyzeNodeKind, AnalyzeNodeV1, AnalyzeProgressV1, AnalyzeRootIdentity, AnalyzeRunOutcome,
    AnalyzeRunStats, AnalyzeSnapshotV1, AnalyzeWarningClass, AnalyzeWarningV1, MAX_LOGICAL_DEPTH,
    MAX_OWNED_BYTES, MAX_STORED_NODES, MAX_WARNINGS, MemoryBudget, PROGRESS_INTERVAL_MS,
    PROGRESS_NODE_BATCH, PROGRESS_QUEUE_CAP,
};
pub use walker::analyze_path;

#[cfg(test)]
pub(crate) use walker::analyze_with_fs_and_stats;
#[cfg(test)]
pub(crate) use walker::{Fake250k, MapFs, dir_entry, file_entry, reparse_entry};

#[cfg(test)]
mod tests;
