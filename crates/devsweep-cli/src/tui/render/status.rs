use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

use crate::{
    i18n::{Locale, format_binary_bytes},
    tui::app::App,
    tui::modes::status::{ChartPoint, ProcessSort, StatusPhase, basis_points},
};
use devsweep_core::status::{AcState, AvailabilityV1, StatusSnapshotV1};

use super::theme::*;

pub(super) fn render_status(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(9),
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, chunks[0], app);
    render_cards(frame, chunks[1], app);
    render_charts(frame, chunks[2], app);
    render_processes(frame, chunks[3], app);
    render_footer(frame, chunks[4], app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let phase = copy(locale, phase_key(app.status.phase));
    let interval = (app.status.interval_ms / 1_000).to_string();
    let mut lines = vec![Line::from(vec![
        Span::styled(app.shell.app_title.clone(), accent_style()),
        Span::styled(format!("  {}  |  ", app.shell.active_label), panel_style()),
        Span::styled(phase, warning_style()),
        Span::styled(
            format!(
                "  {}",
                render_copy(
                    locale,
                    "status.v1.interval.value",
                    &[("seconds", interval.as_str())]
                )
            ),
            muted_style(),
        ),
    ])];
    if let Some(snapshot) = &app.status.snapshot {
        lines.push(Line::from(vec![
            Span::styled(
                render_copy(
                    locale,
                    "status.v1.snapshot.title",
                    &[("id", snapshot.snapshot_id.as_str())],
                ),
                panel_style(),
            ),
            Span::styled(
                format!(
                    "  {}",
                    render_copy(
                        locale,
                        "status.v1.sampled_at",
                        &[("timestamp", &snapshot.sampled_at_unix_ms.to_string())]
                    )
                ),
                muted_style(),
            ),
        ]));
        lines.push(Line::from(copy(locale, "status.v1.capability.note")));
    } else {
        lines.push(Line::from(copy(locale, "status.v1.chart.empty")));
    }
    if let Some(error) = &app.status.error {
        lines.push(Line::from(Span::styled(
            render_copy(locale, "status.v1.decode.error", &[("reason", error)]),
            warning_style(),
        )));
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(focused_panel_block("Status"))
            .style(panel_style()),
        area,
    );
}

fn render_cards(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let Some(snapshot) = &app.status.snapshot else {
        frame.render_widget(
            Paragraph::new(copy(locale, "status.v1.chart.empty")).style(muted_style()),
            area,
        );
        return;
    };
    let text = card_lines(locale, snapshot).join("\n");
    frame.render_widget(
        Paragraph::new(text)
            .block(focused_panel_block("Metrics"))
            .wrap(Wrap { trim: true })
            .style(panel_style()),
        area,
    );
}

fn render_charts(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let mut lines = Vec::new();
    if app.status.chart.is_empty() {
        lines.push(Line::from(copy(locale, "status.v1.chart.empty")));
    } else {
        lines.push(chart_line(
            locale,
            "status.v1.chart.cpu",
            &app.status.chart,
            |point| point.cpu_bp.map(basis_points),
        ));
        lines.push(chart_line(
            locale,
            "status.v1.chart.memory",
            &app.status.chart,
            |point| point.memory_used_bytes.map(format_binary_bytes),
        ));
        lines.push(chart_line(
            locale,
            "status.v1.chart.network.rx",
            &app.status.chart,
            |point| point.rx_bytes_per_second.map(format_binary_bytes),
        ));
        lines.push(chart_line(
            locale,
            "status.v1.chart.network.tx",
            &app.status.chart,
            |point| point.tx_bytes_per_second.map(format_binary_bytes),
        ));
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(focused_panel_block("Live"))
            .wrap(Wrap { trim: true })
            .style(panel_style()),
        area,
    );
}

