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
pub fn displayed_pid_from_snapshot(snapshot: &devsweep_core::status::StatusSnapshotV1) -> Option<u32> {
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
        explain_health, CpuV1, GpuV1, MemoryV1, NetworkV1, PowerV1, ProcessesV1,
        ThermalV1, VolumesV1,
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
    lines.push(metric_line(locale, "cpu", &snapshot.cpu, |value: &CpuV1| {
        format!("{}", value.system_utilization_basis_points / 100)
    }));
    lines.push(metric_line(locale, "memory", &snapshot.memory, |value: &MemoryV1| {
        format!("{} B", value.used_bytes)
    }));
    lines.push(metric_line(locale, "gpu", &snapshot.gpu, |value: &GpuV1| {
        value
            .adapters
            .first()
            .map(|adapter| format!("{}", adapter.utilization_basis_points / 100))
            .unwrap_or_else(|| "unavailable".to_string())
    }));
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
        AvailabilityV1::Partial { reason_codes, value, .. } => {
            let detail = present(value);
            if detail.split_whitespace().any(|token| token == "0" || token == "0%")
                && reason_codes.iter().any(|code| code.contains("missing") || code.contains("unavailable"))
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

/// Searchable, sortable process rows. A displayed PID is not a kill request.
#[must_use]
pub fn process_lines(
    snapshot: &devsweep_core::status::StatusSnapshotV1,
    query: &str,
    sort: ProcessSort,
) -> Vec<String> {
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
    rows.iter()
        .map(|row| {
            format!(
                "{} pid {}: cpu {} memory {}",
                row.name,
                row.pid,
                row.cpu_basis_points_of_one_logical_core,
                row.private_bytes
            )
        })
        .collect()
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
        use devsweep_core::status::{
            AvailabilityV1, ProcessV1, ProcessesV1, StatusSnapshotV1,
        };

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
        assert!(lines.iter().any(|line| line.contains("unavailable") && line.contains("counter_missing")));
        assert!(lines.iter().any(|line| line.contains("permission denied") || line.contains("permission_denied")));
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

        let source = include_str!("status_panel.rs");
        assert!(!source.contains(&["terminate", "_bound_process"].concat()));
    }
}
