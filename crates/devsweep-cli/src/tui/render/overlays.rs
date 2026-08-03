use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Clear, Paragraph, Wrap},
};

use crate::{
    rules::risk_label,
    tui::{
        app::{
            App, CleanupItemStatus, CleanupProgress, CleanupProgressItem, ConfirmState, Overlay,
        },
        display::{
            action_summary, compact_context_text, compact_text, format_cleanup_progress,
            selected_target_summary,
        },
    },
};

use super::{format_bytes, selected_details_lines, theme::*};

pub(super) fn render_overlay(frame: &mut Frame<'_>, app: &App) {
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

pub(super) fn dry_run_lines(app: &App) -> Vec<Line<'static>> {
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
