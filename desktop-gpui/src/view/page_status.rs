//! Status page: a live metric grid with one-minute trends, the process table
//! and the volumes. Sampling runs while the page is on screen and stops when
//! it is not. Missing sensors stay visibly unavailable with their reason; no
//! reading is drawn as zero. A displayed PID is never a kill control.

use super::*;
use crate::copy::{Locale, StatusLiveState};
use crate::shell::{STATUS_LIVE_INTERVAL_MS, StatusLiveFault};
use crate::status_panel::{
    ProcessSort, StatusPhase, TREND_CAPACITY, Trend, TrendMetric, traffic_interfaces,
    traffic_totals, trend_segments, value_of,
};
use devsweep_core::status::{
    AcState, AvailabilityV1, HostV1, NetworkKind, PowerV1, StatusSnapshotV1, explain_health,
};
use gpui::{PathBuilder, canvas, point};

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

/// How a tile's lead line is set.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Figure {
    /// A measured reading, in the output colour.
    Reading,
    /// A plain fact or conclusion, in the text colour.
    Plain,
    /// Nothing measured: the state in words, smaller and muted.
    Missing,
}

/// What sits between the figure and the explanation line.
enum Visual {
    Trend(TrendMetric),
    Bar(f32, u32),
    Lines(Vec<String>),
    None,
}

/// One metric tile: label, one figure, a trend or bar, one explanation line.
struct Tile {
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    figure: String,
    style: Figure,
    partial: bool,
    visual: Visual,
    detail: String,
}

impl Tile {
    fn missing(
        locale: Locale,
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        state: &str,
        reason: &str,
    ) -> Self {
        let figure = match state {
            "permission_denied" => copy::pick(locale, "Permission denied", "权限不足"),
            "unsupported" => copy::pick(locale, "Not supported here", "此设备不支持"),
            _ => copy::pick(locale, "Unavailable", "不可用"),
        };
        Self {
            id,
            icon,
            label,
            figure: figure.to_string(),
            style: Figure::Missing,
            partial: false,
            visual: Visual::None,
            detail: copy::status_reason(locale, reason),
        }
    }
}

fn frequency(mhz: u32) -> String {
    if mhz >= 1_000 {
        format!("{:.2} GHz", mhz as f32 / 1_000.0)
    } else {
        format!("{mhz} MHz")
    }
}

fn uptime(locale: Locale, ms: u64) -> String {
    let minutes = ms / 60_000;
    let (days, hours, mins) = (minutes / 1_440, (minutes % 1_440) / 60, minutes % 60);
    match locale {
        Locale::En if days > 0 => format!("Up {days} d {hours} h"),
        Locale::En => format!("Up {hours} h {mins} min"),
        Locale::ZhCn if days > 0 => format!("已运行 {days} 天 {hours} 小时"),
        Locale::ZhCn => format!("已运行 {hours} 小时 {mins} 分钟"),
    }
}

fn per_second(value: u64) -> String {
    format!("{}/s", ui::bytes(value))
}

/// Machine identity lines for the health tile: CPU model, Windows version,
/// uptime. A field that was not read says so.
fn identity_lines(locale: Locale, snapshot: &StatusSnapshotV1) -> Vec<String> {
    match reading(&snapshot.host) {
        Reading::Value(host, _) => {
            let HostV1 {
                cpu_model,
                os_version,
                boot_time_unix_ms,
            } = host;
            vec![
                cpu_model.clone().unwrap_or_else(|| {
                    copy::pick(locale, "CPU model not reported", "未报告 CPU 型号").to_string()
                }),
                os_version.as_ref().map_or_else(
                    || {
                        copy::pick(
                            locale,
                            "Windows version not reported",
                            "未报告 Windows 版本",
                        )
                        .to_string()
                    },
                    |version| format!("Windows {version}"),
                ),
                boot_time_unix_ms
                    .filter(|boot| *boot <= snapshot.sampled_at_unix_ms)
                    .map_or_else(
                        || copy::pick(locale, "Uptime not reported", "未报告运行时长").to_string(),
                        |boot| uptime(locale, snapshot.sampled_at_unix_ms - boot),
                    ),
            ]
        }
        Reading::Missing(_, reason) => vec![match locale {
            Locale::En => format!(
                "Machine identity not reported. {}",
                copy::status_reason(locale, &reason)
            ),
            Locale::ZhCn => format!("未报告机器信息。{}", copy::status_reason(locale, &reason)),
        }],
    }
}

/// The health tile states `explain_health`'s conclusion, never a score.
fn health_tile(locale: Locale, snapshot: &StatusSnapshotV1) -> Tile {
    let evidence = explain_health(snapshot).evidence;
    let gaps = evidence
        .iter()
        .filter(|line| {
            matches!(
                line.state.as_str(),
                "unavailable" | "unsupported" | "permission_denied"
            )
        })
        .count();
    let readings = evidence.len() - gaps;
    // `explain_health` has no headline; this names its CPU verdict. Only CPU
    // evidence carries the `load` state, so a missing CPU reading is said as
    // such rather than as "not busy".
    let cpu_measured = evidence.iter().any(|line| {
        line.subject == "cpu" && matches!(line.state.as_str(), "available" | "partial")
    });
    let figure = if evidence.iter().any(|line| line.state == "load") {
        copy::pick(locale, "CPU busy", "CPU 繁忙")
    } else if evidence.iter().any(|line| line.state == "stale") {
        copy::pick(locale, "Stale reading", "读数过时")
    } else if cpu_measured {
        copy::pick(locale, "CPU not busy", "CPU 不繁忙")
    } else {
        copy::pick(locale, "Load not measured", "未测得负载")
    };
    Tile {
        id: "health",
        icon: "shield",
        label: copy::pick(locale, "HEALTH EVIDENCE", "健康证据"),
        figure: figure.to_string(),
        style: Figure::Plain,
        partial: false,
        visual: Visual::Lines(identity_lines(locale, snapshot)),
        detail: match locale {
            Locale::En => format!("{readings} readings, {gaps} not reported. No score."),
            Locale::ZhCn => format!("{readings} 项有读数，{gaps} 项未上报。不打分。"),
        },
    }
}

