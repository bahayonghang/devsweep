//! Bilingual Analyze human renderer over the V1 snapshot.

use devsweep_core::analysis::{AnalyzeCompleteness, AnalyzeSnapshotV1};

use super::render;
use crate::i18n::{CatalogueError, Locale, format_binary_bytes};

#[cfg(test)]
pub(super) struct Route;

pub(crate) fn scan_snapshot(
    locale: Locale,
    snapshot: &AnalyzeSnapshotV1,
) -> Result<String, CatalogueError> {
    let count = snapshot.nodes.len().to_string();
    let bytes = format_binary_bytes(snapshot.root_node().map(|node| node.bytes).unwrap_or(0));
    let status_key = match snapshot.completeness {
        AnalyzeCompleteness::Complete => "analyze.v1.scan.complete",
        AnalyzeCompleteness::PartialBudget => "analyze.v1.scan.partial",
        AnalyzeCompleteness::Canceled => "analyze.v1.scan.canceled",
    };
    Ok([
        render(
            locale,
            "analyze.v1.scan.root",
            &[("path", snapshot.root.normalized.as_str())],
            None,
        )?,
        render(
            locale,
            status_key,
            &[("count", &count), ("bytes", &bytes)],
            None,
        )?,
    ]
    .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::analysis::{ANALYZE_SNAPSHOT_VERSION, AnalyzeRootIdentity};

    fn empty_snapshot(completeness: AnalyzeCompleteness) -> AnalyzeSnapshotV1 {
        AnalyzeSnapshotV1 {
            version: ANALYZE_SNAPSHOT_VERSION,
            root: AnalyzeRootIdentity {
                input: "C:/root".to_string(),
                normalized: "C:/root".to_string(),
                volume: "vol".to_string(),
            },
            nodes: Vec::new(),
            warnings: Vec::new(),
            completeness,
            accounted_owned_bytes: 0,
        }
    }

    #[test]
    fn bilingual_snapshots_use_owned_renderer() {
        let complete = empty_snapshot(AnalyzeCompleteness::Complete);
        let english = scan_snapshot(Locale::En, &complete).unwrap();
        let chinese = scan_snapshot(Locale::ZhCn, &complete).unwrap();
        assert!(english.contains("Analysis complete"));
        assert!(english.contains("C:/root"));
        assert!(chinese.contains("分析完成"));
        assert!(chinese.contains("C:/root"));
        let partial = scan_snapshot(
            Locale::En,
            &empty_snapshot(AnalyzeCompleteness::PartialBudget),
        )
        .unwrap();
        assert!(partial.contains("budget bound"));
        assert!(!english.contains("CleanupPlan"));
        assert!(!chinese.contains("CleanupPlan"));
    }
}
