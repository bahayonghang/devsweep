//! Startup-item inventory and the current-user `StartupApproved` toggle.
//!
//! The toggle writes only the `Explorer\StartupApproved` value that Windows
//! Task Manager uses. It never deletes a Run value or a shortcut and never
//! executes a startup command line. Machine entries are view-only.

use std::{
    fmt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::digest_encoding::hex_digest;

use super::{
    RegistryHive, RegistryView, SoftwareAuditError, SoftwareScope, SoftwareSourceState,
    audit::{
        SoftwareAuditJournal, SoftwareSupportAuditRecordV1, SoftwareSupportJournal,
        SoftwareSupportLifecycleRecordV1, SoftwareSupportOperation, SoftwareSupportTerminalResult,
        SupportAppendFault, is_sha256_digest, is_sha256_hex,
    },
    software_audit_v1_path, unix_ms,
};

/// Startup inventory and toggle wire version.
pub const SOFTWARE_STARTUP_VERSION: u32 = 1;

const CURRENT_VERSION_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion";
const APPROVED_KEY: &str = r"Explorer\StartupApproved";
const FILETIME_UNIX_EPOCH: u64 = 116_444_736_000_000_000;

/// Exact startup source. Scope and toggle authority derive from this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareStartupLocation {
    CurrentUserRun,
    MachineRun,
    MachineRun32,
    CurrentUserStartupFolder,
}

impl SoftwareStartupLocation {
    const ALL: [Self; 4] = [
        Self::CurrentUserRun,
        Self::MachineRun,
        Self::MachineRun32,
        Self::CurrentUserStartupFolder,
    ];

    fn scope(self) -> SoftwareScope {
        match self {
            Self::CurrentUserRun | Self::CurrentUserStartupFolder => SoftwareScope::CurrentUser,
            Self::MachineRun | Self::MachineRun32 => SoftwareScope::Machine,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::CurrentUserRun => "current_user_run",
            Self::MachineRun => "machine_run",
            Self::MachineRun32 => "machine_run32",
            Self::CurrentUserStartupFolder => "current_user_startup_folder",
        }
    }
}

/// Decoded `StartupApproved` state. A missing value means enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareStartupState {
    Enabled,
    Disabled,
    /// The approval value exists but is not a readable binary value.
    Unknown,
}

/// Whether DevSweep may write the approval value for this entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareStartupToggle {
    Allowed,
    RequiresAdministrator,
}

/// One startup entry. It carries no command line and no executable path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareStartupEntryV1 {
    pub id: String,
    pub location: SoftwareStartupLocation,
    pub scope: SoftwareScope,
    /// Run value name or Startup-folder file name.
    pub name: String,
    pub state: SoftwareStartupState,
    pub toggle: SoftwareStartupToggle,
}

/// Evidence for one startup source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareStartupSourceV1 {
    pub location: SoftwareStartupLocation,
    pub state: SoftwareSourceState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
}

/// Read-only startup inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareStartupListV1 {
    pub version: u32,
    pub observed_at_unix_ms: u64,
    pub sources: Vec<SoftwareStartupSourceV1>,
    pub entries: Vec<SoftwareStartupEntryV1>,
}

/// Closed outcome of one Software support action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareSupportOutcomeCode {
    Succeeded,
    Failed,
    Skipped,
}

/// Stable error code of one Software support action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareSupportErrorCode {
    RegistryWriteFailed,
    VerificationFailed,
    TrashFailed,
    NotPresent,
    UnsafePath,
    Protected,
    Canceled,
}

/// Result of one startup toggle. The audit record is already durable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareStartupToggleReportV1 {
    pub version: u32,
    pub operation_id: String,
    pub requested_enabled: bool,
    pub outcome: SoftwareSupportOutcomeCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<SoftwareSupportErrorCode>,
    /// Live re-read of the entry after the write.
    pub entry: SoftwareStartupEntryV1,
}

/// Failures before any registry write.
#[derive(Debug)]
pub enum SoftwareStartupError {
    ConfirmationRequired,
    UnsupportedPlanVersion(u32),
    InvalidPlan,
    /// The entry id is not in the live startup inventory.
    UnknownEntry(String),
    RequiresAdministrator(String),
    Unsupported,
    StartupSourceUnavailable,
    StartupIdentityUnavailable,
    StartupEntryChanged,
    StartupStateChanged,
    StaleConfirmation,
    AuditUnavailable,
    Protected(String),
    Audit(SoftwareAuditError),
}

impl fmt::Display for SoftwareStartupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfirmationRequired => {
                formatter.write_str("startup toggle requires explicit confirmation")
            }
            Self::UnknownEntry(id) => write!(formatter, "startup entry {id} is not present"),
            Self::RequiresAdministrator(id) => {
                write!(formatter, "startup entry {id} requires administrator")
            }
            Self::Unsupported => formatter.write_str("startup items are Windows-only"),
            Self::UnsupportedPlanVersion(version) => {
                write!(formatter, "unsupported startup plan version {version}")
            }
            Self::InvalidPlan => formatter.write_str("startup plan is invalid"),
            Self::StartupSourceUnavailable => {
                formatter.write_str("startup source is not a complete writable observation")
            }
            Self::StartupIdentityUnavailable => {
                formatter.write_str("startup entry identity is unavailable")
            }
            Self::StartupEntryChanged => formatter.write_str("startup entry identity changed"),
            Self::StartupStateChanged => formatter.write_str("startup approval identity changed"),
            Self::StaleConfirmation => formatter.write_str("startup preview digest is stale"),
            Self::AuditUnavailable => formatter.write_str("startup support audit is unavailable"),
            Self::Protected(id) => write!(formatter, "startup entry {id} is protected"),
            Self::Audit(error) => error.fmt(formatter),
        }
    }
}

impl SoftwareStartupError {
    /// Stable machine code. Display text is not a contract.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConfirmationRequired => "confirmation_required",
            Self::UnsupportedPlanVersion(_) => "unsupported_plan_version",
            Self::InvalidPlan => "invalid_plan",
            Self::UnknownEntry(_) => "unknown_entry",
            Self::RequiresAdministrator(_) => "requires_administrator",
            Self::Unsupported => "unsupported_platform",
            Self::StartupSourceUnavailable => "startup_source_unavailable",
            Self::StartupIdentityUnavailable => "startup_identity_unavailable",
            Self::StartupEntryChanged => "startup_entry_changed",
            Self::StartupStateChanged => "startup_state_changed",
            Self::StaleConfirmation => "stale_confirmation",
            Self::AuditUnavailable | Self::Audit(_) => "audit_unavailable",
            Self::Protected(_) => "protected",
        }
    }
}

