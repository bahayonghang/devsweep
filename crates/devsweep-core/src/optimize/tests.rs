//! Exhaustive, hostile, fake, and native Optimize V1 tests, organized as the
//! `catalogue`, `authorizer`, `windows`, `execution`, and `audit` suites.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tempfile::TempDir;

use super::audit::{
    OptimizeAuditError, OptimizeAuditErrorCode, OptimizeAuditEvent, OptimizeAuditJournal,
    OptimizeAuditRecordV1, OptimizeAuditStatusCode, OptimizeAuditTransition,
    optimize_audit_v1_path_from,
};
use super::catalogue::{
    GUIDANCE_DRIVE_OPTIMIZE_ID, GUIDANCE_FILESYSTEM_CHECK_ID, GUIDANCE_NETWORK_RESET_ID,
    GUIDANCE_SYSTEM_INTEGRITY_ID, MaintenanceActionClass, MaintenanceCatalogueEntryV1,
    SETTINGS_ENERGY_RECOMMENDATIONS_ID, SETTINGS_SEARCH_ID, SETTINGS_STORAGE_RECOMMENDATIONS_ID,
    catalogue_entries, catalogue_entry, settings_handoff,
};
use super::execution::{
    AdapterEvidence, AdapterOutcome, DispatchTiming, MaintenanceDispatch,
    MaintenanceExecutionError, MaintenanceExecutionOutcome, MaintenanceExecutionReportV1,
    MaintenanceExecutionRequest, MaintenanceExecutor, classify_terminal,
};
use super::plan::{
    OptimizePlanError, ResolvedMaintenanceAction, ValidatedMaintenanceAction,
    construct_validated_action, preview_maintenance_plan, validate_preview_digest,
};
use super::windows::{
    adapter_evidence_from_status, dns_flush_request, resolve_action, verify_ipconfig_identity,
};
use super::{
    DNS_FLUSH_ID, LivePreflight, MaintenancePlanV1, MaintenancePreviewV1,
    OPTIMIZE_CATALOGUE_VERSION, OPTIMIZE_PLAN_VERSION, OPTIMIZE_PREVIEW_VERSION, plan_operation,
    preview_maintenance_plan_live,
};
use crate::process::{FlagCancelObserver, NoopCancelObserver};

fn plan(operation_id: &str) -> MaintenancePlanV1 {
    plan_operation(operation_id).unwrap_or(MaintenancePlanV1 {
        version: OPTIMIZE_PLAN_VERSION,
        catalogue_version: OPTIMIZE_CATALOGUE_VERSION,
        operation_id: operation_id.to_string(),
    })
}

const DISPATCHABLE_IDS: [&str; 4] = [
    DNS_FLUSH_ID,
    SETTINGS_STORAGE_RECOMMENDATIONS_ID,
    SETTINGS_SEARCH_ID,
    SETTINGS_ENERGY_RECOMMENDATIONS_ID,
];

const GUIDANCE_IDS: [&str; 4] = [
    GUIDANCE_DRIVE_OPTIMIZE_ID,
    GUIDANCE_SYSTEM_INTEGRITY_ID,
    GUIDANCE_FILESYSTEM_CHECK_ID,
    GUIDANCE_NETWORK_RESET_ID,
];

/// One configurable live preflight fake.
#[derive(Debug, Clone)]
struct FakePreflight {
    result: Result<ResolvedMaintenanceAction, OptimizePlanError>,
}

impl FakePreflight {
    fn dns(program: &str) -> Self {
        Self {
            result: Ok(ResolvedMaintenanceAction::DnsFlush {
                program: PathBuf::from(program),
            }),
        }
    }

    fn settings(uri: &'static str, build: u32) -> Self {
        Self {
            result: Ok(ResolvedMaintenanceAction::SettingsHandoff {
                uri,
                observed_build: build,
            }),
        }
    }

    fn failing(error: OptimizePlanError) -> Self {
        Self { result: Err(error) }
    }
}

impl LivePreflight for FakePreflight {
    fn resolve(
        &self,
        _entry: &MaintenanceCatalogueEntryV1,
    ) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
        self.result.clone()
    }
}

/// One configurable dispatch fake that counts invocations.
struct FakeDispatch {
    evidence: Mutex<AdapterEvidence>,
    calls: AtomicUsize,
    sleep: Option<std::time::Duration>,
    cancel_flag: Option<Arc<FlagCancelObserver>>,
}

impl FakeDispatch {
    fn success() -> Self {
        Self::fixed(AdapterEvidence::success())
    }

    fn fixed(evidence: AdapterEvidence) -> Self {
        Self {
            evidence: Mutex::new(evidence),
            calls: AtomicUsize::new(0),
            sleep: None,
            cancel_flag: None,
        }
    }

