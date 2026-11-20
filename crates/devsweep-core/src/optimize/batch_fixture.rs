//! Deterministic adapter for isolated CLI and native-window acceptance.
use super::{
    LivePreflight, MaintenanceCatalogueEntryV1, MaintenanceExecutor, OptimizeAuditErrorCode,
    OptimizePlanError, PrecheckReason, ResolvedMaintenanceAction, ValidatedMaintenanceAction,
    execution::{AdapterEvidence, DispatchTiming, MaintenanceDispatch},
};
use crate::process::FlagCancelObserver;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
struct Isolated;
impl LivePreflight for Isolated {
    fn resolve(
        &self,
        entry: &MaintenanceCatalogueEntryV1,
    ) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
        match entry.id {
            "dns.flush" | "maintenance.clock_probe" => {
                Ok(ResolvedMaintenanceAction::NativeCommand {
                    program: PathBuf::from("isolated-unlaunchable"),
                    argv: vec![entry.id.to_string()],
                })
            }
            _ => Err(OptimizePlanError::PrecheckRefused {
                operation_id: entry.id.to_string(),
                reason: PrecheckReason::UacDeclined,
            }),
        }
    }
}
impl MaintenanceDispatch for Isolated {
    fn dispatch(
        &self,
        action: &ValidatedMaintenanceAction,
        _: Option<&Arc<FlagCancelObserver>>,
        _: DispatchTiming,
    ) -> AdapterEvidence {
        if action.operation_id == "maintenance.clock_probe" {
            AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterOperationFailed)
        } else {
            AdapterEvidence::success()
        }
    }
}
impl MaintenanceExecutor {
    /// This fixture never delegates to a real preflight or dispatch implementation.
    pub fn isolated_batch_fixture(audit_path: &Path) -> Self {
        Self::for_test(
            audit_path.to_owned(),
            Arc::new(Isolated),
            Arc::new(Isolated),
            DispatchTiming::default(),
        )
    }
}
