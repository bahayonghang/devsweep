//! One-operation Optimize V1 execution, closed outcomes, and crash recovery.

use std::{
    fmt,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::LivePreflight;
use super::audit::{
    OptimizeAuditError, OptimizeAuditErrorCode, OptimizeAuditEvent, OptimizeAuditJournal,
    OptimizeAuditRecordV1, OptimizeAuditStatusCode, OptimizeAuditTransition,
    optimize_audit_v1_path,
};
use super::catalogue::MaintenanceActionClass;
use super::plan::{OptimizePlanError, ValidatedMaintenanceAction, construct_validated_action};
use crate::process::{CancelObserver, FlagCancelObserver};

/// Execution report schema generation.
pub const OPTIMIZE_EXECUTION_VERSION: u32 = 1;

/// The fixed 10-second bound of the DNS resolver cache refresh.
pub const DNS_FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// The closed adapter completion states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterOutcome {
    /// The adapter proved its fixed success signal.
    Success,
    /// The adapter observed a defined failure.
    Failure,
    /// The adapter could not prove anything after dispatch.
    Unfinished,
}

/// The closed Optimize terminal outcomes: one pre-dispatch cancellation and
/// exactly the post-dispatch terminals of the V1 evidence table. A Settings
/// action can only ever reach `Launched`, which is never maintenance
/// completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceExecutionOutcome {
    CanceledBeforeStart,
    Succeeded,
    Launched,
    Failed,
    UnknownAfterDispatch,
}

impl MaintenanceExecutionOutcome {
    #[must_use]
    pub fn was_dispatched(self) -> bool {
        self != Self::CanceledBeforeStart
    }

    #[must_use]
    pub fn is_success(self) -> bool {
        matches!(self, Self::Succeeded | Self::Launched)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AdapterEvidence {
    pub(super) outcome: AdapterOutcome,
    pub(super) error_code: Option<OptimizeAuditErrorCode>,
}

impl AdapterEvidence {
    pub(super) const fn success() -> Self {
        Self {
            outcome: AdapterOutcome::Success,
            error_code: None,
        }
    }

    pub(super) const fn failure(error_code: OptimizeAuditErrorCode) -> Self {
        Self {
            outcome: AdapterOutcome::Failure,
            error_code: Some(error_code),
        }
    }

    pub(super) const fn unfinished(error_code: OptimizeAuditErrorCode) -> Self {
        Self {
            outcome: AdapterOutcome::Unfinished,
            error_code: Some(error_code),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DispatchTiming {
    pub(super) timeout: Duration,
    pub(super) termination_grace: Duration,
    pub(super) poll_interval: Duration,
}

impl Default for DispatchTiming {
    fn default() -> Self {
        Self {
            timeout: DNS_FLUSH_TIMEOUT,
            termination_grace: Duration::from_secs(2),
            poll_interval: Duration::from_millis(20),
        }
    }
}

pub(super) trait MaintenanceDispatch: Send + Sync {
    fn dispatch(
        &self,
        action: &ValidatedMaintenanceAction,
        cancel: Option<&Arc<FlagCancelObserver>>,
        timing: DispatchTiming,
    ) -> AdapterEvidence;
}

/// One terminal Optimize outcome. It carries no program path, argv, URI, or
/// process output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceActionOutcomeV1 {
    pub operation_id: String,
    pub catalogue_id: String,
    pub action_class: MaintenanceActionClass,
    pub outcome: MaintenanceExecutionOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<OptimizeAuditErrorCode>,
}

/// The V1 execution report of the exactly one dispatched operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceExecutionReportV1 {
    pub version: u32,
    pub catalogue_version: u32,
    pub outcomes: Vec<MaintenanceActionOutcomeV1>,
}

impl MaintenanceExecutionReportV1 {
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.outcomes
            .iter()
            .all(|outcome| outcome.outcome.is_success())
    }
}

#[derive(Debug)]
pub enum MaintenanceExecutionError {
    Plan(OptimizePlanError),
    Audit(OptimizeAuditError),
    ConfirmationRequired,
    PermitPoisoned,
}

impl fmt::Display for MaintenanceExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => error.fmt(formatter),
            Self::Audit(error) => error.fmt(formatter),
            Self::ConfirmationRequired => {
                formatter.write_str("Optimize run requires explicit confirmation")
            }
            Self::PermitPoisoned => formatter.write_str("Optimize execution permit is poisoned"),
        }
    }
}