    fn canceling_after_dispatch(flag: Arc<FlagCancelObserver>) -> Self {
        Self {
            evidence: Mutex::new(AdapterEvidence::unfinished(
                OptimizeAuditErrorCode::AdapterCanceledAfterDispatch,
            )),
            calls: AtomicUsize::new(0),
            sleep: None,
            cancel_flag: Some(flag),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl MaintenanceDispatch for FakeDispatch {
    fn dispatch(
        &self,
        _action: &ValidatedMaintenanceAction,
        _cancel: Option<&Arc<FlagCancelObserver>>,
        _timing: DispatchTiming,
    ) -> AdapterEvidence {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(sleep) = self.sleep {
            std::thread::sleep(sleep);
        }
        // Simulates the operator observing a cancel request mid-run: the
        // dispatch already started, so nothing can be proven afterwards.
        if let Some(flag) = &self.cancel_flag {
            flag.request_cancel();
        }
        *self.evidence.lock().expect("fake dispatch evidence mutex")
    }
}

fn executor(
    directory: &TempDir,
    preflight: FakePreflight,
    dispatch: Arc<FakeDispatch>,
) -> (MaintenanceExecutor, PathBuf) {
    let audit_path = directory.path().join("optimize.jsonl");
    let executor = MaintenanceExecutor::for_test(
        audit_path.clone(),
        dispatch,
        Arc::new(preflight),
        DispatchTiming {
            timeout: std::time::Duration::from_millis(200),
            ..DispatchTiming::default()
        },
    );
    (executor, audit_path)
}

mod catalogue {
    use super::*;

    #[test]
    fn exactly_eight_rows_in_frozen_order() {
        let entries = catalogue_entries();
        assert_eq!(entries.len(), 8);
        let snapshot: Vec<(&str, MaintenanceActionClass, Option<u32>)> = entries
            .iter()
            .map(|entry| (entry.id, entry.action_class, entry.build_floor))
            .collect();
        assert_eq!(
            snapshot,
            vec![
                ("dns.flush", MaintenanceActionClass::Execute, None),
                (
                    "settings.storage_recommendations",
                    MaintenanceActionClass::SettingsHandoff,
                    Some(22_000)
                ),
                (
                    "settings.search",
                    MaintenanceActionClass::SettingsHandoff,
                    Some(22_000)
                ),
                (
                    "settings.energy_recommendations",
                    MaintenanceActionClass::SettingsHandoff,
                    Some(22_624)
                ),
                (
                    "guidance.drive_optimize",
                    MaintenanceActionClass::Guidance,
                    None
                ),
                (
                    "guidance.system_integrity",
                    MaintenanceActionClass::Guidance,
                    None
                ),
                (
                    "guidance.filesystem_check",
                    MaintenanceActionClass::Guidance,
                    None
                ),
                (
                    "guidance.network_reset",
                    MaintenanceActionClass::Guidance,
                    None
                ),
            ]
        );
        assert_eq!(super::OPTIMIZE_CATALOGUE_VERSION, 1);
    }

    #[test]
    fn settings_handoffs_are_the_exact_frozen_literals() {
        assert_eq!(
            settings_handoff("settings.storage_recommendations"),
            Some(("ms-settings:storagerecommendations", 22_000))
        );
        assert_eq!(
            settings_handoff("settings.search"),
            Some(("ms-settings:search", 22_000))
        );
        assert_eq!(
            settings_handoff("settings.energy_recommendations"),
            Some(("ms-settings:energyrecommendations", 22_624))
        );
        for id in ["dns.flush"]
            .iter()
            .chain(GUIDANCE_IDS.iter())
            .chain(["unknown", "ms-settings:search", ""].iter())
        {
            assert_eq!(settings_handoff(id), None, "{id} must not hand off");
        }
    }

    #[test]
    fn unknown_and_rejected_categories_cannot_resolve() {
        let rejected = [
            "security.defender",
            "security.uac",
            "firewall.reset",
            "windows.update.reset",
            "registry.tweak",
            "service.restart",
            "performance.visual_effects",
            "pagefile.change",
            "power.plan",
            "hibernation.disable",
            "storage_sense.configure",
            "cache.rebuild",
            "explorer.cache.rebuild",
            "script.run",
            "cmd.run",
            "powershell.run",
            "ipconfig",
            "ipconfig.exe",
            "/flushdns",
            "DNS.FLUSH",
            "dns.flush ",
            " dns.flush",
            "dns.flush\n",
            "dns/flush",
            "../dns.flush",
            "dns.flush;cmd",
            "dns.flush$(cmd)",
            "%PATH%",
            "ms-settings:storagerecommendations",
            "guidance.*",
            "",
            "\u{0064}ns.flush\u{200b}",
        ];
        for id in rejected {
            assert!(
                catalogue_entry(id).is_none(),
                "hostile id {id:?} must not resolve"
            );
            assert_eq!(
                plan_operation(id),
                Err(OptimizePlanError::UnknownOperation(id.to_string())),
                "hostile id {id:?} must not plan"
            );
        }
    }

    #[test]
    fn guidance_rows_never_plan() {
        for id in GUIDANCE_IDS {
            assert_eq!(
                plan_operation(id),
                Err(OptimizePlanError::GuidanceNotExecutable(id.to_string()))
            );
        }
    }

    #[test]
    fn plans_carry_only_version_catalogue_and_one_id() {
        for id in DISPATCHABLE_IDS {
            let maintenance_plan = plan_operation(id).expect("dispatchable id plans");
            assert_eq!(maintenance_plan.version, OPTIMIZE_PLAN_VERSION);
            assert_eq!(
                maintenance_plan.catalogue_version,
                OPTIMIZE_CATALOGUE_VERSION
            );
            assert_eq!(maintenance_plan.operation_id, id);
            let json = serde_json::to_value(&maintenance_plan).unwrap();
            let keys = json
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(keys, vec!["catalogue_version", "operation_id", "version"]);
            assert!(
                !serde_json::to_string(&maintenance_plan)
                    .unwrap()
                    .contains("program")
            );
        }
    }

    #[test]
    fn hostile_plan_fields_fail_closed_at_decode() {
        let hostile = "{\"version\":1,\"catalogue_version\":1,\"operation_id\":\"dns.flush\",\
             \"program\":\"cmd.exe\",\"argv\":[\"/c\"],\"uri\":\"ms-settings:evil\",\
             \"UninstallString\":\"calc\"}";
        let decoded: Result<MaintenancePlanV1, _> = serde_json::from_str(hostile);
        assert!(
            decoded.is_err(),
            "unknown executable fields must fail closed"
        );
        let wrong_id: MaintenancePlanV1 = serde_json::from_str(
            "{\"version\":1,\"catalogue_version\":1,\"operation_id\":\"cmd.exe /c calc\"}",
        )
        .unwrap();
        assert!(matches!(
            preview_maintenance_plan_live(&wrong_id),
            Err(OptimizePlanError::UnknownOperation(_))
        ));
    }

    #[test]
    fn action_class_serialization_is_closed() {
        assert_eq!(
            serde_json::to_value(MaintenanceActionClass::Execute).unwrap(),
            "execute"
        );
        assert_eq!(
            serde_json::to_value(MaintenanceActionClass::SettingsHandoff).unwrap(),
            "settings_handoff"
        );
        assert_eq!(
            serde_json::to_value(MaintenanceActionClass::Guidance).unwrap(),
            "guidance"
        );
        assert!(
            serde_json::from_value::<MaintenanceActionClass>(serde_json::json!("command")).is_err()
        );
    }
}

mod authorizer {
    use super::*;

    #[test]
    fn dry_run_and_execute_resolve_byte_equivalent_identities() {
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let first = preview_maintenance_plan(&operation, &preflight).unwrap();
        let second = preview_maintenance_plan(&operation, &preflight).unwrap();
        assert_eq!(first, second);
        let first_action = construct_validated_action(&operation, &preflight).unwrap();
        let second_action = construct_validated_action(&operation, &preflight).unwrap();
        assert_eq!(first_action, second_action);
        assert_eq!(first_action.preview_digest, first.digest);
        assert_eq!(first.version, OPTIMIZE_PREVIEW_VERSION);
    }

    #[test]
    fn digest_binds_catalogue_version_id_class_and_identity() {
        let dns = plan(DNS_FLUSH_ID);
        let program_a = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let program_b = FakePreflight::dns("C:/Windows/System32/evil.exe");
        let settings_one = plan(SETTINGS_SEARCH_ID);
        let settings_two = plan(SETTINGS_STORAGE_RECOMMENDATIONS_ID);
        let handoff = FakePreflight::settings("ms-settings:search", 22_000);

        let base = preview_maintenance_plan(&dns, &program_a).unwrap();
        assert_ne!(
            base.digest,
            preview_maintenance_plan(&dns, &program_b).unwrap().digest,
            "a different resolved program must change the digest"
        );
        assert_ne!(
            preview_maintenance_plan(&settings_one, &handoff)
                .unwrap()
                .digest,
            preview_maintenance_plan(&settings_two, &handoff)
                .unwrap()
                .digest,
            "a different id must change the digest"
        );
        assert_ne!(
            preview_maintenance_plan(&settings_one, &handoff)
                .unwrap()
                .digest,
            preview_maintenance_plan(
                &settings_one,
                &FakePreflight::settings("ms-settings:search", 22_621)
            )
            .unwrap()
            .digest,
            "a different observed build capability must change the digest"
        );

        let settings = preview_maintenance_plan(&settings_one, &handoff).unwrap();
        assert_ne!(settings.digest, base.digest, "class change changes digest");
        assert_eq!(
            settings.action_class,
            MaintenanceActionClass::SettingsHandoff
        );
        assert_eq!(base.action_class, MaintenanceActionClass::Execute);
    }

    #[test]
    fn digest_validation_rejects_mismatch_and_foreign_digest() {
        let operation = plan(DNS_FLUSH_ID);
        let preview = preview_maintenance_plan(
            &operation,
            &FakePreflight::dns("C:/Windows/System32/ipconfig.exe"),
        )
        .unwrap();
        validate_preview_digest(&preview, &preview.digest).unwrap();
        assert_eq!(
            validate_preview_digest(
                &preview,
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            ),
            Err(OptimizePlanError::DigestMismatch)
        );
        let forged = MaintenancePreviewV1 {
            digest: "not-a-digest".to_string(),
            ..preview.clone()
        };
        assert_eq!(
            validate_preview_digest(&forged, "not-a-digest"),
            Err(OptimizePlanError::DigestMismatch)
        );
    }

    #[test]
    fn guidance_cannot_resolve_any_dispatch_identity() {
        for id in GUIDANCE_IDS {
            let guidance_plan = plan(id);
            assert_eq!(
                preview_maintenance_plan(
                    &guidance_plan,
                    &FakePreflight::dns("C:/Windows/System32/ipconfig.exe")
                ),
                Err(OptimizePlanError::GuidanceNotExecutable(id.to_string())),
                "guidance must not resolve even against a willing preflight"
            );
            assert!(matches!(
                resolve_action(catalogue_entry(id).unwrap()),
                Err(OptimizePlanError::GuidanceNotExecutable(_))
            ));
        }
    }

    #[test]
    fn changed_preflight_produces_a_different_digest_and_fails_the_old_one() {
        let operation = plan(DNS_FLUSH_ID);
        let before = preview_maintenance_plan(
            &operation,
            &FakePreflight::dns("C:/Windows/System32/ipconfig.exe"),
        )
        .unwrap();
        let after = preview_maintenance_plan(
            &operation,
            &FakePreflight::dns("C:/Windows/Sysnative/ipconfig.exe"),
        )
        .unwrap();
        assert_ne!(before.digest, after.digest);
        assert_eq!(
            validate_preview_digest(&after, &before.digest),
            Err(OptimizePlanError::DigestMismatch)
        );
    }

    #[test]
    fn unsupported_preflight_states_are_typed_failures() {
        let operation = plan(DNS_FLUSH_ID);
        let cases = [
            OptimizePlanError::PlatformUnsupported,
            OptimizePlanError::OsBuildUnavailable,
            OptimizePlanError::BuildUnsupported {
                build: 19_000,
                floor: 22_000,
            },
            OptimizePlanError::ResolverUnavailable,
        ];
        for error in cases {
            assert_eq!(
                preview_maintenance_plan(&operation, &FakePreflight::failing(error.clone())),
                Err(error)
            );
        }
    }

    #[test]
    fn versioned_headers_fail_closed() {
        let mut wrong_version = plan(DNS_FLUSH_ID);
        wrong_version.version = 2;
        assert_eq!(
            preview_maintenance_plan(&wrong_version, &FakePreflight::dns("C:/x/ipconfig.exe")),
            Err(OptimizePlanError::UnsupportedPlanVersion(2))
        );
        let mut wrong_catalogue = plan(DNS_FLUSH_ID);
        wrong_catalogue.catalogue_version = 99;
        assert_eq!(
            preview_maintenance_plan(&wrong_catalogue, &FakePreflight::dns("C:/x/ipconfig.exe")),
            Err(OptimizePlanError::UnsupportedCatalogueVersion(99))
        );
        let unknown = MaintenancePlanV1 {
            version: OPTIMIZE_PLAN_VERSION,
            catalogue_version: OPTIMIZE_CATALOGUE_VERSION,
            operation_id: "no.such.id".to_string(),
        };
        assert_eq!(
            preview_maintenance_plan(&unknown, &FakePreflight::dns("C:/x/ipconfig.exe")),
            Err(OptimizePlanError::UnknownOperation(
                "no.such.id".to_string()
            ))
        );
    }

    #[test]
    fn preview_documents_never_leak_resolved_identities() {
        let operation = plan(DNS_FLUSH_ID);
        let preview = preview_maintenance_plan(
            &operation,
            &FakePreflight::dns("C:/Windows/System32/ipconfig.exe"),
        )
        .unwrap();
        let json = serde_json::to_string(&preview).unwrap();
        for forbidden in [
            "program",
            "argv",
            "ipconfig",
            "Sysnative",
            "System32",
            "ms-settings",
            "flushdns",
        ] {
            assert!(!json.contains(forbidden), "preview leaked {forbidden}");
        }
        let action = construct_validated_action(
            &operation,
            &FakePreflight::dns("C:/Windows/System32/ipconfig.exe"),
        )
        .unwrap();
        assert_eq!(action.operation_id, DNS_FLUSH_ID);
    }
}

mod windows {
    use super::*;

    #[test]
    fn dns_flush_request_is_program_plus_exact_fixed_argv() {
        let cancel = NoopCancelObserver;
        let timing = DispatchTiming::default();
        let request = dns_flush_request(
            Path::new("C:/Windows/System32/ipconfig.exe"),
            &cancel,
            timing,
        );
        assert_eq!(
            request.program,
            std::ffi::OsString::from("C:/Windows/System32/ipconfig.exe")
        );
        assert_eq!(request.args, vec![std::ffi::OsString::from("/flushdns")]);
        assert_eq!(request.timeout, Some(timing.timeout));
        assert_eq!(timing.timeout, std::time::Duration::from_secs(10));
        assert!(request.job_deadline.is_none());
        assert!(matches!(request.cwd, crate::process::CwdPolicy::Neutral));
    }

    #[test]
    fn process_status_mapping_is_closed() {
        use crate::process::ProcessStatus;
        let cases = [
            (ProcessStatus::Success, AdapterOutcome::Success),
            (
                ProcessStatus::Exit { code: Some(1) },
                AdapterOutcome::Failure,
            ),
            (
                ProcessStatus::Exit { code: None },
                AdapterOutcome::Unfinished,
            ),
            (ProcessStatus::NotFound, AdapterOutcome::Failure),
            (ProcessStatus::InvalidOutput, AdapterOutcome::Failure),
            (ProcessStatus::Timeout, AdapterOutcome::Unfinished),
            (ProcessStatus::Canceled, AdapterOutcome::Unfinished),
        ];
        for (status, outcome) in cases {
            assert_eq!(adapter_evidence_from_status(status).outcome, outcome);
        }
    }

    #[test]
    fn settings_resolution_requires_a_real_build_answer() {
        let entry = catalogue_entry(SETTINGS_STORAGE_RECOMMENDATIONS_ID).unwrap();
        match resolve_action(entry) {
            Ok(ResolvedMaintenanceAction::SettingsHandoff {
                uri,
                observed_build,
            }) => {
                assert_eq!(uri, "ms-settings:storagerecommendations");
                assert_eq!(observed_build, super::super::windows::os_build().unwrap());
            }
            Err(OptimizePlanError::OsBuildUnavailable) => {
                // Non-Windows host: the build query is unavailable by design.
                assert!(super::super::windows::os_build().is_none());
            }
            other => panic!("unexpected settings resolution: {other:?}"),
        }
    }

    #[test]
    fn execute_resolution_is_windows_only() {
        let entry = catalogue_entry(DNS_FLUSH_ID).unwrap();
        match resolve_action(entry) {
            Ok(ResolvedMaintenanceAction::DnsFlush { program }) => {
                assert!(program.is_absolute());
                assert!(
                    program
                        .to_string_lossy()
                        .eq_ignore_ascii_case("C:/Windows/System32/ipconfig.exe")
                        || program
                            .file_name()
                            .unwrap()
                            .eq_ignore_ascii_case("ipconfig.exe")
                );
            }
            Err(OptimizePlanError::PlatformUnsupported) => {}
            other => panic!("unexpected execute resolution: {other:?}"),
        }
    }

    #[test]
    #[cfg(windows)]
    fn build_query_is_manifest_independent_and_monotonic() {
        let build = super::super::windows::os_build().expect("RtlGetVersion build");
        assert!(build >= 1_000, "implausible build {build}");
        let again = super::super::windows::os_build().expect("second build query");
        assert_eq!(build, again);
    }

    #[test]
    #[cfg(windows)]
    fn resolved_ipconfig_identity_is_native_system32_not_syswow64() {
        let program = super::super::windows::resolve_ipconfig().expect("native resolver");
        let text = program.to_string_lossy().to_ascii_uppercase();
        assert!(!text.contains("SYSWOW64"), "resolved {program:?}");
        assert!(
            program
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("ipconfig.exe"))
        );
        let parent = program.parent().expect("parent directory");
        let parent_name = parent
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        assert!(
            parent_name.eq_ignore_ascii_case("system32")
                || parent_name.eq_ignore_ascii_case("sysnative"),
            "parent must be System32 (native) or Sysnative (WOW64), got {parent:?}"
        );
        let grandparent = parent.parent().expect("windows directory");
        assert!(
            grandparent
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("windows")),
            "grandparent must be Windows, got {grandparent:?}"
        );
    }

    #[test]
    #[cfg(windows)]
    fn identity_checks_reject_hostile_paths() {
        let temp = TempDir::new().unwrap();
        let good = temp.path().join("ipconfig.exe");
        std::fs::write(&good, b"MZ").unwrap();
        verify_ipconfig_identity(&good).expect("exact existing file");

        let wrong_name = temp.path().join("evil.exe");
        std::fs::write(&wrong_name, b"MZ").unwrap();
        assert!(verify_ipconfig_identity(&wrong_name).is_err());

        let double_extension = temp.path().join("ipconfig.exe.txt");
        std::fs::write(&double_extension, b"MZ").unwrap();
        assert!(verify_ipconfig_identity(&double_extension).is_err());

        let directory = temp.path().join("ipconfig.exe.dir");
        std::fs::create_dir_all(&directory).unwrap();
        assert!(verify_ipconfig_identity(&directory).is_err());

        assert!(verify_ipconfig_identity(Path::new("ipconfig.exe")).is_err());
        assert!(verify_ipconfig_identity(Path::new("C:/Windows/../ipconfig.exe")).is_err());
        assert!(verify_ipconfig_identity(Path::new("C:/missing/ipconfig.exe")).is_err());
        assert!(
            verify_ipconfig_identity(Path::new(r"\\?\C:\Windows\System32\ipconfig.exe")).is_err()
        );
    }

    #[test]
    fn dispatch_runs_the_exact_request_without_a_shell() {
        let timing = DispatchTiming {
            timeout: std::time::Duration::from_millis(500),
            ..DispatchTiming::default()
        };
        // The fixture receives the fixed single argv element and reports an
        // unknown mode, which proves exactly one argument was passed.
        let fixture = crate::process::test_support::process_fixture_exe();
        let evidence = super::super::windows::dispatch_dns_flush(&fixture, None, timing);
        assert_eq!(evidence.outcome, AdapterOutcome::Failure);
        assert_eq!(
            evidence.error_code,
            Some(OptimizeAuditErrorCode::AdapterOperationFailed)
        );

        let missing = TempDir::new().unwrap().path().join("missing/ipconfig.exe");
        let evidence = super::super::windows::dispatch_dns_flush(&missing, None, timing);
        assert_eq!(evidence.outcome, AdapterOutcome::Failure);
        assert_eq!(
            evidence.error_code,
            Some(OptimizeAuditErrorCode::AdapterDispatchFailed)
        );
    }

    #[test]
    #[cfg(windows)]
    fn native_ipconfig_flushdns_dispatch_succeeds() {
        let program = super::super::windows::resolve_ipconfig().expect("resolver");
        let evidence =
            super::super::windows::dispatch_dns_flush(&program, None, DispatchTiming::default());
        assert_eq!(
            evidence.outcome,
            AdapterOutcome::Success,
            "ipconfig /flushdns"
        );
        assert_eq!(evidence.error_code, None);
    }

    /// Native Settings probe: run explicitly with `--ignored --nocapture`.
    /// Success opens one frozen Settings page (`launched` only). Failure
    /// records `SE_ERR_*` / `GetLastError` / `hProcess` without claiming
    /// launched.
    #[test]
    #[ignore = "native Settings ShellExecuteExW probe"]
    #[cfg(windows)]
    fn native_settings_shell_execute_probe() {
        fn report(label: &str, probe: super::super::windows::SettingsLaunchProbe) {
            eprintln!(
                "{label}: succeeded={} h_inst_app={} (SE_ERR_ACCESSDENIED=5 SE_ERR_NOASSOC=31) last_error={} h_process={} (HWND not returned; SEE_MASK_FLAG_NO_UI only)",
                probe.succeeded, probe.h_inst_app, probe.last_error, probe.h_process
            );
        }

        let allowlisted = super::super::windows::launch_settings_probe("ms-settings:search");
        report("allowlisted ms-settings:search", allowlisted);
        if allowlisted.succeeded {
            assert!(
                allowlisted.h_inst_app > 32,
                "success HINSTANCE must be greater than 32, got {}",
                allowlisted.h_inst_app
            );
            assert_eq!(
                allowlisted.h_process, 0,
                "hProcess must stay null without SEE_MASK_NOCLOSEPROCESS"
            );
        }

        // Adapter-level launch-failure control: not a catalogue id. An
        // unregistered scheme can still return success on this host (the
        // shell reports hInstApp>32). A nonexistent drive is the documented
        // ShellExecuteExW failure that proves non-success without claiming
        // a Settings page launched.
        let unregistered =
            super::super::windows::launch_settings_probe("devsweep-not-a-protocol:test");
        report(
            "unregistered scheme control (not a catalogue id; host may still succeed)",
            unregistered,
        );
        let missing_drive =
            super::super::windows::launch_settings_probe(r"!:\devsweep-optimize-missing-control");
        report(
            "nonexistent-drive control (not a catalogue id)",
            missing_drive,
        );
        if !missing_drive.succeeded {
            assert!(
                missing_drive.h_inst_app <= 32,
                "failed ShellExecuteExW must report SE_ERR_* in hInstApp, got {}",
                missing_drive.h_inst_app
            );
        }
    }
}

mod execution {
    use super::*;

