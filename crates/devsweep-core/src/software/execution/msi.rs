//! MSI-only native dispatch and truthful result classification.

use super::{
    AdapterEvidence, AdapterOutcome, SoftwareAuditErrorCode, SoftwareExecutionOutcome,
    SoftwareInstalledState, SoftwareRebootEvidence, ValidatedSoftwareAction,
};
use crate::{
    process::{
        CwdPolicy, FlagCancelObserver, NoopCancelObserver, ProcessRequest, ProcessRunner,
        ProcessStatus,
    },
    software::{
        MsiContext, SoftwareIdentity,
        msi::MsiRegistrationBinding,
        msi_authority::{BoundFile, valid_evidence},
    },
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MsiInterruption {
    TimedOut,
    Canceled,
    StatusUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MsiProcessEvidenceV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interruption: Option<MsiInterruption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MsiExecutionEvidenceV1 {
    pub user_sid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<MsiProcessEvidenceV1>,
}

pub(super) trait RegistrationProvider: Send + Sync {
    fn bind(&self, identity: &SoftwareIdentity) -> Result<MsiRegistrationBinding, ()>;
    fn query(&self, identity: &SoftwareIdentity, user_sid: &str) -> SoftwareInstalledState;
}
pub(super) trait ToolResolver: Send + Sync {
    fn bind(&self) -> Result<BoundFile, ()>;
}
pub(super) trait ProcessAdapter: Send + Sync {
    fn run(
        &self,
        program: &std::path::Path,
        args: &[String],
        cancel: Option<&Arc<FlagCancelObserver>>,
        timeout: Duration,
    ) -> ProcessStatus;
}

struct NativeRegistration;
impl RegistrationProvider for NativeRegistration {
    fn bind(&self, identity: &SoftwareIdentity) -> Result<MsiRegistrationBinding, ()> {
        let SoftwareIdentity::Msi {
            product_code,
            context: MsiContext::UserUnmanaged,
        } = identity
        else {
            return Err(());
        };
        crate::software::msi::bind_registration(product_code)
    }
    fn query(&self, identity: &SoftwareIdentity, user_sid: &str) -> SoftwareInstalledState {
        crate::software::msi::query_registration(identity, user_sid)
    }
}
struct NativeTool;
impl ToolResolver for NativeTool {
    fn bind(&self) -> Result<BoundFile, ()> {
        crate::software::msi_authority::system_tool()
    }
}
struct NativeProcess;
impl ProcessAdapter for NativeProcess {
    fn run(
        &self,
        program: &std::path::Path,
        args: &[String],
        cancel: Option<&Arc<FlagCancelObserver>>,
        timeout: Duration,
    ) -> ProcessStatus {
        let never = NoopCancelObserver;
        let observer: &dyn crate::process::CancelObserver =
            cancel.map_or(&never, |cancel| cancel.as_ref());
        ProcessRunner::default()
            .run(&ProcessRequest {
                program: program.as_os_str().to_owned(),
                args: args.iter().map(Into::into).collect(),
                cwd: CwdPolicy::Neutral,
                timeout: Some(timeout),
                job_deadline: None,
                cancel: observer,
            })
            .status
    }
}

#[derive(Clone)]
pub(super) struct MsiRuntime {
    pub registrations: Arc<dyn RegistrationProvider>,
    pub tool: Arc<dyn ToolResolver>,
    pub process: Arc<dyn ProcessAdapter>,
}
impl Default for MsiRuntime {
    fn default() -> Self {
        Self {
            registrations: Arc::new(NativeRegistration),
            tool: Arc::new(NativeTool),
            process: Arc::new(NativeProcess),
        }
    }
}
impl MsiRuntime {
    pub(super) fn remove(
        &self,
        action: &ValidatedSoftwareAction,
        cancel: Option<&Arc<FlagCancelObserver>>,
        timeout: Duration,
    ) -> AdapterEvidence {
        let refused = || AdapterEvidence {
            msi_result: Some(MsiProcessEvidenceV1 {
                exit_code: None,
                interruption: Some(MsiInterruption::StatusUnavailable),
            }),
            ..AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterDispatchFailed)
        };
        let Some(expected) = &action.msi_registration else {
            return refused();
        };
        let (Ok(mut registration), Ok(tool)) =
            (self.registrations.bind(&action.identity), self.tool.bind())
        else {
            return refused();
        };
        registration.evidence.tool_digest = tool.digest.clone();
        if !valid_evidence(expected) || &registration.evidence != expected {
            return refused();
        }
        let SoftwareIdentity::Msi { product_code, .. } = &action.identity else {
            return refused();
        };
        let args = [
            "/x".to_string(),
            product_code.clone(),
            "/qn".to_string(),
            "/norestart".to_string(),
        ];
        // The Installer must be able to remove its own cached LocalPackage.
        // Release that read binding after the immediate authority check; retain
        // the system program binding across the whole bounded dispatch.
        drop(registration);
        let status = self.process.run(&tool.path, &args, cancel, timeout);
        let result = match status {
            ProcessStatus::Success => MsiProcessEvidenceV1 {
                exit_code: Some(0),
                interruption: None,
            },
            ProcessStatus::Exit { code: Some(code) } => MsiProcessEvidenceV1 {
                exit_code: Some(code as u32),
                interruption: None,
            },
            ProcessStatus::Timeout => MsiProcessEvidenceV1 {
                exit_code: None,
                interruption: Some(MsiInterruption::TimedOut),
            },
            ProcessStatus::Canceled => MsiProcessEvidenceV1 {
                exit_code: None,
                interruption: Some(MsiInterruption::Canceled),
            },
            _ => MsiProcessEvidenceV1 {
                exit_code: None,
                interruption: Some(MsiInterruption::StatusUnavailable),
            },
        };
        evidence(result)
    }
}

pub(super) fn evidence(result: MsiProcessEvidenceV1) -> AdapterEvidence {
    let mut adapter = match (result.exit_code, result.interruption) {
        (Some(0), None) => AdapterEvidence::success(),
        (Some(1641 | 3010), None) => AdapterEvidence::reboot_required(),
        (Some(_), None) => AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
        (_, Some(MsiInterruption::TimedOut)) => {
            AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterTimedOut)
        }
        (_, Some(MsiInterruption::Canceled)) => {
            AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterCanceledAfterDispatch)
        }
        _ => AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterStatusUnavailable),
    };
    adapter.msi_result = Some(result);
    adapter
}

