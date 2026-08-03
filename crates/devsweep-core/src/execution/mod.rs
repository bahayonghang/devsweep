use std::{collections::HashSet, fmt, path::PathBuf, sync::Arc, time::Instant};

#[cfg(test)]
use std::cell::RefCell;

use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};

mod audit;
mod command;
mod safety;

use audit::default_audit_log_path;
#[cfg(test)]
use audit::replay_unconfirmed_starts;
pub use command::{
    CommandOutcome, CommandRequest, CommandRunner, ProcessCommandRunner, SystemTrashRunner,
    TrashRunner,
};
pub use safety::UserProtectionList;
use safety::{
    AuthorizationContext, AuthorizedAction, ProtectionCategory, SELF_CLEAN_SKIP_MESSAGE,
    SafetyPolicy,
};

#[cfg(test)]
use audit::JournalIo;
use audit::{AuditJournal, JournalEvent, action_path_from_authorized, command_from_action};
use command::command_argv;

use crate::{
    model::{CleanAction, CleanTarget, ScanTotals, TargetId},
    plan::{ValidatedPlan, ValidatedTarget},
    process::{CancelObserver, FlagCancelObserver, NoopCancelObserver, sanitize_process_output},
};

const EXECUTOR_DIAGNOSTIC_CAP: usize = 4 * 1024;

#[derive(Debug, Clone)]
/// Explicit selection and mode for one validated-plan execution.
pub struct ExecutionRequest {
    /// Whether to perform side effects instead of a dry run.
    pub execute: bool,
    /// Optional audit JSONL path used only during execution.
    pub audit_log: Option<PathBuf>,
    /// Exact target identifiers selected by the caller.
    pub selected: Vec<TargetId>,
    /// Confirmation digest to verify before execution. Legacy CLI/TUI callers may omit it.
    pub expected_digest: Option<ConfirmationDigest>,
    /// Cooperative cancel flag shared with the UI/runtime.
    pub cancel: Option<Arc<crate::process::FlagCancelObserver>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
/// Stable SHA-256 identity of a validated manifest and canonical target selection.
pub struct ConfirmationDigest(String);

impl ConfirmationDigest {
    /// Wraps a digest received from a serialized boundary for later comparison.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    fn from_canonical(value: String) -> Self {
        Self(value)
    }

    /// Returns the lowercase hexadecimal digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConfirmationDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case", deny_unknown_fields)]
/// Caller-correctable execution request error.
pub enum ExecutionError {
    /// A selected target is not present in the validated manifest.
    UnknownTarget {
        /// Unknown target identifier.
        target_id: TargetId,
    },
    /// An inspect-only target was selected for cleanup.
    InspectOnlyTarget {
        /// Inspect-only target identifier.
        target_id: TargetId,
    },
    /// The supplied confirmation no longer matches the manifest and selection.
    StaleConfirmation {
        /// Digest supplied by the caller.
        expected_digest: ConfirmationDigest,
        /// Digest calculated from the current validated request.
        actual_digest: ConfirmationDigest,
    },
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTarget { target_id } => {
                write!(formatter, "unknown cleanup target: {}", target_id.as_str())
            }
            Self::InspectOnlyTarget { target_id } => write!(
                formatter,
                "target {} is inspect-only and cannot be executed",
                target_id.as_str()
            ),
            Self::StaleConfirmation { .. } => {
                formatter.write_str("cleanup confirmation digest is stale")
            }
        }
    }
}

impl std::error::Error for ExecutionError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Public projection of a trusted cleanup action without command or path details.
pub enum ActionKind {
    /// Bounded command action.
    Command {
        /// Whether the command cannot be reversed.
        irreversible: bool,
    },
    /// Move an exact validated path to the operating-system trash.
    MoveToTrash,
    /// Inspect-only target with no cleanup authority.
    InspectOnly,
    /// Disabled permanent-delete deny case.
    PermanentDelete,
}

