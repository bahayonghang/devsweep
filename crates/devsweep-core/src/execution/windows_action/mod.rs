//! Closed Windows fixture-action boundary.
//!
//! The request carries an operation id, a closed operation, the live preview
//! digest, the caller SID, the helper hash, and the plan-time target identity.
//! It has no program, argv, shell string, or registry path. The helper target
//! is always `<isolated root>/subject.bin` and `<isolated root>/subject.hiv`.

mod audit;
mod helper;
mod registry;

#[cfg(test)]
mod tests;

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    digest_encoding::hex_digest,
    filesystem::{capture_path_identity, normalize_absolute_path, normalize_path_for_compare},
    process::run_helper,
};

pub use helper::run_windows_action_helper;

const SCHEMA_VERSION: u32 = 1;
const SUBJECT_FILE: &str = "subject.bin";
const SUBJECT_HIVE: &str = "subject.hiv";
const PLAN_FILE: &str = "plan.json";
const MAX_SUBJECT_BYTES: u64 = 1024 * 1024;
const DISCONNECT_PADDING: usize = 256 * 1024;

/// Capability of one action. External confirmation is reserved and never dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionCapability {
    UserScope,
    Elevated,
    Unsupported,
    ExternalConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FixtureOperation {
    Backup,
    Restore,
    IdentityCheck,
}

impl FixtureOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Backup => "fixture.backup",
            Self::Restore => "fixture.restore",
            Self::IdentityCheck => "fixture.identity_check",
        }
    }

    fn capability(self) -> ActionCapability {
        match self {
            Self::Backup | Self::Restore => ActionCapability::Elevated,
            Self::IdentityCheck => ActionCapability::UserScope,
        }
    }
}

/// Test-injected elevation decision. This build never opens a consent prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectedElevation {
    Consented,
    Declined,
    MissingPrompt,
}

/// Test-injected helper fault. The value is a closed token, not a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperFault {
    Hang,
    Crash,
    Disconnect,
    MissingTerminal,
}

impl HelperFault {
    fn as_env(self) -> &'static str {
        match self {
            Self::Hang => "hang",
            Self::Crash => "crash",
            Self::Disconnect => "disconnect",
            Self::MissingTerminal => "missing_terminal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    Forged,
    Replayed,
    StalePreview,
    CrossUser,
    ToolReplacement,
    PathReplacement,
    DigestMismatch,
    UnsupportedOperation,
    ExternalConfirmation,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    IdentityDrift,
    RestoreSubjectChanged,
    DigestMismatch,
    ToolReplacement,
    CrossUser,
    Forged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownKind {
    Timeout,
    Disconnect,
    Crash,
    MissingTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowsActionOutcome {
    Succeeded,
    CanceledBeforeDispatch,
    AuthorizationDeclined,
    ConsentMissing,
    AuditFailed,
    ReadBackFailed,
    Rejected(RejectReason),
    Refused(RefuseReason),
    Unknown(UnknownKind),
}

impl WindowsActionOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsActionReport {
    pub outcome: WindowsActionOutcome,
    pub capability: ActionCapability,
}

#[derive(Debug)]
pub struct WindowsActionDispatch<'a> {
    pub helper_path: &'a Path,
    pub elevation: InjectedElevation,
    pub timeout: Duration,
    pub fault: Option<HelperFault>,
    pub gate_dir: Option<&'a Path>,
    pub cancel_before_dispatch: bool,
}

/// Plan captured by [`prepare_windows_fixture`]. The path lives here, not in the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFixture {
    root: PathBuf,
    root_text: String,
    plan_id: String,
    preview_digest: String,
    caller_user_sid: String,
    tool_identity_hash: String,
    target_identity: String,
}

impl PreparedFixture {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn preview_digest(&self) -> &str {
        &self.preview_digest
    }

    pub fn caller_user_sid(&self) -> &str {
        &self.caller_user_sid
    }

    pub fn tool_identity_hash(&self) -> &str {
        &self.tool_identity_hash
    }

