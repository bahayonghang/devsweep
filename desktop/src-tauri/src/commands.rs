use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use devsweep_core::{
    execution::UserProtectionList,
    presentation_settings::{
        PresentationLanguageTag, PresentationSettingsV1, load_presentation_settings,
        save_presentation_settings,
    },
    scan::ScanOptions,
};
use tauri::{State, ipc::Channel};

use crate::{
    error::CommandError,
    scan::{CoreScanRunner, DesktopScanProgress, DesktopScanResult, ScanCoordinator, run_scan_job},
};

/// Serializes settings calls within this process while the core store provides
/// the cross-process transaction lock.
#[derive(Clone, Default)]
pub(crate) struct PresentationSettingsCoordinator(Arc<Mutex<()>>);

#[tauri::command]
pub(crate) async fn presentation_settings_get(
    state: State<'_, PresentationSettingsCoordinator>,
) -> Result<PresentationSettingsV1, CommandError> {
    let coordinator = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = coordinator
            .0
            .lock()
            .map_err(|_| CommandError::io("presentation settings coordinator is poisoned"))?;
        load_presentation_settings().map_err(CommandError::io)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn presentation_settings_set(
    state: State<'_, PresentationSettingsCoordinator>,
    language: Option<PresentationLanguageTag>,
) -> Result<PresentationSettingsV1, CommandError> {
    let coordinator = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = coordinator
            .0
            .lock()
            .map_err(|_| CommandError::io("presentation settings coordinator is poisoned"))?;
        let settings = PresentationSettingsV1 { language };
        save_presentation_settings(settings).map_err(CommandError::io)?;
        Ok(settings)
    })
    .await
    .map_err(CommandError::io)?
}

#[cfg(debug_assertions)]
fn debug_native_fault_mode_from(value: Option<std::ffi::OsString>) -> &'static str {
    match value.as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("route_cancel_once") => "route_cancel_once",
        _ => "disabled",
    }
}

/// Closed native-evidence seam. The command is absent from release builds and
/// cannot receive paths, commands, argv, or cleanup authority.
#[cfg(debug_assertions)]
#[tauri::command]
pub(crate) fn debug_native_fault_mode() -> &'static str {
    debug_native_fault_mode_from(std::env::var_os("DEVSWEEP_TASK_NATIVE_FAULT"))
}

fn protection_list_get_inner() -> Result<Vec<PathBuf>, CommandError> {
    UserProtectionList::load()
        .map(|list| list.list().to_vec())
        .map_err(CommandError::protection)
}

