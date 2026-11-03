//! Shared Status main-panel and tray subscription.
//!
//! One running sampler bit. Main and tray visibility share it. This module
//! projects `AvailabilityV1` into panel text. It does not open a sensor API.
//! A displayed PID is presentation only and is not termination authority.

use std::fmt;

use devsweep_core::status::AvailabilityV1;

/// Why a displayed PID cannot authorize a kill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayedPidError;

impl fmt::Display for DisplayedPidError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("displayed PID is not termination authority")
    }
}

impl std::error::Error for DisplayedPidError {}

/// Main/tray visibility and the single running subscription bit.
#[derive(Debug)]
pub struct StatusPanel {
    main_visible: bool,
    tray_visible: bool,
    running: bool,
    sensor_line: Option<String>,
    displayed_pid: Option<u32>,
}

impl StatusPanel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            main_visible: false,
            tray_visible: false,
            running: false,
            sensor_line: None,
            displayed_pid: None,
        }
    }

    pub fn set_main_visible(&mut self, visible: bool) {
        self.main_visible = visible;
    }

    pub fn set_tray_visible(&mut self, visible: bool) {
        self.tray_visible = visible;
    }

    /// Start only when nothing was running and at least one panel is visible.
    /// Stop only when both are hidden and it was running.
    pub fn reconcile(&mut self, start: impl FnOnce(), stop: impl FnOnce()) {
        let want = self.main_visible || self.tray_visible;
        if want {
            if !self.running {
                start();
                self.running = true;
            }
        } else if self.running {
            stop();
            self.running = false;
        }
    }

    pub fn set_sensor<T>(&mut self, availability: &AvailabilityV1<T>) {
        self.sensor_line = Some(availability_line(availability));
    }

    #[must_use]
    pub fn sensor_line(&self) -> Option<&str> {
        self.sensor_line.as_deref()
    }

    pub fn set_displayed_pid(&mut self, pid: Option<u32>) {
        self.displayed_pid = pid;
    }

    #[must_use]
    pub fn displayed_pid(&self) -> Option<u32> {
        self.displayed_pid
    }

    #[must_use]
    pub fn displayed_pid_line(&self) -> Option<String> {
        self.displayed_pid.map(|pid| format!("pid {pid}"))
    }

    /// A displayed PID is not a kill plan. This never calls Status process
    /// termination.
    pub fn request_kill_displayed_pid(&self) -> Result<(), DisplayedPidError> {
        let _pid = self.displayed_pid;
        Err(DisplayedPidError)
    }
}

impl Default for StatusPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Landing shows a snapshot. Live is the shared sampler, not a second panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusPhase {
    Landing,
    Live,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessSort {
    Name,
    Cpu,
    Memory,
    Pid,
}

/// One line per status group. Missing sensors stay unavailable.
#[must_use]
pub fn snapshot_lines(snapshot: &devsweep_core::status::StatusSnapshotV1) -> Vec<String> {
    [
        ("cpu", availability_line(&snapshot.cpu)),
        ("memory", availability_line(&snapshot.memory)),
        ("volumes", availability_line(&snapshot.volumes)),
        ("network", availability_line(&snapshot.network)),
        ("power", availability_line(&snapshot.power)),
        ("gpu", availability_line(&snapshot.gpu)),
        ("thermal", availability_line(&snapshot.thermal)),
        ("processes", availability_line(&snapshot.processes)),
    ]
    .into_iter()
    .map(|(group, line)| format!("{group} {line}"))
    .collect()
}

/// First displayed process id, if the process group has a value.
#[must_use]
pub fn displayed_pid_from_snapshot(
    snapshot: &devsweep_core::status::StatusSnapshotV1,
) -> Option<u32> {
    match &snapshot.processes {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            value.items.first().map(|item| item.pid)
        }
        AvailabilityV1::Unavailable { .. }
        | AvailabilityV1::PermissionDenied { .. }
        | AvailabilityV1::Unsupported { .. } => None,
    }
}