    pub fn target_identity(&self) -> &str {
        &self.target_identity
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanDocument {
    schema_version: u32,
    plan_id: String,
    isolated_root: String,
    bound_user_sid: String,
    tool_identity_hash: String,
    target_identity: String,
    preview_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Subject {
    file_bytes: Vec<u8>,
    hive_bytes: Vec<u8>,
    registry_value: Vec<u8>,
    file_identity: String,
    hive_identity: String,
    target_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupManifest {
    schema_version: u32,
    backup_version: u32,
    file_identity: String,
    hive_identity: String,
    target_identity: String,
    file_sha256: String,
    hive_sha256: String,
    registry_value_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequest {
    schema_version: u32,
    operation_id: String,
    operation: String,
    preview_digest: String,
    caller_user_sid: String,
    tool_identity_hash: String,
    target_identity: String,
}

struct ParsedRequest {
    operation_id: String,
    operation: FixtureOperation,
    preview_digest: String,
    caller_user_sid: String,
    tool_identity_hash: String,
    target_identity: String,
}

enum ParseFail {
    Forged,
    Unsupported,
    External,
}

/// Creates an isolated file and offline-hive subject and records their identity.
pub fn prepare_windows_fixture(
    isolated_root: &Path,
    file_bytes: &[u8],
    registry_bytes: &[u8],
    helper_path: &Path,
) -> io::Result<PreparedFixture> {
    fs::create_dir_all(isolated_root)?;
    let (root, root_text) = canonical_root(isolated_root)?;
    let file_path = root.join(SUBJECT_FILE);
    let hive_path = root.join(SUBJECT_HIVE);
    fs::write(&file_path, file_bytes)?;
    registry::create_hive(&hive_path, registry_bytes)?;
    let subject = read_subject(&root)?;
    if subject.file_bytes != file_bytes || subject.registry_value != registry_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fixture subject did not round-trip",
        ));
    }
    let caller_user_sid = current_user_sid()?;
    let tool_identity_hash = hash_tool(helper_path)?;
    let plan_id = new_plan_id()?;
    let preview_digest = preview_digest(
        &plan_id,
        &caller_user_sid,
        &tool_identity_hash,
        &subject.target_identity,
        &root_text,
    );
    let prepared = PreparedFixture {
        root,
        root_text,
        plan_id,
        preview_digest,
        caller_user_sid,
        tool_identity_hash,
        target_identity: subject.target_identity,
    };
    write_plan(&prepared)?;
    Ok(prepared)
}

/// Parses `request_bytes` and runs the fixture service from the prepared plan.
///
/// One call makes at most one helper spawn. A failure is not retried.
pub fn execute_windows_action(
    prepared: &PreparedFixture,
    request_bytes: &[u8],
    dispatch: &WindowsActionDispatch<'_>,
) -> WindowsActionReport {
    let parsed = match parse_request(request_bytes) {
        Ok(parsed) => parsed,
        Err(ParseFail::Forged) => {
            return rejected(ActionCapability::Unsupported, RejectReason::Forged);
        }
        Err(ParseFail::Unsupported) => {
            return rejected(
                ActionCapability::Unsupported,
                RejectReason::UnsupportedOperation,
            );
        }
        Err(ParseFail::External) => {
            return rejected(
                ActionCapability::ExternalConfirmation,
                RejectReason::ExternalConfirmation,
            );
        }
    };
    let capability = parsed.operation.capability();
    let plan = match load_plan(&prepared.root) {
        Ok(plan) => plan,
        Err(_) => return rejected(capability, RejectReason::Unavailable),
    };
    if !plan_matches_handle(&plan, prepared) {
        return rejected(capability, RejectReason::Forged);
    }

    let live_sid = match current_user_sid() {
        Ok(sid) => sid,
        Err(_) => return rejected(capability, RejectReason::Unavailable),
    };
    let live_tool = match hash_tool(dispatch.helper_path) {
        Ok(hash) => hash,
        Err(_) => return rejected(capability, RejectReason::ToolReplacement),
    };
    let live_subject = match read_subject(&prepared.root) {
        Ok(subject) => subject,
        Err(_) => return rejected(capability, RejectReason::PathReplacement),
    };
    let live_digest = preview_digest(
        &prepared.plan_id,
        &live_sid,
        &live_tool,
        &live_subject.target_identity,
        &prepared.root_text,
    );

    if parsed.caller_user_sid != prepared.caller_user_sid || live_sid != prepared.caller_user_sid {
        return rejected(capability, RejectReason::CrossUser);
    }
    if parsed.tool_identity_hash != prepared.tool_identity_hash
        || live_tool != prepared.tool_identity_hash
    {
        return rejected(capability, RejectReason::ToolReplacement);
    }
    if parsed.target_identity != prepared.target_identity {
        return rejected(capability, RejectReason::Forged);
    }
    if live_subject.target_identity != prepared.target_identity {
        return rejected(capability, RejectReason::PathReplacement);
    }
    if parsed.preview_digest != live_digest {
        return rejected(capability, RejectReason::StalePreview);
    }
    if prepared.preview_digest != live_digest {
        return rejected(capability, RejectReason::DigestMismatch);
    }

    let records = match audit::read(&prepared.root) {
        Ok(records) => records,
        Err(_) => return audit_failed(capability),
    };
    if audit::operation_is_spent(&records, &parsed.operation_id) {
        return rejected(capability, RejectReason::Replayed);
    }
    if dispatch.cancel_before_dispatch {
        return write_parent_terminal(
            prepared,
            &parsed,
            capability,
            "canceled",
            None,
            WindowsActionOutcome::CanceledBeforeDispatch,
        );
    }
    if capability == ActionCapability::Elevated {
        match dispatch.elevation {
            InjectedElevation::Declined => {
                return write_parent_terminal(
                    prepared,
                    &parsed,
                    capability,
                    "authorization_declined",
                    None,
                    WindowsActionOutcome::AuthorizationDeclined,
                );
            }
            InjectedElevation::MissingPrompt => {
                return write_parent_terminal(
                    prepared,
                    &parsed,
                    capability,
                    "consent_missing",
                    None,
                    WindowsActionOutcome::ConsentMissing,
                );
            }
            InjectedElevation::Consented => {}
        }
    }

    let started = audit::started_record(
        &parsed.operation_id,
        parsed.operation,
        &parsed.preview_digest,
        &parsed.target_identity,
        &parsed.tool_identity_hash,
        &parsed.caller_user_sid,
        capability,
    );
    if audit::append(&prepared.root, &started).is_err() {
        return audit_failed(capability);
    }

    let body = match serde_json::to_vec(&envelope_from(prepared, &parsed)) {
        Ok(body) => body,
        Err(_) => return unknown(capability, UnknownKind::MissingTerminal),
    };
    let frame = match helper::seal(&body) {
        Ok(frame) => frame,
        Err(_) => return unknown(capability, UnknownKind::MissingTerminal),
    };
    let extra = if matches!(dispatch.fault, Some(HelperFault::Disconnect)) {
        vec![0u8; DISCONNECT_PADDING]
    } else {
        Vec::new()
    };
    let run = run_helper(
        dispatch.helper_path,
        &frame,
        &extra,
        dispatch.timeout,
        dispatch.fault.map(HelperFault::as_env),
        dispatch.gate_dir,
    );
    classify_after_dispatch(prepared, &parsed, capability, &live_subject, run)
}

fn classify_after_dispatch(
    prepared: &PreparedFixture,
    parsed: &ParsedRequest,
    capability: ActionCapability,
    before: &Subject,
    run: io::Result<crate::process::HelperRun>,
) -> WindowsActionReport {
    let records = audit::read(&prepared.root).ok();
    let terminal = records
        .as_ref()
        .and_then(|records| audit::terminal_for(records, &parsed.operation_id));
    if let Some(record) = terminal {
        return classify_terminal(prepared, parsed.operation, capability, before, record);
    }
    let Ok(run) = run else {
        return unknown(capability, UnknownKind::MissingTerminal);
    };
    if run.timed_out {
        return unknown(capability, UnknownKind::Timeout);
    }
    if run.exit_code.is_some_and(|code| code != 0) {
        return unknown(capability, UnknownKind::Crash);
    }
    if run.stdin_broken {
        return unknown(capability, UnknownKind::Disconnect);
    }
    unknown(capability, UnknownKind::MissingTerminal)
}

fn classify_terminal(
    prepared: &PreparedFixture,
    operation: FixtureOperation,
    capability: ActionCapability,
    before: &Subject,
    record: &audit::AuditRecord,
) -> WindowsActionReport {
    match record.outcome.as_deref() {
        Some("succeeded") if postcondition(prepared, operation, before) => {
            report(WindowsActionOutcome::Succeeded, capability)
        }
        Some("succeeded") => report(WindowsActionOutcome::ReadBackFailed, capability),
        Some("read_back_failed") => report(WindowsActionOutcome::ReadBackFailed, capability),
        Some("refused") => report(
            WindowsActionOutcome::Refused(refuse_reason(record.reason.as_deref())),
            capability,
        ),
        _ => unknown(capability, UnknownKind::MissingTerminal),
    }
}

fn refuse_reason(reason: Option<&str>) -> RefuseReason {
    match reason {
        Some("restore_subject_changed") => RefuseReason::RestoreSubjectChanged,
        Some("digest_mismatch") => RefuseReason::DigestMismatch,
        Some("tool_replacement") => RefuseReason::ToolReplacement,
        Some("cross_user") => RefuseReason::CrossUser,
        Some("forged") => RefuseReason::Forged,
        _ => RefuseReason::IdentityDrift,
    }
}

fn postcondition(
    prepared: &PreparedFixture,
    operation: FixtureOperation,
    before: &Subject,
) -> bool {
    let Ok(after) = read_subject(&prepared.root) else {
        return false;
    };
    match operation {
        FixtureOperation::IdentityCheck => subject_unchanged(before, &after),
        FixtureOperation::Backup => {
            subject_unchanged(before, &after) && backup_matches_live(&prepared.root, &after)
        }
        FixtureOperation::Restore => restore_matches_live(&prepared.root, &after),
    }
}

fn subject_unchanged(before: &Subject, after: &Subject) -> bool {
    before.file_bytes == after.file_bytes
        && before.hive_bytes == after.hive_bytes
        && before.registry_value == after.registry_value
        && before.target_identity == after.target_identity
}

fn backup_matches_live(root: &Path, live: &Subject) -> bool {
    let Ok(Some(dir)) = latest_backup_dir(root) else {
        return false;
    };
    backup_files_match(&dir, live) && manifest_identity_matches(&dir, live)
}

fn restore_matches_live(root: &Path, live: &Subject) -> bool {
    let Ok(Some(dir)) = latest_backup_dir(root) else {
        return false;
    };
    backup_files_match(&dir, live) && manifest_identity_matches(&dir, live)
}

fn backup_files_match(dir: &Path, live: &Subject) -> bool {
    let Ok(file_bytes) = fs::read(dir.join("file.bin")) else {
        return false;
    };
    let Ok(hive_bytes) = fs::read(dir.join("hive.bin")) else {
        return false;
    };
    let Ok(value_bytes) = fs::read(dir.join("registry-value.bin")) else {
        return false;
    };
    file_bytes == live.file_bytes
        && hive_bytes == live.hive_bytes
        && value_bytes == live.registry_value
}

fn manifest_identity_matches(dir: &Path, live: &Subject) -> bool {
    let Ok(text) = fs::read_to_string(dir.join("manifest.json")) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_str::<BackupManifest>(&text) else {
        return false;
    };
    manifest.schema_version == SCHEMA_VERSION
        && manifest.file_identity == live.file_identity
        && manifest.hive_identity == live.hive_identity
        && manifest.target_identity == live.target_identity
        && manifest.file_sha256 == content_digest(&live.file_bytes)
        && manifest.hive_sha256 == content_digest(&live.hive_bytes)
        && manifest.registry_value_sha256 == content_digest(&live.registry_value)
}

fn write_parent_terminal(
    prepared: &PreparedFixture,
    parsed: &ParsedRequest,
    capability: ActionCapability,
    outcome: &str,
    reason: Option<&str>,
    report_outcome: WindowsActionOutcome,
) -> WindowsActionReport {
    let record = audit::terminal_record(audit::TerminalParts {
        operation_id: &parsed.operation_id,
        operation: parsed.operation.as_str(),
        preview_digest: &parsed.preview_digest,
        target_identity: &parsed.target_identity,
        tool_identity_hash: &parsed.tool_identity_hash,
        caller_user_sid: &parsed.caller_user_sid,
        capability,
        outcome,
        reason,
    });
    if audit::append(&prepared.root, &record).is_err() {
        return audit_failed(capability);
    }
    report(report_outcome, capability)
}

fn envelope_from(prepared: &PreparedFixture, parsed: &ParsedRequest) -> helper::Envelope {
    helper::Envelope {
        schema_version: SCHEMA_VERSION,
        operation_id: parsed.operation_id.clone(),
        operation: parsed.operation.as_str().to_string(),
        preview_digest: parsed.preview_digest.clone(),
        caller_user_sid: parsed.caller_user_sid.clone(),
        tool_identity_hash: parsed.tool_identity_hash.clone(),
        target_identity: parsed.target_identity.clone(),
        isolated_root: prepared.root_text.clone(),
        plan_id: prepared.plan_id.clone(),
    }
}

fn plan_matches_handle(plan: &PlanDocument, prepared: &PreparedFixture) -> bool {
    plan.schema_version == SCHEMA_VERSION
        && plan.plan_id == prepared.plan_id
        && plan.isolated_root == prepared.root_text
        && plan.bound_user_sid == prepared.caller_user_sid
        && plan.tool_identity_hash == prepared.tool_identity_hash
        && plan.target_identity == prepared.target_identity
        && plan.preview_digest == prepared.preview_digest
}

fn write_plan(prepared: &PreparedFixture) -> io::Result<()> {
    let document = PlanDocument {
        schema_version: SCHEMA_VERSION,
        plan_id: prepared.plan_id.clone(),
        isolated_root: prepared.root_text.clone(),
        bound_user_sid: prepared.caller_user_sid.clone(),
        tool_identity_hash: prepared.tool_identity_hash.clone(),
        target_identity: prepared.target_identity.clone(),
        preview_digest: prepared.preview_digest.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let path = prepared.root.join(PLAN_FILE);
    let mut file = fs::File::create(&path)?;
    file.write_all(&bytes)?;
    file.sync_all()
}

fn load_plan(root: &Path) -> io::Result<PlanDocument> {
    let bytes = fs::read(root.join(PLAN_FILE))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn parse_request(bytes: &[u8]) -> Result<ParsedRequest, ParseFail> {
    let raw: RawRequest = serde_json::from_slice(bytes).map_err(|_| ParseFail::Forged)?;
    if raw.schema_version != SCHEMA_VERSION
        || !valid_token(&raw.operation_id)
        || !valid_digest(&raw.preview_digest)
        || !valid_digest(&raw.tool_identity_hash)
        || !valid_token(&raw.caller_user_sid)
        || !valid_token(&raw.target_identity)
    {
        return Err(ParseFail::Forged);
    }
    let operation = match raw.operation.as_str() {
        "fixture.backup" => FixtureOperation::Backup,
        "fixture.restore" => FixtureOperation::Restore,
        "fixture.identity_check" => FixtureOperation::IdentityCheck,
        "external.confirm" => return Err(ParseFail::External),
        _ => return Err(ParseFail::Unsupported),
    };
    Ok(ParsedRequest {
        operation_id: raw.operation_id,
        operation,
        preview_digest: raw.preview_digest,
        caller_user_sid: raw.caller_user_sid,
        tool_identity_hash: raw.tool_identity_hash,
        target_identity: raw.target_identity,
    })
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_token(value: &str) -> bool {
    let length = value.len();
    (1..=256).contains(&length)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'/' && byte != b'\\')
}

fn read_subject(root: &Path) -> io::Result<Subject> {
    let file_path = root.join(SUBJECT_FILE);
    let hive_path = root.join(SUBJECT_HIVE);
    let registry_value = registry::read_value(&hive_path)?;
    let file_bytes = read_limited(&file_path)?;
    let hive_bytes = read_limited(&hive_path)?;
    let file_identity = identity_token(&file_path)?;
    let hive_identity = identity_token(&hive_path)?;
    let target_identity = format!("file={file_identity};hive={hive_identity}");
    Ok(Subject {
        file_bytes,
        hive_bytes,
        registry_value,
        file_identity,
        hive_identity,
        target_identity,
    })
}

fn read_limited(path: &Path) -> io::Result<Vec<u8>> {
    let len = fs::metadata(path)?.len();
    if len > MAX_SUBJECT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fixture subject is too large",
        ));
    }
    fs::read(path)
}

fn identity_token(path: &Path) -> io::Result<String> {
    let identity = capture_path_identity(path)?;
    Ok(format!("{:?}:{}", identity.kind, identity.value))
}

fn canonical_root(path: &Path) -> io::Result<(PathBuf, String)> {
    let canonical = path.canonicalize()?;
    let compared = normalize_path_for_compare(&canonical);
    let text = compared.to_str().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "fixture root is not unicode")
    })?;
    let normalized = normalize_absolute_path(Path::new(text))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    Ok((compared, normalized))
}

