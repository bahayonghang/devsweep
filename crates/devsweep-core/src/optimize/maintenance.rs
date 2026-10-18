//! C06 maintenance rows: fixed program/argv, injected prechecks, and read-only
//! disk health.
//!
//! Dispatch never builds a shell command string. Cancel and timeout leave an
//! already-spawned repair running and report the outcome unknown. Tests pass
//! a command double; this module does not start `defrag.exe`, `dism.exe`, or
//! `w32tm.exe` by itself.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::audit::OptimizeAuditErrorCode;
use super::catalogue::{
    MAINTENANCE_CLOCK_PROBE_ID, MAINTENANCE_CLOCK_SYNC_ID, MAINTENANCE_DEAD_STARTUP_ID,
    MAINTENANCE_DISK_HEALTH_ID, MAINTENANCE_IMAGE_REPAIR_ID, MAINTENANCE_IMAGE_SCAN_ID,
    MAINTENANCE_RETRIM_ID, MaintenanceActionClass, MaintenanceCatalogueEntryV1,
};
use super::execution::AdapterEvidence;
use super::plan::{OptimizePlanError, PrecheckReason, ResolvedMaintenanceAction};
#[cfg(windows)]
use crate::execution::{SystemTrashRunner, TrashRunner};
use crate::process::{CancelObserver, FlagCancelObserver};

/// Seven days. Retrim cooldown is read from the optimize audit, not a new file.
pub(super) const RETRIM_COOLDOWN_MS: u64 = 7 * 24 * 60 * 60 * 1000;

const CLOCK_PROBE_ARGV: [&str; 5] = [
    "/stripchart",
    "/computer:time.windows.com",
    "/samples:3",
    "/period:1",
    "/dataonly",
];
const CLOCK_SYNC_ARGV: [&str; 1] = ["/resync"];
const IMAGE_SCAN_ARGV: [&str; 3] = ["/Online", "/Cleanup-Image", "/ScanHealth"];
const IMAGE_REPAIR_ARGV: [&str; 3] = ["/Online", "/Cleanup-Image", "/RestoreHealth"];

/// One startup entry the dead-startup operation may delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeadStartupDeletion {
    pub name: String,
    pub kind: DeadStartupKind,
    pub target: PathBuf,
    pub link_path: Option<PathBuf>,
}

/// Run value or Startup-folder link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeadStartupKind {
    RunValue,
    StartupLink,
}

/// The one removal path a dead-startup entry takes. Neither path is an
/// irreversible in-place unlink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeadStartupRemoval {
    /// `RegDeleteValueW`; the `.reg` backup restores the value.
    RegistryValue,
    /// `SystemTrashRunner`; the Recycle Bin restores the shortcut.
    RecycleBin,
}

impl DeadStartupDeletion {
    /// A Startup shortcut is always recycled, never deleted in place.
    fn removal(&self) -> DeadStartupRemoval {
        match self.kind {
            DeadStartupKind::RunValue => DeadStartupRemoval::RegistryValue,
            DeadStartupKind::StartupLink => DeadStartupRemoval::RecycleBin,
        }
    }
}

/// One observed startup row supplied by a live reader or a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartupFact {
    pub name: String,
    pub kind: DeadStartupKind,
    pub target: PathBuf,
    pub target_exists: bool,
    pub link_path: Option<PathBuf>,
    pub reg_line: String,
}

/// Volume fact for `maintenance.retrim`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum VolumeFact {
    ReadySsd { volume: String },
    NotSsd,
    DiskProblem,
    None,
}

/// Injected or live precheck facts. Resolution does not launch a program.
#[derive(Debug, Clone)]
pub(super) struct MaintenanceFacts {
    pub uac_declined: bool,
    pub battery_low: bool,
    pub reboot_pending: bool,
    pub volume: VolumeFact,
    pub defrag: Option<PathBuf>,
    pub w32tm: Option<PathBuf>,
    pub dism: Option<PathBuf>,
    pub image_repairable: bool,
    pub retrim_cooldown: bool,
    pub startup: Vec<StartupFact>,
    pub backup_dir: PathBuf,
    pub backup_fails: bool,
}

/// Closed disk-health status. Live observation uses [`DiskHealthStatus::Unknown`]
/// unless a probe proves a more specific value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskHealthStatus {
    Healthy,
    PredictFailure,
    Unavailable,
    Unknown,
}

impl DiskHealthStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::PredictFailure => "predict_failure",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
        }
    }
}

/// One read-only disk row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskHealthDeviceV1 {
    pub device_id: String,
    pub health_status: DiskHealthStatus,
}

/// Read-only `maintenance.disk_health` report. It has no executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskHealthReportV1 {
    pub catalogue_id: &'static str,
    pub action_class: MaintenanceActionClass,
    pub devices: Vec<DiskHealthDeviceV1>,
}

/// Live read-only disk health. This function does not launch a repair.
#[must_use]
pub fn observe_disk_health() -> DiskHealthReportV1 {
    DiskHealthReportV1 {
        catalogue_id: MAINTENANCE_DISK_HEALTH_ID,
        action_class: MaintenanceActionClass::Observe,
        devices: live_disk_devices(),
    }
}

pub(super) fn native_timeout(operation_id: &str) -> Duration {
    match operation_id {
        MAINTENANCE_CLOCK_PROBE_ID => Duration::from_secs(30),
        MAINTENANCE_CLOCK_SYNC_ID => Duration::from_secs(60),
        MAINTENANCE_RETRIM_ID => Duration::from_secs(15 * 60),
        MAINTENANCE_IMAGE_SCAN_ID => Duration::from_secs(30 * 60),
        MAINTENANCE_IMAGE_REPAIR_ID => Duration::from_secs(60 * 60),
        _ => Duration::from_secs(60),
    }
}

/// Resolves one V2 execute row from facts. No process is spawned.
pub(super) fn resolve_with_facts(
    entry: &MaintenanceCatalogueEntryV1,
    facts: &MaintenanceFacts,
) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
    match entry.id {
        MAINTENANCE_RETRIM_ID => resolve_retrim(facts),
        MAINTENANCE_CLOCK_PROBE_ID => resolve_clock(facts, false),
        MAINTENANCE_CLOCK_SYNC_ID => resolve_clock(facts, true),
        MAINTENANCE_IMAGE_SCAN_ID => resolve_image(facts, false),
        MAINTENANCE_IMAGE_REPAIR_ID => resolve_image(facts, true),
        MAINTENANCE_DEAD_STARTUP_ID => resolve_dead_startup(facts),
        MAINTENANCE_DISK_HEALTH_ID => Err(OptimizePlanError::ObserveNotPlanable(entry.id.into())),
        id => Err(OptimizePlanError::UnknownOperation(id.to_string())),
    }
}

fn refuse(operation_id: &str, reason: PrecheckReason) -> OptimizePlanError {
    OptimizePlanError::PrecheckRefused {
        operation_id: operation_id.to_string(),
        reason,
    }
}

fn resolve_retrim(
    facts: &MaintenanceFacts,
) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
    if facts.uac_declined {
        return Err(refuse(MAINTENANCE_RETRIM_ID, PrecheckReason::UacDeclined));
    }
    if facts.battery_low {
        return Err(refuse(MAINTENANCE_RETRIM_ID, PrecheckReason::LowBattery));
    }
    let volume = match &facts.volume {
        VolumeFact::ReadySsd { volume } if valid_volume(volume) => volume.clone(),
        VolumeFact::DiskProblem => {
            return Err(refuse(MAINTENANCE_RETRIM_ID, PrecheckReason::DiskProblem));
        }
        VolumeFact::ReadySsd { .. } | VolumeFact::NotSsd | VolumeFact::None => {
            return Err(refuse(MAINTENANCE_RETRIM_ID, PrecheckReason::NotSsd));
        }
    };
    if facts.reboot_pending {
        return Err(refuse(MAINTENANCE_RETRIM_ID, PrecheckReason::RebootPending));
    }
    if facts.retrim_cooldown {
        return Err(refuse(MAINTENANCE_RETRIM_ID, PrecheckReason::Cooldown));
    }
    let Some(program) = facts.defrag.clone() else {
        return Err(refuse(
            MAINTENANCE_RETRIM_ID,
            PrecheckReason::ProgramMissing,
        ));
    };
    Ok(native(program, vec!["/L".to_string(), volume]))
}

