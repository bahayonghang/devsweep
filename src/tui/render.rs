use std::path::{Path, PathBuf};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Clear, Padding, Paragraph, Tabs, Wrap},
};

use crate::{
    inventory::{CapacityObservation, InventoryClassification, InventoryReport},
    model::{
        CleanAction, CleanTarget, Ecosystem, Evidence, RiskLevel, ScanDiagnostic,
        ScanDiagnosticOutcome, ScanDiagnosticStage, ScanHealth, Scope, SizingWarningKind, TargetId,
    },
    rules::{RuleScope, risk_label, rule_catalogue, rule_row},
};

use super::app::{
    ActiveTab, App, AppLogLevel, AppLogSource, CleanupItemStatus, CleanupProgress,
    CleanupProgressItem, CommandPreview, ConfirmState, LogEntry, Overlay, PycacheGroup, ScopeKind,
    TargetListRow,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyLayoutKind {
    Full,
    Focused,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FooterTone {
    Accent,
    Warning,
    Danger,
    Neutral,
}

const SURFACE: Color = Color::Rgb(28, 31, 44);
const SURFACE_RAISED: Color = Color::Rgb(42, 47, 62);
const FOOTER_SURFACE: Color = Color::Rgb(22, 25, 35);
const BORDER: Color = Color::Rgb(92, 101, 135);
const TEXT: Color = Color::Rgb(206, 212, 236);
const TEXT_MUTED: Color = Color::Rgb(134, 143, 177);
const TEXT_STRONG: Color = Color::Rgb(190, 198, 230);
const ACCENT: Color = Color::Rgb(112, 208, 178);
const ACCENT_SOFT: Color = Color::Rgb(190, 236, 220);
const WARNING: Color = Color::Rgb(245, 215, 132);
const DANGER: Color = Color::Rgb(239, 112, 138);
const RISK_LOW: Color = Color::Rgb(130, 198, 167);
const RISK_MEDIUM: Color = Color::Rgb(221, 185, 112);
const RISK_HIGH: Color = Color::Rgb(231, 137, 111);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FooterAction {
    key: &'static str,
    label: &'static str,
    tone: FooterTone,
}

pub(super) fn render_app(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_tabs(frame, chunks[1], app);
    render_body(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
    render_overlay(frame, app);
}

fn panel_block(title: &'static str) -> Block<'static> {
    Block::bordered()
        .title(title)
        .title_style(
            Style::default()
                .fg(TEXT_STRONG)
                .add_modifier(Modifier::BOLD),
        )
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(SURFACE))
        .padding(Padding::horizontal(1))
}

fn focused_panel_block(title: &'static str) -> Block<'static> {
    panel_block(title).border_style(Style::default().fg(ACCENT))
}

fn panel_style() -> Style {
    Style::default().fg(TEXT).bg(SURFACE)
}

fn muted_style() -> Style {
    Style::default().fg(TEXT_MUTED)
}

fn accent_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

fn warning_style() -> Style {
    Style::default().fg(WARNING).add_modifier(Modifier::BOLD)
}

fn error_style() -> Style {
    Style::default()
        .fg(DANGER)
        .bg(SURFACE)
        .add_modifier(Modifier::BOLD)
}

fn selected_row_style() -> Style {
    Style::default().bg(SURFACE_RAISED)
}

fn app_log_level_style(level: AppLogLevel) -> Style {
    match level {
        AppLogLevel::Info => muted_style(),
        AppLogLevel::Warning => warning_style(),
        AppLogLevel::Error => error_style(),
    }
}

fn risk_style(risk: &RiskLevel) -> Style {
    match risk {
        RiskLevel::Low => Style::default().fg(RISK_LOW),
        RiskLevel::Medium => Style::default().fg(RISK_MEDIUM),
        RiskLevel::High => Style::default().fg(RISK_HIGH).add_modifier(Modifier::BOLD),
        RiskLevel::Dangerous => Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
    }
}

fn body_layout_kind(area: Rect) -> BodyLayoutKind {
    if area.width >= 100 {
        BodyLayoutKind::Full
    } else if area.width >= 80 {
        BodyLayoutKind::Focused
    } else {
        BodyLayoutKind::Compact
    }
}

fn header_layout_kind(area: Rect) -> BodyLayoutKind {
    if area.width >= 100 {
        BodyLayoutKind::Full
    } else {
        BodyLayoutKind::Focused
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let active_jobs = app.jobs.iter().filter(|job| job.status.is_active()).count();
    let text = match header_layout_kind(area) {
        BodyLayoutKind::Full => vec![
            Line::from(vec![
                Span::styled("devsweep", accent_style()),
                Span::styled("  cleanup plan cockpit", muted_style()),
                Span::styled("  |  ", muted_style()),
                Span::styled("Selected ", muted_style()),
                Span::styled(
                    format!(
                        "{} ({})",
                        app.selected_ids.len(),
                        format_bytes(app.selected_bytes())
                    ),
                    warning_style(),
                ),
                Span::styled("  Scan ", muted_style()),
                Span::styled(scan_health_label(app), scan_health_style(app)),
            ]),
            scan_totals_line(app, false),
        ],
        BodyLayoutKind::Focused => vec![
            Line::from(vec![
                Span::styled("devsweep", accent_style()),
                Span::styled("  |  ", muted_style()),
                Span::styled("Selected ", muted_style()),
                Span::styled(
                    format!(
                        "{} ({})",
                        app.selected_ids.len(),
                        format_bytes(app.selected_bytes())
                    ),
                    warning_style(),
                ),
                Span::styled("  Scan ", muted_style()),
                Span::styled(scan_health_label(app), scan_health_style(app)),
                Span::styled("  Jobs ", muted_style()),
                Span::styled(active_jobs.to_string(), panel_style()),
            ]),
            scan_totals_line(app, true),
        ],
        BodyLayoutKind::Compact => vec![Line::from(vec![
            Span::styled("devsweep", accent_style()),
            Span::styled("  |  ", muted_style()),
            Span::styled("Selected ", muted_style()),
            Span::styled(
                format!(
                    "{} ({})",
                    app.selected_ids.len(),
                    format_bytes(app.selected_bytes())
                ),
                warning_style(),
            ),
            Span::styled("  Scan ", muted_style()),
            Span::styled(scan_health_label(app), scan_health_style(app)),
            Span::styled("  Jobs ", muted_style()),
            Span::styled(active_jobs.to_string(), panel_style()),
        ])],
    };
    frame.render_widget(
        Paragraph::new(Text::from(text))
            .block(focused_panel_block("Summary"))
            .style(panel_style())
            .alignment(Alignment::Left),
        area,
    );
}

fn scan_health_label(app: &App) -> &'static str {
    health_label(&app.scan_health)
}

fn health_label(health: &ScanHealth) -> &'static str {
    if health.is_complete() {
        "complete"
    } else {
        "partial"
    }
}

fn scan_health_style(app: &App) -> Style {
    health_style(&app.scan_health)
}

fn health_style(health: &ScanHealth) -> Style {
    if health.is_complete() {
        accent_style()
    } else {
        warning_style()
    }
}

fn scan_totals_line(app: &App, compact: bool) -> Line<'static> {
    health_totals_line(&app.scan_health, compact)
}

fn health_totals_line(health: &ScanHealth, compact: bool) -> Line<'static> {
    let totals = health.totals;
    if compact {
        return Line::from(vec![
            Span::styled("Verified ", muted_style()),
            Span::styled(format_bytes(totals.verified_bytes), accent_style()),
            Span::styled("  >= ", muted_style()),
            Span::styled(
                format_bytes(totals.partial_lower_bound_bytes),
                warning_style(),
            ),
            Span::styled("  Unknown ", muted_style()),
            Span::styled(totals.unknown_target_count.to_string(), warning_style()),
            Span::styled("  Diag ", muted_style()),
            Span::styled(health.diagnostics.len().to_string(), health_style(health)),
        ]);
    }

    Line::from(vec![
        Span::styled("Verified ", muted_style()),
        Span::styled(format_bytes(totals.verified_bytes), accent_style()),
        Span::styled("  Partial lower bound >= ", muted_style()),
        Span::styled(
            format_bytes(totals.partial_lower_bound_bytes),
            warning_style(),
        ),
        Span::styled("  Unknown ", muted_style()),
        Span::styled(totals.unknown_target_count.to_string(), warning_style()),
        Span::styled("  Diagnostics ", muted_style()),
        Span::styled(health.diagnostics.len().to_string(), health_style(health)),
    ])
}

fn render_tabs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let titles = ActiveTab::ALL
        .iter()
        .map(|tab| Line::from(tab.title()))
        .collect::<Vec<_>>();
    let selected = ActiveTab::ALL
        .iter()
        .position(|tab| *tab == app.active_tab)
        .unwrap_or_default();
    let tabs = Tabs::new(titles)
        .select(selected)
        .block(panel_block("Views"))
        .style(muted_style().bg(SURFACE))
        .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, app: &App) {
    match app.active_tab {
        ActiveTab::Rules => render_rules(frame, area),
        ActiveTab::JobsLogs => render_jobs_logs(frame, area, app),
        ActiveTab::Inventory => render_inventory(frame, area, app),
        ActiveTab::Dashboard | ActiveTab::Global | ActiveTab::Projects => {
            match body_layout_kind(area) {
                BodyLayoutKind::Full => {
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(22),
                            Constraint::Percentage(45),
                            Constraint::Percentage(33),
                        ])
                        .split(area);
                    render_categories(frame, chunks[0], app);
                    render_targets(frame, chunks[1], app);
                    render_details_panel(frame, chunks[2], app);
                }
                BodyLayoutKind::Focused => {
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
                        .split(area);
                    render_targets(frame, chunks[0], app);
                    render_details_panel(frame, chunks[1], app);
                }
                BodyLayoutKind::Compact => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(5),
                            Constraint::Min(10),
                            Constraint::Length(7),
                        ])
                        .split(area);
                    render_compact_summary(frame, chunks[0], app);
                    render_targets(frame, chunks[1], app);
                    render_details_panel(frame, chunks[2], app);
                }
            }
        }
    }
}

