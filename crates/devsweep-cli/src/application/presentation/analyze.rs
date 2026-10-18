//! Bilingual Analyze human renderer over the V1 snapshot.

use devsweep_core::analysis::{
    AnalyzeCompleteness, AnalyzeExplainV1, AnalyzeListV1, AnalyzeRootsV1, AnalyzeSnapshotV1,
};

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

pub(crate) fn roots(locale: Locale, report: &AnalyzeRootsV1) -> Result<String, CatalogueError> {
    let count = report.roots.len().to_string();
    let failed = report
        .roots
        .iter()
        .filter(|root| root.error.is_some())
        .count()
        .to_string();
    let mut lines = vec![render(
        locale,
        "analyze.cli.roots.summary",
        &[("count", count.as_str()), ("failed", failed.as_str())],
        None,
    )?];
    for root in &report.roots {
        if let Some(error) = root.error {
            let code = serde_json::to_value(error)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "inaccessible".to_string());
            lines.push(render(
                locale,
                "analyze.cli.roots.error",
                &[("error", code.as_str()), ("path", root.input.as_str())],
                None,
            )?);
        } else {
            let volume = root.volume.as_deref().unwrap_or("unknown");
            lines.push(render(
                locale,
                "analyze.cli.roots.item",
                &[("path", root.input.as_str()), ("volume", volume)],
                None,
            )?);
        }
    }
    Ok(lines.join("\n"))
}

pub(crate) fn list_children(
    locale: Locale,
    page: &AnalyzeListV1,
) -> Result<String, CatalogueError> {
    let parent = page.parent_id.to_string();
    let count = page.total_children.to_string();
    let shown = page.children.len().to_string();
    let mut lines = vec![render(
        locale,
        "analyze.cli.list.summary",
        &[
            ("count", count.as_str()),
            ("parent", parent.as_str()),
            ("shown", shown.as_str()),
        ],
        None,
    )?];
    for child in &page.children {
        let node = child.id.to_string();
        let bytes = format_binary_bytes(child.bytes);
        lines.push(render(
            locale,
            "analyze.cli.list.item",
            &[
                ("bytes", bytes.as_str()),
                ("name", child.name.as_str()),
                ("node", node.as_str()),
            ],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn explain(
    locale: Locale,
    explanation: &AnalyzeExplainV1,
) -> Result<String, CatalogueError> {
    let node = explanation.node_id.to_string();
    let class = serde_json::to_value(explanation.class)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string());
    let supported = if explanation.lock.supported {
        "true"
    } else {
        "false"
    };
    let owners = explanation.lock.owners.len().to_string();
    Ok([
        render(
            locale,
            "analyze.cli.explain.line",
            &[("class", class.as_str()), ("node", node.as_str())],
            None,
        )?,
        render(
            locale,
            "analyze.cli.explain.lock",
            &[("count", owners.as_str()), ("supported", supported)],
            None,
        )?,
    ]
    .join("\n"))
}

pub(crate) fn reveal(locale: Locale, node_id: u32) -> Result<String, CatalogueError> {
    let node = node_id.to_string();
    render(
        locale,
        "analyze.cli.reveal.opened",
        &[("node", node.as_str())],
        None,
    )
}

pub(crate) fn trash_preview(
    locale: Locale,
    preview: &devsweep_core::analysis::AnalyzeTrashStrictPreviewV1,
) -> Result<String, CatalogueError> {
    let count = preview.items.len().to_string();
    let refused = preview.refused.len().to_string();
    let mut lines = vec![render(
        locale,
        "analyze.cli.trash.preview",
        &[
            ("digest", preview.digest.as_str()),
            ("count", count.as_str()),
            ("refused", refused.as_str()),
        ],
        None,
    )?];
    for item in &preview.items {
        let node = item.node_id.to_string();
        let bytes = format_binary_bytes(item.bytes);
        lines.push(render(
            locale,
            "analyze.cli.trash.item",
            &[("node", node.as_str()), ("bytes", bytes.as_str())],
            None,
        )?);
    }
    if !preview.collapsed_node_ids.is_empty() {
        let ids = preview
            .collapsed_node_ids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        lines.push(render(
            locale,
            "analyze.cli.trash.collapsed",
            &[("ids", ids.as_str())],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn trash_execute(
    locale: Locale,
    report: &devsweep_core::analysis::AnalyzeTrashStrictReportV1,
) -> Result<String, CatalogueError> {
    let count = report.moved_node_ids.len().to_string();
    render(
        locale,
        "analyze.cli.trash.execute",
        &[("count", count.as_str())],
        None,
    )
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