fn resolve_clock(
    facts: &MaintenanceFacts,
    sync: bool,
) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
    let operation_id = if sync {
        MAINTENANCE_CLOCK_SYNC_ID
    } else {
        MAINTENANCE_CLOCK_PROBE_ID
    };
    if sync && facts.uac_declined {
        return Err(refuse(operation_id, PrecheckReason::UacDeclined));
    }
    if sync && facts.reboot_pending {
        return Err(refuse(operation_id, PrecheckReason::RebootPending));
    }
    let Some(program) = facts.w32tm.clone() else {
        return Err(refuse(operation_id, PrecheckReason::ProgramMissing));
    };
    let argv = if sync {
        owned(&CLOCK_SYNC_ARGV)
    } else {
        owned(&CLOCK_PROBE_ARGV)
    };
    Ok(native(program, argv))
}

fn resolve_image(
    facts: &MaintenanceFacts,
    repair: bool,
) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
    let operation_id = if repair {
        MAINTENANCE_IMAGE_REPAIR_ID
    } else {
        MAINTENANCE_IMAGE_SCAN_ID
    };
    if facts.uac_declined {
        return Err(refuse(operation_id, PrecheckReason::UacDeclined));
    }
    if repair && !facts.image_repairable {
        return Err(refuse(operation_id, PrecheckReason::ImageNotRepairable));
    }
    if facts.reboot_pending {
        return Err(refuse(operation_id, PrecheckReason::RebootPending));
    }
    let Some(program) = facts.dism.clone() else {
        return Err(refuse(operation_id, PrecheckReason::ProgramMissing));
    };
    let argv = if repair {
        owned(&IMAGE_REPAIR_ARGV)
    } else {
        owned(&IMAGE_SCAN_ARGV)
    };
    Ok(native(program, argv))
}

fn resolve_dead_startup(
    facts: &MaintenanceFacts,
) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
    if facts.uac_declined {
        return Err(refuse(
            MAINTENANCE_DEAD_STARTUP_ID,
            PrecheckReason::UacDeclined,
        ));
    }
    if facts.backup_fails {
        return Err(refuse(
            MAINTENANCE_DEAD_STARTUP_ID,
            PrecheckReason::BackupWriteFailed,
        ));
    }
    let mut deletions = Vec::new();
    let mut skip_names = Vec::new();
    let mut body = String::from("Windows Registry Editor Version 5.00\r\n\r\n");
    body.push_str("[HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run]\r\n");
    for entry in &facts.startup {
        if entry.target_exists || entry.target.as_os_str().is_empty() {
            skip_names.push(entry.name.clone());
            continue;
        }
        deletions.push(DeadStartupDeletion {
            name: entry.name.clone(),
            kind: entry.kind,
            target: entry.target.clone(),
            link_path: entry.link_path.clone(),
        });
        if !entry.reg_line.is_empty() {
            body.push_str(&entry.reg_line);
            if !entry.reg_line.ends_with("\r\n") {
                body.push_str("\r\n");
            }
        }
        // A Startup shortcut has no Run value line to restore, so record the
        // moved path as a `;` comment. The `.reg` stays a valid registry file
        // and still names every link the operation takes out of the folder.
        if entry.kind == DeadStartupKind::StartupLink
            && let Some(link) = &entry.link_path
        {
            body.push_str(&shortcut_comment(link));
        }
    }
    Ok(ResolvedMaintenanceAction::DeadStartup {
        backup_path: facts.backup_dir.join("dead-startup.reg"),
        backup_body: body,
        deletions,
        skip_names,
    })
}

fn native(program: PathBuf, argv: Vec<String>) -> ResolvedMaintenanceAction {
    ResolvedMaintenanceAction::NativeCommand { program, argv }
}

fn owned(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|arg| (*arg).to_string()).collect()
}

/// One `;` comment line naming a Startup shortcut that moved to the Recycle
/// Bin. `.reg` comments start with `;`, so the backup stays importable.
fn shortcut_comment(link: &Path) -> String {
    format!("; shortcut moved to Recycle Bin: {}\r\n", link.display())
}

fn valid_volume(volume: &str) -> bool {
    let mut chars = volume.chars();
    let Some(letter) = chars.next() else {
        return false;
    };
    letter.is_ascii_alphabetic() && chars.as_str() == ":"
}

/// Runs one already-resolved program. A shell basename is refused. Cancel and
/// timeout do not kill the child.
pub(super) fn dispatch_native(
    program: &Path,
    argv: &[String],
    cancel: Option<&Arc<FlagCancelObserver>>,
    timeout: Duration,
) -> AdapterEvidence {
    if is_shell(program) {
        return AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterDispatchFailed);
    }
    let mut command = Command::new(program);
    command.args(argv);
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterDispatchFailed);
        }
        Err(_) => {
            return AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterDispatchFailed);
        }
    };
    match wait_spawned(&mut child, cancel, timeout) {
        MaintenanceWait::Exited(status) if status.success() => AdapterEvidence::success(),
        MaintenanceWait::Exited(_) => {
            AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterOperationFailed)
        }
        MaintenanceWait::LeaveRunning => {
            AdapterEvidence::unfinished(OptimizeAuditErrorCode::AdapterCanceledAfterDispatch)
        }
    }
}

/// Result of waiting on a maintenance child. [`MaintenanceWait::LeaveRunning`]
/// does not kill the process.
#[derive(Debug)]
pub(super) enum MaintenanceWait {
    Exited(std::process::ExitStatus),
    LeaveRunning,
}

pub(super) fn wait_spawned(
    child: &mut Child,
    cancel: Option<&Arc<FlagCancelObserver>>,
    timeout: Duration,
) -> MaintenanceWait {
    let started = std::time::Instant::now();
    loop {
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) || started.elapsed() >= timeout {
            return MaintenanceWait::LeaveRunning;
        }
        match child.try_wait() {
            Ok(Some(status)) => return MaintenanceWait::Exited(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return MaintenanceWait::LeaveRunning,
        }
    }
}

fn is_shell(program: &Path) -> bool {
    program
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case("cmd.exe")
                || name.eq_ignore_ascii_case("powershell.exe")
                || name.eq_ignore_ascii_case("pwsh.exe")
        })
}

pub(super) trait DeadStartupStore {
    fn write_backup(&self, path: &Path, body: &str) -> bool;
    fn target_exists(&self, deletion: &DeadStartupDeletion) -> bool;
    fn delete(&self, deletion: &DeadStartupDeletion) -> bool;
}

/// Writes the backup, then deletes only entries whose targets are still gone.
pub(super) fn dispatch_dead_startup(
    action: &ResolvedMaintenanceAction,
    store: &dyn DeadStartupStore,
) -> AdapterEvidence {
    let ResolvedMaintenanceAction::DeadStartup {
        backup_path,
        backup_body,
        deletions,
        ..
    } = action
    else {
        return AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterDispatchFailed);
    };
    if !store.write_backup(backup_path, backup_body) || !backup_path.is_file() {
        return AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterOperationFailed);
    }
    for deletion in deletions {
        if store.target_exists(deletion) {
            continue;
        }
        if !store.delete(deletion) {
            return AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterOperationFailed);
        }
    }
    AdapterEvidence::success()
}

