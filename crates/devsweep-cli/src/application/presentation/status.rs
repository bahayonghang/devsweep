//! Bilingual Status V1 human renderer over the frozen snapshot DTO.

use devsweep_core::status::{
    AcState, AvailabilityV1, PowerV1, ProcessesV1, StatusEventV1, StatusSnapshotV1, TerminalReason,
};

use super::render;
use crate::i18n::{CatalogueError, Locale, format_binary_bytes};

#[cfg(test)]
pub(super) struct Route;

pub(crate) fn snapshot(
    locale: Locale,
    snapshot: &StatusSnapshotV1,
) -> Result<String, CatalogueError> {
    let mut lines = vec![render(
        locale,
        "status.v1.snapshot.title",
        &[("id", snapshot.snapshot_id.as_str())],
        None,
    )?];
    lines.push(render_cpu(locale, &snapshot.cpu)?);
    lines.push(render_memory(locale, &snapshot.memory)?);
    lines.extend(render_volumes(locale, &snapshot.volumes)?);
    lines.extend(render_network(locale, &snapshot.network)?);
    lines.push(render_power(locale, &snapshot.power)?);
    lines.extend(render_processes(locale, &snapshot.processes)?);
    Ok(lines.join("\n"))
}

pub(crate) fn live_started(
    locale: Locale,
    interval_ms: u32,
    process_limit: u32,
) -> Result<String, CatalogueError> {
    let interval = (interval_ms / 1_000).to_string();
    let limit = process_limit.to_string();
    render(
        locale,
        "status.v1.live.started",
        &[("interval", &interval), ("limit", &limit)],
        None,
    )
}

pub(crate) fn live_skipped(locale: Locale, skipped_total: u64) -> Result<String, CatalogueError> {
    let total = skipped_total.to_string();
    render(locale, "status.v1.live.skipped", &[("total", &total)], None)
}

pub(crate) fn live_stopped(
    locale: Locale,
    reason: TerminalReason,
) -> Result<String, CatalogueError> {
    let reason = match reason {
        TerminalReason::Completed => "completed",
        TerminalReason::Canceled => "canceled",
        TerminalReason::BrokenPipe => "broken_pipe",
        TerminalReason::ProducerError => "producer_error",
    };
    render(
        locale,
        "status.v1.live.stopped",
        &[("reason", reason)],
        None,
    )
}

pub(crate) fn live_event(
    locale: Locale,
    event: &StatusEventV1,
) -> Result<Option<String>, CatalogueError> {
    match event {
        StatusEventV1::Started { data, .. } => Ok(Some(live_started(
            locale,
            data.interval_ms,
            data.process_limit,
        )?)),
        StatusEventV1::Snapshot { data, .. } => Ok(Some(snapshot(locale, data.as_ref())?)),
        StatusEventV1::TickSkipped { data, .. } => {
            Ok(Some(live_skipped(locale, data.skipped_total)?))
        }
        StatusEventV1::Terminal { data, .. } => Ok(Some(live_stopped(locale, data.reason)?)),
    }
}

fn render_cpu(
    locale: Locale,
    cpu: &AvailabilityV1<devsweep_core::status::CpuV1>,
) -> Result<String, CatalogueError> {
    match cpu {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            let percent = basis_points(value.system_utilization_basis_points);
            let line = render(locale, "status.v1.cpu", &[("percent", &percent)], None)?;
            suffix_partial(locale, "cpu", cpu, line)
        }
        other => tagged_group(locale, "cpu", other),
    }
}

fn render_memory(
    locale: Locale,
    memory: &AvailabilityV1<devsweep_core::status::MemoryV1>,
) -> Result<String, CatalogueError> {
    match memory {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            let line = render(
                locale,
                "status.v1.memory",
                &[
                    ("used", &format_binary_bytes(value.used_bytes)),
                    ("total", &format_binary_bytes(value.total_bytes)),
                    ("available", &format_binary_bytes(value.available_bytes)),
                ],
                None,
            )?;
            suffix_partial(locale, "memory", memory, line)
        }
        other => tagged_group(locale, "memory", other),
    }
}