fn render_processes(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let mut lines = Vec::new();
    if let Some(snapshot) = &app.status.snapshot {
        match &snapshot.processes {
            AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
                lines.push(Line::from(render_copy(
                    locale,
                    "status.v1.process.summary",
                    &[
                        ("returned", &value.returned_count.to_string()),
                        ("enumerated", &value.enumerated_count.to_string()),
                        ("limit", &value.requested_limit.to_string()),
                    ],
                )));
                if value.truncated_by_limit {
                    lines.push(Line::from(render_copy(
                        locale,
                        "status.v1.process.truncated",
                        &[
                            ("returned", &value.returned_count.to_string()),
                            ("enumerated", &value.enumerated_count.to_string()),
                        ],
                    )));
                }
                if value.budget_exhausted {
                    lines.push(Line::from(render_copy(
                        locale,
                        "status.v1.process.budget",
                        &[("budget", &value.detail_budget_ms.to_string())],
                    )));
                }
                if let AvailabilityV1::Partial { reason_codes, .. } = &snapshot.processes {
                    lines.push(Line::from(render_copy(
                        locale,
                        "status.v1.group.partial",
                        &[("group", "processes"), ("reasons", &reason_codes.join(","))],
                    )));
                }
            }
            other => lines.push(Line::from(tagged_group(locale, "processes", other))),
        }
    }
    let sort = match app.status.process_sort {
        ProcessSort::Cpu => "status.v1.process.sort.cpu",
        ProcessSort::Memory => "status.v1.process.sort.memory",
        ProcessSort::Name => "status.v1.process.sort.name",
        ProcessSort::Pid => "status.v1.process.sort.pid",
    };
    lines.push(Line::from(copy(locale, sort)));
    for (index, process) in app.status.sorted_processes().into_iter().enumerate() {
        let marker = if index == app.status.cursor { ">" } else { " " };
        lines.push(Line::from(format!(
            "{marker} {}",
            render_copy(
                locale,
                "status.v1.process.item",
                &[
                    ("name", process.name.as_str()),
                    ("pid", &process.pid.to_string()),
                    (
                        "percent",
                        &basis_points(process.cpu_basis_points_of_one_logical_core)
                    ),
                    ("memory", &format_binary_bytes(process.private_bytes)),
                ],
            )
        )));
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(focused_panel_block("Processes"))
            .wrap(Wrap { trim: true })
            .style(panel_style()),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let locale = app.shell.locale;
    let live =
        if app.status.phase == StatusPhase::Live || app.status.phase == StatusPhase::Canceling {
            copy(locale, "status.v1.action.live.stop")
        } else {
            copy(locale, "status.v1.action.live.start")
        };
    frame.render_widget(
        Paragraph::new(format!(
            "{}  {}  {}",
            copy(locale, "status.v1.action.snapshot"),
            live,
            copy(locale, "status.v1.footer.help")
        ))
        .alignment(Alignment::Left)
        .style(muted_style()),
        area,
    );
}

fn card_lines(locale: Locale, snapshot: &StatusSnapshotV1) -> Vec<String> {
    let mut lines = vec![
        render_copy(
            locale,
            "status.v1.logical.processors",
            &[("count", &snapshot.logical_processor_count.to_string())],
        ),
        render_copy(
            locale,
            "status.v1.window",
            &[("ms", &snapshot.sample_window_ms.to_string())],
        ),
        metric_line(locale, "cpu", &snapshot.cpu, |cpu| {
            render_copy(
                locale,
                "status.v1.cpu",
                &[(
                    "percent",
                    &basis_points(cpu.system_utilization_basis_points),
                )],
            )
        }),
        metric_line(locale, "memory", &snapshot.memory, |memory| {
            render_copy(
                locale,
                "status.v1.memory",
                &[
                    ("used", &format_binary_bytes(memory.used_bytes)),
                    ("total", &format_binary_bytes(memory.total_bytes)),
                    ("available", &format_binary_bytes(memory.available_bytes)),
                ],
            )
        }),
    ];
    match &snapshot.volumes {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            for volume in &value.items {
                let mount = volume
                    .mount_points
                    .first()
                    .map(String::as_str)
                    .unwrap_or("-");
                lines.push(render_copy(
                    locale,
                    "status.v1.volume.item",
                    &[
                        ("mount", mount),
                        ("available", &format_binary_bytes(volume.available_bytes)),
                        ("total", &format_binary_bytes(volume.total_bytes)),
                    ],
                ));
            }
        }
        other => lines.push(tagged_group(locale, "volumes", other)),
    }
    match &snapshot.network {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            for interface in &value.interfaces {
                lines.push(render_copy(
                    locale,
                    "status.v1.network.item",
                    &[
                        ("name", interface.name.as_str()),
                        ("rx", &format_binary_bytes(interface.rx_bytes_per_second)),
                        ("tx", &format_binary_bytes(interface.tx_bytes_per_second)),
                    ],
                ));
            }
        }
        other => lines.push(tagged_group(locale, "network", other)),
    }
    lines.push(power_line(locale, snapshot));
    lines
}

