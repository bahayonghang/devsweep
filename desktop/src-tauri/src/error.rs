use devsweep_core::{
    execution::ExecutionError,
    optimize::{MaintenanceExecutionError, OptimizeAuditError, OptimizePlanError},
    software::{SoftwareAuditError, SoftwareExecutionError, SoftwarePlanError},
};
use serde::Serialize;

use devsweep_core::{execution::ConfirmationDigest, model::TargetId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub(crate) enum CommandError {
    ScanAlreadyRunning,
    ScanFailed {
        message: String,
    },
    AnalyzeAlreadyRunning,
    AnalyzeFailed {
        message: String,
    },
    SoftwareAlreadyRunning,
    SoftwareFailed {
        message: String,
    },
    SoftwareStaleAuthority {
        message: String,
    },
    SoftwareAuditUnavailable {
        message: String,
    },
    OptimizeAlreadyRunning,
    OptimizeFailed {
        message: String,
    },
    OptimizeStaleAuthority {
        message: String,
    },
    OptimizeUnavailable {
        message: String,
    },
    OptimizeAuditUnavailable {
        message: String,
    },
    StatusAlreadyRunning,
    StatusFailed {
        message: String,
    },
    InvalidPlan {
        issues: Vec<String>,
    },
    StaleConfirmation {
        expected_digest: ConfirmationDigest,
        actual_digest: ConfirmationDigest,
    },
    UnknownTarget {
        target_id: TargetId,
    },
    InspectOnlyTarget {
        target_id: TargetId,
    },
    Io {
        message: String,
    },
    ProtectionStoreUnavailable {
        message: String,
    },
    ProtectionAuditUnknown {
        message: String,
    },
    ProtectionPathMissing {
        message: String,
    },
    ProtectionConfirmationRequired {
        message: String,
    },
    HistoryStoreUnavailable {
        message: String,
    },
    HistoryNotFound {
        message: String,
    },
    RuleNotFound {
        message: String,
    },
}

impl CommandError {
    pub(crate) fn invalid_plan(error: anyhow::Error) -> Self {
        Self::InvalidPlan {
            issues: vec![format!("{error:#}")],
        }
    }

    pub(crate) fn scan_failed(error: anyhow::Error) -> Self {
        Self::ScanFailed {
            message: format!("{error:#}"),
        }
    }

    pub(crate) fn analyze_failed(error: anyhow::Error) -> Self {
        Self::AnalyzeFailed {
            message: format!("{error:#}"),
        }
    }

    pub(crate) fn software_failed(error: anyhow::Error) -> Self {
        Self::SoftwareFailed {
            message: format!("{error:#}"),
        }
    }

    pub(crate) fn optimize_failed(error: anyhow::Error) -> Self {
        Self::OptimizeFailed {
            message: format!("{error:#}"),
        }
    }

    pub(crate) fn optimize(error: anyhow::Error) -> Self {
        if let Some(error) = error.downcast_ref::<MaintenanceExecutionError>() {
            return match error {
                MaintenanceExecutionError::Plan(plan) => Self::optimize_plan(plan),
                MaintenanceExecutionError::Audit(
                    OptimizeAuditError::LockUnavailable(_)
                    | OptimizeAuditError::LocalAppDataUnavailable,
                ) => Self::OptimizeAuditUnavailable {
                    message: error.to_string(),
                },
                _ => Self::OptimizeFailed {
                    message: error.to_string(),
                },
            };
        }
        if let Some(error) = error.downcast_ref::<OptimizePlanError>() {
            return Self::optimize_plan(error);
        }
        Self::optimize_failed(error)
    }

    fn optimize_plan(error: &OptimizePlanError) -> Self {
        match error {
            OptimizePlanError::UnsupportedPlanVersion(_)
            | OptimizePlanError::UnsupportedCatalogueVersion(_)
            | OptimizePlanError::DigestMismatch => Self::OptimizeStaleAuthority {
                message: error.to_string(),
            },
            OptimizePlanError::PlatformUnsupported
            | OptimizePlanError::OsBuildUnavailable
            | OptimizePlanError::BuildUnsupported { .. }
            | OptimizePlanError::ResolverUnavailable => Self::OptimizeUnavailable {
                message: error.to_string(),
            },
            _ => Self::OptimizeFailed {
                message: error.to_string(),
            },
        }
    }

    pub(crate) fn software(error: anyhow::Error) -> Self {
        if let Some(error) = error.downcast_ref::<SoftwareExecutionError>() {
            return match error {
                SoftwareExecutionError::Plan(plan) => Self::software_plan(plan),
                SoftwareExecutionError::Audit(
                    SoftwareAuditError::LockUnavailable(_)
                    | SoftwareAuditError::LocalAppDataUnavailable,
                ) => Self::SoftwareAuditUnavailable {
                    message: error.to_string(),
                },
                _ => Self::SoftwareFailed {
                    message: error.to_string(),
                },
            };
        }
        if let Some(error) = error.downcast_ref::<SoftwarePlanError>() {
            return Self::software_plan(error);
        }
        Self::software_failed(error)
    }

    fn software_plan(error: &SoftwarePlanError) -> Self {
        match error {
            SoftwarePlanError::UnsupportedInventoryVersion(_)
            | SoftwarePlanError::UnsupportedPlanVersion(_)
            | SoftwarePlanError::InvalidFingerprint
            | SoftwarePlanError::InventoryFingerprintMismatch
            | SoftwarePlanError::InventoryExpired
            | SoftwarePlanError::StaleSelection(_)
            | SoftwarePlanError::DigestMismatch => Self::SoftwareStaleAuthority {
                message: error.to_string(),
            },
            _ => Self::SoftwareFailed {
                message: error.to_string(),
            },
        }
    }

    pub(crate) fn status_failed(error: anyhow::Error) -> Self {
        Self::StatusFailed {
            message: format!("{error:#}"),
        }
    }

    pub(crate) fn io(error: impl std::fmt::Display) -> Self {
        Self::Io {
            message: error.to_string(),
        }
    }

    pub(crate) fn protection(error: devsweep_core::execution::ProtectionError) -> Self {
        let message = error.to_string();
        match error {
            devsweep_core::execution::ProtectionError::TargetMissing { .. } => {
                Self::ProtectionPathMissing { message }
            }
            devsweep_core::execution::ProtectionError::AuditUnknown => {
                Self::ProtectionAuditUnknown { message }
            }
            _ => Self::ProtectionStoreUnavailable { message },
        }
    }

    pub(crate) fn history(error: devsweep_core::history::HistoryError) -> Self {
        let message = error.to_string();
        match error {
            devsweep_core::history::HistoryError::NotFound {
                stores_incomplete: false,
                ..
            } => Self::HistoryNotFound { message },
            _ => Self::HistoryStoreUnavailable { message },
        }
    }

    pub(crate) fn execution(error: anyhow::Error) -> Self {
        if let Some(error) = error.downcast_ref::<ExecutionError>() {
            return match error {
                ExecutionError::UnknownTarget { target_id } => Self::UnknownTarget {
                    target_id: target_id.clone(),
                },
                ExecutionError::InspectOnlyTarget { target_id } => Self::InspectOnlyTarget {
                    target_id: target_id.clone(),
                },
                ExecutionError::StaleConfirmation {
                    expected_digest,
                    actual_digest,
                } => Self::StaleConfirmation {
                    expected_digest: expected_digest.clone(),
                    actual_digest: actual_digest.clone(),
                },
            };
        }
        Self::io(format!("{error:#}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_errors_keep_typed_target_identity() {
        let error = anyhow::Error::new(ExecutionError::UnknownTarget {
            target_id: TargetId::new("missing"),
        });

        assert_eq!(
            CommandError::execution(error),
            CommandError::UnknownTarget {
                target_id: TargetId::new("missing")
            }
        );
    }

    #[test]
    fn command_errors_serialize_as_stable_tagged_payloads() {
        let payload = serde_json::to_value(CommandError::UnknownTarget {
            target_id: TargetId::new("missing"),
        })
        .expect("command error serializes");

        assert_eq!(
            payload,
            serde_json::json!({
                "code": "unknown_target",
                "target_id": "missing"
            })
        );
    }
}
