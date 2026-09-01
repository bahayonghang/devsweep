//! Compact V1 Analyze snapshot DTOs and deterministic owned-byte accounting.

use std::mem::size_of;

use serde::{Deserialize, Serialize};

/// Immutable snapshot format version.
pub const ANALYZE_SNAPSHOT_VERSION: u32 = 1;
/// Stored node ceiling. No user flag raises this cap.
pub const MAX_STORED_NODES: u32 = 250_000;
/// Stored warning ceiling. No user flag raises this cap.
pub const MAX_WARNINGS: u32 = 10_000;
/// Accounted owned-memory ceiling in bytes (256 MiB).
pub const MAX_OWNED_BYTES: u64 = 268_435_456;
/// Maximum logical directory depth from the analysis root.
pub const MAX_LOGICAL_DEPTH: u16 = 1_024;
/// Dedicated Analyze worker threads. Never uses the global Rayon pool.
pub const ANALYZE_WORKERS: usize = 2;
/// Changed-node batch size for progress.
pub const PROGRESS_NODE_BATCH: usize = 256;
/// Progress queue capacity. A newer cumulative batch replaces an older unsent batch.
pub const PROGRESS_QUEUE_CAP: usize = 4;
/// Progress time batching interval in milliseconds.
pub const PROGRESS_INTERVAL_MS: u64 = 100;

const fn vec_header_bytes() -> u64 {
    size_of::<Vec<u8>>() as u64
}

const fn string_header_bytes() -> u64 {
    size_of::<String>() as u64
}

/// Owned-byte cost of a `String` with the given capacity.
#[must_use]
pub(crate) const fn string_owned_bytes(capacity: usize) -> u64 {
    string_header_bytes().saturating_add(capacity as u64)
}

pub(crate) const fn vec_owned_bytes(element_size: usize, capacity: usize) -> u64 {
    vec_header_bytes().saturating_add((element_size as u64).saturating_mul(capacity as u64))
}

pub(crate) const fn node_record_bytes() -> u64 {
    size_of::<AnalyzeNodeV1>() as u64
}

pub(crate) const fn warning_record_bytes() -> u64 {
    size_of::<AnalyzeWarningV1>() as u64
}

/// Live accounted-memory tracker used before every Analyze allocation.
#[derive(Debug, Clone)]
pub struct MemoryBudget {
    used: u64,
    peak: u64,
    cap: u64,
}

impl MemoryBudget {
    /// Creates a budget with the frozen V1 cap.
    #[must_use]
    pub fn v1() -> Self {
        Self::with_cap(MAX_OWNED_BYTES)
    }

    #[must_use]
    pub(crate) fn with_cap(cap: u64) -> Self {
        Self {
            used: 0,
            peak: 0,
            cap,
        }
    }

    /// Currently charged owned bytes.
    #[must_use]
    pub fn used(&self) -> u64 {
        self.used
    }

    /// Peak charged owned bytes.
    #[must_use]
    pub fn peak(&self) -> u64 {
        self.peak
    }

    /// Frozen cap.
    #[must_use]
    pub fn cap(&self) -> u64 {
        self.cap
    }

    /// Charges `bytes` if the cap would not be exceeded.
    pub fn try_charge(&mut self, bytes: u64) -> bool {
        let Some(next) = self.used.checked_add(bytes) else {
            return false;
        };
        if next > self.cap {
            return false;
        }
        self.used = next;
        if next > self.peak {
            self.peak = next;
        }
        true
    }

    /// Releases previously charged owned bytes.
    pub fn refund(&mut self, bytes: u64) {
        self.used = self.used.saturating_sub(bytes);
    }
}

/// Versioned read-only analysis snapshot. It never carries cleanup actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeSnapshotV1 {
    /// Snapshot schema version.
    pub version: u32,
    /// Normalized input/root/volume identity.
    pub root: AnalyzeRootIdentity,
    /// Represented nodes in snapshot-local id order.
    pub nodes: Vec<AnalyzeNodeV1>,
    /// Stable warning records.
    pub warnings: Vec<AnalyzeWarningV1>,
    /// Walk completeness. Partial values are lower bounds.
    pub completeness: AnalyzeCompleteness,
    /// Accounted owned bytes of the retained snapshot structures.
    pub accounted_owned_bytes: u64,
}

impl AnalyzeSnapshotV1 {
    /// Recomputes snapshot-owned bytes from live capacities.
    #[must_use]
    pub fn snapshot_owned_bytes(&self) -> u64 {
        let mut total = size_of::<Self>() as u64;
        total = total.saturating_add(string_owned_bytes(self.root.input.capacity()));
        total = total.saturating_add(string_owned_bytes(self.root.normalized.capacity()));
        total = total.saturating_add(string_owned_bytes(self.root.volume.capacity()));
        total = total.saturating_add(vec_owned_bytes(
            size_of::<AnalyzeNodeV1>(),
            self.nodes.capacity(),
        ));
        for node in &self.nodes {
            total = total.saturating_add(string_owned_bytes(node.name.capacity()));
            total = total.saturating_add(vec_owned_bytes(
                size_of::<AnalyzeWarningClass>(),
                node.warnings.capacity(),
            ));
        }
        total = total.saturating_add(vec_owned_bytes(
            size_of::<AnalyzeWarningV1>(),
            self.warnings.capacity(),
        ));
        total
    }

    /// Returns the root node when the snapshot stored one.
    #[must_use]
    pub fn root_node(&self) -> Option<&AnalyzeNodeV1> {
        self.nodes.first()
    }
}

