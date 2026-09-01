//! Locked, append-only, redacted Optimize V1 execution journal.

use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::catalogue::{MaintenanceActionClass, OPTIMIZE_CATALOGUE_VERSION, catalogue_entry};
use super::execution::{AdapterOutcome, MaintenanceExecutionOutcome};
use super::plan::is_sha256;

pub const OPTIMIZE_AUDIT_VERSION: u32 = 1;
const OPTIMIZE_AUDIT_DOMAIN: &str = "optimize";

/// Closed Optimize V1 audit status codes. No localized or process-owned text
/// is ever a status: identity is the catalogue id plus these stable codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizeAuditStatusCode {
    Validated,
    DispatchStarted,
    AdapterSucceeded,
    AdapterFailed,
    AdapterUnfinished,
    CanceledBeforeStart,
    RecoveredBeforeDispatch,
    Succeeded,
    Launched,
    Failed,
    UnknownAfterDispatch,
}

/// Closed Optimize V1 audit error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizeAuditErrorCode {
    AdapterDispatchFailed,
    AdapterOperationFailed,
    AdapterTimedOut,
    AdapterCanceledAfterDispatch,
    RecoveredAfterCrash,
}

/// One closed Optimize audit transition kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OptimizeAuditTransition {
    Validated,
    DispatchStarted,
    AdapterCompleted,
    Terminal {
        outcome: MaintenanceExecutionOutcome,
    },
}

impl OptimizeAuditTransition {
    pub(super) fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal { .. })
    }
}

/// One locale-neutral Optimize audit transition. It intentionally has no
/// program path, argv, URI, environment, process output, or plan payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizeAuditRecordV1 {
    pub schema_version: u32,
    pub domain: String,
    pub operation_id: String,
    pub timestamp_unix_ms: u64,
    pub catalogue_version: u32,
    pub catalogue_id: String,
    pub action_class: MaintenanceActionClass,
    pub preview_digest: String,
    pub transition: OptimizeAuditTransition,
    pub status_code: OptimizeAuditStatusCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<OptimizeAuditErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_outcome: Option<AdapterOutcome>,
}

