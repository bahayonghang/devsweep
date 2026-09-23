use devsweep_core::{
    execution::{ConfirmationDigest, ExecutionReport, ExecutionRequest, Executor},
    model::{TargetId, UntrustedPlan},
    plan::validate_plan,
};
use serde::Serialize;

use crate::error::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct DryRunOutcome {
    pub report: ExecutionReport,
    pub digest: ConfirmationDigest,
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

pub(crate) fn plan_dry_run_inner(
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
    plan: UntrustedPlan,
    selected_ids: Vec<TargetId>,
    digest: ConfirmationDigest,
) -> Result<ExecutionReport, CommandError> {
    tauri::async_runtime::spawn_blocking(move || plan_execute_inner(plan, selected_ids, digest))
        .await
        .map_err(CommandError::io)?
}

pub(crate) fn plan_execute_inner(
    plan: UntrustedPlan,
    selected_ids: Vec<TargetId>,
    digest: ConfirmationDigest,
) -> Result<ExecutionReport, CommandError> {
    let validated = validate_plan(&plan).map_err(CommandError::invalid_plan)?;
    Executor::default()
        .run_plan(
            &validated,
            ExecutionRequest {
                execute: true,
                audit_log: None,
                selected: selected_ids,
                expected_digest: Some(digest),
                cancel: None,
            },
        )
        .map_err(CommandError::execution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::model::CLEANUP_PLAN_VERSION;

    #[test]
    fn empty_plan_dry_run_does_not_require_audit() {
        let outcome = plan_dry_run_inner(UntrustedPlan::empty(), Vec::new()).expect("dry run");
        assert!(outcome.report.dry_run);
        assert!(outcome.report.audit_log.is_none());
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
    fn unknown_target_is_structured() {
        let error = plan_dry_run_inner(UntrustedPlan::empty(), vec![TargetId::new("unknown")])
            .expect_err("unknown selection is rejected");
        assert!(matches!(error, CommandError::UnknownTarget { .. }));
        let _ = CLEANUP_PLAN_VERSION;
    }
}
