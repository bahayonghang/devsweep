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

use super::{
    RegistryHive, RegistryView, SoftwareAuditError, SoftwareScope, SoftwareSourceState,
    audit::{SoftwareAuditJournal, SoftwareSupportAuditRecordV1},
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
    /// The entry id is not in the live startup inventory.
    UnknownEntry(String),
    RequiresAdministrator(String),
    Unsupported,
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
            Self::Audit(error) => error.fmt(formatter),
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
    let digest = Sha256::digest(format!("devsweep.startup.v1\0{}\0{name}", location.tag()));
    format!("startup:v1:{digest:x}")
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

#[cfg(windows)]
mod native {
    use std::fs;

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
                let nonce = Sha256::digest(format!(
                    "{}-{:?}-{}",
                    std::process::id(),
                    std::thread::current().id(),
                    unix_ms(SystemTime::now())
                ));
                Self {
                    root: format!(r"Software\DevSweepTest-startup-{nonce:x}"),
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
    }
}