impl std::error::Error for SoftwareStartupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Audit(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SoftwareAuditError> for SoftwareStartupError {
    fn from(value: SoftwareAuditError) -> Self {
        Self::Audit(value)
    }
}

/// Registry and folder locations for the four startup sources.
#[derive(Debug, Clone)]
struct StartupSpecs {
    user_hive: RegistryHive,
    user_base: String,
    machine_hive: RegistryHive,
    machine_base: String,
    /// Separate 32-bit Run key base. Production uses the 32-bit view of
    /// `machine_base`; tests use a distinct key because HKCU is not redirected.
    machine_run32_base: String,
    startup_folder: Option<PathBuf>,
}

impl StartupSpecs {
    fn production() -> Self {
        Self {
            user_hive: RegistryHive::CurrentUser,
            user_base: CURRENT_VERSION_KEY.to_string(),
            machine_hive: RegistryHive::LocalMachine,
            machine_base: CURRENT_VERSION_KEY.to_string(),
            machine_run32_base: CURRENT_VERSION_KEY.to_string(),
            startup_folder: std::env::var_os("APPDATA")
                .filter(|value| !value.is_empty())
                .map(|root| {
                    PathBuf::from(root).join(r"Microsoft\Windows\Start Menu\Programs\Startup")
                }),
        }
    }

    /// Returns `(hive, view, key)` for the Run key of a registry location.
    fn run_key(
        &self,
        location: SoftwareStartupLocation,
    ) -> Option<(RegistryHive, RegistryView, String)> {
        match location {
            SoftwareStartupLocation::CurrentUserRun => Some((
                self.user_hive,
                RegistryView::Registry64,
                format!(r"{}\Run", self.user_base),
            )),
            SoftwareStartupLocation::MachineRun => Some((
                self.machine_hive,
                RegistryView::Registry64,
                format!(r"{}\Run", self.machine_base),
            )),
            SoftwareStartupLocation::MachineRun32 => Some((
                self.machine_hive,
                RegistryView::Registry32,
                format!(r"{}\Run", self.machine_run32_base),
            )),
            SoftwareStartupLocation::CurrentUserStartupFolder => None,
        }
    }

    /// Returns `(hive, key)` of the matching `StartupApproved` subkey.
    fn approved_key(&self, location: SoftwareStartupLocation) -> (RegistryHive, String) {
        match location {
            SoftwareStartupLocation::CurrentUserRun => (
                self.user_hive,
                format!(r"{}\{APPROVED_KEY}\Run", self.user_base),
            ),
            SoftwareStartupLocation::MachineRun => (
                self.machine_hive,
                format!(r"{}\{APPROVED_KEY}\Run", self.machine_base),
            ),
            SoftwareStartupLocation::MachineRun32 => (
                self.machine_hive,
                format!(r"{}\{APPROVED_KEY}\Run32", self.machine_base),
            ),
            SoftwareStartupLocation::CurrentUserStartupFolder => (
                self.user_hive,
                format!(r"{}\{APPROVED_KEY}\StartupFolder", self.user_base),
            ),
        }
    }
}

/// Lists startup entries from the four fixed sources. Read-only.
pub fn list_startup_entries() -> SoftwareStartupListV1 {
    list_with(&StartupSpecs::production())
}

/// Enables or disables one current-user startup entry by writing only its
/// `StartupApproved` value, re-reading it, and appending a Software audit record.
pub fn set_startup_enabled(
    entry_id: &str,
    enabled: bool,
    confirmed: bool,
) -> Result<SoftwareStartupToggleReportV1, SoftwareStartupError> {
    let audit_path = software_audit_v1_path()?;
    set_with(
        &StartupSpecs::production(),
        &audit_path,
        entry_id,
        enabled,
        confirmed,
    )
}

fn list_with(specs: &StartupSpecs) -> SoftwareStartupListV1 {
    let mut sources = Vec::new();
    let mut entries = Vec::new();
    for location in SoftwareStartupLocation::ALL {
        let (source, names) = native::enumerate(specs, location);
        sources.push(source);
        for name in names {
            let state = native::approval_state(specs, location, &name);
            entries.push(entry(location, name, state));
        }
    }
    entries.sort_by(|left, right| {
        left.location
            .cmp(&right.location)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    SoftwareStartupListV1 {
        version: SOFTWARE_STARTUP_VERSION,
        observed_at_unix_ms: unix_ms(SystemTime::now()),
        sources,
        entries,
    }
}

fn entry(
    location: SoftwareStartupLocation,
    name: String,
    state: SoftwareStartupState,
) -> SoftwareStartupEntryV1 {
    let scope = location.scope();
    SoftwareStartupEntryV1 {
        id: startup_id(location, &name),
        location,
        scope,
        name,
        state,
        toggle: if scope == SoftwareScope::CurrentUser {
            SoftwareStartupToggle::Allowed
        } else {
            SoftwareStartupToggle::RequiresAdministrator
        },
    }
}

fn startup_id(location: SoftwareStartupLocation, name: &str) -> String {
    let digest = hex_digest(Sha256::digest(format!(
        "devsweep.startup.v1\0{}\0{name}",
        location.tag()
    )));
    format!("startup:v1:{digest}")
}

/// The id [`plan_startup_hive`] expects for one fixture entry.
///
/// This is the same identity the hive planner hashes. It does not read the
/// host registry and it does not write.
#[must_use]
pub fn startup_fixture_entry_id(location: SoftwareStartupLocation, name: &str) -> String {
    startup_id(location, name)
}

fn set_with(
    specs: &StartupSpecs,
    audit_path: &std::path::Path,
    entry_id: &str,
    enabled: bool,
    confirmed: bool,
) -> Result<SoftwareStartupToggleReportV1, SoftwareStartupError> {
    if !confirmed {
        return Err(SoftwareStartupError::ConfirmationRequired);
    }
    if !cfg!(windows) {
        return Err(SoftwareStartupError::Unsupported);
    }
    let live = list_with(specs);
    let target = live
        .entries
        .into_iter()
        .find(|entry| entry.id == entry_id)
        .ok_or_else(|| SoftwareStartupError::UnknownEntry(entry_id.to_string()))?;
    if target.toggle != SoftwareStartupToggle::Allowed {
        return Err(SoftwareStartupError::RequiresAdministrator(
            entry_id.to_string(),
        ));
    }
    // The journal lock is held before the write so an unavailable audit store
    // prevents the side effect.
    let mut journal = SoftwareAuditJournal::open(audit_path)?;
    let now = unix_ms(SystemTime::now());
    let bytes = approval_bytes(enabled, now);
    let written = native::write_approval(specs, target.location, &target.name, &bytes);
    let observed = native::approval_state(specs, target.location, &target.name);
    let expected = if enabled {
        SoftwareStartupState::Enabled
    } else {
        SoftwareStartupState::Disabled
    };
    let (outcome, error_code) = if !written {
        (
            SoftwareSupportOutcomeCode::Failed,
            Some(SoftwareSupportErrorCode::RegistryWriteFailed),
        )
    } else if observed != expected {
        (
            SoftwareSupportOutcomeCode::Failed,
            Some(SoftwareSupportErrorCode::VerificationFailed),
        )
    } else {
        (SoftwareSupportOutcomeCode::Succeeded, None)
    };
    let operation_id = next_operation_id(now);
    journal.append_support(SoftwareSupportAuditRecordV1::startup_toggled(
        &operation_id,
        now,
        &target.id,
        target.location,
        enabled,
        outcome,
        error_code,
    ))?;
    Ok(SoftwareStartupToggleReportV1 {
        version: SOFTWARE_STARTUP_VERSION,
        operation_id,
        requested_enabled: enabled,
        outcome,
        error_code,
        entry: SoftwareStartupEntryV1 {
            state: observed,
            ..target
        },
    })
}

fn next_operation_id(now: u64) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let sequence = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!(
        "software-startup-op-{now:016x}-{:08x}-{sequence:016x}",
        std::process::id()
    )
}

/// Decodes the first byte of a `StartupApproved` value: even is enabled.
fn decode_approval(bytes: Option<&[u8]>) -> SoftwareStartupState {
    match bytes {
        None => SoftwareStartupState::Enabled,
        Some([first, ..]) if first % 2 == 0 => SoftwareStartupState::Enabled,
        Some([_, ..]) => SoftwareStartupState::Disabled,
        Some([]) => SoftwareStartupState::Unknown,
    }
}

/// Closed saved startup toggle. Identity hashes are lowercase SHA-256 hex.
/// The plan has no registry path, value bytes, or command line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupTogglePlanV1 {
    pub version: u32,
    pub entry_id: String,
    pub requested_enabled: bool,
    pub expected_entry_identity_sha256: String,
    pub expected_approval_identity_sha256: String,
}

/// Live preview of one saved startup toggle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupTogglePreviewV1 {
    pub version: u32,
    pub entry: SoftwareStartupEntryV1,
    pub before_state: SoftwareStartupState,
    pub after_state: SoftwareStartupState,
    pub digest: String,
}

/// Strict toggle result. `already_in_state` performs no registry or audit write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupStrictOutcome {
    Succeeded,
    Failed,
    Unknown,
    AlreadyInState,
}

/// Result of the strict startup wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupToggleStrictReportV1 {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    pub requested_enabled: bool,
    pub outcome: StartupStrictOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub entry: SoftwareStartupEntryV1,
}

/// Startup plan wire version.
pub const STARTUP_TOGGLE_PLAN_VERSION: u32 = 1;

const STARTUP_CONTENT_BOUND: u64 = 1_048_576;

/// The 12-byte Task Manager layout: flag byte, three zero bytes, then a
/// FILETIME that is zero on enable and the disable time on disable.
fn approval_bytes(enabled: bool, now_unix_ms: u64) -> [u8; 12] {
    let mut bytes = [0_u8; 12];
    bytes[0] = if enabled { 0x02 } else { 0x03 };
    if !enabled {
        let filetime = now_unix_ms
            .saturating_mul(10_000)
            .saturating_add(FILETIME_UNIX_EPOCH);
        bytes[4..].copy_from_slice(&filetime.to_le_bytes());
    }
    bytes
}

/// Observes one live entry and returns a saved toggle plan. No write.
pub fn plan_startup_toggle(
    entry_id: &str,
    requested_enabled: bool,
) -> Result<StartupTogglePlanV1, SoftwareStartupError> {
    plan_startup_with(&StartupSpecs::production(), entry_id, requested_enabled)
}

/// Re-observes the saved entry and returns the redacted row plus a live digest.
pub fn preview_startup_toggle(
    plan: &StartupTogglePlanV1,
) -> Result<StartupTogglePreviewV1, SoftwareStartupError> {
    preview_startup_with(&StartupSpecs::production(), plan)
}

/// Validates the saved plan, binds the live digest, and writes only the fixed
/// approval value. Does not call [`set_startup_enabled`].
pub fn execute_startup_toggle(
    plan: &StartupTogglePlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
) -> Result<StartupToggleStrictReportV1, SoftwareStartupError> {
    let audit_path = software_audit_v1_path()?;
    execute_startup_with(
        &StartupSpecs::production(),
        &audit_path,
        plan,
        expected_preview_digest,
        confirmed,
        SupportAppendFault::NONE,
        None,
    )
}

