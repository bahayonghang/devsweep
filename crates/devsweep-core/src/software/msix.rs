//! Current-process-SID MSIX inventory on one joined MTA worker.

use std::sync::Arc;

use super::{SourceInventory, model::*};
use crate::process::FlagCancelObserver;

pub(super) fn inventory(
    observed_at_unix_ms: u64,
    cancel: Option<Arc<FlagCancelObserver>>,
) -> SourceInventory {
    #[cfg(windows)]
    {
        windows_inventory(observed_at_unix_ms, cancel)
    }
    #[cfg(not(windows))]
    {
        let _ = (observed_at_unix_ms, cancel);
        SourceInventory::one(
            SoftwareSourceEvidence::unavailable(
                SoftwareSourceId::MsixCurrentUser,
                SoftwareSourceState::Unsupported,
                "windows_only_source",
            ),
            Vec::new(),
        )
    }
}

#[cfg(windows)]
fn windows_inventory(
    observed_at_unix_ms: u64,
    cancel: Option<Arc<FlagCancelObserver>>,
) -> SourceInventory {
    use std::sync::atomic::Ordering;

    let source = SoftwareSourceId::MsixCurrentUser;
    let worker = std::thread::Builder::new()
        .name("devsweep-software-mta".to_string())
        .spawn(move || {
            ACTIVE_MTA_WORKERS.fetch_add(1, Ordering::SeqCst);
            struct ActiveGuard;
            impl Drop for ActiveGuard {
                fn drop(&mut self) {
                    ACTIVE_MTA_WORKERS.fetch_sub(1, Ordering::SeqCst);
                }
            }
            let _active = ActiveGuard;
            enumerate_on_mta(observed_at_unix_ms, cancel.as_ref())
        });
    let result = match worker {
        Ok(worker) => match worker.join() {
            Ok(result) => result,
            Err(_) => MtaResult::failed("mta_worker_panicked"),
        },
        Err(_) => MtaResult::failed("mta_worker_spawn_failed"),
    };
    let complete = result.state == SoftwareSourceState::Available;
    let evidence = if complete {
        SoftwareSourceEvidence::available(source.clone())
    } else {
        SoftwareSourceEvidence::unavailable(source.clone(), result.state, result.reason_code)
    };
    let observations = result
        .packages
        .into_iter()
        .map(|package| {
            let protected = super::is_protected_product(
                package.display_name.as_deref(),
                package.publisher.as_deref(),
            );
            SoftwareObservation {
                identity: SoftwareIdentity::Msix {
                    package_full_name: package.package_full_name,
                },
                scope: SoftwareScope::CurrentUser,
                display_name: package.display_name,
                publisher: package.publisher,
                version: Some(package.version),
                provenance: vec![source.clone()],
                size: package.size,
                flags: EligibilityFlags {
                    protected,
                    source_incomplete: !complete,
                    hidden: package.hidden,
                    system_or_update: package.system,
                    dependency: package.dependency,
                    stub: package.stub,
                    unhealthy: !package.healthy,
                    ..EligibilityFlags::default()
                },
            }
        })
        .collect();
    SourceInventory::one(evidence, observations)
}

#[cfg(windows)]
static ACTIVE_MTA_WORKERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(windows)]
struct MtaResult {
    state: SoftwareSourceState,
    reason_code: &'static str,
    packages: Vec<MsixPackage>,
}

#[cfg(windows)]
impl MtaResult {
    fn failed(reason_code: &'static str) -> Self {
        Self {
            state: SoftwareSourceState::Partial,
            reason_code,
            packages: Vec::new(),
        }
    }
}

#[cfg(windows)]
struct MsixPackage {
    package_full_name: String,
    display_name: Option<String>,
    publisher: Option<String>,
    version: String,
    size: SoftwareSizeEvidence,
    hidden: bool,
    system: bool,
    dependency: bool,
    stub: bool,
    healthy: bool,
}

