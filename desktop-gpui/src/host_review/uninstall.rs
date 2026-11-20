//! One exact current-user registration, reviewed through the core uninstall pipeline.
use super::{ActionResult, Intent};
use crate::copy::{self, Locale};
use devsweep_core::{process::FlagCancelObserver, software::*};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UninstallSelection {
    pub id: String,
    pub name: String,
    target: Target,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Msix(String),
    Msi {
        product_code: String,
        registration: MsiRegistrationEvidenceV1,
    },
}

impl UninstallSelection {
    pub(crate) fn from_entry(entry: &SoftwareEntryV1) -> Result<Self, String> {
        let refusal = match entry.eligibility.reason {
            SoftwareEligibilityReason::MsiRequiresAdministrator => "requires_administrator",
            SoftwareEligibilityReason::MsiContextUnsupported => "msi_context_unsupported",
            SoftwareEligibilityReason::MsiAuthorityUnavailable => "msi_authority_unavailable",
            SoftwareEligibilityReason::ConflictingIdentity => "conflicting_identity",
            SoftwareEligibilityReason::ProtectedProduct => "protected",
            _ => "software_not_eligible",
        };
        if !entry.eligibility.is_selectable()
            || entry.scope != SoftwareScope::CurrentUser
            || entry.launch.is_some()
        {
            return Err(refusal.into());
        }
        let target = match (&entry.identity, entry.eligibility.reason) {
            (
                SoftwareIdentity::Msix { package_full_name },
                SoftwareEligibilityReason::EligibleCurrentUserMsix,
            ) => Target::Msix(package_full_name.clone()),
            (
                SoftwareIdentity::Msi {
                    product_code,
                    context: MsiContext::UserUnmanaged,
                },
                SoftwareEligibilityReason::EligibleCurrentUserMsi,
            ) => Target::Msi {
                product_code: product_code.clone(),
                registration: entry
                    .msi_registration
                    .clone()
                    .ok_or("msi_authority_unavailable")?,
            },
            _ => return Err(refusal.into()),
        };
        if software_entry_id(&entry.identity).map_err(|_| "invalid_identity")? != entry.id {
            return Err("invalid_identity".into());
        }
        Ok(Self {
            id: entry.id.clone(),
            name: entry
                .display_name
                .clone()
                .unwrap_or_else(|| entry.id.clone()),
            target,
        })
    }

    pub(super) fn identity_label(&self) -> &str {
        match &self.target {
            Target::Msix(name) => name,
            Target::Msi { product_code, .. } => product_code,
        }
    }

    pub(crate) fn allows_leftovers(&self, outcome: &SoftwareActionOutcomeV1) -> bool {
        if outcome.operation_id.is_empty()
            || outcome.software_id != self.id
            || outcome.outcome != SoftwareExecutionOutcome::Removed
            || outcome.installed_state != Some(SoftwareInstalledState::Absent)
        {
            return false;
        }
        match (&self.target, &outcome.msi) {
            (Target::Msix(_), None) => true,
            (Target::Msi { registration, .. }, Some(evidence)) => {
                evidence.user_sid == registration.user_sid
                    && evidence.process
                        == Some(MsiProcessEvidenceV1 {
                            exit_code: Some(0),
                            interruption: None,
                        })
            }
            _ => false,
        }
    }

    pub(super) fn source(&self) -> SoftwareInventorySource {
        match self.target {
            Target::Msix(_) => SoftwareInventorySource::Msix,
            Target::Msi { .. } => SoftwareInventorySource::Msi,
        }
    }

    pub(crate) fn preview(
        &self,
        inventory: &SoftwareInventoryV1,
        now: u64,
    ) -> Result<(SoftwareSelectionPlanV1, String), String> {
        let plan = build_selection_plan(inventory, std::slice::from_ref(&self.id), now)
            .map_err(plan_error)?;
        let preview = preview_selection_plan_live(&plan, inventory, now).map_err(plan_error)?;
        let [item] = preview.selected.as_slice() else {
            return Err("invalid_plan".into());
        };
        let live = inventory
            .entries
            .iter()
            .find(|entry| entry.id == self.id)
            .ok_or("stale_confirmation")?;
        let action = match self.target {
            Target::Msix(_) => SoftwareActionClass::RemoveCurrentUserMsix,
            Target::Msi { .. } => SoftwareActionClass::RemoveMsiProduct,
        };
        if Self::from_entry(live)? != *self || item.id != self.id || item.action_class != action {
            return Err("stale_confirmation".into());
        }
        Ok((plan, preview.digest))
    }

    pub(crate) fn execute(
        &self,
        plan: &SoftwareSelectionPlanV1,
        digest: &str,
        confirmed: bool,
        cancel: Arc<FlagCancelObserver>,
        dispatch: impl FnOnce(
            SoftwareExecutionRequest<'_>,
        ) -> Result<SoftwareExecutionReportV1, SoftwareExecutionError>,
    ) -> Result<ActionResult, String> {
        if !confirmed {
            return Err("confirmation_required".into());
        }
        if plan.selected_ids.as_slice() != std::slice::from_ref(&self.id) {
            return Err("invalid_plan".into());
        }
        let report = dispatch(SoftwareExecutionRequest {
            plan,
            expected_preview_digest: digest,
            confirmed,
            cancel: Some(cancel),
        })
        .map_err(execution_error)?;
        let mut outcomes = report.outcomes.into_iter();
        let outcome = outcomes
            .next()
            .filter(|outcome| {
                outcome.software_id == self.id
                    && match (&self.target, &outcome.msi) {
                        (Target::Msix(_), None) => true,
                        (Target::Msi { registration, .. }, Some(evidence)) => {
                            evidence.user_sid == registration.user_sid
                        }
                        (Target::Msi { .. }, None) => {
                            outcome.outcome == SoftwareExecutionOutcome::CanceledBeforeStart
                        }
                        _ => false,
                    }
            })
            .ok_or("unknown_after_dispatch")?;
        if outcomes.next().is_some() {
            return Err("unknown_after_dispatch".into());
        }
        let code = match outcome.outcome {
            SoftwareExecutionOutcome::Removed => "removed",
            SoftwareExecutionOutcome::RebootRequired => "restart_required",
            SoftwareExecutionOutcome::StillPresent => "still_present",
            SoftwareExecutionOutcome::Failed => "failed",
            SoftwareExecutionOutcome::CanceledBeforeStart => "canceled_before_start",
            SoftwareExecutionOutcome::UnknownAfterDispatch => "unknown_after_dispatch",
        };
        Ok(ActionResult {
            intent: Intent::Uninstall(self.clone()),
            code: code.into(),
            reverse: None,
            uninstall: Some(outcome),
        })
    }
}