impl OptimizeAuditRecordV1 {
    pub(super) fn new(
        operation_id: &str,
        timestamp_unix_ms: u64,
        catalogue_id: &str,
        action_class: super::catalogue::MaintenanceActionClass,
        preview_digest: &str,
        event: OptimizeAuditEvent,
    ) -> Self {
        Self {
            schema_version: OPTIMIZE_AUDIT_VERSION,
            domain: OPTIMIZE_AUDIT_DOMAIN.to_string(),
            operation_id: operation_id.to_string(),
            timestamp_unix_ms,
            catalogue_version: OPTIMIZE_CATALOGUE_VERSION,
            catalogue_id: catalogue_id.to_string(),
            action_class,
            preview_digest: preview_digest.to_string(),
            transition: event.transition,
            status_code: event.status_code,
            error_code: event.error_code,
            adapter_outcome: event.adapter_outcome,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OptimizeAuditEvent {
    pub transition: OptimizeAuditTransition,
    pub status_code: OptimizeAuditStatusCode,
    pub error_code: Option<OptimizeAuditErrorCode>,
    pub adapter_outcome: Option<AdapterOutcome>,
}

#[derive(Debug)]
pub enum OptimizeAuditError {
    LocalAppDataUnavailable,
    InvalidPath(PathBuf),
    LockUnavailable(PathBuf),
    UnknownVersion(PathBuf),
    Corrupt(PathBuf),
    InvalidTransition {
        operation_id: String,
        detail: &'static str,
    },
    Io {
        stage: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

impl std::fmt::Display for OptimizeAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalAppDataUnavailable => {
                formatter.write_str("LOCALAPPDATA is unavailable for Optimize audit persistence")
            }
            Self::InvalidPath(path) => {
                write!(formatter, "invalid Optimize audit path {}", path.display())
            }
            Self::LockUnavailable(path) => {
                write!(
                    formatter,
                    "Optimize audit lock is unavailable at {}",
                    path.display()
                )
            }
            Self::UnknownVersion(path) => write!(
                formatter,
                "Optimize audit journal at {} has an unknown version",
                path.display()
            ),
            Self::Corrupt(path) => {
                write!(
                    formatter,
                    "Optimize audit journal at {} is corrupt",
                    path.display()
                )
            }
            Self::InvalidTransition {
                operation_id,
                detail,
            } => write!(
                formatter,
                "invalid Optimize audit transition for {operation_id}: {detail}"
            ),
            Self::Io {
                stage,
                path,
                source,
            } => write!(
                formatter,
                "Optimize audit {stage} failed for {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for OptimizeAuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn io_error(stage: &'static str, path: &Path, source: io::Error) -> OptimizeAuditError {
    OptimizeAuditError::Io {
        stage,
        path: path.to_path_buf(),
        source,
    }
}

/// Resolve the sole supported Optimize V1 journal path.
pub fn optimize_audit_v1_path() -> Result<PathBuf, OptimizeAuditError> {
    optimize_audit_v1_path_from(std::env::var_os("LOCALAPPDATA"))
}

pub(super) fn optimize_audit_v1_path_from(
    local_app_data: Option<OsString>,
) -> Result<PathBuf, OptimizeAuditError> {
    let root = local_app_data
        .filter(|value| !value.is_empty())
        .ok_or(OptimizeAuditError::LocalAppDataUnavailable)?;
    Ok(PathBuf::from(root)
        .join("DevSweep")
        .join("audit")
        .join("v1")
        .join("optimize.jsonl"))
}

#[derive(Debug, Clone)]
pub(super) struct PendingOperation {
    pub operation_id: String,
    pub catalogue_id: String,
    pub action_class: MaintenanceActionClass,
    pub preview_digest: String,
    pub last_transition: OptimizeAuditTransition,
}

#[derive(Debug, Clone)]
struct OperationState {
    catalogue_id: String,
    action_class: MaintenanceActionClass,
    preview_digest: String,
    last_transition: OptimizeAuditTransition,
    adapter_outcome: Option<AdapterOutcome>,
}

impl OperationState {
    fn pending(&self, operation_id: &str) -> Option<PendingOperation> {
        (!self.last_transition.is_terminal()).then(|| PendingOperation {
            operation_id: operation_id.to_string(),
            catalogue_id: self.catalogue_id.clone(),
            action_class: self.action_class,
            preview_digest: self.preview_digest.clone(),
            last_transition: self.last_transition,
        })
    }
}

pub(super) struct OptimizeAuditJournal {
    path: PathBuf,
    file: File,
    _lock: SidecarLock,
    states: BTreeMap<String, OperationState>,
}

impl OptimizeAuditJournal {
    pub(super) fn open(path: &Path) -> Result<Self, OptimizeAuditError> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| OptimizeAuditError::InvalidPath(path.to_path_buf()))?;
        fs::create_dir_all(parent).map_err(|error| io_error("create_directory", parent, error))?;
        let lock_path = parent.join("optimize.lock");
        let lock = SidecarLock::acquire(&lock_path)?;

        let bytes = if path
            .try_exists()
            .map_err(|error| io_error("probe", path, error))?
        {
            fs::read(path).map_err(|error| io_error("read", path, error))?
        } else {
            Vec::new()
        };
        let states = parse_journal(path, &bytes)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)
            .map_err(|error| io_error("open", path, error))?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            _lock: lock,
            states,
        })
    }

    pub(super) fn append(
        &mut self,
        record: OptimizeAuditRecordV1,
    ) -> Result<(), OptimizeAuditError> {
        validate_record(&record)?;
        let mut next = self.states.clone();
        apply_record(&mut next, &record)?;
        let mut line = serde_json::to_vec(&record)
            .map_err(|_| OptimizeAuditError::Corrupt(self.path.clone()))?;
        line.push(b'\n');
        self.file
            .write_all(&line)
            .map_err(|error| io_error("append", &self.path, error))?;
        self.file
            .flush()
            .map_err(|error| io_error("flush", &self.path, error))?;
        self.file
            .sync_all()
            .map_err(|error| io_error("sync", &self.path, error))?;
        self.states = next;
        Ok(())
    }

    pub(super) fn pending_operations(&self) -> Vec<PendingOperation> {
        self.states
            .iter()
            .filter_map(|(operation_id, state)| state.pending(operation_id))
            .collect()
    }
}

fn parse_journal(
    path: &Path,
    bytes: &[u8],
) -> Result<BTreeMap<String, OperationState>, OptimizeAuditError> {
    if bytes.is_empty() {
        return Ok(BTreeMap::new());
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| OptimizeAuditError::Corrupt(path.to_path_buf()))?;
    if !text.ends_with('\n') {
        return Err(OptimizeAuditError::Corrupt(path.to_path_buf()));
    }
    let mut states = BTreeMap::new();
    for line in text.lines() {
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|_| OptimizeAuditError::Corrupt(path.to_path_buf()))?;
        match value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
        {
            Some(version) if version == u64::from(OPTIMIZE_AUDIT_VERSION) => {}
            Some(_) => return Err(OptimizeAuditError::UnknownVersion(path.to_path_buf())),
            None => return Err(OptimizeAuditError::Corrupt(path.to_path_buf())),
        }
        if value.get("domain").and_then(serde_json::Value::as_str) != Some(OPTIMIZE_AUDIT_DOMAIN) {
            return Err(OptimizeAuditError::Corrupt(path.to_path_buf()));
        }
        let record: OptimizeAuditRecordV1 = serde_json::from_value(value)
            .map_err(|_| OptimizeAuditError::Corrupt(path.to_path_buf()))?;
        validate_record(&record)?;
        apply_record(&mut states, &record)?;
    }
    Ok(states)
}

