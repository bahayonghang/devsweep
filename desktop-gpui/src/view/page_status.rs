//! Status page: capability-aware metric cards and a process table. Missing
//! sensors stay visibly unavailable. A displayed PID is never a kill control.

use super::*;
use crate::status_panel::{ProcessSort, StatusPhase};
use devsweep_core::status::{AcState, AvailabilityV1, StatusSnapshotV1};

/// Value or the reason it is missing.
enum Reading<'a, T> {
    Value(&'a T, bool),
    Missing(&'static str, String),
}

fn reading<T>(availability: &AvailabilityV1<T>) -> Reading<'_, T> {
    match availability {
        AvailabilityV1::Available { value, .. } => Reading::Value(value, false),
        AvailabilityV1::Partial { value, .. } => Reading::Value(value, true),
        AvailabilityV1::Unavailable { reason_code, .. } => {
            Reading::Missing("unavailable", reason_code.clone())
        }
        AvailabilityV1::PermissionDenied { reason_code, .. } => {
            Reading::Missing("permission_denied", reason_code.clone())
        }
        AvailabilityV1::Unsupported { reason_code, .. } => {
            Reading::Missing("unsupported", reason_code.clone())
        }
    }
}

fn percent(basis_points: u32) -> f32 {
    basis_points as f32 / 100.0
}

