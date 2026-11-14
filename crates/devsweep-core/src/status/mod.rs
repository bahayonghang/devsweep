//! Bounded read-only Status V1 snapshots. This module never creates cleanup
//! authority, never persists samples, and never inspects privileged process
//! fields.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::process::{CancelObserver, FlagCancelObserver};

mod battery;
mod channel;
mod health;
mod host;
mod keep_awake;
mod network;
mod pdh;
mod process;
mod process_action;
mod sampler;
mod system;

#[cfg(test)]
mod tests;

pub use channel::StatusEventReceiver;
pub use health::{HealthEvidenceV1, HealthExplanationV1, STALE_SAMPLE_AGE_MS, explain_health};
pub use keep_awake::{
    AwakeBackend, BackendCall, FakeClock, KeepAwakeClock, KeepAwakeError, KeepAwakeOwner,
    KeepAwakeService, KeepAwakeStatusV1, MAX_KEEP_AWAKE_MS, NoopBackend, RecordingBackend,
    SystemClock,
};
pub use process_action::{
    HostProcessControl, ObservedProcess, PROCESS_TERMINATION_DATA_LOSS_TEXT,
    PROCESS_TERMINATION_PLAN_VERSION, ProcessActionError, ProcessKiller, ProcessProbe,
    ProcessTerminationPlanV1, ProcessTerminationPreviewV1, ProcessTerminationReportV1,
    ProtectedClass, bind_process_termination_plan, preview_bound_process, protected_process_names,
    terminate_bound_process,
};
pub use sampler::{LiveControl, StatusError, StatusSampler, spawn_live};
pub use system::{cpu_utilization_basis_points, memory_from_physical};

/// Snapshot rate window in milliseconds.
pub const SNAPSHOT_WINDOW_MS: u32 = 500;
/// Default live interval in milliseconds.
pub const DEFAULT_LIVE_INTERVAL_MS: u32 = 2_000;
/// Minimum live interval in milliseconds.
pub const MIN_LIVE_INTERVAL_MS: u32 = 1_000;
/// Maximum live interval in milliseconds.
pub const MAX_LIVE_INTERVAL_MS: u32 = 60_000;
/// Process group refresh cadence during live sampling.
pub const PROCESS_REFRESH_MS: u32 = 4_000;
/// Volume and battery refresh cadence during live sampling.
pub const VOLUME_POWER_REFRESH_MS: u32 = 30_000;
/// Default returned process rows.
pub const DEFAULT_PROCESS_LIMIT: u32 = 15;
/// Maximum returned process rows.
pub const MAX_PROCESS_LIMIT: u32 = 100;
/// Maximum enumerated process ids.
pub const PROCESS_ENUMERATION_CEILING: u32 = 4_096;
/// Cooperative process-detail budget in milliseconds.
pub const PROCESS_DETAIL_BUDGET_MS: u32 = 150;
/// Status snapshot schema version.
pub const STATUS_SNAPSHOT_VERSION: u32 = 1;

/// Tagged metric availability. Missing hardware or access is never mapped to
/// a synthesized zero value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AvailabilityV1<T> {
    Available {
        sampled_at_unix_ms: u64,
        age_ms: u64,
        value: T,
    },
    Partial {
        sampled_at_unix_ms: u64,
        age_ms: u64,
        value: T,
        reason_codes: Vec<String>,
    },
    Unavailable {
        sampled_at_unix_ms: Option<u64>,
        reason_code: String,
    },
    PermissionDenied {
        sampled_at_unix_ms: Option<u64>,
        reason_code: String,
    },
    Unsupported {
        sampled_at_unix_ms: Option<u64>,
        reason_code: String,
    },
}

impl<T> AvailabilityV1<T> {
    #[must_use]
    pub fn available(sampled_at_unix_ms: u64, age_ms: u64, value: T) -> Self {
        Self::Available {
            sampled_at_unix_ms,
            age_ms,
            value,
        }
    }

    #[must_use]
    pub fn partial(
        sampled_at_unix_ms: u64,
        age_ms: u64,
        value: T,
        reason_codes: Vec<String>,
    ) -> Self {
        debug_assert!(!reason_codes.is_empty());
        Self::Partial {
            sampled_at_unix_ms,
            age_ms,
            value,
            reason_codes,
        }
    }