/// Exact-root identity recorded for one analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeRootIdentity {
    /// Caller-supplied input path. This is data, not a command.
    pub input: String,
    /// Normalized absolute root path.
    pub normalized: String,
    /// Volume identity for the resolved root.
    pub volume: String,
}

/// One represented filesystem node. No cleanup target or action fields exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeNodeV1 {
    /// Snapshot-local identifier. Root is `0`.
    pub id: u32,
    /// Parent snapshot-local id. `null` for the root.
    pub parent_id: Option<u32>,
    /// Represented kind. Reparse points are leaves.
    pub kind: AnalyzeNodeKind,
    /// Final path component. Full paths stay on the root identity.
    pub name: String,
    /// Lower-bound owned bytes for this node and represented descendants.
    pub bytes: u64,
    /// Represented immediate children.
    pub immediate_count: u32,
    /// Represented descendants, excluding self.
    pub recursive_count: u32,
    /// Evidence state for this node.
    pub evidence: AnalyzeEvidence,
    /// Warning classes attached to this node.
    pub warnings: Vec<AnalyzeWarningClass>,
    /// Optional last-modified time in Unix milliseconds.
    pub mtime_ms: Option<u64>,
}

/// Distinguishes files, directories, and unfollowed reparse points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeNodeKind {
    /// Regular file.
    File,
    /// Directory that may have children.
    Directory,
    /// Reparse point stored as a leaf.
    Reparse,
}

/// Evidence state for one node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeEvidence {
    /// All represented descendants are present and complete.
    Complete,
    /// Missing or incomplete descendants. Size is a lower bound.
    Incomplete,
    /// Size and children could not be observed.
    Unknown,
}

/// Stable warning class. These never encode cleanup authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeWarningClass {
    /// A node, warning, or memory cap stopped scheduling.
    PartialBudget,
    /// Enumeration or metadata was denied.
    AccessDenied,
    /// A non-permission I/O failure.
    IoError,
    /// A listed entry disappeared or changed while walking.
    Churn,
    /// A directory identity was seen again.
    Cycle,
    /// A reparse point was recorded as a leaf.
    Reparse,
    /// Bytes for a hard-linked file were already counted.
    DuplicateLink,
}

/// Snapshot-level warning record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeWarningV1 {
    /// Warning class.
    pub class: AnalyzeWarningClass,
    /// Node the warning attaches to, when represented.
    pub node_id: Option<u32>,
}

/// Walk completeness. Partial values remain lower bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeCompleteness {
    /// The walker finished without hitting a cap or cancel.
    Complete,
    /// Scheduling stopped at a node, warning, or memory cap.
    PartialBudget,
    /// Cooperative cancellation joined before a complete walk.
    Canceled,
}

/// Bounded progress batch emitted by the walker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeProgressV1 {
    /// Monotonic sequence starting at 1 for one operation.
    pub sequence: u64,
    /// Nodes stored when this batch was published.
    pub stored_nodes: u32,
    /// Live accounted owned bytes when this batch was published.
    pub accounted_owned_bytes: u64,
    /// At most 256 changed nodes since the previous batch.
    pub changed_nodes: Vec<AnalyzeNodeV1>,
    /// Occupancy of the capacity-4 progress queue after publish.
    pub queue_depth: u8,
}

/// Result of one Analyze job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnalyzeRunOutcome {
    /// Walk finished or hit a budget bound. Inspect `completeness`.
    Completed {
        /// Immutable snapshot.
        snapshot: AnalyzeSnapshotV1,
    },
    /// Cancel was observed and workers joined.
    Canceled {
        /// Lower-bound snapshot of represented nodes.
        snapshot: AnalyzeSnapshotV1,
    },
}

impl AnalyzeRunOutcome {
    /// Returns the contained snapshot.
    #[must_use]
    pub fn snapshot(&self) -> &AnalyzeSnapshotV1 {
        match self {
            Self::Completed { snapshot } | Self::Canceled { snapshot } => snapshot,
        }
    }
}

/// Resource observations recorded for gates and evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeRunStats {
    /// Dedicated worker threads used (1 serial fallback, otherwise 2).
    pub workers: u8,
    /// Whether the dedicated pool failed to construct.
    pub serial_fallback: bool,
    /// Peak occupancy of the capacity-4 progress queue.
    pub max_progress_queue_depth: u8,
    /// Peak live accounted owned bytes during the walk.
    pub peak_accounted_owned_bytes: u64,
    /// Cancel-request to worker-join latency in milliseconds, when canceled.
    pub cancel_to_join_ms: Option<u64>,
}

/// Allocates a string with capacity equal to `value.len()` for deterministic accounting.
pub(crate) fn exact_string(value: &str) -> String {
    let mut owned = String::with_capacity(value.len());
    owned.push_str(value);
    owned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_dto_has_no_cleanup_fields() {
        let json = serde_json::to_value(AnalyzeSnapshotV1 {
            version: ANALYZE_SNAPSHOT_VERSION,
            root: AnalyzeRootIdentity {
                input: exact_string("C:/root"),
                normalized: exact_string("C:/root"),
                volume: exact_string("vol"),
            },
            nodes: Vec::new(),
            warnings: Vec::new(),
            completeness: AnalyzeCompleteness::Complete,
            accounted_owned_bytes: 0,
        })
        .expect("snapshot serializes");
        let encoded = json.to_string();
        assert!(!encoded.contains("cleanup"));
        assert!(!encoded.contains("intent"));
        assert!(!encoded.contains("action"));
        assert_eq!(json["version"], ANALYZE_SNAPSHOT_VERSION);
    }
}