fn render_inventory(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Some(report) = &app.inventory_report else {
        frame.render_widget(
            Paragraph::new(Line::styled("No capacity observations.", muted_style()))
                .block(focused_panel_block("Inventory"))
                .style(panel_style()),
            area,
        );
        return;
    };

    match body_layout_kind(area) {
        BodyLayoutKind::Full => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
                .split(area);
            render_inventory_observations(frame, chunks[0], app, report);
            render_inventory_details(frame, chunks[1], app, report);
        }
        BodyLayoutKind::Focused | BodyLayoutKind::Compact => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(8), Constraint::Length(9)])
                .split(area);
            render_inventory_observations(frame, chunks[0], app, report);
            render_inventory_details(frame, chunks[1], app, report);
        }
    }
}

fn render_inventory_observations(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    report: &InventoryReport,
) {
    let content_height = area.height.saturating_sub(3) as usize;
    let page_size = content_height.saturating_sub(1).max(1);
    let selected_index = app
        .inventory_selected_index
        .min(report.observations.len().saturating_sub(1));
    let mut scroll = app
        .inventory_list_scroll
        .min(report.observations.len().saturating_sub(1));
    if selected_index < scroll {
        scroll = selected_index;
    } else if selected_index >= scroll + page_size {
        scroll = selected_index + 1 - page_size;
    }
    scroll = scroll.min(report.observations.len().saturating_sub(page_size));
    let end = (scroll + page_size).min(report.observations.len());

    let mut lines = vec![Line::styled(
        "  Size          Class          Path",
        muted_style(),
    )];
    if report.observations.is_empty() {
        lines.push(Line::styled("No capacity observations.", muted_style()));
    } else {
        lines.extend(report.observations[scroll..end].iter().enumerate().map(
            |(row, observation)| {
                inventory_observation_line(observation, scroll + row == selected_index, area.width)
            },
        ));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(focused_panel_block("Capacity observations"))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn inventory_observation_line(
    observation: &CapacityObservation,
    highlighted: bool,
    available_width: u16,
) -> Line<'static> {
    let marker = if highlighted { ">" } else { " " };
    let size = inventory_size_label(observation);
    let classification = inventory_classification_label(observation.classification);
    let path_width = available_width.saturating_sub(33).max(16) as usize;
    let path = compact_text(&display_path(&observation.path), path_width);
    let style = if highlighted {
        selected_row_style()
    } else {
        panel_style()
    };
    Line::from(vec![
        Span::styled(format!("{marker} {size:>12}  "), style),
        Span::styled(format!("{classification:<13} "), muted_style()),
        Span::styled(path, style),
    ])
}

fn inventory_size_label(observation: &CapacityObservation) -> String {
    if observation.size_complete {
        format_bytes(observation.estimated_bytes)
    } else if observation.estimated_bytes == 0 {
        "unknown".to_string()
    } else {
        format!(">= {}", format_bytes(observation.estimated_bytes))
    }
}

fn inventory_classification_label(classification: InventoryClassification) -> &'static str {
    match classification {
        InventoryClassification::InventoryOnly => "inventory only",
        InventoryClassification::InspectOnly => "inspect only",
    }
}

fn render_inventory_details(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    report: &InventoryReport,
) {
    let evidence_path_width = area.width.saturating_sub(18).max(12) as usize;
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Root ", muted_style()),
            Span::styled(display_path(&report.root), panel_style()),
        ]),
        Line::from(vec![
            Span::styled("Health ", muted_style()),
            Span::styled(health_label(&report.health), health_style(&report.health)),
        ]),
        health_totals_line(&report.health, true),
    ];

    if let Some(observation) = report.observations.get(
        app.inventory_selected_index
            .min(report.observations.len().saturating_sub(1)),
    ) {
        lines.push(Line::from(""));
        lines.push(Line::styled("Observation", muted_style()));
        lines.push(Line::from(vec![
            Span::styled("Class ", muted_style()),
            Span::styled(
                inventory_classification_label(observation.classification),
                panel_style(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Size ", muted_style()),
            Span::styled(inventory_size_label(observation), panel_style()),
        ]));
        if !observation.warnings.is_empty() {
            lines.push(Line::styled(
                format!("Warnings {}", observation.warnings.len()),
                warning_style(),
            ));
            lines.extend(observation.warnings.iter().take(1).map(|warning| {
                Line::styled(
                    format!(
                        "{}: {}",
                        sizing_warning_kind_label(warning.kind),
                        sanitize_display_text(&warning.detail)
                    ),
                    warning_style(),
                )
            }));
        }
    }

    if let Some(finding) = &report.orphan_pnpm_store {
        lines.push(Line::from(""));
        lines.push(Line::styled("Inspect-only pnpm store", warning_style()));
        lines.push(Line::styled(
            compact_context_text(&display_path(&finding.candidate_path), 48),
            panel_style(),
        ));
        lines.push(Line::styled(
            format!(
                "Configured {}",
                compact_context_text(
                    &display_path(&finding.configured_store),
                    evidence_path_width
                )
            ),
            muted_style(),
        ));
        if let Some(reference) = finding.project_references.first() {
            lines.push(Line::styled(
                format!(
                    "Reference {}",
                    compact_context_text(&display_path(&reference.path), evidence_path_width)
                ),
                muted_style(),
            ));
        }
        lines.push(Line::styled(
            format!("{} reference file(s)", finding.project_references.len()),
            muted_style(),
        ));
    }

    if !report.health.diagnostics.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::styled("Diagnostics", warning_style()));
        lines.extend(
            report
                .health
                .diagnostics
                .iter()
                .rev()
                .take(1)
                .flat_map(scan_diagnostic_lines),
        );
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block("Inventory details"))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_categories(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = vec![
        Line::styled("Scope", muted_style()),
        metric_line("Global", count_scope(app, ScopeKind::Global)),
        metric_line("Projects", count_scope(app, ScopeKind::Project)),
        Line::from(""),
        Line::styled("Ecosystem", muted_style()),
        metric_line("Rust", count_ecosystem(app, Ecosystem::Rust)),
        metric_line("Node", count_ecosystem(app, Ecosystem::Node)),
        metric_line("Python", count_ecosystem(app, Ecosystem::Python)),
        metric_line("Generic", count_ecosystem(app, Ecosystem::Generic)),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block("Categories"))
            .style(panel_style()),
        area,
    );
}

fn render_compact_summary(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let active_jobs = app.jobs.iter().filter(|job| job.status.is_active()).count();
    let filter = if app.filter.is_empty() {
        "none".to_string()
    } else {
        app.filter.clone()
    };
    let risk = app.risk_filter.as_ref().map(risk_label).unwrap_or("all");
    let lines = vec![
        Line::from(vec![
            Span::styled("Selected ", muted_style()),
            Span::styled(
                format!(
                    "{} ({})",
                    app.selected_ids.len(),
                    format_bytes(app.selected_bytes())
                ),
                warning_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Filter ", muted_style()),
            Span::styled(filter, panel_style()),
            Span::styled("  Risk ", muted_style()),
            Span::styled(risk, panel_style()),
            Span::styled("  Jobs ", muted_style()),
            Span::styled(active_jobs.to_string(), panel_style()),
        ]),
        scan_totals_line(app, true),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block("Summary"))
            .style(panel_style()),
        area,
    );
}

fn metric_line(label: &'static str, count: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<10}"), panel_style()),
        Span::styled(count.to_string(), accent_style()),
    ])
}