fn cpu_tile(locale: Locale, snapshot: &StatusSnapshotV1) -> Tile {
    let label = "CPU";
    match reading(&snapshot.cpu) {
        Reading::Value(cpu, partial) => {
            let speed = cpu.frequency_mhz.map_or_else(
                || copy::pick(locale, "frequency not reported", "未报告频率").to_string(),
                frequency,
            );
            let logical = snapshot.logical_processor_count;
            let mut detail = match locale {
                Locale::En => format!("{speed} · {logical} logical processors"),
                Locale::ZhCn => format!("{speed} · {logical} 个逻辑处理器"),
            };
            if let Some(hottest) = value_of(&snapshot.thermal).and_then(|thermal| {
                thermal
                    .zones
                    .iter()
                    .map(|zone| zone.temperature_tenths_celsius)
                    .max()
            }) {
                detail.push_str(&format!(" · {:.0} °C", hottest as f32 / 10.0));
            }
            Tile {
                id: "cpu",
                icon: "cpu",
                label,
                figure: format!("{:.0}%", percent(cpu.system_utilization_basis_points)),
                style: Figure::Reading,
                partial,
                visual: Visual::Trend(TrendMetric::Cpu),
                detail,
            }
        }
        Reading::Missing(state, reason) => {
            Tile::missing(locale, "cpu", "cpu", label, state, &reason)
        }
    }
}

fn gpu_tile(locale: Locale, snapshot: &StatusSnapshotV1) -> Tile {
    let label = "GPU";
    match reading(&snapshot.gpu) {
        Reading::Value(gpu, partial) => {
            // The figure, the name and the trend all describe the busiest
            // adapter. A tie goes to the first named adapter.
            let Some(busiest) =
                gpu.adapters.iter().rev().max_by_key(|adapter| {
                    (adapter.utilization_basis_points, adapter.name.is_some())
                })
            else {
                return Tile::missing(locale, "gpu", "gpu", label, "unavailable", "no_instances");
            };
            let name = busiest.name.clone().unwrap_or_else(|| {
                copy::pick(locale, "Adapter name not reported", "未报告适配器名称").to_string()
            });
            let count = gpu.adapters.len();
            let detail = match locale {
                Locale::En if count > 1 => format!("{name} · busiest of {count} adapters"),
                Locale::ZhCn if count > 1 => format!("{name} · {count} 个适配器中最忙"),
                _ => name,
            };
            Tile {
                id: "gpu",
                icon: "gpu",
                label,
                figure: format!("{:.0}%", percent(busiest.utilization_basis_points)),
                style: Figure::Reading,
                partial,
                visual: Visual::Trend(TrendMetric::Gpu),
                detail,
            }
        }
        Reading::Missing(state, reason) => {
            Tile::missing(locale, "gpu", "gpu", label, state, &reason)
        }
    }
}

fn memory_tile(locale: Locale, snapshot: &StatusSnapshotV1) -> Tile {
    let label = copy::pick(locale, "MEMORY", "内存");
    match reading(&snapshot.memory) {
        Reading::Value(memory, partial) if memory.total_bytes > 0 => {
            let used = memory.used_bytes as f64 * 100.0 / memory.total_bytes as f64;
            let commit = match (memory.commit_used_bytes, memory.commit_limit_bytes) {
                (Some(used), Some(limit)) => match locale {
                    Locale::En => format!("commit {} of {}", ui::bytes(used), ui::bytes(limit)),
                    Locale::ZhCn => format!("提交 {} / {}", ui::bytes(used), ui::bytes(limit)),
                },
                _ => copy::pick(locale, "commit not reported", "未报告提交内存").to_string(),
            };
            let detail = match locale {
                Locale::En => format!(
                    "{} of {} · {commit}",
                    ui::bytes(memory.used_bytes),
                    ui::bytes(memory.total_bytes)
                ),
                Locale::ZhCn => format!(
                    "已用 {} / 共 {} · {commit}",
                    ui::bytes(memory.used_bytes),
                    ui::bytes(memory.total_bytes)
                ),
            };
            Tile {
                id: "memory",
                icon: "memory",
                label,
                figure: format!("{used:.0}%"),
                style: Figure::Reading,
                partial,
                visual: Visual::Trend(TrendMetric::Memory),
                detail,
            }
        }
        Reading::Value(_, _) => Tile::missing(
            locale,
            "memory",
            "memory",
            label,
            "unavailable",
            "inconsistent_counters",
        ),
        Reading::Missing(state, reason) => {
            Tile::missing(locale, "memory", "memory", label, state, &reason)
        }
    }
}

/// Battery health, cycle count and charge rate, each with its own gap.
fn battery_facts(locale: Locale, power: &PowerV1) -> Vec<String> {
    let health = power.full_charge_basis_points.map_or_else(
        || copy::pick(locale, "health not reported", "未报告健康度").to_string(),
        |points| match locale {
            Locale::En => format!("health {:.0}%", percent(points)),
            Locale::ZhCn => format!("健康度 {:.0}%", percent(points)),
        },
    );
    let cycles = power.cycle_count.map_or_else(
        || copy::pick(locale, "cycle count not reported", "未报告循环次数").to_string(),
        |count| match locale {
            Locale::En => format!("{count} cycles"),
            Locale::ZhCn => format!("循环 {count} 次"),
        },
    );
    let rate = power.rate_milliwatts.map_or_else(
        || copy::pick(locale, "charge rate not reported", "未报告充放电功率").to_string(),
        |milliwatts| {
            let watts = f64::from(milliwatts.unsigned_abs()) / 1_000.0;
            match (locale, milliwatts.signum()) {
                (Locale::En, -1) => format!("discharging {watts:.1} W"),
                (Locale::En, 1) => format!("charging {watts:.1} W"),
                (Locale::En, _) => format!("rate {watts:.1} W"),
                (Locale::ZhCn, -1) => format!("放电 {watts:.1} W"),
                (Locale::ZhCn, 1) => format!("充电 {watts:.1} W"),
                (Locale::ZhCn, _) => format!("功率 {watts:.1} W"),
            }
        },
    );
    vec![health, cycles, rate]
}

