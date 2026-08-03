use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(test)]
use std::collections::HashSet;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    model::{CleanAction, CleanTarget},
    process::sanitize_process_output,
};

use super::{
    EXECUTOR_DIAGNOSTIC_CAP,
    command::{CommandRequest, command_argv},
    safety::{AuthorizedAction, UserProtectionList},
};

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
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create audit log directory {}", parent.display())
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)
            .with_context(|| format!("failed to open audit log {}", path.display()))?;
        // Separate handle for sync_data after buffered writes.
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

pub(super) struct AuditJournal {
    path: PathBuf,
    run_id: String,
    sequence: u64,
    io: Box<dyn JournalIo>,
}

impl AuditJournal {
    pub(super) fn open(path: &Path, _plan_digest: String) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            run_id: format!("run-{}", unix_epoch_ms()),
            sequence: 0,
            io: Box::new(FileJournalIo::open(path)?),
        })
    }

    #[cfg(test)]
    pub(super) fn with_io(path: PathBuf, io: Box<dyn JournalIo>) -> Self {
        Self {
            path,
            run_id: "run-test".to_string(),
            sequence: 0,
            io,
        }
    }

    pub(super) fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(super) fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    pub(super) fn write_started_durable(&mut self, event: &JournalEvent) -> Result<()> {
        self.write_event(event)?;
        self.io.flush()?;
        self.io.sync_data()?;
        Ok(())
    }

    pub(super) fn write_terminal(&mut self, event: JournalEvent) -> Result<()> {
        self.write_event(&event)?;
        self.io.flush()?;
        // Terminal sync is best-effort; write+flush success is enough to avoid
        // Unknown, but callers may still observe degraded sync separately.
        let _ = self.io.sync_data();
        Ok(())
    }

    fn write_event(&mut self, event: &JournalEvent) -> Result<()> {
        let line = serde_json::to_string(event).with_context(|| {
            format!(
                "failed to serialize audit event for {}",
                self.path.display()
            )
        })?;
        self.io.write_line(&line)
    }

    pub(super) fn flush(&mut self) -> Result<()> {
        self.io.flush()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(super) enum JournalEvent {
    ActionStarted {
        timestamp_epoch_ms: u128,
        run_id: String,
        sequence: u64,
        plan_digest: String,
        target_id: String,
        action: String,
        command: Option<Vec<String>>,
        action_path: Option<String>,
        estimated_bytes: u64,
    },
    ActionFinished {
        timestamp_epoch_ms: u128,
        run_id: String,
        sequence: u64,
        plan_digest: String,
        target_id: String,
        action: String,
        status: String,
        command: Option<Vec<String>>,
        action_path: Option<String>,
        exit_code: Option<i32>,
        estimated_bytes: u64,
        duration_ms: u128,
        error: Option<String>,
    },
}

impl JournalEvent {
    pub(super) fn started(
        run_id: &str,
        sequence: u64,
        plan_digest: &str,
        target: &CleanTarget,
        authorized: &AuthorizedAction,
    ) -> Self {
        Self::ActionStarted {
            timestamp_epoch_ms: unix_epoch_ms(),
            run_id: run_id.to_string(),
            sequence,
            plan_digest: plan_digest.to_string(),
            target_id: target.id.as_str().to_string(),
            action: action_name(&target.action).to_string(),
            command: command_from_action(&target.action),
            action_path: action_path_from_authorized(authorized)
                .map(|path| path.display().to_string()),
            estimated_bytes: target.estimated_bytes,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn finished(
        run_id: &str,
        sequence: u64,
        plan_digest: &str,
        target: &CleanTarget,
        status: &str,
        command: Option<Vec<String>>,
        exit_code: Option<i32>,
        action_path: Option<PathBuf>,
        duration_ms: u128,
        error: Option<String>,
    ) -> Self {
        Self::ActionFinished {
            timestamp_epoch_ms: unix_epoch_ms(),
            run_id: run_id.to_string(),
            sequence,
            plan_digest: plan_digest.to_string(),
            target_id: target.id.as_str().to_string(),
            action: action_name(&target.action).to_string(),
            status: status.to_string(),
            command,
            action_path: action_path.map(|path| path.display().to_string()),
            exit_code,
            estimated_bytes: target.estimated_bytes,
            duration_ms,
            error: error.map(|message| {
                sanitize_process_output(message.as_bytes(), EXECUTOR_DIAGNOSTIC_CAP)
            }),
        }
    }

    pub(super) fn skipped(
        run_id: &str,
        sequence: u64,
        plan_digest: &str,
        target: &CleanTarget,
        message: String,
        duration_ms: u128,
    ) -> Self {
        Self::finished(
            run_id,
            sequence,
            plan_digest,
            target,
            "skipped",
            None,
            None,
            target.path.clone(),
            duration_ms,
            Some(message),
        )
    }
}

/// Replay API: find started events that never received a terminal record.
#[cfg(test)]
pub(super) fn replay_unconfirmed_starts(path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read audit log {}", path.display()))?;
    let mut started = HashSet::new();
    let mut finished = HashSet::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let event = value.get("event").and_then(|v| v.as_str()).unwrap_or("");
        let key = format!(
            "{}:{}",
            value.get("run_id").and_then(|v| v.as_str()).unwrap_or(""),
            value.get("sequence").and_then(|v| v.as_u64()).unwrap_or(0)
        );
        match event {
            "action_started" => {
                started.insert(key);
            }
            "action_finished" => {
                // finished sequences are distinct; pair by prior started target+run
                if let Some(target) = value.get("target_id").and_then(|v| v.as_str()) {
                    finished.insert(format!(
                        "{}:{}",
                        value.get("run_id").and_then(|v| v.as_str()).unwrap_or(""),
                        target
                    ));
                }
            }
            _ => {}
        }
    }

    let mut unconfirmed = Vec::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("event").and_then(|v| v.as_str()) != Some("action_started") {
            continue;
        }
        let run_id = value.get("run_id").and_then(|v| v.as_str()).unwrap_or("");
        let target = value
            .get("target_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let key = format!("{run_id}:{target}");
        if !finished.contains(&key) {
            unconfirmed.push(format!("started_unconfirmed:{target}"));
        }
    }
    let _ = started;
    Ok(unconfirmed)
}

pub(super) fn default_audit_log_path() -> Result<PathBuf> {
    let dir = UserProtectionList::config_path()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create audit directory {}", dir.display()))?;
    Ok(dir.join("audit.jsonl"))
}

pub(super) fn action_path_from_authorized(authorized: &AuthorizedAction) -> Option<PathBuf> {
    authorized
        .trash_path()
        .map(|path| path.to_path_buf())
        .or_else(|| match authorized.action() {
            CleanAction::Command { program, cwd, .. } => {
                cwd.clone().or_else(|| Some(PathBuf::from(program)))
            }
            CleanAction::MoveToTrash { path } => Some(path.clone()),
            _ => None,
        })
}

pub(super) fn command_from_action(action: &CleanAction) -> Option<Vec<String>> {
    match action {
        CleanAction::Command {
            program,
            args,
            cwd: _,
            irreversible: _,
        } => {
            let request = CommandRequest {
                program: program.clone(),
                args: args.clone(),
                cwd: None,
            };
            Some(command_argv(&request))
        }
        CleanAction::MoveToTrash { .. }
        | CleanAction::DeletePermanently { .. }
        | CleanAction::NoopInspectOnly => None,
    }
}

fn action_name(action: &CleanAction) -> &'static str {
    match action {
        CleanAction::Command { .. } => "command",
        CleanAction::MoveToTrash { .. } => "move_to_trash",
        CleanAction::DeletePermanently { .. } => "delete_permanently",
        CleanAction::NoopInspectOnly => "noop_inspect_only",
    }
}

fn unix_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