fn plan_startup_with(
    specs: &StartupSpecs,
    entry_id: &str,
    requested_enabled: bool,
) -> Result<StartupTogglePlanV1, SoftwareStartupError> {
    if !cfg!(windows) {
        return Err(SoftwareStartupError::Unsupported);
    }
    let observed = observe_startup_entry(specs, entry_id)?;
    Ok(StartupTogglePlanV1 {
        version: STARTUP_TOGGLE_PLAN_VERSION,
        entry_id: observed.entry.id,
        requested_enabled,
        expected_entry_identity_sha256: observed.entry_identity_sha256,
        expected_approval_identity_sha256: observed.approval_identity_sha256,
    })
}

fn preview_startup_with(
    specs: &StartupSpecs,
    plan: &StartupTogglePlanV1,
) -> Result<StartupTogglePreviewV1, SoftwareStartupError> {
    if !cfg!(windows) {
        return Err(SoftwareStartupError::Unsupported);
    }
    let observed = observe_matching(specs, plan)?;
    Ok(StartupTogglePreviewV1 {
        version: STARTUP_TOGGLE_PLAN_VERSION,
        digest: startup_digest(plan, observed.entry.state)?,
        after_state: requested_state(plan.requested_enabled),
        before_state: observed.entry.state,
        entry: observed.entry,
    })
}

fn execute_startup_with(
    specs: &StartupSpecs,
    audit_path: &std::path::Path,
    plan: &StartupTogglePlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
    fault: SupportAppendFault,
    readback_override: Option<SoftwareStartupState>,
) -> Result<StartupToggleStrictReportV1, SoftwareStartupError> {
    if !confirmed {
        return Err(SoftwareStartupError::ConfirmationRequired);
    }
    if !cfg!(windows) {
        return Err(SoftwareStartupError::Unsupported);
    }
    validate_startup_plan(plan)?;
    if !is_sha256_digest(expected_preview_digest) {
        return Err(SoftwareStartupError::InvalidPlan);
    }
    let observed = observe_matching(specs, plan)?;
    let digest = startup_digest(plan, observed.entry.state)?;
    if digest != expected_preview_digest {
        return Err(SoftwareStartupError::StaleConfirmation);
    }
    if observed.entry.state == requested_state(plan.requested_enabled) {
        return Ok(strict_report(
            plan,
            None,
            StartupStrictOutcome::AlreadyInState,
            None,
            observed.entry,
        ));
    }

    let mut journal = SoftwareSupportJournal::open(audit_path).map_err(map_startup_audit)?;
    let observed = observe_matching(specs, plan)?;
    let digest = startup_digest(plan, observed.entry.state)?;
    if digest != expected_preview_digest {
        return Err(SoftwareStartupError::StaleConfirmation);
    }
    if observed.entry.state == requested_state(plan.requested_enabled) {
        return Ok(strict_report(
            plan,
            None,
            StartupStrictOutcome::AlreadyInState,
            None,
            observed.entry,
        ));
    }

    let operation_id = next_operation_id(unix_ms(SystemTime::now()));
    let plan_identity = startup_plan_identity(plan)?;
    journal
        .append_lifecycle(
            SoftwareSupportLifecycleRecordV1::started(
                SoftwareSupportOperation::StartupToggle,
                &operation_id,
                unix_ms(SystemTime::now()),
                &plan_identity,
                &digest,
            ),
            fault,
        )
        .map_err(|_| SoftwareStartupError::AuditUnavailable)?;

    let rechecked = observe_matching(specs, plan);
    let rechecked = match rechecked {
        Ok(value) => value,
        Err(error) => {
            return finish_without_write(
                &mut journal,
                fault,
                plan,
                &StartupAuditBinding {
                    operation_id: &operation_id,
                    plan_identity: &plan_identity,
                    digest: &digest,
                },
                &observed.entry,
                error,
            );
        }
    };
    if startup_digest(plan, rechecked.entry.state)? != digest {
        return finish_without_write(
            &mut journal,
            fault,
            plan,
            &StartupAuditBinding {
                operation_id: &operation_id,
                plan_identity: &plan_identity,
                digest: &digest,
            },
            &rechecked.entry,
            SoftwareStartupError::StaleConfirmation,
        );
    }

    let expected = requested_state(plan.requested_enabled);
    let now = unix_ms(SystemTime::now());
    let bytes = approval_bytes(plan.requested_enabled, now);
    let written = native::write_approval(
        specs,
        rechecked.entry.location,
        &rechecked.entry.name,
        &bytes,
    );
    let observed_state = readback_override.unwrap_or_else(|| {
        native::approval_state(specs, rechecked.entry.location, &rechecked.entry.name)
    });
    let (outcome, error_code) = if !written {
        (StartupStrictOutcome::Failed, Some("registry_write_failed"))
    } else if observed_state != expected {
        (StartupStrictOutcome::Failed, Some("verification_failed"))
    } else {
        (StartupStrictOutcome::Succeeded, None)
    };
    let terminal = if outcome == StartupStrictOutcome::Succeeded {
        SoftwareSupportTerminalResult::Succeeded
    } else {
        SoftwareSupportTerminalResult::Failed
    };
    let entry = SoftwareStartupEntryV1 {
        state: observed_state,
        ..rechecked.entry
    };
    if journal
        .append_lifecycle(
            SoftwareSupportLifecycleRecordV1::terminal(
                SoftwareSupportOperation::StartupToggle,
                &operation_id,
                unix_ms(SystemTime::now()),
                &plan_identity,
                &digest,
                terminal,
                error_code,
            ),
            fault,
        )
        .is_err()
    {
        return Ok(strict_report(
            plan,
            Some(operation_id),
            StartupStrictOutcome::Unknown,
            Some("audit_unavailable".to_string()),
            entry,
        ));
    }
    Ok(strict_report(
        plan,
        Some(operation_id),
        outcome,
        error_code.map(str::to_string),
        entry,
    ))
}

struct StartupAuditBinding<'a> {
    operation_id: &'a str,
    plan_identity: &'a str,
    digest: &'a str,
}

fn finish_without_write(
    journal: &mut SoftwareSupportJournal,
    fault: SupportAppendFault,
    plan: &StartupTogglePlanV1,
    binding: &StartupAuditBinding<'_>,
    entry: &SoftwareStartupEntryV1,
    error: SoftwareStartupError,
) -> Result<StartupToggleStrictReportV1, SoftwareStartupError> {
    let recorded = journal.append_lifecycle(
        SoftwareSupportLifecycleRecordV1::terminal(
            SoftwareSupportOperation::StartupToggle,
            binding.operation_id,
            unix_ms(SystemTime::now()),
            binding.plan_identity,
            binding.digest,
            SoftwareSupportTerminalResult::Refused,
            Some(error.code()),
        ),
        fault,
    );
    if recorded.is_err() {
        return Ok(strict_report(
            plan,
            Some(binding.operation_id.to_string()),
            StartupStrictOutcome::Unknown,
            Some("audit_unavailable".to_string()),
            entry.clone(),
        ));
    }
    Err(error)
}

fn strict_report(
    plan: &StartupTogglePlanV1,
    operation_id: Option<String>,
    outcome: StartupStrictOutcome,
    error_code: Option<String>,
    entry: SoftwareStartupEntryV1,
) -> StartupToggleStrictReportV1 {
    StartupToggleStrictReportV1 {
        version: STARTUP_TOGGLE_PLAN_VERSION,
        operation_id,
        requested_enabled: plan.requested_enabled,
        outcome,
        error_code,
        entry,
    }
}

fn requested_state(enabled: bool) -> SoftwareStartupState {
    if enabled {
        SoftwareStartupState::Enabled
    } else {
        SoftwareStartupState::Disabled
    }
}

fn validate_startup_plan(plan: &StartupTogglePlanV1) -> Result<(), SoftwareStartupError> {
    if plan.version != STARTUP_TOGGLE_PLAN_VERSION {
        return Err(SoftwareStartupError::UnsupportedPlanVersion(plan.version));
    }
    if !plan.entry_id.starts_with("startup:v1:")
        || plan.entry_id.len() > 96
        || !is_sha256_hex(&plan.expected_entry_identity_sha256)
        || !is_sha256_hex(&plan.expected_approval_identity_sha256)
    {
        return Err(SoftwareStartupError::InvalidPlan);
    }
    Ok(())
}

struct StartupObservation {
    entry: SoftwareStartupEntryV1,
    entry_identity_sha256: String,
    approval_identity_sha256: String,
}

fn observe_matching(
    specs: &StartupSpecs,
    plan: &StartupTogglePlanV1,
) -> Result<StartupObservation, SoftwareStartupError> {
    validate_startup_plan(plan)?;
    let observed = observe_startup_entry(specs, &plan.entry_id)?;
    if observed.entry_identity_sha256 != plan.expected_entry_identity_sha256 {
        return Err(SoftwareStartupError::StartupEntryChanged);
    }
    if observed.approval_identity_sha256 != plan.expected_approval_identity_sha256 {
        return Err(SoftwareStartupError::StartupStateChanged);
    }
    Ok(observed)
}

