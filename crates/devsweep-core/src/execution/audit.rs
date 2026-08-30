use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{CleanAction, CleanTarget};

use super::CapacityEstimate;

const SCHEMA_VERSION: u8 = 1;
const DOMAIN: &str = "clean";
const LOCK_RETRIES: usize = 500;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);

const FORBIDDEN_KEYS: &[&str] = &[
    "argv",
    "args",
    "program",
    "cwd",
    "env",
    "environment",
    "action_path",
    "plan",
    "message",
    "command",
    "path",
];

/// Fault-injectable durable journal backend.
pub(super) trait JournalIo {
    fn write_line(&mut self, line: &str) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
    fn sync_data(&mut self) -> Result<()>;
}

struct FileJournalIo {
    path: PathBuf,
    file: File,
    writer: BufWriter<File>,
}

impl FileJournalIo {
    fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create audit log directory {}", parent.display())
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)
            .with_context(|| format!("failed to open audit log {}", path.display()))?;
        let sync_file = OpenOptions::new()
            .append(true)
            .open(path)
            .with_context(|| format!("failed to reopen audit log {}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            file: sync_file,
            writer: BufWriter::new(file),
        })
    }
}

impl JournalIo for FileJournalIo {
    fn write_line(&mut self, line: &str) -> Result<()> {
        self.writer
            .write_all(line.as_bytes())
            .and_then(|_| self.writer.write_all(b"\n"))
            .with_context(|| format!("failed to write audit log {}", self.path.display()))
    }

    fn flush(&mut self) -> Result<()> {
        self.writer
            .flush()
            .with_context(|| format!("failed to flush audit log {}", self.path.display()))
    }

    fn sync_data(&mut self) -> Result<()> {
        self.file
            .sync_data()
            .with_context(|| format!("failed to sync audit log {}", self.path.display()))
    }
}

/// Closed Clean V1 audit envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "record_kind", rename_all = "snake_case")]
pub enum CleanAuditRecordV1 {
    /// One execution-state transition for a redacted cleanup target.
    ExecutionTransition {
        schema_version: u8,
        domain: String,
        operation_id: String,
        timestamp_epoch_ms: u64,
        transition: u64,
        outcome_code: ExecutionOutcomeCode,
        error_code: Option<ExecutionErrorCode>,
        evidence: ExecutionEvidence,
    },
    /// One protection-list mutation transition. The Protection task emits this
    /// variant through [`append_clean_audit_record`]; it does not own the journal.
    ProtectionMutation {
        schema_version: u8,
        domain: String,
        operation_id: String,
        timestamp_epoch_ms: u64,
        transition: u64,
        outcome_code: ProtectionOutcomeCode,
        error_code: Option<ProtectionErrorCode>,
        evidence: ProtectionEvidence,
    },
}

/// Stable execution transition codes. These are never localized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcomeCode {
    Started,
    Succeeded,
    Failed,
    Skipped,
    Unknown,
    Canceled,
}

/// Stable execution error codes. Absence means no typed failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionErrorCode {
    AuthorizationDenied,
    SafetySkip,
    SelfCleanSkip,
    CommandFailed,
    TrashFailed,
    AuditBlocked,
    AuditTerminalFailed,
    DuplicateFingerprint,
    Canceled,
}

/// Stable protection mutation outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionOutcomeCode {
    Requested,
    Committed,
    Failed,
}

/// Stable protection mutation error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionErrorCode {
    StoreUnavailable,
    AuditBlocked,
    Unknown,
}

/// Redacted execution evidence. It never carries argv, paths, or plan payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEvidence {
    pub target_identity_sha256: String,
    pub action_kind: RedactedActionKind,
    pub irreversible: bool,
    pub capacity_class: CapacityClass,
    pub duration_ms: Option<u64>,
}

/// Redacted protection evidence. Identity is a SHA-256, never a raw path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectionEvidence {
    pub action: ProtectionMutationAction,
    pub identity_sha256: String,
}

/// Public action classification without program, argv, or path details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactedActionKind {
    Command,
    MoveToTrash,
}

/// Capacity confidence without byte totals that could reconstruct a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityClass {
    Verified,
    Partial,
    Unknown,
}

