use super::*;

#[test]
fn snapshot_round_trips_without_field_loss_or_rename() {
    let snapshot = fixture_snapshot();
    let encoded = serde_json::to_value(&snapshot).expect("snapshot serializes");
    let decoded: StatusSnapshotV1 =
        serde_json::from_value(encoded.clone()).expect("snapshot deserializes");
    assert_eq!(decoded, snapshot);
    let object = encoded.as_object().expect("object");
    for key in [
        "snapshot_id",
        "sampled_at_unix_ms",
        "sample_window_ms",
        "logical_processor_count",
        "cpu",
        "memory",
        "volumes",
        "network",
        "power",
        "gpu",
        "thermal",
        "processes",
        "unsupported_capabilities",
    ] {
        assert!(object.contains_key(key), "missing {key}");
    }
    assert_eq!(object.len(), 13);
    assert_eq!(
        snapshot
            .unsupported_capabilities
            .iter()
            .map(|capability| capability.code)
            .collect::<Vec<_>>(),
        [
            UnsupportedCode::Vram,
            UnsupportedCode::Thermal,
            UnsupportedCode::Fan,
            UnsupportedCode::Smart,
            UnsupportedCode::PhysicalDiskActivity,
        ]
    );
}

#[test]
fn availability_union_covers_all_five_states() {
    let available = AvailabilityV1::available(
        1,
        0,
        CpuV1 {
            system_utilization_basis_points: 1,
        },
    );
    let partial = AvailabilityV1::partial(
        1,
        1,
        CpuV1 {
            system_utilization_basis_points: 1,
        },
        vec!["counter_lagged".into()],
    );
    let unavailable = AvailabilityV1::<CpuV1>::unavailable("fixture_unavailable");
    let denied = AvailabilityV1::<CpuV1>::permission_denied("fixture_denied");
    let unsupported = AvailabilityV1::<CpuV1>::unsupported("not_supported_v1");
    for value in [available, partial, unavailable, denied, unsupported] {
        let encoded = serde_json::to_value(&value).unwrap();
        let decoded: AvailabilityV1<CpuV1> = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, value);
        assert!(encoded.get("sampled_at_unix_ms").is_some());
    }
}

#[test]
fn live_events_are_internally_tagged_with_sequence() {
    let started = StatusEventV1::Started {
        schema_version: 1,
        operation_id: "op-fixture".into(),
        sequence: 0,
        emitted_at_unix_ms: 1,
        data: StatusStartedV1 {
            interval_ms: 2_000,
            process_limit: 15,
        },
    };
    let encoded = serde_json::to_value(&started).unwrap();
    assert_eq!(encoded["event"], "status_started");
    assert_eq!(encoded["schema_version"], 1);
    let terminal = StatusEventV1::Terminal {
        schema_version: 1,
        operation_id: "op-fixture".into(),
        sequence: 3,
        emitted_at_unix_ms: 4,
        data: StatusTerminalV1 {
            reason: TerminalReason::BrokenPipe,
            error_code: None,
        },
    };
    let encoded = serde_json::to_value(&terminal).unwrap();
    assert_eq!(encoded["event"], "status_terminal");
    assert_eq!(encoded["data"]["reason"], "broken_pipe");
    assert!(encoded["data"]["error_code"].is_null());
}

#[test]
fn capabilities_are_unsupported_not_zero() {
    let gpu = AvailabilityV1::<GpuV1>::unavailable("counter_missing");
    let thermal = AvailabilityV1::<ThermalV1>::unavailable("no_instances");
    let listed = unsupported_capabilities(&gpu, &thermal);
    assert_eq!(listed.len(), 6);
    for capability in listed {
        assert_eq!(capability.state, UnsupportedCapabilityState::Unsupported);
        assert_eq!(capability.reason_code, "not_supported_v1");
    }
}

#[test]
fn probed_capabilities_leave_the_unsupported_list_only_with_a_value() {
    let gpu = AvailabilityV1::available(
        1,
        0,
        GpuV1 {
            adapters: vec![GpuAdapterV1 {
                adapter_id: "luid_0x0_0x1_phys_0".into(),
                utilization_basis_points: 0,
            }],
        },
    );
    let thermal = AvailabilityV1::available(
        1,
        0,
        ThermalV1 {
            zones: vec![ThermalZoneV1 {
                zone_id: "zone".into(),
                temperature_tenths_celsius: 279,
            }],
        },
    );
    let codes = |gpu: &AvailabilityV1<GpuV1>, thermal: &AvailabilityV1<ThermalV1>| {
        unsupported_capabilities(gpu, thermal)
            .into_iter()
            .map(|capability| capability.code)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        codes(&gpu, &thermal),
        [
            UnsupportedCode::Vram,
            UnsupportedCode::Fan,
            UnsupportedCode::Smart,
            UnsupportedCode::PhysicalDiskActivity,
        ]
    );
    let missing_gpu = AvailabilityV1::<GpuV1>::unavailable("counter_missing");
    assert_eq!(
        codes(&missing_gpu, &thermal)[0],
        UnsupportedCode::GpuUtilization
    );
    let unsupported_thermal = AvailabilityV1::<ThermalV1>::unsupported("platform_unsupported");
    assert!(codes(&gpu, &unsupported_thermal).contains(&UnsupportedCode::Thermal));
}

