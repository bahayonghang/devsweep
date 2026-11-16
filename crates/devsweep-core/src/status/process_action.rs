//! Reviewed process termination. The plan binds PID and creation time.
//! A mismatch returns before any kill. This module does not elevate.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::digest_encoding::hex_digest;

/// Plan document version.
pub const PROCESS_TERMINATION_PLAN_VERSION: u32 = 1;

/// Canonical data-loss sentence included in every successful preview.
pub const PROCESS_TERMINATION_DATA_LOSS_TEXT: &str =
    "Terminating this process can discard unsaved work and in-memory data.";

/// Closed protected-name classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectedClass {
    Critical,
    Shared,
    System,
}

/// Why termination did not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessActionError {
    Protected { class: ProtectedClass, name: String },
    IdentityMismatch,
    NotFound,
    PermissionDenied,
    Unsupported,
    ConfirmRequired,
    StaleDigest,
    InvalidPlan,
    AuditFailed { after_dispatch: bool },
    CanceledBeforeDispatch,
}

impl fmt::Display for ProcessActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protected { class, name } => {
                write!(
                    formatter,
                    "protected process name {name} ({class:?}) is refused"
                )
            }
            Self::IdentityMismatch => {
                write!(formatter, "process pid and creation time do not match")
            }
            Self::NotFound => write!(formatter, "process identity was not found"),
            Self::PermissionDenied => {
                write!(formatter, "process query was denied without elevation")
            }
            Self::Unsupported => write!(formatter, "process termination is unsupported"),
            Self::ConfirmRequired => write!(formatter, "process termination requires confirm"),
            Self::StaleDigest => write!(formatter, "process termination preview digest is stale"),
            Self::InvalidPlan => write!(formatter, "process termination plan is invalid"),
            Self::CanceledBeforeDispatch => {
                write!(formatter, "process termination canceled before dispatch")
            }
            Self::AuditFailed {
                after_dispatch: false,
            } => {
                write!(
                    formatter,
                    "process action history is unavailable; no termination dispatched"
                )
            }
            Self::AuditFailed {
                after_dispatch: true,
            } => {
                write!(
                    formatter,
                    "process action result could not be recorded after dispatch; outcome is unknown"
                )
            }
        }
    }
}

impl std::error::Error for ProcessActionError {}

impl ProcessActionError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Protected { .. } => "protected",
            Self::IdentityMismatch | Self::NotFound => "invalid_identity",
            Self::PermissionDenied => "identity_unavailable",
            Self::Unsupported => "unsupported_platform",
            Self::ConfirmRequired => "confirmation_required",
            Self::StaleDigest => "stale_confirmation",
            Self::InvalidPlan => "invalid_plan",
            Self::CanceledBeforeDispatch => "canceled_before_start",
            Self::AuditFailed {
                after_dispatch: false,
            } => "audit_unavailable",
            Self::AuditFailed {
                after_dispatch: true,
            } => "unknown_after_dispatch",
        }
    }
}

/// Saved termination plan. Creation time is a decimal Windows FILETIME.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessTerminationPlanV1 {
    pub version: u32,
    pub pid: u32,
    pub creation_time_filetime: String,
    pub name: String,
}

/// Live identity used to bind and re-check a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedProcess {
    pub pid: u32,
    pub creation_time_filetime: u64,
    pub name: String,
}

/// Preview. `data_loss_text` is part of the digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessTerminationPreviewV1 {
    pub version: u32,
    pub pid: u32,
    pub creation_time_filetime: String,
    pub name: String,
    pub data_loss_text: String,
    pub digest: String,
}

/// Result of one confirmed termination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessTerminationReportV1 {
    pub version: u32,
    pub pid: u32,
    pub creation_time_filetime: String,
    pub name: String,
    pub outcome: String,
}

/// Read-only identity lookup.
pub trait ProcessProbe {
    fn lookup(&self, pid: u32) -> Result<Option<ObservedProcess>, ProcessActionError>;
}