fn observe_startup_entry(
    specs: &StartupSpecs,
    entry_id: &str,
) -> Result<StartupObservation, SoftwareStartupError> {
    let live = list_with(specs);
    let entry = live
        .entries
        .into_iter()
        .find(|entry| entry.id == entry_id)
        .ok_or_else(|| SoftwareStartupError::UnknownEntry(entry_id.to_string()))?;
    if entry.toggle != SoftwareStartupToggle::Allowed {
        return Err(SoftwareStartupError::RequiresAdministrator(
            entry_id.to_string(),
        ));
    }
    let source = live
        .sources
        .iter()
        .find(|source| source.location == entry.location);
    if !source.is_some_and(|source| source.state == SoftwareSourceState::Available) {
        return Err(SoftwareStartupError::StartupSourceUnavailable);
    }
    let evidence = native::entry_evidence(specs, entry.location, &entry.name)
        .map_err(|_| SoftwareStartupError::StartupIdentityUnavailable)?;
    if entry.state == SoftwareStartupState::Unknown || evidence.approval_unreadable {
        return Err(SoftwareStartupError::StartupIdentityUnavailable);
    }
    Ok(StartupObservation {
        entry_identity_sha256: evidence.entry_identity_sha256,
        approval_identity_sha256: evidence.approval_identity_sha256,
        entry,
    })
}

fn startup_digest(
    plan: &StartupTogglePlanV1,
    before: SoftwareStartupState,
) -> Result<String, SoftwareStartupError> {
    let canonical = serde_json::to_vec(&(
        "devsweep.startup.toggle.preview.v1",
        plan.version,
        &plan.entry_id,
        plan.requested_enabled,
        &plan.expected_entry_identity_sha256,
        &plan.expected_approval_identity_sha256,
        before,
    ))
    .map_err(|_| SoftwareStartupError::InvalidPlan)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
}

fn startup_plan_identity(plan: &StartupTogglePlanV1) -> Result<String, SoftwareStartupError> {
    let canonical = serde_json::to_vec(&(
        "devsweep.startup.toggle.plan.v1",
        plan.version,
        &plan.entry_id,
        plan.requested_enabled,
        &plan.expected_entry_identity_sha256,
        &plan.expected_approval_identity_sha256,
    ))
    .map_err(|_| SoftwareStartupError::InvalidPlan)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
}

fn identity_hash(label: &str, parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(label.as_bytes());
    hasher.update([0]);
    for part in parts {
        hasher.update(part);
        hasher.update([0]);
    }
    hex_digest(hasher.finalize())
}

fn map_startup_audit(error: SoftwareAuditError) -> SoftwareStartupError {
    match error {
        SoftwareAuditError::LocalAppDataUnavailable => SoftwareStartupError::Audit(error),
        _ => SoftwareStartupError::AuditUnavailable,
    }
}

struct EntryEvidence {
    entry_identity_sha256: String,
    approval_identity_sha256: String,
    approval_unreadable: bool,
}

#[cfg(windows)]
mod native {
    use std::fs;

    use sha2::{Digest, Sha256};

    use super::super::arp::native_registry::{ERROR_ACCESS_DENIED, RegKey, predefined_hive};
    use super::*;

    pub(super) fn enumerate(
        specs: &StartupSpecs,
        location: SoftwareStartupLocation,
    ) -> (SoftwareStartupSourceV1, Vec<String>) {
        let source = |state, reason_code: Option<&str>| SoftwareStartupSourceV1 {
            location,
            state,
            reason_code: reason_code.map(str::to_string),
        };
        let Some((hive, view, key)) = specs.run_key(location) else {
            return enumerate_folder(specs, location);
        };
        match RegKey::open(predefined_hive(hive), &key, view) {
            Ok(Some(run)) => match run.value_names() {
                Ok(names) => (
                    source(SoftwareSourceState::Available, None),
                    names.into_iter().filter(|name| !name.is_empty()).collect(),
                ),
                Err(()) => (
                    source(
                        SoftwareSourceState::Partial,
                        Some("registry_enumeration_failed"),
                    ),
                    Vec::new(),
                ),
            },
            Ok(None) => (source(SoftwareSourceState::Available, None), Vec::new()),
            Err(ERROR_ACCESS_DENIED) => (
                source(
                    SoftwareSourceState::Permission,
                    Some("registry_access_denied"),
                ),
                Vec::new(),
            ),
            Err(_) => (
                source(SoftwareSourceState::Partial, Some("registry_open_failed")),
                Vec::new(),
            ),
        }
    }

    fn enumerate_folder(
        specs: &StartupSpecs,
        location: SoftwareStartupLocation,
    ) -> (SoftwareStartupSourceV1, Vec<String>) {
        let source = |state, reason_code: Option<&str>| SoftwareStartupSourceV1 {
            location,
            state,
            reason_code: reason_code.map(str::to_string),
        };
        let Some(folder) = &specs.startup_folder else {
            return (
                source(SoftwareSourceState::Partial, Some("appdata_unavailable")),
                Vec::new(),
            );
        };
        let read = match fs::read_dir(folder) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return (source(SoftwareSourceState::Available, None), Vec::new());
            }
            Err(_) => {
                return (
                    source(SoftwareSourceState::Partial, Some("folder_read_failed")),
                    Vec::new(),
                );
            }
        };
        let mut partial = false;
        let mut names = Vec::new();
        for item in read {
            let Ok(item) = item else {
                partial = true;
                continue;
            };
            let Ok(file_type) = item.file_type() else {
                partial = true;
                continue;
            };
            let Some(name) = item.file_name().to_str().map(str::to_string) else {
                partial = true;
                continue;
            };
            if file_type.is_dir() || name.eq_ignore_ascii_case("desktop.ini") {
                continue;
            }
            names.push(name);
        }
        if partial {
            (
                source(
                    SoftwareSourceState::Partial,
                    Some("folder_entry_unreadable"),
                ),
                names,
            )
        } else {
            (source(SoftwareSourceState::Available, None), names)
        }
    }

    pub(super) fn approval_state(
        specs: &StartupSpecs,
        location: SoftwareStartupLocation,
        name: &str,
    ) -> SoftwareStartupState {
        let (hive, key) = specs.approved_key(location);
        match RegKey::open(predefined_hive(hive), &key, RegistryView::Registry64) {
            Ok(Some(approved)) => match approved.binary(name) {
                Ok(value) => decode_approval(value.as_deref()),
                Err(()) => SoftwareStartupState::Unknown,
            },
            Ok(None) => SoftwareStartupState::Enabled,
            Err(_) => SoftwareStartupState::Unknown,
        }
    }

    /// Writes only the `StartupApproved` value for one current-user entry.
    pub(super) fn write_approval(
        specs: &StartupSpecs,
        location: SoftwareStartupLocation,
        name: &str,
        bytes: &[u8],
    ) -> bool {
        if location.scope() != SoftwareScope::CurrentUser {
            return false;
        }
        let (hive, key) = specs.approved_key(location);
        if hive != RegistryHive::CurrentUser {
            return false;
        }
        RegKey::create_writable(predefined_hive(hive), &key, RegistryView::Registry64)
            .and_then(|approved| approved.set_binary(name, bytes))
            .is_ok()
    }

    pub(super) fn entry_evidence(
        specs: &StartupSpecs,
        location: SoftwareStartupLocation,
        name: &str,
    ) -> Result<EntryEvidence, ()> {
        let entry_identity_sha256 = if specs.run_key(location).is_some() {
            match registry_raw(specs, location, name, false) {
                ObservedBytes::Present { value_type, bytes } => identity_hash(
                    "devsweep.startup.entry.v1",
                    &[
                        location.tag().as_bytes(),
                        name.as_bytes(),
                        &value_type.to_le_bytes(),
                        &bytes,
                    ],
                ),
                ObservedBytes::Missing | ObservedBytes::Unreadable => return Err(()),
            }
        } else {
            let folder = specs.startup_folder.as_deref().ok_or(())?;
            folder_identity(location, &folder.join(name))?
        };
        let (approval_identity_sha256, approval_unreadable) =
            match registry_raw(specs, location, name, true) {
                ObservedBytes::Missing => (
                    identity_hash(
                        "devsweep.startup.approval.v1",
                        &[location.tag().as_bytes(), name.as_bytes(), b"missing"],
                    ),
                    false,
                ),
                ObservedBytes::Present { value_type, bytes }
                    if value_type == super::super::arp::native_registry::REG_BINARY =>
                {
                    (
                        identity_hash(
                            "devsweep.startup.approval.v1",
                            &[
                                location.tag().as_bytes(),
                                name.as_bytes(),
                                b"present",
                                &value_type.to_le_bytes(),
                                &bytes,
                            ],
                        ),
                        false,
                    )
                }
                ObservedBytes::Present { .. } | ObservedBytes::Unreadable => (String::new(), true),
            };
        Ok(EntryEvidence {
            entry_identity_sha256,
            approval_identity_sha256,
            approval_unreadable,
        })
    }

    enum ObservedBytes {
        Missing,
        Present { value_type: u32, bytes: Vec<u8> },
        Unreadable,
    }

    fn registry_raw(
        specs: &StartupSpecs,
        location: SoftwareStartupLocation,
        name: &str,
        approved: bool,
    ) -> ObservedBytes {
        let opened = if approved {
            let (hive, key) = specs.approved_key(location);
            RegKey::open(predefined_hive(hive), &key, RegistryView::Registry64)
        } else {
            let Some((hive, view, key)) = specs.run_key(location) else {
                return ObservedBytes::Unreadable;
            };
            RegKey::open(predefined_hive(hive), &key, view)
        };
        match opened {
            Ok(Some(key)) => match key.value_raw(name) {
                Ok(None) => ObservedBytes::Missing,
                Ok(Some((value_type, bytes))) => ObservedBytes::Present { value_type, bytes },
                Err(()) => ObservedBytes::Unreadable,
            },
            Ok(None) => ObservedBytes::Missing,
            Err(_) => ObservedBytes::Unreadable,
        }
    }

    fn folder_identity(
        location: SoftwareStartupLocation,
        path: &std::path::Path,
    ) -> Result<String, ()> {
        use crate::filesystem::{
            SystemPathReparseProbe, capture_path_identity, inspect_entry_no_follow,
        };

        let entry = inspect_entry_no_follow(path, &SystemPathReparseProbe).map_err(|_| ())?;
        if !entry.is_file || entry.safety != crate::filesystem::PathSafety::Safe {
            return Err(());
        }
        if entry.len > STARTUP_CONTENT_BOUND {
            return Err(());
        }
        let bytes = fs::read(path).map_err(|_| ())?;
        if bytes.len() as u64 != entry.len {
            return Err(());
        }
        let identity = capture_path_identity(path).map_err(|_| ())?;
        let mtime = entry
            .modified
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64);
        let kind = serde_json::to_string(&identity.kind).map_err(|_| ())?;
        Ok(identity_hash(
            "devsweep.startup.entry.v1",
            &[
                location.tag().as_bytes(),
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
                    .as_bytes(),
                kind.as_bytes(),
                identity.value.as_bytes(),
                &entry.len.to_le_bytes(),
                &mtime.unwrap_or(u64::MAX).to_le_bytes(),
                &Sha256::digest(&bytes),
            ],
        ))
    }
}