#[test]
fn missing_gpu_or_thermal_does_not_degrade_the_snapshot_outcome() {
    let mut snapshot = fixture_snapshot();
    snapshot.memory = AvailabilityV1::available(
        1,
        0,
        MemoryV1 {
            total_bytes: 2,
            available_bytes: 1,
            used_bytes: 1,
        },
    );
    snapshot.processes = AvailabilityV1::unsupported("platform_unsupported");
    snapshot.thermal = AvailabilityV1::unsupported("platform_unsupported");
    assert_eq!(snapshot.envelope_outcome(), "success");
    snapshot.gpu = AvailabilityV1::unavailable("counter_missing");
    assert_eq!(snapshot.envelope_outcome(), "success");
    assert!(snapshot.envelope_warnings().is_empty());
}

fn fixture_snapshot() -> StatusSnapshotV1 {
    StatusSnapshotV1 {
        snapshot_id: "snapshot-fixture".into(),
        sampled_at_unix_ms: 1_788_019_200_000,
        sample_window_ms: 1_000,
        logical_processor_count: 16,
        cpu: AvailabilityV1::available(
            1_788_019_200_000,
            0,
            CpuV1 {
                system_utilization_basis_points: 4_321,
            },
        ),
        memory: AvailabilityV1::partial(
            1_788_019_200_000,
            1,
            MemoryV1 {
                total_bytes: 34_359_738_368,
                available_bytes: 17_179_869_184,
                used_bytes: 17_179_869_184,
            },
            vec!["counter_lagged".into()],
        ),
        volumes: AvailabilityV1::available(
            1_788_019_200_000,
            2,
            VolumesV1 {
                items: vec![VolumeV1 {
                    volume_id: "volume-fixture".into(),
                    mount_points: vec!["C:\\".into()],
                    total_bytes: 1_000_000_000,
                    available_bytes: 400_000_000,
                }],
                complete: true,
            },
        ),
        network: AvailabilityV1::available(
            1_788_019_200_000,
            2,
            NetworkV1 {
                interval_ms: 1_000,
                interfaces: vec![NetworkInterfaceV1 {
                    interface_luid: "18446744073709551615".into(),
                    name: "fixture-adapter".into(),
                    rx_bytes_per_second: 4_096,
                    tx_bytes_per_second: 2_048,
                }],
            },
        ),
        power: AvailabilityV1::available(
            1_788_019_200_000,
            2,
            PowerV1 {
                battery_present: false,
                ac_state: AcState::Online,
                charge_basis_points: None,
                remaining_seconds: None,
            },
        ),
        gpu: AvailabilityV1::available(
            1_788_019_200_000,
            0,
            GpuV1 {
                adapters: vec![GpuAdapterV1 {
                    adapter_id: "luid_0x00000000_0x0000C0B6_phys_0".into(),
                    utilization_basis_points: 1_250,
                }],
            },
        ),
        thermal: AvailabilityV1::unavailable("counter_missing"),
        processes: AvailabilityV1::partial(
            1_788_019_200_000,
            3,
            ProcessesV1 {
                items: vec![ProcessV1 {
                    pid: 42,
                    name: "fixture.exe".into(),
                    cpu_basis_points_of_one_logical_core: 12_500,
                    private_bytes: 1_048_576,
                    read_bytes_per_second: 2_048,
                    write_bytes_per_second: 1_024,
                }],
                enumerated_count: 4_096,
                returned_count: 1,
                requested_limit: 15,
                enumeration_ceiling: 4_096,
                detail_budget_ms: 150,
                truncated_by_limit: true,
                budget_exhausted: true,
            },
            vec!["process_limit".into(), "detail_budget_exhausted".into()],
        ),
        unsupported_capabilities: unsupported_capabilities(
            &AvailabilityV1::available(
                1,
                0,
                GpuV1 {
                    adapters: Vec::new(),
                },
            ),
            &AvailabilityV1::<ThermalV1>::unavailable("counter_missing"),
        ),
    }
}