#[cfg(windows)]
fn enumerate_on_mta(
    observed_at_unix_ms: u64,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> MtaResult {
    use std::{collections::BTreeSet, path::Path};

    use windows::{
        ApplicationModel::PackageSignatureKind,
        Management::Deployment::PackageManager,
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        core::HSTRING,
    };

    use crate::{filesystem::estimate_tree_with_budget_and_cancel, process::CancelObserver};

    // SAFETY: this dedicated thread has not initialized COM/WinRT elsewhere.
    if unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_err() {
        return MtaResult::failed("winrt_mta_initialize_failed");
    }
    struct RoGuard;
    impl Drop for RoGuard {
        fn drop(&mut self) {
            // SAFETY: paired with the successful RoInitialize on this thread.
            unsafe { RoUninitialize() };
        }
    }
    let _ro = RoGuard;

    if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
        return MtaResult::failed("canceled");
    }
    let sid = match current_process_sid() {
        Ok(sid) => sid,
        Err(_) => return MtaResult::failed("current_user_sid_unavailable"),
    };
    let manager = match PackageManager::new() {
        Ok(manager) => manager,
        Err(_) => return MtaResult::failed("package_manager_unavailable"),
    };
    let iterable = match manager.FindPackagesByUserSecurityId(&HSTRING::from(sid)) {
        Ok(packages) => packages,
        Err(_) => return MtaResult::failed("current_user_msix_query_failed"),
    };

    struct RawPackage {
        full_name: String,
        display_name: Option<String>,
        publisher: Option<String>,
        version: String,
        installed_path: Option<String>,
        framework: bool,
        resource: bool,
        optional: bool,
        stub: bool,
        healthy: bool,
        system: bool,
        dependency_names: Vec<String>,
        dependency_graph_partial: bool,
    }

    let (packages, mut partial) = collect_iterable(&iterable, cancel);
    let mut raw = Vec::new();
    for package in packages {
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            partial = true;
            break;
        }
        let extracted = (|| -> windows::core::Result<RawPackage> {
            let id = package.Id()?;
            let full_name = bounded_hstring(id.FullName()?, 8 * 1024)?;
            if full_name.is_empty() {
                return Err(windows::core::Error::from_win32());
            }
            let version = id.Version()?;
            let status = package.Status()?;
            let dependencies = package.Dependencies()?;
            let (dependencies, dependency_graph_partial) =
                collect_vector_view(&dependencies, cancel);
            let mut dependency_names = Vec::with_capacity(dependencies.len());
            for dependency in dependencies {
                dependency_names.push(bounded_hstring(dependency.Id()?.FullName()?, 8 * 1024)?);
            }
            Ok(RawPackage {
                full_name,
                display_name: optional_bounded(package.DisplayName(), 32 * 1024),
                publisher: optional_bounded(package.PublisherDisplayName(), 32 * 1024),
                version: format!(
                    "{}.{}.{}.{}",
                    version.Major, version.Minor, version.Build, version.Revision
                ),
                installed_path: package
                    .InstalledPath()
                    .ok()
                    .and_then(|path| bounded_hstring(path, 32 * 1024).ok())
                    .filter(|path| !path.is_empty()),
                framework: package.IsFramework()?,
                resource: package.IsResourcePackage()?,
                optional: package.IsOptional()?,
                stub: package.IsStub()?,
                healthy: status.VerifyIsOK()?,
                system: package.SignatureKind()? == PackageSignatureKind::System,
                dependency_names,
                dependency_graph_partial,
            })
        })();
        match extracted {
            Ok(package) => {
                partial |= package.dependency_graph_partial;
                raw.push(package);
            }
            Err(_) => partial = true,
        }
    }

    let dependency_names = raw
        .iter()
        .flat_map(|package| package.dependency_names.iter().cloned())
        .collect::<BTreeSet<_>>();
    let packages = raw
        .into_iter()
        .map(|package| {
            let size = match package.installed_path.as_deref() {
                Some(path) => {
                    let estimate = estimate_tree_with_budget_and_cancel(
                        Path::new(path),
                        crate::filesystem::DEFAULT_SIZE_ENTRY_BUDGET,
                        cancel,
                    );
                    match (estimate.logical_bytes, estimate.complete) {
                        (Some(value_bytes), true) => SoftwareSizeEvidence::Available {
                            value_bytes,
                            basis: SoftwareSizeBasis::MeasuredInstalledLocation,
                            source_code: SoftwareSizeSourceCode::MsixInstalledPath,
                            observed_at_unix_ms,
                        },
                        (Some(lower_bound_bytes), false) => SoftwareSizeEvidence::Partial {
                            lower_bound_bytes,
                            basis: SoftwareSizeBasis::MeasuredInstalledLocation,
                            source_code: SoftwareSizeSourceCode::MsixInstalledPath,
                            reason_code: "bounded_walk_incomplete".to_string(),
                            observed_at_unix_ms,
                        },
                        (None, _) => SoftwareSizeEvidence::Unknown {
                            reason_code: "installed_path_measurement_unavailable".to_string(),
                        },
                    }
                }
                None => SoftwareSizeEvidence::Unknown {
                    reason_code: "installed_path_unavailable".to_string(),
                },
            };
            let dependency = package.framework
                || package.resource
                || package.optional
                || dependency_names.contains(&package.full_name);
            let hidden = package.display_name.is_none();
            MsixPackage {
                package_full_name: package.full_name,
                display_name: package.display_name,
                publisher: package.publisher,
                version: package.version,
                size,
                hidden,
                system: package.system,
                dependency,
                stub: package.stub,
                healthy: package.healthy,
            }
        })
        .collect();
    MtaResult {
        state: if partial {
            SoftwareSourceState::Partial
        } else {
            SoftwareSourceState::Available
        },
        reason_code: if partial {
            "current_user_msix_incomplete"
        } else {
            "complete"
        },
        packages,
    }
}

