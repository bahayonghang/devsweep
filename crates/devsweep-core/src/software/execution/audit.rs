//! Locked, append-only, redacted Software V1 execution journal.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::execution::{
    AdapterEvidence, AdapterOutcome, SoftwareExecutionOutcome, SoftwareInstalledState,
    SoftwareRebootEvidence, classify_terminal, merge_requery_state,
};
use crate::software::{
    SoftwareIdentity, SoftwareLeftoverCertainty, SoftwareStartupLocation, SoftwareSupportErrorCode,
    SoftwareSupportOutcomeCode,
};

pub const SOFTWARE_AUDIT_VERSION: u32 = 1;
const SOFTWARE_AUDIT_DOMAIN: &str = "software";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareAuditStatusCode {
    Validated,
    DispatchStarted,
    AdapterSucceeded,
    AdapterFailed,
    AdapterRebootRequired,
    RequeryPresent,
    RequeryAbsent,
    RequeryUnavailable,
    RequeryConflicting,
    CanceledBeforeStart,
    Removed,
    RebootRequired,
    StillPresent,
    Failed,
    UnknownAfterDispatch,
    RecoveredBeforeDispatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareAuditErrorCode {
    AdapterDispatchFailed,
    AdapterOperationFailed,
    AdapterStatusUnavailable,
    AdapterTimedOut,
    AdapterCanceledAfterDispatch,
    AdapterCancelFailed,
    RequeryUnavailable,
    RequeryConflicting,
    RecoveredAfterCrash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareAuditRequeryResult {
    Present,
    Absent,
    Unavailable,
    Conflicting,
}

impl From<SoftwareInstalledState> for SoftwareAuditRequeryResult {
    fn from(value: SoftwareInstalledState) -> Self {
        match value {
            SoftwareInstalledState::Present => Self::Present,
            SoftwareInstalledState::Absent => Self::Absent,
            SoftwareInstalledState::Unavailable => Self::Unavailable,
            SoftwareInstalledState::Conflicting => Self::Conflicting,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoftwareAuditTransition {
    Validated,
    DispatchStarted,
    AdapterCompleted,
    RequeryObserved,
    Terminal { outcome: SoftwareExecutionOutcome },
}

impl SoftwareAuditTransition {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal { .. })
    }
}

/// One locale-neutral Software audit transition. It intentionally has no raw
/// adapter output, vendor text, argv, environment, path, or plan payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareAuditRecordV1 {
    pub schema_version: u32,
    pub domain: String,
    pub operation_id: String,
    pub timestamp_unix_ms: u64,
    pub identity: SoftwareIdentity,
    pub inventory_fingerprint: String,
    pub preview_digest: String,
    pub transition: SoftwareAuditTransition,
    pub status_code: SoftwareAuditStatusCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<SoftwareAuditErrorCode>,
    pub reboot_evidence: SoftwareRebootEvidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_state: Option<SoftwareInstalledState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requery_result: Option<SoftwareAuditRequeryResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_outcome: Option<AdapterOutcome>,
    pub irreversible: bool,
}

impl SoftwareAuditRecordV1 {
    pub(super) fn new(
        operation_id: &str,
        timestamp_unix_ms: u64,
        identity: &SoftwareIdentity,
        inventory_fingerprint: &str,
        preview_digest: &str,
        event: SoftwareAuditEvent,
    ) -> Self {
        Self {
            schema_version: SOFTWARE_AUDIT_VERSION,
            domain: SOFTWARE_AUDIT_DOMAIN.to_string(),
            operation_id: operation_id.to_string(),
            timestamp_unix_ms,
            identity: identity.clone(),
            inventory_fingerprint: inventory_fingerprint.to_string(),
            preview_digest: preview_digest.to_string(),
            transition: event.transition,
            status_code: event.status_code,
            error_code: event.error_code,
            reboot_evidence: event.reboot_evidence,
            installed_state: event.installed_state,
            requery_result: event.installed_state.map(SoftwareAuditRequeryResult::from),
            adapter_outcome: event.adapter_outcome,
            irreversible: true,
        }
    }
}

/// One complete Software support action in the same fixed journal. These
/// records are separate from MSIX removal transitions and carry no path,
/// command line, argv, or localized text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "record_kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoftwareSupportAuditRecordV1 {
    /// One current-user `StartupApproved` value write and its re-read result.
    StartupToggled {
        schema_version: u32,
        domain: String,
        operation_id: String,
        timestamp_unix_ms: u64,
        startup_id: String,
        location: SoftwareStartupLocation,
        requested_enabled: bool,
        outcome_code: SoftwareSupportOutcomeCode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_code: Option<SoftwareSupportErrorCode>,
    },
    /// One leftover directory move to the Recycle Bin after a succeeded uninstall.
    LeftoverMoved {
        schema_version: u32,
        domain: String,
        operation_id: String,
        timestamp_unix_ms: u64,
        uninstall_operation_id: String,
        identity: SoftwareIdentity,
        candidate_id: String,
        certainty: SoftwareLeftoverCertainty,
        outcome_code: SoftwareSupportOutcomeCode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        estimated_bytes: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_code: Option<SoftwareSupportErrorCode>,
    },
}

impl SoftwareSupportAuditRecordV1 {
    pub(in crate::software) fn startup_toggled(
        operation_id: &str,
        timestamp_unix_ms: u64,
        startup_id: &str,
        location: SoftwareStartupLocation,
        requested_enabled: bool,
        outcome_code: SoftwareSupportOutcomeCode,
        error_code: Option<SoftwareSupportErrorCode>,
    ) -> Self {
        Self::StartupToggled {
            schema_version: SOFTWARE_AUDIT_VERSION,
            domain: SOFTWARE_AUDIT_DOMAIN.to_string(),
            operation_id: operation_id.to_string(),
            timestamp_unix_ms,
            startup_id: startup_id.to_string(),
            location,
            requested_enabled,
            outcome_code,
            error_code,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::software) fn leftover_moved(
        operation_id: &str,
        timestamp_unix_ms: u64,
        uninstall_operation_id: &str,
        identity: &SoftwareIdentity,
        candidate_id: &str,
        certainty: SoftwareLeftoverCertainty,
        outcome_code: SoftwareSupportOutcomeCode,
        estimated_bytes: Option<u64>,
        error_code: Option<SoftwareSupportErrorCode>,
    ) -> Self {
        Self::LeftoverMoved {
            schema_version: SOFTWARE_AUDIT_VERSION,
            domain: SOFTWARE_AUDIT_DOMAIN.to_string(),
            operation_id: operation_id.to_string(),
            timestamp_unix_ms,
            uninstall_operation_id: uninstall_operation_id.to_string(),
            identity: identity.clone(),
            candidate_id: candidate_id.to_string(),
            certainty,
            outcome_code,
            estimated_bytes,
            error_code,
        }
    }

    #[must_use]
    pub fn operation_id(&self) -> &str {
        match self {
            Self::StartupToggled { operation_id, .. }
            | Self::LeftoverMoved { operation_id, .. } => operation_id,
        }
    }

    #[must_use]
    pub fn timestamp_unix_ms(&self) -> u64 {
        match self {
            Self::StartupToggled {
                timestamp_unix_ms, ..
            }
            | Self::LeftoverMoved {
                timestamp_unix_ms, ..
            } => *timestamp_unix_ms,
        }
    }

    #[must_use]
    pub fn outcome_code(&self) -> SoftwareSupportOutcomeCode {
        match self {
            Self::StartupToggled { outcome_code, .. }
            | Self::LeftoverMoved { outcome_code, .. } => *outcome_code,
        }
    }

    fn header(&self) -> (u32, &str) {
        match self {
            Self::StartupToggled {
                schema_version,
                domain,
                ..
            }
            | Self::LeftoverMoved {
                schema_version,
                domain,
                ..
            } => (*schema_version, domain),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SoftwareAuditEvent {
    pub transition: SoftwareAuditTransition,
    pub status_code: SoftwareAuditStatusCode,
    pub error_code: Option<SoftwareAuditErrorCode>,
    pub reboot_evidence: SoftwareRebootEvidence,
    pub installed_state: Option<SoftwareInstalledState>,
    pub adapter_outcome: Option<AdapterOutcome>,
}

#[derive(Debug)]
pub enum SoftwareAuditError {
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

impl std::fmt::Display for SoftwareAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalAppDataUnavailable => {
                formatter.write_str("LOCALAPPDATA is unavailable for Software audit persistence")
            }
            Self::InvalidPath(path) => {
                write!(formatter, "invalid Software audit path {}", path.display())
            }
            Self::LockUnavailable(path) => {
                write!(
                    formatter,
                    "Software audit lock is unavailable at {}",
                    path.display()
                )
            }
            Self::UnknownVersion(path) => write!(
                formatter,
                "Software audit journal at {} has an unknown version",
                path.display()
            ),
            Self::Corrupt(path) => {
                write!(
                    formatter,
                    "Software audit journal at {} is corrupt",
                    path.display()
                )
            }
            Self::InvalidTransition {
                operation_id,
                detail,
            } => write!(
                formatter,
                "invalid Software audit transition for {operation_id}: {detail}"
            ),
            Self::Io {
                stage,
                path,
                source,
            } => write!(
                formatter,
                "Software audit {stage} failed for {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SoftwareAuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn io_error(stage: &'static str, path: &Path, source: io::Error) -> SoftwareAuditError {
    SoftwareAuditError::Io {
        stage,
        path: path.to_path_buf(),
        source,
    }
}

pub fn software_audit_v1_path() -> Result<PathBuf, SoftwareAuditError> {
    software_audit_v1_path_from(std::env::var_os("LOCALAPPDATA"))
}

fn software_audit_v1_path_from(
    local_app_data: Option<OsString>,
) -> Result<PathBuf, SoftwareAuditError> {
    let root = local_app_data
        .filter(|value| !value.is_empty())
        .ok_or(SoftwareAuditError::LocalAppDataUnavailable)?;
    Ok(PathBuf::from(root)
        .join("DevSweep")
        .join("audit")
        .join("v1")
        .join("software.jsonl"))
}

#[derive(Debug, Clone)]
pub(super) struct PendingOperation {
    pub operation_id: String,
    pub identity: SoftwareIdentity,
    pub inventory_fingerprint: String,
    pub preview_digest: String,
    pub last_transition: SoftwareAuditTransition,
    pub adapter_outcome: Option<AdapterOutcome>,
    pub reboot_evidence: SoftwareRebootEvidence,
}

#[derive(Debug, Clone)]
struct OperationState {
    identity: SoftwareIdentity,
    inventory_fingerprint: String,
    preview_digest: String,
    last_transition: SoftwareAuditTransition,
    adapter_outcome: Option<AdapterOutcome>,
    reboot_evidence: SoftwareRebootEvidence,
    installed_state: Option<SoftwareInstalledState>,
}

impl OperationState {
    fn pending(&self, operation_id: &str) -> Option<PendingOperation> {
        (!self.last_transition.is_terminal()).then(|| PendingOperation {
            operation_id: operation_id.to_string(),
            identity: self.identity.clone(),
            inventory_fingerprint: self.inventory_fingerprint.clone(),
            preview_digest: self.preview_digest.clone(),
            last_transition: self.last_transition,
            adapter_outcome: self.adapter_outcome,
            reboot_evidence: self.reboot_evidence,
        })
    }
}

pub(in crate::software) struct SoftwareAuditJournal {
    path: PathBuf,
    file: File,
    _lock: SidecarLock,
    states: BTreeMap<String, OperationState>,
    support_operation_ids: BTreeSet<String>,
}

impl SoftwareAuditJournal {
    pub(in crate::software) fn open(path: &Path) -> Result<Self, SoftwareAuditError> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| SoftwareAuditError::InvalidPath(path.to_path_buf()))?;
        fs::create_dir_all(parent).map_err(|error| io_error("create_directory", parent, error))?;
        let lock_path = parent.join("software.lock");
        let lock = SidecarLock::acquire(&lock_path)?;

        let bytes = if path
            .try_exists()
            .map_err(|error| io_error("probe", path, error))?
        {
            fs::read(path).map_err(|error| io_error("read", path, error))?
        } else {
            Vec::new()
        };
        let (records, states, support_operation_ids) = parse_journal(path, &bytes)?;
        let _ = records;
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
            support_operation_ids,
        })
    }

    pub(super) fn append(
        &mut self,
        record: SoftwareAuditRecordV1,
    ) -> Result<(), SoftwareAuditError> {
        validate_record(&record)?;
        if self.support_operation_ids.contains(&record.operation_id) {
            return Err(SoftwareAuditError::InvalidTransition {
                operation_id: record.operation_id.clone(),
                detail: "operation id belongs to a support record",
            });
        }
        let mut next = self.states.clone();
        apply_record(&mut next, &record)?;
        let line = serde_json::to_vec(&record)
            .map_err(|_| SoftwareAuditError::Corrupt(self.path.clone()))?;
        self.write_line(line)?;
        self.states = next;
        Ok(())
    }

    /// Appends one durable support record. Leftover records require a
    /// succeeded uninstall terminal for the same identity.
    pub(in crate::software) fn append_support(
        &mut self,
        record: SoftwareSupportAuditRecordV1,
    ) -> Result<(), SoftwareAuditError> {
        validate_support_record(&self.states, &record)?;
        let line = serde_json::to_vec(&record)
            .map_err(|_| SoftwareAuditError::Corrupt(self.path.clone()))?;
        self.write_line(line)?;
        self.support_operation_ids
            .insert(record.operation_id().to_string());
        Ok(())
    }

    /// True when `operation_id` reached `removed` or `reboot_required` for
    /// exactly `identity`. Leftover execution uses [`Self::uninstall_removed`].
    #[allow(dead_code)]
    pub(in crate::software) fn uninstall_succeeded(
        &self,
        operation_id: &str,
        identity: &SoftwareIdentity,
    ) -> bool {
        uninstall_succeeded(&self.states, operation_id, identity)
    }

    /// True when `operation_id` reached exactly `removed` for `identity`.
    pub(in crate::software) fn uninstall_removed(
        &self,
        operation_id: &str,
        identity: &SoftwareIdentity,
    ) -> bool {
        uninstall_removed(&self.states, operation_id, identity)
    }

    fn write_line(&mut self, mut line: Vec<u8>) -> Result<(), SoftwareAuditError> {
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
        Ok(())
    }

    pub(super) fn pending_operations(&self) -> Vec<PendingOperation> {
        self.states
            .iter()
            .filter_map(|(operation_id, state)| state.pending(operation_id))
            .collect()
    }
}

type ParsedJournal = (
    Vec<SoftwareAuditRecordV1>,
    BTreeMap<String, OperationState>,
    BTreeSet<String>,
);

fn parse_journal(path: &Path, bytes: &[u8]) -> Result<ParsedJournal, SoftwareAuditError> {
    if bytes.is_empty() {
        return Ok((Vec::new(), BTreeMap::new(), BTreeSet::new()));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| SoftwareAuditError::Corrupt(path.to_path_buf()))?;
    if !text.ends_with('\n') {
        return Err(SoftwareAuditError::Corrupt(path.to_path_buf()));
    }
    let mut records = Vec::new();
    let mut states = BTreeMap::new();
    let mut support_operation_ids = BTreeSet::new();
    for line in text.lines() {
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|_| SoftwareAuditError::Corrupt(path.to_path_buf()))?;
        match value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
        {
            Some(version) if version == u64::from(SOFTWARE_AUDIT_VERSION) => {}
            Some(_) => return Err(SoftwareAuditError::UnknownVersion(path.to_path_buf())),
            None => return Err(SoftwareAuditError::Corrupt(path.to_path_buf())),
        }
        if value.get("domain").and_then(serde_json::Value::as_str) != Some(SOFTWARE_AUDIT_DOMAIN) {
            return Err(SoftwareAuditError::Corrupt(path.to_path_buf()));
        }
        if value.get("record_kind").is_some() {
            let record: SoftwareSupportAuditRecordV1 = serde_json::from_value(value)
                .map_err(|_| SoftwareAuditError::Corrupt(path.to_path_buf()))?;
            validate_support_record(&states, &record)?;
            support_operation_ids.insert(record.operation_id().to_string());
            continue;
        }
        let record: SoftwareAuditRecordV1 = serde_json::from_value(value)
            .map_err(|_| SoftwareAuditError::Corrupt(path.to_path_buf()))?;
        validate_record(&record)?;
        if support_operation_ids.contains(&record.operation_id) {
            return Err(SoftwareAuditError::Corrupt(path.to_path_buf()));
        }
        apply_record(&mut states, &record)?;
        records.push(record);
    }
    Ok((records, states, support_operation_ids))
}

#[allow(dead_code)]
fn uninstall_succeeded(
    states: &BTreeMap<String, OperationState>,
    operation_id: &str,
    identity: &SoftwareIdentity,
) -> bool {
    states.get(operation_id).is_some_and(|state| {
        &state.identity == identity
            && matches!(
                state.last_transition,
                SoftwareAuditTransition::Terminal { outcome } if outcome.is_success()
            )
    })
}

fn uninstall_removed(
    states: &BTreeMap<String, OperationState>,
    operation_id: &str,
    identity: &SoftwareIdentity,
) -> bool {
    states.get(operation_id).is_some_and(|state| {
        &state.identity == identity
            && matches!(
                state.last_transition,
                SoftwareAuditTransition::Terminal {
                    outcome: SoftwareExecutionOutcome::Removed
                }
            )
    })
}

fn validate_support_record(
    states: &BTreeMap<String, OperationState>,
    record: &SoftwareSupportAuditRecordV1,
) -> Result<(), SoftwareAuditError> {
    let invalid = |detail| SoftwareAuditError::InvalidTransition {
        operation_id: record.operation_id().to_string(),
        detail,
    };
    let (schema_version, domain) = record.header();
    if schema_version != SOFTWARE_AUDIT_VERSION
        || domain != SOFTWARE_AUDIT_DOMAIN
        || record.operation_id().is_empty()
        || states.contains_key(record.operation_id())
    {
        return Err(invalid("support record shape is not Software V1"));
    }
    match record {
        SoftwareSupportAuditRecordV1::StartupToggled {
            startup_id,
            location,
            outcome_code,
            error_code,
            ..
        } => {
            let current_user = matches!(
                location,
                SoftwareStartupLocation::CurrentUserRun
                    | SoftwareStartupLocation::CurrentUserStartupFolder
            );
            let consistent = match outcome_code {
                SoftwareSupportOutcomeCode::Succeeded => error_code.is_none(),
                SoftwareSupportOutcomeCode::Failed => error_code.is_some(),
                SoftwareSupportOutcomeCode::Skipped => false,
            };
            if startup_id.is_empty() || !current_user || !consistent {
                return Err(invalid("startup record is inconsistent"));
            }
        }
        SoftwareSupportAuditRecordV1::LeftoverMoved {
            uninstall_operation_id,
            identity,
            candidate_id,
            outcome_code,
            estimated_bytes,
            error_code,
            ..
        } => {
            let consistent = match outcome_code {
                SoftwareSupportOutcomeCode::Succeeded => error_code.is_none(),
                SoftwareSupportOutcomeCode::Failed | SoftwareSupportOutcomeCode::Skipped => {
                    error_code.is_some() && estimated_bytes.is_none()
                }
            };
            if candidate_id.is_empty() || !consistent {
                return Err(invalid("leftover record is inconsistent"));
            }
            if !uninstall_removed(states, uninstall_operation_id, identity) {
                return Err(invalid("leftover record has no removed uninstall"));
            }
        }
    }
    Ok(())
}

fn validate_record(record: &SoftwareAuditRecordV1) -> Result<(), SoftwareAuditError> {
    if record.schema_version != SOFTWARE_AUDIT_VERSION
        || record.domain != SOFTWARE_AUDIT_DOMAIN
        || record.operation_id.is_empty()
        || !record.irreversible
        || !is_sha256(&record.inventory_fingerprint)
        || !is_sha256(&record.preview_digest)
    {
        return Err(SoftwareAuditError::InvalidTransition {
            operation_id: record.operation_id.clone(),
            detail: "record shape is not Software V1",
        });
    }
    if !valid_execution_identity(&record.identity) {
        return Err(SoftwareAuditError::InvalidTransition {
            operation_id: record.operation_id.clone(),
            detail: "record identity is not an executable software identity",
        });
    }
    let transition_shape_valid = match record.transition {
        SoftwareAuditTransition::Validated => {
            record.status_code == SoftwareAuditStatusCode::Validated
                && record.adapter_outcome.is_none()
                && record.installed_state.is_none()
                && record.requery_result.is_none()
                && record.reboot_evidence == SoftwareRebootEvidence::None
        }
        SoftwareAuditTransition::DispatchStarted => {
            record.status_code == SoftwareAuditStatusCode::DispatchStarted
                && record.adapter_outcome.is_none()
                && record.installed_state.is_none()
                && record.requery_result.is_none()
                && record.reboot_evidence == SoftwareRebootEvidence::None
        }
        SoftwareAuditTransition::AdapterCompleted => {
            matches!(
                (record.adapter_outcome, record.status_code),
                (
                    Some(AdapterOutcome::Success),
                    SoftwareAuditStatusCode::AdapterSucceeded
                ) | (
                    Some(AdapterOutcome::Failure),
                    SoftwareAuditStatusCode::AdapterFailed
                ) | (
                    Some(AdapterOutcome::RebootRequired),
                    SoftwareAuditStatusCode::AdapterRebootRequired
                )
            ) && record.installed_state.is_none()
                && record.requery_result.is_none()
                && record.reboot_evidence
                    == if record.adapter_outcome == Some(AdapterOutcome::RebootRequired) {
                        SoftwareRebootEvidence::Required
                    } else {
                        SoftwareRebootEvidence::None
                    }
        }
        SoftwareAuditTransition::RequeryObserved => {
            record.installed_state.is_some()
                && record.requery_result
                    == record.installed_state.map(SoftwareAuditRequeryResult::from)
                && record.adapter_outcome.is_none()
                && record.reboot_evidence == SoftwareRebootEvidence::None
                && record.status_code
                    == match record.installed_state {
                        Some(SoftwareInstalledState::Present) => {
                            SoftwareAuditStatusCode::RequeryPresent
                        }
                        Some(SoftwareInstalledState::Absent) => {
                            SoftwareAuditStatusCode::RequeryAbsent
                        }
                        Some(SoftwareInstalledState::Unavailable) => {
                            SoftwareAuditStatusCode::RequeryUnavailable
                        }
                        Some(SoftwareInstalledState::Conflicting) => {
                            SoftwareAuditStatusCode::RequeryConflicting
                        }
                        None => SoftwareAuditStatusCode::RequeryUnavailable,
                    }
        }
        SoftwareAuditTransition::Terminal {
            outcome: SoftwareExecutionOutcome::CanceledBeforeStart,
        } => {
            matches!(
                record.status_code,
                SoftwareAuditStatusCode::CanceledBeforeStart
                    | SoftwareAuditStatusCode::RecoveredBeforeDispatch
            ) && record.adapter_outcome.is_none()
                && record.installed_state.is_none()
                && record.requery_result.is_none()
                && record.reboot_evidence == SoftwareRebootEvidence::None
        }
        SoftwareAuditTransition::Terminal { outcome } => {
            record.adapter_outcome.is_some()
                && record.installed_state.is_some()
                && record.requery_result
                    == record.installed_state.map(SoftwareAuditRequeryResult::from)
                && record.status_code
                    == match outcome {
                        SoftwareExecutionOutcome::Removed => SoftwareAuditStatusCode::Removed,
                        SoftwareExecutionOutcome::RebootRequired => {
                            SoftwareAuditStatusCode::RebootRequired
                        }
                        SoftwareExecutionOutcome::StillPresent => {
                            SoftwareAuditStatusCode::StillPresent
                        }
                        SoftwareExecutionOutcome::Failed => SoftwareAuditStatusCode::Failed,
                        SoftwareExecutionOutcome::UnknownAfterDispatch => {
                            SoftwareAuditStatusCode::UnknownAfterDispatch
                        }
                        SoftwareExecutionOutcome::CanceledBeforeStart => {
                            SoftwareAuditStatusCode::CanceledBeforeStart
                        }
                    }
                && classify_terminal(
                    AdapterEvidence {
                        outcome: record.adapter_outcome.unwrap_or(AdapterOutcome::Unfinished),
                        reboot_evidence: record.reboot_evidence,
                        error_code: record.error_code,
                    },
                    record
                        .installed_state
                        .unwrap_or(SoftwareInstalledState::Unavailable),
                ) == outcome
        }
    };
    if !transition_shape_valid {
        return Err(SoftwareAuditError::InvalidTransition {
            operation_id: record.operation_id.clone(),
            detail: "transition evidence shape is inconsistent",
        });
    }
    Ok(())
}

fn valid_execution_identity(identity: &SoftwareIdentity) -> bool {
    match identity {
        SoftwareIdentity::Msix { package_full_name } => {
            super::plan::valid_package_full_name(package_full_name)
        }
        SoftwareIdentity::Msi { product_code, .. } => {
            super::launch::valid_product_code(product_code)
        }
        SoftwareIdentity::Arp { subkey, .. } => super::launch::valid_app_id(subkey),
    }
}

fn is_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn apply_record(
    states: &mut BTreeMap<String, OperationState>,
    record: &SoftwareAuditRecordV1,
) -> Result<(), SoftwareAuditError> {
    let operation_id = &record.operation_id;
    match states.get_mut(operation_id) {
        None => {
            if record.transition != SoftwareAuditTransition::Validated {
                return Err(SoftwareAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "first transition must be validated",
                });
            }
            states.insert(
                operation_id.clone(),
                OperationState {
                    identity: record.identity.clone(),
                    inventory_fingerprint: record.inventory_fingerprint.clone(),
                    preview_digest: record.preview_digest.clone(),
                    last_transition: record.transition,
                    adapter_outcome: record.adapter_outcome,
                    reboot_evidence: record.reboot_evidence,
                    installed_state: None,
                },
            );
            Ok(())
        }
        Some(state) => {
            if state.identity != record.identity
                || state.inventory_fingerprint != record.inventory_fingerprint
                || state.preview_digest != record.preview_digest
            {
                return Err(SoftwareAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "operation identity changed",
                });
            }
            if state.last_transition.is_terminal() {
                return Err(SoftwareAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "transition after terminal",
                });
            }
            let valid = match (state.last_transition, record.transition) {
                (SoftwareAuditTransition::Validated, SoftwareAuditTransition::DispatchStarted)
                | (
                    SoftwareAuditTransition::Validated,
                    SoftwareAuditTransition::Terminal {
                        outcome: SoftwareExecutionOutcome::CanceledBeforeStart,
                    },
                )
                | (
                    SoftwareAuditTransition::DispatchStarted,
                    SoftwareAuditTransition::AdapterCompleted,
                )
                | (
                    SoftwareAuditTransition::DispatchStarted,
                    SoftwareAuditTransition::RequeryObserved,
                )
                | (
                    SoftwareAuditTransition::AdapterCompleted,
                    SoftwareAuditTransition::RequeryObserved,
                )
                | (
                    SoftwareAuditTransition::RequeryObserved,
                    SoftwareAuditTransition::RequeryObserved,
                ) => true,
                (
                    SoftwareAuditTransition::RequeryObserved,
                    SoftwareAuditTransition::Terminal { outcome },
                ) => outcome != SoftwareExecutionOutcome::CanceledBeforeStart,
                _ => false,
            };
            if !valid {
                return Err(SoftwareAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "non-monotonic transition",
                });
            }
            if record.transition == SoftwareAuditTransition::AdapterCompleted
                && state.adapter_outcome.is_some()
            {
                return Err(SoftwareAuditError::InvalidTransition {
                    operation_id: operation_id.clone(),
                    detail: "adapter completion repeated",
                });
            }
            if record.transition == SoftwareAuditTransition::RequeryObserved {
                let observed = record.installed_state.ok_or_else(|| {
                    SoftwareAuditError::InvalidTransition {
                        operation_id: operation_id.clone(),
                        detail: "requery observation is missing installed state",
                    }
                })?;
                state.installed_state = Some(merge_requery_state(state.installed_state, observed));
            }
            if let SoftwareAuditTransition::Terminal { outcome } = record.transition
                && outcome != SoftwareExecutionOutcome::CanceledBeforeStart
            {
                let expected_adapter = state.adapter_outcome.unwrap_or(AdapterOutcome::Unfinished);
                if record.adapter_outcome != Some(expected_adapter)
                    || record.reboot_evidence != state.reboot_evidence
                    || record.installed_state != state.installed_state
                {
                    return Err(SoftwareAuditError::InvalidTransition {
                        operation_id: operation_id.clone(),
                        detail: "terminal does not match accumulated adapter and requery evidence",
                    });
                }
            }
            state.last_transition = record.transition;
            if record.adapter_outcome.is_some() {
                state.adapter_outcome = record.adapter_outcome;
            }
            if record.reboot_evidence == SoftwareRebootEvidence::Required {
                state.reboot_evidence = SoftwareRebootEvidence::Required;
            }
            Ok(())
        }
    }
}