#[cfg(not(windows))]
mod native {
    use super::*;

    pub(super) fn enumerate(
        _specs: &StartupSpecs,
        location: SoftwareStartupLocation,
    ) -> (SoftwareStartupSourceV1, Vec<String>) {
        (
            SoftwareStartupSourceV1 {
                location,
                state: SoftwareSourceState::Unsupported,
                reason_code: Some("windows_only_source".to_string()),
            },
            Vec::new(),
        )
    }

    pub(super) fn approval_state(
        _specs: &StartupSpecs,
        _location: SoftwareStartupLocation,
        _name: &str,
    ) -> SoftwareStartupState {
        SoftwareStartupState::Unknown
    }

    pub(super) fn write_approval(
        _specs: &StartupSpecs,
        _location: SoftwareStartupLocation,
        _name: &str,
        _bytes: &[u8],
    ) -> bool {
        false
    }

    pub(super) fn entry_evidence(
        _specs: &StartupSpecs,
        _location: SoftwareStartupLocation,
        _name: &str,
    ) -> Result<EntryEvidence, ()> {
        Err(())
    }
}

/// Class stored on an offline startup fixture. It is not inferred from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupFixtureClass {
    Ordinary,
    Security,
    Network,
    DeviceManagement,
}

impl StartupFixtureClass {
    fn tag(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::Security => "security",
            Self::Network => "network",
            Self::DeviceManagement => "device_management",
        }
    }

    fn is_protected(self) -> bool {
        !matches!(self, Self::Ordinary)
    }
}

/// One offline startup hive. Tests use this file instead of host HKCU or HKLM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupFixtureHiveV1 {
    pub version: u32,
    pub entries: Vec<StartupFixtureEntryV1>,
}

/// One fixture startup record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupFixtureEntryV1 {
    pub name: String,
    pub location: SoftwareStartupLocation,
    pub class: StartupFixtureClass,
    pub run_value: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<Vec<u8>>,
}

/// Backup taken after a hive toggle. Restore writes `restore_approval` only
/// when the live approval identity still equals the post-change identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupBackupV1 {
    pub version: u32,
    pub entry_id: String,
    pub post_change_approval_identity_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_approval: Option<Vec<u8>>,
}

/// Saved restore of one offline startup hive entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupRestorePlanV1 {
    pub version: u32,
    pub entry_id: String,
    pub post_change_approval_identity_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_approval: Option<Vec<u8>>,
}

/// Result of one offline hive toggle. `backup` is present only after a write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupHiveExecuteReportV1 {
    pub version: u32,
    pub toggle: StartupToggleStrictReportV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup: Option<StartupBackupV1>,
}

/// Live preview of one hive restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupRestorePreviewV1 {
    pub version: u32,
    pub digest: String,
}

/// Offline hive wire version.
pub const STARTUP_FIXTURE_HIVE_VERSION: u32 = 1;

const HIVE_REG_SZ: u32 = 1;
const HIVE_REG_BINARY: u32 = 3;

/// Observes one fixture entry and returns [`StartupTogglePlanV1`]. No write.
pub fn plan_startup_hive(
    hive_path: &std::path::Path,
    entry_id: &str,
    requested_enabled: bool,
) -> Result<StartupTogglePlanV1, SoftwareStartupError> {
    let hive = read_hive(hive_path)?;
    let entry = hive_entry(&hive, entry_id)?;
    refuse_hive_mutation(&entry)?;
    let observed = hive_observation(&entry)?;
    Ok(StartupTogglePlanV1 {
        version: STARTUP_TOGGLE_PLAN_VERSION,
        entry_id: observed.entry.id,
        requested_enabled,
        expected_entry_identity_sha256: observed.entry_identity_sha256,
        expected_approval_identity_sha256: observed.approval_identity_sha256,
    })
}

/// Re-reads the fixture and returns the live toggle digest.
pub fn preview_startup_hive(
    hive_path: &std::path::Path,
    plan: &StartupTogglePlanV1,
) -> Result<StartupTogglePreviewV1, SoftwareStartupError> {
    let observed = observe_hive_plan(hive_path, plan)?;
    Ok(StartupTogglePreviewV1 {
        version: STARTUP_TOGGLE_PLAN_VERSION,
        digest: startup_digest(plan, observed.entry.state)?,
        after_state: requested_state(plan.requested_enabled),
        before_state: observed.entry.state,
        entry: observed.entry,
    })
}

/// Toggles one ordinary current-user fixture entry. `already_in_state` does not write.
pub fn execute_startup_hive(
    hive_path: &std::path::Path,
    plan: &StartupTogglePlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
) -> Result<StartupHiveExecuteReportV1, SoftwareStartupError> {
    if !confirmed {
        return Err(SoftwareStartupError::ConfirmationRequired);
    }
    validate_startup_plan(plan)?;
    if !is_sha256_digest(expected_preview_digest) {
        return Err(SoftwareStartupError::InvalidPlan);
    }
    let observed = observe_hive_plan(hive_path, plan)?;
    let digest = startup_digest(plan, observed.entry.state)?;
    if digest != expected_preview_digest {
        return Err(SoftwareStartupError::StaleConfirmation);
    }
    if observed.entry.state == requested_state(plan.requested_enabled) {
        return Ok(StartupHiveExecuteReportV1 {
            version: STARTUP_FIXTURE_HIVE_VERSION,
            backup: None,
            toggle: strict_report(
                plan,
                None,
                StartupStrictOutcome::AlreadyInState,
                None,
                observed.entry,
            ),
        });
    }
    let mut hive = read_hive(hive_path)?;
    let index = hive
        .entries
        .iter()
        .position(|entry| startup_id(entry.location, &entry.name) == plan.entry_id)
        .ok_or_else(|| SoftwareStartupError::UnknownEntry(plan.entry_id.clone()))?;
    refuse_hive_mutation(&hive.entries[index])?;
    let restore_approval = hive.entries[index].approval.clone();
    let now = unix_ms(SystemTime::now());
    hive.entries[index].approval = Some(approval_bytes(plan.requested_enabled, now).to_vec());
    write_hive(hive_path, &hive)?;
    let written = hive_observation(&hive.entries[index])?;
    if written.entry.state != requested_state(plan.requested_enabled) {
        return Ok(StartupHiveExecuteReportV1 {
            version: STARTUP_FIXTURE_HIVE_VERSION,
            backup: None,
            toggle: strict_report(
                plan,
                None,
                StartupStrictOutcome::Failed,
                Some("verification_failed".to_string()),
                written.entry,
            ),
        });
    }
    Ok(StartupHiveExecuteReportV1 {
        version: STARTUP_FIXTURE_HIVE_VERSION,
        backup: Some(StartupBackupV1 {
            version: STARTUP_FIXTURE_HIVE_VERSION,
            entry_id: plan.entry_id.clone(),
            post_change_approval_identity_sha256: written.approval_identity_sha256,
            restore_approval,
        }),
        toggle: strict_report(
            plan,
            None,
            StartupStrictOutcome::Succeeded,
            None,
            written.entry,
        ),
    })
}