impl ActionKind {
    fn from_action(action: &CleanAction) -> Self {
        match action {
            CleanAction::Command { irreversible, .. } => Self::Command {
                irreversible: *irreversible,
            },
            CleanAction::MoveToTrash { .. } => Self::MoveToTrash,
            CleanAction::NoopInspectOnly => Self::InspectOnly,
            CleanAction::DeletePermanently { .. } => Self::PermanentDelete,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Confidence-preserving recoverable-capacity estimate for one target.
pub enum CapacityEstimate {
    /// Complete byte estimate.
    Verified {
        /// Estimated bytes.
        bytes: u64,
    },
    /// Incomplete lower-bound estimate.
    Partial {
        /// Observed lower-bound bytes.
        lower_bound_bytes: u64,
    },
    /// No trustworthy byte estimate is available.
    Unknown,
}

impl CapacityEstimate {
    fn from_target(target: &CleanTarget) -> Self {
        if target.size_complete {
            Self::Verified {
                bytes: target.estimated_bytes,
            }
        } else if target.estimated_bytes == 0 {
            Self::Unknown
        } else {
            Self::Partial {
                lower_bound_bytes: target.estimated_bytes,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Terminal result for one selected target.
pub enum OutcomeStatus {
    /// The action completed successfully.
    Succeeded,
    /// The target failed or its result became unknown.
    Failed {
        /// Sanitized failure detail.
        message: String,
    },
    /// The target was not dispatched.
    Skipped {
        /// Sanitized skip reason.
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Detailed result for one selected cleanup target.
pub struct TargetOutcome {
    /// Selected target identifier.
    pub target_id: TargetId,
    /// Public action classification.
    pub action: ActionKind,
    /// Terminal target status.
    pub status: OutcomeStatus,
    /// Confidence-preserving estimated recoverable capacity.
    pub estimated_recoverable: CapacityEstimate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Non-fatal normalization note for an execution request.
pub enum ExecutionNote {
    /// A repeated selected target identifier was removed.
    DuplicateSelectionRemoved {
        /// Deduplicated target identifier.
        target_id: TargetId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Aggregate outcome of a dry run or cleanup execution.
pub struct ExecutionReport {
    /// Whether the run performed no side effects.
    pub dry_run: bool,
    /// Number of validated targets selected by the request.
    pub selected: usize,
    /// Number of selected targets represented by terminal outcomes.
    pub attempted: usize,
    /// Number of successful target actions.
    pub succeeded: usize,
    /// Number of failed target actions.
    pub failed: usize,
    /// Number of targets skipped before dispatch.
    pub skipped: usize,
    /// Per-target failure summaries.
    pub failures: Vec<ActionFailure>,
    /// One terminal outcome for every deduplicated selected target.
    pub outcomes: Vec<TargetOutcome>,
    /// Non-fatal request-normalization notes.
    pub notes: Vec<ExecutionNote>,
    /// Estimated recoverable capacity across all selected target outcomes.
    pub estimated_recoverable: ScanTotals,
    /// Confirmation identity for the validated manifest and canonical selection.
    pub confirmation_digest: ConfirmationDigest,
    /// Audit JSONL path used by an executing run.
    pub audit_log: Option<PathBuf>,
}

impl ExecutionReport {
    /// Returns whether any target action failed.
    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }

    fn new(
        selected_targets: &[&ValidatedTarget],
        notes: Vec<ExecutionNote>,
        confirmation_digest: ConfirmationDigest,
        dry_run: bool,
        audit_log: Option<PathBuf>,
    ) -> Self {
        Self {
            dry_run,
            selected: selected_targets.len(),
            attempted: 0,
            succeeded: 0,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
            outcomes: Vec::with_capacity(selected_targets.len()),
            notes,
            estimated_recoverable: ScanTotals::from_sizes(selected_targets.iter().map(|target| {
                let target = target.target();
                (target.estimated_bytes, target.size_complete)
            })),
            confirmation_digest,
            audit_log,
        }
    }

    fn record_outcome(&mut self, target: &CleanTarget, status: OutcomeStatus) {
        self.attempted += 1;
        match &status {
            OutcomeStatus::Succeeded => self.succeeded += 1,
            OutcomeStatus::Failed { message } => {
                self.failed += 1;
                self.failures.push(ActionFailure {
                    target_id: target.id.clone(),
                    message: message.clone(),
                });
            }
            OutcomeStatus::Skipped { .. } => self.skipped += 1,
        }
        self.outcomes.push(TargetOutcome {
            target_id: target.id.clone(),
            action: ActionKind::from_action(&target.action),
            status,
            estimated_recoverable: CapacityEstimate::from_target(target),
        });
    }

    fn record_remaining_skipped(&mut self, selected_targets: &[&ValidatedTarget], reason: &str) {
        for validated in selected_targets.iter().skip(self.outcomes.len()) {
            self.record_outcome(
                validated.target(),
                OutcomeStatus::Skipped {
                    reason: reason.to_string(),
                },
            );
        }
    }

    fn assert_consistent(&self) {
        debug_assert_eq!(self.selected, self.outcomes.len());
        debug_assert_eq!(self.attempted, self.outcomes.len());
        debug_assert_eq!(self.attempted, self.succeeded + self.failed + self.skipped);
        debug_assert_eq!(self.failed, self.failures.len());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Failure summary for one cleanup target.
pub struct ActionFailure {
    /// Target that failed or reached an unknown result.
    pub target_id: TargetId,
    /// Sanitized failure detail.
    pub message: String,
}

struct PreparedSelection<'a> {
    targets: Vec<&'a ValidatedTarget>,
    notes: Vec<ExecutionNote>,
}

/// Computes the confirmation identity for a validated manifest and target selection.
///
/// Unknown and inspect-only target identifiers are rejected using [`ExecutionError`].
/// Repeated identifiers are treated as one canonical selection.
pub fn confirmation_digest(
    plan: &ValidatedPlan,
    selected: &[TargetId],
) -> Result<ConfirmationDigest> {
    let prepared = prepare_selection(plan, selected)?;
    confirmation_digest_for_targets(plan, &prepared.targets)
}

fn confirmation_digest_for_targets(
    plan: &ValidatedPlan,
    selected_targets: &[&ValidatedTarget],
) -> Result<ConfirmationDigest> {
    let selected = selected_targets
        .iter()
        .map(|target| target.target().id.clone())
        .collect::<Vec<_>>();
    plan.confirmation_digest(&selected)
        .map(ConfirmationDigest::from_canonical)
}

fn prepare_selection<'a>(
    plan: &'a ValidatedPlan,
    requested: &[TargetId],
) -> Result<PreparedSelection<'a>> {
    let plan_ids = plan
        .targets()
        .iter()
        .map(|target| &target.target().id)
        .collect::<HashSet<_>>();
    let mut selected_ids = HashSet::new();
    let mut noted_duplicates = HashSet::new();
    let mut notes = Vec::new();

    for target_id in requested {
        if !plan_ids.contains(target_id) {
            bail!(ExecutionError::UnknownTarget {
                target_id: target_id.clone(),
            });
        }
        if !selected_ids.insert(target_id.clone()) && noted_duplicates.insert(target_id.clone()) {
            notes.push(ExecutionNote::DuplicateSelectionRemoved {
                target_id: target_id.clone(),
            });
        }
    }

    let targets = plan
        .targets()
        .iter()
        .filter(|target| selected_ids.contains(&target.target().id))
        .collect::<Vec<_>>();

    for target in &targets {
        match &target.target().action {
            CleanAction::NoopInspectOnly => {
                bail!(ExecutionError::InspectOnlyTarget {
                    target_id: target.target().id.clone(),
                });
            }
            CleanAction::DeletePermanently { .. } => {
                bail!("permanent delete is disabled in this build");
            }
            CleanAction::Command { .. } | CleanAction::MoveToTrash { .. } => {}
        }
    }

    Ok(PreparedSelection { targets, notes })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Terminal status reported for one target action.
pub enum ExecutionTargetStatus {
    /// The action completed successfully.
    Succeeded,
    /// The action failed.
    Failed,
    /// The action was safely skipped before dispatch.
    Skipped,
    /// Side effect may have run but durable terminal audit failed.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Progress snapshot emitted after processing one selected target.
pub struct ExecutionProgress {
    /// Number of selected targets processed so far.
    pub completed: usize,
    /// Total number of selected targets in the request.
    pub total: usize,
    /// Target associated with this progress update.
    pub target_id: TargetId,
    /// Terminal status for the target.
    pub status: ExecutionTargetStatus,
    /// Sanitized progress detail.
    pub message: String,
}

/// Executes validated cleanup plans through command, trash, safety, and audit boundaries.
pub struct Executor<C = ProcessCommandRunner, T = SystemTrashRunner> {
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
    /// Creates an executor with injectable command and trash runners.
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
    /// Runs a validated plan without progress callbacks.
    pub fn run_plan(
        &self,
        plan: &ValidatedPlan,
        request: ExecutionRequest,
    ) -> Result<ExecutionReport> {
        self.run_plan_with_progress(plan, request, |_| {})
    }

    /// Runs a validated plan and reports terminal progress per target.
    pub fn run_plan_with_progress<F>(
        &self,
        plan: &ValidatedPlan,
        request: ExecutionRequest,
        mut on_progress: F,
    ) -> Result<ExecutionReport>
    where
        F: FnMut(ExecutionProgress),
    {
        let prepared = prepare_selection(plan, &request.selected)?;
        let confirmation_digest = confirmation_digest_for_targets(plan, &prepared.targets)?;
        if request.execute
            && let Some(expected_digest) = request.expected_digest.as_ref()
            && expected_digest != &confirmation_digest
        {
            bail!(ExecutionError::StaleConfirmation {
                expected_digest: expected_digest.clone(),
                actual_digest: confirmation_digest,
            });
        }
        let selected_targets = prepared.targets;

        if !request.execute {
            let mut report = ExecutionReport::new(
                &selected_targets,
                prepared.notes,
                confirmation_digest,
                true,
                None,
            );
            for validated in &selected_targets {
                report.record_outcome(
                    validated.target(),
                    OutcomeStatus::Skipped {
                        reason: "dry run; no cleanup action executed".to_string(),
                    },
                );
            }
            report.assert_consistent();
            return Ok(report);
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
        let mut report = ExecutionReport::new(
            &selected_targets,
            prepared.notes,
            confirmation_digest,
            false,
            Some(audit_path.clone()),
        );

        let total = selected_targets.len();
        let mut executed_fingerprints = HashSet::new();
        let auth_context = AuthorizationContext {
            scan_roots: Vec::new(),
            audit_log: Some(audit_path.clone()),
            expected_identity: None,
        };
        let cancel = request.cancel.clone();
        let mut halt_reason = None;
        for validated_target in &selected_targets {
            if cancel
                .as_ref()
                .is_some_and(|flag| flag.is_cancel_requested())
            {
                let message = "canceled before action started".to_string();
                report.record_outcome(
                    validated_target.target(),
                    OutcomeStatus::Skipped { reason: message },
                );
                on_progress(ExecutionProgress {
                    completed: report.succeeded + report.failed + report.skipped,
                    total,
                    target_id: validated_target.target().id.clone(),
                    status: ExecutionTargetStatus::Skipped,
                    message: "canceled".to_string(),
                });
                // Remaining targets are not started once cancel is observed.
                halt_reason = Some("canceled before action started");
                break;
            }

            let target = validated_target.target();
            let started_at = Instant::now();
            let dispatch_attempted =
                executed_fingerprints.insert(validated_target.fingerprint().clone());

            if !dispatch_attempted {
                let message = format!(
                    "duplicate action fingerprint rejected before execution: {}",
                    validated_target.fingerprint().as_str()
                );
                report.record_outcome(
                    target,
                    OutcomeStatus::Failed {
                        message: message.clone(),
                    },
                );
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
                    let seq = journal.next_sequence();
                    let terminal = JournalEvent::skipped(
                        journal.run_id(),
                        seq,
                        plan.digest(),
                        target,
                        message.clone(),
                        started_at.elapsed().as_millis(),
                    );
                    if let Err(audit_error) = journal.write_terminal(terminal) {
                        let audit_message = format!(
                            "safety skip ({message}); audit persistence failed: {audit_error}"
                        );
                        let message = sanitize_process_output(
                            audit_message.as_bytes(),
                            EXECUTOR_DIAGNOSTIC_CAP,
                        );
                        report.record_outcome(
                            target,
                            OutcomeStatus::Failed {
                                message: message.clone(),
                            },
                        );
                        on_progress(ExecutionProgress {
                            completed: report.succeeded + report.failed + report.skipped,
                            total,
                            target_id: target.id.clone(),
                            status: ExecutionTargetStatus::Failed,
                            message,
                        });
                        // Do not dispatch another target after the journal has failed.
                        halt_reason = Some("execution halted after audit persistence failure");
                        break;
                    }
                    report.record_outcome(
                        target,
                        OutcomeStatus::Skipped {
                            reason: message.clone(),
                        },
                    );
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
                        report.record_outcome(
                            target,
                            OutcomeStatus::Failed {
                                message: message.clone(),
                            },
                        );
                        on_progress(ExecutionProgress {
                            completed: report.succeeded + report.failed + report.skipped,
                            total,
                            target_id: target.id.clone(),
                            status: ExecutionTargetStatus::Failed,
                            message,
                        });
                        // Do not dispatch another target without recording this denial.
                        halt_reason = Some("execution halted after audit persistence failure");
                        break;
                    }
                    report.record_outcome(
                        target,
                        OutcomeStatus::Failed {
                            message: message.clone(),
                        },
                    );
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
                let message = format!("audit-blocked before side effect: {error}");
                report.record_outcome(
                    target,
                    OutcomeStatus::Failed {
                        message: message.clone(),
                    },
                );
                on_progress(ExecutionProgress {
                    completed: report.succeeded + report.failed + report.skipped,
                    total,
                    target_id: target.id.clone(),
                    status: ExecutionTargetStatus::Failed,
                    message,
                });
                // Halt further actions after audit-start failure.
                halt_reason = Some("execution halted after audit persistence failure");
                break;
            }

            let outcome = self.dispatch_authorized(&authorized, target, cancel.as_deref());
            let duration_ms = started_at.elapsed().as_millis();
            let finish_seq = journal.next_sequence();
            let run_id = journal.run_id().to_string();
            let (progress_status, progress_message, outcome_status, terminal) = match outcome {
                Ok(ActionStatus::Success {
                    command,
                    exit_code,
                    action_path,
                }) => (
                    ExecutionTargetStatus::Succeeded,
                    "completed".to_string(),
                    OutcomeStatus::Succeeded,
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
                ),
                Ok(ActionStatus::Skipped { message }) => (
                    ExecutionTargetStatus::Skipped,
                    format!("skipped: {message}"),
                    OutcomeStatus::Skipped {
                        reason: message.clone(),
                    },
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
                ),
                Err(error) => {
                    let message = error.to_string();
                    (
                        ExecutionTargetStatus::Failed,
                        message.clone(),
                        OutcomeStatus::Failed {
                            message: message.clone(),
                        },
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
                let message = format!(
                    "action result known ({progress_message}); audit persistence failed: {error}"
                );
                report.record_outcome(
                    target,
                    OutcomeStatus::Failed {
                        message: message.clone(),
                    },
                );
                on_progress(ExecutionProgress {
                    completed: report.succeeded + report.failed + report.skipped,
                    total,
                    target_id: target.id.clone(),
                    status: ExecutionTargetStatus::Unknown,
                    message,
                });
                halt_reason = Some("execution halted after audit persistence failure");
                break;
            }

            report.record_outcome(target, outcome_status);
            on_progress(ExecutionProgress {
                completed: report.succeeded + report.failed + report.skipped,
                total,
                target_id: target.id.clone(),
                status: progress_status,
                message: progress_message,
            });
        }

        if let Some(reason) = halt_reason {
            report.record_remaining_skipped(&selected_targets, reason);
        }
        let _ = journal.flush();
        report.assert_consistent();
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
                    expected_digest: None,
                    cancel: None,
                },
            )
            .expect("dry-run succeeds");

        assert!(report.dry_run);
        assert_eq!(report.selected, 1);
        assert_eq!(report.attempted, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.outcomes.len(), 1);
        assert_eq!(
            report.confirmation_digest,
            confirmation_digest(&test_validated_plan(&plan), &plan.default_selected_ids())
                .expect("confirmation digest")
        );
        assert!(cleanup_path.exists(), "dry-run must not move fixture");
        assert!(command_runner.requests().is_empty());
        assert!(trash_runner.paths().is_empty());
    }

    #[test]
    fn execution_report_round_trips_with_outcomes_and_capacity_confidence() {
        let fixture = TempDir::new().expect("temp dir");
        let mut verified = target(
            "verified",
            CleanAction::MoveToTrash {
                path: fixture.path().join("verified"),
            },
            Some(fixture.path().join("verified")),
        );
        verified.estimated_bytes = 100;
        let mut partial = target(
            "partial",
            CleanAction::MoveToTrash {
                path: fixture.path().join("partial"),
            },
            Some(fixture.path().join("partial")),
        );
        partial.estimated_bytes = 50;
        partial.size_complete = false;
        let mut unknown = target(
            "unknown",
            CleanAction::MoveToTrash {
                path: fixture.path().join("unknown"),
            },
            Some(fixture.path().join("unknown")),
        );
        unknown.estimated_bytes = 0;
        unknown.size_complete = false;
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![verified, partial, unknown],
        };
        let validated = test_validated_plan(&plan);

        let report = test_executor(
            RecordingCommandRunner::default(),
            RecordingTrashRunner::default(),
        )
        .run_plan(
            &validated,
            ExecutionRequest {
                selected: validated.default_selected_ids(),
                execute: false,
                audit_log: None,
                expected_digest: None,
                cancel: None,
            },
        )
        .expect("dry-run report");

        assert_eq!(report.selected, 3);
        assert_eq!(report.attempted, 3);
        assert_eq!(report.succeeded + report.failed + report.skipped, 3);
        assert_eq!(report.outcomes.len(), 3);
        assert_eq!(report.estimated_recoverable.verified_bytes, 100);
        assert_eq!(report.estimated_recoverable.partial_lower_bound_bytes, 50);
        assert_eq!(report.estimated_recoverable.unknown_target_count, 1);
        assert_eq!(
            report.outcomes[0].estimated_recoverable,
            CapacityEstimate::Verified { bytes: 100 }
        );
        assert_eq!(
            report.outcomes[1].estimated_recoverable,
            CapacityEstimate::Partial {
                lower_bound_bytes: 50
            }
        );
        assert_eq!(
            report.outcomes[2].estimated_recoverable,
            CapacityEstimate::Unknown
        );

        let json = serde_json::to_value(&report).expect("execution report serializes");
        assert_eq!(json["outcomes"][0]["action"]["type"], "move_to_trash");
        assert_eq!(json["outcomes"][0]["status"]["type"], "skipped");
        assert_eq!(
            serde_json::from_value::<ExecutionReport>(json).expect("execution report deserializes"),
            report
        );

        let failure = ActionFailure {
            target_id: TargetId::new("failed.target"),
            message: "sanitized failure".to_string(),
        };
        let failure_json = serde_json::to_string(&failure).expect("failure serializes");
        assert_eq!(
            serde_json::from_str::<ActionFailure>(&failure_json).expect("failure deserializes"),
            failure
        );
    }

    #[test]
    fn confirmation_digest_is_stable_and_sensitive_to_manifest_and_selection() {
        let fixture = TempDir::new().expect("temp dir");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "first",
                    CleanAction::MoveToTrash {
                        path: fixture.path().join("first"),
                    },
                    Some(fixture.path().join("first")),
                ),
                target(
                    "second",
                    CleanAction::MoveToTrash {
                        path: fixture.path().join("second"),
                    },
                    Some(fixture.path().join("second")),
                ),
            ],
        };
        let validated = test_validated_plan(&plan);
        let first_id = plan.targets[0].id.clone();
        let second_id = plan.targets[1].id.clone();

        let digest = confirmation_digest(&validated, &[first_id.clone(), second_id.clone()])
            .expect("confirmation digest");
        let reordered = confirmation_digest(
            &validated,
            &[second_id.clone(), first_id.clone(), first_id.clone()],
        )
        .expect("reordered confirmation digest");
        let different_selection =
            confirmation_digest(&validated, &[first_id]).expect("selection digest");
        let mut changed_plan = plan.clone();
        changed_plan.targets[0].estimated_bytes += 1;
        let changed_manifest = confirmation_digest(
            &test_validated_plan(&changed_plan),
            &changed_plan.default_selected_ids(),
        )
        .expect("changed manifest digest");

        assert_eq!(digest, reordered);
        assert_ne!(digest, different_selection);
        assert_ne!(digest, changed_manifest);
        assert_eq!(digest.as_str().len(), 64);
    }

    #[test]
    fn stale_confirmation_rejects_before_audit_or_side_effects() {
        let fixture = TempDir::new().expect("temp dir");
        let audit_path = fixture.path().join("audit.jsonl");
        let first_path = fixture.path().join("first");
        let second_path = fixture.path().join("second");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "first",
                    CleanAction::MoveToTrash {
                        path: first_path.clone(),
                    },
                    Some(first_path.clone()),
                ),
                target(
                    "second",
                    CleanAction::MoveToTrash {
                        path: second_path.clone(),
                    },
                    Some(second_path.clone()),
                ),
            ],
        };
        let validated = test_validated_plan(&plan);
        let stale = confirmation_digest(&validated, &[plan.targets[0].id.clone()])
            .expect("stale selection digest");
        let actual = confirmation_digest(&validated, &validated.default_selected_ids())
            .expect("current selection digest");
        let trash = RecordingTrashRunner::default();

        let error = test_executor(RecordingCommandRunner::default(), trash.clone())
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected: validated.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    expected_digest: Some(stale.clone()),
                    cancel: None,
                },
            )
            .expect_err("stale confirmation rejects");

        assert!(matches!(
            error.downcast_ref::<ExecutionError>(),
            Some(ExecutionError::StaleConfirmation {
                expected_digest,
                actual_digest,
            }) if expected_digest == &stale && actual_digest == &actual
        ));
        assert!(trash.paths().is_empty());
        assert!(!audit_path.exists());
        assert!(first_path.exists());
        assert!(second_path.exists());
    }

    #[test]
    fn duplicate_selection_is_deduplicated_and_noted() {
        let fixture = TempDir::new().expect("temp dir");
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
        let validated = test_validated_plan(&plan);
        let selected_id = plan.targets[0].id.clone();

        let report = test_executor(
            RecordingCommandRunner::default(),
            RecordingTrashRunner::default(),
        )
        .run_plan(
            &validated,
            ExecutionRequest {
                selected: vec![
                    selected_id.clone(),
                    selected_id.clone(),
                    selected_id.clone(),
                ],
                execute: false,
                audit_log: None,
                expected_digest: None,
                cancel: None,
            },
        )
        .expect("duplicate selection is normalized");

        assert_eq!(report.selected, 1);
        assert_eq!(report.attempted, 1);
        assert_eq!(report.outcomes.len(), 1);
        assert_eq!(
            report.notes,
            vec![ExecutionNote::DuplicateSelectionRemoved {
                target_id: selected_id
            }]
        );
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
                    expected_digest: None,
                    cancel: None,
                },
            )
            .expect("execute succeeds");

        assert_eq!(report.succeeded, 1);
        assert_eq!(
            report.outcomes[0].action,
            ActionKind::Command { irreversible: true }
        );
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
                    audit_log: Some(audit_path.clone()),
                    expected_digest: None,
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
                    expected_digest: None,
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
                    expected_digest: None,
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
                    expected_digest: None,
                    cancel: None,
                },
            )
            .expect_err("disabled permanent delete cannot be selected");