struct SidecarLock {
    _file: File,
}

#[cfg(unix)]
impl Drop for SidecarLock {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;

        // A forked child can retain this open file description until exec.
        // Release the lock explicitly instead of waiting for the last descriptor
        // to close.
        // SAFETY: the guard still owns a valid descriptor for its acquired lock.
        unsafe { libc::flock(self._file.as_raw_fd(), libc::LOCK_UN) };
    }
}

impl SidecarLock {
    fn acquire(path: &Path) -> Result<Self, SoftwareAuditError> {
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
                        SoftwareAuditError::LockUnavailable(path.to_path_buf())
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
                    return Err(SoftwareAuditError::LockUnavailable(path.to_path_buf()));
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
                .map_err(|error| SoftwareAuditError::LockUnavailable(path.to_path_buf()))?;
            Ok(Self { _file: file })
        }
    }
}

pub(in crate::software) const SOFTWARE_SUPPORT_LIFECYCLE_VERSION: u32 = 1;
const SOFTWARE_SUPPORT_DOMAIN: &str = "software_support";

/// Strict startup and leftover operations. These records never enter
/// `software.jsonl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::software) enum SoftwareSupportOperation {
    StartupToggle,
    LeftoverMove,
}

/// Terminal result of one strict support operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::software) enum SoftwareSupportTerminalResult {
    Succeeded,
    Failed,
    Unknown,
    Refused,
}

