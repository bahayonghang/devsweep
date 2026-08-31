//! Closed Optimize V1 service boundary: the exhaustive maintenance catalogue,
//! live-preflight preview identity, one-operation execution, and the locked
//! durable audit.
//!
//! This domain never creates a [`crate::model::CleanupPlan`], never executes
//! anything outside the eight-id catalogue, and never elevates. Guidance rows
//! have no dispatch representation anywhere in this module tree.

mod audit;
mod catalogue;
mod execution;
mod plan;
#[cfg(test)]
mod tests;
mod windows;

pub use audit::{
    OPTIMIZE_AUDIT_VERSION, OptimizeAuditError, OptimizeAuditErrorCode, OptimizeAuditRecordV1,
    OptimizeAuditStatusCode, OptimizeAuditTransition, optimize_audit_v1_path,
};
pub use catalogue::{
    DNS_FLUSH_ID, GUIDANCE_DRIVE_OPTIMIZE_ID, GUIDANCE_FILESYSTEM_CHECK_ID,
    GUIDANCE_NETWORK_RESET_ID, GUIDANCE_SYSTEM_INTEGRITY_ID, MaintenanceActionClass,
    MaintenanceCatalogueEntryV1, OPTIMIZE_CATALOGUE_VERSION, SETTINGS_ENERGY_RECOMMENDATIONS_ID,
    SETTINGS_SEARCH_ID, SETTINGS_STORAGE_RECOMMENDATIONS_ID, catalogue_entries, catalogue_entry,
};
pub use execution::{
    DNS_FLUSH_TIMEOUT, MaintenanceActionOutcomeV1, MaintenanceExecutionError,
    MaintenanceExecutionOutcome, MaintenanceExecutionReportV1, MaintenanceExecutionRequest,
    MaintenanceExecutor, OPTIMIZE_EXECUTION_VERSION,
};
pub use plan::{
    MaintenancePlanV1, MaintenancePreviewV1, OPTIMIZE_PLAN_VERSION, OPTIMIZE_PREVIEW_VERSION,
    OptimizePlanError, ValidatedMaintenanceAction, plan_operation, validate_preview_digest,
};

pub(super) use plan::ResolvedMaintenanceAction;

/// The live preflight contract: resolve one catalogue row into its fixed
/// dispatch identity, or fail closed with a typed plan error.
pub(super) trait LivePreflight: Send + Sync {
    fn resolve(
        &self,
        entry: &MaintenanceCatalogueEntryV1,
    ) -> Result<ResolvedMaintenanceAction, OptimizePlanError>;
}
/// Revalidates a saved plan against the closed catalogue and the real platform
/// preflight, resolving exactly one non-deserializable action identity.
pub fn preview_maintenance_plan_live(
    plan: &MaintenancePlanV1,
) -> Result<MaintenancePreviewV1, OptimizePlanError> {
    plan::preview_maintenance_plan(plan, &windows::RealLivePreflight)
}
