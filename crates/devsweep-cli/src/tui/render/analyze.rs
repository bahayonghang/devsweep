use devsweep_core::analysis::{AnalyzeEvidence, AnalyzeNodeKind, AnalyzeWarningClass};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{List, ListItem, Paragraph},
};

use crate::{
    i18n::catalogue,
    tui::{
        app::App,
        modes::analyze::{AnalyzePhase, proportional_bar},
    },
};

use super::{
    format::format_bytes,
    theme::{
        accent_style, error_style, focused_panel_block, muted_style, panel_block, panel_style,
        selected_row_style, warning_style,
    },
};

pub(super) fn render_analyze(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, chunks[0], app);
    render_breadcrumbs(frame, chunks[1], app);
    render_directory(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let catalogue = catalogue(app.shell.locale);
    let count = app.analyze.stored_nodes.to_string();
    let bytes = app
        .analyze
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.nodes.first())
        .map_or_else(
            || format_bytes(app.analyze.accounted_owned_bytes),
            |root| format_bytes(root.bytes),
        );
    let status = match app.analyze.phase {
        AnalyzePhase::Idle => catalogue
            .static_text("analyze.v1.state.empty.detail")
            .unwrap_or("Choose a read-only root and start analysis.")
            .to_string(),
        AnalyzePhase::Loading => catalogue
            .render("analyze.v1.state.loading", &[("count", &count)], None)
            .unwrap_or_else(|_| format!("Analyzing {count} node(s)")),
        AnalyzePhase::Canceling => catalogue
            .static_text("analyze.v1.state.canceling")
            .unwrap_or("Cancel requested; waiting for workers to join")
            .to_string(),
        AnalyzePhase::Complete => catalogue
            .render(
                "analyze.v1.scan.complete",
                &[("count", &count), ("bytes", &bytes)],
                None,
            )
            .unwrap_or_else(|_| format!("Analysis complete: {count} node(s), {bytes}")),
        AnalyzePhase::Partial => catalogue
            .render(
                "analyze.v1.scan.partial",
                &[("count", &count), ("bytes", &bytes)],
                None,
            )
            .unwrap_or_else(|_| format!("Partial analysis: {count} node(s), >= {bytes}")),
        AnalyzePhase::Canceled => catalogue
            .render(
                "analyze.v1.scan.canceled",
                &[("count", &count), ("bytes", &bytes)],
                None,
            )
            .unwrap_or_else(|_| format!("Canceled: {count} node(s), >= {bytes}")),
        AnalyzePhase::Failed => {
            let summary = catalogue
                .static_text("analyze.v1.state.error")
                .unwrap_or("Analyze failed");
            app.analyze
                .error
                .as_ref()
                .map_or_else(|| summary.to_string(), |error| format!("{summary} {error}"))
        }
    };
    let style = match app.analyze.phase {
        AnalyzePhase::Complete => accent_style(),
        AnalyzePhase::Partial | AnalyzePhase::Canceled | AnalyzePhase::Canceling => warning_style(),
        AnalyzePhase::Failed => error_style(),
        AnalyzePhase::Idle | AnalyzePhase::Loading => panel_style(),
    };
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled(app.shell.app_title.clone(), accent_style()),
                Span::styled(format!("  {}", app.shell.active_label), panel_style()),
                Span::styled("  |  ", muted_style()),
                Span::styled(status, style),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "{} ",
                        catalogue
                            .static_text("analyze.v1.search.label")
                            .unwrap_or("Search")
                    ),
                    muted_style(),
                ),
                Span::styled(
                    if app.filter_active {
                        format!("/{}█", app.analyze.query)
                    } else if app.analyze.query.is_empty() {
                        "(none)".to_string()
                    } else {
                        app.analyze.query.clone()
                    },
                    panel_style(),
                ),
                Span::styled(
                    format!(
                        "  {} ",
                        catalogue
                            .static_text("analyze.v1.sort.label")
                            .unwrap_or("Sort")
                    ),
                    muted_style(),
                ),
                Span::styled(sort_label(app), panel_style()),
            ]),
        ]))
        .block(focused_panel_block(
            catalogue
                .static_text("command.analyze")
                .unwrap_or("Analyze"),
        )),
        area,
    );
}

fn sort_label(app: &App) -> &'static str {
    let key = match app.analyze.sort {
        crate::tui::modes::analyze::AnalyzeSort::SizeDescending => "analyze.v1.sort.size_desc",
        crate::tui::modes::analyze::AnalyzeSort::SizeAscending => "analyze.v1.sort.size_asc",
        crate::tui::modes::analyze::AnalyzeSort::Name => "analyze.v1.sort.name",
        crate::tui::modes::analyze::AnalyzeSort::Kind => "analyze.v1.sort.kind",
    };
    catalogue(app.shell.locale)
        .static_text(key)
        .unwrap_or("Sort")
}