impl std::error::Error for MaintenanceExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            Self::Audit(error) => Some(error),
            Self::ConfirmationRequired | Self::PermitPoisoned => None,
        }
    }
}

impl From<OptimizePlanError> for MaintenanceExecutionError {
    fn from(value: OptimizePlanError) -> Self {
        Self::Plan(value)
    }
}

impl From<OptimizeAuditError> for MaintenanceExecutionError {
    fn from(value: OptimizeAuditError) -> Self {
        Self::Audit(value)
    }
}

/// The confirmed run request. There is no field that can inject a validated
/// action, a resolved identity, or a live preflight result.
#[derive(Debug, Clone)]
pub struct MaintenanceExecutionRequest<'a> {
    pub plan: &'a super::plan::MaintenancePlanV1,
    pub expected_preview_digest: &'a str,
    pub confirmed: bool,
    pub cancel: Option<Arc<FlagCancelObserver>>,
}

pub struct MaintenanceExecutor {
    dispatch: Arc<dyn MaintenanceDispatch>,
    preflight: Arc<dyn LivePreflight>,
    audit_path: Option<PathBuf>,
    timing: DispatchTiming,
}

impl Default for MaintenanceExecutor {
    fn default() -> Self {
        Self {
            dispatch: Arc::new(super::windows::RealMaintenanceDispatch),
            preflight: Arc::new(super::windows::RealLivePreflight),
            audit_path: None,
            timing: DispatchTiming::default(),
        }
    }
}

static EXECUTION_PERMIT: OnceLock<Mutex<()>> = OnceLock::new();
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

