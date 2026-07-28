use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;

use crate::{
    model::{CleanAction, CleanTarget, TargetId},
    path_safety::target_contains_current_exe,
    plan_validation::ValidatedPlan,
    process_runner::{
        CancelObserver, CwdPolicy, DEFAULT_EXECUTOR_COMMAND_TIMEOUT, NoopCancelObserver,
        ProcessRequest, ProcessRunner, ProcessStatus, sanitize_process_output,
    },
};

#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub execute: bool,
    pub audit_log: Option<PathBuf>,
    pub selected: Vec<TargetId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReport {
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
    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionFailure {
    pub target_id: TargetId,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTargetStatus {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionProgress {
    pub completed: usize,
    pub total: usize,
    pub target_id: TargetId,
    pub status: ExecutionTargetStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutcome>;
}

pub trait TrashRunner {
    fn move_to_trash(&self, path: &Path) -> Result<()>;
}

#[derive(Debug)]
pub struct ProcessCommandRunner {
    runner: ProcessRunner,
}

impl Default for ProcessCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessCommandRunner {
    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
        }
    }
}

impl CommandRunner for ProcessCommandRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutcome> {
        self.run_with_cancel(request, &NoopCancelObserver)
    }
}

impl ProcessCommandRunner {
    fn run_with_cancel(
        &self,
        request: &CommandRequest,
        cancel: &dyn CancelObserver,
    ) -> Result<CommandOutcome> {
        let cwd = match &request.cwd {
            Some(path) => CwdPolicy::Explicit {
                path: path.clone(),
                reason: "executor command cwd from validated plan".to_string(),
            },
            None => CwdPolicy::Neutral,
        };

        let process_request = ProcessRequest {
            program: OsString::from(&request.program),
            args: request.args.iter().map(OsString::from).collect(),
            cwd,
            timeout: Some(DEFAULT_EXECUTOR_COMMAND_TIMEOUT),
            job_deadline: None,
            cancel,
        };

        let result = self.runner.run(&process_request);
        let stdout = String::from_utf8_lossy(&result.output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&result.output.stderr).into_owned();
        let diagnostic = sanitize_process_output(&result.output.stderr, EXECUTOR_DIAGNOSTIC_CAP);
        let display = command_display(request);

        match result.status {
            ProcessStatus::Success => Ok(CommandOutcome {
                code: Some(0),
                stdout,
                stderr,
            }),
            ProcessStatus::NotFound => Err(anyhow!("failed to run {display}: program not found")),
            ProcessStatus::Timeout => Err(anyhow!(
                "{display} timed out after {:?}: {}",
                DEFAULT_EXECUTOR_COMMAND_TIMEOUT,
                diagnostic.trim()
            )),
            ProcessStatus::Canceled => Err(anyhow!("{display} canceled")),
            ProcessStatus::InvalidOutput => Err(anyhow!(
                "failed to run {display}: invalid process output or spawn failure"
            )),
            ProcessStatus::Exit { code } => Err(anyhow!(
                "{display} exited with status {code:?}: {}",
                diagnostic.trim()
            )),
        }
    }
}

const EXECUTOR_DIAGNOSTIC_CAP: usize = 4 * 1024;

#[derive(Debug, Default)]
pub struct SystemTrashRunner;

impl TrashRunner for SystemTrashRunner {
    fn move_to_trash(&self, path: &Path) -> Result<()> {
        trash::delete(path).with_context(|| format!("failed to move {} to trash", path.display()))
    }
}

#[derive(Debug)]
pub struct Executor<C = ProcessCommandRunner, T = SystemTrashRunner> {
    command_runner: C,
    trash_runner: T,
}

impl Default for Executor<ProcessCommandRunner, SystemTrashRunner> {
    fn default() -> Self {
        Self::new(ProcessCommandRunner::new(), SystemTrashRunner)
    }
}

impl<C, T> Executor<C, T> {
    pub fn new(command_runner: C, trash_runner: T) -> Self {
        Self {
            command_runner,
            trash_runner,
        }
    }
}

