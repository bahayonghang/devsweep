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
    let sizing_cancel = cancel.clone();
    let (result, measured) = run_joined_mta(move || {
        crate::filesystem::estimate_streamed_trees(
            crate::filesystem::DEFAULT_SIZE_ENTRY_BUDGET,
            cancel.as_ref(),
            |submit| enumerate_on_mta(cancel.as_ref(), submit),
        )
    })
    .unwrap_or_else(|reason| (MtaResult::failed(reason), Default::default()));
    let sizes = finish_streamed_sizes(
        &result.packages,
        measured,
        observed_at_unix_ms,
        sizing_cancel.as_ref(),
    );
    inventory_from_msix(result, sizes)
}

#[cfg(windows)]
fn inventory_from_msix(result: MtaResult, sizes: Vec<SoftwareSizeEvidence>) -> SourceInventory {
    let source = SoftwareSourceId::MsixCurrentUser;
    let complete = result.state == SoftwareSourceState::Available;
    let evidence = if complete {
        SoftwareSourceEvidence::available(source.clone())
    } else {
        SoftwareSourceEvidence::unavailable(source.clone(), result.state, result.reason_code)
    };
    let observations = result
        .packages
        .into_iter()
        .zip(sizes)
        .map(|(package, size)| {
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
                size,
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

/// Runs one Software WinRT job on the same joined, counted MTA worker used by
/// inventory. The closure is invoked only after successful MTA initialization.
#[cfg(windows)]
pub(crate) fn run_joined_mta<T, F>(job: F) -> Result<T, &'static str>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    use std::sync::atomic::Ordering;
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};

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

            // SAFETY: this dedicated worker has not initialized COM/WinRT.
            if unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_err() {
                return Err("winrt_mta_initialize_failed");
            }
            struct RoGuard;
            impl Drop for RoGuard {
                fn drop(&mut self) {
                    // SAFETY: paired with successful initialization above.
                    unsafe { RoUninitialize() };
                }
            }
            let _ro = RoGuard;
            Ok(job())
        })
        .map_err(|_| "mta_worker_spawn_failed")?;
    worker.join().map_err(|_| "mta_worker_panicked")?
}

