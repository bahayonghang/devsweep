use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

use crate::{
    model::{ScanDiagnostic, ScanDiagnosticOutcome, ScanDiagnosticStage},
    tui::{
        app::{App, AppLogLevel, AppLogSource, LogEntry},
        display::{compact_target_id, display_path, sanitize_display_text},
    },
};

use super::theme::*;

pub(super) fn render_jobs_logs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = if area.width >= 100 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(6)])
            .split(area)
    };

    let job_lines = if app.jobs.is_empty() {
        vec![Line::from("No jobs yet.")]
    } else {
        app.jobs
            .iter()
            .rev()
            .take(12)
            .map(|job| {
                Line::from(format!(
                    "#{} {:?} {:?}: {} ({})",
                    job.id, job.kind, job.status, job.label, job.progress
                ))
            })
            .collect()
    };
    let mut log_lines = Vec::new();
    if !app.scan_health.diagnostics.is_empty() {
        log_lines.push(Line::styled("Scan diagnostics", warning_style()));
        log_lines.extend(
            app.scan_health
                .diagnostics
                .iter()
                .rev()
                .take(6)
                .flat_map(scan_diagnostic_lines),
        );
    }
    if !app.logs.is_empty() {
        if !log_lines.is_empty() {
            log_lines.push(Line::from(""));
        }
        log_lines.extend(app.logs.iter().rev().take(16).map(log_entry_line));
    }
    if log_lines.is_empty() {
        log_lines.push(Line::from("No logs yet."));
    }

    frame.render_widget(
        Paragraph::new(Text::from(job_lines))
            .block(panel_block("Jobs"))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(Text::from(log_lines))
            .block(panel_block("Logs"))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn log_entry_line(entry: &LogEntry) -> Line<'static> {
    let mut spans = vec![
        Span::styled(format!("#{} ", entry.seq), muted_style()),
        Span::styled(
            format!("{:<4} ", app_log_level_label(entry.level)),
            app_log_level_style(entry.level),
        ),
        Span::styled(
            format!("{:<5} ", app_log_source_label(entry.source)),
            accent_style(),
        ),
    ];
    if let Some(job_id) = entry.job_id {
        spans.push(Span::styled(format!("job:{job_id} "), muted_style()));
    }
    spans.push(Span::styled(entry.message.clone(), panel_style()));
    if let Some(target_id) = &entry.target_id {
        spans.push(Span::styled(
            format!(" target:{}", compact_target_id(target_id)),
            muted_style(),
        ));
    }
    Line::from(spans)
}

pub(super) fn scan_diagnostic_lines(diagnostic: &ScanDiagnostic) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled(
            format!(
                "{} {}",
                scan_diagnostic_stage_label(diagnostic.stage),
                scan_diagnostic_outcome_label(diagnostic.outcome),
            ),
            warning_style(),
        ),
        Line::styled(
            format!("path {}", display_path(&diagnostic.path)),
            warning_style(),
        ),
    ];
    if let Some(process) = &diagnostic.process {
        lines.push(Line::styled(
            format!("status {:?}", process.status),
            warning_style(),
        ));
        lines.push(scan_process_output_line(
            "stdout",
            process.stdout.retained_bytes,
            process.stdout.total_bytes,
            process.stdout.truncated,
        ));
        lines.push(scan_process_output_line(
            "stderr",
            process.stderr.retained_bytes,
            process.stderr.total_bytes,
            process.stderr.truncated,
        ));
    }
    lines.push(Line::styled(
        format!("detail {}", sanitize_display_text(&diagnostic.detail)),
        warning_style(),
    ));
    lines
}

fn scan_process_output_line(
    stream: &str,
    retained_bytes: u64,
    total_bytes: u64,
    truncated: bool,
) -> Line<'static> {
    Line::styled(
        format!(
            "{stream} {retained_bytes}/{total_bytes}{}",
            if truncated { " truncated" } else { "" },
        ),
        warning_style(),
    )
}

fn scan_diagnostic_stage_label(stage: ScanDiagnosticStage) -> &'static str {
    match stage {
        ScanDiagnosticStage::Discovery => "discovery",
        ScanDiagnosticStage::Sizing => "sizing",
        ScanDiagnosticStage::CargoMetadata => "cargo metadata",
        ScanDiagnosticStage::Provider => "provider",
    }
}

fn scan_diagnostic_outcome_label(outcome: ScanDiagnosticOutcome) -> &'static str {
    match outcome {
        ScanDiagnosticOutcome::Skipped => "skipped",
        ScanDiagnosticOutcome::Failed => "failed",
        ScanDiagnosticOutcome::Canceled => "canceled",
        ScanDiagnosticOutcome::OutputTruncated => "output truncated",
    }
}

fn app_log_level_label(level: AppLogLevel) -> &'static str {
    match level {
        AppLogLevel::Info => "INFO",
        AppLogLevel::Warning => "WARN",
        AppLogLevel::Error => "ERR",
    }
}

fn app_log_source_label(source: AppLogSource) -> &'static str {
    match source {
        AppLogSource::App => "App",
        AppLogSource::Scan => "Scan",
        AppLogSource::Inventory => "Invent",
        AppLogSource::Clean => "Clean",
        AppLogSource::Audit => "Audit",
        AppLogSource::Analyze => "Analyze",
        AppLogSource::Software => "Software",
        AppLogSource::Optimize => "Optimize",
        AppLogSource::Status => "Status",
    }
}