/// Builds a restore plan from a hive backup. No write.
pub fn plan_startup_restore(
    backup: &StartupBackupV1,
) -> Result<StartupRestorePlanV1, SoftwareStartupError> {
    if backup.version != STARTUP_FIXTURE_HIVE_VERSION
        || !backup.entry_id.starts_with("startup:v1:")
        || !is_sha256_hex(&backup.post_change_approval_identity_sha256)
    {
        return Err(SoftwareStartupError::InvalidPlan);
    }
    Ok(StartupRestorePlanV1 {
        version: STARTUP_FIXTURE_HIVE_VERSION,
        entry_id: backup.entry_id.clone(),
        post_change_approval_identity_sha256: backup.post_change_approval_identity_sha256.clone(),
        restore_approval: backup.restore_approval.clone(),
    })
}

/// Re-reads the fixture approval identity and binds it to `plan`.
pub fn preview_startup_restore(
    hive_path: &std::path::Path,
    plan: &StartupRestorePlanV1,
) -> Result<StartupRestorePreviewV1, SoftwareStartupError> {
    let identity = hive_restore_identity(hive_path, plan)?;
    Ok(StartupRestorePreviewV1 {
        version: STARTUP_FIXTURE_HIVE_VERSION,
        digest: hive_restore_digest(plan, &identity)?,
    })
}

/// Restores the backed-up approval only when the live identity still matches.
/// An external change returns an error and leaves the file bytes untouched.
pub fn execute_startup_restore(
    hive_path: &std::path::Path,
    plan: &StartupRestorePlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
) -> Result<(), SoftwareStartupError> {
    if !confirmed {
        return Err(SoftwareStartupError::ConfirmationRequired);
    }
    if !is_sha256_digest(expected_preview_digest) {
        return Err(SoftwareStartupError::InvalidPlan);
    }
    let identity = hive_restore_identity(hive_path, plan)?;
    if identity != plan.post_change_approval_identity_sha256 {
        return Err(SoftwareStartupError::StartupStateChanged);
    }
    let digest = hive_restore_digest(plan, &identity)?;
    if digest != expected_preview_digest {
        return Err(SoftwareStartupError::StaleConfirmation);
    }
    let mut hive = read_hive(hive_path)?;
    let entry = hive
        .entries
        .iter_mut()
        .find(|entry| startup_id(entry.location, &entry.name) == plan.entry_id)
        .ok_or_else(|| SoftwareStartupError::UnknownEntry(plan.entry_id.clone()))?;
    refuse_hive_mutation(entry)?;
    if hive_approval_identity(entry)? != plan.post_change_approval_identity_sha256 {
        return Err(SoftwareStartupError::StartupStateChanged);
    }
    entry.approval = plan.restore_approval.clone();
    write_hive(hive_path, &hive)
}

fn hive_restore_identity(
    hive_path: &std::path::Path,
    plan: &StartupRestorePlanV1,
) -> Result<String, SoftwareStartupError> {
    if plan.version != STARTUP_FIXTURE_HIVE_VERSION
        || !is_sha256_hex(&plan.post_change_approval_identity_sha256)
    {
        return Err(SoftwareStartupError::InvalidPlan);
    }
    let hive = read_hive(hive_path)?;
    let entry = hive_entry(&hive, &plan.entry_id)?;
    refuse_hive_mutation(&entry)?;
    hive_approval_identity(&entry)
}

fn hive_restore_digest(
    plan: &StartupRestorePlanV1,
    live_identity: &str,
) -> Result<String, SoftwareStartupError> {
    let canonical = serde_json::to_vec(&(
        "devsweep.startup.restore.preview.v1",
        plan.version,
        &plan.entry_id,
        &plan.post_change_approval_identity_sha256,
        &plan.restore_approval,
        live_identity,
    ))
    .map_err(|_| SoftwareStartupError::InvalidPlan)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
}

fn observe_hive_plan(
    hive_path: &std::path::Path,
    plan: &StartupTogglePlanV1,
) -> Result<StartupObservation, SoftwareStartupError> {
    validate_startup_plan(plan)?;
    let hive = read_hive(hive_path)?;
    let entry = hive_entry(&hive, &plan.entry_id)?;
    refuse_hive_mutation(&entry)?;
    let observed = hive_observation(&entry)?;
    if observed.entry_identity_sha256 != plan.expected_entry_identity_sha256 {
        return Err(SoftwareStartupError::StartupEntryChanged);
    }
    if observed.approval_identity_sha256 != plan.expected_approval_identity_sha256 {
        return Err(SoftwareStartupError::StartupStateChanged);
    }
    Ok(observed)
}

fn refuse_hive_mutation(entry: &StartupFixtureEntryV1) -> Result<(), SoftwareStartupError> {
    let id = startup_id(entry.location, &entry.name);
    if entry.class.is_protected() {
        return Err(SoftwareStartupError::Protected(id));
    }
    if entry.location.scope() != SoftwareScope::CurrentUser {
        return Err(SoftwareStartupError::RequiresAdministrator(id));
    }
    Ok(())
}

fn hive_entry(
    hive: &StartupFixtureHiveV1,
    entry_id: &str,
) -> Result<StartupFixtureEntryV1, SoftwareStartupError> {
    hive.entries
        .iter()
        .find(|entry| startup_id(entry.location, &entry.name) == entry_id)
        .cloned()
        .ok_or_else(|| SoftwareStartupError::UnknownEntry(entry_id.to_string()))
}

fn hive_observation(
    fixture: &StartupFixtureEntryV1,
) -> Result<StartupObservation, SoftwareStartupError> {
    let state = decode_approval(fixture.approval.as_deref());
    if state == SoftwareStartupState::Unknown {
        return Err(SoftwareStartupError::StartupIdentityUnavailable);
    }
    let row = entry(fixture.location, fixture.name.clone(), state);
    Ok(StartupObservation {
        entry_identity_sha256: hive_entry_identity(fixture)?,
        approval_identity_sha256: hive_approval_identity(fixture)?,
        entry: row,
    })
}

fn hive_entry_identity(entry: &StartupFixtureEntryV1) -> Result<String, SoftwareStartupError> {
    Ok(identity_hash(
        "devsweep.startup.entry.v1",
        &[
            entry.location.tag().as_bytes(),
            entry.name.as_bytes(),
            entry.class.tag().as_bytes(),
            &HIVE_REG_SZ.to_le_bytes(),
            &entry.run_value,
        ],
    ))
}

fn hive_approval_identity(entry: &StartupFixtureEntryV1) -> Result<String, SoftwareStartupError> {
    Ok(match &entry.approval {
        None => identity_hash(
            "devsweep.startup.approval.v1",
            &[
                entry.location.tag().as_bytes(),
                entry.name.as_bytes(),
                b"missing",
            ],
        ),
        Some(bytes) => identity_hash(
            "devsweep.startup.approval.v1",
            &[
                entry.location.tag().as_bytes(),
                entry.name.as_bytes(),
                b"present",
                &HIVE_REG_BINARY.to_le_bytes(),
                bytes,
            ],
        ),
    })
}

fn read_hive(path: &std::path::Path) -> Result<StartupFixtureHiveV1, SoftwareStartupError> {
    let bytes = std::fs::read(path).map_err(|_| SoftwareStartupError::StartupSourceUnavailable)?;
    let hive: StartupFixtureHiveV1 = serde_json::from_slice(&bytes)
        .map_err(|_| SoftwareStartupError::StartupSourceUnavailable)?;
    if hive.version != STARTUP_FIXTURE_HIVE_VERSION {
        return Err(SoftwareStartupError::UnsupportedPlanVersion(hive.version));
    }
    Ok(hive)
}

