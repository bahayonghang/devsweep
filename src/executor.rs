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
    plan_validation::{ValidatedPlan, ValidatedTarget},
    process_runner::{
        CancelObserver, CwdPolicy, DEFAULT_EXECUTOR_COMMAND_TIMEOUT, NoopCancelObserver,
        ProcessRequest, ProcessRunner, ProcessStatus, sanitize_process_output,
    },
    safety::{
        AuthorizationContext, AuthorizedAction, ProtectionCategory, SELF_CLEAN_SKIP_MESSAGE,
        SafetyPolicy,
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
    /// Side effect may have run but durable terminal audit failed.
    Unknown,
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

pub struct Executor<C = ProcessCommandRunner, T = SystemTrashRunner> {
    command_runner: C,
    trash_runner: T,
    safety: SafetyPolicy,
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
            safety: SafetyPolicy::from_user_list(
                crate::safety::UserProtectionList::load().unwrap_or_else(|_| {
                    crate::safety::UserProtectionList::empty_in_memory_for_tests_only()
                }),
            ),
        }
    }

    pub fn with_safety_policy(mut self, safety: SafetyPolicy) -> Self {
        self.safety = safety;
        self
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

        let audit_path = match request.audit_log {
            Some(path) => path,
            None => default_audit_log_path()?,
        };
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
        for validated_target in selected_targets {
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
                    let message = error.to_string();
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

            let outcome = self.dispatch_authorized(&authorized, target);
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
                let outcome = self.command_runner.run(&request)?;
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

/// Fault-injectable durable journal backend.
trait JournalIo {
    fn write_line(&mut self, line: &str) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
    fn sync_data(&mut self) -> Result<()>;
}

struct FileJournalIo {
    path: PathBuf,
    file: File,
    writer: BufWriter<File>,
}

impl FileJournalIo {
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
            .read(true)
            .open(path)
            .with_context(|| format!("failed to open audit log {}", path.display()))?;
        // Separate handle for sync_data after buffered writes.
        let sync_file = OpenOptions::new()
            .append(true)
            .open(path)
            .with_context(|| format!("failed to reopen audit log {}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            file: sync_file,
            writer: BufWriter::new(file),
        })
    }
}

impl JournalIo for FileJournalIo {
    fn write_line(&mut self, line: &str) -> Result<()> {
        self.writer
            .write_all(line.as_bytes())
            .and_then(|_| self.writer.write_all(b"\n"))
            .with_context(|| format!("failed to write audit log {}", self.path.display()))
    }

    fn flush(&mut self) -> Result<()> {
        self.writer
            .flush()
            .with_context(|| format!("failed to flush audit log {}", self.path.display()))
    }

    fn sync_data(&mut self) -> Result<()> {
        self.file
            .sync_data()
            .with_context(|| format!("failed to sync audit log {}", self.path.display()))
    }
}

struct AuditJournal {
    path: PathBuf,
    run_id: String,
    sequence: u64,
    io: Box<dyn JournalIo>,
}

impl AuditJournal {
    fn open(path: &Path, _plan_digest: String) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            run_id: format!("run-{}", unix_epoch_ms()),
            sequence: 0,
            io: Box::new(FileJournalIo::open(path)?),
        })
    }

    #[cfg(test)]
    fn with_io(path: PathBuf, io: Box<dyn JournalIo>) -> Self {
        Self {
            path,
            run_id: "run-test".to_string(),
            sequence: 0,
            io,
        }
    }

    fn run_id(&self) -> &str {
        &self.run_id
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    fn write_started_durable(&mut self, event: &JournalEvent) -> Result<()> {
        self.write_event(event)?;
        self.io.flush()?;
        self.io.sync_data()?;
        Ok(())
    }

    fn write_terminal(&mut self, event: JournalEvent) -> Result<()> {
        self.write_event(&event)?;
        self.io.flush()?;
        // Terminal sync is best-effort; write+flush success is enough to avoid
        // Unknown, but callers may still observe degraded sync separately.
        let _ = self.io.sync_data();
        Ok(())
    }

    fn write_event(&mut self, event: &JournalEvent) -> Result<()> {
        let line = serde_json::to_string(event).with_context(|| {
            format!(
                "failed to serialize audit event for {}",
                self.path.display()
            )
        })?;
        self.io.write_line(&line)
    }

    fn flush(&mut self) -> Result<()> {
        self.io.flush()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "event", rename_all = "snake_case")]
enum JournalEvent {
    ActionStarted {
        timestamp_epoch_ms: u128,
        run_id: String,
        sequence: u64,
        plan_digest: String,
        target_id: String,
        action: String,
        command: Option<Vec<String>>,
        action_path: Option<String>,
        estimated_bytes: u64,
    },
    ActionFinished {
        timestamp_epoch_ms: u128,
        run_id: String,
        sequence: u64,
        plan_digest: String,
        target_id: String,
        action: String,
        status: String,
        command: Option<Vec<String>>,
        action_path: Option<String>,
        exit_code: Option<i32>,
        estimated_bytes: u64,
        duration_ms: u128,
        error: Option<String>,
    },
}

