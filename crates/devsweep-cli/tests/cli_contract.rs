use std::{
    fs,
    io::Read,
    path::Path,
    process::{Command, Output, Stdio},
};

use tempfile::TempDir;

fn devsweep(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("devsweep process runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

#[test]
fn help_exposes_only_the_frozen_roots_in_english_and_chinese() {
    let english = devsweep(&["--help"]);
    assert_eq!(english.status.code(), Some(0));
    let english = stdout(&english);
    for root in [
        "clean", "software", "optimize", "analyze", "status", "history",
    ] {
        assert!(english.contains(root), "English help omits {root}");
    }
    for removed in ["tui", "scan [ROOT]", "inventory [ROOT]", "protect add"] {
        assert!(!english.contains(removed), "English help retains {removed}");
    }

    let chinese = devsweep(&["--language", "zh-CN", "--help"]);
    assert_eq!(chinese.status.code(), Some(0));
    let chinese = stdout(&chinese);
    assert!(chinese.contains("检查、规划"));
    assert!(chinese.contains("直接运行 devsweep"));
    for root in [
        "clean", "software", "optimize", "analyze", "status", "history",
    ] {
        assert!(chinese.contains(root), "Chinese help omits {root}");
    }

    for (path, expected) in [
        (
            ["clean", "execute"].as_slice(),
            [
                "上下文：devsweep clean execute",
                "--language <LANGUAGE>",
                "--preview-digest",
                "sha256:",
            ]
            .as_slice(),
        ),
        (
            ["status", "snapshot"].as_slice(),
            [
                "上下文：devsweep status snapshot",
                "--process-limit",
                "默认 15",
                "范围 1..100",
            ]
            .as_slice(),
        ),
        (
            ["history", "show"].as_slice(),
            ["上下文：devsweep history show", "--operation-id", "必需"].as_slice(),
        ),
    ] {
        let mut args = vec!["--language", "zh-CN"];
        args.extend_from_slice(path);
        args.push("--help");
        let nested = devsweep(&args);
        assert_eq!(
            nested.status.code(),
            Some(0),
            "Chinese nested help {path:?}"
        );
        let nested = stdout(&nested);
        for token in expected {
            assert!(
                nested.contains(token),
                "Chinese help {path:?} omits {token}"
            );
        }
    }

    let digest = format!("sha256:{}", "a".repeat(64));
    let invalid_flag_value = devsweep(&[
        "clean",
        "execute",
        "--plan",
        "plan.json",
        "--preview-digest",
        &digest,
        "--confirm",
        "true",
    ]);
    assert_eq!(invalid_flag_value.status.code(), Some(2));
    assert!(stderr(&invalid_flag_value).contains("unexpected argument 'true'"));
}

#[test]
fn explicit_chinese_language_localizes_human_errors_only() {
    let output = devsweep(&["--language", "zh-CN", "history", "list"]);
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("当前分阶段构建尚未接入"));
}

#[test]
fn removed_roots_and_authority_omissions_are_usage_exit_two() {
    for root in ["tui", "scan", "inventory", "protect", "rules"] {
        assert_eq!(
            devsweep(&[root]).status.code(),
            Some(2),
            "removed root {root}"
        );
    }
    assert_eq!(
        devsweep(&["clean", "execute", "--plan", "plan.json"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        devsweep(&["software", "uninstall", "--plan", "plan.json"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        devsweep(&["optimize", "run", "--plan", "plan.json"])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn every_legacy_migration_fixture_is_rejected_without_conversion() {
    let cases: Vec<Vec<String>> =
        serde_json::from_str(include_str!("fixtures/cli/migration-cases.json")).unwrap();
    for case in cases {
        let args = case.iter().map(String::as_str).collect::<Vec<_>>();
        assert_eq!(devsweep(&args).status.code(), Some(2), "legacy {case:?}");
    }

    let directory = TempDir::new().unwrap();
    let plan = directory.path().join("legacy-plan.json");
    let bytes = br#"{"version":1,"targets":[]}"#;
    fs::write(&plan, bytes).unwrap();
    let output = devsweep(&["clean", "--plan", plan.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read(plan).unwrap(), bytes);
}

#[test]
fn bare_redirected_invocation_and_human_live_name_the_tty_contract() {
    let bare = devsweep(&[]);
    assert_eq!(bare.status.code(), Some(2));
    assert!(stderr(&bare).contains("tty_required"));
    assert!(stderr(&bare).contains("explicit command"));

    let live = devsweep(&["status", "live"]);
    assert_eq!(live.status.code(), Some(2));
    assert!(stderr(&live).contains("--format ndjson"));
}

#[test]
fn json_and_ndjson_are_locale_neutral_and_keep_diagnostics_off_stderr() {
    let first = devsweep(&["status", "snapshot", "--format", "json"]);
    let second = devsweep(&["status", "snapshot", "--format", "json"]);
    let first_code = first.status.code();
    let second_code = second.status.code();
    assert!(
        first_code == Some(0) || first_code == Some(5),
        "snapshot exit {first_code:?}"
    );
    assert!(
        second_code == Some(0) || second_code == Some(5),
        "snapshot exit {second_code:?}"
    );
    assert!(first.stderr.is_empty());
    assert!(second.stderr.is_empty());
    let envelope: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["command"], "status.snapshot");
    assert!(envelope["error"].is_null());
    status_v1_contract::decode_snapshot_envelope(&envelope).expect("live snapshot is closed V1");

    let mut child = Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(["status", "live", "--format", "ndjson", "--interval", "1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("status live starts");
    let mut stdout = child.stdout.take().expect("live stdout");
    let mut buffer = [0_u8; 4096];
    let _ = stdout.read(&mut buffer);
    drop(stdout);
    let status = child.wait().expect("live exits after pipe close");
    assert_eq!(status.code(), Some(0));
}

#[test]
fn language_is_rejected_for_machine_and_plan_outputs() {
    let machine = devsweep(&[
        "--language",
        "zh-CN",
        "status",
        "snapshot",
        "--format",
        "json",
    ]);
    assert_eq!(machine.status.code(), Some(2));
    assert!(machine.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&machine.stdout).unwrap();
    assert_eq!(document["error"]["code"], "invalid_cli");

    let plan = devsweep(&[
        "--language",
        "en",
        "optimize",
        "plan",
        "--operation",
        "dns.flush",
        "--output",
        "plan.json",
    ]);
    assert_eq!(plan.status.code(), Some(2));
}

#[test]
fn create_new_refuses_collisions_without_changing_existing_bytes() {
    let directory = TempDir::new().unwrap();
    let output_path = directory.path().join("result.json");
    fs::write(&output_path, b"original bytes").unwrap();
    let output = devsweep(&[
        "status",
        "snapshot",
        "--format",
        "json",
        "--output",
        output_path.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(6));
    assert!(stderr(&output).contains("output_exists"));
    assert_eq!(fs::read(output_path).unwrap(), b"original bytes");
}

#[test]
fn dash_file_sentinels_and_same_input_output_are_rejected() {
    for args in [
        vec!["status", "snapshot", "--output", "-"],
        vec!["clean", "preview", "--plan", "-"],
        vec![
            "clean",
            "preview",
            "--plan",
            "same.json",
            "--output",
            "same.json",
        ],
    ] {
        assert_eq!(devsweep(&args).status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn legacy_audit_paths_are_not_accepted_discovered_or_modified() {
    let directory = TempDir::new().unwrap();
    let arbitrary = directory.path().join("old-audit.jsonl");
    let legacy_default = directory.path().join("devsweep").join("audit.jsonl");
    fs::create_dir_all(legacy_default.parent().unwrap()).unwrap();
    fs::write(&arbitrary, b"arbitrary legacy bytes\n").unwrap();
    fs::write(&legacy_default, b"default legacy bytes\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(["clean", "--audit-log", arbitrary.to_str().unwrap()])
        .env("APPDATA", directory.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read(arbitrary).unwrap(), b"arbitrary legacy bytes\n");
    assert_eq!(fs::read(legacy_default).unwrap(), b"default legacy bytes\n");
}

#[test]
fn status_wire_fixtures_are_closed_typed_and_cli_tauri_exact() {
    status_v1_contract::verify_canonical_fixtures();
    status_v1_contract::verify_closed_schema_rejections();
}

#[test]
fn reference_and_migration_documents_cover_the_breaking_surface() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let english = fs::read_to_string(root.join("docs/reference/cli.md")).unwrap();
    let chinese = fs::read_to_string(root.join("docs/zh/reference/cli.md")).unwrap();
    let migration = fs::read_to_string(root.join("docs/guide/cli-migration.md")).unwrap();
    for command in [
        "clean scan",
        "clean plan",
        "software inventory",
        "software uninstall",
        "optimize run",
        "analyze scan",
        "status snapshot",
        "status live",
        "history show",
    ] {
        assert!(
            english.contains(command),
            "English reference omits {command}"
        );
        assert!(
            chinese.contains(command),
            "Chinese reference omits {command}"
        );
    }
    for legacy in [
        "devsweep tui",
        "devsweep scan",
        "devsweep inventory",
        "--audit-log",
        "devsweep protect",
        "devsweep rules",
        "%APPDATA%\\devsweep\\audit.jsonl",
    ] {
        assert!(migration.contains(legacy), "migration guide omits {legacy}");
    }
    assert!(migration.contains("no automatic discovery, import, conversion, or replay"));
}

mod status_v1_contract {
    #![allow(dead_code)]

    use std::collections::BTreeSet;

    use serde_json::{Map, Value};

    type DecodeResult<T> = Result<T, String>;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct SnapshotEnvelopeV1 {
        outcome: Outcome,
        data: StatusSnapshotV1,
        warnings: Vec<String>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Outcome {
        Success,
        Partial,
        Failed,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct StatusSnapshotV1 {
        snapshot_id: String,
        sampled_at_unix_ms: u64,
        sample_window_ms: u32,
        logical_processor_count: u32,
        cpu: AvailabilityV1<CpuV1>,
        memory: AvailabilityV1<MemoryV1>,
        volumes: AvailabilityV1<VolumesV1>,
        network: AvailabilityV1<NetworkV1>,
        power: AvailabilityV1<PowerV1>,
        processes: AvailabilityV1<ProcessesV1>,
        unsupported_capabilities: Vec<UnsupportedCapabilityV1>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum AvailabilityV1<T> {
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
            reason_code: String,
        },
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct CpuV1 {
        system_utilization_basis_points: u32,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MemoryV1 {
        total_bytes: u64,
        available_bytes: u64,
        used_bytes: u64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct VolumesV1 {
        items: Vec<VolumeV1>,
        complete: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct VolumeV1 {
        volume_id: String,
        mount_points: Vec<String>,
        total_bytes: u64,
        available_bytes: u64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct NetworkV1 {
        interval_ms: u32,
        interfaces: Vec<NetworkInterfaceV1>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct NetworkInterfaceV1 {
        interface_luid: String,
        name: String,
        rx_bytes_per_second: u64,
        tx_bytes_per_second: u64,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum AcState {
        Online,
        Offline,
        Unknown,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct PowerV1 {
        battery_present: bool,
        ac_state: AcState,
        charge_basis_points: Option<u32>,
        remaining_seconds: Option<u64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ProcessesV1 {
        items: Vec<ProcessV1>,
        enumerated_count: u32,
        returned_count: u32,
        requested_limit: u32,
        enumeration_ceiling: u32,
        detail_budget_ms: u32,
        truncated_by_limit: bool,
        budget_exhausted: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ProcessV1 {
        pid: u32,
        name: String,
        cpu_basis_points_of_one_logical_core: u32,
        private_bytes: u64,
        read_bytes_per_second: u64,
        write_bytes_per_second: u64,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum UnsupportedCode {
        GpuUtilization,
        Vram,
        Thermal,
        Fan,
        Smart,
        PhysicalDiskActivity,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct UnsupportedCapabilityV1 {
        code: UnsupportedCode,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum StatusEventV1 {
        Started {
            common: EventCommon,
            interval_ms: u32,
            process_limit: u32,
        },
        Snapshot {
            common: EventCommon,
            data: Box<StatusSnapshotV1>,
        },
        TickSkipped {
            common: EventCommon,
            skipped_total: u64,
        },
        Terminal {
            common: EventCommon,
            reason: TerminalReason,
            error_code: Option<String>,
        },
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct EventCommon {
        operation_id: String,
        sequence: u64,
        emitted_at_unix_ms: u64,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TerminalReason {
        Completed,
        Canceled,
        BrokenPipe,
        ProducerError,
    }

    struct ClosedObject {
        context: String,
        fields: Map<String, Value>,
    }

    impl ClosedObject {
        fn decode(value: Value, context: impl Into<String>) -> DecodeResult<Self> {
            let context = context.into();
            let fields = value
                .as_object()
                .cloned()
                .ok_or_else(|| format!("{context} must be an object"))?;
            Ok(Self { context, fields })
        }

        fn take(&mut self, key: &str) -> DecodeResult<Value> {
            self.fields
                .remove(key)
                .ok_or_else(|| format!("{} missing {key}", self.context))
        }

        fn take_string(&mut self, key: &str) -> DecodeResult<String> {
            self.take(key)?
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{}.{} must be a string", self.context, key))
        }

        fn take_nonempty_string(&mut self, key: &str) -> DecodeResult<String> {
            let value = self.take_string(key)?;
            if value.is_empty() {
                return Err(format!("{}.{} must not be empty", self.context, key));
            }
            Ok(value)
        }

        fn take_identifier(&mut self, key: &str) -> DecodeResult<String> {
            let value = self.take_nonempty_string(key)?;
            if !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            {
                return Err(format!(
                    "{}.{} must be a closed ASCII identifier",
                    self.context, key
                ));
            }
            Ok(value)
        }

        fn take_u64(&mut self, key: &str) -> DecodeResult<u64> {
            self.take(key)?.as_u64().ok_or_else(|| {
                format!("{}.{} must be a nonnegative u64 integer", self.context, key)
            })
        }

        fn take_u32(&mut self, key: &str) -> DecodeResult<u32> {
            u32::try_from(self.take_u64(key)?)
                .map_err(|_| format!("{}.{} must fit u32", self.context, key))
        }

        fn take_bool(&mut self, key: &str) -> DecodeResult<bool> {
            self.take(key)?
                .as_bool()
                .ok_or_else(|| format!("{}.{} must be a boolean", self.context, key))
        }

        fn take_nullable_u64(&mut self, key: &str) -> DecodeResult<Option<u64>> {
            let value = self.take(key)?;
            if value.is_null() {
                Ok(None)
            } else {
                value.as_u64().map(Some).ok_or_else(|| {
                    format!(
                        "{}.{} must be null or a nonnegative u64 integer",
                        self.context, key
                    )
                })
            }
        }

        fn take_nullable_u32(&mut self, key: &str) -> DecodeResult<Option<u32>> {
            self.take_nullable_u64(key)?
                .map(|value| {
                    u32::try_from(value)
                        .map_err(|_| format!("{}.{} must fit u32", self.context, key))
                })
                .transpose()
        }

        fn take_null(&mut self, key: &str) -> DecodeResult<()> {
            if self.take(key)?.is_null() {
                Ok(())
            } else {
                Err(format!("{}.{} must be null", self.context, key))
            }
        }

        fn take_array<T, F>(&mut self, key: &str, mut decode: F) -> DecodeResult<Vec<T>>
        where
            F: FnMut(Value, usize) -> DecodeResult<T>,
        {
            self.take(key)?
                .as_array()
                .cloned()
                .ok_or_else(|| format!("{}.{} must be an array", self.context, key))?
                .into_iter()
                .enumerate()
                .map(|(index, value)| decode(value, index))
                .collect()
        }

        fn finish(self) -> DecodeResult<()> {
            if self.fields.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "{} has unknown fields {:?}",
                    self.context,
                    self.fields.keys().collect::<Vec<_>>()
                ))
            }
        }
    }

    impl SnapshotEnvelopeV1 {
        fn decode(value: Value) -> DecodeResult<Self> {
            let mut object = ClosedObject::decode(value, "status snapshot envelope")?;
            if object.take_u32("schema_version")? != 1 {
                return Err("status snapshot schema_version must be 1".to_string());
            }
            if object.take_string("command")? != "status.snapshot" {
                return Err("status snapshot command must be status.snapshot".to_string());
            }
            let outcome = match object.take_string("outcome")?.as_str() {
                "success" => Outcome::Success,
                "partial" => Outcome::Partial,
                "failed" => Outcome::Failed,
                other => return Err(format!("unsupported snapshot outcome {other}")),
            };
            let data = StatusSnapshotV1::decode(object.take("data")?)?;
            let expected_outcome = if data.supported_group_is_degraded() {
                Outcome::Partial
            } else {
                Outcome::Success
            };
            if outcome != expected_outcome {
                return Err(
                    "status snapshot outcome must reflect supported-group availability".to_string(),
                );
            }
            let warnings = object.take_array("warnings", |value, index| {
                let warning = value
                    .as_str()
                    .ok_or_else(|| format!("warning {index} must be a string"))?
                    .to_string();
                require_identifier(&warning, &format!("warning {index}"))?;
                Ok(warning)
            })?;
            object.take_null("error")?;
            object.finish()?;
            Ok(Self {
                outcome,
                data,
                warnings,
            })
        }
    }

    impl StatusSnapshotV1 {
        fn supported_group_is_degraded(&self) -> bool {
            self.cpu.is_degraded()
                || self.memory.is_degraded()
                || self.volumes.is_degraded()
                || self.network.is_degraded()
                || self.power.is_degraded()
                || self.processes.is_degraded()
        }

        fn decode(value: Value) -> DecodeResult<Self> {
            let mut object = ClosedObject::decode(value, "StatusSnapshotV1")?;
            let snapshot_id = object.take_nonempty_string("snapshot_id")?;
            let sampled_at_unix_ms = object.take_u64("sampled_at_unix_ms")?;
            let sample_window_ms = object.take_u32("sample_window_ms")?;
            let logical_processor_count = object.take_u32("logical_processor_count")?;
            if logical_processor_count == 0 {
                return Err("logical_processor_count must be positive".to_string());
            }
            let cpu = decode_availability(object.take("cpu")?, "cpu", decode_cpu)?;
            let memory = decode_availability(object.take("memory")?, "memory", decode_memory)?;
            let volumes = decode_availability(object.take("volumes")?, "volumes", decode_volumes)?;
            let network = decode_availability(object.take("network")?, "network", decode_network)?;
            let power = decode_availability(object.take("power")?, "power", decode_power)?;
            let processes =
                decode_availability(object.take("processes")?, "processes", decode_processes)?;
            validate_process_availability(&processes)?;
            let unsupported_capabilities = object
                .take_array("unsupported_capabilities", |value, index| {
                    decode_unsupported(value, index)
                })?;
            let unique = unsupported_capabilities
                .iter()
                .map(|capability| capability.code)
                .collect::<BTreeSet<_>>();
            let expected = BTreeSet::from([
                UnsupportedCode::GpuUtilization,
                UnsupportedCode::Vram,
                UnsupportedCode::Thermal,
                UnsupportedCode::Fan,
                UnsupportedCode::Smart,
                UnsupportedCode::PhysicalDiskActivity,
            ]);
            if unsupported_capabilities.len() != expected.len() || unique != expected {
                return Err(
                    "Status V1 must contain each of the six unsupported capabilities exactly once"
                        .to_string(),
                );
            }
            object.finish()?;
            Ok(Self {
                snapshot_id,
                sampled_at_unix_ms,
                sample_window_ms,
                logical_processor_count,
                cpu,
                memory,
                volumes,
                network,
                power,
                processes,
                unsupported_capabilities,
            })
        }
    }

    impl<T> AvailabilityV1<T> {
        fn is_degraded(&self) -> bool {
            matches!(
                self,
                Self::Partial { .. } | Self::Unavailable { .. } | Self::PermissionDenied { .. }
            )
        }
    }

    fn validate_process_availability(processes: &AvailabilityV1<ProcessesV1>) -> DecodeResult<()> {
        match processes {
            AvailabilityV1::Available { value, .. } => {
                if value.truncated_by_limit || value.budget_exhausted {
                    return Err(
                        "truncated or budget-exhausted processes must be partial".to_string()
                    );
                }
            }
            AvailabilityV1::Partial {
                value,
                reason_codes,
                ..
            } => {
                if value.truncated_by_limit
                    && !reason_codes.iter().any(|code| code == "process_limit")
                {
                    return Err("process_limit reason is required for limit truncation".to_string());
                }
                if value.budget_exhausted
                    && !reason_codes
                        .iter()
                        .any(|code| code == "detail_budget_exhausted")
                {
                    return Err(
                        "detail_budget_exhausted reason is required for budget truncation"
                            .to_string(),
                    );
                }
            }
            AvailabilityV1::Unavailable { .. }
            | AvailabilityV1::PermissionDenied { .. }
            | AvailabilityV1::Unsupported { .. } => {}
        }
        Ok(())
    }

    fn decode_availability<T, F>(
        value: Value,
        context: &str,
        decode_value: F,
    ) -> DecodeResult<AvailabilityV1<T>>
    where
        F: FnOnce(Value) -> DecodeResult<T>,
    {
        let mut object = ClosedObject::decode(value, format!("AvailabilityV1<{context}>"))?;
        let state = object.take_string("state")?;
        let result = match state.as_str() {
            "available" => AvailabilityV1::Available {
                sampled_at_unix_ms: object.take_u64("sampled_at_unix_ms")?,
                age_ms: object.take_u64("age_ms")?,
                value: decode_value(object.take("value")?)?,
            },
            "partial" => {
                let reason_codes = object.take_array("reason_codes", |value, index| {
                    let reason = value
                        .as_str()
                        .ok_or_else(|| format!("{context} reason {index} must be a string"))?
                        .to_string();
                    require_identifier(&reason, &format!("{context} reason {index}"))?;
                    Ok(reason)
                })?;
                if reason_codes.is_empty() {
                    return Err(format!("{context} partial reason_codes must not be empty"));
                }
                AvailabilityV1::Partial {
                    sampled_at_unix_ms: object.take_u64("sampled_at_unix_ms")?,
                    age_ms: object.take_u64("age_ms")?,
                    value: decode_value(object.take("value")?)?,
                    reason_codes,
                }
            }
            "unavailable" => AvailabilityV1::Unavailable {
                sampled_at_unix_ms: object.take_nullable_u64("sampled_at_unix_ms")?,
                reason_code: take_reason_code(&mut object)?,
            },
            "permission_denied" => AvailabilityV1::PermissionDenied {
                sampled_at_unix_ms: object.take_nullable_u64("sampled_at_unix_ms")?,
                reason_code: take_reason_code(&mut object)?,
            },
            "unsupported" => {
                object.take_null("sampled_at_unix_ms")?;
                AvailabilityV1::Unsupported {
                    reason_code: take_reason_code(&mut object)?,
                }
            }
            other => {
                return Err(format!(
                    "{context} has unsupported availability state {other}"
                ));
            }
        };
        object.finish()?;
        Ok(result)
    }

    fn take_reason_code(object: &mut ClosedObject) -> DecodeResult<String> {
        let reason = object.take_identifier("reason_code")?;
        Ok(reason)
    }

    fn decode_cpu(value: Value) -> DecodeResult<CpuV1> {
        let mut object = ClosedObject::decode(value, "CpuV1")?;
        let system_utilization_basis_points = object.take_u32("system_utilization_basis_points")?;
        if system_utilization_basis_points > 10_000 {
            return Err("system CPU basis points must be 0..10000".to_string());
        }
        object.finish()?;
        Ok(CpuV1 {
            system_utilization_basis_points,
        })
    }

    fn decode_memory(value: Value) -> DecodeResult<MemoryV1> {
        let mut object = ClosedObject::decode(value, "MemoryV1")?;
        let total_bytes = object.take_u64("total_bytes")?;
        let available_bytes = object.take_u64("available_bytes")?;
        let used_bytes = object.take_u64("used_bytes")?;
        if available_bytes > total_bytes || used_bytes != total_bytes - available_bytes {
            return Err("memory byte fields must be internally consistent".to_string());
        }
        object.finish()?;
        Ok(MemoryV1 {
            total_bytes,
            available_bytes,
            used_bytes,
        })
    }

    fn decode_volumes(value: Value) -> DecodeResult<VolumesV1> {
        let mut object = ClosedObject::decode(value, "VolumesV1")?;
        let items = object.take_array("items", |value, index| {
            let mut item = ClosedObject::decode(value, format!("VolumeV1[{index}]"))?;
            let volume_id = item.take_nonempty_string("volume_id")?;
            let mount_points = item.take_array("mount_points", |value, mount_index| {
                value
                    .as_str()
                    .filter(|mount| !mount.is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| format!("mount point {mount_index} must be a nonempty string"))
            })?;
            let total_bytes = item.take_u64("total_bytes")?;
            let available_bytes = item.take_u64("available_bytes")?;
            if available_bytes > total_bytes {
                return Err(format!(
                    "VolumeV1[{index}] available_bytes exceeds total_bytes"
                ));
            }
            item.finish()?;
            Ok(VolumeV1 {
                volume_id,
                mount_points,
                total_bytes,
                available_bytes,
            })
        })?;
        let complete = object.take_bool("complete")?;
        object.finish()?;
        Ok(VolumesV1 { items, complete })
    }

    fn decode_network(value: Value) -> DecodeResult<NetworkV1> {
        let mut object = ClosedObject::decode(value, "NetworkV1")?;
        let interval_ms = object.take_u32("interval_ms")?;
        if interval_ms == 0 {
            return Err("network interval_ms must be positive".to_string());
        }
        let interfaces = object.take_array("interfaces", |value, index| {
            let mut item = ClosedObject::decode(value, format!("NetworkInterfaceV1[{index}]"))?;
            let interface_luid = item.take_nonempty_string("interface_luid")?;
            interface_luid.parse::<u64>().map_err(|_| {
                format!("NetworkInterfaceV1[{index}] interface_luid must be decimal u64")
            })?;
            let name = item.take_nonempty_string("name")?;
            let rx_bytes_per_second = item.take_u64("rx_bytes_per_second")?;
            let tx_bytes_per_second = item.take_u64("tx_bytes_per_second")?;
            item.finish()?;
            Ok(NetworkInterfaceV1 {
                interface_luid,
                name,
                rx_bytes_per_second,
                tx_bytes_per_second,
            })
        })?;
        object.finish()?;
        Ok(NetworkV1 {
            interval_ms,
            interfaces,
        })
    }

    fn decode_power(value: Value) -> DecodeResult<PowerV1> {
        let mut object = ClosedObject::decode(value, "PowerV1")?;
        let battery_present = object.take_bool("battery_present")?;
        let ac_state = match object.take_string("ac_state")?.as_str() {
            "online" => AcState::Online,
            "offline" => AcState::Offline,
            "unknown" => AcState::Unknown,
            other => return Err(format!("unsupported ac_state {other}")),
        };
        let charge_basis_points = object.take_nullable_u32("charge_basis_points")?;
        if charge_basis_points.is_some_and(|charge| charge > 10_000) {
            return Err("charge_basis_points must be 0..10000 or null".to_string());
        }
        let remaining_seconds = object.take_nullable_u64("remaining_seconds")?;
        if !battery_present && (charge_basis_points.is_some() || remaining_seconds.is_some()) {
            return Err(
                "battery-absent power values must use null charge and remaining time".to_string(),
            );
        }
        object.finish()?;
        Ok(PowerV1 {
            battery_present,
            ac_state,
            charge_basis_points,
            remaining_seconds,
        })
    }

    fn decode_processes(value: Value) -> DecodeResult<ProcessesV1> {
        let mut object = ClosedObject::decode(value, "ProcessesV1")?;
        let items = object.take_array("items", |value, index| {
            let mut item = ClosedObject::decode(value, format!("ProcessV1[{index}]"))?;
            let process = ProcessV1 {
                pid: item.take_u32("pid")?,
                name: item.take_nonempty_string("name")?,
                cpu_basis_points_of_one_logical_core: item
                    .take_u32("cpu_basis_points_of_one_logical_core")?,
                private_bytes: item.take_u64("private_bytes")?,
                read_bytes_per_second: item.take_u64("read_bytes_per_second")?,
                write_bytes_per_second: item.take_u64("write_bytes_per_second")?,
            };
            item.finish()?;
            Ok(process)
        })?;
        let enumerated_count = object.take_u32("enumerated_count")?;
        let returned_count = object.take_u32("returned_count")?;
        let requested_limit = object.take_u32("requested_limit")?;
        let enumeration_ceiling = object.take_u32("enumeration_ceiling")?;
        let detail_budget_ms = object.take_u32("detail_budget_ms")?;
        let truncated_by_limit = object.take_bool("truncated_by_limit")?;
        let budget_exhausted = object.take_bool("budget_exhausted")?;
        if enumeration_ceiling != 4096 || detail_budget_ms != 150 {
            return Err("process V1 ceiling/budget constants drifted".to_string());
        }
        if returned_count as usize != items.len() || returned_count > requested_limit {
            return Err("process returned_count must match rows and requested_limit".to_string());
        }
        if enumerated_count > enumeration_ceiling {
            return Err("process enumerated_count must not exceed the V1 ceiling".to_string());
        }
        if truncated_by_limit != (enumerated_count > requested_limit) {
            return Err(
                "truncated_by_limit must exactly reflect enumerated rows beyond requested_limit"
                    .to_string(),
            );
        }
        object.finish()?;
        Ok(ProcessesV1 {
            items,
            enumerated_count,
            returned_count,
            requested_limit,
            enumeration_ceiling,
            detail_budget_ms,
            truncated_by_limit,
            budget_exhausted,
        })
    }

    fn decode_unsupported(value: Value, index: usize) -> DecodeResult<UnsupportedCapabilityV1> {
        let mut object = ClosedObject::decode(value, format!("UnsupportedCapabilityV1[{index}]"))?;
        let code = match object.take_string("code")?.as_str() {
            "gpu_utilization" => UnsupportedCode::GpuUtilization,
            "vram" => UnsupportedCode::Vram,
            "thermal" => UnsupportedCode::Thermal,
            "fan" => UnsupportedCode::Fan,
            "smart" => UnsupportedCode::Smart,
            "physical_disk_activity" => UnsupportedCode::PhysicalDiskActivity,
            other => return Err(format!("unsupported static capability code {other}")),
        };
        if object.take_string("state")? != "unsupported"
            || object.take_string("reason_code")? != "not_supported_v1"
        {
            return Err("static capability must be unsupported/not_supported_v1".to_string());
        }
        object.finish()?;
        Ok(UnsupportedCapabilityV1 { code })
    }

    impl StatusEventV1 {
        fn decode(value: Value) -> DecodeResult<Self> {
            let mut object = ClosedObject::decode(value, "StatusEventV1")?;
            if object.take_u32("schema_version")? != 1 {
                return Err("status event schema_version must be 1".to_string());
            }
            let event = object.take_string("event")?;
            let common = EventCommon {
                operation_id: object.take_nonempty_string("operation_id")?,
                sequence: object.take_u64("sequence")?,
                emitted_at_unix_ms: object.take_u64("emitted_at_unix_ms")?,
            };
            let data = object.take("data")?;
            object.finish()?;
            match event.as_str() {
                "status_started" => {
                    let mut data = ClosedObject::decode(data, "StatusStartedV1.data")?;
                    let interval_ms = data.take_u32("interval_ms")?;
                    let process_limit = data.take_u32("process_limit")?;
                    if !(1_000..=60_000).contains(&interval_ms)
                        || interval_ms % 1_000 != 0
                        || !(1..=100).contains(&process_limit)
                    {
                        return Err("status_started interval/limit outside V1 bounds".to_string());
                    }
                    data.finish()?;
                    Ok(Self::Started {
                        common,
                        interval_ms,
                        process_limit,
                    })
                }
                "status_snapshot" => Ok(Self::Snapshot {
                    common,
                    data: Box::new(StatusSnapshotV1::decode(data)?),
                }),
                "tick_skipped" => {
                    let mut data = ClosedObject::decode(data, "TickSkippedV1.data")?;
                    if data.take_string("reason")? != "sample_in_flight" {
                        return Err("tick_skipped reason must be sample_in_flight".to_string());
                    }
                    let skipped_total = data.take_u64("skipped_total")?;
                    data.finish()?;
                    Ok(Self::TickSkipped {
                        common,
                        skipped_total,
                    })
                }
                "status_terminal" => {
                    let mut data = ClosedObject::decode(data, "StatusTerminalV1.data")?;
                    let reason = decode_terminal_reason(&data.take_string("reason")?)?;
                    let error_code = match data.take("error_code")? {
                        Value::Null => None,
                        Value::String(code) => {
                            require_identifier(&code, "terminal error_code")?;
                            Some(code)
                        }
                        _ => return Err("terminal error_code must be string or null".to_string()),
                    };
                    if (reason == TerminalReason::ProducerError) != error_code.is_some() {
                        return Err(
                            "only producer_error terminals carry a non-null error_code".to_string()
                        );
                    }
                    data.finish()?;
                    Ok(Self::Terminal {
                        common,
                        reason,
                        error_code,
                    })
                }
                other => Err(format!("unsupported status event {other}")),
            }
        }

        fn common(&self) -> &EventCommon {
            match self {
                Self::Started { common, .. }
                | Self::Snapshot { common, .. }
                | Self::TickSkipped { common, .. }
                | Self::Terminal { common, .. } => common,
            }
        }
    }

    fn decode_terminal_reason(value: &str) -> DecodeResult<TerminalReason> {
        match value {
            "completed" => Ok(TerminalReason::Completed),
            "canceled" => Ok(TerminalReason::Canceled),
            "broken_pipe" => Ok(TerminalReason::BrokenPipe),
            "producer_error" => Ok(TerminalReason::ProducerError),
            other => Err(format!("unsupported terminal reason {other}")),
        }
    }

    fn require_identifier(value: &str, context: &str) -> DecodeResult<()> {
        if !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            Ok(())
        } else {
            Err(format!("{context} must be a closed ASCII identifier"))
        }
    }

    fn decode_stream(source: &str) -> DecodeResult<Vec<StatusEventV1>> {
        let events = source
            .lines()
            .enumerate()
            .map(|(line, source)| {
                serde_json::from_str::<Value>(source)
                    .map_err(|error| format!("NDJSON line {} is invalid: {error}", line + 1))
                    .and_then(StatusEventV1::decode)
            })
            .collect::<DecodeResult<Vec<_>>>()?;
        if events.is_empty() {
            return Err("status stream must not be empty".to_string());
        }
        let operation_id = events[0].common().operation_id.clone();
        for (sequence, event) in events.iter().enumerate() {
            if event.common().operation_id != operation_id
                || event.common().sequence != sequence as u64
            {
                return Err("status stream operation_id/sequence is not exact".to_string());
            }
        }
        if !matches!(events.first(), Some(StatusEventV1::Started { .. })) {
            return Err("status stream must start with status_started".to_string());
        }
        if !matches!(events.last(), Some(StatusEventV1::Terminal { .. })) {
            return Err("status stream must end with exactly one terminal".to_string());
        }
        if events
            .iter()
            .filter(|event| matches!(event, StatusEventV1::Terminal { .. }))
            .count()
            != 1
        {
            return Err("status stream must contain exactly one terminal".to_string());
        }
        Ok(events)
    }

    pub(super) fn decode_snapshot_envelope(value: &Value) -> DecodeResult<()> {
        SnapshotEnvelopeV1::decode(value.clone()).map(|_| ())
    }

    pub(super) fn verify_canonical_fixtures() {
        let cli_snapshot_source = include_str!("fixtures/cli/status-snapshot.json");
        let tauri_snapshot_source = include_str!("fixtures/cli/status-snapshot.tauri.json");
        assert_eq!(cli_snapshot_source, tauri_snapshot_source);
        let cli_snapshot = SnapshotEnvelopeV1::decode(
            serde_json::from_str(cli_snapshot_source).expect("CLI snapshot fixture JSON"),
        )
        .expect("CLI snapshot satisfies closed Status V1");
        let tauri_snapshot = SnapshotEnvelopeV1::decode(
            serde_json::from_str(tauri_snapshot_source).expect("Tauri snapshot fixture JSON"),
        )
        .expect("Tauri snapshot satisfies closed Status V1");
        assert_eq!(cli_snapshot, tauri_snapshot);
        assert_eq!(cli_snapshot.data.sample_window_ms, 1000);
        assert_eq!(cli_snapshot.data.logical_processor_count, 16);
        let AvailabilityV1::Available { value: network, .. } = &cli_snapshot.data.network else {
            panic!("network fixture must be available");
        };
        assert_eq!(network.interval_ms, 1000);
        assert_eq!(network.interfaces[0].interface_luid, "18446744073709551615");
        let AvailabilityV1::Partial {
            value: processes,
            reason_codes,
            ..
        } = &cli_snapshot.data.processes
        else {
            panic!("process fixture must be partial");
        };
        assert_eq!(processes.enumeration_ceiling, 4096);
        assert_eq!(processes.detail_budget_ms, 150);
        assert_eq!(processes.enumerated_count, 4096);
        assert_eq!(processes.returned_count, 1);
        assert_eq!(processes.requested_limit, 15);
        assert!(processes.truncated_by_limit);
        assert!(processes.budget_exhausted);
        assert_eq!(reason_codes, &["process_limit", "detail_budget_exhausted"]);
        assert_eq!(
            cli_snapshot
                .data
                .unsupported_capabilities
                .iter()
                .map(|capability| capability.code)
                .collect::<Vec<_>>(),
            [
                UnsupportedCode::GpuUtilization,
                UnsupportedCode::Vram,
                UnsupportedCode::Thermal,
                UnsupportedCode::Fan,
                UnsupportedCode::Smart,
                UnsupportedCode::PhysicalDiskActivity,
            ]
        );

        let cli_stream_source = include_str!("fixtures/cli/status-live.ndjson");
        let tauri_stream_source = include_str!("fixtures/cli/status-live.tauri.ndjson");
        assert_eq!(cli_stream_source, tauri_stream_source);
        let cli_stream = decode_stream(cli_stream_source).expect("CLI NDJSON satisfies closed V1");
        let tauri_stream =
            decode_stream(tauri_stream_source).expect("Tauri NDJSON satisfies closed V1");
        assert_eq!(cli_stream, tauri_stream);
        assert_eq!(cli_stream.len(), 4);
        let StatusEventV1::Started {
            interval_ms,
            process_limit,
            ..
        } = &cli_stream[0]
        else {
            panic!("canonical stream must begin with status_started");
        };
        assert_eq!(
            *interval_ms,
            2 * 1_000,
            "CLI seconds convert exactly to wire ms"
        );
        assert_eq!(*process_limit, 15);
        let StatusEventV1::Snapshot { data, .. } = &cli_stream[1] else {
            panic!("canonical stream second event must be status_snapshot");
        };
        assert_eq!(data.unsupported_capabilities.len(), 6);
        assert!(matches!(
            cli_stream.last(),
            Some(StatusEventV1::Terminal {
                reason: TerminalReason::Completed,
                error_code: None,
                ..
            })
        ));

        verify_availability_fixture();
        verify_terminal_reason_fixture();
    }

    fn verify_availability_fixture() {
        let mut variants = ClosedObject::decode(
            serde_json::from_str(include_str!(
                "fixtures/cli/status-availability-variants.json"
            ))
            .expect("availability fixture JSON"),
            "availability variants",
        )
        .unwrap();
        fn decode_fixture_value(value: Value) -> DecodeResult<u32> {
            let mut object = ClosedObject::decode(value, "fixture value")?;
            let fixture = object.take_u32("fixture")?;
            object.finish()?;
            Ok(fixture)
        }
        assert!(matches!(
            decode_availability(
                variants.take("available").unwrap(),
                "fixture",
                decode_fixture_value
            )
            .unwrap(),
            AvailabilityV1::Available { .. }
        ));
        assert!(matches!(
            decode_availability(
                variants.take("partial").unwrap(),
                "fixture",
                decode_fixture_value
            )
            .unwrap(),
            AvailabilityV1::Partial { .. }
        ));
        assert!(matches!(
            decode_availability(
                variants.take("unavailable").unwrap(),
                "fixture",
                decode_fixture_value
            )
            .unwrap(),
            AvailabilityV1::Unavailable { .. }
        ));
        assert!(matches!(
            decode_availability(
                variants.take("permission_denied").unwrap(),
                "fixture",
                decode_fixture_value,
            )
            .unwrap(),
            AvailabilityV1::PermissionDenied { .. }
        ));
        assert!(matches!(
            decode_availability(
                variants.take("unsupported").unwrap(),
                "fixture",
                decode_fixture_value
            )
            .unwrap(),
            AvailabilityV1::Unsupported { .. }
        ));
        variants.finish().unwrap();
    }

    fn verify_terminal_reason_fixture() {
        let values: Vec<Value> =
            serde_json::from_str(include_str!("fixtures/cli/status-terminal-reasons.json"))
                .expect("terminal reason fixture JSON");
        let reasons = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let mut object = ClosedObject::decode(value, format!("terminal reason {index}"))?;
                let reason = decode_terminal_reason(&object.take_string("reason")?)?;
                let error_code = match object.take("error_code")? {
                    Value::Null => None,
                    Value::String(code) => {
                        require_identifier(&code, "terminal reason error_code")?;
                        Some(code)
                    }
                    _ => {
                        return Err("terminal reason error_code must be string or null".to_string());
                    }
                };
                if (reason == TerminalReason::ProducerError) != error_code.is_some() {
                    return Err("terminal reason/error_code pairing drifted".to_string());
                }
                object.finish()?;
                Ok(reason)
            })
            .collect::<DecodeResult<Vec<_>>>()
            .unwrap();
        assert_eq!(
            reasons,
            [
                TerminalReason::Completed,
                TerminalReason::Canceled,
                TerminalReason::BrokenPipe,
                TerminalReason::ProducerError,
            ]
        );
    }

    pub(super) fn verify_closed_schema_rejections() {
        let source = include_str!("fixtures/cli/status-snapshot.json");
        let canonical: Value = serde_json::from_str(source).unwrap();

        let mut extra = canonical.clone();
        extra.as_object_mut().unwrap().insert(
            "localized_message".to_string(),
            Value::String("no".to_string()),
        );
        assert!(SnapshotEnvelopeV1::decode(extra).is_err());

        let mut missing = canonical.clone();
        missing["data"].as_object_mut().unwrap().remove("cpu");
        assert!(SnapshotEnvelopeV1::decode(missing).is_err());

        let mut floating = canonical.clone();
        floating["data"]["sample_window_ms"] = serde_json::json!(1.5);
        assert!(SnapshotEnvelopeV1::decode(floating).is_err());

        let mut widened_cpu = canonical.clone();
        widened_cpu["data"]["cpu"]["value"]["system_utilization_basis_points"] =
            serde_json::json!(10001);
        assert!(SnapshotEnvelopeV1::decode(widened_cpu).is_err());

        let mut constants = canonical.clone();
        constants["data"]["processes"]["value"]["enumeration_ceiling"] = serde_json::json!(4097);
        assert!(SnapshotEnvelopeV1::decode(constants).is_err());

        let mut inconsistent_outcome = canonical.clone();
        inconsistent_outcome["outcome"] = Value::String("success".to_string());
        assert!(SnapshotEnvelopeV1::decode(inconsistent_outcome).is_err());

        let mut available_truncation = canonical.clone();
        available_truncation["data"]["processes"]["state"] = Value::String("available".to_string());
        available_truncation["data"]["processes"]
            .as_object_mut()
            .unwrap()
            .remove("reason_codes");
        assert!(SnapshotEnvelopeV1::decode(available_truncation).is_err());

        let mut missing_truncation_reason = canonical.clone();
        missing_truncation_reason["data"]["processes"]["reason_codes"] =
            serde_json::json!(["detail_budget_exhausted"]);
        assert!(SnapshotEnvelopeV1::decode(missing_truncation_reason).is_err());

        let mut nested_extra = canonical.clone();
        nested_extra["data"]["network"]["value"]
            .as_object_mut()
            .unwrap()
            .insert(
                "localized_unit".to_string(),
                Value::String("字节".to_string()),
            );
        assert!(SnapshotEnvelopeV1::decode(nested_extra).is_err());

        let mut luid = canonical;
        luid["data"]["network"]["value"]["interfaces"][0]["interface_luid"] =
            Value::String("1.5".to_string());
        assert!(SnapshotEnvelopeV1::decode(luid).is_err());

        let stream = include_str!("fixtures/cli/status-live.ndjson");
        let canonical_lines = stream.lines().map(str::to_string).collect::<Vec<_>>();
        for valid_interval_ms in [1_000, 60_000] {
            let mut lines = canonical_lines.clone();
            let mut started: Value = serde_json::from_str(&lines[0]).unwrap();
            started["data"]["interval_ms"] = serde_json::json!(valid_interval_ms);
            lines[0] = serde_json::to_string(&started).unwrap();
            decode_stream(&lines.join("\n")).expect("whole-second interval within CLI range");
        }
        for invalid_interval_ms in [2, 999, 1_500, 60_001] {
            let mut lines = canonical_lines.clone();
            let mut started: Value = serde_json::from_str(&lines[0]).unwrap();
            started["data"]["interval_ms"] = serde_json::json!(invalid_interval_ms);
            lines[0] = serde_json::to_string(&started).unwrap();
            assert!(
                decode_stream(&lines.join("\n")).is_err(),
                "wire interval {invalid_interval_ms} must not be accepted"
            );
        }

        let mut lines = canonical_lines.clone();
        let mut second: Value = serde_json::from_str(&lines[1]).unwrap();
        second["sequence"] = serde_json::json!(0);
        lines[1] = serde_json::to_string(&second).unwrap();
        assert!(decode_stream(&lines.join("\n")).is_err());

        let mut lines = canonical_lines.clone();
        let mut second: Value = serde_json::from_str(&lines[1]).unwrap();
        second["data"]["unsupported_capabilities"]
            .as_array_mut()
            .unwrap()
            .pop();
        lines[1] = serde_json::to_string(&second).unwrap();
        assert!(decode_stream(&lines.join("\n")).is_err());

        let mut lines = canonical_lines.clone();
        let mut second: Value = serde_json::from_str(&lines[1]).unwrap();
        let duplicate = second["data"]["unsupported_capabilities"][0].clone();
        second["data"]["unsupported_capabilities"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        lines[1] = serde_json::to_string(&second).unwrap();
        assert!(decode_stream(&lines.join("\n")).is_err());

        let mut lines = canonical_lines;
        let mut second: Value = serde_json::from_str(&lines[1]).unwrap();
        second["data"]["unsupported_capabilities"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "code": "localized_extra",
                "state": "unsupported",
                "reason_code": "not_supported_v1"
            }));
        lines[1] = serde_json::to_string(&second).unwrap();
        assert!(decode_stream(&lines.join("\n")).is_err());
    }
}