impl MaintenanceExecutor {
    /// Executes the exactly one confirmed operation. The exclusive journal
    /// lock and the process-wide one-operation permit cover intent, dispatch,
    /// and the terminal/unknown transition.
    pub fn execute(
        &self,
        request: MaintenanceExecutionRequest<'_>,
    ) -> Result<MaintenanceExecutionReportV1, MaintenanceExecutionError> {
        if !request.confirmed {
            return Err(MaintenanceExecutionError::ConfirmationRequired);
        }
        let _permit = EXECUTION_PERMIT
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| MaintenanceExecutionError::PermitPoisoned)?;
        let path = match &self.audit_path {
            Some(path) => path.clone(),
            None => optimize_audit_v1_path()?,
        };
        let mut journal = OptimizeAuditJournal::open(&path)?;
        self.recover_pending(&mut journal)?;
        // Live authority is resolved behind both single-flight boundaries and
        // after pending dispatches are reconciled. Callers cannot substitute
        // a serialized or precomputed validated action.
        let action = construct_validated_action(request.plan, self.preflight.as_ref())?;
        if action.preview_digest != request.expected_preview_digest {
            return Err(MaintenanceExecutionError::Plan(
                OptimizePlanError::DigestMismatch,
            ));
        }
        let outcome = self.execute_one(&mut journal, &action, request.cancel.as_ref())?;
        Ok(MaintenanceExecutionReportV1 {
            version: OPTIMIZE_EXECUTION_VERSION,
            catalogue_version: action.catalogue_version,
            outcomes: vec![outcome],
        })
    }

    /// Reconciles durable nonterminal operations before accepting a new
    /// action. Recovery never redispatches and never re-resolves identities.
    pub fn recover_startup(
        &self,
    ) -> Result<Vec<MaintenanceActionOutcomeV1>, MaintenanceExecutionError> {
        let _permit = EXECUTION_PERMIT
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| MaintenanceExecutionError::PermitPoisoned)?;
        let path = match &self.audit_path {
            Some(path) => path.clone(),
            None => optimize_audit_v1_path()?,
        };
        let mut journal = OptimizeAuditJournal::open(&path)?;
        self.recover_pending(&mut journal)
    }

    fn execute_one(
        &self,
        journal: &mut OptimizeAuditJournal,
        action: &ValidatedMaintenanceAction,
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<MaintenanceActionOutcomeV1, MaintenanceExecutionError> {
        let operation_id = next_operation_id();
        journal.append(record(
            &operation_id,
            action,
            OptimizeAuditEvent {
                transition: OptimizeAuditTransition::Validated,
                status_code: OptimizeAuditStatusCode::Validated,
                error_code: None,
                adapter_outcome: None,
            },
        ))?;

        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            let outcome = MaintenanceExecutionOutcome::CanceledBeforeStart;
            journal.append(record(
                &operation_id,
                action,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::Terminal { outcome },
                    status_code: OptimizeAuditStatusCode::CanceledBeforeStart,
                    error_code: None,
                    adapter_outcome: None,
                },
            ))?;
            return Ok(action_outcome(&operation_id, action, outcome, None));
        }

        // This flush+sync inside the journal append is the durable line
        // immediately before the process or handoff boundary.
        journal.append(record(
            &operation_id,
            action,
            OptimizeAuditEvent {
                transition: OptimizeAuditTransition::DispatchStarted,
                status_code: OptimizeAuditStatusCode::DispatchStarted,
                error_code: None,
                adapter_outcome: None,
            },
        ))?;

        let evidence = self.dispatch.dispatch(action, cancel, self.timing);
        journal.append(record(
            &operation_id,
            action,
            OptimizeAuditEvent {
                transition: OptimizeAuditTransition::AdapterCompleted,
                status_code: adapter_status(evidence.outcome),
                error_code: None,
                adapter_outcome: Some(evidence.outcome),
            },
        ))?;

        let outcome = classify_terminal(action.action_class, evidence.outcome);
        journal.append(record(
            &operation_id,
            action,
            OptimizeAuditEvent {
                transition: OptimizeAuditTransition::Terminal { outcome },
                status_code: terminal_status(outcome),
                error_code: evidence.error_code,
                adapter_outcome: Some(evidence.outcome),
            },
        ))?;
        Ok(action_outcome(
            &operation_id,
            action,
            outcome,
            evidence.error_code,
        ))
    }

    fn recover_pending(
        &self,
        journal: &mut OptimizeAuditJournal,
    ) -> Result<Vec<MaintenanceActionOutcomeV1>, MaintenanceExecutionError> {
        let mut recovered = Vec::new();
        for pending in journal.pending_operations() {
            let (outcome, status, adapter) = match pending.last_transition {
                OptimizeAuditTransition::Validated => (
                    MaintenanceExecutionOutcome::CanceledBeforeStart,
                    OptimizeAuditStatusCode::RecoveredBeforeDispatch,
                    None,
                ),
                _ => (
                    MaintenanceExecutionOutcome::UnknownAfterDispatch,
                    OptimizeAuditStatusCode::UnknownAfterDispatch,
                    Some(AdapterOutcome::Unfinished),
                ),
            };
            let action = RecordedAction {
                operation_id: pending.operation_id.clone(),
                catalogue_id: pending.catalogue_id.clone(),
                action_class: pending.action_class,
                preview_digest: pending.preview_digest.clone(),
            };
            journal.append(OptimizeAuditRecordV1::new(
                &action.operation_id,
                unix_ms(SystemTime::now()),
                &action.catalogue_id,
                action.action_class,
                &action.preview_digest,
                OptimizeAuditEvent {
                    transition: OptimizeAuditTransition::Terminal { outcome },
                    status_code: status,
                    error_code: Some(OptimizeAuditErrorCode::RecoveredAfterCrash),
                    adapter_outcome: adapter,
                },
            ))?;
            recovered.push(MaintenanceActionOutcomeV1 {
                operation_id: action.operation_id,
                catalogue_id: action.catalogue_id,
                action_class: action.action_class,
                outcome,
                error_code: Some(OptimizeAuditErrorCode::RecoveredAfterCrash),
            });
        }
        Ok(recovered)
    }

    #[cfg(test)]
    pub(super) fn for_test(
        audit_path: PathBuf,
        dispatch: Arc<dyn MaintenanceDispatch>,
        preflight: Arc<dyn LivePreflight>,
        timing: DispatchTiming,
    ) -> Self {
        Self {
            dispatch,
            preflight,
            audit_path: Some(audit_path),
            timing,
        }
    }
}

