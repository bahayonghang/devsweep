use devsweep_core::optimize::{
    MaintenanceActionClass, MaintenanceCatalogueEntryV1, MaintenanceExecutionOutcome,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    i18n::Locale,
    tui::{app::App, modes::optimize::OptimizePhase},
};

use super::theme::*;

pub(super) fn render_optimize(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, chunks[0], app);
    render_body(frame, chunks[1], app);
    render_summary(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
    if app.optimize.phase == OptimizePhase::Confirming {
        render_confirmation(frame, app);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let phase = copy(locale, phase_key(app.optimize.phase));
    let count = app.optimize.visible_entries().len();
    let title = if locale == Locale::ZhCn {
        "优化工作台"
    } else {
        "Optimize workbench"
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(app.shell.app_title.clone(), accent_style()),
            Span::styled(format!("  {title}  |  "), panel_style()),
            Span::styled(phase, warning_style()),
        ]),
        Line::from(copy(locale, "optimize.v1.summary.catalogue")),
        Line::from(format!("{count}/8")),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(focused_panel_block(if locale == Locale::ZhCn {
            "优化"
        } else {
            "Optimize"
        })),
        area,
    );
}

fn render_body(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if area.width < 86 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area);
        render_list(frame, chunks[0], app);
        render_details(frame, chunks[1], app);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area);
        render_list(frame, chunks[0], app);
        render_details(frame, chunks[1], app);
    }
}

fn render_list(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let items: Vec<ListItem<'_>> = app
        .optimize
        .visible_entries()
        .iter()
        .map(|entry| {
            let selected = app.optimize.selected_id.as_deref() == Some(entry.id);
            let mark = if selected { ">" } else { " " };
            let badge = copy(locale, action_key(entry.action_class));
            let extra = if entry.action_class == MaintenanceActionClass::Guidance {
                format!(" · {}", copy(locale, "optimize.v1.guidance.no_run"))
            } else {
                String::new()
            };
            ListItem::new(Line::from(format!(
                "{mark} {}{extra} | {badge} | {}",
                entry.id,
                copy(locale, title_key(entry.id))
            )))
        })
        .collect();
    let mut state = ListState::default();
    if !app.optimize.visible_entries().is_empty() {
        state.select(Some(app.optimize.cursor));
    }
    let list = List::new(items)
        .block(panel_block(if locale == Locale::ZhCn {
            "优化"
        } else {
            "Optimize"
        }))
        .highlight_style(selected_row_style().add_modifier(Modifier::BOLD));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let Some(entry) = app.optimize.focused_entry() else {
        frame.render_widget(
            Paragraph::new(copy(locale, "optimize.v1.state.empty.detail"))
                .wrap(Wrap { trim: true })
                .block(panel_block(if locale == Locale::ZhCn {
                    "目录标识"
                } else {
                    "Catalogue identity"
                })),
            area,
        );
        return;
    };
    let mut lines = vec![
        Line::from(format!(
            "{}: {}",
            copy(locale, "optimize.v1.detail.identity"),
            entry.id
        )),
        Line::from(copy(locale, action_key(entry.action_class))),
        Line::from(copy(locale, effect_key(entry))),
        Line::from(copy(locale, risk_key(entry.action_class))),
    ];
    if let Some(floor) = entry.build_floor {
        lines.push(Line::from(render_copy(
            locale,
            "optimize.v1.predicate.settings",
            &[("floor", &floor.to_string())],
        )));
    }
    if entry.action_class == MaintenanceActionClass::Guidance {
        lines.push(Line::from(copy(locale, "optimize.v1.guidance.no_run")));
        lines.push(Line::from(copy(locale, "optimize.v1.summary.guidance")));
    }
    if let Some(preview) = &app.optimize.preview {
        lines.push(Line::from(format!("digest {}", preview.digest)));
        lines.push(Line::from(copy(locale, "optimize.v1.preview.note")));
    }
    if let Some(report) = &app.optimize.report {
        for item in &report.outcomes {
            lines.push(Line::from(format!(
                "{} | {}",
                item.catalogue_id,
                copy(locale, outcome_key(item.outcome))
            )));
        }
    }
    if let Some(error) = &app.optimize.error {
        lines.push(Line::from(error.as_str()));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel_block(if locale == Locale::ZhCn {
                "能力"
            } else {
                "Capability"
            })),
        area,
    );
}

