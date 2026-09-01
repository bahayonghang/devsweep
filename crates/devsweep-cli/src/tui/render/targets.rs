use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

use crate::{
    model::{CleanTarget, Ecosystem, Scope},
    rules::{RuleScope, risk_label, rule_catalogue, rule_row},
    tui::{
        app::{App, PycacheGroup, ScopeKind, TargetListRow},
        display::{
            action_summary, compact_path, compact_text, display_path, display_path_text,
            path_label, sanitize_display_text, scope_label, target_title,
        },
    },
};

use super::{
    detail_line, evidence_summary, format_bytes, format_target_bytes, scan_totals_line,
    sizing_warning_kind_label, theme::*,
};

pub(super) fn render_categories(frame: &mut Frame<'_>, area: Rect, app: &App) {
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

pub(super) fn render_compact_summary(frame: &mut Frame<'_>, area: Rect, app: &App) {
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

pub(super) fn render_targets(frame: &mut Frame<'_>, area: Rect, app: &App) {
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

pub(super) fn render_details_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = selected_details_lines(app);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block("Details"))
            .style(panel_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(super) fn selected_details_lines(app: &App) -> Vec<Line<'static>> {
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

pub(super) fn render_rules(frame: &mut Frame<'_>, area: Rect) {
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

pub(super) fn target_details_lines(target: &CleanTarget) -> Vec<Line<'static>> {
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