fn render_targets(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let visible = app.visible_target_rows();
    let content_height = area.height.saturating_sub(3) as usize; // borders + header
    let page_size = content_height.max(1);
    // Render is pure: clamp reads against a local window derived from app state.
    let selected_row = app.selected_index.min(visible.len().saturating_sub(1));
    let mut scroll = app.list_scroll.min(visible.len().saturating_sub(1));
    if selected_row < scroll {
        scroll = selected_row;
    } else if selected_row >= scroll + page_size {
        scroll = selected_row + 1 - page_size;
    }
    let max_scroll = visible.len().saturating_sub(page_size);
    scroll = scroll.min(max_scroll);
    let window = if visible.is_empty() {
        Vec::new()
    } else {
        let end = (scroll + page_size).min(visible.len());
        visible[scroll..end].to_vec()
    };

    let lines = if visible.is_empty() {
        vec![Line::styled("No targets in this view.", muted_style())]
    } else {
        let mut rows = vec![target_header_row(area)];
        rows.extend(window.iter().enumerate().map(|(row, target_row)| {
            let absolute = scroll + row;
            render_target_row(target_row, app, absolute == selected_row, area)
        }));
        rows
    };

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(focused_panel_block("Targets"))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_target_row(row: &TargetListRow, app: &App, selected: bool, area: Rect) -> Line<'static> {
    match row {
        TargetListRow::Target(index) => {
            let target = &app.targets[*index];
            target_row(
                target,
                selected,
                app.selected_ids.contains(&target.id),
                app.is_cleaned(&target.id),
                area,
            )
        }
        TargetListRow::PycacheGroup(group) => pycache_group_row(group, app, selected, area),
    }
}

fn target_header_row(_area: Rect) -> Line<'static> {
    let mut spans = vec![
        Span::styled("  ", muted_style()),
        Span::styled("Sel ", muted_style()),
        Span::styled("Risk ", muted_style()),
        Span::styled("Size ", muted_style()),
    ];
    spans.push(Span::styled("Target", muted_style()));
    Line::from(spans)
}

fn target_row(
    target: &CleanTarget,
    selected: bool,
    checked: bool,
    cleaned: bool,
    area: Rect,
) -> Line<'static> {
    let cursor_style = if selected {
        accent_style()
    } else {
        muted_style()
    };
    let mark = if cleaned {
        "[-]"
    } else if checked {
        "[x]"
    } else {
        "[ ]"
    };
    let cursor = if selected { ">" } else { " " };
    let target_width = target_text_width(area);
    let identity = if cleaned {
        format!(
            "{}  cleaned; rescan to refresh",
            if area.width >= 70 {
                compact_target_identity(target)
            } else {
                target_title(target)
            }
        )
    } else if area.width >= 70 {
        compact_target_identity(target)
    } else {
        target_title(target)
    };
    let target_text = compact_text(&identity, target_width);
    let text_style = if cleaned {
        muted_style()
    } else {
        panel_style()
    };

    let spans = vec![
        Span::styled(format!("{cursor} "), cursor_style),
        Span::styled(format!("{mark} "), accent_style()),
        Span::styled(
            format!("{:<9} ", risk_label(&target.risk)),
            risk_style(&target.risk),
        ),
        Span::styled(
            format!("{:>9} ", format_target_bytes(target)),
            warning_style(),
        ),
        Span::styled(target_text, text_style),
    ];

    let mut line = Line::from(spans);
    if selected {
        line = line.style(selected_row_style());
    }
    line
}

fn pycache_group_row(group: &PycacheGroup, app: &App, selected: bool, area: Rect) -> Line<'static> {
    let targets: Vec<&CleanTarget> = group
        .target_indices
        .iter()
        .filter_map(|index| app.targets.get(*index))
        .collect();
    let executable: Vec<&CleanTarget> = targets
        .iter()
        .copied()
        .filter(|target| target.action.is_executable() && !app.is_cleaned(&target.id))
        .collect();
    let selected_count = executable
        .iter()
        .filter(|target| app.selected_ids.contains(&target.id))
        .count();
    let mark = if executable.is_empty() || selected_count == 0 {
        "[ ]"
    } else if selected_count == executable.len() {
        "[x]"
    } else {
        "[-]"
    };
    let cursor = if selected { ">" } else { " " };
    let identity = format!(
        "Python project {} | {} __pycache__ entries",
        compact_path(&group.project_root),
        targets.len()
    );
    let target_text = compact_text(&identity, target_text_width(area));
    let mut line = Line::from(vec![
        Span::styled(
            format!("{cursor} "),
            if selected {
                accent_style()
            } else {
                muted_style()
            },
        ),
        Span::styled(format!("{mark} "), accent_style()),
        Span::styled(format!("{:<9} ", "Group"), muted_style()),
        Span::styled(
            format!("{:>9} ", format!("{} rows", targets.len())),
            warning_style(),
        ),
        Span::styled(target_text, panel_style()),
    ]);
    if selected {
        line = line.style(selected_row_style());
    }
    line
}

fn short_target_identity(target: &CleanTarget) -> String {
    compact_text(&target_title(target), 32)
}

fn compact_target_identity(target: &CleanTarget) -> String {
    let scope = match &target.scope {
        Scope::Global => "Global".to_string(),
        Scope::Project { root } => format!("Project {}", compact_path(root)),
    };
    let identity = short_target_identity(target);
    format!("{scope} | {identity}")
}

fn target_text_width(area: Rect) -> usize {
    let inner_width = area.width.saturating_sub(4) as usize;
    inner_width.saturating_sub(26).max(12)
}

fn render_details_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = selected_details_lines(app);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block("Details"))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn selected_details_lines(app: &App) -> Vec<Line<'static>> {
    if let Some(target) = app.selected_target() {
        target_details_lines(target)
    } else if let Some(group) = app.selected_pycache_group() {
        pycache_group_details_lines(&group, app)
    } else {
        vec![Line::from("No target selected.")]
    }
}

fn pycache_group_details_lines(group: &PycacheGroup, app: &App) -> Vec<Line<'static>> {
    let targets: Vec<&CleanTarget> = group
        .target_indices
        .iter()
        .filter_map(|index| app.targets.get(*index))
        .collect();
    let selected = targets
        .iter()
        .filter(|target| app.selected_ids.contains(&target.id))
        .count();
    vec![
        Line::styled("__pycache__ group", accent_style()),
        detail_line("Project", display_path(&group.project_root)),
        detail_line("Targets", targets.len().to_string()),
        detail_line("Selected", selected.to_string()),
        Line::from(""),
        Line::styled("Enter or g expands this project group.", muted_style()),
        Line::styled(
            "Expand it to inspect each exact target path.",
            muted_style(),
        ),
    ]
}

fn render_rules(frame: &mut Frame<'_>, area: Rect) {
    let catalogue = rule_catalogue();
    let mut lines = Vec::new();

    lines.push(Line::styled("Project rules", muted_style()));
    for doc in catalogue
        .iter()
        .filter(|doc| doc.scope == RuleScope::Project)
    {
        lines.push(Line::from(format!("  {}", rule_row(doc))));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled("Global providers & caches", muted_style()));
    for doc in catalogue
        .iter()
        .filter(|doc| doc.scope == RuleScope::Global)
    {
        lines.push(Line::from(format!("  {}", rule_row(doc))));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block("Rules"))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_jobs_logs(frame: &mut Frame<'_>, area: Rect, app: &App) {
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

fn scan_diagnostic_lines(diagnostic: &ScanDiagnostic) -> Vec<Line<'static>> {
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

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let (mode, mode_tone, actions) = footer_actions(app);
    let visible_actions = footer_visible_actions(area.width, mode, &actions);
    let mut spans = vec![
        Span::styled(format!(" {mode} "), footer_mode_style(mode_tone)),
        Span::styled(" ", footer_bar_style()),
    ];
    for action in visible_actions {
        spans.push(Span::styled(
            format!("[{}]", action.key),
            footer_key_style(action.tone),
        ));
        spans.push(Span::styled(
            format!(" {}  ", action.label),
            footer_label_style(action.tone),
        ));
    }
    let text = Line::from(spans);
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().bg(FOOTER_SURFACE))
            .alignment(Alignment::Left),
        area,
    );
}

fn footer_visible_actions(
    width: u16,
    mode: &'static str,
    actions: &[FooterAction],
) -> Vec<FooterAction> {
    let mut visible = Vec::new();
    let mut used = mode.len() + 2 + 1;
    let limit = width as usize;

    for action in actions {
        let action_width = footer_action_width(action);
        if used + action_width > limit {
            break;
        }
        used += action_width;
        visible.push(*action);
    }

    visible
}

fn footer_action_width(action: &FooterAction) -> usize {
    action.key.len() + action.label.len() + 5
}