/// One strict support lifecycle record. It stores operation, plan, and preview
/// identity only: no path, registry bytes, or command line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::software) enum SoftwareSupportLifecycleRecordV1 {
    Started {
        schema_version: u32,
        domain: String,
        operation: SoftwareSupportOperation,
        operation_id: String,
        timestamp_unix_ms: u64,
        plan_identity: String,
        preview_digest: String,
    },
    Terminal {
        schema_version: u32,
        domain: String,
        operation: SoftwareSupportOperation,
        operation_id: String,
        timestamp_unix_ms: u64,
        plan_identity: String,
        preview_digest: String,
        result: SoftwareSupportTerminalResult,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_code: Option<String>,
    },
}

impl SoftwareSupportLifecycleRecordV1 {
    pub(in crate::software) fn started(
        operation: SoftwareSupportOperation,
        operation_id: &str,
        timestamp_unix_ms: u64,
        plan_identity: &str,
        preview_digest: &str,
    ) -> Self {
        Self::Started {
            schema_version: SOFTWARE_SUPPORT_LIFECYCLE_VERSION,
            domain: SOFTWARE_SUPPORT_DOMAIN.to_string(),
            operation,
            operation_id: operation_id.to_string(),
            timestamp_unix_ms,
            plan_identity: plan_identity.to_string(),
            preview_digest: preview_digest.to_string(),
        }
    }