    fn run_confirmed(
        executor: &MaintenanceExecutor,
        operation: &MaintenancePlanV1,
        digest: &str,
        cancel: Option<Arc<FlagCancelObserver>>,
    ) -> Result<MaintenanceExecutionReportV1, MaintenanceExecutionError> {
        executor.execute(MaintenanceExecutionRequest {
            plan: operation,
            expected_preview_digest: digest,
            confirmed: true,
            cancel,
        })
    }

    fn record(
        operation_id: &str,
        catalogue_id: &str,
        action_class: MaintenanceActionClass,
        digest: &str,
        event: OptimizeAuditEvent,
    ) -> OptimizeAuditRecordV1 {
        OptimizeAuditRecordV1::new(operation_id, 1, catalogue_id, action_class, digest, event)
    }

    #[test]
    fn confirmed_dns_success_writes_the_full_audit_trail() {
        let temp = TempDir::new().unwrap();
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, audit_path) = executor(&temp, preflight.clone(), Arc::clone(&dispatch));
        let report = run_confirmed(&executor, &operation, &preview.digest, None).unwrap();
        assert_eq!(report.version, 1);
        assert_eq!(report.catalogue_version, OPTIMIZE_CATALOGUE_VERSION);
        assert_eq!(report.outcomes.len(), 1);
        assert_eq!(
            report.outcomes[0].outcome,
            MaintenanceExecutionOutcome::Succeeded
        );
        assert_eq!(report.outcomes[0].catalogue_id, DNS_FLUSH_ID);
        assert!(report.outcomes[0].outcome.was_dispatched());
        assert_eq!(dispatch.calls(), 1);