fn write_hive(
    path: &std::path::Path,
    hive: &StartupFixtureHiveV1,
) -> Result<(), SoftwareStartupError> {
    let mut bytes =
        serde_json::to_vec_pretty(hive).map_err(|_| SoftwareStartupError::InvalidPlan)?;
    bytes.push(b'\n');
    std::fs::write(path, bytes).map_err(|_| SoftwareStartupError::AuditUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_bytes_decode_even_enabled_odd_disabled_missing_enabled() {
        assert_eq!(decode_approval(None), SoftwareStartupState::Enabled);
        for first in [0x02_u8, 0x06] {
            assert_eq!(
                decode_approval(Some(&[first, 0, 0, 0])),
                SoftwareStartupState::Enabled
            );
        }
        for first in [0x03_u8, 0x07] {
            assert_eq!(
                decode_approval(Some(&[first, 0, 0, 0])),
                SoftwareStartupState::Disabled
            );
        }
        assert_eq!(decode_approval(Some(&[])), SoftwareStartupState::Unknown);
    }

    #[test]
    fn written_layout_matches_task_manager() {
        let enabled = approval_bytes(true, 1_700_000_000_000);
        assert_eq!(enabled, [0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let disabled = approval_bytes(false, 1_700_000_000_000);
        assert_eq!(disabled[0], 0x03);
        assert_eq!(&disabled[1..4], &[0, 0, 0]);
        let filetime = u64::from_le_bytes(disabled[4..].try_into().unwrap());
        assert_eq!(filetime, 1_700_000_000_000 * 10_000 + FILETIME_UNIX_EPOCH);
        assert_eq!(
            decode_approval(Some(&enabled)),
            SoftwareStartupState::Enabled
        );
        assert_eq!(
            decode_approval(Some(&disabled)),
            SoftwareStartupState::Disabled
        );
    }

    #[test]
    fn machine_locations_are_view_only_and_ids_are_location_scoped() {
        let user = entry(
            SoftwareStartupLocation::CurrentUserRun,
            "Tool".into(),
            SoftwareStartupState::Enabled,
        );
        let machine = entry(
            SoftwareStartupLocation::MachineRun,
            "Tool".into(),
            SoftwareStartupState::Enabled,
        );
        assert_eq!(user.toggle, SoftwareStartupToggle::Allowed);
        assert_eq!(machine.toggle, SoftwareStartupToggle::RequiresAdministrator);
        assert_eq!(machine.scope, SoftwareScope::Machine);
        assert_ne!(user.id, machine.id);
        let encoded = serde_json::to_string(&user).unwrap();
        for forbidden in ["command", "path", "argv", "program"] {
            assert!(!encoded.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn unconfirmed_toggle_is_rejected_before_any_read_or_write() {
        let temp = tempfile::TempDir::new().unwrap();
        let error = set_with(
            &StartupSpecs::production(),
            &temp.path().join("software.jsonl"),
            "startup:v1:any",
            false,
            false,
        )
        .expect_err("confirmation is required");
        assert!(matches!(error, SoftwareStartupError::ConfirmationRequired));
        assert!(!temp.path().join("software.jsonl").exists());
    }

    #[cfg(windows)]
    mod registry {
        use super::super::super::arp::native_registry::{RegKey, delete_tree, predefined_hive};
        use super::*;

        /// Temporary HKCU test tree. It never touches the real Run keys.
        struct TestTree {
            root: String,
        }

        impl TestTree {
            fn new() -> Self {
                let nonce = hex_digest(Sha256::digest(format!(
                    "{}-{:?}-{}",
                    std::process::id(),
                    std::thread::current().id(),
                    unix_ms(SystemTime::now())
                )));
                Self {
                    root: format!(r"Software\DevSweepTest-startup-{nonce}"),
                }
            }

            fn specs(&self, folder: PathBuf) -> StartupSpecs {
                StartupSpecs {
                    user_hive: RegistryHive::CurrentUser,
                    user_base: format!(r"{}\user", self.root),
                    machine_hive: RegistryHive::CurrentUser,
                    machine_base: format!(r"{}\machine", self.root),
                    machine_run32_base: format!(r"{}\machine32", self.root),
                    startup_folder: Some(folder),
                }
            }

            fn set_string(&self, key: &str, name: &str) {
                // A REG_BINARY placeholder is enough: Run value data is never read.
                RegKey::create_writable(
                    predefined_hive(RegistryHive::CurrentUser),
                    &format!(r"{}\{key}", self.root),
                    RegistryView::Registry64,
                )
                .unwrap()
                .set_binary(name, b"x")
                .unwrap();
            }

            fn set_binary(&self, key: &str, name: &str, bytes: &[u8]) {
                RegKey::create_writable(
                    predefined_hive(RegistryHive::CurrentUser),
                    &format!(r"{}\{key}", self.root),
                    RegistryView::Registry64,
                )
                .unwrap()
                .set_binary(name, bytes)
                .unwrap();
            }

            fn values(&self, key: &str) -> Vec<String> {
                RegKey::open(
                    predefined_hive(RegistryHive::CurrentUser),
                    &format!(r"{}\{key}", self.root),
                    RegistryView::Registry64,
                )
                .unwrap()
                .map(|key| key.value_names().unwrap())
                .unwrap_or_default()
            }

            fn set_typed(&self, key: &str, name: &str, value_type: u32, bytes: &[u8]) {
                RegKey::create_writable(
                    predefined_hive(RegistryHive::CurrentUser),
                    &format!(r"{}\{key}", self.root),
                    RegistryView::Registry64,
                )
                .unwrap()
                .set_typed(name, value_type, bytes)
                .unwrap();
            }

            fn binary(&self, key: &str, name: &str) -> Option<Vec<u8>> {
                RegKey::open(
                    predefined_hive(RegistryHive::CurrentUser),
                    &format!(r"{}\{key}", self.root),
                    RegistryView::Registry64,
                )
                .unwrap()
                .and_then(|key| key.binary(name).unwrap())
            }
        }

        impl Drop for TestTree {
            fn drop(&mut self) {
                let _ = delete_tree(predefined_hive(RegistryHive::CurrentUser), &self.root);
            }
        }

        #[test]
        fn temporary_key_lists_decodes_toggles_and_audits_only_approval_values() {
            let tree = TestTree::new();
            let folder = tempfile::TempDir::new().unwrap();
            std::fs::write(folder.path().join("Launcher.lnk"), b"shortcut").unwrap();
            std::fs::write(folder.path().join("desktop.ini"), b"ini").unwrap();
            tree.set_string(r"user\Run", "UserTool");
            tree.set_string(r"user\Run", "PausedTool");
            tree.set_binary(
                r"user\Explorer\StartupApproved\Run",
                "PausedTool",
                &[0x03, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8],
            );
            tree.set_string(r"machine\Run", "MachineTool");
            tree.set_binary(
                r"machine\Explorer\StartupApproved\Run",
                "MachineTool",
                &[0x07, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            );
            tree.set_string(r"machine32\Run", "LegacyTool");
            let specs = tree.specs(folder.path().to_path_buf());

            let listed = list_with(&specs);
            assert!(
                listed
                    .sources
                    .iter()
                    .all(|source| source.state == SoftwareSourceState::Available)
            );
            let find = |name: &str| {
                listed
                    .entries
                    .iter()
                    .find(|entry| entry.name == name)
                    .cloned()
                    .unwrap_or_else(|| panic!("{name} listed"))
            };
            assert_eq!(find("UserTool").state, SoftwareStartupState::Enabled);
            assert_eq!(find("PausedTool").state, SoftwareStartupState::Disabled);
            assert_eq!(
                find("Launcher.lnk").location,
                SoftwareStartupLocation::CurrentUserStartupFolder
            );
            assert!(
                listed
                    .entries
                    .iter()
                    .all(|entry| entry.name != "desktop.ini")
            );
            let machine = find("MachineTool");
            assert_eq!(machine.state, SoftwareStartupState::Disabled);
            assert_eq!(machine.toggle, SoftwareStartupToggle::RequiresAdministrator);
            let legacy = find("LegacyTool");
            assert_eq!(legacy.location, SoftwareStartupLocation::MachineRun32);
            assert_eq!(legacy.toggle, SoftwareStartupToggle::RequiresAdministrator);

            let audit = tempfile::TempDir::new().unwrap();
            let audit_path = audit.path().join("software.jsonl");
            let run_before = tree.values(r"user\Run");

            let report = set_with(&specs, &audit_path, &find("UserTool").id, false, true).unwrap();
            assert_eq!(report.outcome, SoftwareSupportOutcomeCode::Succeeded);
            assert_eq!(report.entry.state, SoftwareStartupState::Disabled);
            let written = tree
                .binary(r"user\Explorer\StartupApproved\Run", "UserTool")
                .unwrap();
            assert_eq!(written.len(), 12);
            assert_eq!(written[0], 0x03);

            let report =
                set_with(&specs, &audit_path, &find("Launcher.lnk").id, false, true).unwrap();
            assert_eq!(report.entry.state, SoftwareStartupState::Disabled);
            assert!(
                tree.binary(
                    r"user\Explorer\StartupApproved\StartupFolder",
                    "Launcher.lnk"
                )
                .is_some()
            );

            let report = set_with(&specs, &audit_path, &find("UserTool").id, true, true).unwrap();
            assert_eq!(report.entry.state, SoftwareStartupState::Enabled);
            assert_eq!(
                tree.binary(r"user\Explorer\StartupApproved\Run", "UserTool")
                    .unwrap(),
                vec![0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            );

            // The Run key and the shortcut are unchanged: only approval values move.
            assert_eq!(tree.values(r"user\Run"), run_before);
            assert!(folder.path().join("Launcher.lnk").exists());

            // Machine entries are refused before the journal or registry changes.
            let machine_before =
                tree.binary(r"machine\Explorer\StartupApproved\Run", "MachineTool");
            let error = set_with(&specs, &audit_path, &machine.id, true, true).unwrap_err();
            assert!(matches!(
                error,
                SoftwareStartupError::RequiresAdministrator(_)
            ));
            assert_eq!(
                tree.binary(r"machine\Explorer\StartupApproved\Run", "MachineTool"),
                machine_before
            );
            let error =
                set_with(&specs, &audit_path, "startup:v1:missing", true, true).unwrap_err();
            assert!(matches!(error, SoftwareStartupError::UnknownEntry(_)));

            let journal = std::fs::read_to_string(&audit_path).unwrap();
            let records = journal.lines().collect::<Vec<_>>();
            assert_eq!(records.len(), 3);
            for line in records {
                let value: serde_json::Value = serde_json::from_str(line).unwrap();
                assert_eq!(value["record_kind"], "startup_toggled");
                assert_eq!(value["domain"], "software");
                assert_eq!(value["outcome_code"], "succeeded");
            }
            assert!(!journal.contains("\"path\""));
            assert!(!journal.contains("command"));
        }

        #[test]
        fn strict_toggle_binds_identity_and_skips_an_unchanged_state() {
            let tree = TestTree::new();
            let folder = tempfile::TempDir::new().unwrap();
            std::fs::write(folder.path().join("Launcher.lnk"), b"shortcut").unwrap();
            tree.set_string(r"user\Run", "UserTool");
            tree.set_string(r"machine\Run", "MachineTool");
            let specs = tree.specs(folder.path().to_path_buf());
            let listed = list_with(&specs);
            let user = listed
                .entries
                .iter()
                .find(|entry| entry.name == "UserTool")
                .unwrap();
            let launcher = listed
                .entries
                .iter()
                .find(|entry| entry.name == "Launcher.lnk")
                .unwrap();
            let audit = tempfile::TempDir::new().unwrap();
            let audit_path = audit.path().join("software.jsonl");

            let enabled = plan_startup_with(&specs, &user.id, true).unwrap();
            let preview = preview_startup_with(&specs, &enabled).unwrap();
            let report = execute_startup_with(
                &specs,
                &audit_path,
                &enabled,
                &preview.digest,
                true,
                SupportAppendFault::NONE,
                None,
            )
            .unwrap();
            assert_eq!(report.outcome, StartupStrictOutcome::AlreadyInState);
            assert!(report.operation_id.is_none());
            assert!(
                tree.binary(r"user\Explorer\StartupApproved\Run", "UserTool")
                    .is_none()
            );
            assert!(!audit_path.exists());

            let plan = plan_startup_with(&specs, &user.id, false).unwrap();
            let preview = preview_startup_with(&specs, &plan).unwrap();
            let encoded = serde_json::to_string(&plan).unwrap();
            assert!(!encoded.contains("command"));
            assert!(!encoded.contains("\\\\"));
            let unconfirmed = execute_startup_with(
                &specs,
                &audit_path,
                &plan,
                &preview.digest,
                false,
                SupportAppendFault::NONE,
                None,
            )
            .unwrap_err();
            assert!(matches!(
                unconfirmed,
                SoftwareStartupError::ConfirmationRequired
            ));
            assert!(!audit_path.exists());

            let started = execute_startup_with(
                &specs,
                &audit_path,
                &plan,
                &preview.digest,
                true,
                SupportAppendFault {
                    fail_started: true,
                    fail_terminal: false,
                },
                None,
            )
            .unwrap_err();
            assert!(matches!(started, SoftwareStartupError::AuditUnavailable));
            assert!(
                tree.binary(r"user\Explorer\StartupApproved\Run", "UserTool")
                    .is_none()
            );

            let report = execute_startup_with(
                &specs,
                &audit_path,
                &plan,
                &preview.digest,
                true,
                SupportAppendFault::NONE,
                Some(SoftwareStartupState::Enabled),
            )
            .unwrap();
            assert_eq!(report.outcome, StartupStrictOutcome::Failed);
            assert_eq!(report.error_code.as_deref(), Some("verification_failed"));
            let written = tree
                .binary(r"user\Explorer\StartupApproved\Run", "UserTool")
                .unwrap();
            assert_eq!(written[0], 0x03);

            tree.set_binary(
                r"user\Explorer\StartupApproved\Run",
                "UserTool",
                &[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            );
            let drifted = plan_startup_with(&specs, &user.id, false).unwrap();
            tree.set_binary(
                r"user\Explorer\StartupApproved\Run",
                "UserTool",
                &[0x02, 0, 0, 0, 9, 9, 9, 9, 9, 9, 9, 9],
            );
            let error = preview_startup_with(&specs, &drifted).unwrap_err();
            assert!(matches!(error, SoftwareStartupError::StartupStateChanged));

            tree.set_typed(r"user\Explorer\StartupApproved\Run", "UserTool", 1, b"text");
            let error = plan_startup_with(&specs, &user.id, false).unwrap_err();
            assert!(matches!(
                error,
                SoftwareStartupError::StartupIdentityUnavailable
            ));

            let folder_plan = plan_startup_with(&specs, &launcher.id, false).unwrap();
            std::fs::write(folder.path().join("Launcher.lnk"), b"replaced").unwrap();
            let error = preview_startup_with(&specs, &folder_plan).unwrap_err();
            assert!(matches!(error, SoftwareStartupError::StartupEntryChanged));

            let machine = listed
                .entries
                .iter()
                .find(|entry| entry.location == SoftwareStartupLocation::MachineRun);
            if let Some(machine) = machine {
                let error = plan_startup_with(&specs, &machine.id, false).unwrap_err();
                assert!(matches!(
                    error,
                    SoftwareStartupError::RequiresAdministrator(_)
                ));
            }
            let support = std::fs::read_to_string(audit.path().join("software-support.jsonl"))
                .unwrap_or_default();
            assert!(!support.contains("command"));
            if audit_path.exists() {
                let v1 = std::fs::read_to_string(&audit_path).unwrap();
                assert!(!v1.contains("startup_toggled"));
            }
        }
    }

    #[test]
    fn ac2_offline_hive_restore_refuses_external_change_and_skips_protected_classes() {
        let temp = tempfile::TempDir::new().unwrap();
        let hive_path = temp.path().join("hive.json");
        let ordinary = StartupFixtureEntryV1 {
            name: "Contoso".to_string(),
            location: SoftwareStartupLocation::CurrentUserRun,
            class: StartupFixtureClass::Ordinary,
            run_value: b"Contoso".to_vec(),
            approval: None,
        };
        let mut hive = StartupFixtureHiveV1 {
            version: STARTUP_FIXTURE_HIVE_VERSION,
            entries: vec![ordinary],
        };
        std::fs::write(&hive_path, serde_json::to_vec_pretty(&hive).unwrap()).unwrap();
        let before = std::fs::read(&hive_path).unwrap();
        let id = startup_id(SoftwareStartupLocation::CurrentUserRun, "Contoso");
        let plan = plan_startup_hive(&hive_path, &id, true).unwrap();
        let preview = preview_startup_hive(&hive_path, &plan).unwrap();
        let report = execute_startup_hive(&hive_path, &plan, &preview.digest, true).unwrap();
        assert_eq!(report.toggle.outcome, StartupStrictOutcome::AlreadyInState);
        assert!(report.backup.is_none());
        assert_eq!(std::fs::read(&hive_path).unwrap(), before);

        hive.entries[0].location = SoftwareStartupLocation::MachineRun;
        std::fs::write(&hive_path, serde_json::to_vec_pretty(&hive).unwrap()).unwrap();
        let machine_bytes = std::fs::read(&hive_path).unwrap();
        let machine_id = startup_id(SoftwareStartupLocation::MachineRun, "Contoso");
        let error = plan_startup_hive(&hive_path, &machine_id, false).unwrap_err();
        assert_eq!(error.code(), "requires_administrator");
        assert_eq!(std::fs::read(&hive_path).unwrap(), machine_bytes);

        hive.entries[0].location = SoftwareStartupLocation::CurrentUserRun;
        for class in [
            StartupFixtureClass::Security,
            StartupFixtureClass::Network,
            StartupFixtureClass::DeviceManagement,
        ] {
            hive.entries[0].class = class;
            std::fs::write(&hive_path, serde_json::to_vec_pretty(&hive).unwrap()).unwrap();
            let protected_bytes = std::fs::read(&hive_path).unwrap();
            let error = plan_startup_hive(&hive_path, &id, false).unwrap_err();
            assert_eq!(error.code(), "protected");
            assert_eq!(std::fs::read(&hive_path).unwrap(), protected_bytes);
        }

        hive.entries[0].class = StartupFixtureClass::Ordinary;
        std::fs::write(&hive_path, serde_json::to_vec_pretty(&hive).unwrap()).unwrap();
        let plan = plan_startup_hive(&hive_path, &id, false).unwrap();
        let preview = preview_startup_hive(&hive_path, &plan).unwrap();
        let report = execute_startup_hive(&hive_path, &plan, &preview.digest, true).unwrap();
        assert_eq!(report.toggle.outcome, StartupStrictOutcome::Succeeded);
        let backup = report.backup.expect("toggle backup");
        let restore = plan_startup_restore(&backup).unwrap();
        let mut changed: StartupFixtureHiveV1 =
            serde_json::from_slice(&std::fs::read(&hive_path).unwrap()).unwrap();
        changed.entries[0].approval = Some(vec![9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let mut changed_bytes = serde_json::to_vec_pretty(&changed).unwrap();
        changed_bytes.push(b'\n');
        std::fs::write(&hive_path, &changed_bytes).unwrap();
        let error = execute_startup_restore(
            &hive_path,
            &restore,
            &format!("sha256:{}", "a".repeat(64)),
            true,
        )
        .unwrap_err();
        assert_eq!(error.code(), "startup_state_changed");
        assert_eq!(std::fs::read(&hive_path).unwrap(), changed_bytes);
    }
}