fn validate_record(record: &OptimizeAuditRecordV1) -> Result<(), OptimizeAuditError> {
    if record.schema_version != OPTIMIZE_AUDIT_VERSION
        || record.domain != OPTIMIZE_AUDIT_DOMAIN
        || record.operation_id.is_empty()
        || !is_sha256(&record.preview_digest)
    {
        return Err(OptimizeAuditError::InvalidTransition {
            operation_id: record.operation_id.clone(),
            detail: "record shape is not Optimize V1",
        });
    }
    // Identity is closed: the catalogue id must exist and its class must match.
    let entry = catalogue_entry(&record.catalogue_id).ok_or_else(|| {
        OptimizeAuditError::InvalidTransition {
            operation_id: record.operation_id.clone(),
            detail: "record catalogue id is not in the closed catalogue",
        }
    })?;
    if entry.action_class != record.action_class
        || record.catalogue_version != OPTIMIZE_CATALOGUE_VERSION
    {
        return Err(OptimizeAuditError::InvalidTransition {
            operation_id: record.operation_id.clone(),
            detail: "record catalogue identity is inconsistent",
        });
    }
    if record.action_class == MaintenanceActionClass::Guidance {
        return Err(OptimizeAuditError::InvalidTransition {
            operation_id: record.operation_id.clone(),
            detail: "guidance rows have no operation journal",
        });
    }
    let transition_shape_valid = match record.transition {
        OptimizeAuditTransition::Validated => {
            record.status_code == OptimizeAuditStatusCode::Validated
                && record.adapter_outcome.is_none()
                && record.error_code.is_none()
        }
        OptimizeAuditTransition::DispatchStarted => {
            record.status_code == OptimizeAuditStatusCode::DispatchStarted
                && record.adapter_outcome.is_none()
                && record.error_code.is_none()
        }
        OptimizeAuditTransition::AdapterCompleted => {
            matches!(
                (record.adapter_outcome, record.status_code),
                (
                    Some(AdapterOutcome::Success),
                    OptimizeAuditStatusCode::AdapterSucceeded
                ) | (
                    Some(AdapterOutcome::Failure),
                    OptimizeAuditStatusCode::AdapterFailed
                ) | (
                    Some(AdapterOutcome::Unfinished),
                    OptimizeAuditStatusCode::AdapterUnfinished
                )
            ) && record.error_code.is_none()
        }
        OptimizeAuditTransition::Terminal {
            outcome: MaintenanceExecutionOutcome::CanceledBeforeStart,
        } => {
            matches!(
                record.status_code,
                OptimizeAuditStatusCode::CanceledBeforeStart
                    | OptimizeAuditStatusCode::RecoveredBeforeDispatch
            ) && record.adapter_outcome.is_none()
        }
        OptimizeAuditTransition::Terminal { outcome } => {
            record.status_code
                == match outcome {
                    MaintenanceExecutionOutcome::Succeeded => OptimizeAuditStatusCode::Succeeded,
                    MaintenanceExecutionOutcome::Launched => OptimizeAuditStatusCode::Launched,
                    MaintenanceExecutionOutcome::Failed => OptimizeAuditStatusCode::Failed,
                    MaintenanceExecutionOutcome::UnknownAfterDispatch => {
                        OptimizeAuditStatusCode::UnknownAfterDispatch
                    }
                    MaintenanceExecutionOutcome::CanceledBeforeStart => {
                        OptimizeAuditStatusCode::CanceledBeforeStart
                    }
                }
                && classify_terminal_evidence(record.adapter_outcome, record.action_class)
                    == Some(outcome)
        }
    };
    if !transition_shape_valid {
        return Err(OptimizeAuditError::InvalidTransition {
            operation_id: record.operation_id.clone(),
            detail: "transition evidence shape is inconsistent",
        });
    }
    Ok(())
}

/// The terminal outcome implied by the recorded adapter evidence, or `None`
/// when the evidence cannot prove any terminal outcome.
fn classify_terminal_evidence(
    adapter_outcome: Option<AdapterOutcome>,
    action_class: MaintenanceActionClass,
) -> Option<MaintenanceExecutionOutcome> {
    match (action_class, adapter_outcome) {
        (MaintenanceActionClass::Execute, Some(AdapterOutcome::Success)) => {
            Some(MaintenanceExecutionOutcome::Succeeded)
        }
        (MaintenanceActionClass::SettingsHandoff, Some(AdapterOutcome::Success)) => {
            Some(MaintenanceExecutionOutcome::Launched)
        }
        (_, Some(AdapterOutcome::Failure)) => Some(MaintenanceExecutionOutcome::Failed),
        (_, Some(AdapterOutcome::Unfinished)) => {
            Some(MaintenanceExecutionOutcome::UnknownAfterDispatch)
        }
        _ => None,
    }
}