    pub(in crate::software) fn terminal(
        operation: SoftwareSupportOperation,
        operation_id: &str,
        timestamp_unix_ms: u64,
        plan_identity: &str,
        preview_digest: &str,
        result: SoftwareSupportTerminalResult,
        error_code: Option<&str>,
    ) -> Self {
        Self::Terminal {
            schema_version: SOFTWARE_SUPPORT_LIFECYCLE_VERSION,
            domain: SOFTWARE_SUPPORT_DOMAIN.to_string(),
            operation,
            operation_id: operation_id.to_string(),
            timestamp_unix_ms,
            plan_identity: plan_identity.to_string(),
            preview_digest: preview_digest.to_string(),
            result,
            error_code: error_code.map(str::to_string),
        }
    }

    fn operation_id(&self) -> &str {
        match self {
            Self::Started { operation_id, .. } | Self::Terminal { operation_id, .. } => {
                operation_id
            }
        }
    }
}

/// Read-only view of a Software V1 journal. It does not create the directory,
/// the lock, or the append file.
pub(in crate::software) struct SoftwareAuditInspection {
    states: BTreeMap<String, OperationState>,
}

impl SoftwareAuditInspection {
    pub(in crate::software) fn read(path: &Path) -> Result<Self, SoftwareAuditError> {
        if path.as_os_str().is_empty() {
            return Err(SoftwareAuditError::InvalidPath(path.to_path_buf()));
        }
        let exists = path
            .try_exists()
            .map_err(|error| io_error("probe", path, error))?;
        if !exists {
            return Ok(Self {
                states: BTreeMap::new(),
            });
        }
        let bytes = fs::read(path).map_err(|error| io_error("read", path, error))?;
        let (_, states, _) = parse_journal(path, &bytes)?;
        Ok(Self { states })
    }