/// Live startup store. Tests use an in-memory store instead.
#[derive(Debug, Default)]
pub(super) struct LiveDeadStartupStore;

impl DeadStartupStore for LiveDeadStartupStore {
    fn write_backup(&self, path: &Path, body: &str) -> bool {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && std::fs::create_dir_all(parent).is_err()
        {
            return false;
        }
        std::fs::write(path, body).is_ok()
    }

    fn target_exists(&self, deletion: &DeadStartupDeletion) -> bool {
        deletion.target.as_os_str().is_empty() || deletion.target.exists()
    }

    fn delete(&self, deletion: &DeadStartupDeletion) -> bool {
        match deletion.removal() {
            DeadStartupRemoval::RegistryValue => delete_run_value(&deletion.name),
            DeadStartupRemoval::RecycleBin => delete_startup_link(deletion.link_path.as_deref()),
        }
    }
}

pub(super) fn live_facts() -> Result<MaintenanceFacts, OptimizePlanError> {
    #[cfg(not(windows))]
    {
        Err(OptimizePlanError::PlatformUnsupported)
    }
    #[cfg(windows)]
    {
        let audit = super::audit::optimize_audit_v1_path().ok();
        let now = unix_ms(SystemTime::now());
        Ok(MaintenanceFacts {
            uac_declined: !process_is_elevated(),
            battery_low: battery_is_low(),
            reboot_pending: reboot_is_pending(),
            volume: probe_retrim_volume(),
            defrag: existing_system_program("defrag.exe"),
            w32tm: existing_system_program("w32tm.exe"),
            dism: existing_system_program("dism.exe"),
            // No DISM process is launched to discover this. Repair stays
            // refused until a caller injects a repairable result.
            image_repairable: false,
            retrim_cooldown: audit
                .as_deref()
                .is_some_and(|path| retrim_cooldown_active(path, now)),
            startup: read_live_startup(),
            backup_dir: std::env::temp_dir().join("devsweep-dead-startup"),
            backup_fails: false,
        })
    }
}

pub(super) fn retrim_cooldown_active(path: &Path, now_unix_ms: u64) -> bool {
    match super::audit::last_succeeded_unix_ms(path, MAINTENANCE_RETRIM_ID) {
        Ok(Some(at)) => now_unix_ms.saturating_sub(at) < RETRIM_COOLDOWN_MS,
        Ok(None) => false,
        Err(_) => true,
    }
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |duration| {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    })
}

fn live_disk_devices() -> Vec<DiskHealthDeviceV1> {
    #[cfg(windows)]
    {
        windows_disk_devices()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn existing_system_program(file_name: &str) -> Option<PathBuf> {
    super::windows::resolve_system_program(file_name).ok()
}

#[cfg(windows)]
fn process_is_elevated() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut TOKEN_ELEVATION as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        );
        let _ = CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

#[cfg(windows)]
fn battery_is_low() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    // SAFETY: `status` is a writable zeroed `SYSTEM_POWER_STATUS`.
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // SAFETY: `status` remains valid for the duration of the call.
    let ok = unsafe { GetSystemPowerStatus(&mut status) };
    ok == 0 || status.ACLineStatus != 1
}

#[cfg(windows)]
fn reboot_is_pending() -> bool {
    const HKEY_LOCAL_MACHINE: isize = -2_147_483_646;
    key_exists(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending",
    ) || key_exists(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired",
    ) || value_exists(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\Session Manager",
        "PendingFileRenameOperations",
    )
}

#[cfg(windows)]
fn key_exists(hive: isize, subkey: &str) -> bool {
    let wide = wide(subkey);
    let mut handle = std::ptr::null_mut();
    // SAFETY: `wide` is NUL-terminated and `handle` is writable.
    let status = unsafe {
        RegOpenKeyExW(
            hive as *mut _,
            wide.as_ptr(),
            0,
            0x0002_0019 | 0x0100,
            &mut handle,
        )
    };
    if status == 0 {
        // SAFETY: `handle` was opened by the call above.
        unsafe { RegCloseKey(handle) };
        return true;
    }
    // Access denied fails closed: treat the reboot state as pending.
    status == 5
}

#[cfg(windows)]
fn value_exists(hive: isize, subkey: &str, value: &str) -> bool {
    let wide_key = wide(subkey);
    let mut handle = std::ptr::null_mut();
    // SAFETY: `wide_key` is NUL-terminated and `handle` is writable.
    let status = unsafe {
        RegOpenKeyExW(
            hive as *mut _,
            wide_key.as_ptr(),
            0,
            0x0002_0019 | 0x0100,
            &mut handle,
        )
    };
    if status == 5 {
        return true;
    }
    if status != 0 {
        return false;
    }
    let wide_value = wide(value);
    let mut data_len = 0u32;
    // SAFETY: the key handle is open and the length pointer is writable.
    let query = unsafe {
        RegQueryValueExW(
            handle,
            wide_value.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut data_len,
        )
    };
    // SAFETY: `handle` was opened by the call above.
    unsafe { RegCloseKey(handle) };
    query == 0 || query == 234
}

#[cfg(windows)]
fn probe_retrim_volume() -> VolumeFact {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

    const DRIVE_FIXED: u32 = 3;
    // SAFETY: `GetLogicalDrives` takes no pointer.
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return VolumeFact::DiskProblem;
    }
    for index in 0..26 {
        if mask & (1 << index) == 0 {
            continue;
        }
        let letter = char::from(b'A' + index);
        let root = wide(&format!("{letter}:\\"));
        // SAFETY: `root` is a NUL-terminated path.
        let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
        if kind != DRIVE_FIXED {
            continue;
        }
        let volume = format!("{letter}:");
        return match volume_is_ssd(&volume) {
            Some(true) => VolumeFact::ReadySsd { volume },
            Some(false) => VolumeFact::NotSsd,
            None => VolumeFact::NotSsd,
        };
    }
    VolumeFact::None
}

#[cfg(windows)]
fn volume_is_ssd(volume: &str) -> Option<bool> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        DEVICE_SEEK_PENALTY_DESCRIPTOR, IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery,
        STORAGE_PROPERTY_QUERY, StorageDeviceSeekPenaltyProperty,
    };

    let path = wide(&format!(r"\\.\{volume}"));
    // SAFETY: `path` is NUL-terminated. The handle is closed below.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceSeekPenaltyProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0],
    };
    let mut penalty: DEVICE_SEEK_PENALTY_DESCRIPTOR = unsafe { std::mem::zeroed() };
    let mut returned = 0u32;
    // SAFETY: both buffers are writable for the duration of the call.
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &mut query as *mut STORAGE_PROPERTY_QUERY as *const _,
            std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            &mut penalty as *mut DEVICE_SEEK_PENALTY_DESCRIPTOR as *mut _,
            std::mem::size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    // SAFETY: `handle` was opened above.
    unsafe { CloseHandle(handle) };
    if ok == 0 {
        return None;
    }
    Some(!penalty.IncursSeekPenalty)
}

#[cfg(windows)]
fn windows_disk_devices() -> Vec<DiskHealthDeviceV1> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

    const DRIVE_FIXED: u32 = 3;
    // SAFETY: `GetLogicalDrives` takes no pointer.
    let mask = unsafe { GetLogicalDrives() };
    let mut devices = Vec::new();
    for index in 0..26 {
        if mask & (1 << index) == 0 {
            continue;
        }
        let letter = char::from(b'A' + index);
        let root = wide(&format!("{letter}:\\"));
        // SAFETY: `root` is a NUL-terminated path.
        let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
        if kind != DRIVE_FIXED {
            continue;
        }
        devices.push(DiskHealthDeviceV1 {
            device_id: format!("{letter}:"),
            health_status: DiskHealthStatus::Unknown,
        });
    }
    devices
}