fn footer_actions(app: &App) -> (&'static str, FooterTone, Vec<FooterAction>) {
    if app.filter_active {
        return (
            "FILTER",
            FooterTone::Warning,
            vec![
                footer_action("Enter", "Apply", FooterTone::Warning),
                footer_action("Esc", "Close", FooterTone::Neutral),
                footer_action("Backspace", "Delete", FooterTone::Neutral),
                footer_action("Ctrl-C", "Quit", FooterTone::Danger),
            ],
        );
    }

    match &app.overlay {
        Overlay::Confirm(confirm) => {
            let enter_action = if confirm.invalidated_by_scan {
                footer_action("Enter", "Blocked", FooterTone::Warning)
            } else {
                footer_action("Enter", "Run", FooterTone::Danger)
            };
            return (
                "CONFIRM",
                FooterTone::Danger,
                vec![
                    enter_action,
                    footer_action("Esc", "Cancel", FooterTone::Neutral),
                    footer_action("Backspace", "Edit", FooterTone::Neutral),
                    footer_action("Ctrl-C", "Quit", FooterTone::Danger),
                ],
            );
        }
        Overlay::Help | Overlay::Details | Overlay::DryRun => {
            return (
                "MODAL",
                FooterTone::Accent,
                vec![
                    footer_action("Enter", "Close", FooterTone::Accent),
                    footer_action("Esc", "Close", FooterTone::Neutral),
                    footer_action("Ctrl-C", "Quit", FooterTone::Danger),
                ],
            );
        }
        Overlay::QuitConfirm => {
            return (
                "QUIT?",
                FooterTone::Warning,
                vec![
                    footer_action("w", "Wait", FooterTone::Accent),
                    footer_action("c", "Cancel+Wait", FooterTone::Danger),
                    footer_action("Esc", "Stay", FooterTone::Neutral),
                ],
            );
        }
        Overlay::None => {}
    }

    if let Some(progress) = &app.cleanup_progress {
        if progress.finished {
            return (
                "DONE",
                FooterTone::Accent,
                vec![
                    footer_action("Enter", "Close", FooterTone::Accent),
                    footer_action("Esc", "Close", FooterTone::Neutral),
                    footer_action("l", "Logs", FooterTone::Neutral),
                ],
            );
        }

        return (
            "CLEANING",
            FooterTone::Warning,
            vec![
                footer_action("x", "Request stop", FooterTone::Danger),
                footer_action("l", "Logs", FooterTone::Neutral),
                footer_action("Ctrl-C", "Quit", FooterTone::Danger),
            ],
        );
    }

    if app.active_tab == ActiveTab::Inventory {
        let mut actions = vec![
            footer_action("i", "Refresh", FooterTone::Accent),
            footer_action("Up/Down", "Browse", FooterTone::Neutral),
            footer_action("q", "Quit", FooterTone::Danger),
            footer_action("?", "Help", FooterTone::Neutral),
        ];
        if app.jobs.iter().any(|job| job.status.is_active()) {
            actions.insert(2, footer_action("x", "Request stop", FooterTone::Danger));
        }
        return ("INVENTORY", FooterTone::Accent, actions);
    }

    let mut actions = vec![
        footer_action("s", "Scan", FooterTone::Accent),
        footer_action("Space", "Select", FooterTone::Neutral),
        footer_action("c", "Clean", FooterTone::Danger),
        footer_action("q", "Quit", FooterTone::Danger),
        footer_action("/", "Filter", FooterTone::Neutral),
        footer_action("?", "Help", FooterTone::Neutral),
        footer_action("a", "All", FooterTone::Neutral),
        footer_action("d", "Dry-run", FooterTone::Neutral),
        footer_action("r", "Risk", FooterTone::Neutral),
        footer_action("g", "Group", FooterTone::Neutral),
    ];
    if app.jobs.iter().any(|job| job.status.is_active()) {
        actions.insert(3, footer_action("x", "Request stop", FooterTone::Danger));
    }

    ("NORMAL", FooterTone::Accent, actions)
}

fn footer_action(key: &'static str, label: &'static str, tone: FooterTone) -> FooterAction {
    FooterAction { key, label, tone }
}

fn footer_bar_style() -> Style {
    Style::default().fg(TEXT_STRONG).bg(FOOTER_SURFACE)
}

fn footer_tone_color(tone: FooterTone) -> Color {
    match tone {
        FooterTone::Accent => ACCENT,
        FooterTone::Warning => WARNING,
        FooterTone::Danger => DANGER,
        FooterTone::Neutral => BORDER,
    }
}

fn footer_mode_style(tone: FooterTone) -> Style {
    Style::default()
        .fg(FOOTER_SURFACE)
        .bg(footer_tone_color(tone))
        .add_modifier(Modifier::BOLD)
}

fn footer_key_style(tone: FooterTone) -> Style {
    Style::default()
        .fg(FOOTER_SURFACE)
        .bg(footer_tone_color(tone))
        .add_modifier(Modifier::BOLD)
}

fn footer_label_style(tone: FooterTone) -> Style {
    let fg = match tone {
        FooterTone::Danger => DANGER,
        FooterTone::Warning => WARNING,
        FooterTone::Accent => ACCENT_SOFT,
        FooterTone::Neutral => TEXT_STRONG,
    };
    Style::default().fg(fg).bg(FOOTER_SURFACE)
}

fn render_overlay(frame: &mut Frame<'_>, app: &App) {
    match &app.overlay {
        Overlay::None => {
            if let Some(progress) = &app.cleanup_progress {
                render_cleanup_progress(frame, progress);
            }
        }
        Overlay::Help => render_modal(
            frame,
            "Keyboard help",
            vec![
                Line::from("s scan current directory and global providers"),
                Line::from("i refreshes the read-only capacity inventory"),
                Line::from("Space toggles the selected target"),
                Line::from("a toggles all visible targets"),
                Line::from("g expands or collapses a __pycache__ project group"),
                Line::from("d opens dry-run preview"),
                Line::from("c opens cleanup confirmation"),
                Line::from("/ filters targets; r cycles risk filter"),
                Line::from("x requests a stop at the next action boundary"),
                Line::from("Esc closes overlays; q quits"),
            ],
        ),
        Overlay::Details => {
            render_modal(frame, "Target details", selected_details_lines(app));
        }
        Overlay::DryRun => render_modal(frame, "Dry-run preview", dry_run_lines(app)),
        Overlay::Confirm(confirm) => render_confirm(frame, confirm),
        Overlay::QuitConfirm => render_modal(
            frame,
            "Active job running",
            vec![
                Line::from("A scan or cleanup job is still active."),
                Line::from("w  keep waiting (do not quit yet)"),
                Line::from("c  request cancel and wait for worker confirmation"),
                Line::from("Esc stay in the app"),
                Line::from("Quit is blocked until jobs reach a terminal state."),
            ],
        ),
    }
}

fn render_confirm(frame: &mut Frame<'_>, confirm: &ConfirmState) {
    let strength = if confirm.has_irreversible_commands {
        Line::styled("Irreversible command-backed cleanup", error_style())
    } else {
        Line::styled("Trash-backed cleanup", accent_style())
    };
    let mut lines = vec![
        strength,
        Line::from(confirm.message.clone()),
        Line::from(""),
        Line::from(vec![
            Span::styled("Targets: ", muted_style()),
            Span::styled(confirm.target_count.to_string(), panel_style()),
            Span::styled("  Estimated: ", muted_style()),
            Span::styled(format_bytes(confirm.estimated_bytes), warning_style()),
        ]),
        Line::from(vec![
            Span::styled("Plan digest: ", muted_style()),
            Span::styled(confirm.plan_digest_prefix.clone(), accent_style()),
        ]),
    ];

    if confirm.invalidated_by_scan {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Scan results changed. This confirmation is disabled.",
            error_style(),
        ));
        lines.push(Line::styled(
            "Press Esc, then confirm the updated selection again.",
            warning_style(),
        ));
    }

    if !confirm.selected_targets.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::styled("Selected targets:", muted_style()));
        lines.extend(confirm.selected_targets.iter().take(8).map(|target| {
            Line::from(vec![
                Span::styled("  - ", muted_style()),
                Span::styled(compact_context_text(target, 46), panel_style()),
            ])
        }));
        if confirm.selected_targets.len() > 8 {
            lines.push(Line::styled(
                format!(
                    "  ... {} more target(s)",
                    confirm.selected_targets.len() - 8
                ),
                muted_style(),
            ));
        }
    }

    if !confirm.command_previews.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::styled("Cleanup commands:", muted_style()));
        lines.extend(confirm.command_previews.iter().take(8).map(|preview| {
            Line::from(vec![
                Span::styled("  - ", muted_style()),
                Span::styled(
                    format!("{} -> ", compact_text(&preview.target, 14)),
                    panel_style(),
                ),
                Span::styled(preview.command.clone(), accent_style()),
            ])
        }));
        if confirm.command_previews.len() > 8 {
            lines.push(Line::styled(
                format!(
                    "  ... {} more command(s)",
                    confirm.command_previews.len() - 8
                ),
                muted_style(),
            ));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Required: ", muted_style()),
        Span::styled(confirm.required_phrase.clone(), accent_style()),
    ]));
    let input = if confirm.input.is_empty() {
        "<type confirm>".to_string()
    } else {
        confirm.input.clone()
    };
    let input_style = if confirm.input.is_empty() {
        muted_style()
    } else {
        panel_style()
    };
    lines.push(Line::from(vec![
        Span::styled("Input: ", muted_style()),
        Span::styled(input, input_style),
    ]));
    if let Some(feedback) = &confirm.feedback {
        lines.push(Line::styled(feedback.clone(), error_style()));
    }
    lines.push(Line::styled(
        if confirm.invalidated_by_scan {
            "Enter is disabled until you confirm the updated selection.  Esc cancels."
        } else {
            "Enter runs after confirm matches.  Esc cancels."
        },
        muted_style(),
    ));

    render_modal(frame, "Confirm cleanup", lines);
}