/// Audit-only view of one recorded operation used by recovery.
struct RecordedAction {
    operation_id: String,
    catalogue_id: String,
    action_class: MaintenanceActionClass,
    preview_digest: String,
}

pub(super) fn classify_terminal(
    action_class: MaintenanceActionClass,
    outcome: AdapterOutcome,
) -> MaintenanceExecutionOutcome {
    match (action_class, outcome) {
        (MaintenanceActionClass::Execute, AdapterOutcome::Success) => {
            MaintenanceExecutionOutcome::Succeeded
        }
        (MaintenanceActionClass::SettingsHandoff, AdapterOutcome::Success) => {
            MaintenanceExecutionOutcome::Launched
        }
        // Guidance never dispatches, so this arm is unreachable; it stays
        // closed and conservative instead of panicking.
        (MaintenanceActionClass::Guidance, _) => MaintenanceExecutionOutcome::UnknownAfterDispatch,
        (_, AdapterOutcome::Failure) => MaintenanceExecutionOutcome::Failed,
        (_, AdapterOutcome::Unfinished) => MaintenanceExecutionOutcome::UnknownAfterDispatch,
    }
}

fn adapter_status(outcome: AdapterOutcome) -> OptimizeAuditStatusCode {
    match outcome {
        AdapterOutcome::Success => OptimizeAuditStatusCode::AdapterSucceeded,
        AdapterOutcome::Failure => OptimizeAuditStatusCode::AdapterFailed,
        AdapterOutcome::Unfinished => OptimizeAuditStatusCode::AdapterUnfinished,
    }
}

fn terminal_status(outcome: MaintenanceExecutionOutcome) -> OptimizeAuditStatusCode {
    match outcome {
        MaintenanceExecutionOutcome::CanceledBeforeStart => {
            OptimizeAuditStatusCode::CanceledBeforeStart
        }
        MaintenanceExecutionOutcome::Succeeded => OptimizeAuditStatusCode::Succeeded,
        MaintenanceExecutionOutcome::Launched => OptimizeAuditStatusCode::Launched,
        MaintenanceExecutionOutcome::Failed => OptimizeAuditStatusCode::Failed,
        MaintenanceExecutionOutcome::UnknownAfterDispatch => {
            OptimizeAuditStatusCode::UnknownAfterDispatch
        }
    }
}

fn record(
    operation_id: &str,
    action: &ValidatedMaintenanceAction,
    event: OptimizeAuditEvent,
) -> OptimizeAuditRecordV1 {
    OptimizeAuditRecordV1::new(
        operation_id,
        unix_ms(SystemTime::now()),
        &action.operation_id,
        action.action_class,
        &action.preview_digest,
        event,
    )
}

fn action_outcome(
    operation_id: &str,
    action: &ValidatedMaintenanceAction,
    outcome: MaintenanceExecutionOutcome,
    error_code: Option<OptimizeAuditErrorCode>,
) -> MaintenanceActionOutcomeV1 {
    MaintenanceActionOutcomeV1 {
        operation_id: operation_id.to_string(),
        catalogue_id: action.operation_id.clone(),
        action_class: action.action_class,
        outcome,
        error_code,
    }
}

fn next_operation_id() -> String {
    let sequence = OPERATION_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!(
        "optimize-op-{:016x}-{:08x}-{sequence:016x}",
        unix_ms(SystemTime::now()),
        std::process::id()
    )
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |duration| {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    })
}