/// Kill callback. Implementations must compare creation time before any kill.
pub trait ProcessKiller {
    fn terminate(
        &mut self,
        pid: u32,
        creation_time_filetime: u64,
    ) -> Result<(), ProcessActionError>;
}

/// Host lookup and termination. Non-Windows calls return [`ProcessActionError::Unsupported`].
#[derive(Debug, Default, Clone, Copy)]
pub struct HostProcessControl;

impl ProcessProbe for HostProcessControl {
    fn lookup(&self, pid: u32) -> Result<Option<ObservedProcess>, ProcessActionError> {
        host_lookup(pid)
    }
}

impl ProcessKiller for HostProcessControl {
    fn terminate(
        &mut self,
        pid: u32,
        creation_time_filetime: u64,
    ) -> Result<(), ProcessActionError> {
        host_terminate(pid, creation_time_filetime)
    }
}

/// Closed fixture list of critical, shared, and system names.
#[must_use]
pub fn protected_process_names() -> &'static [(&'static str, ProtectedClass)] {
    PROTECTED_NAMES
}

const PROTECTED_NAMES: &[(&str, ProtectedClass)] = &[
    ("csrss.exe", ProtectedClass::Critical),
    ("wininit.exe", ProtectedClass::Critical),
    ("smss.exe", ProtectedClass::Critical),
    ("lsass.exe", ProtectedClass::Critical),
    ("services.exe", ProtectedClass::Critical),
    ("winlogon.exe", ProtectedClass::Critical),
    ("explorer.exe", ProtectedClass::Shared),
    ("dwm.exe", ProtectedClass::Shared),
    ("svchost.exe", ProtectedClass::Shared),
    ("system", ProtectedClass::System),
    ("registry", ProtectedClass::System),
    ("idle", ProtectedClass::System),
    ("secure system", ProtectedClass::System),
    ("system idle process", ProtectedClass::System),
];

/// Bind a plan. Protected names are refused before a document is returned.
pub fn bind_process_termination_plan(
    pid: u32,
    creation_time_filetime: u64,
    name: &str,
) -> Result<ProcessTerminationPlanV1, ProcessActionError> {
    if name.chars().any(char::is_control) {
        return Err(ProcessActionError::InvalidPlan);
    }
    let name = base_name(name);
    validate_identity(pid, &creation_time_filetime.to_string(), &name)?;
    if let Some(class) = protected_class(&name) {
        return Err(ProcessActionError::Protected { class, name });
    }
    Ok(ProcessTerminationPlanV1 {
        version: PROCESS_TERMINATION_PLAN_VERSION,
        pid,
        creation_time_filetime: creation_time_filetime.to_string(),
        name,
    })
}

/// Re-check the live identity and return the data-loss preview.
pub fn preview_bound_process(
    plan: &ProcessTerminationPlanV1,
    probe: &dyn ProcessProbe,
) -> Result<ProcessTerminationPreviewV1, ProcessActionError> {
    let plan = checked_plan(plan)?;
    refuse_protected(&plan.name)?;
    confirm_live(&plan, probe)?;
    Ok(ProcessTerminationPreviewV1 {
        version: plan.version,
        pid: plan.pid,
        creation_time_filetime: plan.creation_time_filetime.clone(),
        name: plan.name.clone(),
        data_loss_text: PROCESS_TERMINATION_DATA_LOSS_TEXT.to_string(),
        digest: preview_digest(&plan)?,
    })
}

/// Terminate only when confirm, digest, PID, and creation time all match.
pub fn terminate_bound_process(
    plan: &ProcessTerminationPlanV1,
    digest: &str,
    confirm: bool,
    probe: &dyn ProcessProbe,
    killer: &mut dyn ProcessKiller,
) -> Result<ProcessTerminationReportV1, ProcessActionError> {
    if !confirm {
        return Err(ProcessActionError::ConfirmRequired);
    }
    let plan = checked_plan(plan)?;
    refuse_protected(&plan.name)?;
    confirm_live(&plan, probe)?;
    let expected = preview_digest(&plan)?;
    if digest != expected {
        return Err(ProcessActionError::StaleDigest);
    }
    let creation = parse_filetime(&plan.creation_time_filetime)?;
    killer.terminate(plan.pid, creation)?;
    Ok(ProcessTerminationReportV1 {
        version: plan.version,
        pid: plan.pid,
        creation_time_filetime: plan.creation_time_filetime,
        name: plan.name,
        outcome: "terminated".to_string(),
    })
}