fn preview_digest(plan_id: &str, sid: &str, tool: &str, identity: &str, root: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"devsweep-windows-action-preview-v1\0");
    for part in [plan_id, sid, tool, identity, root] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", hex_digest(hasher.finalize()))
}

fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", hex_digest(Sha256::digest(bytes)))
}

fn hash_tool(path: &Path) -> io::Result<String> {
    Ok(content_digest(&fs::read(path)?))
}

fn new_plan_id() -> io::Result<String> {
    let mut bytes = [0u8; 16];
    fill_random(&mut bytes)?;
    Ok(hex_digest(bytes))
}

fn fill_random(dest: &mut [u8]) -> io::Result<()> {
    #[cfg(windows)]
    {
        #[link(name = "advapi32")]
        unsafe extern "system" {
            #[link_name = "SystemFunction036"]
            fn rtl_gen_random(buffer: *mut u8, length: u32) -> u8;
        }
        let length = u32::try_from(dest.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "random buffer is too large")
        })?;
        let ok = unsafe { rtl_gen_random(dest.as_mut_ptr(), length) };
        if ok == 0 {
            return Err(io::Error::other("random bytes are unavailable"));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let mut file = fs::File::open("/dev/urandom")?;
        io::Read::read_exact(&mut file, dest)
    }
}