pub(in crate::software) fn classify(
    msi: &MsiExecutionEvidenceV1,
    installed: SoftwareInstalledState,
) -> SoftwareExecutionOutcome {
    use SoftwareExecutionOutcome as Outcome;
    match msi.process {
        Some(MsiProcessEvidenceV1 {
            exit_code: Some(0),
            interruption: None,
        }) => match installed {
            SoftwareInstalledState::Absent => Outcome::Removed,
            SoftwareInstalledState::Present => Outcome::StillPresent,
            _ => Outcome::UnknownAfterDispatch,
        },
        Some(MsiProcessEvidenceV1 {
            exit_code: Some(1641 | 3010),
            interruption: None,
        }) => Outcome::RebootRequired,
        Some(MsiProcessEvidenceV1 {
            exit_code: Some(_),
            interruption: None,
        }) => Outcome::Failed,
        _ => Outcome::UnknownAfterDispatch,
    }
}

pub(in crate::software) fn valid_process(
    result: MsiProcessEvidenceV1,
    adapter: AdapterOutcome,
    reboot: SoftwareRebootEvidence,
) -> bool {
    if result.exit_code.is_some() == result.interruption.is_some() {
        return false;
    }
    let expected = evidence(result);
    expected.outcome == adapter && expected.reboot_evidence == reboot
}