#[cfg(windows)]
fn read_live_startup() -> Vec<StartupFact> {
    let mut facts = read_run_values();
    facts.extend(read_startup_links());
    facts
}

#[cfg(not(windows))]
fn read_live_startup() -> Vec<StartupFact> {
    Vec::new()
}

#[cfg(windows)]
fn read_run_values() -> Vec<StartupFact> {
    const HKEY_CURRENT_USER: isize = -2_147_483_647;
    let subkey = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let wide_key = wide(subkey);
    let mut handle = std::ptr::null_mut();
    // SAFETY: `wide_key` is NUL-terminated and `handle` is writable.
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER as *mut _,
            wide_key.as_ptr(),
            0,
            0x0002_0019 | 0x0100,
            &mut handle,
        )
    };
    if status != 0 {
        return Vec::new();
    }
    let mut facts = Vec::new();
    let mut index = 0u32;
    loop {
        let mut name = [0u16; 256];
        let mut name_len = name.len() as u32;
        let mut value_type = 0u32;
        let mut data = [0u8; 2048];
        let mut data_len = data.len() as u32;
        // SAFETY: the buffers and lengths are writable for one value.
        let enum_status = unsafe {
            RegEnumValueW(
                handle,
                index,
                name.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                &mut value_type,
                data.as_mut_ptr(),
                &mut data_len,
            )
        };
        if enum_status != 0 {
            break;
        }
        index = index.saturating_add(1);
        if value_type != 1 && value_type != 2 {
            continue;
        }
        let Some(value_name) = wide_to_string(&name, name_len) else {
            continue;
        };
        let Some(command) = bytes_to_wide_string(&data, data_len) else {
            continue;
        };
        let expanded = expand_env(&command);
        let target = command_target(&expanded).unwrap_or_default();
        let target_exists = !target.as_os_str().is_empty() && target.exists();
        let reg_line = format!("\"{value_name}\"=\"{}\"\r\n", command.replace('"', "\\\""));
        facts.push(StartupFact {
            name: value_name,
            kind: DeadStartupKind::RunValue,
            target,
            target_exists,
            link_path: None,
            reg_line,
        });
    }
    // SAFETY: `handle` was opened above.
    unsafe { RegCloseKey(handle) };
    facts
}

#[cfg(windows)]
fn read_startup_links() -> Vec<StartupFact> {
    let Some(folder) = startup_folder() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&folder) else {
        return Vec::new();
    };
    let mut facts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
        {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(target) = lnk_target(&path) else {
            continue;
        };
        let target_exists = target.exists();
        facts.push(StartupFact {
            name: name.to_string(),
            kind: DeadStartupKind::StartupLink,
            target,
            target_exists,
            link_path: Some(path),
            reg_line: String::new(),
        });
    }
    facts
}

#[cfg(windows)]
fn startup_folder() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup"))
}

#[cfg(windows)]
fn delete_run_value(name: &str) -> bool {
    const HKEY_CURRENT_USER: isize = -2_147_483_647;
    let subkey = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let wide_key = wide(subkey);
    let mut handle = std::ptr::null_mut();
    // SAFETY: `wide_key` is NUL-terminated and `handle` is writable.
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER as *mut _,
            wide_key.as_ptr(),
            0,
            0x0002,
            &mut handle,
        )
    };
    if status != 0 {
        return false;
    }
    let wide_name = wide(name);
    // SAFETY: the key is open and the name is NUL-terminated.
    let deleted = unsafe { RegDeleteValueW(handle, wide_name.as_ptr()) } == 0;
    // SAFETY: `handle` was opened above.
    unsafe { RegCloseKey(handle) };
    deleted
}

#[cfg(windows)]
fn delete_startup_link(path: Option<&Path>) -> bool {
    let Some(path) = path else {
        return false;
    };
    let Some(folder) = startup_folder() else {
        return false;
    };
    trash_startup_link_in(path, &folder, &SystemTrashRunner)
}

/// The containment guard plus the move. `path` must resolve inside `folder`,
/// and the move goes through [`TrashRunner`] so the shortcut lands in the
/// Recycle Bin instead of being unlinked in place.
#[cfg(windows)]
fn trash_startup_link_in(path: &Path, folder: &Path, trash: &dyn TrashRunner) -> bool {
    if !path.starts_with(folder) {
        return false;
    }
    trash.move_to_trash(path).is_ok()
}

#[cfg(not(windows))]
fn delete_run_value(_name: &str) -> bool {
    false
}

#[cfg(not(windows))]
fn delete_startup_link(_path: Option<&Path>) -> bool {
    false
}

#[cfg(windows)]
fn command_target(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    if let Some(rest) = command.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(PathBuf::from(&rest[..end]));
    }
    let token = command.split_whitespace().next()?;
    Some(PathBuf::from(token))
}

#[cfg(windows)]
fn expand_env(value: &str) -> String {
    let wide_in = wide(value);
    let mut buffer = vec![0u16; 1024];
    // SAFETY: both buffers are writable and the input is NUL-terminated.
    let len = unsafe {
        ExpandEnvironmentStringsW(wide_in.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32)
    };
    if len == 0 || len as usize > buffer.len() {
        return value.to_string();
    }
    wide_to_string(&buffer, len.saturating_sub(1)).unwrap_or_else(|| value.to_string())
}

#[cfg(windows)]
fn lnk_target(path: &Path) -> Option<PathBuf> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 0x4c || u32::from_le_bytes(bytes[0..4].try_into().ok()?) != 0x4c {
        return None;
    }
    let flags = u32::from_le_bytes(bytes[0x14..0x18].try_into().ok()?);
    let mut offset = 0x4c;
    if flags & 0x1 != 0 {
        let id_size = u16::from_le_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?) as usize;
        offset = offset.checked_add(2 + id_size)?;
    }
    if flags & 0x2 == 0 {
        return None;
    }
    let info = bytes.get(offset..)?;
    if info.len() < 0x1c {
        return None;
    }
    let header_size = u32::from_le_bytes(info[4..8].try_into().ok()?) as usize;
    let info_flags = u32::from_le_bytes(info[8..12].try_into().ok()?);
    if info_flags & 0x1 == 0 {
        return None;
    }
    let unicode = if header_size >= 0x24 {
        u32::from_le_bytes(info.get(0x1c..0x20)?.try_into().ok()?) as usize
    } else {
        0
    };
    if unicode != 0 {
        return read_utf16z(info, unicode);
    }
    let ansi = u32::from_le_bytes(info[0x10..0x14].try_into().ok()?) as usize;
    read_ansi_z(info, ansi)
}

#[cfg(windows)]
fn read_utf16z(bytes: &[u8], offset: usize) -> Option<PathBuf> {
    let rest = bytes.get(offset..)?;
    let mut units = Vec::new();
    let mut index = 0;
    while index + 1 < rest.len() {
        let unit = u16::from_le_bytes(rest[index..index + 2].try_into().ok()?);
        if unit == 0 {
            break;
        }
        units.push(unit);
        index += 2;
    }
    let text = String::from_utf16(&units).ok()?;
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    }
}