/// Production CLI/GUI execution. Prechecks do not open history; a durable
/// Started then precedes the final core/handle check. Never automatically retry.
pub fn terminate_bound_process_audited(
    plan: &ProcessTerminationPlanV1,
    digest: &str,
    confirm: bool,
    probe: &dyn ProcessProbe,
    killer: &mut dyn ProcessKiller,
    audit: &mut dyn super::process_action_audit::ProcessActionAudit,
) -> Result<ProcessTerminationReportV1, ProcessActionError> {
    terminate_bound_process_audited_cancellable(
        plan,
        digest,
        confirm,
        probe,
        killer,
        audit,
        &crate::process::NoopCancelObserver,
    )
}

/// Cancellation is observed only before the one irreversible killer call.
pub fn terminate_bound_process_audited_cancellable(
    plan: &ProcessTerminationPlanV1,
    digest: &str,
    confirm: bool,
    probe: &dyn ProcessProbe,
    killer: &mut dyn ProcessKiller,
    audit: &mut dyn super::process_action_audit::ProcessActionAudit,
    cancel: &dyn crate::process::CancelObserver,
) -> Result<ProcessTerminationReportV1, ProcessActionError> {
    use super::keep_awake_audit::{
        ProcessActionAuditDetailsV1, StatusActionAuditAction, StatusActionAuditCode,
        StatusActionAuditRecordV1,
    };
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    if !confirm {
        return Err(ProcessActionError::ConfirmRequired);
    }
    if cancel.is_cancel_requested() {
        return Err(ProcessActionError::CanceledBeforeDispatch);
    }
    let preview = preview_bound_process(plan, probe)?;
    if preview.digest != digest {
        return Err(ProcessActionError::StaleDigest);
    }
    static NEXT_OPERATION: AtomicU64 = AtomicU64::new(0);
    let now = || {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
    };
    let operation_id = format!(
        "process-{}-{}-{}",
        std::process::id(),
        now().as_nanos(),
        NEXT_OPERATION.fetch_add(1, Ordering::Relaxed),
    );
    let record = |status_code, error_code: Option<String>| StatusActionAuditRecordV1 {
        schema_version: 1,
        domain: "status".into(),
        operation_id: operation_id.clone(),
        timestamp_unix_ms: u64::try_from(now().as_millis()).unwrap_or(u64::MAX),
        action: StatusActionAuditAction::ProcessTerminate,
        status_code,
        duration_ms: 0,
        preview_digest: Some(digest.into()),
        process: Some(ProcessActionAuditDetailsV1 {
            pid: preview.pid,
            creation_time_filetime: preview.creation_time_filetime.clone(),
            name: preview.name.clone(),
            error_code,
        }),
    };
    audit
        .append(record(StatusActionAuditCode::DispatchStarted, None))
        .map_err(|_| ProcessActionError::AuditFailed {
            after_dispatch: false,
        })?;
    struct Dispatch<'a> {
        killer: &'a mut dyn ProcessKiller,
        called: bool,
        cancel: &'a dyn crate::process::CancelObserver,
    }
    impl ProcessKiller for Dispatch<'_> {
        fn terminate(&mut self, pid: u32, creation: u64) -> Result<(), ProcessActionError> {
            if self.cancel.is_cancel_requested() {
                return Err(ProcessActionError::CanceledBeforeDispatch);
            }
            self.called = true;
            self.killer.terminate(pid, creation)
        }
    }
    let mut dispatch = Dispatch {
        killer,
        called: false,
        cancel,
    };
    let result = terminate_bound_process(plan, digest, confirm, probe, &mut dispatch);
    let code = match &result {
        Ok(_) => StatusActionAuditCode::Succeeded,
        Err(_) if dispatch.called => StatusActionAuditCode::BackendFailed,
        Err(_) => StatusActionAuditCode::Refused,
    };
    audit
        .append(record(
            code,
            result.as_ref().err().map(|error| error.code().into()),
        ))
        .map_err(|_| ProcessActionError::AuditFailed {
            after_dispatch: dispatch.called,
        })?;
    result
}