impl ShellView {
    pub(super) fn status_page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let locale = self.session.locale();
        let live = self.session.status_phase() == StatusPhase::Live;
        let actions = vec![
            ui::any(
                ui::icon_button(
                    t,
                    "status-snapshot",
                    "refresh",
                    copy::status_action_snapshot(locale),
                    ButtonKind::Secondary,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.status_refresh(cx);
                })),
            ),
            ui::any(
                ui::icon_button(
                    t,
                    "status-live",
                    if live { "stop" } else { "play" },
                    if live {
                        copy::status_action_live_stop(locale)
                    } else {
                        copy::status_action_live_start(locale)
                    },
                    if live {
                        ButtonKind::Secondary
                    } else {
                        ButtonKind::Primary
                    },
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if this.session.status_phase() == StatusPhase::Live {
                        this.session.status_stop_live();
                    } else if this.session.status_start_live().is_ok() {
                        this.start_status_poll(cx);
                    }
                    cx.notify();
                })),
            ),
        ];
        let mut page = div()
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(self.page_header(
                t,
                "status",
                copy::mode_name(locale, "status"),
                copy::mode_subtitle(locale, "status"),
                actions,
            ))
            .children(self.shared_banners(t));
        if live {
            page = page.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.5))
                    .child(ui::dot(t.success))
                    .child(ui::muted(t, copy::status_phase_live(locale))),
            );
        }
        if let Some(line) = self.session.status_sensor_line() {
            page = page.child(ui::banner(t, Tone::Info, line));
        }
        let Some(snapshot) = self.session.status_snapshot() else {
            let error = self
                .session
                .status_lines()
                .iter()
                .find(|line| line.starts_with("unavailable"))
                .cloned();
            return page
                .child(
                    ui::card(t).items_center().child(ui::empty_state(
                        t,
                        "status",
                        copy::status_phase_landing(locale),
                        error.unwrap_or_else(|| {
                            copy::pick(
                                locale,
                                "Take a snapshot to read CPU, memory, disks, network, power, GPU, temperature and the busiest processes. Nothing is sampled until you ask.",
                                "生成快照以读取 CPU、内存、磁盘、网络、电源、GPU、温度和最繁忙的进程。你不请求就不会采样。",
                            )
                            .to_string()
                        }),
                    )),
                )
                .into_any_element();
        };
        let snapshot = snapshot.clone();
        page.child(self.status_metrics(t, &snapshot))
            .child(self.status_volumes(t, &snapshot))
            .child(self.status_processes(t, &snapshot, cx))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn status_metric(
        &self,
        t: &Theme,
        icon: &'static str,
        label: &'static str,
        value: String,
        detail: String,
        fraction: Option<f32>,
        partial: bool,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let color = match fraction {
            Some(f) if f >= 0.9 => t.danger,
            Some(f) if f >= 0.7 => t.warning,
            Some(_) => t.accent,
            None => t.text_subtle,
        };
        ui::card(t)
            .min_w_0()
            .gap(rems(0.55))
            .p(rems(1.0))
            .id(ElementId::Name(SharedString::from(format!(
                "status-metric-{icon}"
            ))))
            .aria_label(SharedString::from(format!("{label} {value} {detail}")))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.5))
                    .child(ui::icon(icon, t.text_muted))
                    .child(div().flex_1().child(ui::eyebrow(t, label)))
                    .when(partial, |d| {
                        d.child(ui::badge(
                            t,
                            copy::pick(locale, "partial", "部分"),
                            Tone::Warning,
                        ))
                    }),
            )
            .child(
                div()
                    .text_size(rems(1.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .when(fraction.is_none(), |d| {
                        d.text_color(rgb(t.text_subtle)).text_size(rems(1.05))
                    })
                    .child(value),
            )
            .when_some(fraction, |c, f| c.child(ui::meter(t, f, color)))
            .child(
                div()
                    .text_size(rems(0.76))
                    .text_color(rgb(t.text_muted))
                    .truncate()
                    .child(detail),
            )
            .into_any_element()
    }

    fn missing_metric(
        &self,
        t: &Theme,
        icon: &'static str,
        label: &'static str,
        state: &'static str,
        reason: String,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let text = match state {
            "permission_denied" => copy::pick(locale, "Permission denied", "权限不足"),
            "unsupported" => copy::pick(locale, "Not supported here", "此设备不支持"),
            _ => copy::pick(locale, "Unavailable", "不可用"),
        };
        self.status_metric(t, icon, label, text.to_string(), reason, None, false)
    }

    fn status_metrics(&self, t: &Theme, snapshot: &StatusSnapshotV1) -> gpui::AnyElement {
        let locale = self.session.locale();
        let mut cards: Vec<gpui::AnyElement> = Vec::new();
        let cpu_label = copy::pick(locale, "CPU", "CPU");
        cards.push(match reading(&snapshot.cpu) {
            Reading::Value(cpu, partial) => {
                let p = percent(cpu.system_utilization_basis_points);
                self.status_metric(
                    t,
                    "cpu",
                    cpu_label,
                    format!("{p:.0}%"),
                    match locale {
                        copy::Locale::En => {
                            format!("{} logical processors", snapshot.logical_processor_count)
                        }
                        copy::Locale::ZhCn => {
                            format!("{} 个逻辑处理器", snapshot.logical_processor_count)
                        }
                    },
                    Some(p / 100.0),
                    partial,
                )
            }
            Reading::Missing(state, reason) => {
                self.missing_metric(t, "cpu", cpu_label, state, reason)
            }
        });
        let mem_label = copy::pick(locale, "MEMORY", "内存");
        cards.push(match reading(&snapshot.memory) {
            Reading::Value(memory, partial) => {
                let fraction = memory.used_bytes as f32 / memory.total_bytes.max(1) as f32;
                self.status_metric(
                    t,
                    "memory",
                    mem_label,
                    format!("{:.0}%", fraction * 100.0),
                    format!(
                        "{} / {}",
                        ui::bytes(memory.used_bytes),
                        ui::bytes(memory.total_bytes)
                    ),
                    Some(fraction),
                    partial,
                )
            }
            Reading::Missing(state, reason) => {
                self.missing_metric(t, "memory", mem_label, state, reason)
            }
        });
        let gpu_label = copy::pick(locale, "GPU", "GPU");
        cards.push(match reading(&snapshot.gpu) {
            Reading::Value(gpu, partial) if !gpu.adapters.is_empty() => {
                let top = gpu
                    .adapters
                    .iter()
                    .map(|adapter| adapter.utilization_basis_points)
                    .max()
                    .unwrap_or(0);
                let p = percent(top);
                self.status_metric(
                    t,
                    "gpu",
                    gpu_label,
                    format!("{p:.0}%"),
                    match locale {
                        copy::Locale::En => format!("{} adapter(s)", gpu.adapters.len()),
                        copy::Locale::ZhCn => format!("{} 个适配器", gpu.adapters.len()),
                    },
                    Some(p / 100.0),
                    partial,
                )
            }
            Reading::Value(_, _) => {
                self.missing_metric(t, "gpu", gpu_label, "unavailable", String::new())
            }
            Reading::Missing(state, reason) => {
                self.missing_metric(t, "gpu", gpu_label, state, reason)
            }
        });
        let power_label = copy::pick(locale, "POWER", "电源");
        cards.push(match reading(&snapshot.power) {
            Reading::Value(power, partial) => {
                let ac = match power.ac_state {
                    AcState::Online => copy::pick(locale, "Plugged in", "已接通电源"),
                    AcState::Offline => copy::pick(locale, "On battery", "使用电池"),
                    AcState::Unknown => copy::pick(locale, "AC unknown", "电源状态未知"),
                };
                match (power.battery_present, power.charge_basis_points) {
                    (true, Some(charge)) => {
                        let p = percent(charge);
                        let remaining = power
                            .remaining_seconds
                            .map(|secs| format!(" · {}h {:02}m", secs / 3600, (secs % 3600) / 60))
                            .unwrap_or_default();
                        self.status_metric(
                            t,
                            "battery",
                            power_label,
                            format!("{p:.0}%"),
                            format!("{ac}{remaining}"),
                            Some(1.0 - p / 100.0),
                            partial,
                        )
                    }
                    _ => self.status_metric(
                        t,
                        "battery",
                        power_label,
                        copy::pick(locale, "No battery", "无电池").to_string(),
                        ac.to_string(),
                        None,
                        partial,
                    ),
                }
            }
            Reading::Missing(state, reason) => {
                self.missing_metric(t, "battery", power_label, state, reason)
            }
        });
        let net_label = copy::pick(locale, "NETWORK", "网络");
        cards.push(match reading(&snapshot.network) {
            Reading::Value(network, partial) => {
                let rx: u64 = network
                    .interfaces
                    .iter()
                    .map(|i| i.rx_bytes_per_second)
                    .sum();
                let tx: u64 = network
                    .interfaces
                    .iter()
                    .map(|i| i.tx_bytes_per_second)
                    .sum();
                self.status_metric(
                    t,
                    "network",
                    net_label,
                    format!("↓ {}/s", ui::bytes(rx)),
                    format!(
                        "↑ {}/s · {}",
                        ui::bytes(tx),
                        match locale {
                            copy::Locale::En => format!("{} interfaces", network.interfaces.len()),
                            copy::Locale::ZhCn => format!("{} 个接口", network.interfaces.len()),
                        }
                    ),
                    None,
                    partial,
                )
            }
            Reading::Missing(state, reason) => {
                self.missing_metric(t, "network", net_label, state, reason)
            }
        });
        let thermal_label = copy::pick(locale, "TEMPERATURE", "温度");
        cards.push(match reading(&snapshot.thermal) {
            Reading::Value(thermal, partial) if !thermal.zones.is_empty() => {
                let hottest = thermal
                    .zones
                    .iter()
                    .map(|zone| zone.temperature_tenths_celsius)
                    .max()
                    .unwrap_or(0);
                let celsius = hottest as f32 / 10.0;
                self.status_metric(
                    t,
                    "thermal",
                    thermal_label,
                    format!("{celsius:.0} °C"),
                    match locale {
                        copy::Locale::En => format!("hottest of {} zones", thermal.zones.len()),
                        copy::Locale::ZhCn => format!("{} 个区域中的最高值", thermal.zones.len()),
                    },
                    Some(celsius / 100.0),
                    partial,
                )
            }
            Reading::Value(_, _) => {
                self.missing_metric(t, "thermal", thermal_label, "unavailable", String::new())
            }
            Reading::Missing(state, reason) => {
                self.missing_metric(t, "thermal", thermal_label, state, reason)
            }
        });
        let reporting = [
            snapshot.cpu.has_value(),
            snapshot.memory.has_value(),
            snapshot.volumes.has_value(),
            snapshot.network.has_value(),
            snapshot.power.has_value(),
            snapshot.processes.has_value(),
        ]
        .into_iter()
        .filter(|value| *value)
        .count();
        let (health_label, health_tone) = if reporting == 0 {
            (
                copy::pick(
                    locale,
                    "No collectors on this platform",
                    "此平台没有可用采集器",
                ),
                Tone::Neutral,
            )
        } else if snapshot.supported_group_is_degraded() {
            (
                copy::pick(locale, "Some collectors degraded", "部分采集器降级"),
                Tone::Warning,
            )
        } else {
            (
                copy::pick(locale, "All collectors reporting", "所有采集器正常"),
                Tone::Success,
            )
        };
        div()
            .flex()
            .flex_col()
            .gap(rems(0.75))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.6))
                    .child(ui::eyebrow(t, copy::status_health_title(locale)))
                    .child(ui::badge(t, health_label, health_tone)),
            )
            .child(div().grid().grid_cols(3).gap(rems(0.75)).children(cards))
            .into_any_element()
    }

    fn status_volumes(&self, t: &Theme, snapshot: &StatusSnapshotV1) -> gpui::AnyElement {
        let locale = self.session.locale();
        let title = copy::pick(locale, "Disks", "磁盘");
        let body = match reading(&snapshot.volumes) {
            Reading::Value(volumes, _) if !volumes.items.is_empty() => {
                let rows = volumes.items.iter().map(|volume| {
                    let used = volume.total_bytes.saturating_sub(volume.available_bytes);
                    let fraction = used as f32 / volume.total_bytes.max(1) as f32;
                    let color = if fraction >= 0.9 {
                        t.danger
                    } else if fraction >= 0.75 {
                        t.warning
                    } else {
                        t.accent
                    };
                    let name = volume
                        .mount_points
                        .first()
                        .cloned()
                        .unwrap_or_else(|| volume.volume_id.clone());
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(rems(0.75))
                        .child(ui::icon("drive", t.text_muted))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap(rems(0.3))
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .justify_between()
                                        .gap(rems(0.5))
                                        .child(div().truncate().text_size(rems(0.88)).child(name))
                                        .child(ui::muted(
                                            t,
                                            match locale {
                                                copy::Locale::En => format!(
                                                    "{} free of {}",
                                                    ui::bytes(volume.available_bytes),
                                                    ui::bytes(volume.total_bytes)
                                                ),
                                                copy::Locale::ZhCn => format!(
                                                    "可用 {} / 共 {}",
                                                    ui::bytes(volume.available_bytes),
                                                    ui::bytes(volume.total_bytes)
                                                ),
                                            },
                                        )),
                                )
                                .child(ui::meter(t, fraction, color)),
                        )
                });
                div()
                    .flex()
                    .flex_col()
                    .gap(rems(0.9))
                    .children(rows)
                    .into_any_element()
            }
            Reading::Missing(_, reason) => ui::any(ui::muted(t, reason)),
            Reading::Value(_, _) => ui::any(ui::muted(
                t,
                copy::pick(locale, "No volumes reported.", "未报告卷。"),
            )),
        };
        ui::card(t)
            .gap(rems(0.9))
            .child(ui::card_header(t, title))
            .child(body)
            .into_any_element()
    }

    fn status_processes(
        &self,
        t: &Theme,
        snapshot: &StatusSnapshotV1,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let sort = self.session.status_sort();
        let rows =
            crate::status_panel::sorted_processes(snapshot, self.session.status_query(), sort);
        let header_cell = |label: &'static str, active: bool, width: Option<f32>| {
            let cell = div()
                .text_size(rems(0.72))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if active { t.text } else { t.text_subtle }));
            match width {
                Some(w) => cell.w(rems(w)).flex_shrink_0().text_right().child(label),
                None => cell.flex_1().child(label),
            }
        };
        let table_header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(rems(0.75))
            .px(rems(0.85))
            .pb(rems(0.4))
            .child(header_cell(
                copy::pick(locale, "PROCESS", "进程"),
                sort == ProcessSort::Name,
                None,
            ))
            .child(header_cell("PID", sort == ProcessSort::Pid, Some(5.0)))
            .child(header_cell("CPU", sort == ProcessSort::Cpu, Some(5.0)))
            .child(header_cell(
                copy::pick(locale, "MEMORY", "内存"),
                sort == ProcessSort::Memory,
                Some(6.5),
            ))
            .child(header_cell(
                copy::pick(locale, "DISK I/O", "磁盘 I/O"),
                false,
                Some(7.0),
            ));
        let max_cpu = rows
            .iter()
            .map(|row| row.cpu_basis_points_of_one_logical_core)
            .max()
            .unwrap_or(1)
            .max(1);
        let body = rows.iter().take(100).map(|row| {
            let cpu = percent(row.cpu_basis_points_of_one_logical_core);
            div()
                .id(gpui::ElementId::named_usize(
                    "status-process",
                    row.pid as usize,
                ))
                .role(gpui::Role::Row)
                .aria_label(SharedString::from(format!(
                    "{} pid {} cpu {cpu:.1}% {}",
                    row.name,
                    row.pid,
                    ui::bytes(row.private_bytes)
                )))
                .flex()
                .flex_row()
                .items_center()
                .gap(rems(0.75))
                .px(rems(0.85))
                .py(rems(0.45))
                .rounded(rems(0.5))
                .hover(|s| s.bg(rgb(t.surface_alt)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(rems(0.2))
                        .child(
                            div()
                                .truncate()
                                .text_size(rems(0.86))
                                .child(row.name.clone()),
                        )
                        .child(ui::meter(
                            t,
                            row.cpu_basis_points_of_one_logical_core as f32 / max_cpu as f32,
                            t.accent,
                        )),
                )
                .child(
                    div()
                        .w(rems(5.0))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(rems(0.8))
                        .text_color(rgb(t.text_subtle))
                        .child(row.pid.to_string()),
                )
                .child(
                    div()
                        .w(rems(5.0))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(rems(0.84))
                        .child(format!("{cpu:.1}%")),
                )
                .child(
                    div()
                        .w(rems(6.5))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(rems(0.84))
                        .child(ui::bytes(row.private_bytes)),
                )
                .child(
                    div()
                        .w(rems(7.0))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(rems(0.78))
                        .text_color(rgb(t.text_muted))
                        .child(format!(
                            "{}/s",
                            ui::bytes(row.read_bytes_per_second + row.write_bytes_per_second)
                        )),
                )
        });
        let sort_label = match sort {
            ProcessSort::Name => copy::pick(locale, "Sort: name", "排序：名称"),
            ProcessSort::Cpu => copy::pick(locale, "Sort: CPU", "排序：CPU"),
            ProcessSort::Memory => copy::pick(locale, "Sort: memory", "排序：内存"),
            ProcessSort::Pid => copy::pick(locale, "Sort: PID", "排序：PID"),
        };
        ui::card(t)
            .id("status-processes")
            .role(gpui::Role::Table)
            .aria_label(copy::pick(locale, "Processes", "进程"))
            .gap(rems(0.75))
            .child(
                ui::card_header(t, copy::pick(locale, "Processes", "进程"))
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(rems(0.5))
                            .w(rems(16.0))
                            .child(ui::icon("search", t.text_subtle))
                            .child(div().flex_1().child(self.search.clone())),
                    )
                    .child(
                        ui::icon_button(t, "status-sort", "sort", sort_label, ButtonKind::Ghost)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session.status_cycle_sort();
                                cx.notify();
                            })),
                    ),
            )
            .child(ui::muted(
                t,
                copy::pick(
                    locale,
                    "Read-only. A listed PID is information, not permission to end a process.",
                    "只读。列出的 PID 只是信息，不是结束进程的依据。",
                ),
            ))
            .child(table_header)
            .child(ui::divider(t))
            .child(div().flex().flex_col().children(body))
            .into_any_element()
    }
}