#[cfg(windows)]
fn read_ansi_z(bytes: &[u8], offset: usize) -> Option<PathBuf> {
    let rest = bytes.get(offset..)?;
    let end = rest
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(rest.len());
    let text = String::from_utf8_lossy(&rest[..end]);
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text.as_ref()))
    }
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn wide_to_string(buffer: &[u16], len: u32) -> Option<String> {
    let len = usize::try_from(len).ok()?;
    let slice = buffer.get(..len)?;
    String::from_utf16(slice).ok()
}

#[cfg(windows)]
fn bytes_to_wide_string(data: &[u8], len: u32) -> Option<String> {
    let len = usize::try_from(len).ok()?;
    let bytes = data.get(..len)?;
    if bytes.len() < 2 {
        return None;
    }
    let mut units = Vec::new();
    let mut index = 0;
    while index + 1 < bytes.len() {
        let unit = u16::from_le_bytes(bytes[index..index + 2].try_into().ok()?);
        if unit == 0 {
            break;
        }
        units.push(unit);
        index += 2;
    }
    String::from_utf16(&units).ok()
}

#[cfg(windows)]
#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        key: *mut std::ffi::c_void,
        subkey: *const u16,
        options: u32,
        access: u32,
        result: *mut *mut std::ffi::c_void,
    ) -> i32;
    fn RegCloseKey(key: *mut std::ffi::c_void) -> i32;
    fn RegQueryValueExW(
        key: *mut std::ffi::c_void,
        value: *const u16,
        reserved: *mut u32,
        value_type: *mut u32,
        data: *mut u8,
        data_len: *mut u32,
    ) -> i32;
    fn RegEnumValueW(
        key: *mut std::ffi::c_void,
        index: u32,
        name: *mut u16,
        name_len: *mut u32,
        reserved: *mut u32,
        value_type: *mut u32,
        data: *mut u8,
        data_len: *mut u32,
    ) -> i32;
    fn RegDeleteValueW(key: *mut std::ffi::c_void, name: *const u16) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn ExpandEnvironmentStringsW(src: *const u16, dst: *mut u16, size: u32) -> u32;
}

#[cfg(test)]
mod tests {
    use super::super::catalogue::{catalogue_entry, catalogue_generation, published_entries};
    use super::super::plan::{MaintenancePlanV1, OPTIMIZE_PLAN_VERSION, plan_operation};
    use super::*;
    use std::sync::Mutex;

    fn program() -> PathBuf {
        PathBuf::from("C:/devsweep/maintenance-double.exe")
    }

    fn facts() -> MaintenanceFacts {
        MaintenanceFacts {
            uac_declined: false,
            battery_low: false,
            reboot_pending: false,
            volume: VolumeFact::ReadySsd {
                volume: "C:".to_string(),
            },
            defrag: Some(program()),
            w32tm: Some(program()),
            dism: Some(program()),
            image_repairable: true,
            retrim_cooldown: false,
            startup: Vec::new(),
            backup_dir: PathBuf::from("C:/devsweep/backup"),
            backup_fails: false,
        }
    }

    fn argv_of(action: &ResolvedMaintenanceAction) -> &[String] {
        match action {
            ResolvedMaintenanceAction::NativeCommand { argv, .. } => argv,
            _ => panic!("native command"),
        }
    }

    #[test]
    fn published_catalogue_lists_v1_and_v2_classes() {
        let entries = published_entries();
        assert_eq!(entries.len(), 19);
        assert_eq!(entries[0].id, "dns.flush");
        assert_eq!(entries[0].action_class, MaintenanceActionClass::Execute);
        assert_eq!(catalogue_generation("dns.flush"), Some(1));
        let classes = [
            ("maintenance.retrim", MaintenanceActionClass::Execute),
            ("maintenance.clock_probe", MaintenanceActionClass::Execute),
            ("maintenance.clock_sync", MaintenanceActionClass::Execute),
            ("maintenance.image_scan", MaintenanceActionClass::Execute),
            ("maintenance.image_repair", MaintenanceActionClass::Execute),
            ("maintenance.dead_startup", MaintenanceActionClass::Execute),
            ("maintenance.disk_health", MaintenanceActionClass::Observe),
            ("maintenance.update_cache", MaintenanceActionClass::Guidance),
            ("maintenance.icon_cache", MaintenanceActionClass::Guidance),
            ("maintenance.search", MaintenanceActionClass::Guidance),
            (
                "maintenance.winget_repair",
                MaintenanceActionClass::Guidance,
            ),
        ];
        for (id, class) in classes {
            let entry = catalogue_entry(id).unwrap();
            assert_eq!(entry.action_class, class, "{id}");
            assert_eq!(catalogue_generation(id), Some(2), "{id}");
        }
        let encoded = serde_json::to_string(&entries).unwrap();
        assert!(!encoded.contains("program"));
        assert!(!encoded.contains("argv"));
        assert!(!encoded.contains("defrag"));
        assert!(!encoded.contains("dism"));
        assert!(!encoded.contains("w32tm"));
    }

    #[test]
    fn v1_dns_flush_plan_still_previews_and_v1_plan_rejects_v2_id() {
        let plan = plan_operation("dns.flush").unwrap();
        assert_eq!(plan.catalogue_version, 1);
        assert!(plan.action_class.is_none());
        let stale = MaintenancePlanV1 {
            version: OPTIMIZE_PLAN_VERSION,
            catalogue_version: 1,
            operation_id: MAINTENANCE_RETRIM_ID.to_string(),
            action_class: None,
        };
        let preview = super::super::plan::preview_maintenance_plan(&plan, &DnsPreflight).unwrap();
        assert_eq!(preview.operation_id, "dns.flush");
        assert_eq!(preview.catalogue_version, 1);
        assert_eq!(preview.action_class, MaintenanceActionClass::Execute);
        assert!(preview.digest.starts_with("sha256:"));
        assert!(matches!(
            super::super::plan::construct_validated_action(&stale, &DnsPreflight),
            Err(OptimizePlanError::UnknownOperation(_))
        ));
    }

    #[test]
    fn guidance_plan_returns_class_and_observe_plan_is_refused() {
        for id in [
            "maintenance.update_cache",
            "maintenance.icon_cache",
            "maintenance.search",
            "maintenance.winget_repair",
        ] {
            let plan = plan_operation(id).unwrap();
            assert_eq!(plan.action_class, Some(MaintenanceActionClass::Guidance));
            assert_eq!(plan.catalogue_version, 2);
            assert_eq!(plan.operation_id, id);
            assert!(matches!(
                super::super::plan::construct_validated_action(&plan, &FailPreflight),
                Err(OptimizePlanError::GuidanceNotExecutable(_))
            ));
        }
        assert!(matches!(
            plan_operation(MAINTENANCE_DISK_HEALTH_ID),
            Err(OptimizePlanError::ObserveNotPlanable(_))
        ));
    }