/// Protection mutation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionMutationAction {
    Add,
    Remove,
}

/// Fail-closed Clean V1 journal errors.
#[derive(Debug)]
pub enum CleanAuditError {
    LocalAppDataUnavailable,
    LockUnavailable(PathBuf),
    UnknownVersion {
        path: PathBuf,
    },
    Corrupt {
        path: PathBuf,
    },
    Io {
        stage: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    ForbiddenPayload,
}

impl std::fmt::Display for CleanAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalAppDataUnavailable => formatter
                .write_str("LOCALAPPDATA is unavailable; Clean audit persistence is disabled"),
            Self::LockUnavailable(path) => write!(
                formatter,
                "clean audit lock is unavailable at {}",
                path.display()
            ),
            Self::UnknownVersion { path } => write!(
                formatter,
                "clean audit journal at {} has an unknown version; original bytes were preserved",
                path.display()
            ),
            Self::Corrupt { path } => write!(
                formatter,
                "clean audit journal at {} is corrupt; original bytes were preserved",
                path.display()
            ),
            Self::Io {
                stage,
                path,
                source,
            } => write!(
                formatter,
                "clean audit {stage} failed for {}: {source}",
                path.display()
            ),
            Self::ForbiddenPayload => {
                formatter.write_str("clean audit record contained forbidden executable fields")
            }
        }
    }
}

impl std::error::Error for CleanAuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn io_error(stage: &'static str, path: &Path, source: io::Error) -> CleanAuditError {
    CleanAuditError::Io {
        stage,
        path: path.to_path_buf(),
        source,
    }
}

fn io_result<T>(
    stage: &'static str,
    path: &Path,
    result: io::Result<T>,
) -> Result<T, CleanAuditError> {
    result.map_err(|source| io_error(stage, path, source))
}

/// Resolve the sole supported Clean V1 journal path.
pub fn clean_audit_v1_path() -> Result<PathBuf, CleanAuditError> {
    clean_audit_v1_path_from(std::env::var_os("LOCALAPPDATA"))
}

fn clean_audit_v1_path_from(
    local_app_data: Option<std::ffi::OsString>,
) -> Result<PathBuf, CleanAuditError> {
    let root = local_app_data
        .filter(|value| !value.is_empty())
        .ok_or(CleanAuditError::LocalAppDataUnavailable)?;
    Ok(PathBuf::from(root)
        .join("DevSweep")
        .join("audit")
        .join("v1")
        .join("clean.jsonl"))
}

/// Legacy default that must never be opened, imported, or rewritten.
pub fn legacy_audit_log_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .filter(|value| !value.is_empty())
        .map(|root| PathBuf::from(root).join("devsweep").join("audit.jsonl"))
}

/// SHA-256 of a canonical identity string. Never persist the preimage.
pub fn identity_sha256(identity: &str) -> String {
    format!("{:x}", Sha256::digest(identity.as_bytes()))
}

pub(super) fn redacted_action_kind(action: &CleanAction) -> Option<RedactedActionKind> {
    match action {
        CleanAction::Command { .. } => Some(RedactedActionKind::Command),
        CleanAction::MoveToTrash { .. } => Some(RedactedActionKind::MoveToTrash),
        CleanAction::NoopInspectOnly | CleanAction::DeletePermanently { .. } => None,
    }
}

pub(super) fn capacity_class(target: &CleanTarget) -> CapacityClass {
    match CapacityEstimate::from_target(target) {
        CapacityEstimate::Verified { .. } => CapacityClass::Verified,
        CapacityEstimate::Partial { .. } => CapacityClass::Partial,
        CapacityEstimate::Unknown => CapacityClass::Unknown,
    }
}

fn unix_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