fn power_tile(t: &Theme, locale: Locale, snapshot: &StatusSnapshotV1) -> Tile {
    let label = copy::pick(locale, "POWER", "电源");
    match reading(&snapshot.power) {
        Reading::Value(power, partial) => {
            let ac = match power.ac_state {
                AcState::Online => copy::pick(locale, "Plugged in", "已接通电源"),
                AcState::Offline => copy::pick(locale, "On battery", "使用电池"),
                AcState::Unknown => copy::pick(locale, "AC state unknown", "电源状态未知"),
            };
            if !power.battery_present {
                return Tile {
                    id: "power",
                    icon: "battery",
                    label,
                    figure: copy::pick(locale, "No battery", "无电池").to_string(),
                    style: Figure::Plain,
                    partial,
                    visual: Visual::None,
                    detail: format!("{ac} · {}", copy::status_reason(locale, "battery_absent")),
                };
            }
            let mut facts = vec![match power.remaining_seconds {
                Some(seconds) => match locale {
                    Locale::En => {
                        format!(
                            "{ac}, {}h {:02}m left",
                            seconds / 3_600,
                            (seconds % 3_600) / 60
                        )
                    }
                    Locale::ZhCn => format!(
                        "{ac}，剩余 {} 小时 {:02} 分",
                        seconds / 3_600,
                        (seconds % 3_600) / 60
                    ),
                },
                None => ac.to_string(),
            }];
            facts.extend(battery_facts(locale, power));
            let detail = facts.join(" · ");
            match power.charge_basis_points {
                Some(charge) => {
                    let fraction = percent(charge) / 100.0;
                    let color = if fraction < 0.15 {
                        t.danger
                    } else if fraction < 0.30 {
                        t.warning
                    } else {
                        t.input
                    };
                    Tile {
                        id: "power",
                        icon: "battery",
                        label,
                        figure: format!("{:.0}%", percent(charge)),
                        style: Figure::Reading,
                        partial,
                        visual: Visual::Bar(fraction, color),
                        detail,
                    }
                }
                None => Tile {
                    id: "power",
                    icon: "battery",
                    label,
                    figure: copy::pick(locale, "Charge not reported", "未报告电量").to_string(),
                    style: Figure::Missing,
                    partial,
                    visual: Visual::None,
                    detail,
                },
            }
        }
        Reading::Missing(state, reason) => {
            Tile::missing(locale, "power", "battery", label, state, &reason)
        }
    }
}

fn free_space_tile(t: &Theme, locale: Locale, snapshot: &StatusSnapshotV1) -> Tile {
    let label = copy::pick(locale, "FREE SPACE", "可用空间");
    match reading(&snapshot.volumes) {
        Reading::Value(volumes, partial) => {
            let system = std::env::var("SystemDrive")
                .unwrap_or_else(|_| "C:".to_string())
                .to_ascii_uppercase();
            let Some(volume) = volumes
                .items
                .iter()
                .find(|volume| {
                    volume
                        .mount_points
                        .iter()
                        .any(|mount| mount.to_ascii_uppercase().starts_with(&system))
                })
                .or_else(|| volumes.items.first())
            else {
                return Tile::missing(
                    locale,
                    "free",
                    "drive",
                    label,
                    "unavailable",
                    "no_instances",
                );
            };
            let used = volume.total_bytes.saturating_sub(volume.available_bytes);
            let fraction = used as f32 / volume.total_bytes.max(1) as f32;
            let color = if fraction >= 0.9 {
                t.danger
            } else if fraction >= 0.75 {
                t.warning
            } else {
                t.input
            };
            let name = volume.mount_points.first().map_or_else(
                || volume.volume_id.clone(),
                |mount| ui::display_path_text(mount.trim_end_matches(['\\', '/'])),
            );
            let count = volumes.items.len();
            let total = ui::bytes(volume.total_bytes);
            Tile {
                id: "free",
                icon: "drive",
                label,
                figure: ui::bytes(volume.available_bytes),
                style: Figure::Reading,
                partial,
                visual: Visual::Bar(fraction, color),
                detail: match locale {
                    Locale::En if count > 1 => {
                        format!("free of {total} on {name} · {count} volumes")
                    }
                    Locale::En => format!("free of {total} on {name}"),
                    Locale::ZhCn if count > 1 => format!("{name} 共 {total} · {count} 个卷"),
                    Locale::ZhCn => format!("{name} 共 {total}"),
                },
            }
        }
        Reading::Missing(state, reason) => {
            Tile::missing(locale, "free", "drive", label, state, &reason)
        }
    }
}

fn network_tile(locale: Locale, snapshot: &StatusSnapshotV1) -> Tile {
    let label = copy::pick(locale, "NETWORK", "网络");
    match reading(&snapshot.network) {
        Reading::Value(network, partial) => {
            // Rates and link type come from the same interfaces: the hardware
            // adapters when any is listed, so filter-driver copies of one
            // adapter's traffic are not counted again.
            let traffic = traffic_interfaces(network);
            let Some(busiest) = traffic.iter().copied().rev().max_by_key(|item| {
                item.rx_bytes_per_second
                    .saturating_add(item.tx_bytes_per_second)
            }) else {
                return Tile::missing(
                    locale,
                    "network",
                    "network",
                    label,
                    "unavailable",
                    "no_instances",
                );
            };
            let (rx, tx) = traffic_totals(network);
            let kind = match busiest.kind {
                Some(NetworkKind::Ethernet) => copy::pick(locale, "Ethernet", "以太网"),
                Some(NetworkKind::Wireless) => copy::pick(locale, "Wi-Fi", "无线网络"),
                Some(NetworkKind::Other) => copy::pick(locale, "other link", "其他连接"),
                None => copy::pick(locale, "link type not reported", "未报告连接类型"),
            };
            Tile {
                id: "network",
                icon: "network",
                label,
                figure: per_second(rx.saturating_add(tx)),
                style: Figure::Reading,
                partial,
                visual: Visual::Trend(TrendMetric::Network),
                detail: format!("↓ {} · ↑ {} · {kind}", per_second(rx), per_second(tx)),
            }
        }
        Reading::Missing(state, reason) => {
            Tile::missing(locale, "network", "network", label, state, &reason)
        }
    }
}