    #[allow(dead_code)]
    pub(in crate::software) fn uninstall_succeeded(
        &self,
        operation_id: &str,
        identity: &SoftwareIdentity,
    ) -> bool {
        uninstall_succeeded(&self.states, operation_id, identity)
    }

    pub(in crate::software) fn uninstall_removed(
        &self,
        operation_id: &str,
        identity: &SoftwareIdentity,
    ) -> bool {
        uninstall_removed(&self.states, operation_id, identity)
    }
}

#[derive(Debug, Clone)]
struct SupportState {
    operation: SoftwareSupportOperation,
    plan_identity: String,
    preview_digest: String,
    finished: bool,
}

/// Test-only injected append failures. Production passes [`SupportAppendFault::NONE`].
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::software) struct SupportAppendFault {
    pub fail_started: bool,
    pub fail_terminal: bool,
}

impl SupportAppendFault {
    pub(in crate::software) const NONE: Self = Self {
        fail_started: false,
        fail_terminal: false,
    };
}

/// Holds `software.lock` and appends strict lifecycle records to
/// `software-support.jsonl`.
pub(in crate::software) struct SoftwareSupportJournal {
    v1: SoftwareAuditJournal,
    support_path: PathBuf,
    support_file: File,
    states: BTreeMap<String, SupportState>,
}