    #[test]
    fn one_plan_keeps_scan_and_probe_from_starting_the_next_operation() {
        let scan = resolve_with_facts(
            catalogue_entry(MAINTENANCE_IMAGE_SCAN_ID).unwrap(),
            &facts(),
        )
        .unwrap();
        let repair = resolve_with_facts(
            catalogue_entry(MAINTENANCE_IMAGE_REPAIR_ID).unwrap(),
            &facts(),
        )
        .unwrap();
        let probe = resolve_with_facts(
            catalogue_entry(MAINTENANCE_CLOCK_PROBE_ID).unwrap(),
            &facts(),
        )
        .unwrap();
        let sync = resolve_with_facts(
            catalogue_entry(MAINTENANCE_CLOCK_SYNC_ID).unwrap(),
            &facts(),
        )
        .unwrap();
        assert_eq!(
            argv_of(&scan),
            &[
                "/Online".to_string(),
                "/Cleanup-Image".to_string(),
                "/ScanHealth".to_string()
            ]
        );
        assert!(!argv_of(&scan).iter().any(|arg| arg == "/RestoreHealth"));
        assert!(argv_of(&repair).iter().any(|arg| arg == "/RestoreHealth"));
        assert!(!argv_of(&repair).iter().any(|arg| arg == "/ScanHealth"));
        assert_eq!(
            argv_of(&probe),
            &[
                "/stripchart".to_string(),
                "/computer:time.windows.com".to_string(),
                "/samples:3".to_string(),
                "/period:1".to_string(),
                "/dataonly".to_string(),
            ]
        );
        assert!(!argv_of(&probe).iter().any(|arg| arg == "/resync"));
        assert_eq!(argv_of(&sync), &["/resync".to_string()]);
        assert!(!argv_of(&sync).iter().any(|arg| arg.contains("stripchart")));
        let retrim =
            resolve_with_facts(catalogue_entry(MAINTENANCE_RETRIM_ID).unwrap(), &facts()).unwrap();
        assert_eq!(argv_of(&retrim), &["/L".to_string(), "C:".to_string()]);
        match &retrim {
            ResolvedMaintenanceAction::NativeCommand { program, argv } => {
                assert_ne!(program, &PathBuf::from(argv.join(" ")));
                assert!(!argv.iter().any(|arg| arg.contains(' ')));
            }
            _ => panic!("retrim"),
        }
    }

    #[test]
    fn precheck_refusal_is_limited_to_the_affected_action() {
        let cases = [
            (
                "uac",
                PrecheckReason::UacDeclined,
                MAINTENANCE_RETRIM_ID,
                MAINTENANCE_CLOCK_PROBE_ID,
            ),
            (
                "battery",
                PrecheckReason::LowBattery,
                MAINTENANCE_RETRIM_ID,
                MAINTENANCE_CLOCK_PROBE_ID,
            ),
            (
                "ssd",
                PrecheckReason::NotSsd,
                MAINTENANCE_RETRIM_ID,
                MAINTENANCE_IMAGE_SCAN_ID,
            ),
            (
                "disk",
                PrecheckReason::DiskProblem,
                MAINTENANCE_RETRIM_ID,
                MAINTENANCE_CLOCK_SYNC_ID,
            ),
            (
                "reboot",
                PrecheckReason::RebootPending,
                MAINTENANCE_IMAGE_SCAN_ID,
                MAINTENANCE_CLOCK_PROBE_ID,
            ),
            (
                "cooldown",
                PrecheckReason::Cooldown,
                MAINTENANCE_RETRIM_ID,
                MAINTENANCE_IMAGE_REPAIR_ID,
            ),
            (
                "missing",
                PrecheckReason::ProgramMissing,
                MAINTENANCE_CLOCK_PROBE_ID,
                MAINTENANCE_IMAGE_SCAN_ID,
            ),
            (
                "repair",
                PrecheckReason::ImageNotRepairable,
                MAINTENANCE_IMAGE_REPAIR_ID,
                MAINTENANCE_IMAGE_SCAN_ID,
            ),
            (
                "backup",
                PrecheckReason::BackupWriteFailed,
                MAINTENANCE_DEAD_STARTUP_ID,
                MAINTENANCE_CLOCK_PROBE_ID,
            ),
        ];
        for (label, reason, refused, allowed) in cases {
            let mut sample = facts();
            match label {
                "uac" => sample.uac_declined = true,
                "battery" => sample.battery_low = true,
                "ssd" => sample.volume = VolumeFact::NotSsd,
                "disk" => sample.volume = VolumeFact::DiskProblem,
                "reboot" => sample.reboot_pending = true,
                "cooldown" => sample.retrim_cooldown = true,
                "missing" => sample.w32tm = None,
                "repair" => sample.image_repairable = false,
                "backup" => sample.backup_fails = true,
                _ => unreachable!(),
            }
            let error = resolve_with_facts(catalogue_entry(refused).unwrap(), &sample).unwrap_err();
            assert_eq!(
                error,
                OptimizePlanError::PrecheckRefused {
                    operation_id: refused.to_string(),
                    reason,
                },
                "{label}"
            );
            assert!(
                resolve_with_facts(catalogue_entry(allowed).unwrap(), &sample).is_ok(),
                "{label} affected {allowed}"
            );
        }
    }

    #[test]
    fn retrim_cooldown_comes_from_the_optimize_audit() {
        use super::super::audit::{
            OptimizeAuditEvent, OptimizeAuditJournal, OptimizeAuditRecordV1,
            OptimizeAuditStatusCode, OptimizeAuditTransition,
        };
        use super::super::execution::{AdapterOutcome, MaintenanceExecutionOutcome};
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("optimize.jsonl");
        let now = 1_700_000_000_000;
        {
            let mut journal = OptimizeAuditJournal::open(&path).unwrap();
            let digest = format!("sha256:{}", "a".repeat(64));
            for (transition, status, adapter) in [
                (
                    OptimizeAuditTransition::Validated,
                    OptimizeAuditStatusCode::Validated,
                    None,
                ),
                (
                    OptimizeAuditTransition::DispatchStarted,
                    OptimizeAuditStatusCode::DispatchStarted,
                    None,
                ),
                (
                    OptimizeAuditTransition::AdapterCompleted,
                    OptimizeAuditStatusCode::AdapterSucceeded,
                    Some(AdapterOutcome::Success),
                ),
                (
                    OptimizeAuditTransition::Terminal {
                        outcome: MaintenanceExecutionOutcome::Succeeded,
                    },
                    OptimizeAuditStatusCode::Succeeded,
                    Some(AdapterOutcome::Success),
                ),
            ] {
                journal
                    .append(OptimizeAuditRecordV1::new(
                        "op",
                        now,
                        MAINTENANCE_RETRIM_ID,
                        MaintenanceActionClass::Execute,
                        &digest,
                        OptimizeAuditEvent {
                            transition,
                            status_code: status,
                            error_code: None,
                            adapter_outcome: adapter,
                        },
                    ))
                    .unwrap();
            }
        }
        assert!(retrim_cooldown_active(&path, now + 1_000));
        assert!(!retrim_cooldown_active(&path, now + RETRIM_COOLDOWN_MS + 1));
        let mut sample = facts();
        sample.retrim_cooldown = retrim_cooldown_active(&path, now + 1_000);
        assert!(matches!(
            resolve_with_facts(catalogue_entry(MAINTENANCE_RETRIM_ID).unwrap(), &sample),
            Err(OptimizePlanError::PrecheckRefused {
                reason: PrecheckReason::Cooldown,
                ..
            })
        ));
    }

    struct MemStore {
        events: Mutex<Vec<&'static str>>,
        fail_backup: bool,
        backup_path: PathBuf,
        live_targets: Mutex<Vec<String>>,
        deleted: Mutex<Vec<String>>,
        removals: Mutex<Vec<(String, DeadStartupRemoval)>>,
        fail_delete: bool,
    }

    impl MemStore {
        fn new(backup_path: &Path) -> Self {
            Self {
                events: Mutex::new(Vec::new()),
                fail_backup: false,
                backup_path: backup_path.to_path_buf(),
                live_targets: Mutex::new(Vec::new()),
                deleted: Mutex::new(Vec::new()),
                removals: Mutex::new(Vec::new()),
                fail_delete: false,
            }
        }
    }

    impl DeadStartupStore for MemStore {
        fn write_backup(&self, path: &Path, body: &str) -> bool {
            self.events.lock().unwrap().push("backup");
            if self.fail_backup {
                return false;
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, body).unwrap();
            true
        }