#[cfg(windows)]
fn collect_iterable<T>(
    iterable: &windows::Foundation::Collections::IIterable<T>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> (Vec<T>, bool)
where
    T: windows::core::RuntimeType + 'static,
{
    use crate::process::CancelObserver;

    let iterator = match iterable.First() {
        Ok(iterator) => iterator,
        Err(_) => return (Vec::new(), true),
    };
    let mut items = Vec::new();
    loop {
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            return (items, true);
        }
        match iterator.HasCurrent() {
            Ok(false) => return (items, false),
            Ok(true) => {}
            Err(_) => return (items, true),
        }
        match iterator.Current() {
            Ok(item) => items.push(item),
            Err(_) => return (items, true),
        }
        match iterator.MoveNext() {
            Ok(true) => {}
            Ok(false) => return (items, false),
            Err(_) => return (items, true),
        }
    }
}

#[cfg(windows)]
fn collect_vector_view<T>(
    view: &windows::Foundation::Collections::IVectorView<T>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> (Vec<T>, bool)
where
    T: windows::core::RuntimeType + 'static,
{
    use crate::process::CancelObserver;

    let size = match view.Size() {
        Ok(size) => size,
        Err(_) => return (Vec::new(), true),
    };
    let mut items = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
    for index in 0..size {
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            return (items, true);
        }
        match view.GetAt(index) {
            Ok(item) => items.push(item),
            Err(_) => return (items, true),
        }
    }
    (items, false)
}

#[cfg(windows)]
fn bounded_hstring(
    value: windows::core::HSTRING,
    max_bytes: usize,
) -> windows::core::Result<String> {
    let value = value.to_string_lossy();
    if value.len() > max_bytes {
        return Err(windows::core::Error::from_win32());
    }
    Ok(value)
}

#[cfg(windows)]
fn optional_bounded(
    value: windows::core::Result<windows::core::HSTRING>,
    max_bytes: usize,
) -> Option<String> {
    value
        .ok()
        .and_then(|value| bounded_hstring(value, max_bytes).ok())
        .filter(|value| !value.is_empty())
}

#[cfg(windows)]
pub(super) fn current_process_sid() -> Result<String, ()> {
    use std::{ffi::c_void, ptr};

    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{
            GetSidIdentifierAuthority, GetSidSubAuthority, GetSidSubAuthorityCount,
            GetTokenInformation, IsValidSid, TOKEN_QUERY, TOKEN_USER, TokenUser,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    struct Token(HANDLE);
    impl Drop for Token {
        fn drop(&mut self) {
            // SAFETY: the handle was returned by OpenProcessToken and is owned here.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    let mut token = ptr::null_mut();
    // SAFETY: output handle pointer is writable and the pseudo-process handle is valid.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(());
    }
    let token = Token(token);
    let mut required = 0;
    // SAFETY: null buffer intentionally queries the required TOKEN_USER length.
    let _ = unsafe { GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required) };
    if required == 0 {
        return Err(());
    }
    let mut buffer = vec![0_u8; usize::try_from(required).map_err(|_| ())?];
    // SAFETY: `buffer` is writable for the advertised required length.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast::<c_void>(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(());
    }
    // SAFETY: successful TokenUser query returned a TOKEN_USER at buffer start.
    let token_user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let sid = token_user.User.Sid;
    // SAFETY: the SID pointer is owned by the live token-information buffer.
    if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
        return Err(());
    }
    // SAFETY: IsValidSid succeeded, so the authority/count pointers are readable.
    let authority = unsafe { &*GetSidIdentifierAuthority(sid) };
    let authority = authority
        .Value
        .iter()
        .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte));
    // SAFETY: IsValidSid succeeded, so the count pointer is readable.
    let count = unsafe { *GetSidSubAuthorityCount(sid) };
    let mut value = format!("S-{}-{authority}", unsafe { *sid.cast::<u8>() });
    for index in 0..u32::from(count) {
        // SAFETY: each index is below the validated SID sub-authority count.
        value.push_str(&format!("-{}", unsafe { *GetSidSubAuthority(sid, index) }));
    }
    Ok(value)
}

#[cfg(not(windows))]
pub(super) fn current_process_sid() -> Result<String, ()> {
    Err(())
}

#[cfg(all(test, windows))]
pub(crate) fn active_mta_workers() -> usize {
    use std::sync::atomic::Ordering;

    ACTIVE_MTA_WORKERS.load(Ordering::SeqCst)
}
