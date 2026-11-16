//! Redacted action history, never a lifecycle receipt or execution authority.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::keep_awake::{KeepAwakeError, MAX_KEEP_AWAKE_MS};

const MAX_STATUS_AUDIT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusActionAuditAction {
    Start,
    Stop,
    Cancel,
    Expiry,
    OwnerExit,
    ProcessTerminate,
}

/// Compatibility name for the original finite-power action API.
pub type KeepAwakeAuditAction = StatusActionAuditAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusActionAuditCode {
    DispatchStarted,
    Succeeded,
    BackendFailed,
    UnknownAfterDispatch,
    Refused,
}

/// Reviewed process identity only; never command line, executable path or environment.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessActionAuditDetailsV1 {
    pub pid: u32,
    pub creation_time_filetime: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

/// Closed V1 action record. No owner, endpoint, token, executable or error text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusActionAuditRecordV1 {
    pub schema_version: u32,
    pub domain: String,
    pub operation_id: String,
    pub timestamp_unix_ms: u64,
    pub action: StatusActionAuditAction,
    pub status_code: StatusActionAuditCode,
    pub duration_ms: u64,
    pub preview_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessActionAuditDetailsV1>,
}

impl StatusActionAuditRecordV1 {
    pub(crate) fn is_valid(&self) -> bool {
        self.schema_version == 1
            && self.domain == "status"
            && !self.operation_id.is_empty()
            && match (&self.action, &self.process) {
                (StatusActionAuditAction::ProcessTerminate, Some(process)) => {
                    self.duration_ms == 0
                        && self.preview_digest.is_some()
                        && super::process_action::bind_process_termination_plan(
                            process.pid,
                            process.creation_time_filetime.parse().unwrap_or(0),
                            &process.name,
                        )
                        .is_ok_and(|plan| {
                            plan.creation_time_filetime == process.creation_time_filetime
                                && plan.name == process.name
                        })
                        && match self.status_code {
                            StatusActionAuditCode::DispatchStarted
                            | StatusActionAuditCode::Succeeded => process.error_code.is_none(),
                            StatusActionAuditCode::BackendFailed
                            | StatusActionAuditCode::Refused => {
                                process.error_code.as_deref().is_some_and(|code| {
                                    matches!(
                                        code,
                                        "protected"
                                            | "invalid_identity"
                                            | "identity_unavailable"
                                            | "unsupported_platform"
                                            | "confirmation_required"
                                            | "stale_confirmation"
                                            | "invalid_plan"
                                            | "canceled_before_start"
                                    )
                                })
                            }
                            StatusActionAuditCode::UnknownAfterDispatch => false,
                        }
                }
                (StatusActionAuditAction::ProcessTerminate, None) => false,
                (_, None) => {
                    (1..=MAX_KEEP_AWAKE_MS).contains(&self.duration_ms)
                        && self.status_code != StatusActionAuditCode::Refused
                }
                (_, Some(_)) => false,
            }
            && self.preview_digest.as_ref().is_none_or(|digest| {
                digest.strip_prefix("sha256:").is_some_and(|hex| {
                    hex.len() == 64
                        && hex
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                })
            })
    }
}

/// Injection boundary for fixture journals. Production always uses the fixed file.
pub trait KeepAwakeAudit: Send {
    fn append(&mut self, record: StatusActionAuditRecordV1) -> Result<(), KeepAwakeError>;
}

/// Isolated in-memory audit used only with injected backends.
#[derive(Clone, Default)]
pub struct RecordingKeepAwakeAudit(pub Arc<Mutex<Vec<StatusActionAuditRecordV1>>>);

impl KeepAwakeAudit for RecordingKeepAwakeAudit {
    fn append(&mut self, record: StatusActionAuditRecordV1) -> Result<(), KeepAwakeError> {
        self.0.lock().expect("keep-awake audit lock").push(record);
        Ok(())
    }
}

pub(crate) struct FileKeepAwakeAudit {
    path: Result<PathBuf, KeepAwakeError>,
}

impl FileKeepAwakeAudit {
    pub(crate) fn production() -> Self {
        Self {
            path: status_audit_v1_path(),
        }
    }

    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path: Ok(path) }
    }
}

/// The sole Status action store. Its records never prove a live endpoint is off.
pub fn status_audit_v1_path() -> Result<PathBuf, KeepAwakeError> {
    let root = std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| audit_error("LOCALAPPDATA is unavailable"))?;
    Ok(PathBuf::from(root).join("DevSweep/audit/v1/status.jsonl"))
}

