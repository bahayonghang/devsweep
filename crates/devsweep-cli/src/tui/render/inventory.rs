use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

use crate::{
    inventory::{CapacityObservation, InventoryClassification, InventoryReport},
    tui::{
        app::App,
        display::{compact_context_text, compact_text, display_path, sanitize_display_text},
    },
};

use super::{
    BodyLayoutKind, body_layout_kind, format_bytes, health_label, health_style, health_totals_line,
    scan_diagnostic_lines, sizing_warning_kind_label, theme::*,
};

pub(super) fn render_inventory(frame: &mut Frame<'_>, area: Rect, app: &App) {
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