impl SoftwareSupportJournal {
    pub(in crate::software) fn open(v1_path: &Path) -> Result<Self, SoftwareAuditError> {
        let v1 = SoftwareAuditJournal::open(v1_path)?;
        let support_path = v1_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.join("software-support.jsonl"))
            .ok_or_else(|| SoftwareAuditError::InvalidPath(v1_path.to_path_buf()))?;
        let bytes = if support_path
            .try_exists()
            .map_err(|error| io_error("probe", &support_path, error))?
        {
            fs::read(&support_path).map_err(|error| io_error("read", &support_path, error))?
        } else {
            Vec::new()
        };
        let states = parse_support_journal(&support_path, &bytes)?;
        let support_file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&support_path)
            .map_err(|error| io_error("open", &support_path, error))?;
        Ok(Self {
            v1,
            support_path,
            support_file,
            states,
        })
    }

    #[allow(dead_code)]
    pub(in crate::software) fn uninstall_succeeded(
        &self,
        operation_id: &str,
        identity: &SoftwareIdentity,
    ) -> bool {
        self.v1.uninstall_succeeded(operation_id, identity)
    }

    pub(in crate::software) fn uninstall_removed(
        &self,
        operation_id: &str,
        identity: &SoftwareIdentity,
    ) -> bool {
        self.v1.uninstall_removed(operation_id, identity)
    }

    pub(in crate::software) fn append_lifecycle(
        &mut self,
        record: SoftwareSupportLifecycleRecordV1,
        fault: SupportAppendFault,
    ) -> Result<(), SoftwareAuditError> {
        validate_support_lifecycle(&self.states, &record)?;
        let fail = match &record {
            SoftwareSupportLifecycleRecordV1::Started { .. } => fault.fail_started,
            SoftwareSupportLifecycleRecordV1::Terminal { .. } => fault.fail_terminal,
        };
        if fail {
            return Err(io_error(
                "append",
                &self.support_path,
                io::Error::other("injected support append failure"),
            ));
        }
        let mut next = self.states.clone();
        apply_support_lifecycle(&mut next, &record)?;
        let line = serde_json::to_vec(&record)
            .map_err(|_| SoftwareAuditError::Corrupt(self.support_path.clone()))?;
        self.write_support_line(line)?;
        self.states = next;
        Ok(())
    }

    fn write_support_line(&mut self, mut line: Vec<u8>) -> Result<(), SoftwareAuditError> {
        line.push(b'\n');
        self.support_file
            .write_all(&line)
            .map_err(|error| io_error("append", &self.support_path, error))?;
        self.support_file
            .flush()
            .map_err(|error| io_error("flush", &self.support_path, error))?;
        self.support_file
            .sync_all()
            .map_err(|error| io_error("sync", &self.support_path, error))?;
        Ok(())
    }
}

