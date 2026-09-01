//! Fixed Windows adapters: manifest-independent OS build, fail-closed
//! System32/Sysnative resolution, bounded DNS dispatch, and the allowlisted
//! Settings handoff.
//!
//! These adapters never consult `PATH`, the environment, shell text,
//! `SysWOW64`, registry version text, or filesystem-redirection toggles, and
//! they never request elevation.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::LivePreflight;
use super::audit::OptimizeAuditErrorCode;
use super::catalogue::MaintenanceActionClass;
use super::execution::{AdapterEvidence, DispatchTiming, MaintenanceDispatch};
use super::plan::{OptimizePlanError, ResolvedMaintenanceAction, ValidatedMaintenanceAction};
use crate::process::{CancelObserver, FlagCancelObserver};

/// The one fixed argv element of `dns.flush`.
pub(super) const DNS_FLUSH_ARG: &str = "/flushdns";

#[derive(Debug, Default)]
pub(super) struct RealLivePreflight;

impl LivePreflight for RealLivePreflight {
    fn resolve(
        &self,
        entry: &super::catalogue::MaintenanceCatalogueEntryV1,
    ) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
        resolve_action(entry)
    }
}

#[derive(Debug, Default)]
pub(super) struct RealMaintenanceDispatch;

impl MaintenanceDispatch for RealMaintenanceDispatch {
    fn dispatch(
        &self,
        action: &ValidatedMaintenanceAction,
        cancel: Option<&Arc<FlagCancelObserver>>,
        timing: DispatchTiming,
    ) -> AdapterEvidence {
        match &action.resolved {
            ResolvedMaintenanceAction::DnsFlush { program } => {
                dispatch_dns_flush(program, cancel, timing)
            }
            ResolvedMaintenanceAction::SettingsHandoff { uri, .. } => {
                // Once `ShellExecuteExW` returns, the handoff already happened;
                // the adapter can only ever report `launched` or a stable
                // non-success code. Cancellation is a pre-dispatch boundary.
                dispatch_settings_handoff(uri)
            }
        }
    }
}

/// Resolves one catalogue row against the live platform.
pub(super) fn resolve_action(
    entry: &super::catalogue::MaintenanceCatalogueEntryV1,
) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
    match entry.action_class {
        MaintenanceActionClass::Guidance => Err(OptimizePlanError::GuidanceNotExecutable(
            entry.id.to_string(),
        )),
        MaintenanceActionClass::SettingsHandoff => {
            let (uri, floor) = super::catalogue::settings_handoff(entry.id)
                .ok_or_else(|| OptimizePlanError::UnknownOperation(entry.id.to_string()))?;
            let build = os_build().ok_or(OptimizePlanError::OsBuildUnavailable)?;
            if build < floor {
                return Err(OptimizePlanError::BuildUnsupported { build, floor });
            }
            Ok(ResolvedMaintenanceAction::SettingsHandoff {
                uri,
                observed_build: build,
            })
        }
        MaintenanceActionClass::Execute => {
            let program = resolve_ipconfig()?;
            Ok(ResolvedMaintenanceAction::DnsFlush { program })
        }
    }
}