fn render_cleanup_progress(frame: &mut Frame<'_>, progress: &CleanupProgress) {
    let area = centered_modal_rect(frame.area(), 84, 22, 52, 12);
    let block = focused_panel_block("Cleanup progress");
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let summary = vec![
        Line::from(format_cleanup_progress(
            progress.completed,
            progress.total,
            &progress.summary,
        )),
        Line::from(cleanup_progress_bar(progress.completed, progress.total, 32)),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(summary))
            .style(panel_style())
            .alignment(Alignment::Center),
        chunks[0],
    );

    let list_lines = cleanup_progress_item_lines(progress, chunks[1].height as usize);
    frame.render_widget(
        Paragraph::new(Text::from(list_lines))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        chunks[1],
    );

    let hint = if progress.finished {
        "Enter/Esc close"
    } else {
        "x cancel"
    };
    frame.render_widget(
        Paragraph::new(Line::styled(hint, muted_style())).alignment(Alignment::Center),
        chunks[2],
    );
}

fn cleanup_progress_item_lines(progress: &CleanupProgress, max_lines: usize) -> Vec<Line<'static>> {
    if max_lines == 0 {
        return Vec::new();
    }

    let mut lines = vec![Line::styled("Results", muted_style())];
    let available_items = max_lines.saturating_sub(1);
    if available_items == 0 {
        return lines;
    }

    let item_limit = if progress.items.len() > available_items {
        available_items.saturating_sub(1)
    } else {
        available_items
    };

    for item in progress.items.iter().take(item_limit) {
        lines.push(cleanup_progress_item_line(item));
    }

    if progress.items.len() > item_limit {
        lines.push(Line::styled(
            format!("... {} more target(s)", progress.items.len() - item_limit),
            muted_style(),
        ));
    }

    lines
}

fn cleanup_progress_item_line(item: &CleanupProgressItem) -> Line<'static> {
    let (label, style) = cleanup_item_status_display(item.status);
    let mut spans = vec![
        Span::styled(format!("{label:<7} "), style),
        Span::styled(compact_text(&item.label, 12), panel_style()),
    ];
    if let Some(detail) = &item.detail {
        spans.push(Span::styled(" - ", muted_style()));
        spans.push(Span::styled(detail.clone(), muted_style()));
    }
    Line::from(spans)
}

fn cleanup_item_status_display(status: CleanupItemStatus) -> (&'static str, Style) {
    match status {
        CleanupItemStatus::Pending => ("PENDING", muted_style()),
        CleanupItemStatus::Succeeded => ("OK", accent_style()),
        CleanupItemStatus::Failed => ("FAILED", error_style()),
        CleanupItemStatus::Skipped => ("SKIPPED", warning_style()),
    }
}

fn render_modal(frame: &mut Frame<'_>, title: &'static str, lines: Vec<Line<'static>>) {
    let desired_height = (lines.len() as u16).saturating_add(4).max(10);
    let area = centered_modal_rect(frame.area(), 80, desired_height, 52, 10);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(focused_panel_block(title))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn centered_modal_rect(
    area: Rect,
    desired_width: u16,
    desired_height: u16,
    min_width: u16,
    min_height: u16,
) -> Rect {
    let max_width = area.width.saturating_sub(4).max(1);
    let max_height = area.height.saturating_sub(4).max(1);
    let min_width = min_width.min(max_width);
    let min_height = min_height.min(max_height);
    let width = desired_width.clamp(min_width, max_width);
    let height = desired_height.clamp(min_height, max_height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn dry_run_lines(app: &App) -> Vec<Line<'static>> {
    let selected = app.selected_targets();
    if selected.is_empty() {
        return vec![Line::from("No selected targets.")];
    }

    let mut lines = vec![
        Line::from(format!(
            "{} selected target(s), {} estimated.",
            selected.len(),
            format_bytes(app.selected_bytes())
        )),
        Line::from(""),
    ];
    lines.extend(selected.into_iter().take(12).map(|target| {
        Line::from(format!(
            "{} | {} | {}",
            selected_target_summary(target),
            risk_label(&target.risk),
            action_summary(&target.action)
        ))
    }));
    lines
}

fn target_details_lines(target: &CleanTarget) -> Vec<Line<'static>> {
    let mut lines = vec![
        detail_line("ID", display_path_text(target.id.as_str())),
        detail_line("Scope", scope_label(&target.scope)),
        detail_line("Path", path_label(target.path.as_ref())),
        Line::from(vec![
            Span::styled("Risk: ", muted_style()),
            Span::styled(risk_label(&target.risk), risk_style(&target.risk)),
        ]),
        detail_line("Size", format_target_bytes(target)),
        detail_line("Reversible", target.reversible.to_string()),
        detail_line("Action", action_summary(&target.action)),
        Line::styled("Evidence:", muted_style()),
    ];
    lines.extend(target.evidence.iter().take(8).map(|evidence| {
        Line::from(vec![
            Span::styled("  - ", muted_style()),
            Span::styled(evidence_summary(evidence), panel_style()),
        ])
    }));
    if !target.sizing_warnings.is_empty() {
        lines.push(Line::styled("Sizing warnings:", warning_style()));
        lines.extend(target.sizing_warnings.iter().take(4).map(|warning| {
            Line::styled(
                format!(
                    "  - {}: {}",
                    sizing_warning_kind_label(warning.kind),
                    sanitize_display_text(&warning.detail)
                ),
                warning_style(),
            )
        }));
    }
    lines
}

fn sizing_warning_kind_label(kind: SizingWarningKind) -> &'static str {
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

fn detail_line(label: &'static str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), muted_style()),
        Span::styled(value, panel_style()),
    ])
}

fn count_scope(app: &App, scope_kind: ScopeKind) -> usize {
    app.targets
        .iter()
        .filter(|target| match scope_kind {
            ScopeKind::Global => matches!(target.scope, Scope::Global),
            ScopeKind::Project => matches!(target.scope, Scope::Project { .. }),
        })
        .count()
}

fn count_ecosystem(app: &App, ecosystem: Ecosystem) -> usize {
    app.targets
        .iter()
        .filter(|target| target.ecosystem == ecosystem)
        .count()
}

pub(super) fn target_title(target: &CleanTarget) -> String {
    target
        .path
        .as_ref()
        .map(|path| compact_path(path))
        .unwrap_or_else(|| display_path_text(target.id.as_str()))
}

fn compact_path(path: &std::path::Path) -> String {
    compact_text(&display_path(path), 48)
}

fn compact_text(text: &str, max_cells: usize) -> String {
    let cleaned = sanitize_display_text(text);
    truncate_to_width(&cleaned, max_cells)
}

fn compact_context_text(text: &str, max_cells: usize) -> String {
    let cleaned = sanitize_display_text(text);
    if display_width(&cleaned) <= max_cells {
        return cleaned;
    }

    let prefix_budget = max_cells.min(12);
    let prefix = truncate_to_width(&cleaned, prefix_budget);
    let tail_budget = max_cells.saturating_sub(display_width(&prefix) + 3);
    let tail = take_width_suffix(&cleaned, tail_budget);
    format!("{prefix}...{tail}")
}

fn display_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

fn truncate_to_width(text: &str, max_cells: usize) -> String {
    if display_width(text) <= max_cells {
        return text.to_string();
    }
    if max_cells <= 3 {
        return ".".repeat(max_cells.min(3));
    }
    let tail_budget = max_cells.saturating_sub(3);
    let tail = take_width_suffix(text, tail_budget);
    format!("...{tail}")
}

fn take_width_suffix(text: &str, max_cells: usize) -> String {
    let mut width = 0usize;
    let mut chars = Vec::new();
    for ch in text.chars().rev() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > max_cells {
            break;
        }
        width += ch_width;
        chars.push(ch);
    }
    chars.into_iter().rev().collect()
}

pub(super) fn sanitize_display_text(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            '\n' | '\r' | '\t' => ' ',
            c if c.is_control() || c == '\u{7f}' => '?',
            c => c,
        })
        .collect()
}