fn current_user_sid() -> io::Result<String> {
    #[cfg(windows)]
    {
        windows_user_sid()
    }
    #[cfg(not(windows))]
    {
        Ok(format!("unix:{}", unsafe { libc::geteuid() }))
    }
}

#[cfg(windows)]
fn windows_user_sid() -> io::Result<String> {
    use std::ptr;

    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    let mut token = ptr::null_mut();
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) };
    if opened == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut needed = 0u32;
    unsafe {
        GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &raw mut needed);
    }
    if needed == 0 {
        unsafe { CloseHandle(token) };
        return Err(io::Error::other("process token has no user"));
    }
    let words = usize::try_from(needed).unwrap_or(0).div_ceil(8);
    let mut buffer = vec![0u64; words.max(1)];
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &raw mut needed,
        )
    };
    unsafe { CloseHandle(token) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    sid_string(user.User.Sid)
}

#[cfg(windows)]
fn sid_string(sid: windows_sys::Win32::Security::PSID) -> io::Result<String> {
    use std::ptr;

    if sid.is_null() {
        return Err(io::Error::other("process token SID is null"));
    }
    let base = sid.cast::<u8>();
    let revision = unsafe { ptr::read_unaligned(base) };
    let count = unsafe { ptr::read_unaligned(base.add(1)) };
    if revision == 0 || count > 15 {
        return Err(io::Error::other("process token SID is invalid"));
    }
    let mut authority = [0u8; 6];
    unsafe { ptr::copy_nonoverlapping(base.add(2), authority.as_mut_ptr(), 6) };
    let authority = authority.iter().fold(0u64, |value, byte| {
        value.checked_shl(8).unwrap_or(0) | u64::from(*byte)
    });
    let subs = unsafe { base.add(8).cast::<u32>() };
    let mut text = format!("S-{revision}-{authority}");
    for index in 0..count {
        let sub = unsafe { ptr::read_unaligned(subs.add(usize::from(index))) };
        text.push('-');
        text.push_str(&sub.to_string());
    }
    Ok(text)
}

