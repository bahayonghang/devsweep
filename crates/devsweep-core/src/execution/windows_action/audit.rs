//! Append-only journal for one isolated Windows action fixture.
//!
//! A `dispatch_started` line is the dispatch boundary. Cancellation and
//! authorization failure write a terminal line only. Unknown outcomes leave
//! the start line without a terminal line.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{ActionCapability, FixtureOperation};

pub(crate) const JOURNAL_DOMAIN: &str = "windows_action";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditPhase {
    DispatchStarted,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuditRecord {
    pub schema_version: u32,
    pub domain: String,
    pub phase: AuditPhase,
    pub operation_id: String,
    pub operation: String,
    pub preview_digest: String,
    pub target_identity: String,
    pub tool_identity_hash: String,
    pub caller_user_sid: String,
    pub capability: ActionCapability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub(crate) fn started_record(
    operation_id: &str,
    operation: FixtureOperation,
    preview_digest: &str,
    target_identity: &str,
    tool_identity_hash: &str,
    caller_user_sid: &str,
    capability: ActionCapability,
) -> AuditRecord {
    AuditRecord {
        schema_version: super::SCHEMA_VERSION,
        domain: JOURNAL_DOMAIN.to_string(),
        phase: AuditPhase::DispatchStarted,
        operation_id: operation_id.to_string(),
        operation: operation.as_str().to_string(),
        preview_digest: preview_digest.to_string(),
        target_identity: target_identity.to_string(),
        tool_identity_hash: tool_identity_hash.to_string(),
        caller_user_sid: caller_user_sid.to_string(),
        capability,
        outcome: None,
        reason: None,
    }
}

pub(crate) struct TerminalParts<'a> {
    pub operation_id: &'a str,
    pub operation: &'a str,
    pub preview_digest: &'a str,
    pub target_identity: &'a str,
    pub tool_identity_hash: &'a str,
    pub caller_user_sid: &'a str,
    pub capability: ActionCapability,
    pub outcome: &'a str,
    pub reason: Option<&'a str>,
}

pub(crate) fn terminal_record(parts: TerminalParts<'_>) -> AuditRecord {
    AuditRecord {
        schema_version: super::SCHEMA_VERSION,
        domain: JOURNAL_DOMAIN.to_string(),
        phase: AuditPhase::Terminal,
        operation_id: parts.operation_id.to_string(),
        operation: parts.operation.to_string(),
        preview_digest: parts.preview_digest.to_string(),
        target_identity: parts.target_identity.to_string(),
        tool_identity_hash: parts.tool_identity_hash.to_string(),
        caller_user_sid: parts.caller_user_sid.to_string(),
        capability: parts.capability,
        outcome: Some(parts.outcome.to_string()),
        reason: parts.reason.map(str::to_string),
    }
}

pub(crate) fn append(root: &Path, record: &AuditRecord) -> io::Result<()> {
    let dir = journal_dir(root);
    fs::create_dir_all(&dir)?;
    let _lock = Lock::acquire(&dir.join("actions.lock"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("actions.jsonl"))?;
    let mut line = serde_json::to_vec(record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    line.push(b'\n');
    file.write_all(&line)?;
    file.sync_all()
}

pub(crate) fn read(root: &Path) -> io::Result<Vec<AuditRecord>> {
    let path = journal_dir(root).join("actions.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path)?;
    let mut records = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let record: AuditRecord = serde_json::from_str(line)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if record.schema_version != super::SCHEMA_VERSION || record.domain != JOURNAL_DOMAIN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "windows action journal has an unsupported record",
            ));
        }
        records.push(record);
    }
    Ok(records)
}

pub(crate) fn operation_is_spent(records: &[AuditRecord], operation_id: &str) -> bool {
    records
        .iter()
        .any(|record| record.operation_id == operation_id)
}

pub(crate) fn terminal_for<'a>(
    records: &'a [AuditRecord],
    operation_id: &str,
) -> Option<&'a AuditRecord> {
    records
        .iter()
        .rev()
        .find(|record| record.operation_id == operation_id && record.phase == AuditPhase::Terminal)
}

fn journal_dir(root: &Path) -> PathBuf {
    root.join("audit").join("v1")
}

struct Lock {
    _file: File,
}

impl Lock {
    fn acquire(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(path)?;
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
                .open(path)?;
            let locked = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if locked != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self { _file: file })
        }
        #[cfg(not(any(windows, unix)))]
        {
            let file = OpenOptions::new().write(true).create_new(true).open(path)?;
            Ok(Self { _file: file })
        }
    }
}

#[cfg(unix)]
impl Drop for Lock {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        unsafe { libc::flock(self._file.as_raw_fd(), libc::LOCK_UN) };
    }
}
