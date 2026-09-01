//! Bilingual Clean human renderer over typed outcomes.
#![allow(dead_code)]
use devsweep_core::{
    execution::ExecutionReport,
    model::{ScanCompleteness, ScanReport},
};

use super::render;
use crate::i18n::{CatalogueError, Locale, format_binary_bytes};

#[cfg(test)]
pub(super) struct Route;

pub(super) fn scan_report(locale: Locale, report: &ScanReport) -> Result<String, CatalogueError> {
    if report.plan.targets.is_empty() {
        return render(locale, "clean.v1.scan.empty", &[], None);
    }
    let count = report.plan.targets.len().to_string();
    let mut lines = vec![render(
        locale,
        "clean.v1.scan.complete",
        &[("count", &count)],
        None,
    )?];
    if !matches!(report.health.completeness, ScanCompleteness::Complete) {
        lines.push(render(locale, "clean.v1.scan.partial", &[], None)?);
    }
    Ok(lines.join("\n"))
}

pub(super) fn preview(
    locale: Locale,
    report: &ExecutionReport,
    digest: &str,
) -> Result<String, CatalogueError> {
    let count = report.selected.to_string();
    let bytes = format_capacity(report);
    Ok([
        render(
            locale,
            "clean.v1.preview.selected",
            &[("count", &count)],
            None,
        )?,
        render(
            locale,
            "clean.v1.preview.estimated",
            &[("bytes", &bytes)],
            None,
        )?,
        render(
            locale,
            "clean.v1.preview.digest",
            &[("digest", digest)],
            None,
        )?,
    ]
    .join("\n"))
}

pub(super) fn execution(
    locale: Locale,
    report: &ExecutionReport,
) -> Result<String, CatalogueError> {
    let key = if report.failed > 0 && report.succeeded > 0 {
        "clean.v1.execute.partial"
    } else if report.failed > 0 {
        "clean.v1.execute.failed"
    } else {
        "clean.v1.execute.completed"
    };
    let mut lines = vec![render(locale, key, &[], None)?];
    if report.outcomes.iter().any(|outcome| {
        matches!(
            outcome.action,
            devsweep_core::execution::ActionKind::MoveToTrash
        ) && matches!(
            outcome.status,
            devsweep_core::execution::OutcomeStatus::Succeeded
        )
    }) {
        lines.push(render(locale, "clean.v1.trash.moved", &[], None)?);
    }
    Ok(lines.join("\n"))
}

fn format_capacity(report: &ExecutionReport) -> String {
    let totals = &report.estimated_recoverable;
    if totals.verified_bytes > 0 {
        format_binary_bytes(totals.verified_bytes)
    } else if totals.partial_lower_bound_bytes > 0 {
        format_binary_bytes(totals.partial_lower_bound_bytes)
    } else {
        "0 B".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::model::{ScanHealth, UntrustedPlan};

    #[test]
    fn empty_scan_is_truthful_in_both_locales() {
        let report = ScanReport::new(UntrustedPlan::empty(), ScanHealth::complete());
        assert!(
            scan_report(Locale::En, &report)
                .unwrap()
                .contains("no cleanup targets")
        );
        assert!(
            scan_report(Locale::ZhCn, &report)
                .unwrap()
                .contains("未发现")
        );
    }
}