impl KeepAwakeAudit for FileKeepAwakeAudit {
    fn append(&mut self, record: StatusActionAuditRecordV1) -> Result<(), KeepAwakeError> {
        append_status_record(self.path.as_ref().map_err(Clone::clone)?, &record)
    }
}

/// Shared bounded store for independently typed Status actions.
pub(crate) fn append_status_record(
    path: &Path,
    record: &StatusActionAuditRecordV1,
) -> Result<(), KeepAwakeError> {
    if !record.is_valid() {
        return Err(audit_error("invalid status action record"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| audit_error("invalid audit path"))?;
    fs::create_dir_all(parent).map_err(audit_error)?;
    let _lock = AuditLock::acquire(&parent.join("status.lock")).map_err(audit_error)?;
    let mut existing_bytes = 0_u64;
    if path.try_exists().map_err(audit_error)? {
        let mut text = String::new();
        File::open(path)
            .map_err(audit_error)?
            .take(MAX_STATUS_AUDIT_BYTES + 1)
            .read_to_string(&mut text)
            .map_err(audit_error)?;
        if text.len() as u64 > MAX_STATUS_AUDIT_BYTES {
            return Err(audit_error("status journal exceeds read limit"));
        }
        existing_bytes = text.len() as u64;
        if !text.is_empty() && !text.ends_with('\n') {
            return Err(audit_error("incomplete status journal"));
        }
        for line in text.lines() {
            let old: StatusActionAuditRecordV1 = serde_json::from_str(line).map_err(audit_error)?;
            if !old.is_valid() {
                return Err(audit_error("unsupported status journal"));
            }
        }
    }
    let mut bytes = serde_json::to_vec(record).map_err(audit_error)?;
    bytes.push(b'\n');
    if existing_bytes.saturating_add(bytes.len() as u64) > MAX_STATUS_AUDIT_BYTES {
        return Err(audit_error("status journal exceeds append limit"));
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(audit_error)?;
    file.write_all(&bytes).map_err(audit_error)?;
    file.flush().map_err(audit_error)?;
    file.sync_all().map_err(audit_error)
}

struct AuditLock(File);

impl AuditLock {
    fn acquire(path: &Path) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        let file = options.open(path)?;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // SAFETY: the guard owns the descriptor through the lock lifetime.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(Self(file))
    }
}

impl Drop for AuditLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // SAFETY: explicitly unlock the still-owned descriptor before close.
            unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
        }
        #[cfg(not(unix))]
        let _ = &self.0;
    }
}