fn protection_list_set_inner(_paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, CommandError> {
    Err(CommandError::ProtectionConfirmationRequired {
        message: "protection mutations require protection_add or protection_remove with explicit confirmation"
            .to_string(),
    })
}

#[tauri::command]
pub(crate) async fn scan_start(
    state: State<'_, ScanCoordinator>,
    scan_id: String,
    options: ScanOptions,
    on_progress: Channel<DesktopScanProgress>,
) -> Result<DesktopScanResult, CommandError> {
    let cancel = state.begin(scan_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_scan_job(
            &CoreScanRunner,
            &options,
            scan_id.clone(),
            &worker_cancel,
            |progress| on_progress.send(progress).map_err(anyhow::Error::from),
        )
        .map_err(CommandError::scan_failed);
        worker_state.finish(&scan_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn scan_cancel(
    state: State<'_, ScanCoordinator>,
    scan_id: String,
) -> Result<(), CommandError> {
    state.cancel(&scan_id)
}

#[tauri::command]
pub(crate) async fn protection_list_get() -> Result<Vec<PathBuf>, CommandError> {
    tauri::async_runtime::spawn_blocking(protection_list_get_inner)
        .await
        .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn protection_list_set(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, CommandError> {
    protection_list_set_inner(paths)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    #[cfg(windows)]
    use std::{
        fs,
        process::Command,
        thread,
        time::{Duration, Instant},
    };

    use devsweep_core::{
        execution::ConfirmationDigest,
        model::{
            CLEANUP_PLAN_VERSION, CleanupIntent, Ecosystem, Evidence, RiskLevel, Scope, TargetId,
            TargetKind, UntrustedPlan, UntrustedTarget,
        },
    };
    #[cfg(windows)]
    use devsweep_core::{
        execution::{CommandRequest, CommandRunner, ProcessCommandRunner},
        process::FlagCancelObserver,
    };

    use super::*;
    use crate::clean::{plan_dry_run_inner, plan_execute_inner};

    trait ProtectionStore {
        fn get(&self) -> Result<Vec<PathBuf>>;
        fn set(&self, paths: Vec<PathBuf>) -> Result<Vec<PathBuf>>;
    }

    #[derive(Default)]
    struct MemoryProtectionStore(Mutex<Vec<PathBuf>>);

    impl ProtectionStore for MemoryProtectionStore {
        fn get(&self) -> Result<Vec<PathBuf>> {
            Ok(self.0.lock().expect("memory protection lock").clone())
        }

        fn set(&self, paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
            *self.0.lock().expect("memory protection lock") = paths;
            self.get()
        }
    }

    #[test]
    fn presentation_setting_ipc_payload_is_locale_only_and_closed() {
        assert_eq!(
            serde_json::to_value(PresentationSettingsV1 {
                language: Some(PresentationLanguageTag::ZhCn),
            })
            .unwrap(),
            serde_json::json!({ "language": "zh-CN" })
        );
        assert!(
            serde_json::from_value::<PresentationSettingsV1>(
                serde_json::json!({ "language": "en", "future": true })
            )
            .is_err()
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn debug_native_fault_mode_is_exact_and_closed() {
        assert_eq!(debug_native_fault_mode_from(None), "disabled");
        assert_eq!(
            debug_native_fault_mode_from(Some("route_cancel_once".into())),
            "route_cancel_once"
        );
        assert_eq!(
            debug_native_fault_mode_from(Some("route_cancel_always".into())),
            "disabled"
        );
        assert_eq!(
            debug_native_fault_mode_from(Some("ROUTE_CANCEL_ONCE".into())),
            "disabled"
        );
    }

    #[test]
    fn empty_plan_dry_run_accepts_matching_digest() {
        let dry_run =
            plan_dry_run_inner(UntrustedPlan::empty(), Vec::new()).expect("dry run succeeds");
        assert!(dry_run.report.dry_run);
        assert_eq!(dry_run.report.selected, 0);
        assert!(dry_run.report.audit_log.is_none());
    }

    #[test]
    fn stale_digest_is_structured() {
        let error = plan_execute_inner(
            UntrustedPlan::empty(),
            Vec::new(),
            ConfirmationDigest::new("stale"),
        )
        .expect_err("stale confirmation is rejected");

        assert!(matches!(error, CommandError::StaleConfirmation { .. }));
    }

    #[test]
    fn changed_selection_rejects_previous_digest() {
        let plan = command_plan();
        let first = plan.targets[0].id.clone();
        let second = plan.targets[1].id.clone();
        let dry_run = plan_dry_run_inner(plan.clone(), vec![first]).expect("dry run succeeds");

        let error = plan_execute_inner(plan, vec![second], dry_run.digest)
            .expect_err("changed selection invalidates digest");

        assert!(matches!(error, CommandError::StaleConfirmation { .. }));
    }

    #[test]
    fn unknown_target_is_structured() {
        let error = plan_dry_run_inner(UntrustedPlan::empty(), vec![TargetId::new("unknown")])
            .expect_err("unknown selection is rejected");

        assert_eq!(
            error,
            CommandError::UnknownTarget {
                target_id: TargetId::new("unknown")
            }
        );
    }

    #[test]
    fn invalid_plan_is_structured() {
        let error = plan_dry_run_inner(
            UntrustedPlan {
                version: CLEANUP_PLAN_VERSION + 1,
                targets: Vec::new(),
            },
            Vec::new(),
        )
        .expect_err("invalid plan is rejected");

        assert!(matches!(error, CommandError::InvalidPlan { .. }));
    }

    #[test]
    fn inspect_only_target_is_structured() {
        let plan = inspect_only_plan();
        let selected = plan.targets[0].id.clone();

        let error = plan_dry_run_inner(plan, vec![selected.clone()])
            .expect_err("inspect-only selection is rejected");

        assert_eq!(
            error,
            CommandError::InspectOnlyTarget {
                target_id: selected
            }
        );
    }

    #[test]
    fn protection_store_round_trips_paths() {
        let store = MemoryProtectionStore::default();
        let paths = vec![PathBuf::from("C:/work/keep")];

        assert_eq!(store.set(paths.clone()).expect("set paths"), paths);
        assert_eq!(store.get().expect("get paths"), paths);
    }

    #[test]
    fn protection_list_set_cannot_bypass_confirm_lock_or_audit() {
        let error = protection_list_set_inner(vec![PathBuf::from("C:/secret-keep")])
            .expect_err("legacy protection_list_set must refuse mutation");
        assert_eq!(
            error,
            CommandError::ProtectionConfirmationRequired {
                message: "protection mutations require protection_add or protection_remove with explicit confirmation"
                    .to_string(),
            }
        );
        let payload = serde_json::to_value(&error).expect("json");
        assert_eq!(payload["code"], "protection_confirmation_required");
    }

    #[cfg(windows)]
    #[test]
    fn process_command_runner_terminates_tree_inside_desktop_process() {
        let temp = tempfile::tempdir().expect("temp directory");
        let fixture = temp.path().join("desktop-process-fixture.exe");
        std::fs::copy(
            std::env::current_exe().expect("current test executable"),
            &fixture,
        )
        .expect("copy controlled process fixture");

        let cancel = Arc::new(FlagCancelObserver::new());
        let cancel_worker = Arc::clone(&cancel);
        let pid_file = temp.path().join("grandchild.pid");
        let cancel_thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if let Ok(pid) = fs::read_to_string(&pid_file)
                    .and_then(|value| value.trim().parse::<u32>().map_err(std::io::Error::other))
                {
                    cancel_worker.request_cancel();
                    return pid;
                }
                thread::sleep(Duration::from_millis(20));
            }
            panic!("grandchild pid was not recorded before cancellation deadline");
        });

        let error = ProcessCommandRunner::default()
            .run_with_cancel(
                &CommandRequest {
                    program: fixture.display().to_string(),
                    args: vec![
                        "--ignored".to_string(),
                        "--exact".to_string(),
                        "commands::tests::job_object_fixture_child".to_string(),
                        "--nocapture".to_string(),
                    ],
                    cwd: Some(temp.path().to_path_buf()),
                },
                cancel.as_ref(),
            )
            .expect_err("production process runner observes cancellation");
        assert!(error.to_string().contains("canceled"));

        let grandchild_pid = cancel_thread.join().expect("cancel thread joins");
        let deadline = Instant::now() + Duration::from_secs(3);
        while process_is_alive(grandchild_pid) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(50));
        }
        assert!(
            !process_is_alive(grandchild_pid),
            "Job Object did not terminate grandchild pid {grandchild_pid}"
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "process fixture entrypoint"]
    fn job_object_fixture_child() {
        let mut child = Command::new(std::env::current_exe().expect("fixture executable"))
            .args([
                "--ignored",
                "--exact",
                "commands::tests::job_object_fixture_grandchild",
                "--nocapture",
            ])
            .spawn()
            .expect("spawn fixture grandchild");
        fs::write("grandchild.pid", child.id().to_string()).expect("record grandchild pid");
        thread::sleep(Duration::from_secs(60));
        child.wait().expect("fixture grandchild exits");
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "process fixture entrypoint"]
    fn job_object_fixture_grandchild() {
        thread::sleep(Duration::from_secs(60));
    }

    #[cfg(windows)]
    fn process_is_alive(pid: u32) -> bool {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_TIMEOUT},
            System::Threading::{
                OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
                WaitForSingleObject,
            },
        };

        unsafe {
            let handle = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                pid,
            );
            if handle.is_null() {
                return false;
            }
            let status = WaitForSingleObject(handle, 0);
            CloseHandle(handle);
            status == WAIT_TIMEOUT
        }
    }

    fn command_plan() -> UntrustedPlan {
        UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![
                command_target(
                    "npm.cache.clean:test",
                    "npm.cache.clean",
                    Ecosystem::Node,
                    "npm",
                    "cache_clean",
                ),
                command_target(
                    "pip.cache.purge:test",
                    "pip.cache.purge",
                    Ecosystem::Python,
                    "python",
                    "cache_purge",
                ),
            ],
        }
    }

    fn command_target(
        id: &str,
        rule_id: &str,
        ecosystem: Ecosystem,
        provider_id: &str,
        action_id: &str,
    ) -> UntrustedTarget {
        UntrustedTarget {
            id: TargetId::new(id),
            rule_id: rule_id.to_string(),
            scope: Scope::Global,
            ecosystem,
            kind: TargetKind::PackageCache,
            path: None,
            estimated_bytes: 1,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Medium,
            reversible: false,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: rule_id.to_string(),
            }],
            intent: CleanupIntent::RunBuiltInAction {
                provider_id: provider_id.to_string(),
                action_id: action_id.to_string(),
            },
        }
    }

    fn inspect_only_plan() -> UntrustedPlan {
        let cargo_home = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("USERPROFILE").expect("Windows user profile"))
                    .join(".cargo")
            });
        UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![UntrustedTarget {
                id: TargetId::new("cargo.home.inspect:test"),
                rule_id: "cargo.home.inspect".to_string(),
                scope: Scope::Global,
                ecosystem: Ecosystem::Rust,
                kind: TargetKind::PackageCache,
                path: Some(cargo_home),
                estimated_bytes: 0,
                size_complete: true,
                sizing_warnings: Vec::new(),
                last_modified: None,
                risk: RiskLevel::High,
                reversible: true,
                selected_by_default: false,
                evidence: vec![Evidence::RuleMatched {
                    rule_id: "cargo.home.inspect".to_string(),
                }],
                intent: CleanupIntent::InspectOnly {
                    rule_id: "cargo.home.inspect".to_string(),
                },
            }],
        }
    }
}