impl JournalEvent {
    fn started(
        run_id: &str,
        sequence: u64,
        plan_digest: &str,
        target: &CleanTarget,
        authorized: &AuthorizedAction,
    ) -> Self {
        Self::ActionStarted {
            timestamp_epoch_ms: unix_epoch_ms(),
            run_id: run_id.to_string(),
            sequence,
            plan_digest: plan_digest.to_string(),
            target_id: target.id.as_str().to_string(),
            action: action_name(&target.action).to_string(),
            command: command_from_action(&target.action),
            action_path: action_path_from_authorized(authorized)
                .map(|path| path.display().to_string()),
            estimated_bytes: target.estimated_bytes,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn finished(
        run_id: &str,
        sequence: u64,
        plan_digest: &str,
        target: &CleanTarget,
        status: &str,
        command: Option<Vec<String>>,
        exit_code: Option<i32>,
        action_path: Option<PathBuf>,
        duration_ms: u128,
        error: Option<String>,
    ) -> Self {
        Self::ActionFinished {
            timestamp_epoch_ms: unix_epoch_ms(),
            run_id: run_id.to_string(),
            sequence,
            plan_digest: plan_digest.to_string(),
            target_id: target.id.as_str().to_string(),
            action: action_name(&target.action).to_string(),
            status: status.to_string(),
            command,
            action_path: action_path.map(|path| path.display().to_string()),
            exit_code,
            estimated_bytes: target.estimated_bytes,
            duration_ms,
            error: error.map(|message| sanitize_process_output(message.as_bytes(), 4_096)),
        }
    }

    fn skipped(
        run_id: &str,
        sequence: u64,
        plan_digest: &str,
        target: &CleanTarget,
        message: String,
        duration_ms: u128,
    ) -> Self {
        Self::finished(
            run_id,
            sequence,
            plan_digest,
            target,
            "skipped",
            None,
            None,
            target.path.clone(),
            duration_ms,
            Some(message),
        )
    }
}

/// Replay API: find started events that never received a terminal record.
pub fn replay_unconfirmed_starts(path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read audit log {}", path.display()))?;
    let mut started = HashSet::new();
    let mut finished = HashSet::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let event = value.get("event").and_then(|v| v.as_str()).unwrap_or("");
        let key = format!(
            "{}:{}",
            value.get("run_id").and_then(|v| v.as_str()).unwrap_or(""),
            value.get("sequence").and_then(|v| v.as_u64()).unwrap_or(0)
        );
        match event {
            "action_started" => {
                started.insert(key);
            }
            "action_finished" => {
                // finished sequences are distinct; pair by prior started target+run
                if let Some(target) = value.get("target_id").and_then(|v| v.as_str()) {
                    finished.insert(format!(
                        "{}:{}",
                        value.get("run_id").and_then(|v| v.as_str()).unwrap_or(""),
                        target
                    ));
                }
            }
            _ => {}
        }
    }

    let mut unconfirmed = Vec::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("event").and_then(|v| v.as_str()) != Some("action_started") {
            continue;
        }
        let run_id = value.get("run_id").and_then(|v| v.as_str()).unwrap_or("");
        let target = value
            .get("target_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let key = format!("{run_id}:{target}");
        if !finished.contains(&key) {
            unconfirmed.push(format!("started_unconfirmed:{target}"));
        }
    }
    let _ = started;
    Ok(unconfirmed)
}

pub fn default_audit_log_path() -> Result<PathBuf> {
    let dir = crate::safety::UserProtectionList::config_path()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create audit directory {}", dir.display()))?;
    Ok(dir.join("audit.jsonl"))
}

fn action_path_from_authorized(authorized: &AuthorizedAction) -> Option<PathBuf> {
    authorized
        .trash_path()
        .map(|path| path.to_path_buf())
        .or_else(|| match authorized.action() {
            CleanAction::Command { program, cwd, .. } => {
                cwd.clone().or_else(|| Some(PathBuf::from(program)))
            }
            CleanAction::MoveToTrash { path } => Some(path.clone()),
            _ => None,
        })
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
        let executor = test_executor(command_runner.clone(), trash_runner.clone());

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
        let executor = test_executor(RecordingCommandRunner::default(), trash_runner.clone());

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
    fn started_sync_failure_blocks_side_effect() {
        struct FailSyncIo {
            writes: usize,
        }
        impl JournalIo for FailSyncIo {
            fn write_line(&mut self, _line: &str) -> Result<()> {
                self.writes += 1;
                Ok(())
            }
            fn flush(&mut self) -> Result<()> {
                Ok(())
            }
            fn sync_data(&mut self) -> Result<()> {
                bail!("injected sync failure")
            }
        }

        let mut journal = AuditJournal::with_io(
            PathBuf::from("memory-audit.jsonl"),
            Box::new(FailSyncIo { writes: 0 }),
        );
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target(
                "node.node_modules",
                CleanAction::MoveToTrash {
                    path: PathBuf::from("C:/tmp/node_modules"),
                },
                Some(PathBuf::from("C:/tmp/node_modules")),
            )],
        };
        let target = &plan.targets[0];
        // Build a minimal started event without full authorize.
        let event = JournalEvent::ActionStarted {
            timestamp_epoch_ms: 1,
            run_id: "run-test".into(),
            sequence: 1,
            plan_digest: "digest".into(),
            target_id: target.id.as_str().into(),
            action: "move_to_trash".into(),
            command: None,
            action_path: Some("C:/tmp/node_modules".into()),
            estimated_bytes: 1,
        };
        let err = journal
            .write_started_durable(&event)
            .expect_err("sync failure blocks");
        assert!(err.to_string().contains("injected sync failure"));
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
        let root = path
            .as_ref()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("C:/workspace/app"));
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
            SafetyPolicy::from_user_list(
                crate::safety::UserProtectionList::empty_in_memory_for_tests_only(),
            )
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