/// Manifest-independent OS build from `RtlGetVersion`. Query failure is
/// `None`; there is no `GetVersionExW` or environment-text fallback.
#[must_use]
pub(crate) fn os_build() -> Option<u32> {
    #[cfg(windows)]
    {
        use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
        use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;

        // SAFETY: `info` is a writable `OSVERSIONINFOW` whose size field is
        // set before the call, as the API requires.
        let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
        info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        // SAFETY: `info` remains valid for the duration of the call.
        let status = unsafe { RtlGetVersion(&mut info) };
        if status != 0 {
            return None;
        }
        Some(info.dwBuildNumber)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The exact `IsWow64Process2` -> `Sysnative`/`GetSystemDirectoryW` resolver.
///
/// 1. `IsWow64Process2(GetCurrentProcess())`, failing closed on error.
/// 2. Nonzero process machine (32-bit process on 64-bit Windows):
///    `GetWindowsDirectoryW` plus the literal `Sysnative\ipconfig.exe`.
/// 3. Otherwise `GetSystemDirectoryW` plus the literal `ipconfig.exe`.
/// 4. Absolute local drive path, exact case-insensitive basename, existing
///    non-directory file, no reparse point, no path search, and `Sysnative`
///    is never canonicalized into `SysWOW64`.
/// 5. The caller passes the path as program and `/flushdns` as the only argv
///    element to the bounded runner.
pub(super) fn resolve_ipconfig() -> Result<PathBuf, OptimizePlanError> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::{
            GetSystemDirectoryW, GetWindowsDirectoryW, IMAGE_FILE_MACHINE,
        };
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};

        let mut process_machine: IMAGE_FILE_MACHINE = 0;
        let mut native_machine: IMAGE_FILE_MACHINE = 0;
        // SAFETY: both machine pointers are writable for one `IMAGE_FILE_MACHINE`.
        let ok = unsafe {
            IsWow64Process2(
                GetCurrentProcess(),
                &mut process_machine,
                &mut native_machine,
            )
        };
        if ok == 0 {
            return Err(OptimizePlanError::ResolverUnavailable);
        }

        let mut buffer = [0_u16; 1024];
        if process_machine != 0 {
            // SAFETY: `buffer` is writable for its full length.
            let len = unsafe { GetWindowsDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
            let windows_dir = directory_string(&buffer, len)?;
            let mut path = PathBuf::from(windows_dir);
            path.push("Sysnative\\ipconfig.exe");
            verify_ipconfig_identity(&path)?;
            Ok(path)
        } else {
            // SAFETY: `buffer` is writable for its full length.
            let len = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
            let system_dir = directory_string(&buffer, len)?;
            let mut path = PathBuf::from(system_dir);
            path.push("ipconfig.exe");
            verify_ipconfig_identity(&path)?;
            Ok(path)
        }
    }
    #[cfg(not(windows))]
    {
        Err(OptimizePlanError::PlatformUnsupported)
    }
}

#[cfg(windows)]
fn directory_string(buffer: &[u16], len: u32) -> Result<String, OptimizePlanError> {
    let len = usize::try_from(len).map_err(|_| OptimizePlanError::ResolverUnavailable)?;
    if len == 0 || len >= buffer.len() || buffer[len] != 0 {
        return Err(OptimizePlanError::ResolverUnavailable);
    }
    String::from_utf16(&buffer[..len]).map_err(|_| OptimizePlanError::ResolverUnavailable)
}

/// The closed identity checks of step 4. Everything that cannot be proven
/// closes with [`OptimizePlanError::ResolverUnavailable`].
pub(super) fn verify_ipconfig_identity(path: &Path) -> Result<(), OptimizePlanError> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::fs::MetadataExt;
        use std::path::{Component, Prefix};

        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

        let mut components = path.components();
        match components.next() {
            Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_)) => {}
            _ => return Err(OptimizePlanError::ResolverUnavailable),
        }
        match components.next() {
            Some(Component::RootDir) => {}
            _ => return Err(OptimizePlanError::ResolverUnavailable),
        }
        if components.any(|component| !matches!(component, Component::Normal(_))) {
            return Err(OptimizePlanError::ResolverUnavailable);
        }
        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(OptimizePlanError::ResolverUnavailable)?;
        if !name.eq_ignore_ascii_case("ipconfig.exe") {
            return Err(OptimizePlanError::ResolverUnavailable);
        }
        // `File::open` reaches the exact absolute path through `CreateFileW`
        // (which supports the `Sysnative` alias) without any extension or
        // `PATH` search, and the handle metadata proves an existing
        // non-directory file with no reparse point.
        let file = std::fs::File::open(path).map_err(|_| OptimizePlanError::ResolverUnavailable)?;
        let metadata = file
            .metadata()
            .map_err(|_| OptimizePlanError::ResolverUnavailable)?;
        if metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(OptimizePlanError::ResolverUnavailable);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(OptimizePlanError::PlatformUnsupported)
    }
}

/// Builds the exact bounded process request for `dns.flush`: the resolved
/// absolute path as program, exactly one argv element, a neutral working
/// directory, bounded captured output, and the fixed timeout. Exposed for
/// structural tests that prove the program/argv identity.
pub(super) fn dns_flush_request<'a>(
    program: &Path,
    cancel: &'a dyn CancelObserver,
    timing: DispatchTiming,
) -> crate::process::ProcessRequest<'a> {
    crate::process::ProcessRequest {
        program: program.as_os_str().to_os_string(),
        args: vec![std::ffi::OsString::from(DNS_FLUSH_ARG)],
        cwd: crate::process::CwdPolicy::Neutral,
        timeout: Some(timing.timeout),
        job_deadline: None,
        cancel,
    }
}