        let text = std::fs::read_to_string(&audit_path).unwrap();
        for expected in [
            "validated",
            "dispatch_started",
            "adapter_succeeded",
            "\"succeeded\"",
        ] {
            assert!(text.contains(expected), "missing {expected} in {text}");
        }
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn settings_success_is_only_launched() {
        let temp = TempDir::new().unwrap();
        let operation = plan(SETTINGS_SEARCH_ID);
        let preflight = FakePreflight::settings("ms-settings:search", 22_000);
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, audit_path) = executor(&temp, preflight, Arc::clone(&dispatch));
        let report = run_confirmed(&executor, &operation, &preview.digest, None).unwrap();
        assert_eq!(
            report.outcomes[0].outcome,
            MaintenanceExecutionOutcome::Launched
        );
        assert!(report.succeeded());
        let text = std::fs::read_to_string(&audit_path).unwrap();
        assert!(text.contains("\"launched\""));
        assert!(!text.contains("ms-settings"), "audit must stay redacted");
    }

    #[test]
    fn adapter_failure_and_unfinished_map_to_failed_and_unknown() {
        for (evidence, expected) in [
            (
                AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterOperationFailed),
                MaintenanceExecutionOutcome::Failed,
            ),
            (
                AdapterEvidence::unfinished(OptimizeAuditErrorCode::AdapterTimedOut),
                MaintenanceExecutionOutcome::UnknownAfterDispatch,
            ),
        ] {
            let temp = TempDir::new().unwrap();
            let operation = plan(DNS_FLUSH_ID);
            let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
            let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
            let dispatch = Arc::new(FakeDispatch::fixed(evidence));
            let (executor, audit_path) = executor(&temp, preflight, Arc::clone(&dispatch));
            let report = run_confirmed(&executor, &operation, &preview.digest, None).unwrap();
            assert_eq!(report.outcomes[0].outcome, expected);
            assert!(!report.succeeded());
            let text = std::fs::read_to_string(&audit_path).unwrap();
            let expected_status = match expected {
                MaintenanceExecutionOutcome::Failed => "\"failed\"",
                _ => "\"unknown_after_dispatch\"",
            };
            assert!(text.contains(expected_status), "{text}");
        }
    }

    #[test]
    fn digest_mismatch_fails_before_dispatch_without_touching_the_journal() {
        let temp = TempDir::new().unwrap();
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, audit_path) = executor(&temp, preflight, Arc::clone(&dispatch));
        let wrong = format!("sha256:{}", "f".repeat(64));
        assert!(matches!(
            run_confirmed(&executor, &operation, &wrong, None),
            Err(MaintenanceExecutionError::Plan(
                OptimizePlanError::DigestMismatch
            ))
        ));
        assert_eq!(dispatch.calls(), 0);
        // The journal may be created empty by opening it, but no intent may
        // ever be recorded for a refused operation.
        assert!(std::fs::read(&audit_path).unwrap().is_empty());
    }

    #[test]
    fn confirmation_is_required() {
        let temp = TempDir::new().unwrap();
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, audit_path) = executor(&temp, preflight, Arc::clone(&dispatch));
        let error = executor
            .execute(MaintenanceExecutionRequest {
                plan: &operation,
                expected_preview_digest: &preview.digest,
                confirmed: false,
                cancel: None,
            })
            .unwrap_err();
        assert!(matches!(
            error,
            MaintenanceExecutionError::ConfirmationRequired
        ));
        assert!(!audit_path.exists());
    }

    #[test]
    fn cancel_before_dispatch_is_canceled_without_a_call() {
        let temp = TempDir::new().unwrap();
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, audit_path) = executor(&temp, preflight, Arc::clone(&dispatch));
        let flag = Arc::new(FlagCancelObserver::new());
        flag.request_cancel();
        let report = run_confirmed(&executor, &operation, &preview.digest, Some(flag)).unwrap();
        assert_eq!(
            report.outcomes[0].outcome,
            MaintenanceExecutionOutcome::CanceledBeforeStart
        );
        assert!(!report.outcomes[0].outcome.was_dispatched());
        assert!(!report.succeeded());
        assert_eq!(dispatch.calls(), 0);
        let text = std::fs::read_to_string(&audit_path).unwrap();
        assert!(text.contains("canceled_before_start"));
    }

    #[test]
    fn cancel_after_dispatch_is_unknown() {
        let temp = TempDir::new().unwrap();
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let flag = Arc::new(FlagCancelObserver::new());
        let dispatch = Arc::new(FakeDispatch::canceling_after_dispatch(Arc::clone(&flag)));
        let (executor, audit_path) = executor(&temp, preflight, Arc::clone(&dispatch));
        let report = run_confirmed(&executor, &operation, &preview.digest, Some(flag)).unwrap();
        assert_eq!(
            report.outcomes[0].outcome,
            MaintenanceExecutionOutcome::UnknownAfterDispatch
        );
        let text = std::fs::read_to_string(&audit_path).unwrap();
        assert!(text.contains("adapter_canceled_after_dispatch"));
    }

    #[test]
    fn unsupported_preflight_fails_before_dispatch() {
        let temp = TempDir::new().unwrap();
        let operation = plan(DNS_FLUSH_ID);
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, audit_path) = executor(
            &temp,
            FakePreflight::failing(OptimizePlanError::BuildUnsupported {
                build: 19_000,
                floor: 22_000,
            }),
            Arc::clone(&dispatch),
        );
        let error = run_confirmed(&executor, &operation, "sha256:irrelevant", None).unwrap_err();
        assert!(matches!(
            error,
            MaintenanceExecutionError::Plan(OptimizePlanError::BuildUnsupported { .. })
        ));
        assert_eq!(dispatch.calls(), 0);
        assert!(std::fs::read(&audit_path).unwrap().is_empty());
    }

    #[test]
    fn stale_catalogue_version_fails_before_dispatch() {
        let temp = TempDir::new().unwrap();
        let stale = MaintenancePlanV1 {
            version: OPTIMIZE_PLAN_VERSION,
            catalogue_version: OPTIMIZE_CATALOGUE_VERSION + 1,
            operation_id: DNS_FLUSH_ID.to_string(),
        };
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, audit_path) = executor(
            &temp,
            FakePreflight::dns("C:/Windows/System32/ipconfig.exe"),
            Arc::clone(&dispatch),
        );
        let error = run_confirmed(&executor, &stale, "sha256:irrelevant", None).unwrap_err();
        assert!(matches!(
            error,
            MaintenanceExecutionError::Plan(OptimizePlanError::UnsupportedCatalogueVersion(_))
        ));
        assert_eq!(dispatch.calls(), 0);
        assert!(std::fs::read(&audit_path).unwrap().is_empty());
    }

    #[test]
    fn recovery_never_redispatches_pending_operations() {
        let temp = TempDir::new().unwrap();
        let audit_path = temp.path().join("optimize.jsonl");
        {
            let mut journal = OptimizeAuditJournal::open(&audit_path).unwrap();
            let digest = format!("sha256:{}", "1".repeat(64));
            journal
                .append(record(
                    "op-before-dispatch",
                    DNS_FLUSH_ID,
                    MaintenanceActionClass::Execute,
                    &digest,
                    OptimizeAuditEvent {
                        transition: OptimizeAuditTransition::Validated,
                        status_code: OptimizeAuditStatusCode::Validated,
                        error_code: None,
                        adapter_outcome: None,
                    },
                ))
                .unwrap();
            journal
                .append(record(
                    "op-after-dispatch",
                    SETTINGS_SEARCH_ID,
                    MaintenanceActionClass::SettingsHandoff,
                    &digest,
                    OptimizeAuditEvent {
                        transition: OptimizeAuditTransition::Validated,
                        status_code: OptimizeAuditStatusCode::Validated,
                        error_code: None,
                        adapter_outcome: None,
                    },
                ))
                .unwrap();
            journal
                .append(record(
                    "op-after-dispatch",
                    SETTINGS_SEARCH_ID,
                    MaintenanceActionClass::SettingsHandoff,
                    &digest,
                    OptimizeAuditEvent {
                        transition: OptimizeAuditTransition::DispatchStarted,
                        status_code: OptimizeAuditStatusCode::DispatchStarted,
                        error_code: None,
                        adapter_outcome: None,
                    },
                ))
                .unwrap();
        }
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, _) = executor(
            &temp,
            FakePreflight::dns("C:/Windows/System32/ipconfig.exe"),
            Arc::clone(&dispatch),
        );
        let recovered = executor.recover_startup().unwrap();
        assert_eq!(recovered.len(), 2);
        let before = recovered
            .iter()
            .find(|outcome| outcome.catalogue_id == DNS_FLUSH_ID)
            .expect("validated-only operation is recovered");
        assert_eq!(
            before.outcome,
            MaintenanceExecutionOutcome::CanceledBeforeStart
        );
        assert_eq!(
            before.error_code,
            Some(OptimizeAuditErrorCode::RecoveredAfterCrash)
        );
        let after = recovered
            .iter()
            .find(|outcome| outcome.catalogue_id == SETTINGS_SEARCH_ID)
            .expect("dispatched operation is recovered");
        assert_eq!(
            after.outcome,
            MaintenanceExecutionOutcome::UnknownAfterDispatch
        );
        assert_eq!(dispatch.calls(), 0, "recovery must never redispatch");

        let text = std::fs::read_to_string(&audit_path).unwrap();
        assert!(text.contains("recovered_before_dispatch"));
        // A brand-new operation still runs after recovery reconciled the log.
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let report = run_confirmed(&executor, &operation, &preview.digest, None).unwrap();
        assert_eq!(
            report.outcomes[0].outcome,
            MaintenanceExecutionOutcome::Succeeded
        );
        assert_eq!(dispatch.calls(), 1);
    }

    #[test]
    fn lock_conflict_fails_closed_and_preserves_bytes() {
        let temp = TempDir::new().unwrap();
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let dispatch = Arc::new(FakeDispatch::success());
        let (executor, audit_path) = executor(&temp, preflight, Arc::clone(&dispatch));
        run_confirmed(&executor, &operation, &preview.digest, None).unwrap();
        let original = std::fs::read(&audit_path).unwrap();

        // Holding one journal open must make a second execution on the same
        // store fail closed before any dispatch.
        let held = OptimizeAuditJournal::open(&audit_path).unwrap();
        let error = run_confirmed(&executor, &operation, &preview.digest, None).unwrap_err();
        drop(held);
        assert!(matches!(
            error,
            MaintenanceExecutionError::Audit(OptimizeAuditError::LockUnavailable(_))
        ));
        assert_eq!(dispatch.calls(), 1, "the refused run never dispatched");
        assert_eq!(std::fs::read(&audit_path).unwrap(), original);
    }

    #[test]
    fn one_operation_permit_serializes_concurrent_runs() {
        let temp = TempDir::new().unwrap();
        let audit_path = temp.path().join("optimize.jsonl");
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let executor = MaintenanceExecutor::for_test(
            audit_path.clone(),
            Arc::new(FakeDispatch {
                evidence: Mutex::new(AdapterEvidence::success()),
                calls: AtomicUsize::new(0),
                sleep: Some(std::time::Duration::from_millis(50)),
                cancel_flag: None,
            }),
            Arc::new(preflight),
            DispatchTiming::default(),
        );
        let executor = Arc::new(executor);
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let executor = Arc::clone(&executor);
                let operation = operation.clone();
                let digest = preview.digest.clone();
                std::thread::spawn(move || {
                    executor
                        .execute(MaintenanceExecutionRequest {
                            plan: &operation,
                            expected_preview_digest: &digest,
                            confirmed: true,
                            cancel: None,
                        })
                        .unwrap()
                })
            })
            .collect();
        let mut outcomes = Vec::new();
        for handle in handles {
            outcomes.push(handle.join().unwrap());
        }
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes.iter().all(|report| report.succeeded()));
        // Both operations appended, and the journal still parses exactly.
        let journal = OptimizeAuditJournal::open(&audit_path).unwrap();
        assert!(journal.pending_operations().is_empty());
    }

    #[test]
    fn audit_records_stay_redacted() {
        let temp = TempDir::new().unwrap();
        let operation = plan(DNS_FLUSH_ID);
        let preflight = FakePreflight::dns("C:/Windows/System32/ipconfig.exe");
        let preview = preview_maintenance_plan(&operation, &preflight).unwrap();
        let (executor, audit_path) = executor(&temp, preflight, Arc::new(FakeDispatch::success()));
        run_confirmed(&executor, &operation, &preview.digest, None).unwrap();
        let text = std::fs::read_to_string(&audit_path).unwrap();
        for forbidden in [
            "program",
            "argv",
            "ipconfig",
            "flushdns",
            "Sysnative",
            "System32",
            "ms-settings",
            "environment",
            "shell",
        ] {
            assert!(!text.contains(forbidden), "audit leaked {forbidden}");
        }
    }

    #[test]
    fn terminal_classification_is_exhaustive_and_closed() {
        use MaintenanceActionClass as Class;
        let cases = [
            (
                Class::Execute,
                AdapterOutcome::Success,
                MaintenanceExecutionOutcome::Succeeded,
            ),
            (
                Class::SettingsHandoff,
                AdapterOutcome::Success,
                MaintenanceExecutionOutcome::Launched,
            ),
            (
                Class::Execute,
                AdapterOutcome::Failure,
                MaintenanceExecutionOutcome::Failed,
            ),
            (
                Class::SettingsHandoff,
                AdapterOutcome::Failure,
                MaintenanceExecutionOutcome::Failed,
            ),
            (
                Class::Execute,
                AdapterOutcome::Unfinished,
                MaintenanceExecutionOutcome::UnknownAfterDispatch,
            ),
            (
                Class::SettingsHandoff,
                AdapterOutcome::Unfinished,
                MaintenanceExecutionOutcome::UnknownAfterDispatch,
            ),
        ];
        for (class, adapter, expected) in cases {
            assert_eq!(classify_terminal(class, adapter), expected);
        }
    }
}