fn checked_plan(
    plan: &ProcessTerminationPlanV1,
) -> Result<ProcessTerminationPlanV1, ProcessActionError> {
    if plan.version != PROCESS_TERMINATION_PLAN_VERSION {
        return Err(ProcessActionError::InvalidPlan);
    }
    if plan.name.chars().any(char::is_control) {
        return Err(ProcessActionError::InvalidPlan);
    }
    let name = base_name(&plan.name);
    validate_identity(plan.pid, &plan.creation_time_filetime, &name)?;
    Ok(ProcessTerminationPlanV1 {
        version: plan.version,
        pid: plan.pid,
        creation_time_filetime: plan.creation_time_filetime.clone(),
        name,
    })
}

fn validate_identity(
    pid: u32,
    creation_time_filetime: &str,
    name: &str,
) -> Result<(), ProcessActionError> {
    if pid == 0 || name.is_empty() || name.len() > 260 || name.chars().any(char::is_control) {
        return Err(ProcessActionError::InvalidPlan);
    }
    parse_filetime(creation_time_filetime)?;
    if name.contains(['\\', '/']) {
        return Err(ProcessActionError::InvalidPlan);
    }
    Ok(())
}

fn confirm_live(
    plan: &ProcessTerminationPlanV1,
    probe: &dyn ProcessProbe,
) -> Result<ObservedProcess, ProcessActionError> {
    let live = probe
        .lookup(plan.pid)?
        .ok_or(ProcessActionError::NotFound)?;
    if let Some(class) = protected_class(&live.name) {
        return Err(ProcessActionError::Protected {
            class,
            name: base_name(&live.name),
        });
    }
    let creation = parse_filetime(&plan.creation_time_filetime)?;
    if live.pid != plan.pid
        || live.creation_time_filetime != creation
        || !names_equal(&live.name, &plan.name)
    {
        return Err(ProcessActionError::IdentityMismatch);
    }
    Ok(live)
}

fn refuse_protected(name: &str) -> Result<(), ProcessActionError> {
    if let Some(class) = protected_class(name) {
        Err(ProcessActionError::Protected {
            class,
            name: base_name(name),
        })
    } else {
        Ok(())
    }
}

pub fn protected_process_class(name: &str) -> Option<ProtectedClass> {
    let name = base_name(name).to_ascii_lowercase();
    PROTECTED_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, class)| *class)
}

fn protected_class(name: &str) -> Option<ProtectedClass> {
    protected_process_class(name)
}

fn names_equal(live: &str, planned: &str) -> bool {
    base_name(live).eq_ignore_ascii_case(&base_name(planned))
}

fn base_name(name: &str) -> String {
    name.rsplit(['\\', '/'])
        .next()
        .unwrap_or(name)
        .trim()
        .to_string()
}

fn parse_filetime(value: &str) -> Result<u64, ProcessActionError> {
    if value.is_empty()
        || value.starts_with(['+', '-'])
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(ProcessActionError::InvalidPlan);
    }
    value
        .parse()
        .ok()
        .filter(|parsed: &u64| *parsed != 0 && parsed.to_string() == value)
        .ok_or(ProcessActionError::InvalidPlan)
}

fn preview_digest(plan: &ProcessTerminationPlanV1) -> Result<String, ProcessActionError> {
    #[derive(Serialize)]
    struct Canonical<'a> {
        domain: &'static str,
        version: u32,
        pid: u32,
        creation_time_filetime: &'a str,
        name: String,
        data_loss_text: &'static str,
    }
    let bytes = serde_json::to_vec(&Canonical {
        domain: "devsweep.process-termination.v1",
        version: plan.version,
        pid: plan.pid,
        creation_time_filetime: &plan.creation_time_filetime,
        name: plan.name.to_ascii_lowercase(),
        data_loss_text: PROCESS_TERMINATION_DATA_LOSS_TEXT,
    })
    .map_err(|_| ProcessActionError::InvalidPlan)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(bytes))))
}

