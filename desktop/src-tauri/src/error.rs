use devsweep_core::execution::ExecutionError;
use serde::Serialize;

use devsweep_core::{execution::ConfirmationDigest, model::TargetId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub(crate) enum CommandError {
    ScanAlreadyRunning,
    ScanFailed {
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

    pub(crate) fn io(error: impl std::fmt::Display) -> Self {
        Self::Io {
            message: error.to_string(),
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
