//! Local health text. Each line names one piece of evidence. There is no
//! combined score and this module does not sample.

use super::{AvailabilityV1, CpuV1, GpuV1, PowerV1, StatusSnapshotV1, ThermalV1, UnsupportedCode};

/// Age at which a retained sample is described as stale. The measured value
/// stays in the text; it is not replaced with zero.
pub const STALE_SAMPLE_AGE_MS: u64 = 30_000;

/// System utilization at or above this value is named as CPU load.
const CPU_LOAD_BASIS_POINTS: u32 = 8_000;

/// Closed health explanation. The document has evidence lines and no score.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HealthExplanationV1 {
    pub schema_version: u32,
    pub evidence: Vec<HealthEvidenceV1>,
}

/// One named observation, gap, or load statement.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HealthEvidenceV1 {
    pub subject: String,
    pub state: String,
    pub reason_code: Option<String>,
    pub age_ms: Option<u64>,
    pub text: String,
}

/// Project one snapshot into health text. This function does not read hardware.
#[must_use]
pub fn explain_health(snapshot: &StatusSnapshotV1) -> HealthExplanationV1 {
    let mut evidence = Vec::new();
    evidence.push(cpu_evidence(&snapshot.cpu));
    evidence.push(memory_evidence(&snapshot.memory));
    evidence.extend(power_evidence(&snapshot.power));
    evidence.push(gpu_evidence(&snapshot.gpu));
    if let Some(adapters) = gpu_adapters(&snapshot.gpu) {
        evidence.extend(adapters);
    }
    evidence.push(thermal_evidence(&snapshot.thermal));
    for capability in &snapshot.unsupported_capabilities {
        evidence.push(capability_evidence(
            capability_subject(capability.code),
            &capability.reason_code,
        ));
    }
    HealthExplanationV1 {
        schema_version: 1,
        evidence,
    }
}

fn cpu_evidence(cpu: &AvailabilityV1<CpuV1>) -> HealthEvidenceV1 {
    match cpu {
        AvailabilityV1::Available { value, age_ms, .. }
        | AvailabilityV1::Partial { value, age_ms, .. } => {
            let points = value.system_utilization_basis_points;
            let stale = *age_ms >= STALE_SAMPLE_AGE_MS;
            let state = if stale {
                "stale"
            } else if points >= CPU_LOAD_BASIS_POINTS {
                "load"
            } else if matches!(cpu, AvailabilityV1::Partial { .. }) {
                "partial"
            } else {
                "available"
            };
            let reason_code = if stale {
                Some("sample_aged".to_string())
            } else {
                partial_reason(cpu)
            };
            let text = if stale {
                format!(
                    "CPU utilization evidence is {points} basis points at age {age_ms} ms. The aged sample is not rewritten to zero. CPU load is not a system fault."
                )
            } else if points >= CPU_LOAD_BASIS_POINTS {
                format!(
                    "CPU load is {points} basis points of system utilization. CPU load is not a system fault."
                )
            } else {
                format!(
                    "CPU utilization evidence is {points} basis points. CPU load is not a system fault."
                )
            };
            HealthEvidenceV1 {
                subject: "cpu".to_string(),
                state: state.to_string(),
                reason_code,
                age_ms: Some(*age_ms),
                text,
            }
        }
        AvailabilityV1::Unavailable { reason_code, .. } => gap(
            "cpu",
            "unavailable",
            reason_code,
            format!(
                "CPU evidence is unavailable with reason {reason_code}. No utilization is synthesized."
            ),
        ),
        AvailabilityV1::PermissionDenied { reason_code, .. } => gap(
            "cpu",
            "permission_denied",
            reason_code,
            format!("CPU evidence is permission denied with reason {reason_code}."),
        ),
        AvailabilityV1::Unsupported { reason_code, .. } => gap(
            "cpu",
            "unsupported",
            reason_code,
            format!("CPU evidence is unsupported with reason {reason_code}."),
        ),
    }
}