fn render_volumes(
    locale: Locale,
    volumes: &AvailabilityV1<devsweep_core::status::VolumesV1>,
) -> Result<Vec<String>, CatalogueError> {
    match volumes {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            let mut lines = Vec::new();
            if let AvailabilityV1::Partial { reason_codes, .. } = volumes {
                lines.push(render(
                    locale,
                    "status.v1.group.partial",
                    &[("group", "volumes"), ("reasons", &reason_codes.join(","))],
                    None,
                )?);
            }
            for volume in &value.items {
                let mount = volume
                    .mount_points
                    .first()
                    .map(String::as_str)
                    .unwrap_or("-");
                lines.push(render(
                    locale,
                    "status.v1.volume.item",
                    &[
                        ("mount", mount),
                        ("available", &format_binary_bytes(volume.available_bytes)),
                        ("total", &format_binary_bytes(volume.total_bytes)),
                    ],
                    None,
                )?);
            }
            Ok(lines)
        }
        other => Ok(vec![tagged_group(locale, "volumes", other)?]),
    }
}

fn render_network(
    locale: Locale,
    network: &AvailabilityV1<devsweep_core::status::NetworkV1>,
) -> Result<Vec<String>, CatalogueError> {
    match network {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            let mut lines = Vec::new();
            if let AvailabilityV1::Partial { reason_codes, .. } = network {
                lines.push(render(
                    locale,
                    "status.v1.group.partial",
                    &[("group", "network"), ("reasons", &reason_codes.join(","))],
                    None,
                )?);
            }
            for interface in &value.interfaces {
                lines.push(render(
                    locale,
                    "status.v1.network.item",
                    &[
                        ("name", interface.name.as_str()),
                        ("rx", &format_binary_bytes(interface.rx_bytes_per_second)),
                        ("tx", &format_binary_bytes(interface.tx_bytes_per_second)),
                    ],
                    None,
                )?);
            }
            Ok(lines)
        }
        other => Ok(vec![tagged_group(locale, "network", other)?]),
    }
}

fn render_power(locale: Locale, power: &AvailabilityV1<PowerV1>) -> Result<String, CatalogueError> {
    match power {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            let ac_key = match value.ac_state {
                AcState::Online => "status.v1.ac.online",
                AcState::Offline => "status.v1.ac.offline",
                AcState::Unknown => "status.v1.ac.unknown",
            };
            let ac = render(locale, ac_key, &[], None)?;
            let mut line = render(locale, "status.v1.power.ac", &[("ac", &ac)], None)?;
            if value.battery_present {
                let percent = value
                    .charge_basis_points
                    .map(basis_points)
                    .unwrap_or_else(|| "-".into());
                let remaining = value
                    .remaining_seconds
                    .map(|seconds| seconds.to_string())
                    .unwrap_or_else(|| "-".into());
                line.push('\n');
                line.push_str(&render(
                    locale,
                    "status.v1.power.battery",
                    &[("percent", &percent), ("remaining", &remaining)],
                    None,
                )?);
            } else {
                line.push('\n');
                line.push_str(&render(locale, "status.v1.power.no_battery", &[], None)?);
            }
            suffix_partial(locale, "power", power, line)
        }
        other => tagged_group(locale, "power", other),
    }
}

fn render_processes(
    locale: Locale,
    processes: &AvailabilityV1<ProcessesV1>,
) -> Result<Vec<String>, CatalogueError> {
    match processes {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            let mut lines = vec![render(
                locale,
                "status.v1.process.summary",
                &[
                    ("returned", &value.returned_count.to_string()),
                    ("enumerated", &value.enumerated_count.to_string()),
                    ("limit", &value.requested_limit.to_string()),
                ],
                None,
            )?];
            if let AvailabilityV1::Partial { reason_codes, .. } = processes {
                lines.push(render(
                    locale,
                    "status.v1.group.partial",
                    &[("group", "processes"), ("reasons", &reason_codes.join(","))],
                    None,
                )?);
            }
            for process in &value.items {
                lines.push(render(
                    locale,
                    "status.v1.process.item",
                    &[
                        ("name", process.name.as_str()),
                        ("pid", &process.pid.to_string()),
                        (
                            "percent",
                            &basis_points(process.cpu_basis_points_of_one_logical_core),
                        ),
                        ("memory", &format_binary_bytes(process.private_bytes)),
                    ],
                    None,
                )?);
            }
            Ok(lines)
        }
        other => Ok(vec![tagged_group(locale, "processes", other)?]),
    }
}