fn power_line(locale: Locale, snapshot: &StatusSnapshotV1) -> String {
    match &snapshot.power {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            let ac_key = match value.ac_state {
                AcState::Online => "status.v1.ac.online",
                AcState::Offline => "status.v1.ac.offline",
                AcState::Unknown => "status.v1.ac.unknown",
            };
            let ac = copy(locale, ac_key);
            let mut line = render_copy(locale, "status.v1.power.ac", &[("ac", &ac)]);
            if value.battery_present {
                let percent = value
                    .charge_basis_points
                    .map(basis_points)
                    .unwrap_or_else(|| "-".into());
                let remaining = value
                    .remaining_seconds
                    .map(|seconds| seconds.to_string())
                    .unwrap_or_else(|| "-".into());
                line.push(' ');
                line.push_str(&render_copy(
                    locale,
                    "status.v1.power.battery",
                    &[("percent", &percent), ("remaining", &remaining)],
                ));
            } else {
                line.push(' ');
                line.push_str(&copy(locale, "status.v1.power.no_battery"));
            }
            line
        }
        other => tagged_group(locale, "power", other),
    }
}

fn metric_line<T>(
    locale: Locale,
    group: &str,
    availability: &AvailabilityV1<T>,
    format: impl FnOnce(&T) -> String,
) -> String {
    match availability {
        AvailabilityV1::Available { value, age_ms, .. } => {
            let mut line = format(value);
            if *age_ms > 0 {
                line.push(' ');
                line.push_str(&render_copy(
                    locale,
                    "status.v1.age",
                    &[("age", &age_ms.to_string())],
                ));
            }
            line
        }
        AvailabilityV1::Partial {
            value,
            reason_codes,
            age_ms,
            ..
        } => {
            let mut line = format(value);
            line.push(' ');
            line.push_str(&render_copy(
                locale,
                "status.v1.group.partial",
                &[("group", group), ("reasons", &reason_codes.join(","))],
            ));
            if *age_ms > 2_000 {
                line.push(' ');
                line.push_str(&render_copy(
                    locale,
                    "status.v1.stale",
                    &[("age", &age_ms.to_string())],
                ));
            }
            line
        }
        other => tagged_group(locale, group, other),
    }
}

fn tagged_group<T>(locale: Locale, group: &str, availability: &AvailabilityV1<T>) -> String {
    match availability {
        AvailabilityV1::Unavailable { reason_code, .. } => render_copy(
            locale,
            "status.v1.group.unavailable",
            &[("group", group), ("reason", reason_code)],
        ),
        AvailabilityV1::PermissionDenied { reason_code, .. } => render_copy(
            locale,
            "status.v1.group.permission_denied",
            &[("group", group), ("reason", reason_code)],
        ),
        AvailabilityV1::Unsupported { reason_code, .. } => render_copy(
            locale,
            "status.v1.group.unsupported",
            &[("group", group), ("reason", reason_code)],
        ),
        AvailabilityV1::Partial { reason_codes, .. } => render_copy(
            locale,
            "status.v1.group.partial",
            &[("group", group), ("reasons", &reason_codes.join(","))],
        ),
        AvailabilityV1::Available { .. } => group.to_string(),
    }
}

fn chart_line(
    locale: Locale,
    key: &str,
    points: &[ChartPoint],
    present: impl Fn(&ChartPoint) -> Option<String>,
) -> Line<'static> {
    let gap = copy(locale, "status.v1.chart.gap");
    let values = points
        .iter()
        .map(|point| present(point).unwrap_or_else(|| gap.clone()))
        .collect::<Vec<_>>()
        .join(", ");
    let spark = points
        .iter()
        .map(|point| match present(point) {
            Some(_) => '█',
            None => '·',
        })
        .collect::<String>();
    Line::from(format!(
        "{} {spark}  {}",
        copy(locale, key),
        render_copy(
            locale,
            "status.v1.chart.alternative",
            &[("label", &copy(locale, key)), ("values", &values)]
        )
    ))
}

fn phase_key(phase: StatusPhase) -> &'static str {
    match phase {
        StatusPhase::Idle => "status.v1.state.idle",
        StatusPhase::Snapshot => "status.v1.state.loading",
        StatusPhase::Ready => "status.v1.state.ready",
        StatusPhase::Live => "status.v1.state.live",
        StatusPhase::Canceling => "status.v1.state.canceling",
        StatusPhase::Failed => "status.v1.state.failed",
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