    #[must_use]
    pub fn unavailable(reason_code: impl Into<String>) -> Self {
        Self::Unavailable {
            sampled_at_unix_ms: None,
            reason_code: reason_code.into(),
        }
    }

    #[must_use]
    pub fn permission_denied(reason_code: impl Into<String>) -> Self {
        Self::PermissionDenied {
            sampled_at_unix_ms: None,
            reason_code: reason_code.into(),
        }
    }

    #[must_use]
    pub fn unsupported(reason_code: impl Into<String>) -> Self {
        Self::Unsupported {
            sampled_at_unix_ms: None,
            reason_code: reason_code.into(),
        }
    }

    #[must_use]
    pub fn is_degraded(&self) -> bool {
        matches!(
            self,
            Self::Partial { .. } | Self::Unavailable { .. } | Self::PermissionDenied { .. }
        )
    }

    /// True when the group carries a measured value (`available` or `partial`).
    #[must_use]
    pub fn has_value(&self) -> bool {
        matches!(self, Self::Available { .. } | Self::Partial { .. })
    }

    #[must_use]
    pub fn warning_codes(&self) -> &[String] {
        match self {
            Self::Partial { reason_codes, .. } => reason_codes,
            _ => &[],
        }
    }

    /// Transform a measured value; states without a value are unchanged.
    pub(crate) fn map_value<U>(self, map: impl FnOnce(T) -> U) -> AvailabilityV1<U> {
        match self {
            Self::Available {
                sampled_at_unix_ms,
                age_ms,
                value,
            } => AvailabilityV1::Available {
                sampled_at_unix_ms,
                age_ms,
                value: map(value),
            },
            Self::Partial {
                sampled_at_unix_ms,
                age_ms,
                value,
                reason_codes,
            } => AvailabilityV1::Partial {
                sampled_at_unix_ms,
                age_ms,
                value: map(value),
                reason_codes,
            },
            Self::Unavailable {
                sampled_at_unix_ms,
                reason_code,
            } => AvailabilityV1::Unavailable {
                sampled_at_unix_ms,
                reason_code,
            },
            Self::PermissionDenied {
                sampled_at_unix_ms,
                reason_code,
            } => AvailabilityV1::PermissionDenied {
                sampled_at_unix_ms,
                reason_code,
            },
            Self::Unsupported {
                sampled_at_unix_ms,
                reason_code,
            } => AvailabilityV1::Unsupported {
                sampled_at_unix_ms,
                reason_code,
            },
        }
    }

    fn with_age(self, age_ms: u64) -> Self {
        match self {
            Self::Available {
                sampled_at_unix_ms,
                value,
                ..
            } => Self::Available {
                sampled_at_unix_ms,
                age_ms,
                value,
            },
            Self::Partial {
                sampled_at_unix_ms,
                value,
                reason_codes,
                ..
            } => Self::Partial {
                sampled_at_unix_ms,
                age_ms,
                value,
                reason_codes,
            },
            other => other,
        }
    }
}

/// Closed Status V1 snapshot document payload.
///
/// `host` and `disk_activity` were added to V1 additively. A document written
/// before them decodes with both groups `unavailable` / `not_sampled`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSnapshotV1 {
    pub snapshot_id: String,
    pub sampled_at_unix_ms: u64,
    pub sample_window_ms: u32,
    pub logical_processor_count: u32,
    #[serde(default = "not_sampled")]
    pub host: AvailabilityV1<HostV1>,
    pub cpu: AvailabilityV1<CpuV1>,
    pub memory: AvailabilityV1<MemoryV1>,
    pub volumes: AvailabilityV1<VolumesV1>,
    pub network: AvailabilityV1<NetworkV1>,
    #[serde(default = "not_sampled")]
    pub disk_activity: AvailabilityV1<DiskActivityV1>,
    pub power: AvailabilityV1<PowerV1>,
    pub gpu: AvailabilityV1<GpuV1>,
    pub thermal: AvailabilityV1<ThermalV1>,
    pub processes: AvailabilityV1<ProcessesV1>,
    pub unsupported_capabilities: Vec<UnsupportedCapabilityV1>,
}