fn parse_support_journal(
    path: &Path,
    bytes: &[u8],
) -> Result<BTreeMap<String, SupportState>, SoftwareAuditError> {
    if bytes.is_empty() {
        return Ok(BTreeMap::new());
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| SoftwareAuditError::Corrupt(path.to_path_buf()))?;
    if !text.ends_with('\n') {
        return Err(SoftwareAuditError::Corrupt(path.to_path_buf()));
    }
    let mut states = BTreeMap::new();
    for line in text.lines() {
        let record: SoftwareSupportLifecycleRecordV1 = serde_json::from_str(line)
            .map_err(|_| SoftwareAuditError::Corrupt(path.to_path_buf()))?;
        apply_support_lifecycle(&mut states, &record)?;
    }
    Ok(states)
}

fn apply_support_lifecycle(
    states: &mut BTreeMap<String, SupportState>,
    record: &SoftwareSupportLifecycleRecordV1,
) -> Result<(), SoftwareAuditError> {
    validate_support_lifecycle(states, record)?;
    match record {
        SoftwareSupportLifecycleRecordV1::Started {
            operation,
            operation_id,
            plan_identity,
            preview_digest,
            ..
        } => {
            states.insert(
                operation_id.clone(),
                SupportState {
                    operation: *operation,
                    plan_identity: plan_identity.clone(),
                    preview_digest: preview_digest.clone(),
                    finished: false,
                },
            );
        }
        SoftwareSupportLifecycleRecordV1::Terminal { operation_id, .. } => {
            let state = states
                .get_mut(operation_id)
                .expect("validate_support_lifecycle accepts only a known start");
            state.finished = true;
        }
    }
    Ok(())
}

fn validate_support_lifecycle(
    states: &BTreeMap<String, SupportState>,
    record: &SoftwareSupportLifecycleRecordV1,
) -> Result<(), SoftwareAuditError> {
    let invalid = |detail| SoftwareAuditError::InvalidTransition {
        operation_id: record.operation_id().to_string(),
        detail,
    };
    let (schema_version, domain, operation_id, plan_identity, preview_digest) = match record {
        SoftwareSupportLifecycleRecordV1::Started {
            schema_version,
            domain,
            operation_id,
            plan_identity,
            preview_digest,
            ..
        }
        | SoftwareSupportLifecycleRecordV1::Terminal {
            schema_version,
            domain,
            operation_id,
            plan_identity,
            preview_digest,
            ..
        } => (
            *schema_version,
            domain.as_str(),
            operation_id.as_str(),
            plan_identity.as_str(),
            preview_digest.as_str(),
        ),
    };
    if schema_version != SOFTWARE_SUPPORT_LIFECYCLE_VERSION
        || domain != SOFTWARE_SUPPORT_DOMAIN
        || !valid_support_id(operation_id)
        || !is_sha256_digest(plan_identity)
        || !is_sha256_digest(preview_digest)
    {
        return Err(invalid("support lifecycle shape is not V1"));
    }
    match record {
        SoftwareSupportLifecycleRecordV1::Started { .. } => {
            if states.contains_key(operation_id) {
                return Err(invalid("support operation already started"));
            }
        }
        SoftwareSupportLifecycleRecordV1::Terminal {
            operation: terminal_operation,
            plan_identity,
            preview_digest,
            error_code,
            ..
        } => {
            let Some(state) = states.get(operation_id) else {
                return Err(invalid("support terminal has no start"));
            };
            if state.finished
                || state.operation != *terminal_operation
                || state.plan_identity != *plan_identity
                || state.preview_digest != *preview_digest
            {
                return Err(invalid("support terminal does not match its start"));
            }
            if let Some(code) = error_code.as_deref()
                && !valid_support_code(code)
            {
                return Err(invalid("support error code is not a stable token"));
            }
        }
    }
    Ok(())
}

fn valid_support_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

fn valid_support_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

pub(in crate::software) fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(in crate::software) fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(all(test, unix))]
mod lock_tests {
    use super::{SidecarLock, SoftwareAuditError};

    #[test]
    fn owner_drop_unlocks_while_an_inherited_description_remains_open() {
        let directory = tempfile::TempDir::new().expect("isolated lock directory");
        let path = directory.path().join("software.lock");
        let owner = SidecarLock::acquire(&path).expect("first lock owner");
        // dup shares the open file description, just like fork inheritance,
        // without a timing-dependent child or unsafe post-fork test code.
        let inherited = owner._file.try_clone().expect("retained description");
        assert!(matches!(
            SidecarLock::acquire(&path),
            Err(SoftwareAuditError::LockUnavailable(_))
        ));

        drop(owner);
        let replacement = SidecarLock::acquire(&path)
            .expect("owner release unlocks even while the duplicate is alive");
        drop(inherited);
        assert!(matches!(
            SidecarLock::acquire(&path),
            Err(SoftwareAuditError::LockUnavailable(_))
        ));
        drop(replacement);
        SidecarLock::acquire(&path).expect("replacement owner releases its lock");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn identity() -> SoftwareIdentity {
        SoftwareIdentity::Msix {
            package_full_name: "Fixture_1.0.0.0_x64__publisher".to_string(),
        }
    }

    fn record(operation_id: &str, transition: SoftwareAuditTransition) -> SoftwareAuditRecordV1 {
        let (status, reboot, installed, adapter) = match transition {
            SoftwareAuditTransition::Validated => (
                SoftwareAuditStatusCode::Validated,
                SoftwareRebootEvidence::None,
                None,
                None,
            ),
            SoftwareAuditTransition::DispatchStarted => (
                SoftwareAuditStatusCode::DispatchStarted,
                SoftwareRebootEvidence::None,
                None,
                None,
            ),
            SoftwareAuditTransition::AdapterCompleted => (
                SoftwareAuditStatusCode::AdapterSucceeded,
                SoftwareRebootEvidence::None,
                None,
                Some(AdapterOutcome::Success),
            ),
            SoftwareAuditTransition::RequeryObserved => (
                SoftwareAuditStatusCode::RequeryPresent,
                SoftwareRebootEvidence::None,
                Some(SoftwareInstalledState::Present),
                None,
            ),
            SoftwareAuditTransition::Terminal { outcome } => match outcome {
                SoftwareExecutionOutcome::CanceledBeforeStart => (
                    SoftwareAuditStatusCode::CanceledBeforeStart,
                    SoftwareRebootEvidence::None,
                    None,
                    None,
                ),
                SoftwareExecutionOutcome::Removed => (
                    SoftwareAuditStatusCode::Removed,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Absent),
                    Some(AdapterOutcome::Success),
                ),
                SoftwareExecutionOutcome::RebootRequired => (
                    SoftwareAuditStatusCode::RebootRequired,
                    SoftwareRebootEvidence::Required,
                    Some(SoftwareInstalledState::Absent),
                    Some(AdapterOutcome::RebootRequired),
                ),
                SoftwareExecutionOutcome::StillPresent => (
                    SoftwareAuditStatusCode::StillPresent,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    Some(AdapterOutcome::Success),
                ),
                SoftwareExecutionOutcome::Failed => (
                    SoftwareAuditStatusCode::Failed,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    Some(AdapterOutcome::Failure),
                ),
                SoftwareExecutionOutcome::UnknownAfterDispatch => (
                    SoftwareAuditStatusCode::UnknownAfterDispatch,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    Some(AdapterOutcome::Unfinished),
                ),
            },
        };
        SoftwareAuditRecordV1::new(
            operation_id,
            1,
            &identity(),
            &format!("sha256:{}", "1".repeat(64)),
            &format!("sha256:{}", "2".repeat(64)),
            SoftwareAuditEvent {
                transition,
                status_code: status,
                error_code: None,
                reboot_evidence: reboot,
                installed_state: installed,
                adapter_outcome: adapter,
            },
        )
    }

