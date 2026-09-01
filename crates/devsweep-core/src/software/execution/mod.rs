//! Current-user MSIX-only Software execution and recovery.

use std::{
    fmt,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::{
    SoftwareActionClass, SoftwareEligibilityReason, SoftwareIdentity, SoftwareInventorySource,
    SoftwareInventoryV1, SoftwarePlanError, SoftwareScope, SoftwareSelectionPlanV1,
    inventory_software, preview_selection_plan_live, validate_preview_digest,
};
use crate::process::{CancelObserver, FlagCancelObserver};

mod msix;

pub use super::audit::{
    SOFTWARE_AUDIT_VERSION, SoftwareAuditError, SoftwareAuditErrorCode, SoftwareAuditRecordV1,
    SoftwareAuditRequeryResult, SoftwareAuditStatusCode, SoftwareAuditTransition,
    software_audit_v1_path,
};
use super::audit::{SoftwareAuditEvent, SoftwareAuditJournal};
use msix::{RealInstalledStateRequery, RealMsixAdapter};

pub const SOFTWARE_EXECUTION_VERSION: u32 = 1;
pub const MSIX_MONITOR_TIMEOUT: Duration = Duration::from_secs(120);
pub const MSIX_CANCEL_GRACE: Duration = Duration::from_secs(5);
pub const MSIX_REQUERY_OFFSETS: [Duration; 3] = [
    Duration::ZERO,
    Duration::from_secs(2),
    Duration::from_secs(10),
];

/// The six closed states include one pre-dispatch cancellation and exactly the
/// five post-dispatch terminals from the Software V1 evidence table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareExecutionOutcome {
    CanceledBeforeStart,
    Removed,
    RebootRequired,
    StillPresent,
    Failed,
    UnknownAfterDispatch,
}

impl SoftwareExecutionOutcome {
    #[must_use]
    pub fn was_dispatched(self) -> bool {
        self != Self::CanceledBeforeStart
    }