/// Honest availability text. Missing sensors keep their reason; they are never a `0` reading.
#[must_use]
pub fn availability_line<T>(availability: &AvailabilityV1<T>) -> String {
    match availability {
        AvailabilityV1::Unavailable { reason_code, .. } => {
            format!("unavailable {reason_code}")
        }
        AvailabilityV1::PermissionDenied { reason_code, .. } => {
            format!("permission_denied {reason_code}")
        }
        AvailabilityV1::Unsupported { reason_code, .. } => {
            format!("unsupported {reason_code}")
        }
        AvailabilityV1::Available { .. } => "available".to_string(),
        AvailabilityV1::Partial { reason_codes, .. } => {
            format!("partial {}", reason_codes.join(","))
        }
    }
}

/// Health plus the closed sensor groups. Unavailable groups are not drawn as zero.
#[must_use]
pub fn group_lines(
    snapshot: &devsweep_core::status::StatusSnapshotV1,
    locale: crate::copy::Locale,
) -> Vec<String> {
    use crate::copy::{self, Locale};
    use devsweep_core::status::{
        CpuV1, GpuV1, MemoryV1, NetworkV1, PowerV1, ProcessesV1, ThermalV1, VolumesV1,
        explain_health,
    };

    let mut lines = vec![copy::status_health_title(locale).to_string()];
    for evidence in explain_health(snapshot).evidence {
        let reason = evidence.reason_code.unwrap_or_default();
        lines.push(copy::status_health_line(
            locale,
            &evidence.subject,
            &evidence.state,
            &reason,
            &evidence.text,
        ));
    }
    lines.push(metric_line(
        locale,
        "cpu",
        &snapshot.cpu,
        |value: &CpuV1| format!("{}", value.system_utilization_basis_points / 100),
    ));
    lines.push(metric_line(
        locale,
        "memory",
        &snapshot.memory,
        |value: &MemoryV1| format!("{} B", value.used_bytes),
    ));
    lines.push(metric_line(
        locale,
        "gpu",
        &snapshot.gpu,
        |value: &GpuV1| {
            value
                .adapters
                .first()
                .map(|adapter| format!("{}", adapter.utilization_basis_points / 100))
                .unwrap_or_else(|| "unavailable".to_string())
        },
    ));
    lines.push(metric_line(
        locale,
        "power",
        &snapshot.power,
        |value: &PowerV1| {
            if value.battery_present {
                value
                    .charge_basis_points
                    .map(|points| format!("{}", points / 100))
                    .unwrap_or_else(|| "unavailable".to_string())
            } else {
                copy::status_group_unavailable(Locale::En, "power", "no_battery")
            }
        },
    ));
    lines.push(metric_line(
        locale,
        "volumes",
        &snapshot.volumes,
        |value: &VolumesV1| value.items.len().to_string(),
    ));
    lines.push(metric_line(
        locale,
        "network",
        &snapshot.network,
        |value: &NetworkV1| value.interfaces.len().to_string(),
    ));
    lines.push(metric_line(
        locale,
        "thermal",
        &snapshot.thermal,
        |value: &ThermalV1| {
            value
                .zones
                .first()
                .map(|zone| format!("{}", zone.temperature_tenths_celsius))
                .unwrap_or_else(|| "unavailable".to_string())
        },
    ));
    lines.push(metric_line(
        locale,
        "processes",
        &snapshot.processes,
        |value: &ProcessesV1| value.returned_count.to_string(),
    ));
    let _ = locale;
    lines
}

fn metric_line<T>(
    locale: crate::copy::Locale,
    group: &str,
    availability: &devsweep_core::status::AvailabilityV1<T>,
    present: impl FnOnce(&T) -> String,
) -> String {
    use crate::copy;
    use devsweep_core::status::AvailabilityV1;
    match availability {
        AvailabilityV1::Unavailable { reason_code, .. } => {
            copy::status_group_unavailable(locale, group, reason_code)
        }
        AvailabilityV1::PermissionDenied { reason_code, .. } => {
            copy::status_group_permission_denied(locale, group, reason_code)
        }
        AvailabilityV1::Unsupported { reason_code, .. } => {
            copy::status_group_unsupported(locale, group, reason_code)
        }
        AvailabilityV1::Partial {
            reason_codes,
            value,
            ..
        } => {
            let detail = present(value);
            if detail
                .split_whitespace()
                .any(|token| token == "0" || token == "0%")
                && reason_codes
                    .iter()
                    .any(|code| code.contains("missing") || code.contains("unavailable"))
            {
                return copy::status_group_unavailable(locale, group, &reason_codes.join(","));
            }
            copy::status_group_partial(locale, group, &reason_codes.join(","))
        }
        AvailabilityV1::Available { value, .. } => {
            format!("{group} {}", present(value))
        }
    }
}

