//! Short-lived fixture helper. It reads one sealed frame from stdin, re-checks
//! the live subject, and exits. It does not accept argv or a shell string.

use std::{
    env, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process, thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use super::{
    BackupManifest, FixtureOperation, SCHEMA_VERSION, SUBJECT_FILE, SUBJECT_HIVE, content_digest,
    current_user_sid, fill_random, hash_tool, hmac_sha256, mac_equal, next_backup_version,
    preview_digest, read_subject,
};
use crate::filesystem::normalize_absolute_path;
use crate::process::{FAULT_ENV, GATE_ENV};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Envelope {
    pub schema_version: u32,
    pub operation_id: String,
    pub operation: String,
    pub preview_digest: String,
    pub caller_user_sid: String,
    pub tool_identity_hash: String,
    pub target_identity: String,
    pub isolated_root: String,
    pub plan_id: String,
}

pub fn run_windows_action_helper() -> i32 {
    match helper_main() {
        Ok(code) => code,
        Err(stop) => stop.finish(),
    }
}

pub(super) fn seal(body: &[u8]) -> io::Result<Vec<u8>> {
    let mut key = [0u8; 32];
    fill_random(&mut key)?;
    let mac = hmac_sha256(&key, body);
    let len = u32::try_from(body.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "helper frame is too large"))?;
    let mut frame = Vec::with_capacity(5 + 32 + 4 + body.len() + 32);
    frame.extend_from_slice(b"DSWA1");
    frame.extend_from_slice(&key);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(body);
    frame.extend_from_slice(&mac);
    Ok(frame)
}

fn helper_main() -> Result<i32, HelperStop> {
    match env::var(FAULT_ENV).ok().as_deref() {
        Some("hang") => loop {
            thread::sleep(Duration::from_secs(60));
        },
        Some("crash") => process::exit(2),
        Some("disconnect") => {
            thread::sleep(Duration::from_millis(300));
            return Ok(0);
        }
        Some("missing_terminal") => {
            let _ = read_frame(&mut io::stdin())?;
            return Ok(0);
        }
        _ => {}
    }

    let body = read_frame(&mut io::stdin())?;
    let envelope: Envelope = serde_json::from_slice(&body).map_err(|_| HelperStop::Protocol)?;
    if envelope.schema_version != SCHEMA_VERSION {
        return Err(HelperStop::Protocol);
    }
    let root = PathBuf::from(&envelope.isolated_root);
    let plan = load_matching_plan(&root, &envelope)?;
    if let Err(stop) = wait_gate() {
        return finish_known(&envelope, stop);
    }
    if let Err(stop) = revalidate(&envelope, &plan, &root) {
        return finish_known(&envelope, stop);
    }
    let performed = match parse_operation(&envelope.operation)? {
        FixtureOperation::Backup => backup(&root),
        FixtureOperation::Restore => restore(&root),
        FixtureOperation::IdentityCheck => identity_check(&root, &envelope),
    };
    if let Err(stop) = performed {
        return finish_known(&envelope, stop);
    }
    write_terminal(&envelope, "succeeded", None)?;
    Ok(0)
}

fn finish_known(envelope: &Envelope, stop: HelperStop) -> Result<i32, HelperStop> {
    match stop {
        HelperStop::Refused(reason) => {
            write_terminal(envelope, "refused", Some(reason))?;
            Ok(0)
        }
        HelperStop::ReadBack => {
            write_terminal(envelope, "read_back_failed", None)?;
            Ok(0)
        }
        HelperStop::Protocol => Err(HelperStop::Protocol),
    }
}

fn revalidate(
    envelope: &Envelope,
    plan: &super::PlanDocument,
    root: &Path,
) -> Result<(), HelperStop> {
    let root_text = normalize_absolute_path(root).map_err(|_| HelperStop::Protocol)?;
    let live_sid = current_user_sid().map_err(|_| HelperStop::Refused("cross_user"))?;
    let live_tool = hash_tool(&env::current_exe().map_err(|_| HelperStop::Protocol)?)
        .map_err(|_| HelperStop::Refused("tool_replacement"))?;
    let live =
        read_subject(root).map_err(|_| HelperStop::Refused(drift_reason(&envelope.operation)))?;
    let live_digest = preview_digest(
        &plan.plan_id,
        &live_sid,
        &live_tool,
        &live.target_identity,
        &root_text,
    );
    if live_sid != envelope.caller_user_sid || live_sid != plan.bound_user_sid {
        return Err(HelperStop::Refused("cross_user"));
    }
    if live_tool != envelope.tool_identity_hash || live_tool != plan.tool_identity_hash {
        return Err(HelperStop::Refused("tool_replacement"));
    }
    if live.target_identity != envelope.target_identity
        || live.target_identity != plan.target_identity
    {
        return Err(HelperStop::Refused(drift_reason(&envelope.operation)));
    }
    if live_digest != envelope.preview_digest || live_digest != plan.preview_digest {
        return Err(HelperStop::Refused("digest_mismatch"));
    }
    Ok(())
}