mod audit {
    use super::*;

    fn record(
        operation_id: &str,
        catalogue_id: &str,
        action_class: MaintenanceActionClass,
        digest: &str,
        event: OptimizeAuditEvent,
    ) -> OptimizeAuditRecordV1 {
        OptimizeAuditRecordV1::new(operation_id, 1, catalogue_id, action_class, digest, event)
    }

    fn validated() -> OptimizeAuditEvent {
        OptimizeAuditEvent {
            transition: OptimizeAuditTransition::Validated,
            status_code: OptimizeAuditStatusCode::Validated,
            error_code: None,
            adapter_outcome: None,
        }
    }

    fn digest() -> String {
        format!("sha256:{}", "3".repeat(64))
    }

    #[test]
    fn fixed_path_requires_local_app_data() {
        assert!(matches!(
            optimize_audit_v1_path_from(None),
            Err(OptimizeAuditError::LocalAppDataUnavailable)
        ));
        assert_eq!(
            optimize_audit_v1_path_from(Some(std::ffi::OsString::from(
                r"C:\Users\dev\AppData\Local"
            )))
            .unwrap(),
            PathBuf::from(r"C:\Users\dev\AppData\Local")
                .join("DevSweep")
                .join("audit")
                .join("v1")
                .join("optimize.jsonl")
        );
    }