impl CleanAuditRecordV1 {
    pub(super) fn execution(
        operation_id: &str,
        transition: u64,
        outcome_code: ExecutionOutcomeCode,
        error_code: Option<ExecutionErrorCode>,
        target: &CleanTarget,
        duration_ms: Option<u64>,
    ) -> Result<Self, CleanAuditError> {
        let action_kind =
            redacted_action_kind(&target.action).ok_or(CleanAuditError::ForbiddenPayload)?;
        Ok(Self::ExecutionTransition {
            schema_version: SCHEMA_VERSION,
            domain: DOMAIN.to_string(),
            operation_id: operation_id.to_string(),
            timestamp_epoch_ms: unix_epoch_ms(),
            transition,
            outcome_code,
            error_code,
            evidence: ExecutionEvidence {
                target_identity_sha256: identity_sha256(target.id.as_str()),
                action_kind,
                irreversible: matches!(
                    target.action,
                    CleanAction::Command {
                        irreversible: true,
                        ..
                    }
                ),
                capacity_class: capacity_class(target),
                duration_ms,
            },
        })
    }

    /// Build a protection-mutation record for the Protection task's writer handoff.
    pub fn protection(
        operation_id: &str,
        transition: u64,
        outcome_code: ProtectionOutcomeCode,
        error_code: Option<ProtectionErrorCode>,
        action: ProtectionMutationAction,
        identity_sha256: String,
    ) -> Self {
        Self::ProtectionMutation {
            schema_version: SCHEMA_VERSION,
            domain: DOMAIN.to_string(),
            operation_id: operation_id.to_string(),
            timestamp_epoch_ms: unix_epoch_ms(),
            transition,
            outcome_code,
            error_code,
            evidence: ProtectionEvidence {
                action,
                identity_sha256,
            },
        }
    }
}

fn serialize_record(record: &CleanAuditRecordV1) -> Result<String, CleanAuditError> {
    let line = serde_json::to_string(record).map_err(|_| CleanAuditError::ForbiddenPayload)?;
    if contains_forbidden_payload(&line) {
        return Err(CleanAuditError::ForbiddenPayload);
    }
    Ok(line)
}

fn contains_forbidden_payload(line: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return true;
    };
    json_contains_forbidden_key(&value)
}

fn json_contains_forbidden_key(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, child)| {
            FORBIDDEN_KEYS.contains(&key.as_str()) || json_contains_forbidden_key(child)
        }),
        serde_json::Value::Array(items) => items.iter().any(json_contains_forbidden_key),
        _ => false,
    }
}

enum ExistingJournal {
    Valid,
    UnknownVersion,
    Corrupt,
}

fn classify_journal_bytes(bytes: &[u8]) -> ExistingJournal {
    if bytes.is_empty() {
        return ExistingJournal::Valid;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return ExistingJournal::Corrupt;
    };
    if !text.ends_with('\n') {
        return ExistingJournal::Corrupt;
    }
    for line in text.split('\n') {
        if line.is_empty() {
            continue;
        }
        match classify_line(line) {
            ExistingJournal::Valid => {}
            other => return other,
        }
    }
    ExistingJournal::Valid
}

fn classify_line(line: &str) -> ExistingJournal {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return ExistingJournal::Corrupt;
    };
    match value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
    {
        Some(1) => {}
        Some(_) => return ExistingJournal::UnknownVersion,
        None => return ExistingJournal::Corrupt,
    }
    match value.get("domain").and_then(serde_json::Value::as_str) {
        Some(DOMAIN) => {}
        Some(_) => return ExistingJournal::UnknownVersion,
        None => return ExistingJournal::Corrupt,
    }
    match value.get("record_kind").and_then(serde_json::Value::as_str) {
        Some("execution_transition" | "protection_mutation") => {}
        Some(_) => return ExistingJournal::UnknownVersion,
        None => return ExistingJournal::Corrupt,
    }
    if serde_json::from_str::<CleanAuditRecordV1>(line).is_err() {
        return ExistingJournal::Corrupt;
    }
    if json_contains_forbidden_key(&value) {
        return ExistingJournal::Corrupt;
    }
    ExistingJournal::Valid
}

struct SidecarLock {
    _file: File,
}