impl<C, T> Executor<C, T>
where
    C: CommandRunner,
    T: TrashRunner,
{
    pub fn run_plan(
        &self,
        plan: &ValidatedPlan,
        request: ExecutionRequest,
    ) -> Result<ExecutionReport> {
        self.run_plan_with_progress(plan, request, |_| {})
    }

    pub fn run_plan_with_progress<F>(
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

        let audit_path = request
            .audit_log
            .unwrap_or_else(|| PathBuf::from("devsweep-audit.jsonl"));
        let mut audit = AuditLog::open(&audit_path)?;
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
        for validated_target in selected_targets {
            let target = validated_target.target();
            let started = Instant::now();
            let dispatch_attempted =
                executed_fingerprints.insert(validated_target.fingerprint().clone());
            let outcome = if dispatch_attempted {
                self.execute_target(target)
            } else {
                Err(anyhow!(
                    "duplicate action fingerprint rejected before execution: {}",
                    validated_target.fingerprint().as_str()
                ))
            };
            let duration_ms = started.elapsed().as_millis();

            let (progress_status, progress_message) = match outcome {
                Ok(ActionStatus::Success { command }) => {
                    report.attempted += 1;
                    report.succeeded += 1;
                    audit.write(&AuditRecord::success(target, command, duration_ms))?;
                    (ExecutionTargetStatus::Succeeded, "completed".to_string())
                }
                Ok(ActionStatus::Skipped { message }) => {
                    report.skipped += 1;
                    audit.write(&AuditRecord::skipped(target, message.clone(), duration_ms))?;
                    (
                        ExecutionTargetStatus::Skipped,
                        format!("skipped: {message}"),
                    )
                }
                Err(error) => {
                    if dispatch_attempted {
                        report.attempted += 1;
                    }
                    report.failed += 1;
                    let message = error.to_string();
                    report.failures.push(ActionFailure {
                        target_id: target.id.clone(),
                        message: message.clone(),
                    });
                    audit.write(&AuditRecord::failed(target, message.clone(), duration_ms))?;
                    (ExecutionTargetStatus::Failed, message)
                }
            };

            on_progress(ExecutionProgress {
                completed: report.succeeded + report.failed + report.skipped,
                total,
                target_id: target.id.clone(),
                status: progress_status,
                message: progress_message,
            });
        }

        audit.flush()?;
        Ok(report)
    }

    fn execute_target(&self, target: &CleanTarget) -> Result<ActionStatus> {
        if target_contains_current_exe(target.path.as_deref()) {
            return Ok(ActionStatus::Skipped {
                message: SELF_CLEAN_SKIP_MESSAGE.to_string(),
            });
        }

        match &target.action {
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
                self.command_runner.run(&request)?;
                Ok(ActionStatus::Success {
                    command: Some(command_argv(&request)),
                })
            }
            CleanAction::MoveToTrash { path } => {
                self.trash_runner.move_to_trash(path)?;
                Ok(ActionStatus::Success { command: None })
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActionStatus {
    Success { command: Option<Vec<String>> },
    Skipped { message: String },
}

const SELF_CLEAN_SKIP_MESSAGE: &str = "target contains the running devsweep executable";

#[derive(Debug)]
struct AuditLog {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl AuditLog {
    fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create audit log directory {}", parent.display())
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open audit log {}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            writer: BufWriter::new(file),
        })
    }

    fn write(&mut self, record: &AuditRecord) -> Result<()> {
        serde_json::to_writer(&mut self.writer, record)
            .with_context(|| format!("failed to write audit log {}", self.path.display()))?;
        self.writer
            .write_all(b"\n")
            .with_context(|| format!("failed to write audit log {}", self.path.display()))?;
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.writer
            .flush()
            .with_context(|| format!("failed to flush audit log {}", self.path.display()))
    }
}

#[derive(Debug, Serialize)]
struct AuditRecord {
    timestamp_epoch_ms: u128,
    target_id: String,
    action: &'static str,
    command: Option<Vec<String>>,
    estimated_bytes: u64,
    status: &'static str,
    duration_ms: u128,
    error: Option<String>,
    partial: bool,
}

impl AuditRecord {
    fn success(target: &CleanTarget, command: Option<Vec<String>>, duration_ms: u128) -> Self {
        Self::new(target, command, "success", duration_ms, None)
    }

    fn skipped(target: &CleanTarget, message: String, duration_ms: u128) -> Self {
        Self::new(target, None, "skipped", duration_ms, Some(message))
    }

    fn failed(target: &CleanTarget, message: String, duration_ms: u128) -> Self {
        Self::new(
            target,
            command_from_action(&target.action),
            "failed",
            duration_ms,
            Some(message),
        )
    }

    fn new(
        target: &CleanTarget,
        command: Option<Vec<String>>,
        status: &'static str,
        duration_ms: u128,
        error: Option<String>,
    ) -> Self {
        Self {
            timestamp_epoch_ms: unix_epoch_ms(),
            target_id: target.id.as_str().to_string(),
            action: action_name(&target.action),
            command,
            estimated_bytes: target.estimated_bytes,
            status,
            duration_ms,
            partial: status == "failed",
            error,
        }
    }
}

fn command_from_action(action: &CleanAction) -> Option<Vec<String>> {
    match action {
        CleanAction::Command {
            program,
            args,
            cwd: _,
            irreversible: _,
        } => {
            let request = CommandRequest {
                program: program.clone(),
                args: args.clone(),
                cwd: None,
            };
            Some(command_argv(&request))
        }
        CleanAction::MoveToTrash { .. }
        | CleanAction::DeletePermanently { .. }
        | CleanAction::NoopInspectOnly => None,
    }
}

fn action_name(action: &CleanAction) -> &'static str {
    match action {
        CleanAction::Command { .. } => "command",
        CleanAction::MoveToTrash { .. } => "move_to_trash",
        CleanAction::DeletePermanently { .. } => "delete_permanently",
        CleanAction::NoopInspectOnly => "noop_inspect_only",
    }
}