fn memory_evidence(memory: &AvailabilityV1<super::MemoryV1>) -> HealthEvidenceV1 {
    match memory {
        AvailabilityV1::Available { value, age_ms, .. }
        | AvailabilityV1::Partial { value, age_ms, .. } => {
            let stale = *age_ms >= STALE_SAMPLE_AGE_MS;
            HealthEvidenceV1 {
                subject: "memory".to_string(),
                state: if stale {
                    "stale".to_string()
                } else if matches!(memory, AvailabilityV1::Partial { .. }) {
                    "partial".to_string()
                } else {
                    "available".to_string()
                },
                reason_code: if stale {
                    Some("sample_aged".to_string())
                } else {
                    partial_reason(memory)
                },
                age_ms: Some(*age_ms),
                text: format!(
                    "Memory evidence is {} used bytes of {} total bytes at age {age_ms} ms.",
                    value.used_bytes, value.total_bytes
                ),
            }
        }
        AvailabilityV1::Unavailable { reason_code, .. } => gap(
            "memory",
            "unavailable",
            reason_code,
            format!("Memory evidence is unavailable with reason {reason_code}."),
        ),
        AvailabilityV1::PermissionDenied { reason_code, .. } => gap(
            "memory",
            "permission_denied",
            reason_code,
            format!("Memory evidence is permission denied with reason {reason_code}."),
        ),
        AvailabilityV1::Unsupported { reason_code, .. } => gap(
            "memory",
            "unsupported",
            reason_code,
            format!("Memory evidence is unsupported with reason {reason_code}."),
        ),
    }
}