fn tagged_group<T>(
    locale: Locale,
    group: &str,
    availability: &AvailabilityV1<T>,
) -> Result<String, CatalogueError> {
    match availability {
        AvailabilityV1::Unavailable { reason_code, .. } => render(
            locale,
            "status.v1.group.unavailable",
            &[("group", group), ("reason", reason_code)],
            None,
        ),
        AvailabilityV1::PermissionDenied { reason_code, .. } => render(
            locale,
            "status.v1.group.permission_denied",
            &[("group", group), ("reason", reason_code)],
            None,
        ),
        AvailabilityV1::Unsupported { reason_code, .. } => render(
            locale,
            "status.v1.group.unsupported",
            &[("group", group), ("reason", reason_code)],
            None,
        ),
        AvailabilityV1::Partial { reason_codes, .. } => render(
            locale,
            "status.v1.group.partial",
            &[("group", group), ("reasons", &reason_codes.join(","))],
            None,
        ),
        AvailabilityV1::Available { .. } => Ok(group.to_string()),
    }
}

fn suffix_partial<T>(
    locale: Locale,
    group: &str,
    availability: &AvailabilityV1<T>,
    line: String,
) -> Result<String, CatalogueError> {
    if let AvailabilityV1::Partial { reason_codes, .. } = availability {
        let tag = render(
            locale,
            "status.v1.group.partial",
            &[("group", group), ("reasons", &reason_codes.join(","))],
            None,
        )?;
        Ok(format!("{line}\n{tag}"))
    } else {
        Ok(line)
    }
}

fn basis_points(points: u32) -> String {
    format!("{}.{:02}", points / 100, points % 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::status::{
        CpuV1, MemoryV1, PowerV1, ProcessV1, ProcessesV1, static_unsupported_capabilities,
    };

    fn fixture() -> StatusSnapshotV1 {
        StatusSnapshotV1 {
            snapshot_id: "snapshot-fixture".into(),
            sampled_at_unix_ms: 1,
            sample_window_ms: 500,
            logical_processor_count: 8,
            cpu: AvailabilityV1::available(
                1,
                0,
                CpuV1 {
                    system_utilization_basis_points: 4_321,
                },
            ),
            memory: AvailabilityV1::available(
                1,
                0,
                MemoryV1 {
                    total_bytes: 1_024,
                    available_bytes: 512,
                    used_bytes: 512,
                },
            ),
            volumes: AvailabilityV1::unsupported("not_supported_v1"),
            network: AvailabilityV1::unavailable("fixture"),
            power: AvailabilityV1::available(
                1,
                0,
                PowerV1 {
                    battery_present: false,
                    ac_state: AcState::Online,
                    charge_basis_points: None,
                    remaining_seconds: None,
                },
            ),
            processes: AvailabilityV1::partial(
                1,
                0,
                ProcessesV1 {
                    items: vec![ProcessV1 {
                        pid: 42,
                        name: "fixture.exe".into(),
                        cpu_basis_points_of_one_logical_core: 12_500,
                        private_bytes: 1_024,
                        read_bytes_per_second: 0,
                        write_bytes_per_second: 0,
                    }],
                    enumerated_count: 20,
                    returned_count: 1,
                    requested_limit: 15,
                    enumeration_ceiling: 4_096,
                    detail_budget_ms: 150,
                    truncated_by_limit: true,
                    budget_exhausted: false,
                },
                vec!["process_limit".into()],
            ),
            unsupported_capabilities: static_unsupported_capabilities(),
        }
    }

    #[test]
    fn bilingual_snapshot_keeps_privacy_and_availability() {
        let document = fixture();
        let english = snapshot(Locale::En, &document).unwrap();
        let chinese = snapshot(Locale::ZhCn, &document).unwrap();
        assert!(english.contains("CPU: 43.21%"));
        assert!(english.contains("No battery present"));
        assert!(english.contains("fixture.exe"));
        assert!(english.contains("process_limit"));
        assert!(chinese.contains("无电池"));
        assert!(chinese.contains("fixture.exe"));
        for text in [english, chinese] {
            assert!(!text.contains("cmdline"));
            assert!(!text.contains("CleanupPlan"));
        }
    }
}