/// Filtered and sorted process rows shared by the text projection and the
/// process table. A displayed PID is not a kill request.
#[must_use]
pub fn sorted_processes(
    snapshot: &devsweep_core::status::StatusSnapshotV1,
    query: &str,
    sort: ProcessSort,
) -> Vec<devsweep_core::status::ProcessV1> {
    use devsweep_core::status::AvailabilityV1;
    let (AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. }) =
        &snapshot.processes
    else {
        return Vec::new();
    };
    let mut rows = value.items.clone();
    let query = query.to_lowercase();
    if !query.is_empty() {
        rows.retain(|row| {
            row.name.to_lowercase().contains(&query) || row.pid.to_string().contains(&query)
        });
    }
    rows.sort_by(|left, right| {
        let order = match sort {
            ProcessSort::Name => left.name.cmp(&right.name),
            ProcessSort::Cpu => right
                .cpu_basis_points_of_one_logical_core
                .cmp(&left.cpu_basis_points_of_one_logical_core),
            ProcessSort::Memory => right.private_bytes.cmp(&left.private_bytes),
            ProcessSort::Pid => left.pid.cmp(&right.pid),
        };
        order.then_with(|| left.pid.cmp(&right.pid))
    });
    rows
}

/// Searchable, sortable process rows. A displayed PID is not a kill request.
#[must_use]
pub fn process_lines(
    snapshot: &devsweep_core::status::StatusSnapshotV1,
    query: &str,
    sort: ProcessSort,
) -> Vec<String> {
    let rows = sorted_processes(snapshot, query, sort);
    rows.iter()
        .map(|row| {
            format!(
                "{} pid {}: cpu {} memory {}",
                row.name, row.pid, row.cpu_basis_points_of_one_logical_core, row.private_bytes
            )
        })
        .collect()
}

/// How many rows the table shows against what the sample enumerated. `None`
/// when the process group has no value.
#[must_use]
pub fn process_count_line(
    locale: crate::copy::Locale,
    snapshot: &devsweep_core::status::StatusSnapshotV1,
    visible: usize,
) -> Option<String> {
    use crate::copy::Locale;
    use devsweep_core::status::AvailabilityV1;
    let (AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. }) =
        &snapshot.processes
    else {
        return None;
    };
    let shown = visible.min(100);
    let returned = value.returned_count;
    let enumerated = value.enumerated_count;
    Some(match locale {
        Locale::En if value.truncated_by_limit || value.budget_exhausted => format!(
            "Showing {shown} of {returned} sampled rows. {enumerated} processes were running; the sample kept {returned}."
        ),
        Locale::En => format!("Showing {shown} of {returned} sampled processes."),
        Locale::ZhCn if value.truncated_by_limit || value.budget_exhausted => format!(
            "显示 {shown} / {returned} 行采样。运行中的进程有 {enumerated} 个，本次采样保留了 {returned} 个。"
        ),
        Locale::ZhCn => format!("显示 {shown} / {returned} 个采样进程。"),
    })
}

/// Local class of a process name, read from the core's closed protected
/// list. `Unlisted` says only that the name is not on that list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessClass {
    Critical,
    Shared,
    System,
    Unlisted,
}

#[must_use]
pub fn process_class(name: &str) -> ProcessClass {
    use devsweep_core::status::{ProtectedClass, protected_process_names};
    let lower = name.to_lowercase();
    protected_process_names()
        .iter()
        .find(|(protected, _)| *protected == lower)
        .map_or(ProcessClass::Unlisted, |(_, class)| match class {
            ProtectedClass::Critical => ProcessClass::Critical,
            ProtectedClass::Shared => ProcessClass::Shared,
            ProtectedClass::System => ProcessClass::System,
        })
}

/// The row the user picked, keyed by PID and name. A PID can be reused by a
/// later process, so a sample row with the same PID and another name does not
/// match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessPick {
    pub pid: u32,
    pub name: String,
}