fn command_argv(request: &CommandRequest) -> Vec<String> {
    let mut argv = Vec::with_capacity(request.args.len() + 1);
    argv.push(request.program.clone());
    argv.extend(request.args.clone());
    argv
}

fn command_display(request: &CommandRequest) -> String {
    command_argv(request).join(" ")
}

fn unix_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        fs,
        path::{Path, PathBuf},
        rc::Rc,
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
        let executor = Executor::new(command_runner.clone(), trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: false,
                    audit_log: None,
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
                "rust.target",
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
        let executor = Executor::new(command_runner.clone(), RecordingTrashRunner::default());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
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
        assert_eq!(records[0]["status"], "success");
        assert_eq!(records[0]["command"][0], "cargo");
        assert_eq!(records[0]["command"][3], manifest.display().to_string());
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
        let executor = Executor::new(RecordingCommandRunner::default(), trash_runner.clone());

        executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
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
        let executor = Executor::new(command_runner, trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                },
            )
            .expect("job returns failure report instead of aborting");

        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failed, 1);
        assert!(report.has_failures());
        assert_eq!(trash_runner.paths().len(), 1);

        let records = read_jsonl(&audit_path);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["status"], "success");
        assert_eq!(records[1]["status"], "failed");
        assert_eq!(records[1]["partial"], true);
        assert!(
            records[1]["error"]
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
        let executor = Executor::new(RecordingCommandRunner::default(), trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
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
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["status"], "failed");
        assert_eq!(records[0]["partial"], true);
        assert_eq!(records[1]["status"], "success");
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
        let executor = Executor::new(
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
        let executor = Executor::new(
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
        let executor = Executor::new(
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
        let executor = Executor::new(command_runner.clone(), trash_runner.clone());

        let error = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
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
                "rust.target",
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
        let executor = Executor::new(command_runner.clone(), RecordingTrashRunner::default());
        let mut progress = Vec::new();

        let report = executor
            .run_plan_with_progress(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                },
                |event| progress.push(event),
            )
            .expect("self-clean target is skipped");

        assert_eq!(report.succeeded, 0);
        assert_eq!(report.failed, 0);
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
        let executor = Executor::new(RecordingCommandRunner::default(), trash_runner.clone());

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: vec![plan.targets[0].id.clone()],
                    execute: true,
                    audit_log: Some(audit_path),
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
        let executor = Executor::new(RecordingCommandRunner::default(), trash_runner.clone());

        let dry_run = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: Vec::new(),
                    execute: false,
                    audit_log: None,
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
        let executor = Executor::new(RecordingCommandRunner::default(), trash_runner.clone());

        let report = executor
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected: validated.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
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
        assert_eq!(records.len(), 2);
        assert_eq!(records[1]["status"], "failed");
    }

    #[test]
    fn process_command_runner_sanitizes_control_chars_on_failure() {
        use crate::process_runner::{sanitize_process_output, test_support::process_fixture_exe};

        let fixture = process_fixture_exe();

        let runner = ProcessCommandRunner::new();
        let error = runner
            .run(&CommandRequest {
                program: fixture.to_string_lossy().into_owned(),
                args: vec!["control-stderr".to_string()],
                cwd: None,
            })
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
        let executor = Executor::new(RecordingCommandRunner::default(), trash_runner.clone());

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
        fn run(&self, request: &CommandRequest) -> Result<CommandOutcome> {
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
        CleanTarget {
            id: TargetId::new(format!("{rule_id}:{}", path_display(path.as_deref()))),
            scope: Scope::Project {
                root: PathBuf::from("C:/workspace/app"),
            },
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::BuildArtifacts,
            path,
            estimated_bytes: 42,
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: rule_id.to_string(),
            }],
            action,
        }
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