fn render_summary(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let text = summary_text(app, locale);
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(panel_block(if locale == Locale::ZhCn {
                "终态结果"
            } else {
                "Terminal outcome"
            })),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let action = match app
        .optimize
        .selected_entry()
        .map(|entry| entry.action_class)
    {
        Some(MaintenanceActionClass::Execute) => copy(locale, "optimize.v1.action.run"),
        Some(MaintenanceActionClass::SettingsHandoff) => copy(locale, "optimize.v1.action.open"),
        Some(MaintenanceActionClass::Guidance) => copy(locale, "optimize.v1.guidance.no_run"),
        None => copy(locale, "optimize.v1.summary.catalogue"),
    };
    frame.render_widget(Paragraph::new(action).alignment(Alignment::Left), area);
}

fn render_confirmation(frame: &mut Frame<'_>, app: &App) {
    let locale = app.shell.locale;
    let area = centered_rect(70, 40, frame.area());
    frame.render_widget(Clear, area);
    let settings = app
        .optimize
        .preview
        .as_ref()
        .is_some_and(|preview| preview.action_class == MaintenanceActionClass::SettingsHandoff);
    let title = copy(
        locale,
        if settings {
            "optimize.v1.confirm.title.settings"
        } else {
            "optimize.v1.confirm.title.execute"
        },
    );
    let detail = copy(
        locale,
        if settings {
            "optimize.v1.confirm.detail.settings"
        } else {
            "optimize.v1.confirm.detail.execute"
        },
    );
    let digest = app
        .optimize
        .preview
        .as_ref()
        .map(|preview| preview.digest.as_str())
        .unwrap_or("");
    let body = vec![
        Line::from(title),
        Line::from(detail),
        Line::from(render_copy(
            locale,
            "optimize.v1.confirm.digest",
            &[("digest", digest)],
        )),
        Line::from(copy(locale, "optimize.v1.preview.note")),
    ];
    frame.render_widget(
        Paragraph::new(body).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(if locale == Locale::ZhCn {
                    "进入二次确认"
                } else {
                    "Continue to confirmation"
                }),
        ),
        area,
    );
}

fn summary_text(app: &App, locale: Locale) -> String {
    if let Some(report) = &app.optimize.report {
        if report.outcomes.iter().any(|item| {
            item.outcome == MaintenanceExecutionOutcome::Launched
                && item.action_class == MaintenanceActionClass::SettingsHandoff
        }) {
            return copy(locale, "optimize.v1.summary.launched");
        }
        if report
            .outcomes
            .iter()
            .any(|item| item.outcome == MaintenanceExecutionOutcome::UnknownAfterDispatch)
        {
            return copy(locale, "optimize.v1.summary.unknown");
        }
        if report
            .outcomes
            .iter()
            .any(|item| item.outcome == MaintenanceExecutionOutcome::CanceledBeforeStart)
        {
            return copy(locale, "optimize.v1.summary.canceled");
        }
        if report
            .outcomes
            .iter()
            .any(|item| item.outcome == MaintenanceExecutionOutcome::Failed)
        {
            return copy(locale, "optimize.v1.summary.failed");
        }
        if report
            .outcomes
            .iter()
            .any(|item| item.outcome == MaintenanceExecutionOutcome::Succeeded)
        {
            return copy(locale, "optimize.v1.summary.succeeded");
        }
    }
    if let Some(entry) = app.optimize.selected_entry() {
        if entry.action_class == MaintenanceActionClass::Guidance {
            return copy(locale, "optimize.v1.summary.guidance");
        }
        let action = copy(locale, action_key(entry.action_class));
        if let Some(preview) = &app.optimize.preview {
            return render_copy(
                locale,
                "optimize.v1.summary.preview",
                &[
                    ("id", entry.id),
                    ("action", &action),
                    ("digest", preview.digest.as_str()),
                ],
            );
        }
        return render_copy(
            locale,
            "optimize.v1.summary.selected",
            &[("id", entry.id), ("action", &action)],
        );
    }
    copy(locale, "optimize.v1.summary.catalogue")
}

