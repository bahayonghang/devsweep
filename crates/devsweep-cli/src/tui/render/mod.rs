use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Tabs},
};

use crate::model::ScanHealth;

use super::app::{ActiveTab, App, Overlay};

mod format;
mod inventory;
mod jobs;
mod overlays;
mod targets;
mod theme;

use format::*;
use inventory::render_inventory;
use jobs::{render_jobs_logs, scan_diagnostic_lines};
use overlays::render_overlay;
use targets::{
    render_categories, render_compact_summary, render_details_panel, render_rules, render_targets,
    selected_details_lines,
};
use theme::*;

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
                Span::styled(app.shell.app_title.clone(), accent_style()),
                Span::styled(format!("  {}", app.shell.copy.workbench), muted_style()),
                Span::styled("  |  ", muted_style()),
                Span::styled(shell_navigation_label(app), panel_style()),
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
                Span::styled(app.shell.app_title.clone(), accent_style()),
                Span::styled(format!("  {}", app.shell.active_label), panel_style()),
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
            Span::styled(app.shell.app_title.clone(), accent_style()),
            Span::styled(format!("  {}", app.shell.active_label), panel_style()),
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

fn shell_navigation_label(app: &App) -> String {
    app.shell
        .navigation
        .iter()
        .map(|item| match item.accelerator {
            Some(accelerator) => format!("[{}] {}", accelerator.to_ascii_uppercase(), item.label),
            None => item.label.clone(),
        })
        .collect::<Vec<_>>()
        .join("  ")
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
    if app.language_settings.open {
        let copy = &app.shell.copy;
        let enter = if app.language_settings.pending_request.is_some() {
            footer_action("Enter", copy.saving, FooterTone::Warning)
        } else {
            footer_action("Enter", copy.save, FooterTone::Accent)
        };
        return (
            copy.settings_title,
            FooterTone::Accent,
            vec![
                enter,
                footer_action("Esc", copy.cancel, FooterTone::Neutral),
            ],
        );
    }

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
        footer_action("p", app.shell.copy.settings_action, FooterTone::Accent),
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

#[cfg(test)]
mod tests;