#[cfg(windows)]
fn host_lookup(pid: u32) -> Result<Option<ObservedProcess>, ProcessActionError> {
    windows_observation(pid).map(Some)
}

#[cfg(not(windows))]
fn host_lookup(pid: u32) -> Result<Option<ObservedProcess>, ProcessActionError> {
    let _ = pid;
    Err(ProcessActionError::Unsupported)
}

#[cfg(windows)]
fn host_terminate(pid: u32, creation_time_filetime: u64) -> Result<(), ProcessActionError> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
        TerminateProcess,
    };

    // SAFETY: query and terminate rights only. No elevation request is made.
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return Err(windows_open_error());
    }
    struct Owned(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Owned {
        fn drop(&mut self) {
            // SAFETY: `OpenProcess` returned this handle.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let handle = Owned(handle);
    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    // SAFETY: the FILETIME pointers are writable locals and the handle is open.
    let ok = unsafe { GetProcessTimes(handle.0, &mut creation, &mut exit, &mut kernel, &mut user) };
    if ok == 0 {
        return Err(windows_open_error());
    }
    if filetime_u64(creation) != creation_time_filetime {
        return Err(ProcessActionError::IdentityMismatch);
    }
    // SAFETY: creation time matched the plan before this kill.
    let killed = unsafe { TerminateProcess(handle.0, 1) };
    if killed == 0 {
        return Err(windows_open_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn host_terminate(pid: u32, creation_time_filetime: u64) -> Result<(), ProcessActionError> {
    let _ = (pid, creation_time_filetime);
    Err(ProcessActionError::Unsupported)
}

#[cfg(windows)]
fn windows_observation(pid: u32) -> Result<ObservedProcess, ProcessActionError> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    // Name and creation time must come from the same process handle. A name
    // from Toolhelp followed by a separately opened reused PID is not identity.
    // SAFETY: query-limited rights only, never elevation.
    let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if raw.is_null() {
        return Err(windows_open_error());
    }
    struct Owned(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Owned {
        fn drop(&mut self) {
            // SAFETY: owns the handle returned by OpenProcess.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let handle = Owned(raw);
    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = creation;
    let mut kernel = creation;
    let mut user = creation;
    // SAFETY: the handle is owned and output pointers are writable locals.
    if unsafe { GetProcessTimes(handle.0, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(windows_open_error());
    }
    let mut image = [0_u16; 32_768];
    let mut length = image.len() as u32;
    // SAFETY: bounded output array and its length, same still-owned handle.
    if unsafe { QueryFullProcessImageNameW(handle.0, 0, image.as_mut_ptr(), &mut length) } == 0 {
        return Err(windows_open_error());
    }
    let name = base_name(&String::from_utf16_lossy(&image[..length as usize]));
    validate_identity(pid, &filetime_u64(creation).to_string(), &name)?;
    Ok(ObservedProcess {
        pid,
        creation_time_filetime: filetime_u64(creation),
        name,
    })
}

#[cfg(windows)]
fn windows_open_error() -> ProcessActionError {
    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, GetLastError};

    // SAFETY: reads this thread's last-error immediately after a failed API.
    let code = unsafe { GetLastError() };
    if code == ERROR_ACCESS_DENIED {
        ProcessActionError::PermissionDenied
    } else {
        ProcessActionError::NotFound
    }
}

#[cfg(windows)]
fn filetime_u64(time: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scripted {
        live: Option<ObservedProcess>,
        error: Option<ProcessActionError>,
    }

    impl ProcessProbe for Scripted {
        fn lookup(&self, pid: u32) -> Result<Option<ObservedProcess>, ProcessActionError> {
            if let Some(error) = &self.error {
                return Err(error.clone());
            }
            Ok(self.live.clone().filter(|process| process.pid == pid))
        }
    }

    struct Spy {
        calls: u32,
    }

    impl ProcessKiller for Spy {
        fn terminate(
            &mut self,
            pid: u32,
            creation_time_filetime: u64,
        ) -> Result<(), ProcessActionError> {
            let _ = (pid, creation_time_filetime);
            self.calls += 1;
            Ok(())
        }
    }

    fn live(pid: u32, creation: u64, name: &str) -> ObservedProcess {
        ObservedProcess {
            pid,
            creation_time_filetime: creation,
            name: name.to_string(),
        }
    }

    #[test]
    fn noncanonical_filetime_and_control_names_refuse_before_audit_or_kill() {
        let probe = Scripted {
            live: Some(live(42, 1000, "fixture.exe")),
            error: None,
        };
        for creation in ["0", "01000", "+1000", "-1000", "1000 "] {
            let plan = ProcessTerminationPlanV1 {
                version: 1,
                pid: 42,
                creation_time_filetime: creation.into(),
                name: "fixture.exe".into(),
            };
            let mut killer = Spy { calls: 0 };
            let mut audit = super::super::RecordingProcessActionAudit::default();
            assert_eq!(
                preview_bound_process(&plan, &probe),
                Err(ProcessActionError::InvalidPlan)
            );
            assert_eq!(
                terminate_bound_process_audited(&plan, "", true, &probe, &mut killer, &mut audit),
                Err(ProcessActionError::InvalidPlan)
            );
            assert_eq!(killer.calls, 0);
            assert!(audit.0.is_empty());
        }
        for name in ["\nfixture.exe", "fixture.exe\n", "folder\n/fixture.exe"] {
            assert_eq!(
                bind_process_termination_plan(42, 1000, name),
                Err(ProcessActionError::InvalidPlan)
            );
            let plan = ProcessTerminationPlanV1 {
                version: 1,
                pid: 42,
                creation_time_filetime: "1000".into(),
                name: name.into(),
            };
            assert_eq!(
                preview_bound_process(&plan, &probe),
                Err(ProcessActionError::InvalidPlan)
            );
        }
    }

    #[test]
    fn protected_critical_shared_and_system_process_names_refuse() {
        for (name, class) in protected_process_names() {
            let error = bind_process_termination_plan(4242, 99, name).expect_err(name);
            assert_eq!(
                error,
                ProcessActionError::Protected {
                    class: *class,
                    name: (*name).to_string(),
                }
            );
            let plan = ProcessTerminationPlanV1 {
                version: 1,
                pid: 4242,
                creation_time_filetime: "99".to_string(),
                name: (*name).to_string(),
            };
            let probe = Scripted {
                live: Some(live(4242, 99, name)),
                error: None,
            };
            let mut killer = Spy { calls: 0 };
            let refused =
                terminate_bound_process(&plan, "sha256:unused", true, &probe, &mut killer);
            assert!(refused.is_err(), "{name}");
            assert_eq!(killer.calls, 0, "{name}");
        }
    }

    #[test]
    fn pid_reuse_and_process_identity_drift_refuse_termination_before_kill() {
        let plan = bind_process_termination_plan(50, 1000, "fixture.exe").expect("plan");
        let preview = preview_bound_process(
            &plan,
            &Scripted {
                live: Some(live(50, 1000, "fixture.exe")),
                error: None,
            },
        )
        .expect("preview");
        assert_eq!(preview.data_loss_text, PROCESS_TERMINATION_DATA_LOSS_TEXT);
        assert!(preview.digest.starts_with("sha256:"));
        assert!(!preview.data_loss_text.is_empty());

        let mut reused = plan.clone();
        reused.creation_time_filetime = "1001".to_string();
        let mut killer = Spy { calls: 0 };
        let error = terminate_bound_process(
            &reused,
            &preview.digest,
            true,
            &Scripted {
                live: Some(live(50, 1000, "fixture.exe")),
                error: None,
            },
            &mut killer,
        );
        assert_eq!(error, Err(ProcessActionError::IdentityMismatch));
        assert_eq!(killer.calls, 0);

        let mut renamed = plan.clone();
        renamed.name = "other.exe".to_string();
        let error = terminate_bound_process(
            &renamed,
            &preview_bound_process(
                &renamed,
                &Scripted {
                    live: Some(live(50, 1000, "other.exe")),
                    error: None,
                },
            )
            .expect("renamed preview")
            .digest,
            true,
            &Scripted {
                live: Some(live(50, 1000, "fixture.exe")),
                error: None,
            },
            &mut killer,
        );
        assert_eq!(error, Err(ProcessActionError::IdentityMismatch));
        assert_eq!(killer.calls, 0);

        let error = terminate_bound_process(
            &plan,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            true,
            &Scripted {
                live: Some(live(50, 1000, "fixture.exe")),
                error: None,
            },
            &mut killer,
        );
        assert_eq!(error, Err(ProcessActionError::StaleDigest));
        assert_eq!(killer.calls, 0);

        let error = terminate_bound_process(
            &plan,
            &preview.digest,
            false,
            &Scripted {
                live: Some(live(50, 1000, "fixture.exe")),
                error: None,
            },
            &mut killer,
        );
        assert_eq!(error, Err(ProcessActionError::ConfirmRequired));
        assert_eq!(killer.calls, 0);

        let error = terminate_bound_process(
            &plan,
            &preview.digest,
            true,
            &Scripted {
                live: None,
                error: Some(ProcessActionError::PermissionDenied),
            },
            &mut killer,
        );
        assert_eq!(error, Err(ProcessActionError::PermissionDenied));
        assert_eq!(killer.calls, 0);

        let report = terminate_bound_process(
            &plan,
            &preview.digest,
            true,
            &Scripted {
                live: Some(live(50, 1000, "fixture.exe")),
                error: None,
            },
            &mut killer,
        )
        .expect("matching identity terminates");
        assert_eq!(report.outcome, "terminated");
        assert_eq!(killer.calls, 1);
        assert!(
            !serde_json::to_string(&preview)
                .expect("preview json")
                .contains("elevate")
        );
    }

    #[cfg(windows)]
    #[test]
    fn fixture_owned_process_is_the_only_native_termination_target() {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let ping = std::path::PathBuf::from(root)
            .join("System32")
            .join("ping.exe");
        let child = Command::new(ping)
            .args(["-n", "30", "127.0.0.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn ping");
        struct Guard(Option<std::process::Child>);
        impl Drop for Guard {
            fn drop(&mut self) {
                if let Some(child) = self.0.as_mut() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
        let mut guard = Guard(Some(child));
        let child = guard.0.as_mut().expect("child");
        let pid = child.id();
        let observed = HostProcessControl
            .lookup(pid)
            .expect("lookup")
            .expect("spawned process is visible");
        assert_eq!(observed.pid, pid);
        assert!(observed.name.eq_ignore_ascii_case("ping.exe"));
        let plan = bind_process_termination_plan(
            observed.pid,
            observed.creation_time_filetime,
            &observed.name,
        )
        .expect("bind spawned process");
        let preview = preview_bound_process(&plan, &HostProcessControl).expect("preview");
        assert!(
            preview
                .data_loss_text
                .contains("discard unsaved work and in-memory data")
        );

        let mut drifted = plan.clone();
        drifted.creation_time_filetime =
            observed.creation_time_filetime.wrapping_add(1).to_string();
        let mismatch = terminate_bound_process(
            &drifted,
            &preview.digest,
            true,
            &HostProcessControl,
            &mut HostProcessControl,
        );
        assert_eq!(mismatch, Err(ProcessActionError::IdentityMismatch));
        assert!(child.try_wait().expect("poll").is_none());

        let report = terminate_bound_process(
            &plan,
            &preview.digest,
            true,
            &HostProcessControl,
            &mut HostProcessControl,
        )
        .expect("terminate spawned process");
        assert_eq!(report.pid, pid);
        assert_eq!(report.outcome, "terminated");
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if child.try_wait().expect("wait").is_some() {
                break;
            }
            if Instant::now() >= deadline {
                panic!("spawned process was not terminated");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