    #[test]
    fn fixed_path_requires_local_app_data() {
        assert!(matches!(
            software_audit_v1_path_from(None),
            Err(SoftwareAuditError::LocalAppDataUnavailable)
        ));
        assert_eq!(
            software_audit_v1_path_from(Some(OsString::from(r"C:\Users\dev\AppData\Local")))
                .unwrap(),
            PathBuf::from(r"C:\Users\dev\AppData\Local")
                .join("DevSweep")
                .join("audit")
                .join("v1")
                .join("software.jsonl")
        );
    }

    #[test]
    fn transition_machine_is_monotonic_and_terminal_is_closed() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("software.jsonl");
        let mut journal = SoftwareAuditJournal::open(&path).unwrap();
        journal
            .append(record("op", SoftwareAuditTransition::Validated))
            .unwrap();
        let error = journal
            .append(record("op", SoftwareAuditTransition::AdapterCompleted))
            .expect_err("adapter cannot precede dispatch");
        assert!(matches!(
            error,
            SoftwareAuditError::InvalidTransition { .. }
        ));
        journal
            .append(record("op", SoftwareAuditTransition::DispatchStarted))
            .unwrap();
        assert!(matches!(
            journal.append(record(
                "op",
                SoftwareAuditTransition::Terminal {
                    outcome: SoftwareExecutionOutcome::CanceledBeforeStart,
                },
            )),
            Err(SoftwareAuditError::InvalidTransition { .. })
        ));
        assert!(matches!(
            journal.append(record(
                "op",
                SoftwareAuditTransition::Terminal {
                    outcome: SoftwareExecutionOutcome::UnknownAfterDispatch,
                },
            )),
            Err(SoftwareAuditError::InvalidTransition { .. })
        ));
        journal
            .append(record("op", SoftwareAuditTransition::RequeryObserved))
            .unwrap();
        journal
            .append(record(
                "op",
                SoftwareAuditTransition::Terminal {
                    outcome: SoftwareExecutionOutcome::UnknownAfterDispatch,
                },
            ))
            .unwrap();
        assert!(matches!(
            journal.append(record(
                "op",
                SoftwareAuditTransition::Terminal {
                    outcome: SoftwareExecutionOutcome::Removed,
                },
            )),
            Err(SoftwareAuditError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn terminal_must_match_accumulated_adapter_and_requery_evidence() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("software.jsonl");
        let mut journal = SoftwareAuditJournal::open(&path).unwrap();
        journal
            .append(record("op", SoftwareAuditTransition::Validated))
            .unwrap();
        journal
            .append(record("op", SoftwareAuditTransition::DispatchStarted))
            .unwrap();
        journal
            .append(record("op", SoftwareAuditTransition::AdapterCompleted))
            .unwrap();
        journal
            .append(record("op", SoftwareAuditTransition::RequeryObserved))
            .unwrap();
        assert!(matches!(
            journal.append(record(
                "op",
                SoftwareAuditTransition::Terminal {
                    outcome: SoftwareExecutionOutcome::Removed,
                },
            )),
            Err(SoftwareAuditError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn journal_is_durable_redacted_and_rejects_unknown_version() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("software.jsonl");
        {
            let mut journal = SoftwareAuditJournal::open(&path).unwrap();
            journal
                .append(record("op", SoftwareAuditTransition::Validated))
                .unwrap();
        }
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.ends_with('\n'));
        assert!(text.contains("package_full_name"));
        for forbidden in [
            "UninstallString",
            "QuietUninstallString",
            "cmd.exe",
            "powershell",
            "argv",
            "program",
            "environment",
            "rollback",
            "reinstall",
        ] {
            assert!(!text.contains(forbidden), "audit leaked {forbidden}");
        }

        let unknown = text.replacen("\"schema_version\":1", "\"schema_version\":2", 1);
        fs::write(&path, unknown).unwrap();
        assert!(matches!(
            SoftwareAuditJournal::open(&path),
            Err(SoftwareAuditError::UnknownVersion(_))
        ));
    }

    #[test]
    fn exclusive_lock_conflict_does_not_modify_journal() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("software.jsonl");
        let journal = SoftwareAuditJournal::open(&path).unwrap();
        let original = fs::read(&path).unwrap_or_default();
        assert!(matches!(
            SoftwareAuditJournal::open(&path),
            Err(SoftwareAuditError::LockUnavailable(_))
        ));
        assert_eq!(fs::read(&path).unwrap_or_default(), original);
        drop(journal);
    }
}
