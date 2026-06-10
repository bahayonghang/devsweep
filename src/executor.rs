use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;

use crate::model::{CleanAction, CleanTarget, CleanupPlan, TargetId};

#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub execute: bool,
    pub allow_permanent_delete: bool,
    pub audit_log: Option<PathBuf>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionProgress {
    pub completed: usize,
    pub total: usize,
    pub target_id: TargetId,
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

#[derive(Debug, Default)]
pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutcome> {
        let mut command = Command::new(&request.program);
        command.args(&request.args);
        if let Some(cwd) = &request.cwd {
            command.current_dir(cwd);
        }

        let output = command
            .output()
            .with_context(|| format!("failed to run {}", command_display(request)))?;
        let outcome = CommandOutcome {
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };

        if output.status.success() {
            Ok(outcome)
        } else {
            Err(anyhow!(
                "{} exited with status {:?}: {}",
                command_display(request),
                outcome.code,
                outcome.stderr.trim()
            ))
        }
    }
}

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
        Self::new(ProcessCommandRunner, SystemTrashRunner)
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
        plan: &CleanupPlan,
        request: ExecutionRequest,
    ) -> Result<ExecutionReport> {
        self.run_plan_with_progress(plan, request, |_| {})
    }

    pub fn run_plan_with_progress<F>(
        &self,
        plan: &CleanupPlan,
        request: ExecutionRequest,
        mut on_progress: F,
    ) -> Result<ExecutionReport>
    where
        F: FnMut(ExecutionProgress),
    {
        let selected_targets: Vec<&CleanTarget> = plan
            .targets
            .iter()
            .filter(|target| target.selected_by_default)
            .collect();

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
        for target in selected_targets {
            let started = Instant::now();
            let outcome = self.execute_target(target, request.allow_permanent_delete);
            let duration_ms = started.elapsed().as_millis();
            let progress_message;

            match outcome {
                Ok(ActionStatus::Success { command }) => {
                    report.attempted += 1;
                    report.succeeded += 1;
                    audit.write(&AuditRecord::success(target, command, duration_ms))?;
                    progress_message = "completed".to_string();
                }
                Ok(ActionStatus::Skipped { message }) => {
                    report.skipped += 1;
                    progress_message = format!("skipped: {message}");
                    audit.write(&AuditRecord::skipped(target, message, duration_ms))?;
                }
                Err(error) => {
                    report.attempted += 1;
                    report.failed += 1;
                    let message = error.to_string();
                    report.failures.push(ActionFailure {
                        target_id: target.id.clone(),
                        message: message.clone(),
                    });
                    audit.write(&AuditRecord::failed(target, message, duration_ms))?;
                    progress_message = "failed".to_string();
                }
            }

            on_progress(ExecutionProgress {
                completed: report.succeeded + report.failed + report.skipped,
                total,
                target_id: target.id.clone(),
                message: progress_message,
            });
        }

        audit.flush()?;
        Ok(report)
    }

    fn execute_target(
        &self,
        target: &CleanTarget,
        _allow_permanent_delete: bool,
    ) -> Result<ActionStatus> {
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
        CleanAction, CleanTarget, Ecosystem, Evidence, RiskLevel, Scope, TargetKind,
    };

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
                &plan,
                ExecutionRequest {
                    execute: false,
                    allow_permanent_delete: false,
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
                &plan,
                ExecutionRequest {
                    execute: true,
                    allow_permanent_delete: false,
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
                &plan,
                ExecutionRequest {
                    execute: true,
                    allow_permanent_delete: false,
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
                &plan,
                ExecutionRequest {
                    execute: true,
                    allow_permanent_delete: false,
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
                &plan,
                ExecutionRequest {
                    execute: true,
                    allow_permanent_delete: false,
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
    fn permanent_delete_is_disabled_even_when_flag_is_present() {
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

        let report = executor
            .run_plan(
                &plan,
                ExecutionRequest {
                    execute: true,
                    allow_permanent_delete: true,
                    audit_log: Some(audit_path),
                },
            )
            .expect("job records disabled permanent delete");

        assert_eq!(report.failed, 1);
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
                &plan,
                ExecutionRequest {
                    execute: true,
                    allow_permanent_delete: false,
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
        assert_eq!(progress[1].completed, 2);
        assert_eq!(progress[1].total, 2);
        assert_eq!(progress[1].target_id, plan.targets[1].id);
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