fn render_breadcrumbs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let path = app
        .analyze
        .breadcrumbs()
        .into_iter()
        .map(|node| {
            if node.id == 0 {
                app.analyze
                    .snapshot
                    .as_ref()
                    .map_or(node.name.as_str(), |snapshot| {
                        snapshot.root.normalized.as_str()
                    })
            } else {
                node.name.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join(" > ");
    let page = app.analyze.page + 1;
    let pages = app.analyze.page_count();
    let count = app.analyze.visible_children().len().to_string();
    let page = page.to_string();
    let pages = pages.to_string();
    let page_copy = catalogue(app.shell.locale)
        .render(
            "analyze.v1.list.page",
            &[("page", &page), ("pages", &pages), ("count", &count)],
            None,
        )
        .unwrap_or_else(|_| format!("Page {page}/{pages}; {count} entries"));
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(path, panel_style()),
            Span::styled(format!("  |  {page_copy}"), muted_style()),
        ]))
        .block(panel_block(
            catalogue(app.shell.locale)
                .static_text("analyze.v1.breadcrumbs.label")
                .unwrap_or("Breadcrumbs"),
        )),
        area,
    );
}

fn render_directory(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let messages = catalogue(app.shell.locale);
    let row_capacity = area.height.saturating_sub(2) as usize;
    let rows = app.analyze.viewport_rows(row_capacity);
    let focused_node_id = app.analyze.focused_node().map(|node| node.id);
    let maximum = app
        .analyze
        .page_rows()
        .into_iter()
        .filter(|node| node.evidence != AnalyzeEvidence::Unknown)
        .map(|node| node.bytes)
        .max()
        .unwrap_or(0);
    let bar_width = (area.width as usize / 4).clamp(4, 24);
    let items = rows
        .iter()
        .map(|node| {
            let is_focused = focused_node_id == Some(node.id);
            let marker = if is_focused { ">" } else { " " };
            let kind = match node.kind {
                AnalyzeNodeKind::Directory => messages
                    .static_text("analyze.v1.kind.directory")
                    .unwrap_or("Directory"),
                AnalyzeNodeKind::File => messages
                    .static_text("analyze.v1.kind.file")
                    .unwrap_or("File"),
                AnalyzeNodeKind::Reparse => messages
                    .static_text("analyze.v1.kind.reparse")
                    .unwrap_or("Reparse"),
            };
            let evidence = match node.evidence {
                AnalyzeEvidence::Complete => messages
                    .static_text("analyze.v1.evidence.complete")
                    .unwrap_or("Available"),
                AnalyzeEvidence::Incomplete => messages
                    .static_text("analyze.v1.evidence.incomplete")
                    .unwrap_or("Partial"),
                AnalyzeEvidence::Unknown => messages
                    .static_text("analyze.v1.evidence.unknown")
                    .unwrap_or("Unknown"),
            };
            let warning = if node.warnings.contains(&AnalyzeWarningClass::AccessDenied) {
                messages
                    .static_text("analyze.v1.warning.permission")
                    .unwrap_or("Permission denied")
            } else if node.kind == AnalyzeNodeKind::Reparse
                || node.warnings.contains(&AnalyzeWarningClass::Reparse)
            {
                messages
                    .static_text("analyze.v1.warning.unsupported")
                    .unwrap_or("Unsupported entry")
            } else {
                ""
            };
            let bar = proportional_bar(node, maximum, bar_width);
            let bytes = if node.evidence == AnalyzeEvidence::Unknown {
                "unknown".to_string()
            } else {
                format_bytes(node.bytes)
            };
            let line = format!(
                "{marker} {kind:<10} {evidence:<14} {bytes:>10} {bar:<bar_width$} {}{}",
                node.name,
                if warning.is_empty() {
                    String::new()
                } else {
                    format!(" — {warning}")
                }
            );
            let style = if is_focused {
                selected_row_style()
            } else if node.evidence == AnalyzeEvidence::Incomplete {
                warning_style()
            } else {
                panel_style()
            };
            ListItem::new(line).style(style)
        })
        .collect::<Vec<_>>();
    let body = if items.is_empty() {
        List::new(vec![ListItem::new(
            messages
                .static_text("analyze.v1.list.empty")
                .unwrap_or("No represented children in this directory"),
        )])
    } else {
        List::new(items)
    };
    frame.render_widget(
        body.block(panel_block(
            messages
                .static_text("analyze.v1.list.title")
                .unwrap_or("Accessible directory list"),
        )),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let messages = catalogue(app.shell.locale);
    let start = messages
        .static_text("analyze.v1.action.start")
        .unwrap_or("Analyze path");
    let open = messages
        .static_text("analyze.v1.action.open")
        .unwrap_or("Open selected directory");
    let up = messages
        .static_text("analyze.v1.action.up")
        .unwrap_or("Up one level");
    let search = messages
        .static_text("analyze.v1.search.label")
        .unwrap_or("Search");
    let sort = messages
        .static_text("analyze.v1.sort.label")
        .unwrap_or("Sort");
    let quit = messages
        .static_text("analyze.v1.action.quit")
        .unwrap_or("Quit");
    let cancel = if app.analyze.is_active() {
        format!(
            "  [x] {}",
            messages
                .static_text("analyze.v1.action.cancel")
                .unwrap_or("Cancel analysis")
        )
    } else {
        String::new()
    };
    frame.render_widget(
        Paragraph::new(format!(
            " [Alt+C] Clean  [Alt+A] Analyze  [s] {start}{cancel}  [Enter] {open}  [Backspace] {up}  [/] {search}  [o] {sort}  [q] {quit}"
        ))
        .style(panel_style()),
        area,
    );
}