fn drift_reason(operation: &str) -> &'static str {
    if operation == "fixture.restore" {
        "restore_subject_changed"
    } else {
        "identity_drift"
    }
}

fn backup(root: &Path) -> Result<(), HelperStop> {
    let before = read_subject(root).map_err(|_| HelperStop::Refused("identity_drift"))?;
    let version = next_backup_version(root).map_err(|_| HelperStop::Protocol)?;
    let dir = root.join("backup").join(format!("v{version}"));
    fs::create_dir_all(&dir).map_err(|_| HelperStop::Protocol)?;
    if write_backup(&dir, version, &before).is_err() {
        let _ = fs::remove_dir_all(&dir);
        return Err(HelperStop::Protocol);
    }
    let after = match read_subject(root) {
        Ok(subject) => subject,
        Err(_) => {
            let _ = fs::remove_dir_all(&dir);
            return Err(HelperStop::ReadBack);
        }
    };
    if before.file_identity != after.file_identity
        || before.hive_identity != after.hive_identity
        || before.file_bytes != after.file_bytes
        || before.hive_bytes != after.hive_bytes
        || before.registry_value != after.registry_value
    {
        let _ = fs::remove_dir_all(&dir);
        return Err(HelperStop::ReadBack);
    }
    Ok(())
}

fn write_backup(dir: &Path, version: u32, subject: &super::Subject) -> io::Result<()> {
    write_synced(&dir.join("file.bin"), &subject.file_bytes)?;
    write_synced(&dir.join("hive.bin"), &subject.hive_bytes)?;
    write_synced(&dir.join("registry-value.bin"), &subject.registry_value)?;
    let manifest = BackupManifest {
        schema_version: SCHEMA_VERSION,
        backup_version: version,
        file_identity: subject.file_identity.clone(),
        hive_identity: subject.hive_identity.clone(),
        target_identity: subject.target_identity.clone(),
        file_sha256: content_digest(&subject.file_bytes),
        hive_sha256: content_digest(&subject.hive_bytes),
        registry_value_sha256: content_digest(&subject.registry_value),
    };
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_synced(&dir.join("manifest.json"), &bytes)
}

fn restore(root: &Path) -> Result<(), HelperStop> {
    let dir = super::latest_backup_dir(root)
        .map_err(|_| HelperStop::Protocol)?
        .ok_or(HelperStop::Refused("restore_subject_changed"))?;
    let manifest = read_manifest(&dir)?;
    let live = read_subject(root).map_err(|_| HelperStop::Refused("restore_subject_changed"))?;
    if live.file_identity != manifest.file_identity || live.hive_identity != manifest.hive_identity
    {
        return Err(HelperStop::Refused("restore_subject_changed"));
    }
    let file_bytes = fs::read(dir.join("file.bin")).map_err(|_| HelperStop::Protocol)?;
    let hive_bytes = fs::read(dir.join("hive.bin")).map_err(|_| HelperStop::Protocol)?;
    let value_bytes = fs::read(dir.join("registry-value.bin")).map_err(|_| HelperStop::Protocol)?;
    if content_digest(&file_bytes) != manifest.file_sha256
        || content_digest(&hive_bytes) != manifest.hive_sha256
        || content_digest(&value_bytes) != manifest.registry_value_sha256
    {
        return Err(HelperStop::Protocol);
    }
    super::registry::write_in_place(&root.join(SUBJECT_FILE), &file_bytes)
        .map_err(|_| HelperStop::Protocol)?;
    super::registry::write_in_place(&root.join(SUBJECT_HIVE), &hive_bytes)
        .map_err(|_| HelperStop::Protocol)?;
    let after = read_subject(root).map_err(|_| HelperStop::ReadBack)?;
    if after.file_bytes != file_bytes
        || after.hive_bytes != hive_bytes
        || after.registry_value != value_bytes
        || after.file_identity != manifest.file_identity
        || after.hive_identity != manifest.hive_identity
    {
        return Err(HelperStop::ReadBack);
    }
    Ok(())
}

