use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Instant};

#[cfg(test)]
use std::cell::RefCell;

use anyhow::{Result, anyhow, bail};

mod audit;
mod command;
mod safety;

use audit::default_audit_log_path;
#[cfg(test)]
use audit::replay_unconfirmed_starts;
#[cfg(test)]
use command::CommandOutcome;
use command::{
    CommandRequest, CommandRunner, ProcessCommandRunner, SystemTrashRunner, TrashRunner,
};
pub(crate) use safety::UserProtectionList;
use safety::{
    AuthorizationContext, AuthorizedAction, ProtectionCategory, SELF_CLEAN_SKIP_MESSAGE,
    SafetyPolicy,
};

#[cfg(test)]
use audit::JournalIo;
use audit::{AuditJournal, JournalEvent, action_path_from_authorized, command_from_action};
use command::command_argv;

use crate::{
    model::{CleanAction, CleanTarget, TargetId},
    plan::{ValidatedPlan, ValidatedTarget},
    process::{CancelObserver, FlagCancelObserver, NoopCancelObserver, sanitize_process_output},
};

const EXECUTOR_DIAGNOSTIC_CAP: usize = 4 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct ExecutionRequest {
    pub execute: bool,
    pub audit_log: Option<PathBuf>,
    pub selected: Vec<TargetId>,
    /// Cooperative cancel flag shared with the UI/runtime.
    pub cancel: Option<Arc<crate::process::FlagCancelObserver>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionReport {
    pub dry_run: bool,
    pub selected: usize,
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub failures: Vec<ActionFailure>,
    pub audit_log: Option<PathBuf>,
}

impl ExecutionReport {
    pub(crate) fn has_failures(&self) -> bool {
        self.failed > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionFailure {
    pub target_id: TargetId,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionTargetStatus {
    Succeeded,
    Failed,
    Skipped,
    /// Side effect may have run but durable terminal audit failed.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionProgress {
    pub completed: usize,
    pub total: usize,
    pub target_id: TargetId,
    pub status: ExecutionTargetStatus,
    pub message: String,
}

pub(crate) struct Executor<C = ProcessCommandRunner, T = SystemTrashRunner> {
    command_runner: C,
    trash_runner: T,
    safety: SafetyPolicy,
    #[cfg(test)]
    journal_io: RefCell<Option<Box<dyn JournalIo>>>,
}

impl Default for Executor<ProcessCommandRunner, SystemTrashRunner> {
    fn default() -> Self {
        Self::new(ProcessCommandRunner::new(), SystemTrashRunner)
    }
}

impl<C, T> Executor<C, T> {
    pub(crate) fn new(command_runner: C, trash_runner: T) -> Self {
        Self {
            command_runner,
            trash_runner,
            safety: SafetyPolicy::from_user_list(
                UserProtectionList::load()
                    .unwrap_or_else(|_| UserProtectionList::empty_in_memory_for_tests_only()),
            ),
            #[cfg(test)]
            journal_io: RefCell::new(None),
        }
    }

    #[cfg(test)]
    fn with_safety_policy(mut self, safety: SafetyPolicy) -> Self {
        self.safety = safety;
        self
    }

    #[cfg(test)]
    fn with_journal_io(self, io: Box<dyn JournalIo>) -> Self {
        *self.journal_io.borrow_mut() = Some(io);
        self
    }
}

impl<C, T> Executor<C, T>
where
    C: CommandRunner,
    T: TrashRunner,
{
    pub(crate) fn run_plan(
        &self,
        plan: &ValidatedPlan,
        request: ExecutionRequest,
    ) -> Result<ExecutionReport> {
        self.run_plan_with_progress(plan, request, |_| {})
    }

    pub(crate) fn run_plan_with_progress<F>(
        &self,
        plan: &ValidatedPlan,
        request: ExecutionRequest,
        mut on_progress: F,
    ) -> Result<ExecutionReport>
    where
        F: FnMut(ExecutionProgress),
    {
        let selected_ids: HashSet<&TargetId> = request.selected.iter().collect();
        let selected_targets: Vec<_> = plan
            .targets()
            .iter()
            .filter(|target| selected_ids.contains(&target.target().id))
            .collect();

        if let Some(target) = selected_targets
            .iter()
            .find(|target| !target.target().action.is_executable())
        {
            bail!(
                "target {} has no executable cleanup action",
                target.target().id.as_str()
            )
        }

        if !request.execute {
            return Ok(ExecutionReport {
                dry_run: true,
                selected: selected_targets.len(),
                attempted: 0,
                succeeded: 0,
                failed: 0,
                skipped: selected_targets.len(),
                failures: Vec::new(),
                audit_log: None,
            });
        }

        let audit_path = match request.audit_log {
            Some(path) => path,
            None => default_audit_log_path()?,
        };
        #[cfg(test)]
        let mut journal = match self.journal_io.borrow_mut().take() {
            Some(io) => AuditJournal::with_io(audit_path.clone(), io),
            None => AuditJournal::open(&audit_path, plan.digest().to_string())?,
        };
        #[cfg(not(test))]
        let mut journal = AuditJournal::open(&audit_path, plan.digest().to_string())?;
        let mut report = ExecutionReport {
            dry_run: false,
            selected: selected_targets.len(),
            attempted: 0,
            succeeded: 0,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
            audit_log: Some(audit_path.clone()),
        };

        let total = selected_targets.len();
        let mut executed_fingerprints = HashSet::new();
        let auth_context = AuthorizationContext {
            scan_roots: Vec::new(),
            audit_log: Some(audit_path.clone()),
            expected_identity: None,
        };
        let cancel = request.cancel.clone();
        for validated_target in selected_targets {
            if cancel
                .as_ref()
                .is_some_and(|flag| flag.is_cancel_requested())
            {
                report.skipped += 1;
                report.failures.push(ActionFailure {
                    target_id: validated_target.target().id.clone(),
                    message: "canceled before action started".to_string(),
                });
                on_progress(ExecutionProgress {
                    completed: report.succeeded + report.failed + report.skipped,
                    total,
                    target_id: validated_target.target().id.clone(),
                    status: ExecutionTargetStatus::Skipped,
                    message: "canceled".to_string(),
                });
                // Remaining targets are not started once cancel is observed.
                break;
            }

            let target = validated_target.target();
            let started_at = Instant::now();
            let dispatch_attempted =
                executed_fingerprints.insert(validated_target.fingerprint().clone());

            if !dispatch_attempted {
                report.failed += 1;
                let message = format!(
                    "duplicate action fingerprint rejected before execution: {}",
                    validated_target.fingerprint().as_str()
                );
                report.failures.push(ActionFailure {
                    target_id: target.id.clone(),
                    message: message.clone(),
                });
                on_progress(ExecutionProgress {
                    completed: report.succeeded + report.failed + report.skipped,
                    total,
                    target_id: target.id.clone(),
                    status: ExecutionTargetStatus::Failed,
                    message,
                });
                continue;
            }

            let authorized = match self.authorize_target(validated_target, &auth_context) {
                Ok(ActionPrep::Skipped { message }) => {
                    report.skipped += 1;
                    let seq = journal.next_sequence();
                    let _ = journal.write_terminal(JournalEvent::skipped(
                        journal.run_id(),
                        seq,
                        plan.digest(),
                        target,
                        message.clone(),
                        started_at.elapsed().as_millis(),
                    ));
                    on_progress(ExecutionProgress {
                        completed: report.succeeded + report.failed + report.skipped,
                        total,
                        target_id: target.id.clone(),
                        status: ExecutionTargetStatus::Skipped,
                        message: format!("skipped: {message}"),
                    });
                    continue;
                }
                Ok(ActionPrep::Ready(authorized)) => authorized,
                Err(error) => {
                    report.attempted += 1;
                    report.failed += 1;
                    let message = sanitize_process_output(
                        error.to_string().as_bytes(),
                        EXECUTOR_DIAGNOSTIC_CAP,
                    );
                    let run_id = journal.run_id().to_string();
                    let sequence = journal.next_sequence();
                    let terminal = JournalEvent::finished(
                        &run_id,
                        sequence,
                        plan.digest(),
                        target,
                        "failed",
                        command_from_action(&target.action),
                        None,
                        target.path.clone(),
                        started_at.elapsed().as_millis(),
                        Some(message.clone()),
                    );
                    if let Err(audit_error) = journal.write_terminal(terminal) {
                        let audit_message = format!(
                            "authorization denied ({message}); audit persistence failed: {audit_error}"
                        );
                        let message = sanitize_process_output(
                            audit_message.as_bytes(),
                            EXECUTOR_DIAGNOSTIC_CAP,
                        );
                        report.failures.push(ActionFailure {
                            target_id: target.id.clone(),
                            message: message.clone(),
                        });
                        on_progress(ExecutionProgress {
                            completed: report.succeeded + report.failed + report.skipped,
                            total,
                            target_id: target.id.clone(),
                            status: ExecutionTargetStatus::Failed,
                            message,
                        });
                        // Do not dispatch another target without recording this denial.
                        break;
                    }
                    report.failures.push(ActionFailure {
                        target_id: target.id.clone(),
                        message: message.clone(),
                    });
                    on_progress(ExecutionProgress {
                        completed: report.succeeded + report.failed + report.skipped,
                        total,
                        target_id: target.id.clone(),
                        status: ExecutionTargetStatus::Failed,
                        message,
                    });
                    continue;
                }
            };

            let run_id = journal.run_id().to_string();
            let seq = journal.next_sequence();
            let started_event =
                JournalEvent::started(&run_id, seq, plan.digest(), target, &authorized);
            if let Err(error) = journal.write_started_durable(&started_event) {
                report.failed += 1;
                let message = format!("audit-blocked before side effect: {error}");
                report.failures.push(ActionFailure {
                    target_id: target.id.clone(),
                    message: message.clone(),
                });
                on_progress(ExecutionProgress {
                    completed: report.succeeded + report.failed + report.skipped,
                    total,
                    target_id: target.id.clone(),
                    status: ExecutionTargetStatus::Failed,
                    message,
                });
                // Halt further actions after audit-start failure.
                break;
            }

            let outcome = self.dispatch_authorized(&authorized, target, cancel.as_deref());
            let duration_ms = started_at.elapsed().as_millis();
            let finish_seq = journal.next_sequence();
            let run_id = journal.run_id().to_string();
            let (progress_status, progress_message, terminal) = match outcome {
                Ok(ActionStatus::Success {
                    command,
                    exit_code,
                    action_path,
                }) => {
                    report.attempted += 1;
                    report.succeeded += 1;
                    (
                        ExecutionTargetStatus::Succeeded,
                        "completed".to_string(),
                        JournalEvent::finished(
                            &run_id,
                            finish_seq,
                            plan.digest(),
                            target,
                            "success",
                            command,
                            exit_code,
                            action_path,
                            duration_ms,
                            None,
                        ),
                    )
                }
                Ok(ActionStatus::Skipped { message }) => {
                    report.skipped += 1;
                    (
                        ExecutionTargetStatus::Skipped,
                        format!("skipped: {message}"),
                        JournalEvent::finished(
                            &run_id,
                            finish_seq,
                            plan.digest(),
                            target,
                            "skipped",
                            None,
                            None,
                            action_path_from_authorized(&authorized),
                            duration_ms,
                            Some(message),
                        ),
                    )
                }
                Err(error) => {
                    report.attempted += 1;
                    report.failed += 1;
                    let message = error.to_string();
                    report.failures.push(ActionFailure {
                        target_id: target.id.clone(),
                        message: message.clone(),
                    });
                    (
                        ExecutionTargetStatus::Failed,
                        message.clone(),
                        JournalEvent::finished(
                            &run_id,
                            finish_seq,
                            plan.digest(),
                            target,
                            "failed",
                            command_from_action(&target.action),
                            None,
                            action_path_from_authorized(&authorized),
                            duration_ms,
                            Some(message),
                        ),
                    )
                }
            };

            if let Err(error) = journal.write_terminal(terminal) {
                // Side effect already ran; durable terminal record failed.
                if progress_status == ExecutionTargetStatus::Succeeded {
                    report.succeeded = report.succeeded.saturating_sub(1);
                }
                report.failed += 1;
                let message = format!(
                    "action result known ({progress_message}); audit persistence failed: {error}"
                );
                report.failures.push(ActionFailure {
                    target_id: target.id.clone(),
                    message: message.clone(),
                });
                on_progress(ExecutionProgress {
                    completed: report.succeeded + report.failed + report.skipped,
                    total,
                    target_id: target.id.clone(),
                    status: ExecutionTargetStatus::Unknown,
                    message,
                });
                break;
            }

            on_progress(ExecutionProgress {
                completed: report.succeeded + report.failed + report.skipped,
                total,
                target_id: target.id.clone(),
                status: progress_status,
                message: progress_message,
            });
        }

        let _ = journal.flush();
        Ok(report)
    }

    fn authorize_target(
        &self,
        validated: &ValidatedTarget,
        context: &AuthorizationContext,
    ) -> Result<ActionPrep> {
        let mut context = AuthorizationContext {
            scan_roots: context.scan_roots.clone(),
            audit_log: context.audit_log.clone(),
            expected_identity: validated.path_identity().cloned(),
        };

        match self.safety.authorize(validated, &context) {
            Ok(authorized) => {
                context.expected_identity = None;
                Ok(ActionPrep::Ready(authorized))
            }
            Err(denial)
                if denial.category == ProtectionCategory::ProtectedSubtree
                    && denial.message.contains(SELF_CLEAN_SKIP_MESSAGE) =>
            {
                Ok(ActionPrep::Skipped {
                    message: SELF_CLEAN_SKIP_MESSAGE.to_string(),
                })
            }
            Err(denial) => Err(anyhow!(denial.to_string())),
        }
    }

    fn dispatch_authorized(
        &self,
        authorized: &AuthorizedAction,
        target: &CleanTarget,
        cancel: Option<&FlagCancelObserver>,
    ) -> Result<ActionStatus> {
        let _ = target;
        match authorized.action() {
            CleanAction::Command {
                program,
                args,
                cwd,
                irreversible: _,
            } => {
                let request = CommandRequest {
                    program: program.clone(),
                    args: args.clone(),
                    cwd: cwd.clone(),
                };
                let noop = NoopCancelObserver;
                let observer: &dyn CancelObserver = match cancel {
                    Some(flag) => flag,
                    None => &noop,
                };
                let outcome = self.command_runner.run_with_cancel(&request, observer)?;
                Ok(ActionStatus::Success {
                    command: Some(command_argv(&request)),
                    exit_code: outcome.code,
                    action_path: cwd.clone().or_else(|| Some(PathBuf::from(program))),
                })
            }
            CleanAction::MoveToTrash { path } => {
                let path = authorized.trash_path().unwrap_or(path.as_path());
                self.trash_runner.move_to_trash(path)?;
                Ok(ActionStatus::Success {
                    command: None,
                    exit_code: None,
                    action_path: Some(path.to_path_buf()),
                })
            }
            CleanAction::DeletePermanently { .. } => {
                bail!("permanent delete is disabled in this build")
            }
            CleanAction::NoopInspectOnly => Ok(ActionStatus::Skipped {
                message: "inspect-only target has no executable cleanup action".to_string(),
            }),
        }
    }
}

enum ActionPrep {
    Ready(AuthorizedAction),
    Skipped { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActionStatus {
    Success {
        command: Option<Vec<String>>,
        exit_code: Option<i32>,
        action_path: Option<PathBuf>,
    },
    Skipped {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        fs,
        path::{Path, PathBuf},
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use anyhow::{Result, anyhow};
    use serde_json::Value;
    use tempfile::TempDir;

    use super::*;
    use crate::model::{
        CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel, Scope, TargetKind,
    };

    fn test_validated_plan(plan: &CleanupPlan) -> ValidatedPlan {
        ValidatedPlan::from_cleanup_plan_for_test(plan.clone())
    }

    #[test]
    fn dry_run_does_not_call_command_or_trash_runners() {
        let fixture = TempDir::new().expect("temp dir");
        let cleanup_path = fixture.path().join("node_modules");
        fs::create_dir_all(&cleanup_path).expect("cleanup dir");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "node.node_modules",
                CleanAction::MoveToTrash {
                    path: cleanup_path.clone(),
                },
                Some(cleanup_path.clone()),
            )],
        };
        let command_runner = RecordingCommandRunner::default();
        let trash_runner = RecordingTrashRunner::default();
        let executor = test_executor(command_runner.clone(), trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: false,
                    audit_log: None,
                    cancel: None,
                },
            )
            .expect("dry-run succeeds");

        assert!(report.dry_run);
        assert_eq!(report.selected, 1);
        assert_eq!(report.attempted, 0);
        assert!(cleanup_path.exists(), "dry-run must not move fixture");
        assert!(command_runner.requests().is_empty());
        assert!(trash_runner.paths().is_empty());
    }

    #[test]
    fn command_action_uses_program_and_argv_without_shell_composition() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let manifest = fixture.path().join("Cargo.toml");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "test.command",
                CleanAction::Command {
                    program: "cargo".to_string(),
                    args: vec![
                        "clean".to_string(),
                        "--manifest-path".to_string(),
                        manifest.display().to_string(),
                    ],
                    cwd: None,
                    irreversible: true,
                },
                Some(fixture.path().join("target")),
            )],
        };
        let command_runner = RecordingCommandRunner::default();
        let executor = test_executor(command_runner.clone(), RecordingTrashRunner::default());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    cancel: None,
                },
            )
            .expect("execute succeeds");

        assert_eq!(report.succeeded, 1);
        assert_eq!(
            command_runner.requests(),
            vec![CommandRequest {
                program: "cargo".to_string(),
                args: vec![
                    "clean".to_string(),
                    "--manifest-path".to_string(),
                    manifest.display().to_string(),
                ],
                cwd: None,
            }]
        );

        let records = read_jsonl(&audit_path);
        assert!(records.iter().any(|r| r["event"] == "action_started"));
        let finished = records
            .iter()
            .find(|r| r["event"] == "action_finished")
            .expect("finished record");
        assert_eq!(finished["status"], "success");
        assert_eq!(finished["command"][0], "cargo");
        assert_eq!(finished["command"][3], manifest.display().to_string());
        assert!(finished.get("plan_digest").is_some());
        assert!(finished.get("run_id").is_some());
        assert!(finished.get("sequence").is_some());
    }

    #[test]
    fn trash_action_uses_only_path_from_cleanup_plan() {
        let fixture = TempDir::new().expect("temp dir");
        let cleanup_path = fixture.path().join("node_modules");
        let audit_path = fixture.path().join("audit.jsonl");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "node.node_modules",
                CleanAction::MoveToTrash {
                    path: cleanup_path.clone(),
                },
                Some(cleanup_path.clone()),
            )],
        };
        let trash_runner = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash_runner.clone());

        executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
            )
            .expect("execute succeeds");

        assert_eq!(trash_runner.paths(), vec![cleanup_path]);
    }

    #[test]
    fn audit_records_success_and_failure_while_job_continues() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let trash_path = fixture.path().join(".pytest_cache");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "python.pytest_cache",
                    CleanAction::MoveToTrash {
                        path: trash_path.clone(),
                    },
                    Some(trash_path),
                ),
                target(
                    "rust.target",
                    CleanAction::Command {
                        program: "cargo".to_string(),
                        args: vec!["clean".to_string()],
                        cwd: None,
                        irreversible: true,
                    },
                    None,
                ),
            ],
        };
        let command_runner = RecordingCommandRunner::failing("cargo unavailable");
        let trash_runner = RecordingTrashRunner::default();
        let executor = test_executor(command_runner, trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    cancel: None,
                },
            )
            .expect("job returns failure report instead of aborting");

        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failed, 1);
        assert!(report.has_failures());
        assert_eq!(trash_runner.paths().len(), 1);

        let records = read_jsonl(&audit_path);
        // started+finished for each of two targets
        assert_eq!(records.len(), 4);
        let finished: Vec<_> = records
            .iter()
            .filter(|r| r["event"] == "action_finished")
            .collect();
        assert_eq!(finished.len(), 2);
        assert_eq!(finished[0]["status"], "success");
        assert_eq!(finished[1]["status"], "failed");
        assert!(
            finished[1]["error"]
                .as_str()
                .expect("error string")
                .contains("cargo unavailable")
        );
    }

    #[test]
    fn trash_failure_reports_target_and_continues_remaining_targets() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let locked_path = fixture.path().join("node_modules");
        let later_path = fixture.path().join(".pytest_cache");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "node.node_modules",
                    CleanAction::MoveToTrash {
                        path: locked_path.clone(),
                    },
                    Some(locked_path.clone()),
                ),
                target(
                    "python.pytest_cache",
                    CleanAction::MoveToTrash {
                        path: later_path.clone(),
                    },
                    Some(later_path.clone()),
                ),
            ],
        };
        let failed_id = plan.targets[0].id.clone();
        let trash_runner =
            RecordingTrashRunner::failing_on(&locked_path, "Access denied: file is locked");
        let executor = test_executor(RecordingCommandRunner::default(), trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    cancel: None,
                },
            )
            .expect("job returns a partial-failure report");

        assert_eq!(report.attempted, 2);
        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.failures[0].target_id, failed_id);
        assert!(
            report.failures[0].message.contains("locked"),
            "locked-file style error should be preserved"
        );
        assert_eq!(trash_runner.paths(), vec![locked_path, later_path]);

        let records = read_jsonl(&audit_path);
        assert_eq!(records.len(), 4);
        let finished: Vec<_> = records
            .iter()
            .filter(|r| r["event"] == "action_finished")
            .collect();
        assert_eq!(finished.len(), 2);
        assert_eq!(finished[0]["status"], "failed");
        assert_eq!(finished[1]["status"], "success");
    }

    #[test]
    fn permanent_delete_action_is_rejected_before_execution() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let doomed = fixture.path().join("do-not-delete");
        fs::write(&doomed, "keep").expect("fixture file");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "dangerous.delete",
                CleanAction::DeletePermanently {
                    path: doomed.clone(),
                    requires_explicit_flag: true,
                },
                Some(doomed.clone()),
            )],
        };
        let executor = test_executor(
            RecordingCommandRunner::default(),
            RecordingTrashRunner::default(),
        );

        let error = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
            )
            .expect_err("disabled permanent delete cannot be selected");

        assert!(
            error
                .to_string()
                .contains("has no executable cleanup action")
        );
        assert!(doomed.exists(), "permanent delete must remain disabled");
    }

    #[test]
    fn observed_execution_reports_per_target_progress() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let first_path = fixture.path().join("node_modules");
        let second_path = fixture.path().join(".pytest_cache");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "node.node_modules",
                    CleanAction::MoveToTrash {
                        path: first_path.clone(),
                    },
                    Some(first_path),
                ),
                target(
                    "python.pytest_cache",
                    CleanAction::MoveToTrash {
                        path: second_path.clone(),
                    },
                    Some(second_path),
                ),
            ],
        };
        let executor = test_executor(
            RecordingCommandRunner::default(),
            RecordingTrashRunner::default(),
        );
        let mut progress = Vec::new();

        let report = executor
            .run_plan_with_progress(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
                |event| progress.push(event),
            )
            .expect("execute succeeds");

        assert_eq!(report.succeeded, 2);
        assert_eq!(progress.len(), 2);
        assert_eq!(progress[0].completed, 1);
        assert_eq!(progress[0].total, 2);
        assert_eq!(progress[0].target_id, plan.targets[0].id);
        assert_eq!(progress[0].status, ExecutionTargetStatus::Succeeded);
        assert_eq!(progress[1].completed, 2);
        assert_eq!(progress[1].total, 2);
        assert_eq!(progress[1].target_id, plan.targets[1].id);
        assert_eq!(progress[1].status, ExecutionTargetStatus::Succeeded);
    }

    #[test]
    fn observed_execution_progress_reports_success_and_failure_outcomes() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let success_path = fixture.path().join("node_modules");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "node.node_modules",
                    CleanAction::MoveToTrash {
                        path: success_path.clone(),
                    },
                    Some(success_path),
                ),
                target(
                    "rust.target",
                    CleanAction::Command {
                        program: "cargo".to_string(),
                        args: vec!["clean".to_string()],
                        cwd: None,
                        irreversible: true,
                    },
                    None,
                ),
            ],
        };
        let executor = test_executor(
            RecordingCommandRunner::failing("cargo unavailable"),
            RecordingTrashRunner::default(),
        );
        let mut progress = Vec::new();

        let report = executor
            .run_plan_with_progress(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
                |event| progress.push(event),
            )
            .expect("execute returns partial-failure report");

        assert_eq!(report.succeeded, 1);
        assert_eq!(report.skipped, 0);
        assert_eq!(report.failed, 1);
        assert_eq!(progress.len(), 2);
        assert_eq!(progress[0].status, ExecutionTargetStatus::Succeeded);
        assert_eq!(progress[0].message, "completed");
        assert_eq!(progress[1].status, ExecutionTargetStatus::Failed);
        assert!(progress[1].message.contains("cargo unavailable"));
    }

    #[test]
    fn inspect_only_selection_is_rejected_before_audit_or_runner_calls() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "cargo.home.inspect",
                CleanAction::NoopInspectOnly,
                None,
            )],
        };
        let command_runner = RecordingCommandRunner::default();
        let trash_runner = RecordingTrashRunner::default();
        let executor = test_executor(command_runner.clone(), trash_runner.clone());

        let error = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    cancel: None,
                },
            )
            .expect_err("inspect-only target cannot be selected");

        assert!(
            error
                .to_string()
                .contains("has no executable cleanup action")
        );
        assert!(command_runner.requests().is_empty());
        assert!(trash_runner.paths().is_empty());
        assert!(!audit_path.exists());
    }

    #[test]
    fn target_containing_running_executable_is_skipped_before_command_runs() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let current_exe = std::env::current_exe().expect("current executable path");
        let current_exe_dir = current_exe.parent().expect("current executable has parent");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "test.self_clean",
                CleanAction::Command {
                    program: "cargo".to_string(),
                    args: vec!["clean".to_string()],
                    cwd: None,
                    irreversible: true,
                },
                Some(current_exe_dir.to_path_buf()),
            )],
        };
        let command_runner = RecordingCommandRunner::default();
        let executor = test_executor(command_runner.clone(), RecordingTrashRunner::default());
        let mut progress = Vec::new();

        let report = executor
            .run_plan_with_progress(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    cancel: None,
                },
                |event| progress.push(event),
            )
            .expect("self-clean target is skipped");

        assert_eq!(
            report.failed, 0,
            "self-clean should skip, not fail: {:?}",
            report.failures
        );
        assert_eq!(report.succeeded, 0);
        assert_eq!(report.skipped, 1);
        assert!(command_runner.requests().is_empty());
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].status, ExecutionTargetStatus::Skipped);
        assert!(progress[0].message.contains(SELF_CLEAN_SKIP_MESSAGE));

        let records = read_jsonl(&audit_path);
        assert_eq!(records[0]["status"], "skipped");
        assert!(
            records[0]["error"]
                .as_str()
                .expect("skip reason")
                .contains(SELF_CLEAN_SKIP_MESSAGE)
        );
    }

    #[test]
    fn authorization_denial_is_audited_before_later_target_dispatch() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let denied_path = fixture.path().join("denied").join("node_modules");
        let allowed_path = fixture.path().join("allowed").join("node_modules");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "node.node_modules",
                    CleanAction::MoveToTrash {
                        path: denied_path.clone(),
                    },
                    Some(denied_path.clone()),
                ),
                target(
                    "node.node_modules",
                    CleanAction::MoveToTrash {
                        path: allowed_path.clone(),
                    },
                    Some(allowed_path.clone()),
                ),
            ],
        };
        let command_runner = RecordingCommandRunner::default();
        let trash_runner = RecordingTrashRunner::default();
        let authorization_calls = Arc::new(AtomicUsize::new(0));
        let executor = test_executor(command_runner.clone(), trash_runner.clone())
            .with_safety_policy(
                SafetyPolicy::from_user_list(UserProtectionList::empty_in_memory_for_tests_only())
                    .with_home(None)
                    .with_current_exe_fn({
                        let authorization_calls = Arc::clone(&authorization_calls);
                        move || {
                            if authorization_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                                Err(std::io::Error::other("\x1b[31mauthorization denied\x07"))
                            } else {
                                std::env::current_exe()
                            }
                        }
                    }),
            );

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    cancel: None,
                },
            )
            .expect("authorization denial returns a partial-failure report");

        assert_eq!(report.attempted, 2);
        assert_eq!(report.failed, 1);
        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].target_id, plan.targets[0].id);
        assert!(!report.failures[0].message.as_bytes().contains(&0x1b));
        assert!(!report.failures[0].message.as_bytes().contains(&0x07));
        assert!(command_runner.requests().is_empty());
        assert_eq!(trash_runner.paths(), vec![allowed_path]);

        let records = read_jsonl(&audit_path);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0]["event"], "action_finished");
        assert_eq!(records[0]["target_id"], plan.targets[0].id.as_str());
        assert_eq!(records[0]["action_path"], denied_path.display().to_string());
        assert_eq!(records[0]["status"], "failed");
        let reason = records[0]["error"]
            .as_str()
            .expect("denial reason is recorded");
        assert!(reason.contains("authorization denied"));
        assert!(!reason.as_bytes().contains(&0x1b));
        assert!(!reason.as_bytes().contains(&0x07));
        assert_eq!(records[1]["event"], "action_started");
        assert_eq!(records[1]["target_id"], plan.targets[1].id.as_str());
    }

    #[test]
    fn explicit_selection_drives_execution() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let chosen_path = fixture.path().join("node_modules");
        let ignored_path = fixture.path().join(".pytest_cache");
        let mut plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "node.node_modules",
                    CleanAction::MoveToTrash {
                        path: chosen_path.clone(),
                    },
                    Some(chosen_path.clone()),
                ),
                target(
                    "python.pytest_cache",
                    CleanAction::MoveToTrash {
                        path: ignored_path.clone(),
                    },
                    Some(ignored_path),
                ),
            ],
        };
        plan.targets[0].selected_by_default = false;
        let trash_runner = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: vec![plan.targets[0].id.clone()],
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
            )
            .expect("execute succeeds");

        assert_eq!(report.selected, 1);
        assert_eq!(report.succeeded, 1);
        assert_eq!(trash_runner.paths(), vec![chosen_path]);
    }

    #[test]
    fn empty_selection_executes_nothing() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let cleanup_path = fixture.path().join("node_modules");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "node.node_modules",
                CleanAction::MoveToTrash {
                    path: cleanup_path.clone(),
                },
                Some(cleanup_path),
            )],
        };
        let trash_runner = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash_runner.clone());

        let dry_run = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: Vec::new(),
                    execute: false,
                    audit_log: None,
                    cancel: None,
                },
            )
            .expect("dry-run succeeds");
        assert_eq!(dry_run.selected, 0);

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: Vec::new(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
            )
            .expect("execute succeeds");

        assert_eq!(report.selected, 0);
        assert_eq!(report.attempted, 0);
        assert!(trash_runner.paths().is_empty());
    }

    #[test]
    fn once_ledger_rejects_a_duplicate_fingerprint_before_second_runner_call() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let cleanup_path = fixture.path().join("node_modules");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "node.node_modules",
                    CleanAction::MoveToTrash {
                        path: cleanup_path.clone(),
                    },
                    Some(cleanup_path.clone()),
                ),
                target(
                    "duplicate.node_modules",
                    CleanAction::MoveToTrash {
                        path: cleanup_path.clone(),
                    },
                    Some(cleanup_path.clone()),
                ),
            ],
        };
        let validated = test_validated_plan(&plan);
        let trash_runner = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash_runner.clone());

        let report = executor
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected: validated.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    cancel: None,
                },
            )
            .expect("duplicate is recorded as a partial failure");

        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.attempted, 1);
        assert_eq!(trash_runner.paths(), vec![cleanup_path]);
        assert!(
            report.failures[0]
                .message
                .contains("duplicate action fingerprint")
        );
        let records = read_jsonl(&audit_path);
        // first target: started+finished success; second: no started (blocked pre-dispatch)
        // duplicate failure is report-only without journal start
        assert!(
            records
                .iter()
                .filter(|r| r["event"] == "action_finished")
                .any(|r| r["status"] == "success")
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r["event"] == "action_started")
                .count(),
            1
        );
    }

    #[test]
    fn process_command_runner_sanitizes_control_chars_on_failure() {
        use crate::process::{sanitize_process_output, test_support::process_fixture_exe};

        let fixture = process_fixture_exe();

        let runner = ProcessCommandRunner::new();
        let error = runner
            .run_with_cancel(
                &CommandRequest {
                    program: fixture.to_string_lossy().into_owned(),
                    args: vec!["control-stderr".to_string()],
                    cwd: None,
                },
                &NoopCancelObserver,
            )
            .expect_err("control-stderr fixture exits nonzero");

        let message = error.to_string();
        assert!(
            !message.as_bytes().contains(&0x1b),
            "error message must not retain ESC control bytes: {message:?}"
        );
        assert!(
            !message.as_bytes().contains(&0x07),
            "error message must not retain BEL control bytes: {message:?}"
        );
        assert!(
            message.contains("exited with status"),
            "nonzero exit should stay typed in the message: {message}"
        );
        // Sanitizer contract shared with durable-audit consumers.
        let sample = sanitize_process_output(b"\x1b[31m\x07x", 64);
        assert!(!sample.as_bytes().contains(&0x1b));
    }

    #[test]
    fn cancel_is_passed_into_command_runner_for_in_flight_termination() {
        struct CancelAwareRunner {
            observer_polled: Rc<RefCell<bool>>,
        }
        impl CommandRunner for CancelAwareRunner {
            fn run_with_cancel(
                &self,
                _request: &CommandRequest,
                cancel: &dyn CancelObserver,
            ) -> Result<CommandOutcome> {
                // Prove the live observer is consulted during the command path
                // (ProcessCommandRunner forwards this into ProcessRunner).
                let _ = cancel.is_cancel_requested();
                *self.observer_polled.borrow_mut() = true;
                Ok(CommandOutcome {
                    code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
        }

        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let target_dir = fixture.path().join("target");
        fs::create_dir_all(&target_dir).expect("target dir");
        let manifest = fixture.path().join("Cargo.toml");
        fs::write(&manifest, "[package]\nname=\"t\"\nversion=\"0.1.0\"\n").expect("manifest");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "test.command",
                CleanAction::Command {
                    program: "tool".to_string(),
                    args: vec!["clean".to_string()],
                    cwd: None,
                    irreversible: true,
                },
                Some(target_dir),
            )],
        };
        let _ = manifest;
        let cancel = Arc::new(FlagCancelObserver::new());
        let polled = Rc::new(RefCell::new(false));
        let executor = test_executor(
            CancelAwareRunner {
                observer_polled: polled.clone(),
            },
            RecordingTrashRunner::default(),
        );
        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: Some(cancel),
                },
            )
            .expect("report");
        assert!(
            *polled.borrow(),
            "command runner must receive cancel observer mid-action; report={report:?}"
        );
        assert_eq!(report.succeeded, 1);
    }

    #[test]
    fn cancel_flag_stops_later_targets_before_dispatch() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let first = fixture.path().join("a");
        let second = fixture.path().join("b");
        fs::create_dir_all(&first).ok();
        fs::create_dir_all(&second).ok();
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "first",
                    CleanAction::MoveToTrash {
                        path: first.clone(),
                    },
                    Some(first.clone()),
                ),
                target(
                    "second",
                    CleanAction::MoveToTrash {
                        path: second.clone(),
                    },
                    Some(second.clone()),
                ),
            ],
        };
        let cancel = Arc::new(crate::process::FlagCancelObserver::new());
        cancel.request_cancel();
        let trash = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash.clone());
        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: Some(cancel),
                },
            )
            .expect("cancel produces report");
        assert_eq!(trash.paths().len(), 0, "no side effects after cancel");
        assert!(report.skipped >= 1 || report.failed >= 1);
    }

    #[test]
    fn started_sync_failure_blocks_side_effect() {
        struct FailSyncIo;
        impl JournalIo for FailSyncIo {
            fn write_line(&mut self, _line: &str) -> Result<()> {
                Ok(())
            }
            fn flush(&mut self) -> Result<()> {
                Ok(())
            }
            fn sync_data(&mut self) -> Result<()> {
                bail!("injected sync failure")
            }
        }

        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let cleanup_path = fixture.path().join("node_modules");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "node.node_modules",
                CleanAction::MoveToTrash {
                    path: cleanup_path.clone(),
                },
                Some(cleanup_path.clone()),
            )],
        };
        let trash = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash.clone())
            .with_journal_io(Box::new(FailSyncIo));
        let mut progress = Vec::new();

        let report = executor
            .run_plan_with_progress(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
                |event| progress.push(event),
            )
            .expect("audit failure is reported per target");

        assert!(
            trash.paths().is_empty(),
            "started audit must precede dispatch"
        );
        assert_eq!(report.attempted, 0);
        assert_eq!(report.failed, 1);
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].status, ExecutionTargetStatus::Failed);
        assert!(
            progress[0]
                .message
                .contains("audit-blocked before side effect")
        );
    }

    #[test]
    fn terminal_audit_failure_reports_unknown_after_side_effect() {
        struct FailTerminalIo {
            writes: usize,
        }
        impl JournalIo for FailTerminalIo {
            fn write_line(&mut self, _line: &str) -> Result<()> {
                self.writes += 1;
                if self.writes == 2 {
                    bail!("injected terminal write failure");
                }
                Ok(())
            }
            fn flush(&mut self) -> Result<()> {
                Ok(())
            }
            fn sync_data(&mut self) -> Result<()> {
                Ok(())
            }
        }

        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let cleanup_path = fixture.path().join("node_modules");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "node.node_modules",
                CleanAction::MoveToTrash {
                    path: cleanup_path.clone(),
                },
                Some(cleanup_path.clone()),
            )],
        };
        let trash = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash.clone())
            .with_journal_io(Box::new(FailTerminalIo { writes: 0 }));
        let mut progress = Vec::new();

        let report = executor
            .run_plan_with_progress(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
                |event| progress.push(event),
            )
            .expect("terminal audit failure is reported per target");

        assert_eq!(trash.paths(), vec![cleanup_path]);
        assert_eq!(report.attempted, 1);
        assert_eq!(report.succeeded, 0);
        assert_eq!(report.failed, 1);
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].status, ExecutionTargetStatus::Unknown);
        assert!(
            progress[0]
                .message
                .contains("injected terminal write failure")
        );
    }

    #[test]
    fn replay_marks_started_without_finished_as_unconfirmed() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        fs::write(
            &audit_path,
            concat!(
                r#"{"event":"action_started","timestamp_epoch_ms":1,"run_id":"r1","sequence":1,"plan_digest":"d","target_id":"t1","action":"move_to_trash","command":null,"action_path":"C:/x","estimated_bytes":1}"#,
                "\n"
            ),
        )
        .expect("write partial journal");
        let unconfirmed = replay_unconfirmed_starts(&audit_path).expect("replay");
        assert_eq!(unconfirmed, vec!["started_unconfirmed:t1".to_string()]);
    }

    #[test]
    fn unknown_ids_in_selection_are_ignored() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let cleanup_path = fixture.path().join("node_modules");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "node.node_modules",
                CleanAction::MoveToTrash {
                    path: cleanup_path.clone(),
                },
                Some(cleanup_path.clone()),
            )],
        };
        let trash_runner = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: vec![
                        TargetId::new("ghost.target:none"),
                        plan.targets[0].id.clone(),
                    ],
                    execute: true,
                    audit_log: Some(audit_path),
                    cancel: None,
                },
            )
            .expect("execute succeeds");

        assert_eq!(report.selected, 1);
        assert_eq!(report.succeeded, 1);
        assert_eq!(trash_runner.paths(), vec![cleanup_path]);
    }

    #[derive(Clone, Default)]
    struct RecordingCommandRunner {
        requests: Rc<RefCell<Vec<CommandRequest>>>,
        failure: Rc<RefCell<Option<String>>>,
    }

    impl RecordingCommandRunner {
        fn failing(message: &str) -> Self {
            Self {
                requests: Rc::new(RefCell::new(Vec::new())),
                failure: Rc::new(RefCell::new(Some(message.to_string()))),
            }
        }

        fn requests(&self) -> Vec<CommandRequest> {
            self.requests.borrow().clone()
        }
    }

    impl CommandRunner for RecordingCommandRunner {
        fn run_with_cancel(
            &self,
            request: &CommandRequest,
            cancel: &dyn CancelObserver,
        ) -> Result<CommandOutcome> {
            if cancel.is_cancel_requested() {
                return Err(anyhow!("canceled before command start"));
            }
            self.requests.borrow_mut().push(request.clone());
            if let Some(message) = self.failure.borrow().as_ref() {
                return Err(anyhow!(message.clone()));
            }
            Ok(CommandOutcome {
                code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }

    #[derive(Clone, Default)]
    struct RecordingTrashRunner {
        paths: Rc<RefCell<Vec<PathBuf>>>,
        failure: Rc<RefCell<Option<(PathBuf, String)>>>,
    }

    impl RecordingTrashRunner {
        fn failing_on(path: &Path, message: &str) -> Self {
            Self {
                paths: Rc::new(RefCell::new(Vec::new())),
                failure: Rc::new(RefCell::new(Some((
                    path.to_path_buf(),
                    message.to_string(),
                )))),
            }
        }

        fn paths(&self) -> Vec<PathBuf> {
            self.paths.borrow().clone()
        }
    }

    impl TrashRunner for RecordingTrashRunner {
        fn move_to_trash(&self, path: &Path) -> Result<()> {
            self.paths.borrow_mut().push(path.to_path_buf());
            if let Some((failure_path, message)) = self.failure.borrow().as_ref()
                && failure_path == path
            {
                return Err(anyhow!(message.clone()));
            }
            Ok(())
        }
    }

    fn target(rule_id: &str, action: CleanAction, path: Option<PathBuf>) -> CleanTarget {
        let root = path
            .as_ref()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(|| {
                std::env::temp_dir().join(format!(
                    "devsweep-executor-{}-{rule_id}",
                    std::process::id()
                ))
            });
        // Ensure the project root exists so live revalidation can open markers.
        let _ = fs::create_dir_all(&root);
        let marker = root.join("package.json");
        if !marker.exists() {
            let _ = fs::write(&marker, "{}");
        }
        if let Some(path) = path.as_ref() {
            let _ = fs::create_dir_all(path);
        }
        CleanTarget {
            id: TargetId::new(format!("{rule_id}:{}", path_display(path.as_deref()))),
            scope: Scope::Project { root: root.clone() },
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::BuildArtifacts,
            path: path.clone(),
            estimated_bytes: 42,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default: true,
            evidence: vec![
                Evidence::MarkerFile { path: marker },
                Evidence::RuleMatched {
                    rule_id: rule_id.to_string(),
                },
            ],
            action,
        }
    }

    fn test_executor<C: CommandRunner, T: TrashRunner>(
        command_runner: C,
        trash_runner: T,
    ) -> Executor<C, T> {
        let home = std::env::temp_dir().join("devsweep-test-home");
        let _ = fs::create_dir_all(&home);
        Executor::new(command_runner, trash_runner).with_safety_policy(
            SafetyPolicy::from_user_list(UserProtectionList::empty_in_memory_for_tests_only())
                .with_home(Some(home)),
        )
    }

    fn path_display(path: Option<&Path>) -> String {
        path.map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    }

    fn read_jsonl(path: &Path) -> Vec<Value> {
        fs::read_to_string(path)
            .expect("audit log")
            .lines()
            .map(|line| serde_json::from_str(line).expect("audit record"))
            .collect()
    }
}