fn power_evidence(power: &AvailabilityV1<PowerV1>) -> Vec<HealthEvidenceV1> {
    match power {
        AvailabilityV1::Available { value, age_ms, .. }
        | AvailabilityV1::Partial { value, age_ms, .. } => {
            let mut lines = Vec::new();
            if value.battery_present {
                let charge = value
                    .charge_basis_points
                    .map(|points| points.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                lines.push(HealthEvidenceV1 {
                    subject: "battery".to_string(),
                    state: "available".to_string(),
                    reason_code: None,
                    age_ms: Some(*age_ms),
                    text: format!("Battery charge evidence is {charge} basis points."),
                });
            } else {
                lines.push(gap(
                    "battery",
                    "unavailable",
                    "battery_absent",
                    "Battery evidence is unavailable with reason battery_absent. Charge is not reported as zero.".to_string(),
                ));
            }
            lines
        }
        AvailabilityV1::Unavailable { reason_code, .. } => vec![
            gap(
                "power",
                "unavailable",
                reason_code,
                format!("Power evidence is unavailable with reason {reason_code}."),
            ),
            gap(
                "battery",
                "unavailable",
                reason_code,
                format!(
                    "Battery evidence is unavailable with reason {reason_code}. Charge is not reported as zero."
                ),
            ),
        ],
        AvailabilityV1::PermissionDenied { reason_code, .. } => vec![gap(
            "battery",
            "permission_denied",
            reason_code,
            format!("Battery evidence is permission denied with reason {reason_code}."),
        )],
        AvailabilityV1::Unsupported { reason_code, .. } => vec![gap(
            "battery",
            "unsupported",
            reason_code,
            format!("Battery evidence is unsupported with reason {reason_code}."),
        )],
    }
}

fn gpu_evidence(gpu: &AvailabilityV1<GpuV1>) -> HealthEvidenceV1 {
    match gpu {
        AvailabilityV1::Available { value, age_ms, .. }
        | AvailabilityV1::Partial { value, age_ms, .. }
            if !value.adapters.is_empty() =>
        {
            HealthEvidenceV1 {
                subject: "gpu".to_string(),
                state: if *age_ms >= STALE_SAMPLE_AGE_MS {
                    "stale".to_string()
                } else if matches!(gpu, AvailabilityV1::Partial { .. }) {
                    "partial".to_string()
                } else {
                    "available".to_string()
                },
                reason_code: if *age_ms >= STALE_SAMPLE_AGE_MS {
                    Some("sample_aged".to_string())
                } else {
                    partial_reason(gpu)
                },
                age_ms: Some(*age_ms),
                text: format!(
                    "GPU evidence names {} adapter samples at age {age_ms} ms.",
                    value.adapters.len()
                ),
            }
        }
        AvailabilityV1::Available { .. } | AvailabilityV1::Partial { .. } => gap(
            "gpu",
            "unavailable",
            "no_instances",
            "GPU evidence is unavailable with reason no_instances. GPU utilization is not reported as zero.".to_string(),
        ),
        AvailabilityV1::Unavailable { reason_code, .. } => gap(
            "gpu",
            "unavailable",
            reason_code,
            format!(
                "GPU evidence is unavailable with reason {reason_code}. GPU utilization is not reported as zero."
            ),
        ),
        AvailabilityV1::PermissionDenied { reason_code, .. } => gap(
            "gpu",
            "permission_denied",
            reason_code,
            format!("GPU evidence is permission denied with reason {reason_code}."),
        ),
        AvailabilityV1::Unsupported { reason_code, .. } => gap(
            "gpu",
            "unsupported",
            reason_code,
            format!(
                "GPU evidence is unsupported with reason {reason_code}. GPU utilization is not reported as zero."
            ),
        ),
    }
}

fn gpu_adapters(gpu: &AvailabilityV1<GpuV1>) -> Option<Vec<HealthEvidenceV1>> {
    let (adapters, age_ms) = match gpu {
        AvailabilityV1::Available { value, age_ms, .. }
        | AvailabilityV1::Partial { value, age_ms, .. } => (&value.adapters, *age_ms),
        _ => return None,
    };
    Some(
        adapters
            .iter()
            .map(|adapter| HealthEvidenceV1 {
                subject: "gpu_adapter".to_string(),
                state: if age_ms >= STALE_SAMPLE_AGE_MS {
                    "stale".to_string()
                } else {
                    "available".to_string()
                },
                reason_code: (age_ms >= STALE_SAMPLE_AGE_MS).then(|| "sample_aged".to_string()),
                age_ms: Some(age_ms),
                text: format!(
                    "GPU adapter {} utilization evidence is {} basis points.",
                    adapter.adapter_id, adapter.utilization_basis_points
                ),
            })
            .collect(),
    )
}

fn thermal_evidence(thermal: &AvailabilityV1<ThermalV1>) -> HealthEvidenceV1 {
    match thermal {
        AvailabilityV1::Available { value, age_ms, .. }
        | AvailabilityV1::Partial { value, age_ms, .. }
            if !value.zones.is_empty() =>
        {
            let summary = value
                .zones
                .iter()
                .map(|zone| {
                    format!(
                        "{} {} tenths C",
                        zone.zone_id, zone.temperature_tenths_celsius
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            HealthEvidenceV1 {
                subject: "thermal".to_string(),
                state: if *age_ms >= STALE_SAMPLE_AGE_MS {
                    "stale".to_string()
                } else {
                    "available".to_string()
                },
                reason_code: (*age_ms >= STALE_SAMPLE_AGE_MS).then(|| "sample_aged".to_string()),
                age_ms: Some(*age_ms),
                text: format!("Thermal evidence is {summary}."),
            }
        }
        AvailabilityV1::Available { .. } | AvailabilityV1::Partial { .. } => gap(
            "thermal",
            "unavailable",
            "no_instances",
            "Thermal evidence is unavailable with reason no_instances. Temperature is not reported as zero.".to_string(),
        ),
        AvailabilityV1::Unavailable { reason_code, .. } => gap(
            "thermal",
            "unavailable",
            reason_code,
            format!(
                "Thermal evidence is unavailable with reason {reason_code}. Temperature is not reported as zero."
            ),
        ),
        AvailabilityV1::PermissionDenied { reason_code, .. } => gap(
            "thermal",
            "permission_denied",
            reason_code,
            format!("Thermal evidence is permission denied with reason {reason_code}."),
        ),
        AvailabilityV1::Unsupported { reason_code, .. } => gap(
            "thermal",
            "unsupported",
            reason_code,
            format!(
                "Thermal evidence is unsupported with reason {reason_code}. Temperature is not reported as zero."
            ),
        ),
    }
}

fn capability_evidence(subject: &str, reason_code: &str) -> HealthEvidenceV1 {
    gap(
        subject,
        "unsupported",
        reason_code,
        format!(
            "{subject} evidence is unavailable with reason {reason_code}. No numeric reading is synthesized."
        ),
    )
}

fn capability_subject(code: UnsupportedCode) -> &'static str {
    match code {
        UnsupportedCode::GpuUtilization => "gpu_utilization",
        UnsupportedCode::Vram => "vram",
        UnsupportedCode::Thermal => "thermal_capability",
        UnsupportedCode::Fan => "fan",
        UnsupportedCode::Smart => "smart",
        UnsupportedCode::PhysicalDiskActivity => "physical_disk_activity",
    }
}

fn partial_reason<T>(value: &AvailabilityV1<T>) -> Option<String> {
    match value {
        AvailabilityV1::Partial { reason_codes, .. } if !reason_codes.is_empty() => {
            Some(reason_codes.join(","))
        }
        _ => None,
    }
}

fn gap(subject: &str, state: &str, reason_code: &str, text: String) -> HealthEvidenceV1 {
    HealthEvidenceV1 {
        subject: subject.to_string(),
        state: state.to_string(),
        reason_code: Some(reason_code.to_string()),
        age_ms: None,
        text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{
        AcState, GpuAdapterV1, MemoryV1, ThermalZoneV1, VolumesV1, cpu_utilization_basis_points,
        system::CpuTimes, unsupported_capabilities,
    };

    fn snapshot_with(
        cpu: AvailabilityV1<CpuV1>,
        power: AvailabilityV1<PowerV1>,
        gpu: AvailabilityV1<GpuV1>,
        thermal: AvailabilityV1<ThermalV1>,
    ) -> StatusSnapshotV1 {
        StatusSnapshotV1 {
            snapshot_id: "health-fixture".to_string(),
            sampled_at_unix_ms: 10,
            sample_window_ms: 500,
            logical_processor_count: 2,
            cpu,
            memory: AvailabilityV1::available(
                10,
                0,
                MemoryV1 {
                    total_bytes: 100,
                    available_bytes: 40,
                    used_bytes: 60,
                },
            ),
            volumes: AvailabilityV1::available(
                10,
                0,
                VolumesV1 {
                    items: Vec::new(),
                    complete: true,
                },
            ),
            network: AvailabilityV1::unavailable("fixture"),
            power,
            gpu: gpu.clone(),
            thermal: thermal.clone(),
            processes: AvailabilityV1::unavailable("fixture"),
            unsupported_capabilities: unsupported_capabilities(&gpu, &thermal),
        }
    }

    #[test]
    fn counter_reset_multiple_adapters_missing_battery_and_sensor_sample_aging_no_driver_install() {
        let reset = cpu_utilization_basis_points(
            CpuTimes {
                idle: 20,
                kernel: 30,
                user: 10,
            },
            CpuTimes {
                idle: 10,
                kernel: 30,
                user: 10,
            },
            500,
            500,
        );
        assert_eq!(reset, Err(crate::status::system::DeltaIssue::CounterReset));
        let encoded = serde_json::to_string(&AvailabilityV1::<CpuV1>::unavailable("counter_reset"))
            .expect("encode");
        assert!(!encoded.contains("system_utilization_basis_points"));

        let gpu = AvailabilityV1::available(
            10,
            STALE_SAMPLE_AGE_MS,
            GpuV1 {
                adapters: vec![
                    GpuAdapterV1 {
                        adapter_id: "adapter-a".to_string(),
                        utilization_basis_points: 1_500,
                    },
                    GpuAdapterV1 {
                        adapter_id: "adapter-b".to_string(),
                        utilization_basis_points: 250,
                    },
                ],
            },
        );
        let thermal = AvailabilityV1::<ThermalV1>::unavailable("counter_missing");
        let snapshot = snapshot_with(
            AvailabilityV1::unavailable("counter_reset"),
            AvailabilityV1::available(
                10,
                0,
                PowerV1 {
                    battery_present: false,
                    ac_state: AcState::Online,
                    charge_basis_points: None,
                    remaining_seconds: None,
                },
            ),
            gpu,
            thermal,
        );
        let health = explain_health(&snapshot);
        let cpu = line(&health, "cpu");
        assert_eq!(cpu.state, "unavailable");
        assert_eq!(cpu.reason_code.as_deref(), Some("counter_reset"));
        assert!(!cpu.text.contains("0 basis"));

        let adapters = health
            .evidence
            .iter()
            .filter(|item| item.subject == "gpu_adapter")
            .map(|item| item.text.clone())
            .collect::<Vec<_>>();
        assert_eq!(adapters.len(), 2);
        assert!(adapters.iter().any(|text| text.contains("adapter-a")));
        assert!(adapters.iter().any(|text| text.contains("adapter-b")));
        assert_eq!(line(&health, "gpu_adapter").state, "stale");
        assert!(line(&health, "gpu").text.contains("1500") || adapters[0].contains("1500"));

        let battery = line(&health, "battery");
        assert_eq!(battery.state, "unavailable");
        assert_eq!(battery.reason_code.as_deref(), Some("battery_absent"));
        assert!(!battery.text.contains('0'));

        for subject in ["vram", "fan", "physical_disk_activity", "thermal"] {
            let item = if subject == "thermal" {
                line(&health, "thermal")
            } else {
                line(&health, subject)
            };
            assert_ne!(item.state, "available", "{subject}");
            assert!(item.reason_code.is_some(), "{subject}");
            assert!(!item.text.contains(" 0 "), "{subject}");
        }
        let thermal_line = line(&health, "thermal");
        assert_eq!(thermal_line.reason_code.as_deref(), Some("counter_missing"));

        let source = include_str!("pdh.rs");
        let lowered = source.to_ascii_lowercase();
        for needle in ["setupapi", "diinstalldriver", "pnputil", "newdev"] {
            assert!(!lowered.contains(needle), "{needle}");
        }
        let sampler = ["thread", "::", "spawn"].concat();
        assert!(!include_str!("health.rs").contains(&sampler));
    }

    #[test]
    fn health_explanation_names_evidence_and_does_not_call_cpu_load_a_system_fault() {
        let mut snapshot = snapshot_with(
            AvailabilityV1::available(
                10,
                0,
                CpuV1 {
                    system_utilization_basis_points: 9_500,
                },
            ),
            AvailabilityV1::available(
                10,
                0,
                PowerV1 {
                    battery_present: true,
                    ac_state: AcState::Online,
                    charge_basis_points: Some(5_000),
                    remaining_seconds: Some(60),
                },
            ),
            AvailabilityV1::available(
                10,
                0,
                GpuV1 {
                    adapters: vec![GpuAdapterV1 {
                        adapter_id: "adapter-a".to_string(),
                        utilization_basis_points: 10,
                    }],
                },
            ),
            AvailabilityV1::available(
                10,
                0,
                ThermalV1 {
                    zones: vec![ThermalZoneV1 {
                        zone_id: "cpu-zone".to_string(),
                        temperature_tenths_celsius: 400,
                    }],
                },
            ),
        );
        snapshot.cpu = AvailabilityV1::available(
            10,
            0,
            CpuV1 {
                system_utilization_basis_points: 9_500,
            },
        );
        let health = explain_health(&snapshot);
        let cpu = line(&health, "cpu");
        assert_eq!(cpu.state, "load");
        assert!(cpu.text.contains("CPU load is 9500 basis points"));
        assert!(cpu.text.contains("CPU load is not a system fault"));
        assert_eq!(cpu.text.matches("system fault").count(), 1);
        let encoded = serde_json::to_value(&health).expect("health json");
        assert!(encoded.get("score").is_none());
        assert!(encoded.get("evidence").is_some());
        assert!(!encoded.to_string().contains("\"score\""));
        let again = explain_health(&snapshot);
        assert_eq!(again, health);
    }

    #[test]
    fn sample_aging_keeps_the_measured_cpu_value() {
        let health = explain_health(&snapshot_with(
            AvailabilityV1::available(
                10,
                STALE_SAMPLE_AGE_MS,
                CpuV1 {
                    system_utilization_basis_points: 4_321,
                },
            ),
            AvailabilityV1::unsupported("platform_unsupported"),
            AvailabilityV1::unavailable("counter_missing"),
            AvailabilityV1::unavailable("no_instances"),
        ));
        let cpu = line(&health, "cpu");
        assert_eq!(cpu.state, "stale");
        assert_eq!(cpu.reason_code.as_deref(), Some("sample_aged"));
        assert!(cpu.text.contains("4321"));
        assert!(cpu.text.contains("not rewritten to zero"));
        assert!(cpu.text.contains("not a system fault"));
    }

    fn line<'a>(health: &'a HealthExplanationV1, subject: &str) -> &'a HealthEvidenceV1 {
        health
            .evidence
            .iter()
            .find(|item| item.subject == subject)
            .unwrap_or_else(|| panic!("missing {subject}"))
    }
}