impl SidecarLock {
    fn acquire(path: PathBuf) -> Result<Self, CleanAuditError> {
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;

            for attempt in 0..LOCK_RETRIES {
                match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .share_mode(0)
                    .open(&path)
                {
                    Ok(file) => return Ok(Self { _file: file }),
                    Err(error) if lock_error_is_contention(&error) => {
                        if attempt + 1 == LOCK_RETRIES {
                            return Err(CleanAuditError::LockUnavailable(path));
                        }
                        std::thread::sleep(LOCK_RETRY_DELAY);
                    }
                    Err(error) => return Err(io_error("acquire_lock", &path, error)),
                }
            }
            Err(CleanAuditError::LockUnavailable(path))
        }
        #[cfg(not(windows))]
        {
            use std::os::fd::AsRawFd;

            let file = io_result(
                "open_lock",
                &path,
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path),
            )?;
            for attempt in 0..LOCK_RETRIES {
                let result =
                    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
                if result == 0 {
                    return Ok(Self { _file: file });
                }
                let error = io::Error::last_os_error();
                if !lock_error_is_contention(&error) {
                    return Err(io_error("acquire_lock", &path, error));
                }
                if attempt + 1 == LOCK_RETRIES {
                    return Err(CleanAuditError::LockUnavailable(path));
                }
                std::thread::sleep(LOCK_RETRY_DELAY);
            }
            Err(CleanAuditError::LockUnavailable(path))
        }
    }
}

fn lock_error_is_contention(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::PermissionDenied
        || error.kind() == io::ErrorKind::WouldBlock
        || error.kind() == io::ErrorKind::AlreadyExists
    {
        return true;
    }
    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(5 | 32 | 33 | 80 | 183))
    }
    #[cfg(not(windows))]
    {
        matches!(error.raw_os_error(), Some(libc::EAGAIN | libc::EACCES))
    }
}

/// Append one closed V1 record to the fixed Clean journal.
pub fn append_clean_audit_record(record: &CleanAuditRecordV1) -> Result<(), CleanAuditError> {
    append_to_path(&clean_audit_v1_path()?, record)
}

fn append_to_path(path: &Path, record: &CleanAuditRecordV1) -> Result<(), CleanAuditError> {
    let line = serialize_record(record)?;
    let parent = path.parent().ok_or_else(|| {
        io_error(
            "resolve_journal_parent",
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "journal path has no parent"),
        )
    })?;
    io_result(
        "create_journal_directory",
        parent,
        fs::create_dir_all(parent),
    )?;
    let lock_path = parent.join("clean.lock");
    let _lock = SidecarLock::acquire(lock_path)?;
    let existing = if io_result("probe_journal", path, path.try_exists())? {
        io_result("read_journal", path, fs::read(path))?
    } else {
        Vec::new()
    };
    match classify_journal_bytes(&existing) {
        ExistingJournal::Valid => {}
        ExistingJournal::UnknownVersion => {
            return Err(CleanAuditError::UnknownVersion {
                path: path.to_path_buf(),
            });
        }
        ExistingJournal::Corrupt => {
            return Err(CleanAuditError::Corrupt {
                path: path.to_path_buf(),
            });
        }
    }
    let mut journal_io = FileJournalIo::open(path)
        .map_err(|error| io_error("open_journal", path, io::Error::other(error.to_string())))?;
    journal_io
        .write_line(&line)
        .map_err(|error| io_error("append_journal", path, io::Error::other(error.to_string())))?;
    journal_io
        .flush()
        .map_err(|error| io_error("flush_journal", path, io::Error::other(error.to_string())))?;
    journal_io
        .sync_data()
        .map_err(|error| io_error("sync_journal", path, io::Error::other(error.to_string())))?;
    Ok(())
}

pub(super) struct AuditJournal {
    path: PathBuf,
    operation_id: String,
    transition: u64,
    io: Option<Box<dyn JournalIo>>,
}