fn quote_argv_part(part: &str) -> String {
    let cleaned = sanitize_display_text(part);
    if cleaned.is_empty()
        || cleaned.chars().any(|ch| {
            ch.is_whitespace()
                || matches!(ch, '"' | '\'' | '\\' | '*' | '?' | '|' | '&' | ';' | '>')
        })
    {
        format!("\"{}\"", cleaned.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        cleaned
    }
}

fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Global => "Global".to_string(),
        Scope::Project { root } => format!("Project ({})", display_path(root)),
    }
}

fn path_label(path: Option<&PathBuf>) -> String {
    path.map(|path| display_path(path))
        .unwrap_or_else(|| "none".to_string())
}

pub(super) fn selected_target_summary(target: &CleanTarget) -> String {
    format!(
        "{} | {} | {}",
        scope_label(&target.scope),
        target_title(target),
        path_label(target.path.as_ref())
    )
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
    }
}

pub(super) fn action_summary(action: &CleanAction) -> String {
    match action {
        CleanAction::Command {
            program,
            args,
            irreversible,
            cwd,
        } => {
            let suffix = if *irreversible { " irreversible" } else { "" };
            format!("command{}: {}", suffix, command_preview(program, args, cwd))
        }
        CleanAction::MoveToTrash { path } => format!("trash: {}", display_path(path)),
        CleanAction::DeletePermanently { path, .. } => {
            format!("permanent delete disabled: {}", display_path(path))
        }
        CleanAction::NoopInspectOnly => "inspect only".to_string(),
    }
}

fn evidence_summary(evidence: &Evidence) -> String {
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

pub(super) fn command_previews<'a>(
    targets: impl IntoIterator<Item = &'a CleanTarget>,
) -> Vec<CommandPreview> {
    targets
        .into_iter()
        .filter_map(|target| match &target.action {
            CleanAction::Command {
                program,
                args,
                cwd,
                irreversible: _,
            } => Some(CommandPreview {
                target: target_title(target),
                command: command_preview(program, args, cwd),
            }),
            CleanAction::MoveToTrash { .. }
            | CleanAction::DeletePermanently { .. }
            | CleanAction::NoopInspectOnly => None,
        })
        .collect()
}

fn command_preview(program: &str, args: &[String], cwd: &Option<PathBuf>) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(quote_argv_part(&display_path_text(program)));
    parts.extend(
        args.iter()
            .map(|arg| quote_argv_part(&display_path_text(arg))),
    );
    let argv = parts.join(" ");
    if let Some(cwd) = cwd {
        format!("argv: {argv}  cwd: {}", display_path(cwd))
    } else {
        format!("argv: {argv}")
    }
}

pub(super) fn display_path(path: &Path) -> String {
    sanitize_display_text(&display_path_text(&path.display().to_string()))
}

fn display_path_text(text: &str) -> String {
    text.replace("\\\\?\\UNC\\", "\\\\").replace("\\\\?\\", "")
}

pub(super) fn compact_target_id(target_id: &TargetId) -> String {
    compact_text(&display_path_text(target_id.as_str()), 48)
}

pub(super) fn format_cleanup_progress(completed: usize, total: usize, message: &str) -> String {
    format!("{completed} / {total} {message}")
}

fn cleanup_progress_bar(completed: usize, total: usize, width: usize) -> String {
    let filled = width
        .saturating_mul(completed.min(total))
        .checked_div(total)
        .unwrap_or_default();
    format!(
        "[{}{}]",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled))
    )
}

fn format_target_bytes(target: &crate::model::CleanTarget) -> String {
    if !target.size_complete {
        return if target.estimated_bytes == 0 {
            "unknown".to_string()
        } else {
            format!(">= {}", format_bytes(target.estimated_bytes))
        };
    }
    format_bytes(target.estimated_bytes)
}