fn disk_activity_tile(locale: Locale, snapshot: &StatusSnapshotV1) -> Tile {
    let label = copy::pick(locale, "DISK ACTIVITY", "磁盘活动");
    match reading(&snapshot.disk_activity) {
        Reading::Value(disk, partial) => {
            let read = per_second(disk.read_bytes_per_second);
            let write = per_second(disk.write_bytes_per_second);
            Tile {
                id: "disk",
                icon: "disk",
                label,
                figure: per_second(
                    disk.read_bytes_per_second
                        .saturating_add(disk.write_bytes_per_second),
                ),
                style: Figure::Reading,
                partial,
                visual: Visual::Trend(TrendMetric::DiskActivity),
                detail: match locale {
                    Locale::En => format!("read {read} · write {write}"),
                    Locale::ZhCn => format!("读取 {read} · 写入 {write}"),
                },
            }
        }
        Reading::Missing(state, reason) => {
            Tile::missing(locale, "disk", "disk", label, state, &reason)
        }
    }
}

/// The eight tiles in grid order: health first, then CPU, GPU, memory, power,
/// free space, network and disk activity.
#[cfg(windows)]
pub(super) struct TrayReadings {
    pub(super) health: String,
    pub(super) machine: Vec<String>,
    pub(super) metrics: [(&'static str, String); 4],
}

#[cfg(windows)]
pub(super) fn tray_readings(
    t: &Theme,
    locale: Locale,
    snapshot: &StatusSnapshotV1,
) -> TrayReadings {
    TrayReadings {
        health: health_tile(locale, snapshot).figure,
        machine: identity_lines(locale, snapshot),
        metrics: [
            cpu_tile(locale, snapshot),
            gpu_tile(locale, snapshot),
            memory_tile(locale, snapshot),
            power_tile(t, locale, snapshot),
        ]
        .map(|tile| (tile.label, tile.figure)),
    }
}

fn status_tiles(t: &Theme, locale: Locale, snapshot: &StatusSnapshotV1) -> Vec<Tile> {
    vec![
        health_tile(locale, snapshot),
        cpu_tile(locale, snapshot),
        gpu_tile(locale, snapshot),
        memory_tile(locale, snapshot),
        power_tile(t, locale, snapshot),
        free_space_tile(t, locale, snapshot),
        network_tile(locale, snapshot),
        disk_activity_tile(locale, snapshot),
    ]
}

/// A one-minute trend as a polyline. It is painted when the view renders,
/// which happens when a sample arrives; it never asks for an animation frame.
fn trend_line(t: &Theme, metric: TrendMetric, trend: &Trend) -> gpui::AnyElement {
    let samples = trend.samples();
    let ceiling = metric.ceiling(trend);
    let line = rgb(t.output);
    let base = t.control_fill.color();
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let width = f32::from(bounds.size.width);
            let height = f32::from(bounds.size.height);
            let floor = bounds.origin.y + bounds.size.height - px(0.5);
            let mut baseline = PathBuilder::stroke(px(1.0));
            baseline.move_to(point(bounds.origin.x, floor));
            baseline.line_to(point(bounds.origin.x + bounds.size.width, floor));
            if let Ok(path) = baseline.build() {
                window.paint_path(path, base);
            }
            for segment in trend_segments(&samples, ceiling, width, height) {
                let mut builder = PathBuilder::stroke(px(1.5));
                for (index, (x, y)) in segment.into_iter().enumerate() {
                    let at = point(bounds.origin.x + px(x), bounds.origin.y + px(y));
                    if index == 0 {
                        builder.move_to(at);
                    } else {
                        builder.line_to(at);
                    }
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, line);
                }
            }
        },
    )
    .w_full()
    .h(rems(2.5))
    .into_any_element()
}

/// Tiles per grid row: four, or two when the window is narrow.
fn grid_columns(window: &Window) -> usize {
    if window.viewport_size().width < window.rem_size() * 52.0 {
        2
    } else {
        4
    }
}