#[cfg(windows)]
struct MtaResult {
    state: SoftwareSourceState,
    reason_code: &'static str,
    packages: Vec<EnumeratedMsixPackage>,
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
struct EnumeratedMsixPackage {
    package_full_name: String,
    display_name: Option<String>,
    publisher: Option<String>,
    version: String,
    installed_path: Option<String>,
    hidden: bool,
    system: bool,
    dependency: bool,
    stub: bool,
    healthy: bool,
}

#[cfg(windows)]
fn finish_streamed_sizes(
    packages: &[EnumeratedMsixPackage],
    measured: crate::filesystem::StreamedSizeEstimates,
    observed_at_unix_ms: u64,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Vec<SoftwareSizeEvidence> {
    let mut sizes = vec![
        SoftwareSizeEvidence::Unknown {
            reason_code: "installed_path_unavailable".to_string(),
        };
        packages.len()
    ];
    for (index, estimate) in measured.estimates {
        let estimate = estimate.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        sizes[index] = size_from_installed_estimate(estimate, observed_at_unix_ms);
    }
    if measured.canceled_after_produce {
        // Previously every root first saw cancellation after enumeration. The
        // unchanged cancel-aware walker returns Unknown without inspecting paths.
        return measure_msix_installed_paths(packages, observed_at_unix_ms, cancel);
    }
    sizes
}

#[cfg(windows)]
fn measure_msix_installed_paths(
    packages: &[EnumeratedMsixPackage],
    observed_at_unix_ms: u64,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Vec<SoftwareSizeEvidence> {
    use std::path::PathBuf;

    use crate::filesystem::{DEFAULT_SIZE_ENTRY_BUDGET, estimate_trees_with_budget_and_cancel};

    let mut evidence = vec![
        SoftwareSizeEvidence::Unknown {
            reason_code: "installed_path_unavailable".to_string(),
        };
        packages.len()
    ];
    let mut paths = Vec::new();
    let mut indices = Vec::new();
    for (index, package) in packages.iter().enumerate() {
        if let Some(path) = package.installed_path.as_deref() {
            indices.push(index);
            paths.push(PathBuf::from(path));
        }
    }
    let estimates =
        estimate_trees_with_budget_and_cancel(&paths, DEFAULT_SIZE_ENTRY_BUDGET, cancel);
    for (index, estimate) in indices.into_iter().zip(estimates) {
        evidence[index] = size_from_installed_estimate(estimate, observed_at_unix_ms);
    }
    evidence
}

#[cfg(windows)]
fn size_from_installed_estimate(
    estimate: crate::filesystem::SizeEstimate,
    observed_at_unix_ms: u64,
) -> SoftwareSizeEvidence {
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

#[cfg(windows)]
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

#[cfg(windows)]
fn enumerate_on_mta(
    cancel: Option<&Arc<FlagCancelObserver>>,
    submit_path: &mut dyn FnMut(usize, &std::path::Path),
) -> MtaResult {
    use windows::{
        ApplicationModel::PackageSignatureKind, Management::Deployment::PackageManager,
        core::HSTRING,
    };

    use crate::process::CancelObserver;

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
                return Err(windows::core::Error::from_thread());
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
                if let Some(path) = package.installed_path.as_deref() {
                    submit_path(raw.len(), std::path::Path::new(path));
                }
                raw.push(package);
            }
            Err(_) => partial = true,
        }
    }

    finish_enumeration(raw, partial)
}

#[cfg(windows)]
fn finish_enumeration(raw: Vec<RawPackage>, partial: bool) -> MtaResult {
    use std::collections::BTreeSet;

    let dependency_names = raw
        .iter()
        .flat_map(|package| package.dependency_names.iter().cloned())
        .collect::<BTreeSet<_>>();
    let packages = raw
        .into_iter()
        .map(|package| {
            let dependency = package.framework
                || package.resource
                || package.optional
                || dependency_names.contains(&package.full_name);
            let hidden = package.display_name.is_none();
            EnumeratedMsixPackage {
                package_full_name: package.full_name,
                display_name: package.display_name,
                publisher: package.publisher,
                version: package.version,
                installed_path: package.installed_path,
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
    iterable: &windows_collections::IIterable<T>,
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
    view: &windows_collections::IVectorView<T>,
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
        return Err(windows::core::Error::from_thread());
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
pub(crate) fn current_process_sid() -> Result<String, ()> {
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
pub(crate) fn current_process_sid() -> Result<String, ()> {
    Err(())
}

#[cfg(all(test, windows))]
pub(super) fn mta_test_guard() -> std::sync::MutexGuard<'static, ()> {
    // Worker-count assertions observe a process-wide counter. Hold this across
    // the entire test, including its post-join assertions, so sibling tests
    // cannot contribute unrelated workers to that observation.
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TEST_LOCK.lock().expect("MTA test lock")
}

#[cfg(all(test, windows))]
pub(crate) fn active_mta_workers() -> usize {
    use std::sync::atomic::Ordering;

    ACTIVE_MTA_WORKERS.load(Ordering::SeqCst)
}

#[cfg(all(test, windows))]
mod phase_benchmark {
    use super::*;
    use std::{
        collections::HashSet,
        path::{Path, PathBuf},
        sync::{
            Mutex,
            atomic::{AtomicU64, AtomicUsize, Ordering},
        },
        time::{Instant, SystemTime},
    };

    use crate::filesystem::{
        InspectedEntry, PathReparseProbe, ReparseProbeResult, SystemPathReparseProbe,
    };

    #[derive(Default)]
    struct CountingProbe {
        inspections: AtomicUsize,
        fallbacks: AtomicUsize,
        extra_probes: AtomicUsize,
        native_ns: AtomicU64,
        paths: Mutex<HashSet<PathBuf>>,
    }

    impl PathReparseProbe for CountingProbe {
        fn inspect_entry(&self, path: &Path) -> Option<InspectedEntry> {
            self.inspections.fetch_add(1, Ordering::Relaxed);
            self.paths
                .lock()
                .expect("diagnostic path set")
                .insert(path.to_owned());
            let started = Instant::now();
            let result = SystemPathReparseProbe.inspect_entry(path);
            self.native_ns.fetch_add(
                u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
                Ordering::Relaxed,
            );
            if result.is_none() {
                self.fallbacks.fetch_add(1, Ordering::Relaxed);
            }
            result
        }

        fn probe(&self, path: &Path) -> ReparseProbeResult {
            self.extra_probes.fetch_add(1, Ordering::Relaxed);
            let started = Instant::now();
            let result = SystemPathReparseProbe.probe(path);
            self.native_ns.fetch_add(
                u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
                Ordering::Relaxed,
            );
            result
        }
    }

    /// Opt-in diagnostic only: reads the current user's package inventory and
    /// installed-location metadata. Never install/remove packages or alter data.
    #[test]
    #[ignore = "read-only native host benchmark; run explicitly in a quiet measurement slot"]
    fn profile_msix_inventory_phases() {
        let _mta_test = mta_test_guard();
        let started = Instant::now();
        let packages = run_joined_mta(|| enumerate_on_mta(None, &mut |_, _| {}))
            .expect("joined MTA inventory");
        let enumeration_ms = started.elapsed().as_secs_f64() * 1000.0;
        let installed_paths: Vec<_> = packages
            .packages
            .iter()
            .filter_map(|package| package.installed_path.as_deref())
            .collect();
        let unique_paths: std::collections::BTreeSet<_> = installed_paths.iter().copied().collect();
        let observed = crate::software::unix_ms(SystemTime::now());
        let sizing_started = Instant::now();
        let sizes = measure_msix_installed_paths(&packages.packages, observed, None);
        let sizing_ms = sizing_started.elapsed().as_secs_f64() * 1000.0;
        let available = sizes
            .iter()
            .filter(|size| matches!(size, SoftwareSizeEvidence::Available { .. }))
            .count();
        let partial = sizes
            .iter()
            .filter(|size| matches!(size, SoftwareSizeEvidence::Partial { .. }))
            .count();
        println!(
            "{}",
            serde_json::json!({
                "method": "production MSIX enumeration followed by unchanged installed-path sizing; opt-in native diagnostic",
                "enumeration_ms": enumeration_ms, "sizing_ms": sizing_ms,
                "total_ms": started.elapsed().as_secs_f64() * 1000.0,
                "packages": packages.packages.len(), "source_state": packages.state,
            "installed_paths": installed_paths.len(), "unique_installed_paths": unique_paths.len(),
                "available_sizes": available, "partial_sizes": partial,
                "unknown_sizes": sizes.len() - available - partial,
                "remaining_mta_workers": active_mta_workers(),
            })
        );
        assert_eq!(sizes.len(), packages.packages.len());
        assert_eq!(active_mta_workers(), 0);
    }

    #[test]
    #[ignore = "read-only native metadata-count diagnostic; not a release timing benchmark"]
    fn profile_msix_metadata_calls() {
        let _mta_test = mta_test_guard();
        let packages = run_joined_mta(|| enumerate_on_mta(None, &mut |_, _| {}))
            .expect("joined MTA inventory");
        let probe = CountingProbe::default();
        let mut measured = 0;
        let mut complete = 0;
        // Serial package order makes this a call-count diagnostic only. Each
        // tree retains production fan-out, native checks and per-child budgets.
        for package in &packages.packages {
            if let Some(path) = package.installed_path.as_deref() {
                let estimate = crate::filesystem::estimate_tree_with_budget_and_cancel_and_probe(
                    Path::new(path),
                    crate::filesystem::DEFAULT_SIZE_ENTRY_BUDGET,
                    None,
                    &probe,
                );
                measured += 1;
                complete += usize::from(estimate.complete);
            }
        }
        let calls = probe.inspections.load(Ordering::Relaxed);
        let unique = probe.paths.lock().expect("diagnostic path set").len();
        println!(
            "{}",
            serde_json::json!({
                "method": "test-only probe counts, serial package order; not production latency",
                "packages": packages.packages.len(), "measured_paths": measured,
                "complete_sizes": complete, "source_state": packages.state,
                "inspection_calls": calls, "unique_inspection_paths": unique,
                "repeat_inspections": calls - unique,
                "combined_inspection_fallbacks": probe.fallbacks.load(Ordering::Relaxed),
                "extra_reparse_probes": probe.extra_probes.load(Ordering::Relaxed),
                "summed_native_inspection_ms": probe.native_ns.load(Ordering::Relaxed) as f64 / 1_000_000.0,
                "remaining_mta_workers": active_mta_workers(),
            })
        );
        assert_eq!(active_mta_workers(), 0);
    }
}

#[cfg(all(test, windows))]
mod overlap_tests {
    use super::*;
    use crate::filesystem::{
        DEFAULT_SIZE_ENTRY_BUDGET, SizeEstimate, StreamedSizeEstimates, estimate_streamed_trees,
    };
    use std::{
        path::Path,
        sync::atomic::{AtomicBool, Ordering},
    };

    fn raw(name: &str, path: Option<String>, dependencies: &[&str]) -> RawPackage {
        RawPackage {
            full_name: name.to_string(),
            display_name: Some(name.to_string()),
            publisher: Some("fixture".to_string()),
            version: "1.0.0.0".to_string(),
            installed_path: path,
            framework: false,
            resource: false,
            optional: false,
            stub: false,
            healthy: true,
            system: false,
            dependency_names: dependencies
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            dependency_graph_partial: false,
        }
    }

    #[test]
    fn late_dependency_and_partial_source_are_finalized_after_streamed_measurement() {
        let _mta_test = mta_test_guard();
        let fixture = tempfile::tempdir().expect("MSIX path fixture");
        let installed = fixture.path().join("installed");
        std::fs::write(&installed, b"sized fixture").expect("installed fixture");
        let path = installed.to_string_lossy().into_owned();
        let (result, measured) = run_joined_mta(move || {
            estimate_streamed_trees(DEFAULT_SIZE_ENTRY_BUDGET, None, |submit| {
                submit(0, Path::new(&path));
                // The later package makes the first a dependency. The incomplete
                // source must apply to it even if its size has already finished.
                finish_enumeration(
                    vec![raw("base", Some(path), &[]), raw("app", None, &["base"])],
                    true,
                )
            })
        })
        .expect("MTA joined");
        assert_eq!(active_mta_workers(), 0);
        let expected = measure_msix_installed_paths(&result.packages, 123, None);
        let sizes = finish_streamed_sizes(&result.packages, measured, 123, None);
        assert_eq!(sizes, expected);
        let inventory = inventory_from_msix(result, sizes);
        assert_eq!(inventory.evidence[0].state, SoftwareSourceState::Partial);
        assert_eq!(inventory.observations.len(), 2);
        assert!(inventory.observations[0].flags.dependency);
        assert!(!inventory.observations[1].flags.dependency);
        assert!(
            inventory
                .observations
                .iter()
                .all(|row| row.flags.source_incomplete)
        );
    }

    #[test]
    fn cancellation_at_enum_end_discards_completed_sizes_but_tail_cancel_does_not() {
        let packages = finish_enumeration(
            vec![
                raw("installed", Some("not inspected".to_string()), &[]),
                raw("missing", None, &[]),
            ],
            true,
        )
        .packages;
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();
        for canceled_after_produce in [true, false] {
            let measured = StreamedSizeEstimates {
                canceled_after_produce,
                estimates: vec![(0, Ok(SizeEstimate::trusted(123, None)))],
            };
            let sizes = finish_streamed_sizes(&packages, measured, 123, Some(&cancel));
            if canceled_after_produce {
                assert_eq!(
                    sizes,
                    measure_msix_installed_paths(&packages, 123, Some(&cancel))
                );
                assert!(
                    matches!(&sizes[0], SoftwareSizeEvidence::Unknown { reason_code } if reason_code == "installed_path_measurement_unavailable")
                );
            } else {
                assert!(matches!(
                    sizes[0],
                    SoftwareSizeEvidence::Available {
                        value_bytes: 123,
                        ..
                    }
                ));
            }
            assert!(
                matches!(&sizes[1], SoftwareSizeEvidence::Unknown { reason_code } if reason_code == "installed_path_unavailable")
            );
        }
    }

    #[test]
    fn streamed_panic_is_resumed_after_join_even_when_canceled() {
        let _mta_test = mta_test_guard();
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();
        let joined = Arc::new(AtomicBool::new(false));
        let caller_joined = Arc::clone(&joined);
        let outcome = std::panic::catch_unwind(move || {
            let measured = run_joined_mta(|| StreamedSizeEstimates {
                canceled_after_produce: true,
                estimates: vec![(0, Err(Box::new("controlled size panic")))],
            })
            .expect("MTA joined");
            caller_joined.store(true, Ordering::SeqCst);
            let packages = finish_enumeration(
                vec![raw("installed", Some("not inspected".to_string()), &[])],
                true,
            )
            .packages;
            finish_streamed_sizes(&packages, measured, 123, Some(&cancel));
        });
        assert!(joined.load(Ordering::SeqCst));
        assert_eq!(active_mta_workers(), 0);
        assert_eq!(
            outcome
                .expect_err("size panic preserved")
                .downcast_ref::<&str>(),
            Some(&"controlled size panic")
        );
    }

    #[test]
    fn enumeration_panic_keeps_the_existing_joined_mta_failure() {
        let _mta_test = mta_test_guard();
        let fixture = tempfile::tempdir().expect("MSIX path fixture");
        let path = fixture.path().join("installed");
        std::fs::write(&path, b"fixture").expect("installed fixture");
        let outcome = run_joined_mta(move || {
            estimate_streamed_trees(DEFAULT_SIZE_ENTRY_BUDGET, None, |submit| {
                submit(0, &path);
                submit(1, &path);
                panic!("controlled enumeration panic");
            });
        });
        assert_eq!(outcome, Err("mta_worker_panicked"));
        assert_eq!(active_mta_workers(), 0);
    }
}