/// Dispatches the fixed `ipconfig.exe` `/flushdns` refresh through the bounded
/// process runner: absolute program, one argv element, neutral working
/// directory, bounded captured output, no shell, and no elevation.
pub(super) fn dispatch_dns_flush(
    program: &Path,
    cancel: Option<&Arc<FlagCancelObserver>>,
    timing: DispatchTiming,
) -> AdapterEvidence {
    let runner = crate::process::ProcessRunner::new(crate::process::ProcessPolicy {
        termination_grace: timing.termination_grace,
        poll_interval: timing.poll_interval,
        ..crate::process::ProcessPolicy::default()
    });
    let noop = crate::process::NoopCancelObserver;
    let observer: &dyn CancelObserver = match cancel {
        Some(flag) => flag.as_ref(),
        None => &noop,
    };
    adapter_evidence_from_status(
        runner
            .run(&dns_flush_request(program, observer, timing))
            .status,
    )
}

pub(super) fn adapter_evidence_from_status(
    status: crate::process::ProcessStatus,
) -> AdapterEvidence {
    match status {
        crate::process::ProcessStatus::Success => AdapterEvidence::success(),
        crate::process::ProcessStatus::Exit { code: Some(_) } => {
            AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterOperationFailed)
        }
        crate::process::ProcessStatus::Exit { code: None } => {
            AdapterEvidence::unfinished(OptimizeAuditErrorCode::AdapterOperationFailed)
        }
        crate::process::ProcessStatus::NotFound | crate::process::ProcessStatus::InvalidOutput => {
            AdapterEvidence::failure(OptimizeAuditErrorCode::AdapterDispatchFailed)
        }
        crate::process::ProcessStatus::Timeout => {
            AdapterEvidence::unfinished(OptimizeAuditErrorCode::AdapterTimedOut)
        }
        crate::process::ProcessStatus::Canceled => {
            AdapterEvidence::unfinished(OptimizeAuditErrorCode::AdapterCanceledAfterDispatch)
        }
    }
}

/// Raw `ShellExecuteExW` observation for native diagnosis. It is never an
/// audit identity: product terminals stay `launched` or
/// `adapter_operation_failed`.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SettingsLaunchProbe {
    pub succeeded: bool,
    /// `SHELLEXECUTEINFOW.hInstApp` after the call. On failure this is a
    /// `SE_ERR_*` code (`SE_ERR_ACCESSDENIED` = 5). On success it is a
    /// HINSTANCE greater than 32.
    pub h_inst_app: isize,
    /// `GetLastError()` immediately after `ShellExecuteExW`.
    pub last_error: u32,
    /// `SHELLEXECUTEINFOW.hProcess`. Stays null because the approved mask is
    /// only `SEE_MASK_FLAG_NO_UI` (no `SEE_MASK_NOCLOSEPROCESS`).
    pub h_process: isize,
}

/// windows-sys 0.59 gates `ShellExecuteExW` and `SHELLEXECUTEINFOW` behind
/// the extra `Win32_System_Registry` feature (for the unused `hkeyClass`
/// field). To keep the approved dependency surface to the exact four
/// catalogue features, the shell32 entry point is declared here with the
/// documented `SHELLEXECUTEINFOW` layout.
///
/// Layout matches windows-sys 0.59: `repr(C)` (112 bytes) on 64-bit, and
/// `repr(C, packed(1))` (60 bytes) on x86. `ShellExecuteExW` validates
/// `cbSize` against that ABI and rejects any other size with
/// `ERROR_ACCESS_DENIED` before it even resolves `lpFile`.
#[cfg(windows)]
#[cfg_attr(target_arch = "x86", repr(C, packed(1)))]
#[cfg_attr(not(target_arch = "x86"), repr(C))]
struct ShellExecuteInfoW {
    cb_size: u32,
    f_mask: u32,
    hwnd: *mut std::ffi::c_void,
    lp_verb: *const u16,
    lp_file: *const u16,
    lp_parameters: *const u16,
    lp_directory: *const u16,
    n_show: i32,
    h_inst_app: *mut std::ffi::c_void,
    lp_id_list: *mut std::ffi::c_void,
    lp_class: *const u16,
    hkey_class: *mut std::ffi::c_void,
    dw_hot_key: u32,
    icon_or_monitor: *mut std::ffi::c_void,
    h_process: *mut std::ffi::c_void,
}