impl StatusSnapshotV1 {
    #[must_use]
    /// GPU, thermal, host identity and disk activity are optional host probes.
    /// Their absence is reported in the group state and
    /// `unsupported_capabilities`; it does not degrade the snapshot outcome, as
    /// before those probes existed.
    pub fn supported_group_is_degraded(&self) -> bool {
        self.cpu.is_degraded()
            || self.memory.is_degraded()
            || self.volumes.is_degraded()
            || self.network.is_degraded()
            || self.power.is_degraded()
            || self.processes.is_degraded()
    }

    #[must_use]
    pub fn envelope_outcome(&self) -> &'static str {
        if self.supported_group_is_degraded() {
            "partial"
        } else {
            "success"
        }
    }

    #[must_use]
    pub fn envelope_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        for group in [
            self.host.warning_codes(),
            self.cpu.warning_codes(),
            self.memory.warning_codes(),
            self.volumes.warning_codes(),
            self.network.warning_codes(),
            self.disk_activity.warning_codes(),
            self.power.warning_codes(),
            self.gpu.warning_codes(),
            self.thermal.warning_codes(),
            self.processes.warning_codes(),
        ] {
            for code in group {
                if !warnings.iter().any(|existing| existing == code) {
                    warnings.push(code.clone());
                }
            }
        }
        warnings
    }
}

fn not_sampled<T>() -> AvailabilityV1<T> {
    AvailabilityV1::unavailable("not_sampled")
}

/// Machine identity read once per sampler. Each field is `null` when its
/// source could not be read; the group is then `partial` with a reason code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostV1 {
    /// Processor brand string, trimmed.
    pub cpu_model: Option<String>,
    /// Measured kernel version with revision and release label, for example
    /// `10.0.26300.9974 (26H2)`. No product name is inferred.
    pub os_version: Option<String>,
    /// Last boot time, rounded to whole seconds.
    pub boot_time_unix_ms: Option<u64>,
}

/// Whole-system CPU utilization in basis points (0..=10000).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuV1 {
    pub system_utilization_basis_points: u32,
    /// Current effective frequency (nominal frequency x processor
    /// performance). `null` lists `cpu_frequency` as unsupported.
    #[serde(default)]
    pub frequency_mhz: Option<u32>,
}

/// Physical memory totals in integer bytes, plus the system commit charge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryV1 {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    #[serde(default)]
    pub commit_used_bytes: Option<u64>,
    #[serde(default)]
    pub commit_limit_bytes: Option<u64>,
}

/// System-wide physical disk throughput in bytes per second.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskActivityV1 {
    pub read_bytes_per_second: u64,
    pub write_bytes_per_second: u64,
}

/// Fixed-volume capacity group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumesV1 {
    pub items: Vec<VolumeV1>,
    pub complete: bool,
}

/// One fixed volume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeV1 {
    pub volume_id: String,
    pub mount_points: Vec<String>,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

/// Per-interface network rates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkV1 {
    pub interval_ms: u32,
    pub interfaces: Vec<NetworkInterfaceV1>,
}

/// One interface identified by a decimal LUID string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInterfaceV1 {
    pub interface_luid: String,
    pub name: String,
    pub rx_bytes_per_second: u64,
    pub tx_bytes_per_second: u64,
    #[serde(default)]
    pub kind: Option<NetworkKind>,
}

/// Closed interface kind. Virtual adapters that report the Ethernet type
/// (Hyper-V, Bluetooth PAN, WAN miniports) are `other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkKind {
    Ethernet,
    Wireless,
    Other,
}

/// AC line state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcState {
    Online,
    Offline,
    Unknown,
}

/// Battery and AC power. Missing hardware uses `battery_present: false`, never
/// a zero charge; every battery field is then `null`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerV1 {
    pub battery_present: bool,
    pub ac_state: AcState,
    pub charge_basis_points: Option<u32>,
    pub remaining_seconds: Option<u64>,
    /// Full-charge capacity over design capacity. Values above 10000 are
    /// reported as measured.
    #[serde(default)]
    pub full_charge_basis_points: Option<u32>,
    /// Firmware cycle count. A reported 0 means "not tracked" and is `null`.
    #[serde(default)]
    pub cycle_count: Option<u32>,
    /// Signed battery power: negative discharging, positive charging.
    #[serde(default)]
    pub rate_milliwatts: Option<i32>,
}