fn audit_error(error: impl std::fmt::Display) -> KeepAwakeError {
    KeepAwakeError::AuditFailed {
        after_dispatch: false,
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{BackendCall, FakeClock, KeepAwakeService, RecordingBackend};
    use std::time::Duration;

    struct FailAppend {
        calls: usize,
        fail_at: usize,
        records: RecordingKeepAwakeAudit,
    }

    impl KeepAwakeAudit for FailAppend {
        fn append(&mut self, record: StatusActionAuditRecordV1) -> Result<(), KeepAwakeError> {
            self.calls += 1;
            if self.calls == self.fail_at {
                return Err(audit_error("injected append failure"));
            }
            self.records.append(record)
        }
    }

    #[test]
    fn keep_awake_started_audit_failure_refuses_acquisition_and_terminal_failure_is_unknown() {
        for fail_at in [1, 2] {
            let (backend, calls) = RecordingBackend::pair();
            let records = RecordingKeepAwakeAudit::default();
            let clock = Arc::new(FakeClock::new());
            let service = KeepAwakeService::new_with_audit(
                clock.clone(),
                Box::new(backend),
                true,
                Box::new(FailAppend {
                    calls: 0,
                    fail_at,
                    records: records.clone(),
                }),
            );
            let error = service
                .run("private-owner", Duration::from_secs(30), true)
                .expect_err("audit failure");
            assert!(
                matches!(error, KeepAwakeError::AuditFailed { after_dispatch, .. } if after_dispatch == (fail_at == 2))
            );
            if fail_at == 1 {
                assert!(calls.lock().expect("calls").is_empty());
                assert_eq!(service.stored_lease_count(), 0);
            } else {
                assert!(
                    calls
                        .lock()
                        .expect("calls")
                        .iter()
                        .any(|call| matches!(call, BackendCall::Acquire(_)))
                );
                assert_eq!(service.stored_lease_count(), 1);
                let token = match &calls.lock().expect("calls")[0] {
                    BackendCall::Acquire(token) => token.clone(),
                    _ => panic!("acquire was not first"),
                };
                clock.advance(Duration::from_secs(30));
                assert_eq!(
                    service
                        .status(&token, "private-owner")
                        .expect("finite despite unknown start audit")
                        .state,
                    "expired"
                );
            }
            service
                .shutdown()
                .expect("cleanup after one injected failure");
        }
    }

    #[test]
    fn keep_awake_stop_audit_failure_blocks_clear_before_dispatch_and_marks_terminal_failure_unknown()
     {
        for fail_at in [3, 4] {
            let (backend, calls) = RecordingBackend::pair();
            let service = KeepAwakeService::new_with_audit(
                Arc::new(FakeClock::new()),
                Box::new(backend),
                true,
                Box::new(FailAppend {
                    calls: 0,
                    fail_at,
                    records: RecordingKeepAwakeAudit::default(),
                }),
            );
            let lease = service
                .run("private-owner", Duration::from_secs(30), true)
                .expect("start recorded");
            let error = service
                .stop(&lease.token, "private-owner")
                .expect_err("stop audit failure");
            assert!(
                matches!(error, KeepAwakeError::AuditFailed { after_dispatch, .. } if after_dispatch == (fail_at == 4))
            );
            let releases = calls
                .lock()
                .expect("calls")
                .iter()
                .filter(|call| matches!(call, BackendCall::Release(_)))
                .count();
            assert_eq!(releases, usize::from(fail_at == 4));
            if fail_at == 3 {
                assert_eq!(
                    service
                        .status(&lease.token, "private-owner")
                        .expect("still held")
                        .state,
                    "active"
                );
                service.shutdown().expect("exit still clears");
            } else {
                assert!(matches!(
                    service.status(&lease.token, "private-owner"),
                    Err(KeepAwakeError::AuditFailed {
                        after_dispatch: true,
                        ..
                    })
                ));
                assert!(matches!(
                    service.shutdown(),
                    Err(KeepAwakeError::AuditFailed {
                        after_dispatch: true,
                        ..
                    })
                ));
            }
        }
    }

    #[test]
    fn keep_awake_expiry_clears_even_when_lifecycle_audit_is_unavailable_without_claiming_off() {
        let clock = Arc::new(FakeClock::new());
        let (backend, calls) = RecordingBackend::pair();
        let service = KeepAwakeService::new_with_audit(
            clock.clone(),
            Box::new(backend),
            true,
            Box::new(FailAppend {
                calls: 0,
                fail_at: 3,
                records: RecordingKeepAwakeAudit::default(),
            }),
        );
        let lease = service
            .run("owner", Duration::from_millis(5), true)
            .expect("start");
        clock.advance(Duration::from_millis(5));
        assert!(matches!(
            service.status(&lease.token, "owner"),
            Err(KeepAwakeError::AuditFailed {
                after_dispatch: true,
                ..
            })
        ));
        assert!(
            calls
                .lock()
                .expect("calls")
                .iter()
                .any(|call| matches!(call, BackendCall::Release(token) if token == &lease.token))
        );
        assert!(
            service.shutdown().is_err(),
            "join must preserve unconfirmed audit result"
        );
    }

    #[test]
    fn keep_awake_file_history_is_durable_redacted_and_rejects_a_bad_store_before_acquisition() {
        let temp = tempfile::TempDir::new().expect("isolated audit");
        let path = temp.path().join("DevSweep/audit/v1/status.jsonl");
        let (backend, calls) = RecordingBackend::pair();
        let service = KeepAwakeService::new_with_audit(
            Arc::new(FakeClock::new()),
            Box::new(backend),
            true,
            Box::new(FileKeepAwakeAudit::at(path.clone())),
        );
        let lease = service
            .run("private-owner", Duration::from_secs(30), true)
            .expect("start");
        service.stop(&lease.token, "private-owner").expect("stop");
        service.shutdown().expect("join");
        let text = fs::read_to_string(&path).expect("durable journal");
        let records: Vec<StatusActionAuditRecordV1> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("closed V1"))
            .collect();
        assert_eq!(records.len(), 4);
        assert_eq!(records[0].action, KeepAwakeAuditAction::Start);
        assert_eq!(records[3].action, KeepAwakeAuditAction::Stop);
        assert_eq!(records[3].status_code, StatusActionAuditCode::Succeeded);
        for secret in [&lease.token, &lease.owner] {
            assert!(!text.contains(secret));
        }
        for key in ["token", "owner", "endpoint", "argv", "program", "message"] {
            assert!(!text.contains(&format!("\"{key}\"")), "{key}");
        }
        let before = calls.lock().expect("calls").len();
        fs::write(&path, "{bad-json}\n").expect("corrupt isolated journal");
        let (backend, fresh_calls) = RecordingBackend::pair();
        let service = KeepAwakeService::new_with_audit(
            Arc::new(FakeClock::new()),
            Box::new(backend),
            true,
            Box::new(FileKeepAwakeAudit::at(path)),
        );
        assert!(matches!(
            service.run("owner", Duration::from_secs(1), true),
            Err(KeepAwakeError::AuditFailed {
                after_dispatch: false,
                ..
            })
        ));
        assert!(fresh_calls.lock().expect("fresh calls").is_empty());
        assert_eq!(calls.lock().expect("calls").len(), before);
    }

    #[test]
    fn keep_awake_file_audit_lock_and_read_limit_refuse_before_os_calls() {
        let temp = tempfile::TempDir::new().expect("isolated journal");
        let path = temp.path().join("status.jsonl");
        let (backend, calls) = RecordingBackend::pair();
        let service = KeepAwakeService::new_with_audit(
            Arc::new(FakeClock::new()),
            Box::new(backend),
            true,
            Box::new(FileKeepAwakeAudit::at(path.clone())),
        );
        let lock = AuditLock::acquire(&temp.path().join("status.lock")).expect("held lock");
        assert!(matches!(
            service.run("owner", Duration::from_secs(1), true),
            Err(KeepAwakeError::AuditFailed {
                after_dispatch: false,
                ..
            })
        ));
        assert!(calls.lock().expect("calls").is_empty());
        drop(lock);
        File::create(&path)
            .expect("file")
            .set_len(MAX_STATUS_AUDIT_BYTES + 1)
            .expect("oversized store");
        assert!(matches!(
            service.run("owner", Duration::from_secs(1), true),
            Err(KeepAwakeError::AuditFailed {
                after_dispatch: false,
                ..
            })
        ));
        assert!(calls.lock().expect("calls").is_empty());
        service.shutdown().expect("joined without acquisition");
    }

    #[test]
    fn keep_awake_owner_exit_and_cancel_clear_despite_audit_failure() {
        for cancel in [false, true] {
            for fail_at in [3, 4] {
                let (backend, calls) = RecordingBackend::pair();
                let service = KeepAwakeService::new_with_audit(
                    Arc::new(FakeClock::new()),
                    Box::new(backend),
                    true,
                    Box::new(FailAppend {
                        calls: 0,
                        fail_at,
                        records: RecordingKeepAwakeAudit::default(),
                    }),
                );
                let lease = service
                    .run("owner", Duration::from_secs(30), true)
                    .expect("start");
                let result = if cancel {
                    service.cancel(&lease.token, "owner").map(|_| ())
                } else {
                    service.owner_exit("owner").map(|_| ())
                };
                assert!(matches!(
                    result,
                    Err(KeepAwakeError::AuditFailed {
                        after_dispatch: true,
                        ..
                    })
                ));
                assert_eq!(
                    calls
                        .lock()
                        .expect("calls")
                        .iter()
                        .filter(|call| matches!(call, BackendCall::Release(_)))
                        .count(),
                    1
                );
                assert!(
                    service.shutdown().is_err(),
                    "unknown audit result must survive join"
                );
            }
        }
    }

    #[test]
    fn keep_awake_audit_failure_preserves_the_simultaneous_native_clear_failure() {
        struct FailingClear;
        impl crate::status::AwakeBackend for FailingClear {
            fn acquire(&mut self, _: &str) -> Result<(), KeepAwakeError> {
                Ok(())
            }
            fn release(&mut self, _: &str) -> Result<(), KeepAwakeError> {
                Err(KeepAwakeError::BackendFailed {
                    operation: "release",
                    message: "native-clear-fixture".into(),
                })
            }
        }
        let service = KeepAwakeService::new_with_audit(
            Arc::new(FakeClock::new()),
            Box::new(FailingClear),
            true,
            Box::new(FailAppend {
                calls: 0,
                fail_at: 4,
                records: RecordingKeepAwakeAudit::default(),
            }),
        );
        let lease = service
            .run("owner", Duration::from_secs(30), true)
            .expect("start");
        let error = service
            .stop(&lease.token, "owner")
            .expect_err("both failed");
        assert_eq!(error.code(), "unknown_after_dispatch");
        assert!(error.to_string().contains("injected append failure"));
        assert!(error.to_string().contains("native-clear-fixture"));
        assert!(service.shutdown().is_err());
    }
}