#[cfg(all(windows, target_arch = "x86"))]
const _: () = {
    assert!(std::mem::size_of::<ShellExecuteInfoW>() == 60);
    assert!(std::mem::offset_of!(ShellExecuteInfoW, icon_or_monitor) == 52);
    assert!(std::mem::offset_of!(ShellExecuteInfoW, h_process) == 56);
};

#[cfg(all(windows, not(target_arch = "x86")))]
const _: () = {
    assert!(std::mem::size_of::<ShellExecuteInfoW>() == 112);
    assert!(std::mem::offset_of!(ShellExecuteInfoW, icon_or_monitor) == 96);
    assert!(std::mem::offset_of!(ShellExecuteInfoW, h_process) == 104);
};

/// Launches one of the three literal Settings URIs with `ShellExecuteExW`,
/// verb `open`, `SEE_MASK_FLAG_NO_UI`, and no parameters or working directory.
/// A successful return means only `launched`; it is never maintenance
/// completion.
#[cfg(windows)]
pub(super) fn launch_settings(uri: &str) -> Result<(), OptimizeAuditErrorCode> {
    let probe = launch_settings_probe(uri);
    if probe.succeeded {
        Ok(())
    } else {
        tracing::debug!(
            h_inst_app = probe.h_inst_app,
            last_error = probe.last_error,
            h_process = probe.h_process,
            "Settings ShellExecuteExW failed"
        );
        Err(OptimizeAuditErrorCode::AdapterOperationFailed)
    }
}

#[cfg(windows)]
pub(super) fn launch_settings_probe(uri: &str) -> SettingsLaunchProbe {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::UI::Shell::SEE_MASK_FLAG_NO_UI;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteExW(exec_info: *mut ShellExecuteInfoW) -> i32;
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
    let verb = wide("open");
    let file = wide(uri);
    let mut info = ShellExecuteInfoW {
        cb_size: std::mem::size_of::<ShellExecuteInfoW>() as u32,
        f_mask: SEE_MASK_FLAG_NO_UI,
        hwnd: std::ptr::null_mut(),
        lp_verb: verb.as_ptr(),
        lp_file: file.as_ptr(),
        lp_parameters: std::ptr::null(),
        lp_directory: std::ptr::null(),
        n_show: SW_SHOWNORMAL,
        h_inst_app: std::ptr::null_mut(),
        lp_id_list: std::ptr::null_mut(),
        lp_class: std::ptr::null(),
        hkey_class: std::ptr::null_mut(),
        dw_hot_key: 0,
        icon_or_monitor: std::ptr::null_mut(),
        h_process: std::ptr::null_mut(),
    };
    // SAFETY: `info` remains valid for the duration of the call and contains
    // only NUL-terminated wide string pointers owned by this frame.
    let ok = unsafe { ShellExecuteExW(&mut info) };
    // Packed x86 fields are not aligned; copy them out without forming
    // references.
    let h_inst_app = unsafe { std::ptr::addr_of!(info.h_inst_app).read_unaligned() } as isize;
    let h_process = unsafe { std::ptr::addr_of!(info.h_process).read_unaligned() } as isize;
    // SAFETY: `GetLastError` reads the calling thread's last-error code.
    let last_error = unsafe { GetLastError() };
    SettingsLaunchProbe {
        succeeded: ok != 0,
        h_inst_app,
        last_error,
        h_process,
    }
}

#[cfg(not(windows))]
pub(super) fn launch_settings(_uri: &str) -> Result<(), OptimizeAuditErrorCode> {
    Err(OptimizeAuditErrorCode::AdapterOperationFailed)
}

pub(super) fn dispatch_settings_handoff(uri: &'static str) -> AdapterEvidence {
    match launch_settings(uri) {
        Ok(()) => AdapterEvidence::success(),
        Err(code) => AdapterEvidence::failure(code),
    }
}