/// GPU engine utilization from `\GPU Engine(*)\Utilization Percentage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuV1 {
    pub adapters: Vec<GpuAdapterV1>,
}

/// One adapter: the busiest engine type, summed over its engine instances, in
/// basis points (0..=10000).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAdapterV1 {
    pub adapter_id: String,
    pub utilization_basis_points: u32,
    /// Driver description joined by adapter LUID. `null` when no adapter with
    /// that LUID could be named.
    #[serde(default)]
    pub name: Option<String>,
}

/// ACPI thermal zones from `\Thermal Zone Information(*)\Temperature`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThermalV1 {
    pub zones: Vec<ThermalZoneV1>,
}

/// One thermal zone in tenths of a degree Celsius.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThermalZoneV1 {
    pub zone_id: String,
    pub temperature_tenths_celsius: i32,
}

/// Bounded process group with truncation metadata populated before sorting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessesV1 {
    pub items: Vec<ProcessV1>,
    pub enumerated_count: u32,
    pub returned_count: u32,
    pub requested_limit: u32,
    pub enumeration_ceiling: u32,
    pub detail_budget_ms: u32,
    pub truncated_by_limit: bool,
    pub budget_exhausted: bool,
}

/// Privacy-limited process row: name, PID, CPU, memory, and I/O only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessV1 {
    pub pid: u32,
    pub name: String,
    pub cpu_basis_points_of_one_logical_core: u32,
    pub private_bytes: u64,
    pub read_bytes_per_second: u64,
    pub write_bytes_per_second: u64,
}

/// Unsupported V1 capability. VRAM, fan and SMART are always listed. Every
/// other code is listed only when its reading has no value in the same
/// snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedCapabilityV1 {
    pub code: UnsupportedCode,
    pub state: UnsupportedCapabilityState,
    pub reason_code: String,
}

/// Closed unsupported-capability codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedCode {
    GpuUtilization,
    Vram,
    Thermal,
    Fan,
    Smart,
    PhysicalDiskActivity,
    CpuFrequency,
    BatteryHealth,
    BatteryCycleCount,
    BatteryRate,
    GpuName,
}

/// Capability state. V1 values are always `unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedCapabilityState {
    Unsupported,
}

/// Four frozen live NDJSON event shapes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum StatusEventV1 {
    #[serde(rename = "status_started")]
    Started {
        schema_version: u32,
        operation_id: String,
        sequence: u64,
        emitted_at_unix_ms: u64,
        data: StatusStartedV1,
    },
    #[serde(rename = "status_snapshot")]
    Snapshot {
        schema_version: u32,
        operation_id: String,
        sequence: u64,
        emitted_at_unix_ms: u64,
        data: Box<StatusSnapshotV1>,
    },
    #[serde(rename = "tick_skipped")]
    TickSkipped {
        schema_version: u32,
        operation_id: String,
        sequence: u64,
        emitted_at_unix_ms: u64,
        data: TickSkippedV1,
    },
    #[serde(rename = "status_terminal")]
    Terminal {
        schema_version: u32,
        operation_id: String,
        sequence: u64,
        emitted_at_unix_ms: u64,
        data: StatusTerminalV1,
    },
}

impl StatusEventV1 {
    #[must_use]
    pub fn operation_id(&self) -> &str {
        match self {
            Self::Started { operation_id, .. }
            | Self::Snapshot { operation_id, .. }
            | Self::TickSkipped { operation_id, .. }
            | Self::Terminal { operation_id, .. } => operation_id,
        }
    }

    #[must_use]
    pub fn sequence(&self) -> u64 {
        match self {
            Self::Started { sequence, .. }
            | Self::Snapshot { sequence, .. }
            | Self::TickSkipped { sequence, .. }
            | Self::Terminal { sequence, .. } => *sequence,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal { .. })
    }
}

/// Live start payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusStartedV1 {
    pub interval_ms: u32,
    pub process_limit: u32,
}