fn latest_backup_dir(root: &Path) -> io::Result<Option<PathBuf>> {
    let dir = root.join("backup");
    if !dir.exists() {
        return Ok(None);
    }
    let mut best: Option<u32> = None;
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str().and_then(|name| name.strip_prefix('v')) else {
            continue;
        };
        let Ok(version) = name.parse::<u32>() else {
            continue;
        };
        best = Some(best.map_or(version, |current| current.max(version)));
    }
    Ok(best.map(|version| dir.join(format!("v{version}"))))
}

fn next_backup_version(root: &Path) -> io::Result<u32> {
    let dir = root.join("backup");
    if !dir.exists() {
        return Ok(1);
    }
    let mut best = 0u32;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str().and_then(|name| name.strip_prefix('v')) else {
            continue;
        };
        if let Ok(version) = name.parse::<u32>() {
            best = best.max(version);
        }
    }
    Ok(best.saturating_add(1))
}

fn rejected(capability: ActionCapability, reason: RejectReason) -> WindowsActionReport {
    report(WindowsActionOutcome::Rejected(reason), capability)
}

fn audit_failed(capability: ActionCapability) -> WindowsActionReport {
    report(WindowsActionOutcome::AuditFailed, capability)
}

fn unknown(capability: ActionCapability, kind: UnknownKind) -> WindowsActionReport {
    report(WindowsActionOutcome::Unknown(kind), capability)
}

fn report(outcome: WindowsActionOutcome, capability: ActionCapability) -> WindowsActionReport {
    WindowsActionReport {
        outcome,
        capability,
    }
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut key_block = [0u8; BLOCK];
    if key.len() > BLOCK {
        key_block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; BLOCK];
    let mut opad = [0u8; BLOCK];
    for index in 0..BLOCK {
        ipad[index] = key_block[index] ^ 0x36;
        opad[index] = key_block[index] ^ 0x5c;
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    let digest = outer.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn mac_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in left.iter().zip(right) {
        diff |= left ^ right;
    }
    diff == 0
}