    #[test]
    fn happy_trail_is_monotonic_and_terminal_is_closed() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("optimize.jsonl");
        let digest = digest();
        let mut journal = OptimizeAuditJournal::open(&path).unwrap();
        journal
            .append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                validated(),
            ))
            .unwrap();
        // Adapter before dispatch is invalid.
        assert!(matches!(
            journal.append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::AdapterCompleted,
                    status_code: OptimizeAuditStatusCode::AdapterSucceeded,
                    error_code: None,
                    adapter_outcome: Some(AdapterOutcome::Success),
                },
            )),
            Err(OptimizeAuditError::InvalidTransition { .. })
        ));
        journal
            .append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::DispatchStarted,
                    status_code: OptimizeAuditStatusCode::DispatchStarted,
                    error_code: None,
                    adapter_outcome: None,
                },
            ))
            .unwrap();
        journal
            .append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::AdapterCompleted,
                    status_code: OptimizeAuditStatusCode::AdapterSucceeded,
                    error_code: None,
                    adapter_outcome: Some(AdapterOutcome::Success),
                },
            ))
            .unwrap();
        journal
            .append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::Terminal {
                        outcome: MaintenanceExecutionOutcome::Succeeded,
                    },
                    status_code: OptimizeAuditStatusCode::Succeeded,
                    error_code: None,
                    adapter_outcome: Some(AdapterOutcome::Success),
                },
            ))
            .unwrap();
        // Nothing may follow a terminal.
        assert!(matches!(
            journal.append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::Terminal {
                        outcome: MaintenanceExecutionOutcome::Failed,
                    },
                    status_code: OptimizeAuditStatusCode::Failed,
                    error_code: Some(OptimizeAuditErrorCode::AdapterOperationFailed),
                    adapter_outcome: Some(AdapterOutcome::Failure),
                },
            )),
            Err(OptimizeAuditError::InvalidTransition { .. })
        ));
        assert!(journal.pending_operations().is_empty());
    }

    #[test]
    fn terminal_must_match_accumulated_adapter_evidence() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("optimize.jsonl");
        let digest = digest();
        let mut journal = OptimizeAuditJournal::open(&path).unwrap();
        journal
            .append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                validated(),
            ))
            .unwrap();
        journal
            .append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::DispatchStarted,
                    status_code: OptimizeAuditStatusCode::DispatchStarted,
                    error_code: None,
                    adapter_outcome: None,
                },
            ))
            .unwrap();
        // A success terminal without adapter evidence is invalid.
        assert!(matches!(
            journal.append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::Terminal {
                        outcome: MaintenanceExecutionOutcome::Succeeded,
                    },
                    status_code: OptimizeAuditStatusCode::Succeeded,
                    error_code: None,
                    adapter_outcome: None,
                }
            )),
            Err(OptimizeAuditError::InvalidTransition { .. })
        ));
        // A failure terminal without a failure adapter record is invalid.
        assert!(matches!(
            journal.append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::Terminal {
                        outcome: MaintenanceExecutionOutcome::Failed,
                    },
                    status_code: OptimizeAuditStatusCode::Failed,
                    error_code: Some(OptimizeAuditErrorCode::AdapterOperationFailed),
                    adapter_outcome: Some(AdapterOutcome::Failure),
                }
            )),
            Err(OptimizeAuditError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn catalogue_identity_must_be_closed_and_consistent() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("optimize.jsonl");
        let digest = digest();
        let mut journal = OptimizeAuditJournal::open(&path).unwrap();
        assert!(matches!(
            journal.append(record(
                "op",
                "hostile.id",
                MaintenanceActionClass::Execute,
                &digest,
                validated(),
            )),
            Err(OptimizeAuditError::InvalidTransition { .. })
        ));
        // Guidance rows are closed catalogue ids but can never be audited as
        // an operation class... they cannot even start: only dispatchable
        // classes may exist, so a guidance record fails its own shape.
        assert!(matches!(
            journal.append(record(
                "op",
                GUIDANCE_DRIVE_OPTIMIZE_ID,
                MaintenanceActionClass::Guidance,
                &digest,
                validated(),
            )),
            Err(OptimizeAuditError::InvalidTransition { .. })
        ));
        // A mid-operation identity change is rejected.
        journal
            .append(record(
                "op",
                DNS_FLUSH_ID,
                MaintenanceActionClass::Execute,
                &digest,
                validated(),
            ))
            .unwrap();
        assert!(matches!(
            journal.append(record(
                "op",
                SETTINGS_SEARCH_ID,
                MaintenanceActionClass::SettingsHandoff,
                &digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::DispatchStarted,
                    status_code: OptimizeAuditStatusCode::DispatchStarted,
                    error_code: None,
                    adapter_outcome: None,
                },
            )),
            Err(OptimizeAuditError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn journal_is_durable_redacted_and_rejects_unknown_versions() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("optimize.jsonl");
        let digest = digest();
        {
            let mut journal = OptimizeAuditJournal::open(&path).unwrap();
            journal
                .append(record(
                    "op",
                    DNS_FLUSH_ID,
                    MaintenanceActionClass::Execute,
                    &digest,
                    validated(),
                ))
                .unwrap();
        }
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.ends_with('\n'));
        for forbidden in ["program", "argv", "ipconfig", "ms-settings", "rollback"] {
            assert!(!text.contains(forbidden), "audit leaked {forbidden}");
        }
        let unknown = text.replacen("\"schema_version\":1", "\"schema_version\":2", 1);
        std::fs::write(&path, unknown).unwrap();
        assert!(matches!(
            OptimizeAuditJournal::open(&path),
            Err(OptimizeAuditError::UnknownVersion(_))
        ));
    }

    #[test]
    fn exclusive_lock_conflict_does_not_modify_journal() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("optimize.jsonl");
        let journal = OptimizeAuditJournal::open(&path).unwrap();
        let original = std::fs::read(&path).unwrap_or_default();
        assert!(matches!(
            OptimizeAuditJournal::open(&path),
            Err(OptimizeAuditError::LockUnavailable(_))
        ));
        assert_eq!(std::fs::read(&path).unwrap_or_default(), original);
        drop(journal);
    }
}