impl ShellView {
    pub(super) fn status_page(
        &mut self,
        t: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let live = self.session.status_phase() == StatusPhase::Live;
        let fault = self.session.status_live_fault();
        let samples = self
            .session
            .status_trends()
            .map_or(0, |trends| trends.len());
        let state = if live {
            if self.session.status_snapshot().is_some() {
                StatusLiveState::Live
            } else {
                StatusLiveState::Starting
            }
        } else {
            match fault {
                Some(StatusLiveFault::StartFailed) => StatusLiveState::StartFailed,
                Some(StatusLiveFault::Ended) => StatusLiveState::Ended,
                None if self.session.status_sampling() => StatusLiveState::Snapshot,
                None => StatusLiveState::Paused,
            }
        };
        let mut actions = Vec::new();
        actions.push(ui::any(
            ui::button(
                t,
                "status-keep-awake",
                copy::pick(locale, "Keep awake", "保持唤醒"),
                if self.session.host_action_busy() {
                    ButtonKind::Disabled
                } else {
                    ButtonKind::Secondary
                },
            )
            .accessibility_id("status-keep-awake")
            .on_click(cx.listener(|this, _, _, cx| {
                this.session.keep_awake_preview_toggle();
                cx.notify();
            }))
            .on_a11y_action(
                gpui::AccessibleAction::Click,
                on_view_a11y(cx, |this, cx| {
                    this.session.keep_awake_preview_toggle();
                    cx.notify();
                }),
            ),
        ));
        if live {
            actions.push(ui::any(
                ui::icon_button(
                    t,
                    "status-pause",
                    "pause",
                    copy::status_action_pause(locale),
                    ButtonKind::Secondary,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.status_pause();
                    cx.notify();
                })),
            ));
        } else {
            actions.push(ui::any(
                ui::icon_button(
                    t,
                    "status-resume",
                    "play",
                    copy::status_action_resume(locale),
                    ButtonKind::Primary,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.status_resume();
                    if let Some(generation) = this.session.take_status_poll_request() {
                        this.start_status_poll(generation, cx);
                    }
                    cx.notify();
                })),
            ));
            if fault.is_none() {
                actions.push(ui::any(
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
                ));
            }
        }
        let bar = page_bar(
            t,
            Some(copy::status_live_line(
                locale,
                state,
                STATUS_LIVE_INTERVAL_MS / 1_000,
                samples,
                TREND_CAPACITY,
            )),
            None,
            actions,
        );
        let mut page = div()
            .flex()
            .flex_col()
            .gap(ui::SPACE_4)
            .children(self.shared_banners(t));
        page = page.child(ui::muted(t, self.session.keep_awake_line()));
        if let Some(connection) = self.session.keep_awake_connection() {
            page = page.child(ui::muted(
                t,
                format!(
                    "{}: status keep-awake status --endpoint {} --token {} --owner {}",
                    copy::pick(locale, "CLI session query", "CLI 会话查询"),
                    connection.endpoint,
                    connection.token,
                    connection.owner
                ),
            ));
        }
        if fault.is_some() {
            page = page.child(ui::banner(
                t,
                Tone::Warning,
                copy::pick(
                    locale,
                    "Live sampling is not running. Resume to start it again; the last readings stay on screen.",
                    "实时采样没有在运行。点“继续”重新开始；最后的读数仍会显示。",
                ),
            ));
        }
        // The raw sensor line stays in the accessible text projection; the page
        // says it in words, and each tile names its own reason.
        if self.session.status_sensor_line().is_some() {
            page = page.child(ui::banner(
                t,
                Tone::Info,
                copy::pick(
                    locale,
                    "Some sensors did not report. Their tiles say why; nothing is shown as zero.",
                    "部分传感器没有上报。对应格子会说明原因，不会显示为零。",
                ),
            ));
        }
        let Some(snapshot) = self.session.status_snapshot() else {
            let error = self
                .session
                .status_lines()
                .iter()
                .find(|line| line.starts_with("unavailable"))
                .cloned();
            let waiting = matches!(state, StatusLiveState::Starting | StatusLiveState::Snapshot);
            let mut empty = ui::card(t).items_center().gap(ui::SPACE_2).child(ui::empty_state(
                t,
                "status",
                copy::status_empty_title(locale),
                error.unwrap_or_else(|| {
                    if waiting {
                        copy::pick(
                            locale,
                            "Reading CPU, GPU, memory, power, disks, network and the busiest processes. The first reading takes about a second.",
                            "正在读取 CPU、GPU、内存、电源、磁盘、网络和最繁忙的进程。第一次读数大约需要一秒。",
                        )
                    } else {
                        copy::pick(
                            locale,
                            "Sampling is paused. Resume to read CPU, GPU, memory, power, disks, network and the busiest processes.",
                            "采样已暂停。点“继续”读取 CPU、GPU、内存、电源、磁盘、网络和最繁忙的进程。",
                        )
                    }
                    .to_string()
                }),
            ));
            if waiting {
                empty = empty.child(ui::progress_track(t));
            }
            let body = page.child(empty).into_any_element();
            return sheet_frame(bar, body, self.host_review_bar(t, cx));
        };
        let snapshot = snapshot.clone();
        let body = page
            .child(self.status_grid(t, &snapshot, grid_columns(window)))
            .child(self.status_processes(t, &snapshot, cx))
            .child(self.status_volumes(t, &snapshot))
            .into_any_element();
        sheet_frame(bar, body, self.host_review_bar(t, cx))
    }

    /// Eight tiles in rows of `columns`.
    fn status_grid(
        &self,
        t: &Theme,
        snapshot: &StatusSnapshotV1,
        columns: usize,
    ) -> gpui::AnyElement {
        let locale = self.session.locale();
        let mut tiles: Vec<gpui::AnyElement> = status_tiles(t, locale, snapshot)
            .into_iter()
            .map(|tile| self.status_tile(t, tile))
            .collect();
        let mut rows = Vec::new();
        while !tiles.is_empty() {
            let rest = tiles.split_off(columns.min(tiles.len()));
            rows.push(
                div()
                    .flex()
                    .flex_row()
                    .gap(ui::SPACE_2)
                    .children(std::mem::replace(&mut tiles, rest))
                    .into_any_element(),
            );
        }
        div()
            .id("status-tiles")
            .flex()
            .flex_col()
            .gap(ui::SPACE_2)
            .children(rows)
            .into_any_element()
    }

    fn status_tile(&self, t: &Theme, tile: Tile) -> gpui::AnyElement {
        let locale = self.session.locale();
        let mut aria = format!("{} {} {}", tile.label, tile.figure, tile.detail);
        let visual = match tile.visual {
            Visual::Trend(metric) => {
                let trend = self
                    .session
                    .status_trends()
                    .map(|trends| trends.get(metric).clone())
                    .unwrap_or_default();
                Some(trend_line(t, metric, &trend))
            }
            Visual::Bar(fraction, color) => Some(ui::any(ui::meter(t, fraction, color))),
            Visual::Lines(lines) => {
                aria.push(' ');
                aria.push_str(&lines.join(" "));
                Some(
                    div()
                        .flex()
                        .flex_col()
                        .children(lines.into_iter().map(|line| {
                            div()
                                .truncate()
                                .text_size(ui::TYPE_META)
                                .text_color(rgb(t.ink))
                                .child(line)
                        }))
                        .into_any_element(),
                )
            }
            Visual::None => None,
        };
        let figure = div().truncate().child(tile.figure);
        let figure = match tile.style {
            Figure::Missing => figure
                .text_size(ui::TYPE_TITLE)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(t.muted)),
            Figure::Reading | Figure::Plain => figure
                .text_size(ui::TYPE_DISPLAY)
                .font_weight(FontWeight::BOLD)
                .font_features(ui::tabular_numerals())
                .text_color(rgb(if tile.style == Figure::Reading {
                    t.output
                } else {
                    t.ink
                })),
        };
        div()
            .id(ElementId::Name(SharedString::from(format!(
                "status-tile-{}",
                tile.id
            ))))
            .aria_label(SharedString::from(aria))
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(ui::SPACE_1)
            .p(ui::SPACE_3)
            .rounded(ui::RADIUS_ITEM)
            .bg(t.well.color())
            .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.ink)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui::SPACE_1)
                    .child(ui::icon(tile.icon, t.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(ui::eyebrow(t, tile.label)),
                    )
                    .when(tile.partial, |d| {
                        d.child(ui::badge(
                            t,
                            copy::pick(locale, "partial", "部分"),
                            Tone::Warning,
                        ))
                    }),
            )
            .child(figure)
            .children(visual)
            .child(
                div()
                    .text_size(ui::TYPE_META)
                    .text_color(rgb(t.muted))
                    .child(tile.detail),
            )
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
                        t.input
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
                        .gap(ui::SPACE_2)
                        .child(ui::icon("drive", t.muted))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap(ui::SPACE_1)
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .justify_between()
                                        .gap(ui::SPACE_1)
                                        .child(
                                            div().truncate().text_size(ui::TYPE_BODY).child(name),
                                        )
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
                    .gap(ui::SPACE_3)
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
            .gap(ui::SPACE_3)
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
                .text_size(ui::TYPE_META)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if active { t.ink } else { t.muted }));
            match width {
                Some(w) => cell.w(rems(w)).flex_shrink_0().text_right().child(label),
                None => cell.flex_1().child(label),
            }
        };
        let table_header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(ui::SPACE_2)
            .px(ui::SPACE_2)
            .pb(ui::SPACE_1)
            .child(header_cell(
                copy::pick(locale, "PROCESS", "进程"),
                sort == ProcessSort::Name,
                None,
            ))
            .child(header_cell(
                copy::pick(locale, "MEMORY", "内存"),
                sort == ProcessSort::Memory,
                Some(6.5),
            ))
            .child(header_cell("CPU", sort == ProcessSort::Cpu, Some(5.0)))
            .child(header_cell("PID", sort == ProcessSort::Pid, Some(5.0)))
            .child(header_cell(
                copy::pick(locale, "DISK READ", "磁盘读取"),
                false,
                Some(6.5),
            ))
            .child(header_cell(
                copy::pick(locale, "DISK WRITE", "磁盘写入"),
                false,
                Some(6.5),
            ));
        let max_cpu = rows
            .iter()
            .map(|row| row.cpu_basis_points_of_one_logical_core)
            .max()
            .unwrap_or(1)
            .max(1);
        let picked = self.session.status_picked_process().cloned();
        let mut detail = self.status_process_detail(t, snapshot, cx);
        let mut body = Vec::new();
        for row in rows.iter().take(100) {
            let cpu = percent(row.cpu_basis_points_of_one_logical_core);
            let selected = picked
                .as_ref()
                .is_some_and(|pick| pick.pid == row.pid && pick.name == row.name);
            let clicked = row.clone();
            let toggled = clicked.clone();
            // Role::Row is a UIA DataItem. aria-selected suppresses Invoke,
            // and DataItem does not expose SelectionItem, so the row would
            // have no action that opens this explanation.
            let element = div()
                .id(gpui::ElementId::named_usize(
                    "status-process",
                    row.pid as usize,
                ))
                .accessibility_id(SharedString::from(format!("status-process-{}", row.pid)))
                .role(gpui::Role::Row)
                .aria_label(SharedString::from(format!(
                    "{} {} cpu {cpu:.1}% pid {} read {} write {}",
                    row.name,
                    ui::bytes(row.private_bytes),
                    row.pid,
                    per_second(row.read_bytes_per_second),
                    per_second(row.write_bytes_per_second)
                )))
                .border_1()
                .border_color(ui::NO_EDGE)
                .when(selected, |row| row.bg(rgb(t.input_wash)))
                .cursor_pointer()
                .keyboard_focus(t)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.session.status_toggle_process(&clicked);
                    cx.notify();
                }))
                .on_a11y_action(
                    gpui::AccessibleAction::Click,
                    on_view_a11y(cx, move |this, cx| {
                        this.session.status_toggle_process(&toggled);
                        cx.notify();
                    }),
                )
                .flex()
                .flex_row()
                .items_center()
                .gap(ui::SPACE_2)
                .px(ui::SPACE_2)
                .py(ui::SPACE_1)
                .rounded(ui::RADIUS_ITEM)
                .when(!selected, |row| row.hover(|s| s.bg(t.control_fill.color())))
                .font_features(ui::tabular_numerals())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_size(ui::TYPE_BODY)
                                .child(row.name.clone()),
                        )
                        .child(ui::meter(
                            t,
                            row.cpu_basis_points_of_one_logical_core as f32 / max_cpu as f32,
                            t.input,
                        )),
                )
                .child(
                    div()
                        .w(rems(6.5))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(ui::TYPE_BODY)
                        .child(ui::bytes(row.private_bytes)),
                )
                .child(
                    div()
                        .w(rems(5.0))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(ui::TYPE_BODY)
                        .child(format!("{cpu:.1}%")),
                )
                .child(
                    div()
                        .w(rems(5.0))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(ui::TYPE_META)
                        .text_color(rgb(t.muted))
                        .child(row.pid.to_string()),
                )
                .child(
                    div()
                        .w(rems(6.5))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(ui::TYPE_META)
                        .text_color(rgb(t.muted))
                        .child(per_second(row.read_bytes_per_second)),
                )
                .child(
                    div()
                        .w(rems(6.5))
                        .flex_shrink_0()
                        .text_right()
                        .text_size(ui::TYPE_META)
                        .text_color(rgb(t.muted))
                        .child(per_second(row.write_bytes_per_second)),
                );
            body.push(element.into_any_element());
            // The explanation opens under its row so the table does not jump.
            if selected && let Some(panel) = detail.take() {
                body.push(panel);
            }
        }
        // A pick that is filtered out or gone from the sample still explains
        // itself, above the table.
        let orphan = detail.take();
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
            .gap(ui::SPACE_2)
            .child(
                ui::card_header(t, copy::pick(locale, "Processes", "进程"))
                    .flex_wrap()
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(ui::SPACE_1)
                            .w(rems(16.0))
                            .max_w_full()
                            .child(ui::icon("search", t.muted))
                            .child(div().flex_1().min_w_0().child(self.search.clone())),
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
                    "Choose a process to inspect it. Ending it requires a fresh identity check and confirmation.",
                    "选择进程查看详情，结束进程前必须重新检查身份并确认。",
                ),
            ))
            .children(
                crate::status_panel::process_count_line(locale, snapshot, rows.len())
                    .map(|line| ui::muted(t, line)),
            )
            .children(orphan)
            .child(
                div()
                    .id("status-process-columns")
                    .w_full()
                    .overflow_x_scroll()
                    .child(
                        div()
                            .min_w(rems(46.0))
                            .flex()
                            .flex_col()
                            .child(table_header)
                            .child(ui::divider(t))
                            .children(body),
                    ),
            )
            .into_any_element()
    }

    /// Local explanation and an entry into the separate reviewed action.
    fn status_process_detail(
        &self,
        t: &Theme,
        snapshot: &StatusSnapshotV1,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let locale = self.session.locale();
        let pick = self.session.status_picked_process()?;
        let close = ui::button(
            t,
            "status-process-close",
            copy::pick(locale, "Close", "关闭"),
            ButtonKind::Ghost,
        )
        .on_click(cx.listener(|this, _, _, cx| {
            this.session.status_clear_process();
            cx.notify();
        }));
        let panel = div()
            .id("status-process-detail")
            .role(gpui::Role::Region)
            .aria_label(SharedString::from(format!(
                "{} pid {}",
                pick.name, pick.pid
            )))
            .flex()
            .flex_col()
            .gap(ui::SPACE_1)
            .p(ui::SPACE_3)
            .rounded(ui::RADIUS_ITEM)
            .bg(t.control_fill.color())
            .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.ink)));
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(ui::SPACE_1)
            .child(ui::icon("info", t.input))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("{} · PID {}", pick.name, pick.pid)),
            )
            .child(close);
        let lines = match pick.find(snapshot) {
            Some(row) => crate::status_panel::process_explanation(locale, &row, snapshot),
            None => vec![
                copy::pick(
                    locale,
                    "This process is no longer in the latest sample. It may have ended, or the list was truncated.",
                    "最新采样中已没有这个进程。它可能已结束，或列表被截断。",
                )
                .to_string(),
            ],
        };
        let error = self.session.status_process_action_error();
        let can_review = error.is_none() && !self.session.host_action_busy();
        let action = ui::button(
            t,
            "status-process-review",
            copy::pick(locale, "Review ending process", "审阅结束进程"),
            if can_review {
                ButtonKind::Danger
            } else {
                ButtonKind::Disabled
            },
        )
        .accessibility_id("status-process-review")
        .when(can_review, |button| {
            button
                .on_click(cx.listener(|this, _, _, cx| {
                    this.session.status_preview_process_termination();
                    cx.notify();
                }))
                .on_a11y_action(
                    gpui::AccessibleAction::Click,
                    on_view_a11y(cx, |this, cx| {
                        this.session.status_preview_process_termination();
                        cx.notify();
                    }),
                )
        });
        Some(
            panel
                .child(header)
                .child(action)
                .children(
                    error.map(|code| ui::muted(t, crate::host_review::result_label(locale, code))),
                )
                .children(
                    lines
                        .into_iter()
                        .map(|line| ui::muted(t, line).into_any_element()),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> StatusSnapshotV1 {
        serde_json::from_str(include_str!(
            "../../../crates/devsweep-cli/tests/fixtures/status/snapshot.json"
        ))
        .expect("status fixture decodes")
    }

    fn zero_like(text: &str) -> bool {
        text.split(|c: char| c.is_whitespace() || matches!(c, '·' | ',' | '，'))
            .any(|token| matches!(token, "0" | "0%" | "0.0" | "0.0%" | "0B"))
    }

    fn english_words(text: &str) -> bool {
        text.replace("Windows", "")
            .chars()
            .any(|c| c.is_ascii_lowercase())
    }

    #[test]
    fn tiles_follow_the_grid_order_and_name_the_new_readings() {
        let snapshot = fixture();
        let tiles = status_tiles(&ui::LIGHT, Locale::En, &snapshot);
        let ids: Vec<&str> = tiles.iter().map(|tile| tile.id).collect();
        assert_eq!(
            ids,
            [
                "health", "cpu", "gpu", "memory", "power", "free", "network", "disk"
            ]
        );
        let health = &tiles[0];
        assert_eq!(health.figure, "CPU not busy", "evidence, not a score");
        assert!(health.detail.contains("No score"));
        let Visual::Lines(identity) = &health.visual else {
            panic!("health tile carries the machine identity");
        };
        assert_eq!(identity[0], "Fixture CPU");
        assert_eq!(identity[1], "Windows 10.0.26300.9974 (26H2)");
        assert_eq!(identity[2], "Up 1 d 0 h");
        assert_eq!(tiles[1].figure, "43%");
        assert!(tiles[1].detail.contains("4.52 GHz"), "{}", tiles[1].detail);
        assert!(tiles[2].detail.contains("Fixture GPU"));
        assert!(tiles[3].detail.contains("commit 20.0 GB of 40.0 GB"));
        assert!(tiles[3].partial);
        assert!(tiles[6].detail.contains("Ethernet"));
        assert_eq!(tiles[7].figure, "12.0 MB/s");
        assert!(matches!(tiles[1].visual, Visual::Trend(TrendMetric::Cpu)));
        assert!(matches!(
            tiles[7].visual,
            Visual::Trend(TrendMetric::DiskActivity)
        ));
    }

    /// AC6: no battery and no temperature sensor read as reasons, never as
    /// 0% or a blank tile, in both locales.
    #[test]
    fn missing_battery_and_sensors_show_their_reason_not_zero() {
        for locale in [Locale::En, Locale::ZhCn] {
            let snapshot = fixture();
            let tiles = status_tiles(&ui::DARK, locale, &snapshot);
            let power = &tiles[4];
            assert_eq!(
                power.figure,
                copy::pick(locale, "No battery", "无电池"),
                "{locale:?}"
            );
            assert!(
                power
                    .detail
                    .contains(&copy::status_reason(locale, "battery_absent"))
            );
            assert!(!tiles[1].detail.contains("°C"), "no thermal zone, no °C");
            for tile in &tiles {
                assert!(!tile.figure.trim().is_empty(), "{}", tile.id);
                assert!(!tile.detail.trim().is_empty(), "{}", tile.id);
                assert!(!zero_like(&tile.figure), "{}: {}", tile.id, tile.figure);
            }

            let mut missing = snapshot.clone();
            missing.host = AvailabilityV1::unavailable("api_unavailable");
            missing.cpu = AvailabilityV1::unavailable("counter_missing");
            missing.gpu = AvailabilityV1::unsupported("platform_unsupported");
            missing.memory = AvailabilityV1::permission_denied("access_denied");
            missing.power = AvailabilityV1::unavailable("api_unavailable");
            missing.volumes = AvailabilityV1::unavailable("access_denied");
            missing.network = AvailabilityV1::unavailable("pdh_unavailable");
            missing.disk_activity = AvailabilityV1::unavailable("counter_missing");
            let tiles = status_tiles(&ui::DARK, locale, &missing);
            for tile in tiles.iter().skip(1) {
                assert!(tile.style == Figure::Missing, "{}", tile.id);
                assert!(matches!(tile.visual, Visual::None), "{}", tile.id);
                assert!(!zero_like(&tile.figure), "{}: {}", tile.id, tile.figure);
                assert!(!tile.detail.trim().is_empty(), "{}", tile.id);
            }
            if locale == Locale::ZhCn {
                for tile in &tiles {
                    assert!(!english_words(&tile.figure), "{}", tile.figure);
                    assert!(!english_words(&tile.detail), "{}", tile.detail);
                }
            }

            // A battery whose firmware reports only the charge: each missing
            // field is named, and health above 100% is shown as measured.
            let mut battery = snapshot.clone();
            battery.power = AvailabilityV1::available(
                1,
                0,
                PowerV1 {
                    battery_present: true,
                    ac_state: AcState::Offline,
                    charge_basis_points: Some(8_000),
                    remaining_seconds: None,
                    full_charge_basis_points: None,
                    cycle_count: None,
                    rate_milliwatts: None,
                },
            );
            let power = power_tile(&ui::LIGHT, locale, &battery);
            assert_eq!(power.figure, "80%");
            for gap in [
                copy::pick(locale, "health not reported", "未报告健康度"),
                copy::pick(locale, "cycle count not reported", "未报告循环次数"),
                copy::pick(locale, "charge rate not reported", "未报告充放电功率"),
            ] {
                assert!(power.detail.contains(gap), "{}", power.detail);
            }
            if let AvailabilityV1::Available { value, .. } = &mut battery.power {
                value.full_charge_basis_points = Some(10_230);
                value.cycle_count = Some(312);
                value.rate_milliwatts = Some(-8_200);
            }
            let power = power_tile(&ui::LIGHT, locale, &battery);
            let expected = match locale {
                Locale::En => ["health 102%", "312 cycles", "discharging 8.2 W"],
                Locale::ZhCn => ["健康度 102%", "循环 312 次", "放电 8.2 W"],
            };
            for fact in expected {
                assert!(power.detail.contains(fact), "{}", power.detail);
            }
        }
    }

    /// The network figure, its trend and its link type describe the same
    /// interfaces; the GPU figure and name describe the same adapter.
    #[test]
    fn network_and_gpu_tiles_describe_the_interfaces_and_adapter_they_measure() {
        use devsweep_core::status::{GpuAdapterV1, GpuV1, NetworkInterfaceV1, NetworkV1};

        fn iface(name: &str, kind: NetworkKind, rx: u64, tx: u64) -> NetworkInterfaceV1 {
            NetworkInterfaceV1 {
                interface_luid: name.to_string(),
                name: name.to_string(),
                rx_bytes_per_second: rx,
                tx_bytes_per_second: tx,
                kind: Some(kind),
            }
        }
        // Measured shape on a Wi-Fi laptop: the WLAN adapter, five filter
        // layers repeating its counters, a busier proxy tunnel and idle
        // Ethernet ports.
        let mut snapshot = fixture();
        let mut interfaces = vec![
            iface("Ethernet", NetworkKind::Ethernet, 0, 0),
            iface("Tunnel", NetworkKind::Other, 1_000_000, 1_000_000),
        ];
        for layer in 0..5 {
            interfaces.push(iface(
                &format!("WLAN-Filter-{layer}"),
                NetworkKind::Other,
                250_000,
                500_000,
            ));
        }
        interfaces.push(iface("WLAN", NetworkKind::Wireless, 250_000, 500_000));
        snapshot.network = AvailabilityV1::available(
            1,
            0,
            NetworkV1 {
                interval_ms: 1_000,
                interfaces,
            },
        );
        let tile = network_tile(Locale::En, &snapshot);
        assert_eq!(tile.figure, per_second(750_000), "counted once");
        assert!(tile.detail.ends_with("Wi-Fi"), "{}", tile.detail);
        assert_eq!(TrendMetric::Network.reading(&snapshot), Some(750_000.0));

        // Without a hardware adapter every interface counts.
        let AvailabilityV1::Available { value, .. } = &mut snapshot.network else {
            unreachable!()
        };
        value
            .interfaces
            .retain(|item| item.kind == Some(NetworkKind::Other));
        let tile = network_tile(Locale::ZhCn, &snapshot);
        assert!(tile.detail.ends_with("其他连接"), "{}", tile.detail);
        assert_eq!(TrendMetric::Network.reading(&snapshot), Some(5_750_000.0));

        let adapter = |id: &str, points: u32, name: Option<&str>| GpuAdapterV1 {
            adapter_id: id.to_string(),
            utilization_basis_points: points,
            name: name.map(str::to_string),
        };
        snapshot.gpu = AvailabilityV1::available(
            1,
            0,
            GpuV1 {
                adapters: vec![
                    adapter("a", 0, Some("Basic Render")),
                    adapter("b", 967, Some("Intel(R) Graphics")),
                    adapter("c", 823, Some("NVIDIA GeForce RTX")),
                    adapter("d", 967, None),
                ],
            },
        );
        let tile = gpu_tile(Locale::En, &snapshot);
        assert_eq!(tile.figure, "10%");
        assert_eq!(
            tile.detail, "Intel(R) Graphics · busiest of 4 adapters",
            "a tie goes to the named adapter"
        );
        assert_eq!(TrendMetric::Gpu.reading(&snapshot), Some(9.67));
    }
}