fn plan_error(error: SoftwarePlanError) -> String {
    match error {
        SoftwarePlanError::ManualSelection(_) => "software_not_eligible",
        SoftwarePlanError::InventoryExpired
        | SoftwarePlanError::DigestMismatch
        | SoftwarePlanError::StaleSelection(_)
        | SoftwarePlanError::UnknownSelection(_)
        | SoftwarePlanError::InvalidSelectedIdentity(_) => "stale_confirmation",
        _ => "invalid_plan",
    }
    .into()
}

fn execution_error(error: SoftwareExecutionError) -> String {
    match error {
        SoftwareExecutionError::Plan(error) => return plan_error(error),
        SoftwareExecutionError::ConfirmationRequired => "confirmation_required",
        SoftwareExecutionError::LiveRevalidationUnavailable => "software_revalidation_unavailable",
        SoftwareExecutionError::Audit(_) => "audit_unavailable",
        SoftwareExecutionError::AuditAfterDispatch(_)
        | SoftwareExecutionError::CrashInjected(_) => "unknown_after_dispatch",
        SoftwareExecutionError::PermitPoisoned => "worker_unavailable",
    }
    .into()
}

pub(super) fn evidence_lines(item: &SoftwareActionOutcomeV1, locale: Locale) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(code) = item
        .msi
        .as_ref()
        .and_then(|evidence| evidence.process)
        .and_then(|process| process.exit_code)
    {
        let label = match code {
            1641 => copy::pick(
                locale,
                "Windows Installer initiated a restart",
                "Windows Installer 已发起重启",
            ),
            3010 => copy::pick(
                locale,
                "Windows Installer requires a restart",
                "Windows Installer 要求重启",
            ),
            _ => copy::pick(
                locale,
                "Windows Installer exit code",
                "Windows Installer 退出码",
            ),
        };
        lines.push(format!("{label} · {code}"));
    }
    if let Some(state) = item.installed_state {
        lines.push(
            match state {
                SoftwareInstalledState::Present => copy::pick(
                    locale,
                    "Windows still lists this package.",
                    "Windows 仍列出此软件包。",
                ),
                SoftwareInstalledState::Absent => copy::pick(
                    locale,
                    "Windows no longer lists this package.",
                    "Windows 已不再列出此软件包。",
                ),
                SoftwareInstalledState::Unavailable => copy::pick(
                    locale,
                    "Windows could not recheck the package.",
                    "Windows 无法重查此软件包。",
                ),
                SoftwareInstalledState::Conflicting => copy::pick(
                    locale,
                    "Windows returned conflicting installation states.",
                    "Windows 返回了相互矛盾的安装状态。",
                ),
            }
            .into(),
        );
    }
    if item.reboot_evidence == SoftwareRebootEvidence::Required
        && item.outcome != SoftwareExecutionOutcome::RebootRequired
    {
        lines.push(copy::pick(locale, "Windows requested a restart.", "Windows 要求重启。").into());
    }
    if let Some(error) = item.error_code {
        lines.push(
            match error {
                SoftwareAuditErrorCode::AdapterDispatchFailed => copy::pick(
                    locale,
                    "Windows could not start removal.",
                    "Windows 无法开始卸载。",
                ),
                SoftwareAuditErrorCode::AdapterOperationFailed => copy::pick(
                    locale,
                    "Windows reported a removal error.",
                    "Windows 报告卸载错误。",
                ),
                SoftwareAuditErrorCode::AdapterStatusUnavailable => copy::pick(
                    locale,
                    "Windows did not return a final result.",
                    "Windows 未返回最终结果。",
                ),
                SoftwareAuditErrorCode::AdapterTimedOut => copy::pick(
                    locale,
                    "Waiting for removal timed out.",
                    "等待卸载结果超时。",
                ),
                SoftwareAuditErrorCode::AdapterCanceledAfterDispatch => copy::pick(
                    locale,
                    "Cancellation was requested after dispatch.",
                    "派发后已请求取消。",
                ),
                SoftwareAuditErrorCode::AdapterCancelFailed => copy::pick(
                    locale,
                    "Windows could not confirm cancellation.",
                    "Windows 无法确认取消。",
                ),
                SoftwareAuditErrorCode::RequeryUnavailable
                | SoftwareAuditErrorCode::RequeryConflicting => return lines,
                SoftwareAuditErrorCode::RecoveredAfterCrash => copy::pick(
                    locale,
                    "Recovered from the software history after interruption.",
                    "中断后已从软件历史记录恢复。",
                ),
            }
            .into(),
        );
    }
    lines
}