impl ProcessPick {
    #[must_use]
    pub fn of(row: &devsweep_core::status::ProcessV1) -> Self {
        Self {
            pid: row.pid,
            name: row.name.clone(),
        }
    }

    /// The same process in a later sample, if it is still listed.
    #[must_use]
    pub fn find(
        &self,
        snapshot: &devsweep_core::status::StatusSnapshotV1,
    ) -> Option<devsweep_core::status::ProcessV1> {
        use devsweep_core::status::AvailabilityV1;
        let (AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. }) =
            &snapshot.processes
        else {
            return None;
        };
        value
            .items
            .iter()
            .find(|row| row.pid == self.pid && row.name == self.name)
            .cloned()
    }
}

/// Plain local explanation of one sampled process. Built only from the row,
/// the memory total in the same snapshot, and the closed protected list. It
/// does not look the program up anywhere and offers no action.
#[must_use]
pub fn process_explanation(
    locale: crate::copy::Locale,
    row: &devsweep_core::status::ProcessV1,
    snapshot: &devsweep_core::status::StatusSnapshotV1,
) -> Vec<String> {
    use crate::copy::{Locale, pick};
    use crate::ui::bytes;
    use devsweep_core::status::AvailabilityV1;

    let class = match process_class(&row.name) {
        ProcessClass::Critical => pick(
            locale,
            "Core Windows process. Ending it would crash Windows or sign you out.",
            "Windows 核心进程。结束它会导致系统崩溃或注销。",
        ),
        ProcessClass::Shared => pick(
            locale,
            "Shared Windows process. The desktop or many services depend on it.",
            "Windows 共享进程。桌面或许多服务依赖它。",
        ),
        ProcessClass::System => pick(
            locale,
            "Kernel or system accounting entry, not an ordinary program.",
            "内核或系统记账条目，不是普通程序。",
        ),
        ProcessClass::Unlisted => pick(
            locale,
            "Not on DevSweep's protected list. The name alone does not say what the program does.",
            "不在 DevSweep 的受保护名单中。仅凭名称无法判断程序用途。",
        ),
    };

    let cores = row.cpu_basis_points_of_one_logical_core as f64 / 10_000.0;
    let cpu = match locale {
        Locale::En if cores >= 1.0 => {
            format!("CPU: about {cores:.1} logical cores busy in this sample.")
        }
        Locale::En => format!(
            "CPU: {:.1}% of one logical core in this sample.",
            cores * 100.0
        ),
        Locale::ZhCn if cores >= 1.0 => format!("CPU：本次采样约占 {cores:.1} 个逻辑核心。"),
        Locale::ZhCn => format!("CPU：本次采样占一个逻辑核心的 {:.1}%。", cores * 100.0),
    };

    let total = match &snapshot.memory {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. }
            if value.total_bytes > 0 =>
        {
            Some(value.total_bytes)
        }
        _ => None,
    };
    let private = bytes(row.private_bytes);
    let memory = match (locale, total) {
        (Locale::En, Some(total)) => format!(
            "Memory: {private} private, {:.1}% of {} physical memory. Shared pages are not counted.",
            row.private_bytes as f64 * 100.0 / total as f64,
            bytes(total)
        ),
        (Locale::En, None) => {
            format!("Memory: {private} private. Shared pages are not counted.")
        }
        (Locale::ZhCn, Some(total)) => format!(
            "内存：私有 {private}，占物理内存 {} 的 {:.1}%。不含共享页。",
            bytes(total),
            row.private_bytes as f64 * 100.0 / total as f64
        ),
        (Locale::ZhCn, None) => format!("内存：私有 {private}。不含共享页。"),
    };

    let io = match locale {
        Locale::En => format!(
            "Disk I/O: {}/s read, {}/s written since the previous sample.",
            bytes(row.read_bytes_per_second),
            bytes(row.write_bytes_per_second)
        ),
        Locale::ZhCn => format!(
            "磁盘 I/O：自上次采样起读取 {}/s，写入 {}/s。",
            bytes(row.read_bytes_per_second),
            bytes(row.write_bytes_per_second)
        ),
    };

    let pid = match locale {
        Locale::En => format!(
            "PID {} only labels this row in this sample. DevSweep does not end processes from this list.",
            row.pid
        ),
        Locale::ZhCn => format!(
            "PID {} 只用于标记本次采样中的这一行。DevSweep 不会从此列表结束进程。",
            row.pid
        ),
    };

    vec![class.to_string(), cpu, memory, io, pid]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{Mode, Presence, Session, ShellState};
    use crate::view::panel_lines;

    fn looks_like_zero_reading(line: &str) -> bool {
        line.split_whitespace()
            .any(|token| token == "0" || token == "0%" || token == "0.0" || token == "0.00")
    }

    #[test]
    fn status_tray_shares_one_subscription() {
        let mut starts = 0u32;
        let mut stops = 0u32;
        let mut panel = StatusPanel::new();

        let mut shell = ShellState::new();
        shell.set_presence(Presence::Hidden);
        assert!(!shell.request_decorative_frame());
        assert_eq!(shell.decorative_frames_scheduled(), 0);

        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 0, "both hidden must not start");
        assert_eq!(stops, 0);

        panel.set_main_visible(true);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 1);
        assert_eq!(stops, 0);

        panel.set_tray_visible(true);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 1, "tray while main is visible must not start again");
        assert_eq!(stops, 0);

        panel.set_main_visible(false);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 1);
        assert_eq!(stops, 0, "hiding main while tray stays must not stop");

        panel.set_tray_visible(false);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 1);
        assert_eq!(stops, 1, "hiding both must stop once");

        panel.set_main_visible(true);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 2, "a later show must start once more");
        assert_eq!(stops, 1);

        let missing = availability_line(&AvailabilityV1::<()>::unavailable("counter_missing"));
        assert!(missing.contains("counter_missing"));
        assert!(missing.contains("unavailable"));
        assert!(!looks_like_zero_reading(&missing));

        let denied = availability_line(&AvailabilityV1::<()>::permission_denied("access_denied"));
        assert!(denied.contains("access_denied"));
        assert!(denied.contains("permission_denied"));
        assert!(!looks_like_zero_reading(&denied));

        let unsupported =
            availability_line(&AvailabilityV1::<()>::unsupported("platform_unsupported"));
        assert!(unsupported.contains("platform_unsupported"));
        assert!(unsupported.contains("unsupported"));
        assert!(!looks_like_zero_reading(&unsupported));

        let mut session = Session::unbacked();
        session.show_mode(Mode::Status);
        let mut status = StatusPanel::new();
        status.set_sensor(&AvailabilityV1::<()>::unavailable("counter_missing"));
        session.set_status_panel(Some(status));
        let lines = panel_lines(&session, None);
        let sensor = lines
            .iter()
            .find(|line| line.contains("counter_missing") && line.contains("unavailable"))
            .cloned()
            .expect("Status panel_lines must append the sensor line");
        assert_eq!(
            lines.iter().filter(|line| *line == &sensor).count(),
            1,
            "append that one sensor line: {lines:?}"
        );
        assert!(
            !lines.iter().any(|line| looks_like_zero_reading(line)),
            "Status panel_lines must not append a zero: {lines:?}"
        );
    }

    #[test]
    fn missing_sensors_stay_unavailable_and_processes_can_be_searched() {
        use devsweep_core::status::{AvailabilityV1, ProcessV1, ProcessesV1, StatusSnapshotV1};

        let missing = StatusSnapshotV1 {
            snapshot_id: "missing".to_string(),
            sampled_at_unix_ms: 1,
            sample_window_ms: 500,
            logical_processor_count: 1,
            cpu: AvailabilityV1::unavailable("counter_missing"),
            memory: AvailabilityV1::unavailable("counter_missing"),
            volumes: AvailabilityV1::unavailable("counter_missing"),
            network: AvailabilityV1::unavailable("counter_missing"),
            power: AvailabilityV1::unavailable("counter_missing"),
            gpu: AvailabilityV1::unavailable("sensor_missing"),
            thermal: AvailabilityV1::unsupported("platform_unsupported"),
            processes: AvailabilityV1::permission_denied("process_access_denied"),
            unsupported_capabilities: Vec::new(),
        };
        let lines = group_lines(&missing, crate::copy::Locale::En);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("unavailable") && line.contains("counter_missing"))
        );
        assert!(
            lines.iter().any(
                |line| line.contains("permission denied") || line.contains("permission_denied")
            )
        );
        assert!(lines.iter().any(|line| line.contains("unsupported")));
        assert!(
            !lines.iter().any(|line| looks_like_zero_reading(line)),
            "missing sensors must not be drawn as zero: {lines:?}"
        );
        assert!(process_lines(&missing, "", ProcessSort::Name).is_empty());

        let processes = ProcessesV1 {
            items: vec![
                ProcessV1 {
                    pid: 20,
                    name: "beta".to_string(),
                    cpu_basis_points_of_one_logical_core: 10,
                    private_bytes: 2,
                    read_bytes_per_second: 0,
                    write_bytes_per_second: 0,
                },
                ProcessV1 {
                    pid: 10,
                    name: "alpha".to_string(),
                    cpu_basis_points_of_one_logical_core: 50,
                    private_bytes: 9,
                    read_bytes_per_second: 0,
                    write_bytes_per_second: 0,
                },
            ],
            enumerated_count: 2,
            returned_count: 2,
            requested_limit: 8,
            enumeration_ceiling: 8,
            detail_budget_ms: 150,
            truncated_by_limit: false,
            budget_exhausted: false,
        };
        let mut live = missing;
        live.processes = AvailabilityV1::available(1, 0, processes);
        let found = process_lines(&live, "alpha", ProcessSort::Name);
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("pid 10"));
        let sorted = process_lines(&live, "", ProcessSort::Cpu);
        assert!(sorted[0].contains("alpha"));
        assert!(!sorted.iter().any(|line| line.contains("terminate")));
    }

    #[test]
    fn session_main_and_tray_share_one_start() {
        use crate::shell::Session;

        let mut session = Session::unbacked();
        session.status_set_tray_visible(true);
        session.status_set_main_visible(true);
        assert_eq!(session.status_subscription_starts(), 1);
        session.status_set_main_visible(false);
        assert_eq!(session.status_subscription_stops(), 0);
        session.status_set_tray_visible(false);
        assert_eq!(session.status_subscription_stops(), 1);
        assert_eq!(session.shell().decorative_frames_scheduled(), 0);
    }

    #[test]
    fn displayed_pid_does_not_kill() {
        const PID: u32 = 424242;
        let mut panel = StatusPanel::new();
        panel.set_displayed_pid(Some(PID));
        assert_eq!(panel.displayed_pid(), Some(PID));

        let error = panel
            .request_kill_displayed_pid()
            .expect_err("displayed PID is not termination authority");
        let text = error.to_string();
        assert!(
            text.contains("displayed PID is not termination authority"),
            "{text}"
        );
        let lower = text.to_ascii_lowercase();
        assert!(!lower.contains("terminated"), "{text}");
        assert!(!lower.contains("succeeded"), "{text}");
        assert!(!lower.contains("succeed"), "{text}");

        let mut session = Session::unbacked();
        session.show_mode(Mode::Status);
        session.set_status_panel(Some(panel));
        let lines = panel_lines(&session, None);
        let pid_line = format!("pid {PID}");
        assert_eq!(
            lines.iter().filter(|line| *line == &pid_line).count(),
            1,
            "Status panel_lines must append exactly one pid line: {lines:?}"
        );
        assert_eq!(
            lines.iter().filter(|line| line.starts_with("pid ")).count(),
            1,
            "no other PID line: {lines:?}"
        );

        let source = crate::source_lf(include_str!("status_panel.rs"));
        assert!(!source.contains(&["terminate", "_bound_process"].concat()));
    }

    fn snapshot_with_processes(
        items: Vec<devsweep_core::status::ProcessV1>,
    ) -> devsweep_core::status::StatusSnapshotV1 {
        use devsweep_core::status::{MemoryV1, ProcessesV1, StatusSnapshotV1};
        let count = items.len() as u32;
        StatusSnapshotV1 {
            snapshot_id: "explain".to_string(),
            sampled_at_unix_ms: 1,
            sample_window_ms: 500,
            logical_processor_count: 8,
            cpu: AvailabilityV1::unavailable("counter_missing"),
            memory: AvailabilityV1::available(
                1,
                0,
                MemoryV1 {
                    total_bytes: 16 * 1024 * 1024 * 1024,
                    available_bytes: 8 * 1024 * 1024 * 1024,
                    used_bytes: 8 * 1024 * 1024 * 1024,
                },
            ),
            volumes: AvailabilityV1::unavailable("counter_missing"),
            network: AvailabilityV1::unavailable("counter_missing"),
            power: AvailabilityV1::unavailable("counter_missing"),
            gpu: AvailabilityV1::unavailable("sensor_missing"),
            thermal: AvailabilityV1::unsupported("platform_unsupported"),
            processes: AvailabilityV1::available(
                1,
                0,
                ProcessesV1 {
                    items,
                    enumerated_count: count,
                    returned_count: count,
                    requested_limit: 8,
                    enumeration_ceiling: 8,
                    detail_budget_ms: 150,
                    truncated_by_limit: false,
                    budget_exhausted: false,
                },
            ),
            unsupported_capabilities: Vec::new(),
        }
    }

    fn process(pid: u32, name: &str, cpu: u32, private: u64) -> devsweep_core::status::ProcessV1 {
        devsweep_core::status::ProcessV1 {
            pid,
            name: name.to_string(),
            cpu_basis_points_of_one_logical_core: cpu,
            private_bytes: private,
            read_bytes_per_second: 2048,
            write_bytes_per_second: 0,
        }
    }

    #[test]
    fn a_picked_process_is_explained_locally_and_offers_no_action() {
        use crate::copy::Locale;
        use crate::shell::Session;

        assert_eq!(process_class("LSASS.EXE"), ProcessClass::Critical);
        assert_eq!(process_class("explorer.exe"), ProcessClass::Shared);
        assert_eq!(process_class("System"), ProcessClass::System);
        assert_eq!(process_class("code.exe"), ProcessClass::Unlisted);

        let snapshot = snapshot_with_processes(vec![
            process(4, "System", 0, 0),
            process(812, "lsass.exe", 1_500, 64 * 1024 * 1024),
            process(4242, "code.exe", 23_000, 1024 * 1024 * 1024),
        ]);
        let busy = ProcessPick {
            pid: 4242,
            name: "code.exe".to_string(),
        }
        .find(&snapshot)
        .expect("listed");
        let lines = process_explanation(Locale::En, &busy, &snapshot);
        let text = lines.join("\n");
        assert!(text.contains("Not on DevSweep's protected list"), "{text}");
        assert!(text.contains("about 2.3 logical cores"), "{text}");
        assert!(
            text.contains("6.2% of 16.0 GiB") || text.contains("% of 16"),
            "{text}"
        );
        assert!(text.contains("does not end processes"), "{text}");
        for verb in ["terminate", "kill", "End task"] {
            assert!(!text.contains(verb), "{verb} in {text}");
        }

        let core = ProcessPick {
            pid: 812,
            name: "lsass.exe".to_string(),
        }
        .find(&snapshot)
        .expect("listed");
        let zh = process_explanation(Locale::ZhCn, &core, &snapshot).join("\n");
        assert!(zh.contains("Windows 核心进程"), "{zh}");
        assert!(zh.contains("一个逻辑核心的 15.0%"), "{zh}");

        // A reused PID under another name is not the picked process.
        let reused = ProcessPick {
            pid: 4242,
            name: "other.exe".to_string(),
        };
        assert!(reused.find(&snapshot).is_none());

        assert_eq!(
            process_count_line(Locale::En, &snapshot, 3).as_deref(),
            Some("Showing 3 of 3 sampled processes.")
        );
        let mut truncated = snapshot.clone();
        if let AvailabilityV1::Available { value, .. } = &mut truncated.processes {
            value.enumerated_count = 212;
            value.truncated_by_limit = true;
        }
        let line = process_count_line(Locale::ZhCn, &truncated, 2).expect("count");
        assert!(
            line.contains("显示 2 / 3") && line.contains("212"),
            "{line}"
        );

        let mut session = Session::unbacked();
        session.status_toggle_process(&busy);
        assert_eq!(session.status_picked_process().map(|p| p.pid), Some(4242));
        session.status_toggle_process(&busy);
        assert!(
            session.status_picked_process().is_none(),
            "a second pick closes"
        );
        session.status_toggle_process(&core);
        session.status_clear_process();
        assert!(session.status_picked_process().is_none());
    }
}