impl AuditJournal {
    pub(super) fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            operation_id: format!("op-{}", unix_epoch_ms()),
            transition: 0,
            io: None,
        })
    }

    #[cfg(test)]
    pub(super) fn with_io(path: PathBuf, io: Box<dyn JournalIo>) -> Self {
        Self {
            path,
            operation_id: "op-test".to_string(),
            transition: 0,
            io: Some(io),
        }
    }

    pub(super) fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub(super) fn next_transition(&mut self) -> u64 {
        self.transition += 1;
        self.transition
    }

    pub(super) fn write_started_durable(&mut self, record: &CleanAuditRecordV1) -> Result<()> {
        self.write_record(record)
    }

    pub(super) fn write_terminal(&mut self, record: CleanAuditRecordV1) -> Result<()> {
        self.write_record(&record)
    }

    fn write_record(&mut self, record: &CleanAuditRecordV1) -> Result<()> {
        let line = serialize_record(record).map_err(anyhow::Error::from)?;
        if let Some(io) = self.io.as_mut() {
            io.write_line(&line)?;
            io.flush()?;
            io.sync_data()?;
            return Ok(());
        }
        append_to_path(&self.path, record).map_err(anyhow::Error::from)
    }

    pub(super) fn flush(&mut self) -> Result<()> {
        if let Some(io) = self.io.as_mut() {
            io.flush()?;
        }
        Ok(())
    }
}

pub(super) fn default_audit_log_path() -> Result<PathBuf> {
    clean_audit_v1_path().map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ecosystem, Evidence, RiskLevel, Scope, TargetId, TargetKind};
    use tempfile::TempDir;

    fn fixture_target(action: CleanAction) -> CleanTarget {
        CleanTarget {
            id: TargetId::new("fixture.clean.target.1"),
            scope: Scope::Global,
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::BuildArtifacts,
            path: Some(PathBuf::from(r"C:\secret\project\target")),
            estimated_bytes: 42,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "rust.target".to_string(),
            }],
            action,
        }
    }

    fn write_record(path: &Path, record: &CleanAuditRecordV1) {
        append_to_path(path, record).expect("append succeeds");
    }

    #[test]
    fn fixed_v1_path_never_selects_appdata_or_a_fallback() {
        assert_eq!(
            clean_audit_v1_path_from(Some(std::ffi::OsString::from(
                r"C:\Users\dev\AppData\Local"
            )))
            .unwrap(),
            PathBuf::from(r"C:\Users\dev\AppData\Local")
                .join("DevSweep")
                .join("audit")
                .join("v1")
                .join("clean.jsonl")
        );
        assert!(matches!(
            clean_audit_v1_path_from(None),
            Err(CleanAuditError::LocalAppDataUnavailable)
        ));
        assert!(matches!(
            clean_audit_v1_path_from(Some(std::ffi::OsString::new())),
            Err(CleanAuditError::LocalAppDataUnavailable)
        ));
    }

    #[test]
    fn execution_and_protection_records_are_redacted() {
        let temp = TempDir::new().unwrap();
        let path = temp
            .path()
            .join("DevSweep")
            .join("audit")
            .join("v1")
            .join("clean.jsonl");
        let target = fixture_target(CleanAction::Command {
            program: "cargo".to_string(),
            args: vec![
                "clean".to_string(),
                "--manifest-path".to_string(),
                r"C:\secret\project\Cargo.toml".to_string(),
            ],
            cwd: Some(PathBuf::from(r"C:\secret\project")),
            irreversible: true,
        });
        let started = CleanAuditRecordV1::execution(
            "op-clean-v1-fixture",
            1,
            ExecutionOutcomeCode::Started,
            None,
            &target,
            None,
        )
        .unwrap();
        let finished = CleanAuditRecordV1::execution(
            "op-clean-v1-fixture",
            2,
            ExecutionOutcomeCode::Succeeded,
            None,
            &target,
            Some(12),
        )
        .unwrap();
        let protection = CleanAuditRecordV1::protection(
            "op-protect-v1-fixture",
            1,
            ProtectionOutcomeCode::Committed,
            None,
            ProtectionMutationAction::Add,
            identity_sha256("fixture.protection.identity.1"),
        );
        write_record(&path, &started);
        write_record(&path, &finished);
        write_record(&path, &protection);
        let text = fs::read_to_string(&path).unwrap();
        for forbidden in [
            "cargo",
            "--manifest-path",
            r"C:\secret",
            "C:/secret",
            "argv",
            "program",
            "Cargo.toml",
            "fixture.clean.target.1",
            "fixture.protection.identity.1",
        ] {
            assert!(
                !text.contains(forbidden),
                "journal leaked {forbidden}: {text}"
            );
        }
        assert!(text.contains("execution_transition"));
        assert!(text.contains("protection_mutation"));
        assert!(text.contains(&identity_sha256("fixture.clean.target.1")));
        assert!(text.contains(&identity_sha256("fixture.protection.identity.1")));
        let records: Vec<CleanAuditRecordV1> = text
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.len(), 3);
    }

    #[test]
    fn unknown_version_and_corrupt_bytes_are_preserved() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("clean.jsonl");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let unknown =
            br#"{"schema_version":2,"domain":"clean","record_kind":"execution_transition"}