/// Skipped-tick payload. The only V1 reason is `sample_in_flight`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickSkippedV1 {
    pub reason: TickSkipReason,
    pub skipped_total: u64,
}

/// Closed skip reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TickSkipReason {
    SampleInFlight,
}

/// Terminal lifecycle payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusTerminalV1 {
    pub reason: TerminalReason,
    pub error_code: Option<String>,
}

/// Closed terminal reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalReason {
    Completed,
    Canceled,
    BrokenPipe,
    ProducerError,
}

/// Capture one 500 ms Status snapshot.
pub fn capture_snapshot(
    process_limit: u32,
    cancel: Option<&dyn CancelObserver>,
) -> Result<StatusSnapshotV1, StatusError> {
    StatusSampler::default().snapshot(process_limit, cancel)
}

/// Live sampling request. The caller owns output and cancellation.
pub struct LiveRequest<'a> {
    pub interval_ms: u32,
    pub process_limit: u32,
    pub operation_id: String,
    pub cancel: Option<&'a Arc<FlagCancelObserver>>,
}

/// Unsupported capabilities for one snapshot, in the closed V1 order. VRAM,
/// fan and SMART are always listed. GPU utilization, thermal and physical-disk
/// activity are listed when their group has no value. CPU frequency, battery
/// health, cycle count and rate are listed when the field is `null` or its
/// group has no value. GPU name is listed when the GPU group has no value or
/// any adapter has a `null` name. The `unsupported_capabilities` field of
/// `snapshot` itself is ignored.
#[must_use]
pub fn unsupported_capabilities(snapshot: &StatusSnapshotV1) -> Vec<UnsupportedCapabilityV1> {
    let cpu_frequency = group_value(&snapshot.cpu).and_then(|cpu| cpu.frequency_mhz);
    let power = group_value(&snapshot.power);
    let gpu_named = group_value(&snapshot.gpu)
        .is_some_and(|gpu| gpu.adapters.iter().all(|adapter| adapter.name.is_some()));
    [
        (UnsupportedCode::GpuUtilization, !snapshot.gpu.has_value()),
        (UnsupportedCode::Vram, true),
        (UnsupportedCode::Thermal, !snapshot.thermal.has_value()),
        (UnsupportedCode::Fan, true),
        (UnsupportedCode::Smart, true),
        (
            UnsupportedCode::PhysicalDiskActivity,
            !snapshot.disk_activity.has_value(),
        ),
        (UnsupportedCode::CpuFrequency, cpu_frequency.is_none()),
        (
            UnsupportedCode::BatteryHealth,
            power
                .and_then(|power| power.full_charge_basis_points)
                .is_none(),
        ),
        (
            UnsupportedCode::BatteryCycleCount,
            power.and_then(|power| power.cycle_count).is_none(),
        ),
        (
            UnsupportedCode::BatteryRate,
            power.and_then(|power| power.rate_milliwatts).is_none(),
        ),
        (UnsupportedCode::GpuName, !gpu_named),
    ]
    .into_iter()
    .filter_map(|(code, listed)| listed.then_some(code))
    .map(|code| UnsupportedCapabilityV1 {
        code,
        state: UnsupportedCapabilityState::Unsupported,
        reason_code: "not_supported_v1".to_string(),
    })
    .collect()
}

fn group_value<T>(group: &AvailabilityV1<T>) -> Option<&T> {
    match group {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            Some(value)
        }
        _ => None,
    }
}

#[must_use]
pub(crate) fn clamp_process_limit(requested: u32) -> u32 {
    requested.clamp(1, MAX_PROCESS_LIMIT)
}

#[must_use]
pub(crate) fn clamp_interval_ms(interval_ms: u32) -> u32 {
    let rounded = interval_ms / 1_000 * 1_000;
    rounded.clamp(MIN_LIVE_INTERVAL_MS, MAX_LIVE_INTERVAL_MS)
}

#[must_use]
pub(crate) fn unix_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[must_use]
pub(crate) fn logical_processor_count() -> u32 {
    std::thread::available_parallelism()
        .map(|count| u32::try_from(count.get()).unwrap_or(1))
        .unwrap_or(1)
        .max(1)
}
