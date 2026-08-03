use ratatui::text::{Line, Span};

use crate::{
    model::{CleanTarget, Evidence, SizingWarningKind},
    tui::display::{display_path, display_path_text},
};

use super::theme::{muted_style, panel_style};

pub(super) fn sizing_warning_kind_label(kind: SizingWarningKind) -> &'static str {
    match kind {
        SizingWarningKind::Canceled => "canceled",
        SizingWarningKind::EntryBudgetExhausted => "entry budget exhausted",
        SizingWarningKind::MetadataUnavailable => "metadata unavailable",
        SizingWarningKind::ReparseSafetyUnverified => "reparse safety unverified",
        SizingWarningKind::MaxDepthReached => "max depth reached",
        SizingWarningKind::DirectoryReadFailed => "directory read failed",
        SizingWarningKind::DirectoryEntryReadFailed => "directory entry read failed",
        SizingWarningKind::PathUnresolved => "path unresolved",
    }
}

pub(super) fn detail_line(label: &'static str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), muted_style()),
        Span::styled(value, panel_style()),
    ])
}

pub(super) fn evidence_summary(evidence: &Evidence) -> String {
    match evidence {
        Evidence::MarkerFile { path } => format!("marker {}", display_path(path)),
        Evidence::KnownCacheDir { source, path } => {
            format!("{source} -> {}", display_path(path))
        }
        Evidence::OfficialCommand { command } => {
            format!("official command {}", display_path_text(command))
        }
        Evidence::RuleMatched { rule_id } => format!("rule {rule_id}"),
        Evidence::UserConfigured => "user configured".to_string(),
    }
}

pub(super) fn format_target_bytes(target: &CleanTarget) -> String {
    if !target.size_complete {
        return if target.estimated_bytes == 0 {
            "unknown".to_string()
        } else {
            format!(">= {}", format_bytes(target.estimated_bytes))
        };
    }
    format_bytes(target.estimated_bytes)
}

pub(super) fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}