    #[must_use]
    pub fn is_success(self) -> bool {
        matches!(self, Self::Removed | Self::RebootRequired)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareInstalledState {
    Present,
    Absent,
    Unavailable,
    Conflicting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareRebootEvidence {
    None,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterOutcome {
    Success,
    Failure,
    RebootRequired,
    Unfinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AdapterEvidence {
    pub(super) outcome: AdapterOutcome,
    pub(super) reboot_evidence: SoftwareRebootEvidence,
    pub(super) error_code: Option<SoftwareAuditErrorCode>,
}

impl AdapterEvidence {
    const fn success() -> Self {
        Self {
            outcome: AdapterOutcome::Success,
            reboot_evidence: SoftwareRebootEvidence::None,
            error_code: None,
        }
    }

    const fn failure(error_code: SoftwareAuditErrorCode) -> Self {
        Self {
            outcome: AdapterOutcome::Failure,
            reboot_evidence: SoftwareRebootEvidence::None,
            error_code: Some(error_code),
        }
    }

    const fn reboot_required() -> Self {
        Self {
            outcome: AdapterOutcome::RebootRequired,
            reboot_evidence: SoftwareRebootEvidence::Required,
            error_code: None,
        }
    }

    const fn unfinished(error_code: SoftwareAuditErrorCode) -> Self {
        Self {
            outcome: AdapterOutcome::Unfinished,
            reboot_evidence: SoftwareRebootEvidence::None,
            error_code: Some(error_code),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RequeryObservation {
    state: SoftwareInstalledState,
    error_code: Option<SoftwareAuditErrorCode>,
}

impl RequeryObservation {
    const fn unavailable() -> Self {
        Self {
            state: SoftwareInstalledState::Unavailable,
            error_code: Some(SoftwareAuditErrorCode::RequeryUnavailable),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RemovalTiming {
    monitor_timeout: Duration,
    cancel_grace: Duration,
    poll_interval: Duration,
}

impl Default for RemovalTiming {
    fn default() -> Self {
        Self {
            monitor_timeout: MSIX_MONITOR_TIMEOUT,
            cancel_grace: MSIX_CANCEL_GRACE,
            poll_interval: Duration::from_millis(25),
        }
    }
}

trait MsixRemovalAdapter: Send + Sync {
    fn remove(
        &self,
        action: &ValidatedSoftwareAction,
        cancel: Option<&Arc<FlagCancelObserver>>,
        timing: RemovalTiming,
    ) -> AdapterEvidence;
}

trait InstalledStateRequery: Send + Sync {
    fn query(&self, package_full_name: &str) -> RequeryObservation;
}

trait LiveSoftwareInventory: Send + Sync {
    fn inventory(&self) -> Result<SoftwareInventoryV1, SoftwareExecutionError>;
}

#[derive(Debug, Default)]
struct RealLiveSoftwareInventory;

impl LiveSoftwareInventory for RealLiveSoftwareInventory {
    fn inventory(&self) -> Result<SoftwareInventoryV1, SoftwareExecutionError> {
        inventory_software(SoftwareInventorySource::Msix, None)
            .map_err(|_| SoftwareExecutionError::LiveRevalidationUnavailable)
    }
}

/// Opaque execution authority. It has no `Serialize` or `Deserialize`
/// implementation and all fields are private.
///
/// ```compile_fail
/// use devsweep_core::software::{SoftwareIdentity, ValidatedSoftwareAction};
/// let _ = ValidatedSoftwareAction {
///     id: "forged".into(),
///     identity: SoftwareIdentity::Msix { package_full_name: "forged".into() },
///     package_full_name: "forged".into(),
///     inventory_fingerprint: "forged".into(),
///     preview_digest: "forged".into(),
/// };
/// ```
///
/// Execution requests cannot inject a serialized or caller-constructed
/// inventory as live authority:
///
/// ```compile_fail
/// use devsweep_core::software::SoftwareExecutionRequest;
/// let _ = SoftwareExecutionRequest {
///     plan: todo!(),
///     live_inventory: todo!(),
///     expected_preview_digest: "sha256:forged",
///     confirmed: true,
///     cancel: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSoftwareAction {
    id: String,
    identity: SoftwareIdentity,
    package_full_name: String,
    inventory_fingerprint: String,
    preview_digest: String,
}

impl ValidatedSoftwareAction {
    fn package_full_name(&self) -> &str {
        &self.package_full_name
    }
}

#[derive(Clone, Copy)]
struct AuditActionContext<'a> {
    software_id: &'a str,
    identity: &'a SoftwareIdentity,
    package_full_name: &'a str,
    inventory_fingerprint: &'a str,
    preview_digest: &'a str,
}

impl<'a> From<&'a ValidatedSoftwareAction> for AuditActionContext<'a> {
    fn from(action: &'a ValidatedSoftwareAction) -> Self {
        Self {
            software_id: &action.id,
            identity: &action.identity,
            package_full_name: &action.package_full_name,
            inventory_fingerprint: &action.inventory_fingerprint,
            preview_digest: &action.preview_digest,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SoftwareExecutionRequest<'a> {
    pub plan: &'a SoftwareSelectionPlanV1,
    pub expected_preview_digest: &'a str,
    pub confirmed: bool,
    pub cancel: Option<Arc<FlagCancelObserver>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareActionOutcomeV1 {
    pub operation_id: String,
    pub software_id: String,
    pub outcome: SoftwareExecutionOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_state: Option<SoftwareInstalledState>,
    pub reboot_evidence: SoftwareRebootEvidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<SoftwareAuditErrorCode>,
    pub irreversible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareExecutionReportV1 {
    pub version: u32,
    pub irreversible: bool,
    pub outcomes: Vec<SoftwareActionOutcomeV1>,
}

impl SoftwareExecutionReportV1 {
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.outcomes
            .iter()
            .all(|outcome| outcome.outcome.is_success())
    }
}

#[derive(Debug)]
pub enum SoftwareExecutionError {
    Plan(SoftwarePlanError),
    Audit(SoftwareAuditError),
    ConfirmationRequired,
    LiveRevalidationUnavailable,
    PermitPoisoned,
    CrashInjected(&'static str),
}

impl fmt::Display for SoftwareExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => error.fmt(formatter),
            Self::Audit(error) => error.fmt(formatter),
            Self::ConfirmationRequired => {
                formatter.write_str("Software execution requires explicit confirmation")
            }
            Self::LiveRevalidationUnavailable => {
                formatter.write_str("Software live revalidation is unavailable")
            }
            Self::PermitPoisoned => formatter.write_str("Software execution permit is poisoned"),
            Self::CrashInjected(point) => write!(formatter, "injected crash at {point}"),
        }
    }
}

impl std::error::Error for SoftwareExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            Self::Audit(error) => Some(error),
            Self::ConfirmationRequired
            | Self::LiveRevalidationUnavailable
            | Self::PermitPoisoned
            | Self::CrashInjected(_) => None,
        }
    }
}

impl From<SoftwarePlanError> for SoftwareExecutionError {
    fn from(value: SoftwarePlanError) -> Self {
        Self::Plan(value)
    }
}

impl From<SoftwareAuditError> for SoftwareExecutionError {
    fn from(value: SoftwareAuditError) -> Self {
        Self::Audit(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrashPoint {
    None,
    BeforeDispatchStarted,
    AfterDispatchStarted,
    AfterAdapterCompleted,
}

#[derive(Clone)]
pub struct SoftwareExecutor {
    adapter: Arc<dyn MsixRemovalAdapter>,
    requery: Arc<dyn InstalledStateRequery>,
    live_inventory: Arc<dyn LiveSoftwareInventory>,
    audit_path: Option<PathBuf>,
    timing: RemovalTiming,
    requery_offsets: [Duration; 3],
    crash_point: CrashPoint,
}

impl Default for SoftwareExecutor {
    fn default() -> Self {
        Self {
            adapter: Arc::new(RealMsixAdapter),
            requery: Arc::new(RealInstalledStateRequery),
            live_inventory: Arc::new(RealLiveSoftwareInventory),
            audit_path: None,
            timing: RemovalTiming::default(),
            requery_offsets: MSIX_REQUERY_OFFSETS,
            crash_point: CrashPoint::None,
        }
    }
}

static EXECUTION_PERMIT: OnceLock<Mutex<()>> = OnceLock::new();
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

impl SoftwareExecutor {
    pub fn execute(
        &self,
        request: SoftwareExecutionRequest<'_>,
    ) -> Result<SoftwareExecutionReportV1, SoftwareExecutionError> {
        if !request.confirmed {
            return Err(SoftwareExecutionError::ConfirmationRequired);
        }
        let _permit = EXECUTION_PERMIT
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| SoftwareExecutionError::PermitPoisoned)?;
        let path = match &self.audit_path {
            Some(path) => path.clone(),
            None => software_audit_v1_path()?,
        };
        let mut journal = SoftwareAuditJournal::open(&path)?;
        self.recover_pending(&mut journal)?;
        // Live authority is acquired behind both single-flight boundaries and
        // after pending dispatches are reconciled. Public callers cannot
        // substitute a deserialized inventory or race recovery with a stale
        // snapshot.
        let live_inventory = self.live_inventory.inventory()?;
        let actions = construct_validated_actions(
            request.plan,
            &live_inventory,
            request.expected_preview_digest,
            unix_ms(SystemTime::now()),
        )?;

        let mut outcomes = Vec::with_capacity(actions.len());
        for action in &actions {
            outcomes.push(self.execute_one(&mut journal, action, request.cancel.as_ref())?);
        }
        Ok(SoftwareExecutionReportV1 {
            version: SOFTWARE_EXECUTION_VERSION,
            irreversible: true,
            outcomes,
        })
    }

    /// Reconciles durable nonterminal operations before accepting a new
    /// Software action. Recovery only re-queries identity and never calls the
    /// removal adapter.
    pub fn recover_startup(&self) -> Result<Vec<SoftwareActionOutcomeV1>, SoftwareExecutionError> {
        let _permit = EXECUTION_PERMIT
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| SoftwareExecutionError::PermitPoisoned)?;
        let path = match &self.audit_path {
            Some(path) => path.clone(),
            None => software_audit_v1_path()?,
        };
        let mut journal = SoftwareAuditJournal::open(&path)?;
        self.recover_pending(&mut journal)
    }

    fn execute_one(
        &self,
        journal: &mut SoftwareAuditJournal,
        action: &ValidatedSoftwareAction,
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<SoftwareActionOutcomeV1, SoftwareExecutionError> {
        let operation_id = next_operation_id();
        let context = AuditActionContext::from(action);
        journal.append(record(
            &operation_id,
            context,
            audit_event(
                SoftwareAuditTransition::Validated,
                SoftwareAuditStatusCode::Validated,
                None,
                SoftwareRebootEvidence::None,
                None,
                None,
            ),
        ))?;

        if self.crash_point == CrashPoint::BeforeDispatchStarted {
            return Err(SoftwareExecutionError::CrashInjected(
                "before_dispatch_started",
            ));
        }
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            let outcome = SoftwareExecutionOutcome::CanceledBeforeStart;
            journal.append(record(
                &operation_id,
                context,
                audit_event(
                    SoftwareAuditTransition::Terminal { outcome },
                    SoftwareAuditStatusCode::CanceledBeforeStart,
                    None,
                    SoftwareRebootEvidence::None,
                    None,
                    None,
                ),
            ))?;
            return Ok(action_outcome(
                &operation_id,
                context,
                outcome,
                None,
                SoftwareRebootEvidence::None,
                None,
            ));
        }

        // This flush+sync is the durable line immediately before the WinRT
        // side-effect boundary.
        journal.append(record(
            &operation_id,
            context,
            audit_event(
                SoftwareAuditTransition::DispatchStarted,
                SoftwareAuditStatusCode::DispatchStarted,
                None,
                SoftwareRebootEvidence::None,
                None,
                None,
            ),
        ))?;
        if self.crash_point == CrashPoint::AfterDispatchStarted {
            return Err(SoftwareExecutionError::CrashInjected(
                "after_dispatch_started",
            ));
        }

        let adapter = self.adapter.remove(action, cancel, self.timing);
        if adapter.outcome != AdapterOutcome::Unfinished {
            journal.append(record(
                &operation_id,
                context,
                audit_event(
                    SoftwareAuditTransition::AdapterCompleted,
                    adapter_status(adapter.outcome),
                    adapter.error_code,
                    adapter.reboot_evidence,
                    None,
                    Some(adapter.outcome),
                ),
            ))?;
        }
        if self.crash_point == CrashPoint::AfterAdapterCompleted {
            return Err(SoftwareExecutionError::CrashInjected(
                "after_adapter_completed",
            ));
        }

        let installed = self.requery_and_record(journal, &operation_id, context)?;
        let outcome = classify_terminal(adapter, installed.state);
        let terminal_error = terminal_error_code(outcome, adapter, installed);
        journal.append(record(
            &operation_id,
            context,
            audit_event(
                SoftwareAuditTransition::Terminal { outcome },
                terminal_status(outcome),
                terminal_error,
                adapter.reboot_evidence,
                Some(installed.state),
                Some(adapter.outcome),
            ),
        ))?;
        Ok(action_outcome(
            &operation_id,
            context,
            outcome,
            Some(installed.state),
            adapter.reboot_evidence,
            terminal_error,
        ))
    }

    fn recover_pending(
        &self,
        journal: &mut SoftwareAuditJournal,
    ) -> Result<Vec<SoftwareActionOutcomeV1>, SoftwareExecutionError> {
        let pending = journal.pending_operations();
        let mut recovered = Vec::with_capacity(pending.len());
        for pending in pending {
            let SoftwareIdentity::Msix { package_full_name } = &pending.identity else {
                return Err(SoftwareAuditError::InvalidTransition {
                    operation_id: pending.operation_id,
                    detail: "non-MSIX recovery identity",
                }
                .into());
            };
            let software_id = super::software_id(&pending.identity).map_err(|_| {
                SoftwareAuditError::InvalidTransition {
                    operation_id: pending.operation_id.clone(),
                    detail: "recovery identity cannot be hashed",
                }
            })?;
            let context = AuditActionContext {
                software_id: &software_id,
                identity: &pending.identity,
                package_full_name,
                inventory_fingerprint: &pending.inventory_fingerprint,
                preview_digest: &pending.preview_digest,
            };
            if pending.last_transition == SoftwareAuditTransition::Validated {
                let outcome = SoftwareExecutionOutcome::CanceledBeforeStart;
                journal.append(record(
                    &pending.operation_id,
                    context,
                    audit_event(
                        SoftwareAuditTransition::Terminal { outcome },
                        SoftwareAuditStatusCode::RecoveredBeforeDispatch,
                        Some(SoftwareAuditErrorCode::RecoveredAfterCrash),
                        SoftwareRebootEvidence::None,
                        None,
                        None,
                    ),
                ))?;
                recovered.push(action_outcome(
                    &pending.operation_id,
                    context,
                    outcome,
                    None,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareAuditErrorCode::RecoveredAfterCrash),
                ));
                continue;
            }

            let adapter = AdapterEvidence {
                outcome: pending
                    .adapter_outcome
                    .unwrap_or(AdapterOutcome::Unfinished),
                reboot_evidence: pending.reboot_evidence,
                error_code: pending
                    .adapter_outcome
                    .is_none()
                    .then_some(SoftwareAuditErrorCode::RecoveredAfterCrash),
            };
            let installed = self.requery_and_record(journal, &pending.operation_id, context)?;
            let outcome = classify_terminal(adapter, installed.state);
            let terminal_error = terminal_error_code(outcome, adapter, installed).or_else(|| {
                (!outcome.is_success()).then_some(SoftwareAuditErrorCode::RecoveredAfterCrash)
            });
            journal.append(record(
                &pending.operation_id,
                context,
                audit_event(
                    SoftwareAuditTransition::Terminal { outcome },
                    terminal_status(outcome),
                    terminal_error,
                    adapter.reboot_evidence,
                    Some(installed.state),
                    Some(adapter.outcome),
                ),
            ))?;
            recovered.push(action_outcome(
                &pending.operation_id,
                context,
                outcome,
                Some(installed.state),
                adapter.reboot_evidence,
                terminal_error,
            ));
        }
        Ok(recovered)
    }

    fn requery_and_record(
        &self,
        journal: &mut SoftwareAuditJournal,
        operation_id: &str,
        context: AuditActionContext<'_>,
    ) -> Result<RequeryObservation, SoftwareExecutionError> {
        let started = Instant::now();
        let mut final_observation = RequeryObservation::unavailable();
        for offset in self.requery_offsets {
            let elapsed = started.elapsed();
            if offset > elapsed {
                std::thread::sleep(offset - elapsed);
            }
            let observation = self.requery.query(context.package_full_name);
            if merge_requery_state(Some(final_observation.state), observation.state)
                == observation.state
            {
                final_observation = observation;
            }
            journal.append(record(
                operation_id,
                context,
                audit_event(
                    SoftwareAuditTransition::RequeryObserved,
                    requery_status(observation.state),
                    observation.error_code,
                    SoftwareRebootEvidence::None,
                    Some(observation.state),
                    None,
                ),
            ))?;
        }
        Ok(final_observation)
    }

    #[cfg(test)]
    fn for_test(
        path: PathBuf,
        adapter: Arc<dyn MsixRemovalAdapter>,
        requery: Arc<dyn InstalledStateRequery>,
        live_inventory: Arc<dyn LiveSoftwareInventory>,
    ) -> Self {
        Self {
            adapter,
            requery,
            live_inventory,
            audit_path: Some(path),
            timing: RemovalTiming {
                monitor_timeout: Duration::from_millis(20),
                cancel_grace: Duration::from_millis(5),
                poll_interval: Duration::from_millis(1),
            },
            requery_offsets: [Duration::ZERO; 3],
            crash_point: CrashPoint::None,
        }
    }
}

pub(super) fn merge_requery_state(
    current: Option<SoftwareInstalledState>,
    next: SoftwareInstalledState,
) -> SoftwareInstalledState {
    use SoftwareInstalledState as State;

    match current {
        None | Some(State::Unavailable) => next,
        Some(State::Conflicting) => State::Conflicting,
        Some(State::Present | State::Absent) if next == State::Conflicting => State::Conflicting,
        Some(State::Present | State::Absent) if matches!(next, State::Present | State::Absent) => {
            next
        }
        Some(current) => current,
    }
}

fn construct_validated_actions(
    plan: &SoftwareSelectionPlanV1,
    live_inventory: &SoftwareInventoryV1,
    expected_preview_digest: &str,
    now_unix_ms: u64,
) -> Result<Vec<ValidatedSoftwareAction>, SoftwarePlanError> {
    let preview = preview_selection_plan_live(plan, live_inventory, now_unix_ms)?;
    validate_preview_digest(&preview, expected_preview_digest)?;
    preview
        .selected
        .iter()
        .map(|item| {
            if item.action_class != SoftwareActionClass::RemoveCurrentUserMsix
                || item.scope != SoftwareScope::CurrentUser
                || item.eligibility != SoftwareEligibilityReason::EligibleCurrentUserMsix
            {
                return Err(SoftwarePlanError::InvalidSelectedIdentity(item.id.clone()));
            }
            let live = live_inventory
                .entries
                .iter()
                .find(|entry| entry.id == item.id)
                .ok_or_else(|| SoftwarePlanError::StaleSelection(item.id.clone()))?;
            let SoftwareIdentity::Msix { package_full_name } = &live.identity else {
                return Err(SoftwarePlanError::InvalidSelectedIdentity(item.id.clone()));
            };
            if item.identity != live.identity {
                return Err(SoftwarePlanError::StaleSelection(item.id.clone()));
            }
            if !super::plan::valid_package_full_name(package_full_name) {
                return Err(SoftwarePlanError::InvalidSelectedIdentity(item.id.clone()));
            }
            Ok(ValidatedSoftwareAction {
                id: item.id.clone(),
                identity: live.identity.clone(),
                package_full_name: package_full_name.clone(),
                inventory_fingerprint: preview.inventory_fingerprint.clone(),
                preview_digest: preview.digest.clone(),
            })
        })
        .collect()
}

pub(super) fn classify_terminal(
    adapter: AdapterEvidence,
    installed: SoftwareInstalledState,
) -> SoftwareExecutionOutcome {
    use AdapterOutcome as Adapter;
    use SoftwareExecutionOutcome as Outcome;
    use SoftwareInstalledState as Installed;

    if installed == Installed::Absent && adapter.reboot_evidence == SoftwareRebootEvidence::Required
    {
        Outcome::RebootRequired
    } else if installed == Installed::Absent {
        Outcome::Removed
    } else if adapter.reboot_evidence == SoftwareRebootEvidence::Required {
        Outcome::RebootRequired
    } else if adapter.outcome == Adapter::Unfinished
        || matches!(installed, Installed::Unavailable | Installed::Conflicting)
    {
        Outcome::UnknownAfterDispatch
    } else if installed == Installed::Present && adapter.outcome == Adapter::Failure {
        Outcome::Failed
    } else if installed == Installed::Present && adapter.outcome == Adapter::Success {
        Outcome::StillPresent
    } else {
        Outcome::UnknownAfterDispatch
    }
}

fn record(
    operation_id: &str,
    context: AuditActionContext<'_>,
    event: SoftwareAuditEvent,
) -> SoftwareAuditRecordV1 {
    SoftwareAuditRecordV1::new(
        operation_id,
        unix_ms(SystemTime::now()),
        context.identity,
        context.inventory_fingerprint,
        context.preview_digest,
        event,
    )
}

fn audit_event(
    transition: SoftwareAuditTransition,
    status_code: SoftwareAuditStatusCode,
    error_code: Option<SoftwareAuditErrorCode>,
    reboot_evidence: SoftwareRebootEvidence,
    installed_state: Option<SoftwareInstalledState>,
    adapter_outcome: Option<AdapterOutcome>,
) -> SoftwareAuditEvent {
    SoftwareAuditEvent {
        transition,
        status_code,
        error_code,
        reboot_evidence,
        installed_state,
        adapter_outcome,
    }
}

fn action_outcome(
    operation_id: &str,
    context: AuditActionContext<'_>,
    outcome: SoftwareExecutionOutcome,
    installed_state: Option<SoftwareInstalledState>,
    reboot_evidence: SoftwareRebootEvidence,
    error_code: Option<SoftwareAuditErrorCode>,
) -> SoftwareActionOutcomeV1 {
    SoftwareActionOutcomeV1 {
        operation_id: operation_id.to_string(),
        software_id: context.software_id.to_string(),
        outcome,
        installed_state,
        reboot_evidence,
        error_code,
        irreversible: true,
    }
}

fn adapter_status(outcome: AdapterOutcome) -> SoftwareAuditStatusCode {
    match outcome {
        AdapterOutcome::Success => SoftwareAuditStatusCode::AdapterSucceeded,
        AdapterOutcome::Failure => SoftwareAuditStatusCode::AdapterFailed,
        AdapterOutcome::RebootRequired => SoftwareAuditStatusCode::AdapterRebootRequired,
        AdapterOutcome::Unfinished => SoftwareAuditStatusCode::UnknownAfterDispatch,
    }
}

fn requery_status(state: SoftwareInstalledState) -> SoftwareAuditStatusCode {
    match state {
        SoftwareInstalledState::Present => SoftwareAuditStatusCode::RequeryPresent,
        SoftwareInstalledState::Absent => SoftwareAuditStatusCode::RequeryAbsent,
        SoftwareInstalledState::Unavailable => SoftwareAuditStatusCode::RequeryUnavailable,
        SoftwareInstalledState::Conflicting => SoftwareAuditStatusCode::RequeryConflicting,
    }
}

fn terminal_status(outcome: SoftwareExecutionOutcome) -> SoftwareAuditStatusCode {
    match outcome {
        SoftwareExecutionOutcome::CanceledBeforeStart => {
            SoftwareAuditStatusCode::CanceledBeforeStart
        }
        SoftwareExecutionOutcome::Removed => SoftwareAuditStatusCode::Removed,
        SoftwareExecutionOutcome::RebootRequired => SoftwareAuditStatusCode::RebootRequired,
        SoftwareExecutionOutcome::StillPresent => SoftwareAuditStatusCode::StillPresent,
        SoftwareExecutionOutcome::Failed => SoftwareAuditStatusCode::Failed,
        SoftwareExecutionOutcome::UnknownAfterDispatch => {
            SoftwareAuditStatusCode::UnknownAfterDispatch
        }
    }
}

fn terminal_error_code(
    outcome: SoftwareExecutionOutcome,
    adapter: AdapterEvidence,
    requery: RequeryObservation,
) -> Option<SoftwareAuditErrorCode> {
    if matches!(
        outcome,
        SoftwareExecutionOutcome::Removed | SoftwareExecutionOutcome::RebootRequired
    ) {
        None
    } else {
        requery.error_code.or(adapter.error_code)
    }
}

fn next_operation_id() -> String {
    let sequence = OPERATION_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!(
        "software-op-{:016x}-{:08x}-{sequence:016x}",
        unix_ms(SystemTime::now()),
        std::process::id()
    )
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |duration| {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