        fn target_exists(&self, deletion: &DeadStartupDeletion) -> bool {
            self.live_targets
                .lock()
                .unwrap()
                .iter()
                .any(|name| name == &deletion.name)
        }

        fn delete(&self, deletion: &DeadStartupDeletion) -> bool {
            assert!(self.backup_path.is_file(), "delete before backup");
            self.events.lock().unwrap().push("delete");
            self.deleted.lock().unwrap().push(deletion.name.clone());
            self.removals
                .lock()
                .unwrap()
                .push((deletion.name.clone(), deletion.removal()));
            !self.fail_delete
        }
    }

    #[test]
    fn dead_startup_deletes_only_missing_targets_after_the_backup_exists() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut sample = facts();
        sample.backup_dir = temp.path().to_path_buf();
        sample.startup = vec![
            StartupFact {
                name: "Gone".to_string(),
                kind: DeadStartupKind::RunValue,
                target: PathBuf::from("C:/missing/gone.exe"),
                target_exists: false,
                link_path: None,
                reg_line: "\"Gone\"=\"C:\\\\missing\\\\gone.exe\"\r\n".to_string(),
            },
            StartupFact {
                name: "StillHere".to_string(),
                kind: DeadStartupKind::StartupLink,
                target: PathBuf::from("C:/live/app.exe"),
                target_exists: true,
                link_path: Some(PathBuf::from("C:/startup/app.lnk")),
                reg_line: String::new(),
            },
        ];
        let action = resolve_with_facts(
            catalogue_entry(MAINTENANCE_DEAD_STARTUP_ID).unwrap(),
            &sample,
        )
        .unwrap();
        let ResolvedMaintenanceAction::DeadStartup {
            backup_path,
            backup_body,
            skip_names,
            deletions,
            ..
        } = &action
        else {
            panic!("dead startup");
        };
        assert_eq!(skip_names, &vec!["StillHere".to_string()]);
        assert_eq!(deletions.len(), 1);
        assert!(backup_body.contains("Gone"));
        assert!(!backup_body.contains("StillHere"));
        let store = MemStore {
            live_targets: Mutex::new(vec!["Changed".to_string()]),
            ..MemStore::new(backup_path)
        };
        let mut changed = action.clone();
        if let ResolvedMaintenanceAction::DeadStartup { deletions, .. } = &mut changed {
            deletions.push(DeadStartupDeletion {
                name: "Changed".to_string(),
                kind: DeadStartupKind::RunValue,
                target: PathBuf::from("C:/missing/changed.exe"),
                link_path: None,
            });
        }
        assert_eq!(
            dispatch_dead_startup(&changed, &store).outcome,
            super::super::execution::AdapterOutcome::Success
        );
        assert_eq!(store.deleted.lock().unwrap().as_slice(), ["Gone"]);
        assert_eq!(
            store.events.lock().unwrap().as_slice(),
            ["backup", "delete"]
        );
        assert!(backup_path.is_file());