fn apply_record(
    states: &mut BTreeMap<String, OperationState>,
    record: &OptimizeAuditRecordV1,
) -> Result<(), OptimizeAuditError> {
    let operation_id = &record.operation_id;
    match states.get_mut(operation_id) {
        None => {
            if record.transition != OptimizeAuditTransition::Validated {
                return Err(OptimizeAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "first transition must be validated",
                });
            }
            states.insert(
                operation_id.clone(),
                OperationState {
                    catalogue_id: record.catalogue_id.clone(),
                    action_class: record.action_class,
                    preview_digest: record.preview_digest.clone(),
                    last_transition: record.transition,
                    adapter_outcome: record.adapter_outcome,
                },
            );
            Ok(())
        }
        Some(state) => {
            if state.catalogue_id != record.catalogue_id
                || state.action_class != record.action_class
                || state.preview_digest != record.preview_digest
            {
                return Err(OptimizeAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "operation identity changed",
                });
            }
            if state.last_transition.is_terminal() {
                return Err(OptimizeAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "transition after terminal",
                });
            }
            let valid = match (state.last_transition, record.transition) {
                (OptimizeAuditTransition::Validated, OptimizeAuditTransition::DispatchStarted)
                | (
                    OptimizeAuditTransition::Validated,
                    OptimizeAuditTransition::Terminal {
                        outcome: MaintenanceExecutionOutcome::CanceledBeforeStart,
                    },
                )
                | (
                    OptimizeAuditTransition::DispatchStarted,
                    OptimizeAuditTransition::AdapterCompleted,
                )
                | (
                    OptimizeAuditTransition::DispatchStarted,
                    OptimizeAuditTransition::Terminal {
                        outcome: MaintenanceExecutionOutcome::UnknownAfterDispatch,
                    },
                ) => true,
                (
                    OptimizeAuditTransition::AdapterCompleted,
                    OptimizeAuditTransition::Terminal { outcome },
                ) => outcome != MaintenanceExecutionOutcome::CanceledBeforeStart,
                _ => false,
            };
            if !valid {
                return Err(OptimizeAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "non-monotonic transition",
                });
            }
            if record.transition == OptimizeAuditTransition::AdapterCompleted
                && state.adapter_outcome.is_some()
            {
                return Err(OptimizeAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "adapter completion repeated",
                });
            }
            if let OptimizeAuditTransition::Terminal { outcome } = record.transition
                && outcome != MaintenanceExecutionOutcome::CanceledBeforeStart
            {
                let expected_adapter = state.adapter_outcome.unwrap_or(AdapterOutcome::Unfinished);
                if record.adapter_outcome != Some(expected_adapter) {
                    return Err(OptimizeAuditError::InvalidTransition {
                        operation_id: operation_id.clone(),
                        detail: "terminal does not match accumulated adapter evidence",
                    });
                }
            }
            state.last_transition = record.transition;
            if record.adapter_outcome.is_some() {
                state.adapter_outcome = record.adapter_outcome;
            }
            Ok(())
        }
    }
}

struct SidecarLock {
    _file: File,
}

impl SidecarLock {
    fn acquire(path: &Path) -> Result<Self, OptimizeAuditError> {
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(path)
                .map_err(|error| {
                    if matches!(error.raw_os_error(), Some(5 | 32 | 33))
                        || error.kind() == io::ErrorKind::PermissionDenied
                    {
                        OptimizeAuditError::LockUnavailable(path.to_path_buf())
                    } else {
                        io_error("acquire_lock", path, error)
                    }
                })?;
            Ok(Self { _file: file })
        }
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)
                .map_err(|error| io_error("open_lock", path, error))?;
            // SAFETY: `file` owns a valid descriptor for the lock lifetime.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                let error = io::Error::last_os_error();
                if matches!(error.raw_os_error(), Some(libc::EACCES | libc::EAGAIN)) {
                    return Err(OptimizeAuditError::LockUnavailable(path.to_path_buf()));
                }
                return Err(io_error("acquire_lock", path, error));
            }
            Ok(Self { _file: file })
        }
        #[cfg(not(any(windows, unix)))]
        {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|error| OptimizeAuditError::LockUnavailable(path.to_path_buf()))?;
            Ok(Self { _file: file })
        }
    }
}
