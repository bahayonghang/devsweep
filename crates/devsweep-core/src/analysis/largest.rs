//! Size-ranked logical occupancy in one retained snapshot. Never reads the filesystem.

use std::{cmp::Reverse, collections::BinaryHeap};

use serde::{Deserialize, Serialize};

use super::{
    ANALYZE_LIST_MAX_LIMIT, ANALYZE_OBSERVE_VERSION, AnalyzeCompleteness, AnalyzeEvidence,
    AnalyzeNodeKind, AnalyzeObserveError, AnalyzeSnapshotV1, actions::NodeIndex,
    saved::validate_saved_snapshot,
};

/// Include files of at least 100 MiB by default.
pub const ANALYZE_LARGEST_MIN_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeLargestFileV1 {
    pub node_id: u32,
    pub path: String,
    pub bytes: u64,
    pub evidence: AnalyzeEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeLargestV1 {
    pub version: u32,
    pub retained_files_only: bool,
    pub completeness: AnalyzeCompleteness,
    pub root_evidence: AnalyzeEvidence,
    pub matching_files: usize,
    pub min_bytes: u64,
    pub limit: u32,
    pub files: Vec<AnalyzeLargestFileV1>,
}

impl AnalyzeLargestV1 {
    /// A finished traversal can still have unreadable or missing descendants.
    pub fn scan_complete(&self) -> bool {
        self.completeness == AnalyzeCompleteness::Complete
            && self.root_evidence == AnalyzeEvidence::Complete
    }
}

/// Selects at most K files, ordered by size descending, path then id ascending.
/// Sizes come from the snapshot's logical occupancy (including hard-link attribution).
pub fn largest_analyze_files(
    snapshot: &AnalyzeSnapshotV1,
    min_bytes: u64,
    limit: u32,
) -> Result<AnalyzeLargestV1, AnalyzeObserveError> {
    if !(1..=ANALYZE_LIST_MAX_LIMIT).contains(&limit) {
        return Err(AnalyzeObserveError::InvalidLimit);
    }
    validate_saved_snapshot(snapshot).map_err(|_| AnalyzeObserveError::InvalidSnapshot)?;
    let index = NodeIndex::new(snapshot).map_err(|_| AnalyzeObserveError::InvalidSnapshot)?;
    let mut matching_files = 0;
    // The largest key is the least desirable retained row.
    let mut best: BinaryHeap<(Reverse<u64>, String, u32)> =
        BinaryHeap::with_capacity(limit as usize);
    for node in &snapshot.nodes {
        if node.kind != AnalyzeNodeKind::File
            || node.evidence == AnalyzeEvidence::Unknown
            || node.bytes < min_bytes
        {
            continue;
        }
        matching_files += 1;
        if best.len() == limit as usize
            && best
                .peek()
                .is_some_and(|(Reverse(size), _, _)| *size > node.bytes)
        {
            continue;
        }
        let path = index
            .path(node.id)
            .map_err(|_| AnalyzeObserveError::InvalidSnapshot)?;
        let key = (
            Reverse(node.bytes),
            path.to_string_lossy().into_owned(),
            node.id,
        );
        if best.len() < limit as usize {
            best.push(key);
        } else if let Some(mut worst) = best.peek_mut()
            && key < *worst
        {
            *worst = key;
        }
    }
    let files = best
        .into_sorted_vec()
        .into_iter()
        .map(|(Reverse(bytes), path, node_id)| AnalyzeLargestFileV1 {
            node_id,
            path,
            bytes,
            evidence: index.node(node_id).expect("validated index").evidence,
        })
        .collect();
    Ok(AnalyzeLargestV1 {
        version: ANALYZE_OBSERVE_VERSION,
        retained_files_only: true,
        completeness: snapshot.completeness,
        root_evidence: snapshot.nodes[0].evidence,
        matching_files,
        min_bytes,
        limit,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn largest_excludes_unknown_sizes_and_keeps_observed_zero_and_lower_bounds() {
        let root = tempfile::tempdir().unwrap();
        for name in ["empty.bin", "unknown.bin", "partial.bin"] {
            std::fs::write(root.path().join(name), []).unwrap();
        }
        let outcome = crate::analysis::analyze_path(root.path(), None, None).unwrap();
        let mut snapshot = outcome.snapshot().clone();
        for node in &mut snapshot.nodes {
            match node.name.as_str() {
                "unknown.bin" => node.evidence = AnalyzeEvidence::Unknown,
                "partial.bin" => {
                    node.evidence = AnalyzeEvidence::Incomplete;
                    node.bytes = 42;
                }
                _ => {}
            }
        }
        let ranked = largest_analyze_files(&snapshot, 0, 3).unwrap();
        assert_eq!(ranked.matching_files, 2);
        assert_eq!(ranked.files[0].evidence, AnalyzeEvidence::Incomplete);
        assert!(ranked.files[0].path.ends_with("partial.bin"));
        assert_eq!(ranked.files[1].bytes, 0);
        assert!(ranked.files[1].path.ends_with("empty.bin"));

        snapshot.nodes[1].id = snapshot.nodes[0].id;
        assert!(matches!(
            largest_analyze_files(&snapshot, 0, 3),
            Err(AnalyzeObserveError::InvalidSnapshot)
        ));
        assert!(matches!(
            largest_analyze_files(&snapshot, 0, 0),
            Err(AnalyzeObserveError::InvalidLimit)
        ));
    }
}