"#;
        fs::write(&path, unknown).unwrap();
        let record = CleanAuditRecordV1::protection(
            "op",
            1,
            ProtectionOutcomeCode::Requested,
            None,
            ProtectionMutationAction::Remove,
            identity_sha256("x"),
        );
        assert!(matches!(
            append_to_path(&path, &record),
            Err(CleanAuditError::UnknownVersion { .. })
        ));
        assert_eq!(fs::read(&path).unwrap(), unknown);

        let corrupt = b"{\"schema_version\":1\n";
        fs::write(&path, corrupt).unwrap();
        assert!(matches!(
            append_to_path(&path, &record),
            Err(CleanAuditError::Corrupt { .. })
        ));
        assert_eq!(fs::read(&path).unwrap(), corrupt);
    }

    #[test]
    fn lock_failure_leaves_journal_unmodified() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("clean.jsonl");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let lock_path = path.parent().unwrap().join("clean.lock");
        let _held = SidecarLock::acquire(lock_path).expect("test owns the sidecar");
        let record = CleanAuditRecordV1::protection(
            "op",
            1,
            ProtectionOutcomeCode::Requested,
            None,
            ProtectionMutationAction::Add,
            identity_sha256("x"),
        );
        let error = append_to_path(&path, &record).expect_err("contended lock fails closed");
        assert!(matches!(error, CleanAuditError::LockUnavailable(_)));
        assert!(!path.exists());
    }

    #[test]
    fn legacy_appdata_journal_is_never_discovered() {
        let temp = TempDir::new().unwrap();
        let legacy = temp.path().join("devsweep").join("audit.jsonl");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let original =
            br#"{"event":"action_started","command":["cargo","clean"],"action_path":"C:/legacy"}
"#;
        fs::write(&legacy, original).unwrap();
        let v1 = temp
            .path()
            .join("DevSweep")
            .join("audit")
            .join("v1")
            .join("clean.jsonl");
        let record = CleanAuditRecordV1::protection(
            "op",
            1,
            ProtectionOutcomeCode::Committed,
            None,
            ProtectionMutationAction::Add,
            identity_sha256("fixture.protection.identity.1"),
        );
        write_record(&v1, &record);
        assert_eq!(fs::read(&legacy).unwrap(), original);
        assert!(
            fs::read_to_string(&v1)
                .unwrap()
                .contains("protection_mutation")
        );
        assert!(!fs::read_to_string(&v1).unwrap().contains("cargo"));
    }

    #[test]
    fn history_handoff_fixtures_cannot_recover_executable_text() {
        let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("history")
            .join("clean-v1");
        for name in [
            "execution-transition.jsonl",
            "protection-mutation.jsonl",
            "unknown-version.jsonl",
            "corrupt-line.jsonl",
            "legacy-non-discovery.jsonl",
        ] {
            let text = fs::read_to_string(fixture_dir.join(name)).unwrap();
            for forbidden in [
                "argv",
                "cargo clean",
                "--manifest-path",
                "C:\\\\secret",
                "program",
            ] {
                assert!(!text.contains(forbidden), "{name} leaked {forbidden}");
            }
        }
        let execution = fs::read_to_string(fixture_dir.join("execution-transition.jsonl")).unwrap();
        let parsed: CleanAuditRecordV1 =
            serde_json::from_str(execution.lines().next().unwrap()).unwrap();
        match parsed {
            CleanAuditRecordV1::ExecutionTransition { evidence, .. } => {
                assert_eq!(
                    evidence.target_identity_sha256,
                    identity_sha256("fixture.clean.target.1")
                );
            }
            _ => panic!("expected execution transition"),
        }
    }
}