fn phase_key(phase: OptimizePhase) -> &'static str {
    match phase {
        OptimizePhase::Checking => "optimize.v1.state.checking",
        OptimizePhase::Ready => "optimize.v1.state.ready",
        OptimizePhase::Selected => "optimize.v1.state.selected",
        OptimizePhase::Previewing => "optimize.v1.state.previewing",
        OptimizePhase::PreviewReady => "optimize.v1.state.preview_ready",
        OptimizePhase::Confirming => "optimize.v1.state.confirming",
        OptimizePhase::Running => "optimize.v1.state.running",
        OptimizePhase::Launching => "optimize.v1.state.launching",
        OptimizePhase::Terminal => "optimize.v1.state.terminal",
        OptimizePhase::Unknown => "optimize.v1.state.unknown",
        OptimizePhase::Canceling => "optimize.v1.state.canceling",
    }
}

fn action_key(action_class: MaintenanceActionClass) -> &'static str {
    match action_class {
        MaintenanceActionClass::Execute => "optimize.v1.action.execute",
        MaintenanceActionClass::SettingsHandoff => "optimize.v1.action.settings",
        MaintenanceActionClass::Guidance => "optimize.v1.action.guidance",
    }
}

fn title_key(id: &str) -> &'static str {
    match id {
        "dns.flush" => "optimize.v1.title.dns.flush",
        "settings.storage_recommendations" => "optimize.v1.title.settings.storage_recommendations",
        "settings.search" => "optimize.v1.title.settings.search",
        "settings.energy_recommendations" => "optimize.v1.title.settings.energy_recommendations",
        "guidance.drive_optimize" => "optimize.v1.title.guidance.drive_optimize",
        "guidance.system_integrity" => "optimize.v1.title.guidance.system_integrity",
        "guidance.filesystem_check" => "optimize.v1.title.guidance.filesystem_check",
        "guidance.network_reset" => "optimize.v1.title.guidance.network_reset",
        _ => "optimize.v1.detail.identity",
    }
}

fn effect_key(entry: &MaintenanceCatalogueEntryV1) -> &'static str {
    match entry.action_class {
        MaintenanceActionClass::Execute => "optimize.v1.effect.dns.flush",
        MaintenanceActionClass::SettingsHandoff => "optimize.v1.effect.settings",
        MaintenanceActionClass::Guidance => "optimize.v1.effect.guidance",
    }
}

fn risk_key(action_class: MaintenanceActionClass) -> &'static str {
    match action_class {
        MaintenanceActionClass::Execute => "optimize.v1.risk.dns.flush",
        MaintenanceActionClass::SettingsHandoff => "optimize.v1.risk.settings",
        MaintenanceActionClass::Guidance => "optimize.v1.risk.guidance",
    }
}

fn outcome_key(outcome: MaintenanceExecutionOutcome) -> &'static str {
    match outcome {
        MaintenanceExecutionOutcome::CanceledBeforeStart => {
            "optimize.v1.outcome.canceled_before_start"
        }
        MaintenanceExecutionOutcome::Succeeded => "optimize.v1.outcome.succeeded",
        MaintenanceExecutionOutcome::Launched => "optimize.v1.outcome.launched",
        MaintenanceExecutionOutcome::Failed => "optimize.v1.outcome.failed",
        MaintenanceExecutionOutcome::UnknownAfterDispatch => {
            "optimize.v1.outcome.unknown_after_dispatch"
        }
    }
}

fn copy(locale: Locale, key: &str) -> String {
    crate::i18n::catalogue(locale)
        .render(key, &[], None)
        .unwrap_or_else(|_| key.to_string())
}

fn render_copy(locale: Locale, key: &str, values: &[(&str, &str)]) -> String {
    crate::i18n::catalogue(locale)
        .render(key, values, None)
        .unwrap_or_else(|_| key.to_string())
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
