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
        "host",
        "cpu",
        "memory",
        "volumes",
        "network",
        "disk_activity",
        "power",
        "gpu",
        "thermal",
        "processes",
        "unsupported_capabilities",
    ] {
        assert!(object.contains_key(key), "missing {key}");
    }
    assert_eq!(object.len(), 15);
    // Optional fields are explicit nulls, like the existing Status fields.
    assert!(encoded["power"]["value"]["cycle_count"].is_null());
    assert!(encoded["memory"]["value"]["commit_used_bytes"].is_u64());
    assert_eq!(encoded["cpu"]["value"]["frequency_mhz"], 4_520);
    assert_eq!(
        encoded["network"]["value"]["interfaces"][0]["kind"],
        "ethernet"
    );
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
            UnsupportedCode::BatteryHealth,
            UnsupportedCode::BatteryCycleCount,
            UnsupportedCode::BatteryRate,
        ]
    );
}

#[test]
fn snapshot_written_before_the_new_fields_still_decodes() {
    let mut legacy = serde_json::to_value(fixture_snapshot()).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.remove("host");
    object.remove("disk_activity");
    for (group, field) in [
        ("cpu", "frequency_mhz"),
        ("memory", "commit_used_bytes"),
        ("memory", "commit_limit_bytes"),
        ("power", "full_charge_basis_points"),
        ("power", "cycle_count"),
        ("power", "rate_milliwatts"),
    ] {
        object[group]["value"]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }
    object["gpu"]["value"]["adapters"][0]
        .as_object_mut()
        .unwrap()
        .remove("name");
    object["network"]["value"]["interfaces"][0]
        .as_object_mut()
        .unwrap()
        .remove("kind");
    let decoded: StatusSnapshotV1 = serde_json::from_value(legacy).expect("legacy V1 decodes");
    assert_eq!(decoded.host, AvailabilityV1::unavailable("not_sampled"));
    assert_eq!(
        decoded.disk_activity,
        AvailabilityV1::unavailable("not_sampled")
    );
    let AvailabilityV1::Available { value: cpu, .. } = &decoded.cpu else {
        panic!("cpu kept");
    };
    assert_eq!(cpu.frequency_mhz, None);
    assert_eq!(
        decoded.envelope_outcome(),
        fixture_snapshot().envelope_outcome()
    );
}

