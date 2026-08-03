use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use devsweep_core::{
    execution::{
        ConfirmationDigest, ExecutionReport, ExecutionRequest, Executor, UserProtectionList,
    },
    model::{ScanReport, TargetId, UntrustedPlan},
    plan::validate_plan,
    scan::ScanOptions,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    error::CommandError,
    scan::{CoreScanRunner, SCAN_PROGRESS_EVENT, ScanCoordinator, run_scan_job},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DryRunOutcome {
    pub report: ExecutionReport,
    pub digest: ConfirmationDigest,
}

pub(crate) trait ProtectionStore {
    fn get(&self) -> Result<Vec<PathBuf>>;
    fn set(&self, paths: Vec<PathBuf>) -> Result<Vec<PathBuf>>;
}

struct CoreProtectionStore;

impl ProtectionStore for CoreProtectionStore {
    fn get(&self) -> Result<Vec<PathBuf>> {
        Ok(UserProtectionList::load()?.list().to_vec())
    }

    fn set(&self, paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
        let mut list = UserProtectionList::load()?;
        list.replace(paths)?;
        Ok(list.list().to_vec())
    }
}

#[tauri::command]
pub(crate) async fn scan_start(
    app: AppHandle,
    state: State<'_, ScanCoordinator>,
    options: ScanOptions,
) -> Result<ScanReport, CommandError> {
    let cancel = state.begin()?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    let worker_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_scan_job(&CoreScanRunner, &options, &worker_cancel, |progress| {
            let _ = worker_app.emit(SCAN_PROGRESS_EVENT, progress);
        })
        .map_err(CommandError::scan_failed);
        worker_state.finish(&worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn scan_cancel(state: State<'_, ScanCoordinator>) -> Result<(), CommandError> {
    state.cancel()
}

#[tauri::command]
pub(crate) async fn plan_dry_run(
    plan: UntrustedPlan,
    selected_ids: Vec<TargetId>,
) -> Result<DryRunOutcome, CommandError> {
    tauri::async_runtime::spawn_blocking(move || plan_dry_run_inner(plan, selected_ids))
        .await
        .map_err(CommandError::io)?
}

fn plan_dry_run_inner(
    plan: UntrustedPlan,
    selected_ids: Vec<TargetId>,
) -> Result<DryRunOutcome, CommandError> {
    let validated = validate_plan(&plan).map_err(CommandError::invalid_plan)?;
    let report = Executor::default()
        .run_plan(
            &validated,
            ExecutionRequest {
                execute: false,
                audit_log: None,
                selected: selected_ids,
                expected_digest: None,
                cancel: None,
            },
        )
        .map_err(CommandError::execution)?;
    Ok(DryRunOutcome {
        digest: report.confirmation_digest.clone(),
        report,
    })
}

#[tauri::command]
pub(crate) async fn plan_execute(
    app: AppHandle,
    plan: UntrustedPlan,
    selected_ids: Vec<TargetId>,
    digest: ConfirmationDigest,
) -> Result<ExecutionReport, CommandError> {
    let audit_log = app
        .path()
        .app_data_dir()
        .map_err(CommandError::io)?
        .join("audit")
        .join("devsweep-audit.jsonl");
    tauri::async_runtime::spawn_blocking(move || {
        plan_execute_inner(plan, selected_ids, digest, audit_log)
    })
    .await
    .map_err(CommandError::io)?
}

fn plan_execute_inner(
    plan: UntrustedPlan,
    selected_ids: Vec<TargetId>,
    digest: ConfirmationDigest,
    audit_log: PathBuf,
) -> Result<ExecutionReport, CommandError> {
    let validated = validate_plan(&plan).map_err(CommandError::invalid_plan)?;
    Executor::default()
        .run_plan(
            &validated,
            ExecutionRequest {
                execute: true,
                audit_log: Some(audit_log),
                selected: selected_ids,
                expected_digest: Some(digest),
                cancel: None,
            },
        )
        .map_err(CommandError::execution)
}

#[tauri::command]
pub(crate) async fn protection_list_get() -> Result<Vec<PathBuf>, CommandError> {
    tauri::async_runtime::spawn_blocking(|| CoreProtectionStore.get())
        .await
        .map_err(CommandError::io)?
        .map_err(CommandError::io)
}

#[tauri::command]
pub(crate) async fn protection_list_set(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || CoreProtectionStore.set(paths))
        .await
        .map_err(CommandError::io)?
        .map_err(CommandError::io)
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

    use devsweep_core::model::{
        CLEANUP_PLAN_VERSION, CleanupIntent, Ecosystem, Evidence, RiskLevel, Scope, TargetKind,
        UntrustedTarget,
    };
    #[cfg(windows)]
    use devsweep_core::{
        execution::{CommandRequest, CommandRunner, ProcessCommandRunner},
        process::FlagCancelObserver,
    };

    use super::*;

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
    fn empty_plan_dry_run_and_execute_accept_matching_digest() {
        let plan = UntrustedPlan::empty();
        let dry_run = plan_dry_run_inner(plan.clone(), Vec::new()).expect("dry run succeeds");
        let temp = tempfile::tempdir().expect("temp directory");

        let report = plan_execute_inner(
            plan,
            Vec::new(),
            dry_run.digest,
            temp.path().join("audit.jsonl"),
        )
        .expect("matching digest executes");

        assert!(!report.dry_run);
        assert_eq!(report.selected, 0);
        assert!(report.audit_log.is_some());
    }

    #[test]
    fn stale_digest_is_structured() {
        let error = plan_execute_inner(
            UntrustedPlan::empty(),
            Vec::new(),
            ConfirmationDigest::new("stale"),
            PathBuf::from("unused.jsonl"),
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

        let error = plan_execute_inner(
            plan,
            vec![second],
            dry_run.digest,
            PathBuf::from("unused.jsonl"),
        )
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