fn identity_check(root: &Path, envelope: &Envelope) -> Result<(), HelperStop> {
    let live = read_subject(root).map_err(|_| HelperStop::Refused("identity_drift"))?;
    if live.target_identity != envelope.target_identity {
        return Err(HelperStop::Refused("identity_drift"));
    }
    Ok(())
}

fn read_manifest(dir: &Path) -> Result<BackupManifest, HelperStop> {
    let text = fs::read_to_string(dir.join("manifest.json")).map_err(|_| HelperStop::Protocol)?;
    serde_json::from_str(&text).map_err(|_| HelperStop::Protocol)
}

fn load_matching_plan(root: &Path, envelope: &Envelope) -> Result<super::PlanDocument, HelperStop> {
    let bytes = fs::read(root.join("plan.json")).map_err(|_| HelperStop::Protocol)?;
    let plan: super::PlanDocument =
        serde_json::from_slice(&bytes).map_err(|_| HelperStop::Protocol)?;
    let normalized = normalize_absolute_path(root).map_err(|_| HelperStop::Protocol)?;
    if plan.schema_version != SCHEMA_VERSION
        || plan.plan_id != envelope.plan_id
        || plan.isolated_root != envelope.isolated_root
        || plan.isolated_root != normalized
        || plan.bound_user_sid != envelope.caller_user_sid
        || plan.tool_identity_hash != envelope.tool_identity_hash
        || plan.target_identity != envelope.target_identity
        || plan.preview_digest != envelope.preview_digest
    {
        return Err(HelperStop::Protocol);
    }
    Ok(plan)
}

fn parse_operation(operation: &str) -> Result<FixtureOperation, HelperStop> {
    match operation {
        "fixture.backup" => Ok(FixtureOperation::Backup),
        "fixture.restore" => Ok(FixtureOperation::Restore),
        "fixture.identity_check" => Ok(FixtureOperation::IdentityCheck),
        _ => Err(HelperStop::Protocol),
    }
}

fn write_terminal(
    envelope: &Envelope,
    outcome: &str,
    reason: Option<&str>,
) -> Result<(), HelperStop> {
    let capability = match parse_operation(&envelope.operation) {
        Ok(operation) => operation.capability(),
        Err(_) => super::ActionCapability::Unsupported,
    };
    let record = super::audit::terminal_record(super::audit::TerminalParts {
        operation_id: &envelope.operation_id,
        operation: &envelope.operation,
        preview_digest: &envelope.preview_digest,
        target_identity: &envelope.target_identity,
        tool_identity_hash: &envelope.tool_identity_hash,
        caller_user_sid: &envelope.caller_user_sid,
        capability,
        outcome,
        reason,
    });
    super::audit::append(Path::new(&envelope.isolated_root), &record)
        .map_err(|_| HelperStop::Protocol)
}

fn wait_gate() -> Result<(), HelperStop> {
    let Ok(dir) = env::var(GATE_ENV) else {
        return Ok(());
    };
    let dir = PathBuf::from(dir);
    fs::write(dir.join("ready"), b"ready").map_err(|_| HelperStop::Protocol)?;
    let go = dir.join("go");
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if go.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(HelperStop::Protocol)
}

fn read_frame(input: &mut impl Read) -> Result<Vec<u8>, HelperStop> {
    let mut magic = [0u8; 5];
    input
        .read_exact(&mut magic)
        .map_err(|_| HelperStop::Protocol)?;
    if &magic != b"DSWA1" {
        return Err(HelperStop::Protocol);
    }
    let mut key = [0u8; 32];
    input
        .read_exact(&mut key)
        .map_err(|_| HelperStop::Protocol)?;
    let mut len_bytes = [0u8; 4];
    input
        .read_exact(&mut len_bytes)
        .map_err(|_| HelperStop::Protocol)?;
    let len = usize::try_from(u32::from_le_bytes(len_bytes)).unwrap_or(usize::MAX);
    if len > 1024 * 1024 {
        return Err(HelperStop::Protocol);
    }
    let mut body = vec![0u8; len];
    input
        .read_exact(&mut body)
        .map_err(|_| HelperStop::Protocol)?;
    let mut mac = [0u8; 32];
    input
        .read_exact(&mut mac)
        .map_err(|_| HelperStop::Protocol)?;
    if !mac_equal(&mac, &hmac_sha256(&key, &body)) {
        return Err(HelperStop::Protocol);
    }
    Ok(body)
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

enum HelperStop {
    Protocol,
    Refused(&'static str),
    ReadBack,
}

impl HelperStop {
    fn finish(self) -> i32 {
        match self {
            Self::Protocol => 3,
            Self::Refused(reason) => {
                let _ = reason;
                3
            }
            Self::ReadBack => 3,
        }
    }
}