        assert!(error.to_string().contains("permanent delete is disabled"));
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
                    expected_digest: None,
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
                    expected_digest: None,
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
                    expected_digest: None,
                    cancel: None,
                },
            )
            .expect_err("inspect-only target cannot be selected");

        assert!(matches!(
            error.downcast_ref::<ExecutionError>(),
            Some(ExecutionError::InspectOnlyTarget { target_id })
                if target_id == &plan.targets[0].id
        ));
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
                    expected_digest: None,
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
    fn safety_skip_audit_failure_halts_before_later_side_effects() {
        struct FailFirstWriteIo {
            writes: usize,
        }
        impl JournalIo for FailFirstWriteIo {
            fn write_line(&mut self, _line: &str) -> Result<()> {
                self.writes += 1;
                if self.writes == 1 {
                    bail!("injected safety-skip audit failure");
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
        let current_exe = std::env::current_exe().expect("current executable path");
        let self_clean_path = current_exe
            .parent()
            .expect("current executable has parent")
            .to_path_buf();
        let later_path = fixture.path().join("later").join("node_modules");
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "test.self_clean",
                    CleanAction::Command {
                        program: "cargo".to_string(),
                        args: vec!["clean".to_string()],
                        cwd: None,
                        irreversible: true,
                    },
                    Some(self_clean_path),
                ),
                target(
                    "node.node_modules",
                    CleanAction::MoveToTrash {
                        path: later_path.clone(),
                    },
                    Some(later_path),
                ),
            ],
        };
        let trash = RecordingTrashRunner::default();
        let executor = test_executor(RecordingCommandRunner::default(), trash.clone())
            .with_journal_io(Box::new(FailFirstWriteIo { writes: 0 }));

        let report = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: plan.default_selected_ids(),
                    execute: true,
                    audit_log: Some(audit_path),
                    expected_digest: None,
                    cancel: None,
                },
            )
            .expect("audit failure is represented in the report");

        assert!(trash.paths().is_empty(), "later target must not dispatch");
        assert_eq!(report.selected, 2);
        assert_eq!(report.attempted, 2);
        assert_eq!(report.failed, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.outcomes.len(), 2);
        assert!(matches!(
            &report.outcomes[0].status,
            OutcomeStatus::Failed { message }
                if message.contains("injected safety-skip audit failure")
        ));
        assert!(matches!(
            &report.outcomes[1].status,
            OutcomeStatus::Skipped { reason }
                if reason.contains("audit persistence failure")
        ));
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
                    expected_digest: None,
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
                    expected_digest: None,
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
                    expected_digest: None,
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
                    expected_digest: None,
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
                    expected_digest: None,
                    cancel: None,
                },
            )
            .expect("duplicate is recorded as a partial failure");

        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.attempted, 2);
        assert_eq!(report.outcomes.len(), 2);
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
                    expected_digest: None,
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
                    expected_digest: None,
                    cancel: Some(cancel),
                },
            )
            .expect("cancel produces report");
        assert_eq!(trash.paths().len(), 0, "no side effects after cancel");
        assert_eq!(report.selected, 2);
        assert_eq!(report.attempted, 2);
        assert_eq!(report.skipped, 2);
        assert_eq!(report.outcomes.len(), 2);
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
                    expected_digest: None,
                    cancel: None,
                },
                |event| progress.push(event),
            )
            .expect("audit failure is reported per target");

        assert!(
            trash.paths().is_empty(),
            "started audit must precede dispatch"
        );
        assert_eq!(report.attempted, 1);
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
                    expected_digest: None,
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
    fn unknown_ids_in_selection_are_rejected_atomically() {
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

        let unknown_id = TargetId::new("ghost.target:none");
        let error = executor
            .run_plan(
                &test_validated_plan(&plan),
                ExecutionRequest {
                    selected: vec![unknown_id.clone(), plan.targets[0].id.clone()],
                    execute: true,
                    audit_log: Some(audit_path.clone()),
                    expected_digest: None,
                    cancel: None,
                },
            )
            .expect_err("unknown selection rejects before execution");

        assert!(matches!(
            error.downcast_ref::<ExecutionError>(),
            Some(ExecutionError::UnknownTarget { target_id }) if target_id == &unknown_id
        ));
        assert!(trash_runner.paths().is_empty());
        assert!(cleanup_path.exists());
        assert!(!audit_path.exists());
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