#[test]
fn availability_union_covers_all_five_states() {
    let available = AvailabilityV1::available(
        1,
        0,
        CpuV1 {
            system_utilization_basis_points: 1,
            frequency_mhz: Some(2_300),
        },
    );
    let partial = AvailabilityV1::partial(
        1,
        1,
        CpuV1 {
            system_utilization_basis_points: 1,
            frequency_mhz: None,
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
    let mut snapshot = fixture_snapshot();
    snapshot.cpu = AvailabilityV1::unavailable("zero_elapsed");
    snapshot.power = AvailabilityV1::unavailable("api_unavailable");
    snapshot.gpu = AvailabilityV1::unavailable("counter_missing");
    snapshot.thermal = AvailabilityV1::unavailable("no_instances");
    snapshot.disk_activity = AvailabilityV1::unavailable("counter_missing");
    let listed = unsupported_capabilities(&snapshot);
    assert_eq!(listed.len(), 11);
    for capability in listed {
        assert_eq!(capability.state, UnsupportedCapabilityState::Unsupported);
        assert_eq!(capability.reason_code, "not_supported_v1");
    }
}

#[test]
fn new_field_codes_follow_their_field_value() {
    let codes = |snapshot: &StatusSnapshotV1| {
        unsupported_capabilities(snapshot)
            .into_iter()
            .map(|capability| capability.code)
            .collect::<Vec<_>>()
    };
    let mut snapshot = fixture_snapshot();
    snapshot.power = AvailabilityV1::available(
        1,
        0,
        PowerV1 {
            battery_present: true,
            ac_state: AcState::Offline,
            charge_basis_points: Some(9_000),
            remaining_seconds: Some(3_600),
            full_charge_basis_points: Some(10_200),
            cycle_count: Some(42),
            rate_milliwatts: Some(-9_500),
        },
    );
    assert_eq!(
        codes(&snapshot),
        [
            UnsupportedCode::Vram,
            UnsupportedCode::Thermal,
            UnsupportedCode::Fan,
            UnsupportedCode::Smart,
        ]
    );

    let mut missing = snapshot.clone();
    missing.disk_activity = AvailabilityV1::unavailable("counter_missing");
    if let AvailabilityV1::Available { value, .. } = &mut missing.cpu {
        value.frequency_mhz = None;
    }
    if let AvailabilityV1::Available { value, .. } = &mut missing.power {
        value.cycle_count = None;
    }
    if let AvailabilityV1::Available { value, .. } = &mut missing.gpu {
        value.adapters.push(GpuAdapterV1 {
            adapter_id: "luid_0x00000000_0x0001DA3F_phys_0".into(),
            utilization_basis_points: 0,
            name: None,
        });
    }
    assert_eq!(
        codes(&missing),
        [
            UnsupportedCode::Vram,
            UnsupportedCode::Thermal,
            UnsupportedCode::Fan,
            UnsupportedCode::Smart,
            UnsupportedCode::PhysicalDiskActivity,
            UnsupportedCode::CpuFrequency,
            UnsupportedCode::BatteryCycleCount,
            UnsupportedCode::GpuName,
        ]
    );
    // A missing group lists every field code it carries.
    let mut gone = snapshot.clone();
    gone.cpu = AvailabilityV1::unavailable("zero_elapsed");
    gone.power = AvailabilityV1::permission_denied("access_denied");
    gone.gpu = AvailabilityV1::unavailable("counter_missing");
    let listed = codes(&gone);
    for code in [
        UnsupportedCode::GpuUtilization,
        UnsupportedCode::CpuFrequency,
        UnsupportedCode::BatteryHealth,
        UnsupportedCode::BatteryCycleCount,
        UnsupportedCode::BatteryRate,
        UnsupportedCode::GpuName,
    ] {
        assert!(listed.contains(&code), "{code:?}");
    }
}

#[test]
fn host_and_disk_activity_do_not_degrade_the_outcome() {
    let mut snapshot = fixture_snapshot();
    snapshot.memory = AvailabilityV1::available(
        1,
        0,
        MemoryV1 {
            total_bytes: 2,
            available_bytes: 1,
            used_bytes: 1,
            commit_used_bytes: None,
            commit_limit_bytes: None,
        },
    );
    snapshot.processes = AvailabilityV1::unsupported("platform_unsupported");
    snapshot.host = AvailabilityV1::unavailable("api_unavailable");
    snapshot.disk_activity = AvailabilityV1::unavailable("counter_missing");
    assert_eq!(snapshot.envelope_outcome(), "success");
    snapshot.host = AvailabilityV1::partial(
        1,
        0,
        HostV1 {
            cpu_model: None,
            os_version: Some("10.0.26300".into()),
            boot_time_unix_ms: Some(1),
        },
        vec!["cpu_model_unavailable".into()],
    );
    assert_eq!(snapshot.envelope_outcome(), "success");
    assert_eq!(snapshot.envelope_warnings(), ["cpu_model_unavailable"]);
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
                name: Some("Fixture GPU".into()),
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
        let mut snapshot = fixture_snapshot();
        snapshot.gpu = gpu.clone();
        snapshot.thermal = thermal.clone();
        snapshot.disk_activity = AvailabilityV1::unavailable("counter_missing");
        unsupported_capabilities(&snapshot)
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
            UnsupportedCode::BatteryHealth,
            UnsupportedCode::BatteryCycleCount,
            UnsupportedCode::BatteryRate,
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
            commit_used_bytes: Some(1),
            commit_limit_bytes: Some(4),
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
    let mut snapshot = StatusSnapshotV1 {
        snapshot_id: "snapshot-fixture".into(),
        sampled_at_unix_ms: 1_788_019_200_000,
        sample_window_ms: 1_000,
        logical_processor_count: 16,
        host: AvailabilityV1::available(
            1_788_019_200_000,
            0,
            HostV1 {
                cpu_model: Some("Fixture CPU @ 2.30GHz".into()),
                os_version: Some("10.0.26300.9974 (26H2)".into()),
                boot_time_unix_ms: Some(1_788_000_000_000),
            },
        ),
        cpu: AvailabilityV1::available(
            1_788_019_200_000,
            0,
            CpuV1 {
                system_utilization_basis_points: 4_321,
                frequency_mhz: Some(4_520),
            },
        ),
        memory: AvailabilityV1::partial(
            1_788_019_200_000,
            1,
            MemoryV1 {
                total_bytes: 34_359_738_368,
                available_bytes: 17_179_869_184,
                used_bytes: 17_179_869_184,
                commit_used_bytes: Some(21_474_836_480),
                commit_limit_bytes: Some(42_949_672_960),
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
                    kind: Some(NetworkKind::Ethernet),
                }],
            },
        ),
        disk_activity: AvailabilityV1::available(
            1_788_019_200_000,
            0,
            DiskActivityV1 {
                read_bytes_per_second: 1_048_576,
                write_bytes_per_second: 8_388_608,
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
                full_charge_basis_points: None,
                cycle_count: None,
                rate_milliwatts: None,
            },
        ),
        gpu: AvailabilityV1::available(
            1_788_019_200_000,
            0,
            GpuV1 {
                adapters: vec![GpuAdapterV1 {
                    adapter_id: "luid_0x00000000_0x0000C0B6_phys_0".into(),
                    utilization_basis_points: 1_250,
                    name: Some("Fixture GPU".into()),
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
        unsupported_capabilities: Vec::new(),
    };
    snapshot.unsupported_capabilities = unsupported_capabilities(&snapshot);
    snapshot
}