fn format_bytes(bytes: u64) -> String {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::KeyCode;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::execution::{ExecutionReport, ExecutionTargetStatus};
    use crate::inventory::{
        CapacityObservation, INVENTORY_REPORT_VERSION, InventoryClassification, InventoryReport,
        OrphanPnpmStoreFinding, PnpmProjectReference,
    };
    use crate::model::{
        CLEANUP_PLAN_VERSION, CleanupPlan, ScanCompleteness, ScanDiagnostic, ScanDiagnosticOutcome,
        ScanDiagnosticStage, ScanHealth, ScanProcessOutput, ScanProcessProbe, ScanProcessStatus,
        ScanTotals, TargetKind,
    };
    use crate::tui::app::{JobKind, UiEvent, WorkerEvent, cleanup_progress_for_plan};
    use crate::tui::test_support::{
        key, render_text, render_text_with_size, representative_plan, target,
    };

    #[test]
    fn representative_state_renders_with_targets_details_and_jobs() {
        let mut app = App::with_plan(representative_plan());
        let job_id = app.start_job(JobKind::Scan, "Scan fixture");
        app.update(UiEvent::Worker(WorkerEvent::JobProgress {
            job_id,
            message: "Scanning fixture".to_string(),
        }));
        app.active_tab = ActiveTab::JobsLogs;

        let backend = TestBackend::new(120, 32);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("representative app renders");
    }

    #[test]
    fn representative_state_renders_at_full_supported_size() {
        let app = App::with_plan(representative_plan());
        let rendered = render_text_with_size(&app, 100, 28);

        assert!(rendered.contains("devsweep"));
        assert!(rendered.contains("Summary"));
        assert!(rendered.contains("Categories"));
        assert!(rendered.contains("Targets"));
        assert!(rendered.contains("Details"));
        assert!(rendered.contains("Global"));
        assert!(rendered.contains("Sel Risk Size Target"));
        assert!(rendered.contains("> [x] Low"));
        assert!(rendered.contains("1.0 KiB"));
    }

    #[test]
    fn representative_state_renders_at_degraded_supported_size() {
        let app = App::with_plan(representative_plan());
        let rendered = render_text_with_size(&app, 80, 24);

        assert!(rendered.contains("Summary"));
        assert!(rendered.contains("Targets"));
        assert!(rendered.contains("Details"));
        assert!(!rendered.contains("Categories"));
    }

    #[test]
    fn pycache_group_renders_collapsed_and_expanded_target_paths() {
        let project_root = PathBuf::from("C:/workspace/python-project");
        let paths = [
            project_root.join("package_a/__pycache__"),
            project_root.join("package_b/__pycache__"),
            project_root.join("package_c/__pycache__"),
        ];
        let targets = paths
            .iter()
            .map(|path| {
                target(
                    "python.__pycache__",
                    Scope::Project {
                        root: project_root.clone(),
                    },
                    Ecosystem::Python,
                    TargetKind::TestCache,
                    Some(path.clone()),
                    128,
                    RiskLevel::Low,
                    true,
                    true,
                    CleanAction::MoveToTrash { path: path.clone() },
                )
            })
            .collect();
        let mut app = App::with_plan(CleanupPlan {
            version: CLEANUP_PLAN_VERSION,
            targets,
        });

        let collapsed = render_text(&app);
        assert!(collapsed.contains("3 __pycache__ entries"));
        assert!(!collapsed.contains("package_a/__pycache__"));

        app.update(key(KeyCode::Char('g')));
        let expanded = render_text(&app);
        assert!(expanded.contains("package_a/__pycache__"));
        assert!(expanded.contains("package_b/__pycache__"));
        assert!(expanded.contains("package_c/__pycache__"));
    }

    #[test]
    fn partial_scan_health_renders_truthful_totals_and_diagnostics() {
        let mut app = App::with_plan(representative_plan());
        let mut plan = representative_plan();
        plan.targets[1].size_complete = false;
        plan.targets[2].size_complete = false;
        let mut health = ScanHealth::new(
            ScanCompleteness::Partial,
            vec![ScanDiagnostic {
                stage: ScanDiagnosticStage::CargoMetadata,
                path: PathBuf::from("C:/workspace/project/Cargo.toml"),
                outcome: ScanDiagnosticOutcome::OutputTruncated,
                detail: "captured cargo metadata output was truncated".to_string(),
                process: Some(ScanProcessProbe {
                    status: ScanProcessStatus::InvalidOutput,
                    stdout: ScanProcessOutput {
                        truncated: true,
                        retained_bytes: 1024,
                        total_bytes: 4096,
                    },
                    stderr: ScanProcessOutput {
                        truncated: false,
                        retained_bytes: 0,
                        total_bytes: 0,
                    },
                }),
            }],
        );
        health.totals = ScanTotals::from_cleanup_plan(&plan);
        let job_id = app.start_job(JobKind::Scan, "Scan fixture");
        app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
            job_id,
            plan,
            health,
        }));
        app.active_tab = ActiveTab::JobsLogs;

        let rendered = render_text_with_size(&app, 120, 50);

        assert!(rendered.contains("Scan partial"));
        assert!(rendered.contains("Verified 1.0 KiB"));
        assert!(rendered.contains("Partial lower bound >= 2.0 KiB"));
        assert!(rendered.contains("Unknown 1"));
        assert!(rendered.contains("Scan diagnostics"));
        assert!(rendered.contains("cargo metadata output truncated"));
        assert!(rendered.contains("path C:/workspace/project/Cargo.toml"));
        assert!(rendered.contains("status InvalidOutput"));
        assert!(rendered.contains("stdout 1024/4096 truncated"));
        assert!(rendered.contains("stderr 0/0"));
        assert!(rendered.contains("detail captured cargo metadata output was truncated"));
    }

    #[test]
    fn inventory_tab_renders_read_only_observations_health_and_inspection() {
        let mut app = App::with_plan(representative_plan());
        let mut health = ScanHealth::new(
            ScanCompleteness::Partial,
            vec![ScanDiagnostic {
                stage: ScanDiagnosticStage::Sizing,
                path: PathBuf::from("C:/inventory-root/archive"),
                outcome: ScanDiagnosticOutcome::Skipped,
                detail: "size entry budget exhausted".to_string(),
                process: None,
            }],
        );
        health.totals = ScanTotals {
            verified_bytes: 4096,
            partial_lower_bound_bytes: 2048,
            unknown_target_count: 1,
        };
        let report = InventoryReport {
            version: INVENTORY_REPORT_VERSION,
            root: PathBuf::from("C:/inventory-root"),
            observations: vec![
                CapacityObservation {
                    path: PathBuf::from("C:/inventory-root/archive"),
                    classification: InventoryClassification::InventoryOnly,
                    estimated_bytes: 4096,
                    size_complete: true,
                    warnings: Vec::new(),
                },
                CapacityObservation {
                    path: PathBuf::from("C:/inventory-root/old-store"),
                    classification: InventoryClassification::InventoryOnly,
                    estimated_bytes: 2048,
                    size_complete: false,
                    warnings: vec![crate::model::SizingWarning {
                        kind: crate::model::SizingWarningKind::EntryBudgetExhausted,
                        detail: "size entry budget exhausted".to_string(),
                    }],
                },
            ],
            health,
            orphan_pnpm_store: Some(OrphanPnpmStoreFinding {
                classification: InventoryClassification::InspectOnly,
                candidate_path: PathBuf::from("C:/inventory-root/.pnpm-store"),
                configured_store: PathBuf::from("C:/Users/test/AppData/Local/pnpm/store"),
                project_references: vec![PnpmProjectReference {
                    path: PathBuf::from("C:/inventory-root/app/.npmrc"),
                    references_candidate: false,
                    references_configured_store: true,
                }],
            }),
        };
        let job_id = app.start_job(JobKind::Inventory, "Inventory fixture");
        app.update(UiEvent::Worker(WorkerEvent::InventoryFinished {
            job_id,
            report: Box::new(report),
        }));
        app.active_tab = ActiveTab::Inventory;

        let rendered = render_text_with_size(&app, 120, 50);

        assert!(rendered.contains("Capacity observations"));
        assert!(rendered.contains("inventory only"));
        assert!(rendered.contains("C:/inventory-root/archive"));
        assert!(rendered.contains(">= 2.0 KiB"));
        assert!(rendered.contains("Health partial"));
        assert!(rendered.contains("Verified 4.0 KiB"));
        assert!(rendered.contains("Inspect-only pnpm store"));
        assert!(rendered.contains("Configured"), "{rendered}");
        assert!(rendered.contains("pnpm/store"), "{rendered}");
        assert!(rendered.contains("Reference"), "{rendered}");
        assert!(rendered.contains(".npmrc"), "{rendered}");
        assert!(rendered.contains("Diagnostics"));
        assert!(rendered.contains("size entry budget exhausted"));
        assert!(!rendered.contains("node.next_cache:C:"));
    }

    #[test]
    fn rules_tab_lists_catalogue_entries() {
        let mut app = App::with_plan(representative_plan());
        app.active_tab = ActiveTab::Rules;

        let rendered = render_text_with_size(&app, 120, 50);

        assert!(rendered.contains("Project rules"), "project section header");
        assert!(rendered.contains("rust.target"), "project rule listed");
        assert!(
            rendered.contains("gradle.caches"),
            "new global cache listed"
        );
    }

    #[test]
    fn target_details_show_freshness_guard_evidence() {
        let mut target = representative_plan().targets[0].clone();
        target.evidence.push(Evidence::RuleMatched {
            rule_id: crate::scan::FRESHNESS_GUARD_RULE_ID.to_string(),
        });

        let lines = target_details_lines(&target);
        let rendered = lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("rule ranking.freshness_guard.7d"));
    }

    #[test]
    fn target_details_show_typed_sizing_warnings() {
        let mut target = representative_plan().targets[0].clone();
        target.sizing_warnings.push(crate::model::SizingWarning {
            kind: crate::model::SizingWarningKind::EntryBudgetExhausted,
            detail: "review rescan reached the entry budget".to_string(),
        });

        let rendered = target_details_lines(&target)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Sizing warnings"));
        assert!(rendered.contains("entry budget exhausted"));
        assert!(rendered.contains("review rescan reached the entry budget"));
    }

    #[test]
    fn smoke_renders_dashboard_details_confirm_and_jobs_logs_states() {
        let mut app = App::with_plan(representative_plan());

        let dashboard = render_text(&app);
        assert!(dashboard.contains("Dashboard"));
        assert!(dashboard.contains("Selected"));

        app.overlay = Overlay::Details;
        let details = render_text(&app);
        assert!(details.contains("Target details"));
        assert!(details.contains("Evidence"));

        app.overlay = Overlay::None;
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[1].id.clone());
        app.update(key(KeyCode::Char('c')));
        let confirm = render_text(&app);
        assert!(confirm.contains("Confirm cleanup"));
        assert!(confirm.contains("Irreversible command-backed cleanup"));
        assert!(confirm.contains("Cleanup commands"));
        assert!(confirm.contains("argv: npm cache clean --force"));
        assert!(confirm.contains("Plan digest:"));
        assert!(confirm.contains("Required: confirm"));
        assert!(confirm.contains("Enter runs after confirm matches"));

        app.overlay = Overlay::None;
        app.active_tab = ActiveTab::JobsLogs;
        let job_id = app.start_job(JobKind::Scan, "Scan fixture");
        app.update(UiEvent::Worker(WorkerEvent::JobProgress {
            job_id,
            message: "Scanning fixture".to_string(),
        }));
        let jobs_logs = render_text(&app);
        assert!(jobs_logs.contains("Jobs"));
        assert!(jobs_logs.contains("Logs"));
        assert!(jobs_logs.contains("Scanning fixture"));
    }

    #[test]
    fn display_hygiene_quotes_argv_and_strips_controls() {
        assert_eq!(
            super::command_preview(
                "npm",
                &["cache".into(), "clean".into(), "--force".into()],
                &None
            ),
            "argv: npm cache clean --force"
        );
        assert!(
            super::command_preview("tool", &["path with space".into()], &None)
                .contains("\"path with space\"")
        );
        assert!(!super::sanitize_display_text("a\u{1b}[31mb\u{07}c").contains('\u{1b}'));
        assert!(!super::sanitize_display_text("a\u{1b}[31mb\u{07}c").contains('\u{07}'));
        let cjk = "中文路径需要按显示宽度截断并且不能越界溢出";
        let truncated = super::compact_text(cjk, 12);
        assert!(unicode_width::UnicodeWidthStr::width(truncated.as_str()) <= 12);
    }

    #[test]
    fn cleaned_target_renders_as_tombstone() {
        let mut app = App::with_plan(representative_plan());
        let target_id = app.targets[0].id.clone();
        app.cleaned_ids.insert(target_id);
        let text = render_text(&app);
        assert!(text.contains("cleaned; rescan to refresh") || text.contains("[-]"));
    }

    #[test]
    fn invalidated_confirmation_renders_reconfirmation_state() {
        let mut app = App::with_plan(representative_plan());
        app.update(key(KeyCode::Char('c')));
        let effects = app.startup_effects();
        let [crate::tui::app::Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("startup requests a scan");
        };

        app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
            job_id: *job_id,
            phase: crate::scan::ScanPhase::Projects,
            message: "Scan state changed".to_string(),
            plan: None,
        }));

        let rendered = render_text(&app);
        assert!(rendered.contains("This confirmation is disabled."));
        assert!(rendered.contains("confirm the updated selection again"));
        assert!(rendered.contains("[Enter] Blocked"));
    }

    #[test]
    fn confirmation_and_dry_run_show_selected_targets_across_scopes() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[0].id.clone());
        app.selected_ids.insert(app.targets[1].id.clone());
        let project_scope = scope_label(&app.targets[0].scope);
        let project_path = app.targets[0]
            .path
            .as_ref()
            .expect("project target path")
            .display()
            .to_string();
        let global_path = app.targets[1]
            .path
            .as_ref()
            .expect("global target path")
            .display()
            .to_string();

        let dry_run = dry_run_lines(&app)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(dry_run.contains(&project_scope));
        assert!(dry_run.contains("Global"));
        assert!(dry_run.contains(&project_path));
        assert!(dry_run.contains(&global_path));

        app.update(key(KeyCode::Char('c')));
        let Overlay::Confirm(confirm) = &app.overlay else {
            panic!("mixed selection opens confirm");
        };
        assert_eq!(confirm.selected_targets.len(), 2);
        assert!(
            confirm
                .selected_targets
                .iter()
                .any(|target| target.contains(&project_scope))
        );
        assert!(
            confirm
                .selected_targets
                .iter()
                .any(|target| target.contains("Global"))
        );
        let rendered = render_text(&app);
        assert!(rendered.contains("Selected targets:"));
        assert!(rendered.contains("Project"));
        assert!(rendered.contains("Global"));
    }

    #[test]
    fn accepted_irreversible_confirmation_renders_initial_progress() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[1].id.clone());

        app.update(key(KeyCode::Char('c')));
        let phrase = match &app.overlay {
            Overlay::Confirm(confirm) => confirm.required_phrase.clone(),
            _ => panic!("command selection opens confirm"),
        };
        for ch in phrase.chars() {
            app.update(key(KeyCode::Char(ch)));
        }
        let effects = app.update(key(KeyCode::Enter));

        assert!(matches!(
            effects.as_slice(),
            [crate::tui::app::Effect::StartClean { .. }]
        ));
        let progress = app
            .cleanup_progress
            .as_ref()
            .expect("accepted confirmation shows progress immediately");
        assert_eq!(progress.completed, 0);
        assert_eq!(progress.total, 1);
        assert!(!progress.finished);
        let rendered = render_text(&app);
        assert!(rendered.contains("Cleanup progress"));
        assert!(rendered.contains("0 / 1 Executing selected cleanup plan"));
        assert!(rendered.contains("PENDING"));
    }

    #[test]
    fn cleanup_progress_updates_job_and_renders_modal() {
        let mut app = App::with_plan(representative_plan());
        let job_id = app.start_job(JobKind::Clean, "Clean fixture");
        let target_id = app.targets[1].id.clone();
        app.cleanup_progress = Some(CleanupProgress {
            job_id,
            completed: 0,
            total: 3,
            summary: "Executing selected cleanup plan".to_string(),
            finished: false,
            items: vec![CleanupProgressItem {
                target_id: target_id.clone(),
                label: "npm cache clean".to_string(),
                status: CleanupItemStatus::Pending,
                detail: None,
            }],
        });

        app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
            job_id,
            target_id,
            status: ExecutionTargetStatus::Succeeded,
            message: "npm cache clean".to_string(),
            detail: "completed".to_string(),
            completed: 1,
            total: 3,
        }));

        assert_eq!(app.jobs[0].progress, "1 / 3 npm cache clean");
        let rendered = render_text(&app);
        assert!(rendered.contains("Cleanup progress"));
        assert!(rendered.contains("1 / 3 npm cache clean"));
        assert!(rendered.contains("[##########----------------------]"));
        assert!(rendered.contains("OK"));

        app.active_tab = ActiveTab::JobsLogs;
        app.cleanup_progress = None;
        let jobs_logs = render_text(&app);
        assert!(jobs_logs.contains("1 / 3 npm cache clean"));
        assert!(jobs_logs.contains("INFO Clean"));
    }

    #[test]
    fn cleanup_progress_renders_mixed_target_results() {
        let mut app = App::with_plan(representative_plan());
        let job_id = app.start_job(JobKind::Clean, "Clean fixture");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: app.targets.clone(),
        };
        app.cleanup_progress = Some(cleanup_progress_for_plan(job_id, &plan));
        let success_id = app.targets[0].id.clone();
        let failed_id = app.targets[1].id.clone();
        let skipped_id = app.targets[2].id.clone();

        app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
            job_id,
            target_id: success_id,
            status: ExecutionTargetStatus::Succeeded,
            message: "node cache: completed".to_string(),
            detail: "completed".to_string(),
            completed: 1,
            total: 3,
        }));
        app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
            job_id,
            target_id: failed_id.clone(),
            status: ExecutionTargetStatus::Failed,
            message: "npm cache clean: failed".to_string(),
            detail: "Access denied".to_string(),
            completed: 2,
            total: 3,
        }));
        app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
            job_id,
            target_id: skipped_id,
            status: ExecutionTargetStatus::Skipped,
            message: "cargo home: skipped".to_string(),
            detail: "inspect-only target has no executable cleanup action".to_string(),
            completed: 3,
            total: 3,
        }));
        app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
            job_id,
            report: ExecutionReport {
                dry_run: false,
                selected: 3,
                attempted: 2,
                succeeded: 1,
                failed: 1,
                skipped: 1,
                failures: vec![crate::execution::ActionFailure {
                    target_id: failed_id,
                    message: "Access denied: file is locked".to_string(),
                }],
                audit_log: Some(PathBuf::from("audit.jsonl")),
            },
        }));

        let rendered = render_text(&app);
        assert!(rendered.contains("OK"));
        assert!(rendered.contains("FAILED"));
        assert!(rendered.contains("SKIPPED"));
        assert!(rendered.contains("Access denied"));

        app.active_tab = ActiveTab::JobsLogs;
        app.cleanup_progress = None;
        let jobs_logs = render_text(&app);
        assert!(jobs_logs.contains("ERR"));
        assert!(jobs_logs.contains("Audit"));
        assert!(jobs_logs.contains("audit.jsonl"));
    }

    #[test]
    fn footer_renders_contextual_key_actions() {
        let mut app = App::with_plan(representative_plan());

        let normal = render_text(&app);
        assert!(normal.contains("NORMAL"));
        assert!(normal.contains("[s] Scan"));
        assert!(normal.contains("[c] Clean"));

        app.update(key(KeyCode::Char('c')));
        let confirm = render_text(&app);
        assert!(confirm.contains("CONFIRM"));
        assert!(confirm.contains("[Enter] Run"));
        assert!(confirm.contains("[Esc] Cancel"));
        assert!(!confirm.contains("[s] Scan"));
        assert!(!confirm.contains("[/] Filter"));

        app.overlay = Overlay::None;
        app.cleanup_progress = Some(CleanupProgress {
            job_id: 1,
            completed: 1,
            total: 1,
            summary: "finished: 1 succeeded, 0 failed, 0 skipped".to_string(),
            finished: true,
            items: Vec::new(),
        });
        let done = render_text(&app);
        assert!(done.contains("DONE"));
        assert!(done.contains("[Enter] Close"));
        assert!(done.contains("[Esc] Close"));
    }

    #[test]
    fn narrow_footer_keeps_primary_actions_visible() {
        let app = App::with_plan(representative_plan());
        let rendered = render_text_with_size(&app, 80, 24);

        assert!(rendered.contains("NORMAL"));
        assert!(rendered.contains("[s] Scan"));
        assert!(rendered.contains("[c] Clean"));
        assert!(rendered.contains("[q] Quit"));
    }

    #[test]
    fn render_path_does_not_mutate_app_state() {
        let mut app = App::with_plan(representative_plan());
        app.overlay = Overlay::DryRun;
        let before = app.clone();
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("dry-run preview renders");

        assert_eq!(app, before);
    }

    #[test]
    fn display_path_removes_windows_verbatim_prefixes() {
        assert_eq!(
            display_path(Path::new("\\\\?\\D:\\code\\devsweep\\target")),
            "D:\\code\\devsweep\\target"
        );
        assert_eq!(
            display_path(Path::new("\\\\?\\UNC\\server\\share\\cache")),
            "\\\\server\\share\\cache"
        );
        assert_eq!(
            display_path(Path::new("D:\\code\\devsweep\\target")),
            "D:\\code\\devsweep\\target"
        );
    }

    #[test]
    fn rendered_paths_hide_windows_verbatim_prefixes() {
        let target_path = PathBuf::from("\\\\?\\D:\\code\\devsweep\\target");
        let marker_path = PathBuf::from("\\\\?\\D:\\code\\devsweep\\Cargo.toml");
        let plan = CleanupPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "rust.target",
                Scope::Project {
                    root: PathBuf::from("\\\\?\\D:\\code\\devsweep"),
                },
                Ecosystem::Rust,
                TargetKind::BuildArtifacts,
                Some(target_path.clone()),
                1024,
                RiskLevel::Low,
                true,
                false,
                CleanAction::Command {
                    program: "cargo".to_string(),
                    args: vec![
                        "clean".to_string(),
                        "--manifest-path".to_string(),
                        marker_path.display().to_string(),
                    ],
                    cwd: Some(PathBuf::from("\\\\?\\D:\\code\\devsweep")),
                    irreversible: true,
                },
            )],
        };
        let mut app = App::with_plan(plan);
        app.overlay = Overlay::Details;

        let rendered = render_text(&app);

        assert!(!rendered.contains("\\\\?\\"));
        assert!(rendered.contains("D:\\code\\devsweep\\target"));
        assert!(rendered.contains("cwd: D:\\code\\devsweep"));
    }
}