        let fail = MemStore {
            fail_backup: true,
            ..MemStore::new(backup_path)
        };
        assert_eq!(
            dispatch_dead_startup(&action, &fail).outcome,
            super::super::execution::AdapterOutcome::Failure
        );
        assert!(fail.deleted.lock().unwrap().is_empty());
    }

    #[test]
    fn fake_runner_requery_omits_deleted_dead_startup_name() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut sample = facts();
        sample.backup_dir = temp.path().to_path_buf();
        sample.startup = vec![
            StartupFact {
                name: "Gone".to_string(),
                kind: DeadStartupKind::RunValue,
                target: PathBuf::from("C:/missing/gone.exe"),
                target_exists: false,
                link_path: None,
                reg_line: "\"Gone\"=\"C:\\\\missing\\\\gone.exe\"\r\n".to_string(),
            },
            StartupFact {
                name: "StillHere".to_string(),
                kind: DeadStartupKind::StartupLink,
                target: PathBuf::from("C:/live/app.exe"),
                target_exists: true,
                link_path: Some(PathBuf::from("C:/startup/app.lnk")),
                reg_line: String::new(),
            },
        ];
        let action = resolve_with_facts(
            catalogue_entry(MAINTENANCE_DEAD_STARTUP_ID).unwrap(),
            &sample,
        )
        .unwrap();
        let ResolvedMaintenanceAction::DeadStartup { backup_path, .. } = &action else {
            panic!("dead startup");
        };
        let store = MemStore::new(backup_path);
        assert_eq!(
            dispatch_dead_startup(&action, &store).outcome,
            super::super::execution::AdapterOutcome::Success
        );
        assert_eq!(store.deleted.lock().unwrap().as_slice(), ["Gone"]);
        assert!(backup_path.is_file());

        let mut requery = sample.clone();
        requery.startup = vec![StartupFact {
            name: "StillHere".to_string(),
            kind: DeadStartupKind::StartupLink,
            target: PathBuf::from("C:/live/app.exe"),
            target_exists: true,
            link_path: Some(PathBuf::from("C:/startup/app.lnk")),
            reg_line: String::new(),
        }];
        let second = resolve_with_facts(
            catalogue_entry(MAINTENANCE_DEAD_STARTUP_ID).unwrap(),
            &requery,
        )
        .unwrap();
        let ResolvedMaintenanceAction::DeadStartup {
            deletions,
            skip_names,
            ..
        } = &second
        else {
            panic!("dead startup requery");
        };
        assert!(!deletions.iter().any(|entry| entry.name == "Gone"));
        assert!(!skip_names.iter().any(|name| name == "Gone"));
        assert_eq!(skip_names, &vec!["StillHere".to_string()]);
        assert!(backup_path.is_file());
        assert!(
            !store
                .deleted
                .lock()
                .unwrap()
                .iter()
                .any(|name| name == "StillHere")
        );
    }

    /// Records the move so the test can prove the path went to trash.
    #[cfg(windows)]
    #[derive(Default)]
    struct RecordingTrash {
        moved: Mutex<Vec<PathBuf>>,
        fail: bool,
    }

    #[cfg(windows)]
    impl TrashRunner for RecordingTrash {
        fn move_to_trash(&self, path: &Path) -> anyhow::Result<()> {
            if self.fail {
                return Err(anyhow::anyhow!("trash unavailable"));
            }
            self.moved.lock().unwrap().push(path.to_path_buf());
            Ok(())
        }
    }

    #[cfg(windows)]
    #[test]
    fn startup_link_deletion_moves_to_trash_and_never_unlinks_in_place() {
        let startup = PathBuf::from(
            r"C:\Users\someone\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup",
        );
        let link = startup.join("dead-app.lnk");
        let trash = RecordingTrash::default();
        assert!(trash_startup_link_in(&link, &startup, &trash));
        assert_eq!(trash.moved.lock().unwrap().as_slice(), [link.as_path()]);

        // A real shortcut under the Startup folder survives the call: the
        // removal is the injected trash move, not an in-place unlink.
        let temp = tempfile::TempDir::new().unwrap();
        let real_link = temp.path().join("real-dead-app.lnk");
        std::fs::write(&real_link, b"shortcut bytes").unwrap();
        let real_trash = RecordingTrash::default();
        assert!(trash_startup_link_in(&real_link, temp.path(), &real_trash));
        assert_eq!(
            real_trash.moved.lock().unwrap().as_slice(),
            [real_link.as_path()]
        );
        assert!(
            real_link.exists(),
            "a Startup link must never be unlinked in place"
        );

        // A path outside the Startup folder is refused before any move.
        let outside = PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts");
        assert!(!trash_startup_link_in(&outside, &startup, &trash));
        assert_eq!(trash.moved.lock().unwrap().len(), 1);

        // A failed move reports failure; there is no unlink fallback.
        let failing = RecordingTrash {
            moved: Mutex::new(Vec::new()),
            fail: true,
        };
        assert!(!trash_startup_link_in(&real_link, temp.path(), &failing));
        assert!(failing.moved.lock().unwrap().is_empty());
        assert!(
            real_link.exists(),
            "a failed trash move must not fall back to an unlink"
        );
    }

    #[test]
    fn dispatch_routes_a_startup_link_through_the_recycle_bin() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut sample = facts();
        sample.backup_dir = temp.path().to_path_buf();
        sample.startup = vec![
            StartupFact {
                name: "GoneValue".to_string(),
                kind: DeadStartupKind::RunValue,
                target: PathBuf::from("C:/missing/value.exe"),
                target_exists: false,
                link_path: None,
                reg_line: "\"GoneValue\"=\"C:\\\\missing\\\\value.exe\"\r\n".to_string(),
            },
            StartupFact {
                name: "GoneLink".to_string(),
                kind: DeadStartupKind::StartupLink,
                target: PathBuf::from("C:/missing/link.exe"),
                target_exists: false,
                link_path: Some(PathBuf::from("C:/startup/gone-link.lnk")),
                reg_line: String::new(),
            },
        ];
        let action = resolve_with_facts(
            catalogue_entry(MAINTENANCE_DEAD_STARTUP_ID).unwrap(),
            &sample,
        )
        .unwrap();
        let ResolvedMaintenanceAction::DeadStartup { backup_path, .. } = &action else {
            panic!("dead startup");
        };
        let store = MemStore::new(backup_path);
        assert_eq!(
            dispatch_dead_startup(&action, &store).outcome,
            super::super::execution::AdapterOutcome::Success
        );
        assert_eq!(
            store.removals.lock().unwrap().as_slice(),
            [
                ("GoneValue".to_string(), DeadStartupRemoval::RegistryValue),
                ("GoneLink".to_string(), DeadStartupRemoval::RecycleBin),
            ]
        );
    }

    #[test]
    fn dispatch_fails_when_the_recycle_bin_move_fails() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut sample = facts();
        sample.backup_dir = temp.path().to_path_buf();
        sample.startup = vec![StartupFact {
            name: "GoneLink".to_string(),
            kind: DeadStartupKind::StartupLink,
            target: PathBuf::from("C:/missing/link.exe"),
            target_exists: false,
            link_path: Some(PathBuf::from("C:/startup/gone-link.lnk")),
            reg_line: String::new(),
        }];
        let action = resolve_with_facts(
            catalogue_entry(MAINTENANCE_DEAD_STARTUP_ID).unwrap(),
            &sample,
        )
        .unwrap();
        let ResolvedMaintenanceAction::DeadStartup { backup_path, .. } = &action else {
            panic!("dead startup");
        };
        let store = MemStore {
            fail_delete: true,
            ..MemStore::new(backup_path)
        };
        assert_eq!(
            dispatch_dead_startup(&action, &store).outcome,
            super::super::execution::AdapterOutcome::Failure
        );
        assert!(backup_path.is_file());
    }

    #[test]
    fn backup_records_a_moved_shortcut_as_a_reg_comment() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut sample = facts();
        sample.backup_dir = temp.path().to_path_buf();
        sample.startup = vec![StartupFact {
            name: "GoneLink".to_string(),
            kind: DeadStartupKind::StartupLink,
            target: PathBuf::from("C:/missing/link.exe"),
            target_exists: false,
            link_path: Some(PathBuf::from("C:/startup/gone-link.lnk")),
            reg_line: String::new(),
        }];
        let action = resolve_with_facts(
            catalogue_entry(MAINTENANCE_DEAD_STARTUP_ID).unwrap(),
            &sample,
        )
        .unwrap();
        let ResolvedMaintenanceAction::DeadStartup {
            backup_body,
            deletions,
            ..
        } = &action
        else {
            panic!("dead startup");
        };
        assert_eq!(deletions.len(), 1);
        assert!(
            backup_body.contains("; shortcut moved to Recycle Bin: C:/startup/gone-link.lnk"),
            "{backup_body}"
        );
        // The header stays first so the file still imports as a `.reg`.
        assert!(backup_body.starts_with("Windows Registry Editor Version 5.00\r\n\r\n"));
        assert!(backup_body.contains(
            "[HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run]\r\n"
        ));
        assert!(backup_body.lines().all(|line| line.is_empty()
            || line.starts_with(';')
            || line.starts_with('[')
            || line.starts_with("Windows Registry Editor")
            || line.starts_with('"')));
    }

    struct DnsPreflight;

    impl super::super::LivePreflight for DnsPreflight {
        fn resolve(
            &self,
            _entry: &MaintenanceCatalogueEntryV1,
        ) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
            Ok(ResolvedMaintenanceAction::DnsFlush {
                program: PathBuf::from("C:/Windows/System32/ipconfig.exe"),
            })
        }
    }

    struct FailPreflight;

    impl super::super::LivePreflight for FailPreflight {
        fn resolve(
            &self,
            _entry: &MaintenanceCatalogueEntryV1,
        ) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
            panic!("guidance and observe must not resolve");
        }
    }

    #[test]
    fn command_double_receives_separate_argv_without_system_repair_programs() {
        let temp = tempfile::TempDir::new().unwrap();
        let program = temp.path().join(format!(
            "maintenance-double{}",
            std::env::consts::EXE_SUFFIX
        ));
        std::fs::copy(
            crate::process::test_support::process_fixture_exe(),
            &program,
        )
        .unwrap();
        std::fs::write(temp.path().join("optimize-double.json"), b"{}").unwrap();
        let mut sample = facts();
        sample.defrag = Some(program.clone());
        sample.w32tm = Some(program.clone());
        sample.dism = Some(program.clone());
        for id in [
            MAINTENANCE_RETRIM_ID,
            MAINTENANCE_CLOCK_PROBE_ID,
            MAINTENANCE_IMAGE_SCAN_ID,
        ] {
            let action = resolve_with_facts(catalogue_entry(id).unwrap(), &sample).unwrap();
            let ResolvedMaintenanceAction::NativeCommand { program, argv } = &action else {
                panic!("{id}");
            };
            let name = program.file_name().and_then(|name| name.to_str()).unwrap();
            assert!(
                !name.eq_ignore_ascii_case("defrag.exe")
                    && !name.eq_ignore_ascii_case("dism.exe")
                    && !name.eq_ignore_ascii_case("w32tm.exe")
                    && !is_shell(program)
            );
            let calls = temp.path().join("optimize-calls.json");
            let _ = std::fs::remove_file(&calls);
            let evidence = dispatch_native(program, argv, None, Duration::from_secs(10));
            assert_eq!(
                evidence.outcome,
                super::super::execution::AdapterOutcome::Success
            );
            let recorded: Vec<String> =
                serde_json::from_slice(&std::fs::read(&calls).unwrap()).unwrap();
            assert_eq!(recorded, *argv);
            assert!(
                !recorded
                    .iter()
                    .any(|arg| arg.contains("cmd.exe") || arg.contains("/c "))
            );
        }
    }

    #[test]
    fn cancel_leaves_a_dispatched_repair_double_running() {
        let fixture = crate::process::test_support::process_fixture_exe();
        let mut child = Command::new(&fixture)
            .arg("hang")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let flag = Arc::new(FlagCancelObserver::new());
        flag.request_cancel();
        let wait = wait_spawned(&mut child, Some(&flag), Duration::from_secs(5));
        assert!(matches!(wait, MaintenanceWait::LeaveRunning));
        assert!(child.try_wait().unwrap().is_none());
        let _ = child.kill();
        let _ = child.wait();
    }
}
