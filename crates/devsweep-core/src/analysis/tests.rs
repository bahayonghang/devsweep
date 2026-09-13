use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::process::FlagCancelObserver;
#[cfg(all(windows, not(debug_assertions)))]
use std::{
    thread,
    time::{Duration, Instant},
};

#[cfg(all(windows, not(debug_assertions)))]
use super::AnalyzeSnapshotV1;
use super::{
    ANALYZE_WORKERS, AnalyzeCompleteness, AnalyzeEvidence, AnalyzeNodeKind, AnalyzeProgressV1,
    AnalyzeWarningClass, Fake250k, MAX_OWNED_BYTES, MAX_STORED_NODES, MapFs, analyze_path,
    analyze_with_fs_and_stats, dir_entry, file_entry, reparse_entry, walker::analyze_with_fs,
};

fn map_tree() -> (PathBuf, MapFs) {
    let root = PathBuf::from("fixture-root");
    let child = root.join("file.bin");
    let link = root.join("reparse");
    let denied = root.join("denied");
    let mut fs = MapFs {
        volume: "vol".to_string(),
        ..MapFs::default()
    };
    fs.meta.insert(root.clone(), Ok(dir_entry("dir:root")));
    fs.meta.insert(child.clone(), Ok(file_entry(100, "file:a")));
    fs.meta
        .insert(root.join("dup-a"), Ok(file_entry(50, "file:dup")));
    fs.meta
        .insert(root.join("dup-b"), Ok(file_entry(50, "file:dup")));
    fs.meta
        .insert(link.clone(), Ok(reparse_entry(7, "reparse:1")));
    fs.meta.insert(denied.clone(), Ok(dir_entry("dir:denied")));
    fs.dirs.insert(
        root.clone(),
        Ok(vec![
            super::walker::DirChild::named("file.bin"),
            super::walker::DirChild::named("dup-a"),
            super::walker::DirChild::named("dup-b"),
            super::walker::DirChild::named("reparse"),
            super::walker::DirChild::named("denied"),
        ]),
    );
    fs.dirs
        .insert(denied, Err(super::walker::FsFail::AccessDenied));
    fs.dirs.insert(
        link,
        Ok(vec![super::walker::DirChild::named("should-not-see")]),
    );
    (root, fs)
}

#[test]
fn complete_snapshot_reconciles_parent_child_totals() {
    let (root, fs) = map_tree();
    let (outcome, stats) =
        analyze_with_fs_and_stats(&root, &fs, None, None, false).expect("analyze fixture");
    let snapshot = outcome.snapshot();
    assert_eq!(snapshot.completeness, AnalyzeCompleteness::Complete);
    assert_eq!(stats.workers, ANALYZE_WORKERS as u8);
    let root_node = snapshot.root_node().expect("root");
    assert_eq!(root_node.immediate_count, 5);
    assert_eq!(root_node.bytes, 100 + 50 + 7);
    let dup_warnings = snapshot
        .warnings
        .iter()
        .filter(|warning| warning.class == AnalyzeWarningClass::DuplicateLink)
        .count();
    assert_eq!(dup_warnings, 1);
    let reparse = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == AnalyzeNodeKind::Reparse)
        .expect("reparse leaf");
    assert_eq!(reparse.immediate_count, 0);
    assert!(reparse.warnings.contains(&AnalyzeWarningClass::Reparse));
    let denied = snapshot
        .nodes
        .iter()
        .find(|node| node.name == "denied")
        .expect("denied");
    assert_eq!(denied.evidence, AnalyzeEvidence::Incomplete);
    assert!(denied.warnings.contains(&AnalyzeWarningClass::AccessDenied));
    assert_eq!(
        snapshot.accounted_owned_bytes,
        snapshot.snapshot_owned_bytes()
    );
    let encoded = serde_json::to_string(snapshot).expect("json");
    assert!(!encoded.contains("CleanupPlan"));
    assert!(!encoded.contains("intent"));
}

#[test]
fn churn_and_cycle_are_distinguishable() {
    let root = PathBuf::from("cycle-root");
    let mut fs = MapFs {
        volume: "vol".to_string(),
        ..MapFs::default()
    };
    fs.meta
        .insert(root.clone(), Ok(dir_entry("dir:cycle-root")));
    fs.meta
        .insert(root.join("gone"), Err(super::walker::FsFail::NotFound));
    fs.meta
        .insert(root.join("loop"), Ok(dir_entry("dir:cycle-root")));
    fs.dirs.insert(
        root.clone(),
        Ok(vec![
            super::walker::DirChild::named("gone"),
            super::walker::DirChild::named("loop"),
        ]),
    );
    let (outcome, _) = analyze_with_fs_and_stats(&root, &fs, None, None, true).expect("analyze");
    let snapshot = outcome.snapshot();
    assert!(
        snapshot
            .warnings
            .iter()
            .any(|warning| warning.class == AnalyzeWarningClass::Churn)
    );
    assert!(
        snapshot
            .warnings
            .iter()
            .any(|warning| warning.class == AnalyzeWarningClass::Cycle)
    );
}

#[test]
fn fake_sparse_file_reports_logical_size_without_real_content() {
    const SPARSE_LOGICAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
    let root = PathBuf::from("sparse-fixture-root");
    let sparse = root.join("sparse-logical.bin");
    let mut fs = MapFs {
        volume: "vol".to_string(),
        ..MapFs::default()
    };
    fs.meta.insert(root.clone(), Ok(dir_entry("dir:sparse")));
    fs.meta.insert(
        sparse,
        Ok(file_entry(SPARSE_LOGICAL_BYTES, "file:sparse-logical")),
    );
    fs.dirs.insert(
        root.clone(),
        Ok(vec![super::walker::DirChild::named("sparse-logical.bin")]),
    );

    let (outcome, _) =
        analyze_with_fs_and_stats(&root, &fs, None, None, true).expect("analyze sparse fixture");
    let snapshot = outcome.snapshot();
    let sparse = snapshot
        .nodes
        .iter()
        .find(|node| node.name == "sparse-logical.bin")
        .expect("sparse node");
    assert_eq!(sparse.bytes, SPARSE_LOGICAL_BYTES);
    assert_eq!(sparse.evidence, AnalyzeEvidence::Complete);
    assert_eq!(
        snapshot.root_node().map(|node| node.bytes),
        Some(SPARSE_LOGICAL_BYTES)
    );
}

#[test]
fn serial_fallback_matches_two_worker_totals() {
    let (root, fs) = map_tree();
    let (parallel, parallel_stats) =
        analyze_with_fs_and_stats(&root, &fs, None, None, false).expect("parallel");
    let (serial, serial_stats) =
        analyze_with_fs_and_stats(&root, &fs, None, None, true).expect("serial");
    assert_eq!(serial_stats.workers, 1);
    assert!(serial_stats.serial_fallback);
    assert_eq!(parallel_stats.workers, 2);
    assert_eq!(
        parallel.snapshot().root_node().map(|node| node.bytes),
        serial.snapshot().root_node().map(|node| node.bytes)
    );
    assert_eq!(
        parallel.snapshot().nodes.len(),
        serial.snapshot().nodes.len()
    );
}

#[test]
fn cancel_joins_and_marks_canceled() {
    let fs = Fake250k::analysis_250k_v1();
    let cancel = FlagCancelObserver::new();
    cancel.request_cancel();
    let (outcome, stats) =
        analyze_with_fs_and_stats(&Fake250k::root(), &fs, Some(&cancel), None, false)
            .expect("canceled analyze");
    assert!(matches!(
        outcome.snapshot().completeness,
        AnalyzeCompleteness::Canceled
    ));
    // Wall-clock around this helper includes waiting for ANALYZE_JOB, which
    // sibling 250k tests may hold for several seconds on slow Windows MSRV
    // runners. Join latency is measured after that lock is acquired.
    let join_ms = stats
        .cancel_to_join_ms
        .expect("pre-requested cancel must record join latency");
    assert!(
        join_ms < 5_000,
        "cancel_to_join_ms={join_ms} exceeded 5s after ANALYZE_JOB was held"
    );
}

#[test]
fn progress_queue_replaces_unsent_batches() {
    let (root, fs) = map_tree();
    let batches = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&batches);
    let mut progress = |batch: AnalyzeProgressV1| {
        captured.lock().expect("lock").push(batch);
    };
    let _ = analyze_with_fs(&root, &fs, None, Some(&mut progress)).expect("progress");
    let batches = batches.lock().expect("lock");
    assert!(batches.iter().all(|batch| batch.changed_nodes.len() <= 256));
    let mut last = 0;
    for batch in batches.iter() {
        assert!(batch.sequence > last);
        last = batch.sequence;
        assert!(batch.queue_depth <= 4);
    }
}

#[test]
fn analysis_250k_v1_stores_exact_node_ceiling() {
    let fs = Fake250k::analysis_250k_v1();
    let (outcome, stats) =
        analyze_with_fs_and_stats(&Fake250k::root(), &fs, None, None, false).expect("250k");
    let snapshot = outcome.snapshot();
    assert_eq!(snapshot.nodes.len(), MAX_STORED_NODES as usize);
    assert_eq!(snapshot.completeness, AnalyzeCompleteness::Complete);
    assert!(snapshot.accounted_owned_bytes <= MAX_OWNED_BYTES);
    assert!(stats.peak_accounted_owned_bytes <= MAX_OWNED_BYTES);
    assert!(stats.workers <= ANALYZE_WORKERS as u8);
    let wide = snapshot
        .nodes
        .iter()
        .find(|node| node.name == "wide")
        .expect("wide directory");
    assert_eq!(wide.immediate_count, 10_000);
}

#[test]
fn analysis_250k_overflow_returns_partial_budget() {
    let fs = Fake250k::with_overflow_node();
    let (outcome, _) =
        analyze_with_fs_and_stats(&Fake250k::root(), &fs, None, None, false).expect("overflow");
    let snapshot = outcome.snapshot();
    assert_eq!(snapshot.nodes.len(), MAX_STORED_NODES as usize);
    assert_eq!(snapshot.completeness, AnalyzeCompleteness::PartialBudget);
    assert!(
        snapshot
            .warnings
            .iter()
            .any(|warning| warning.class == AnalyzeWarningClass::PartialBudget)
    );
    let incomplete = snapshot
        .nodes
        .iter()
        .filter(|node| node.evidence != AnalyzeEvidence::Complete)
        .count();
    assert!(incomplete >= 1);
    assert!(
        !snapshot.nodes.iter().any(|node| {
            node.evidence == AnalyzeEvidence::Complete
                && node.kind == AnalyzeNodeKind::Directory
                && {
                    snapshot.nodes.iter().any(|child| {
                        child.parent_id == Some(node.id)
                            && child.evidence != AnalyzeEvidence::Complete
                    }) && snapshot.warnings.iter().any(|warning| {
                        warning.class == AnalyzeWarningClass::PartialBudget
                            && warning.node_id == Some(node.id)
                    })
                }
        }) || snapshot
            .nodes
            .iter()
            .any(|node| node.evidence == AnalyzeEvidence::Incomplete)
    );
}

#[test]
fn byte_cap_attempt_returns_partial_budget() {
    let fs = Fake250k::with_huge_name((MAX_OWNED_BYTES as usize).saturating_add(1));
    let (outcome, _) =
        analyze_with_fs_and_stats(&Fake250k::root(), &fs, None, None, true).expect("byte cap");
    let snapshot = outcome.snapshot();
    assert_eq!(snapshot.completeness, AnalyzeCompleteness::PartialBudget);
    assert!(snapshot.accounted_owned_bytes <= MAX_OWNED_BYTES);
    assert!(
        snapshot
            .warnings
            .iter()
            .any(|warning| warning.class == AnalyzeWarningClass::PartialBudget)
    );
}

#[test]
fn source_has_no_cleanup_plan_conversion() {
    let sources = [
        include_str!("mod.rs"),
        include_str!("model.rs"),
        include_str!("walker.rs"),
    ];
    for source in sources {
        assert!(!source.contains("CleanupPlan"));
        assert!(!source.contains("from_analyze"));
        assert!(!source.contains("into_cleanup"));
    }
}

#[cfg(windows)]
fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

#[cfg(unix)]
fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_sparse_file(path: &Path, logical_bytes: u64) -> std::io::Result<()> {
    use std::{ffi::c_void, fs::OpenOptions, os::windows::io::AsRawHandle};
    use windows_sys::Win32::Foundation::HANDLE;

    // CTL_CODE(FILE_DEVICE_FILE_SYSTEM, 49, METHOD_BUFFERED, FILE_SPECIAL_ACCESS).
    const FSCTL_SET_SPARSE: u32 = 590_020;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn DeviceIoControl(
            device: HANDLE,
            control_code: u32,
            input: *const c_void,
            input_size: u32,
            output: *mut c_void,
            output_size: u32,
            bytes_returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
    }

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(path)?;
    let mut bytes_returned = 0;
    let ok = unsafe {
        DeviceIoControl(
            file.as_raw_handle() as HANDLE,
            FSCTL_SET_SPARSE,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &raw mut bytes_returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    file.set_len(logical_bytes)
}

#[cfg(windows)]
struct DeniedDirectoryGuard {
    path: PathBuf,
}

#[cfg(windows)]
impl DeniedDirectoryGuard {
    fn create(path: &Path) -> std::io::Result<Self> {
        create_directory_with_sddl(path, "D:P(A;OICI;FA;;;WD)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)")?;
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    fn deny_read_attributes(&self) -> std::io::Result<()> {
        set_directory_sddl(
            &self.path,
            "D:P(D;OICI;GRGX;;;WD)(A;OICI;FA;;;WD)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)",
        )
    }
}

#[cfg(windows)]
impl Drop for DeniedDirectoryGuard {
    fn drop(&mut self) {
        let _ = set_directory_sddl(
            &self.path,
            "D:P(A;OICI;FA;;;WD)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)",
        );
    }
}

#[cfg(windows)]
fn create_directory_with_sddl(path: &Path, sddl: &str) -> std::io::Result<()> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::{
        Foundation::LocalFree, Security::SECURITY_ATTRIBUTES, Storage::FileSystem::CreateDirectoryW,
    };

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
            string_security_descriptor: *const u16,
            string_sd_revision: u32,
            security_descriptor: *mut *mut c_void,
            security_descriptor_size: *mut u32,
        ) -> i32;
    }

    let wide_sddl = sddl
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut descriptor = std::ptr::null_mut();
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide_sddl.as_ptr(),
            1,
            &raw mut descriptor,
            std::ptr::null_mut(),
        )
    };
    if converted == 0 {
        return Err(std::io::Error::last_os_error());
    }

    let wide_path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let created = unsafe { CreateDirectoryW(wide_path.as_ptr(), &raw const attributes) };
    unsafe {
        let _ = LocalFree(descriptor);
    }
    if created == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn set_directory_sddl(path: &Path, sddl: &str) -> std::io::Result<()> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Foundation::LocalFree;

    const DACL_SECURITY_INFORMATION: u32 = 0x0000_0004;
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
            string_security_descriptor: *const u16,
            string_sd_revision: u32,
            security_descriptor: *mut *mut c_void,
            security_descriptor_size: *mut u32,
        ) -> i32;
        fn SetFileSecurityW(
            file_name: *const u16,
            security_information: u32,
            security_descriptor: *const c_void,
        ) -> i32;
    }

    let wide_sddl = sddl
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut descriptor = std::ptr::null_mut();
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide_sddl.as_ptr(),
            1,
            &raw mut descriptor,
            std::ptr::null_mut(),
        )
    };
    if converted == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let wide_path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let changed =
        unsafe { SetFileSecurityW(wide_path.as_ptr(), DACL_SECURITY_INFORMATION, descriptor) };
    unsafe {
        let _ = LocalFree(descriptor);
    }
    if changed == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
#[test]
fn native_sparse_file_reports_complete_logical_size() {
    const SPARSE_LOGICAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
    let fixture = tempfile::TempDir::new().expect("temp");
    let root = fixture.path().join("native-sparse-root");
    std::fs::create_dir_all(&root).expect("root");
    create_sparse_file(&root.join("native-sparse.bin"), SPARSE_LOGICAL_BYTES)
        .expect("create sparse file");

    let outcome = analyze_path(&root, None, None).expect("native sparse analyze");
    let snapshot = outcome.snapshot();
    let sparse = snapshot
        .nodes
        .iter()
        .find(|node| node.name == "native-sparse.bin")
        .expect("native sparse node");
    assert_eq!(sparse.bytes, SPARSE_LOGICAL_BYTES);
    assert_eq!(sparse.evidence, AnalyzeEvidence::Complete);
    assert!(sparse.warnings.is_empty());
    assert_eq!(
        snapshot.root_node().map(|node| node.bytes),
        Some(SPARSE_LOGICAL_BYTES)
    );
}

#[test]
fn native_reparse_is_a_leaf() {
    let fixture = tempfile::TempDir::new().expect("temp");
    let root = fixture.path().join("root");
    std::fs::create_dir_all(root.join("outside")).expect("outside");
    std::fs::write(root.join("outside/big.bin"), vec![0u8; 64]).expect("file");
    std::fs::create_dir_all(&root).expect("root");
    let link = root.join("reparse");
    if create_dir_symlink(&root.join("outside"), &link).is_err() {
        return;
    }
    let outcome = analyze_path(&root, None, None).expect("native analyze");
    let snapshot = outcome.snapshot();
    let reparse = snapshot
        .nodes
        .iter()
        .find(|node| node.name == "reparse")
        .expect("reparse node");
    assert_eq!(reparse.kind, AnalyzeNodeKind::Reparse);
    assert_eq!(reparse.immediate_count, 0);
    assert!(
        !snapshot
            .nodes
            .iter()
            .any(|node| node.name == "big.bin" && node.parent_id == Some(reparse.id))
    );
}

#[cfg(windows)]
#[test]
fn native_live_churn_distinguishes_deleted_and_growing_files_and_denied_branch() {
    use std::{
        fs::{self, OpenOptions},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
    };

    const STABLE_DIRECTORIES: u32 = 60;
    const FILES_PER_DIRECTORY: u32 = 100;
    const CHURN_FILES: u32 = 64;
    const INITIAL_GROWTH_BYTES: u64 = 4_096;

    let fixture = tempfile::TempDir::new().expect("native churn fixture");
    let root = fixture.path().join("analysis-native-churn-v1");
    fs::create_dir_all(&root).expect("root");
    for dir in 0..STABLE_DIRECTORIES {
        let path = root.join(format!("d{dir:03}"));
        fs::create_dir_all(&path).expect("stable directory");
        for file in 0..FILES_PER_DIRECTORY {
            fs::File::create(path.join(format!("f{file:03}.dat"))).expect("stable file");
        }
    }

    let deleted_dir = root.join("z-deleted");
    let growing_dir = root.join("z-growing");
    fs::create_dir_all(&deleted_dir).expect("deleted directory");
    fs::create_dir_all(&growing_dir).expect("growing directory");
    let deleted_paths = (0..CHURN_FILES)
        .map(|index| deleted_dir.join(format!("deleted-{index:03}.dat")))
        .collect::<Vec<_>>();
    let growing_paths = (0..CHURN_FILES)
        .map(|index| growing_dir.join(format!("growing-{index:03}.dat")))
        .collect::<Vec<_>>();
    for path in &deleted_paths {
        fs::write(path, [1u8]).expect("deleted fixture file");
    }
    for path in &growing_paths {
        create_sparse_file(path, INITIAL_GROWTH_BYTES).expect("growing sparse file");
    }

    let denied_dir = root.join("y-denied");
    let denied_guard = DeniedDirectoryGuard::create(&denied_dir).expect("create denied ACL");
    fs::write(denied_dir.join("excluded.bin"), vec![7u8; 64 * 1024]).expect("denied content");
    denied_guard
        .deny_read_attributes()
        .expect("deny read attributes");

    let stop = Arc::new(AtomicBool::new(false));
    let listed_deleted = Arc::new(AtomicBool::new(false));
    let mutator_stop = Arc::clone(&stop);
    let mutator_listed = Arc::clone(&listed_deleted);
    let mutator = thread::spawn(move || {
        while !mutator_listed.load(Ordering::Relaxed) {
            thread::yield_now();
        }
        // z-* children are queued behind the 6,000 stable files after listing.
        for path in deleted_paths {
            let _ = fs::remove_file(path);
        }
        let growing_files = growing_paths
            .iter()
            .map(|path| OpenOptions::new().write(true).open(path))
            .collect::<std::io::Result<Vec<_>>>()
            .expect("open growing files");
        let mut logical_bytes = INITIAL_GROWTH_BYTES;
        while !mutator_stop.load(Ordering::Relaxed) {
            logical_bytes = logical_bytes.saturating_add(4_096);
            for file in &growing_files {
                file.set_len(logical_bytes).expect("grow sparse file");
            }
            thread::yield_now();
        }
        logical_bytes
    });

    struct SignalOnDir {
        inner: super::walker::NativeFs,
        listed: Arc<AtomicBool>,
    }
    impl super::walker::AnalyzeFs for SignalOnDir {
        fn metadata(&self, path: &Path) -> Result<super::walker::EntryMeta, super::walker::FsFail> {
            self.inner.metadata(path)
        }
        fn read_dir(
            &self,
            path: &Path,
        ) -> Result<Vec<super::walker::DirChild>, super::walker::FsFail> {
            let children = self.inner.read_dir(path)?;
            if path.file_name().is_some_and(|name| name == "z-deleted") {
                self.listed.store(true, Ordering::Relaxed);
            }
            Ok(children)
        }
        fn volume_id(&self, path: &Path) -> Result<String, super::walker::FsFail> {
            self.inner.volume_id(path)
        }
    }

    let fs = SignalOnDir {
        inner: super::walker::NativeFs::default(),
        listed: Arc::clone(&listed_deleted),
    };
    let outcome = analyze_with_fs(&root, &fs, None, None);
    stop.store(true, Ordering::Relaxed);
    let final_growth_bytes = mutator.join().expect("mutator join");
    let outcome = outcome.expect("native churn analyze");
    let snapshot = outcome.snapshot();

    let deleted = snapshot
        .nodes
        .iter()
        .filter(|node| node.name.starts_with("deleted-"))
        .collect::<Vec<_>>();
    assert!(
        deleted.iter().any(|node| {
            node.evidence == AnalyzeEvidence::Unknown
                && node.warnings.contains(&AnalyzeWarningClass::Churn)
        }),
        "at least one listed-then-deleted file must remain explicit unknown churn"
    );
    let growing = snapshot
        .nodes
        .iter()
        .filter(|node| node.name.starts_with("growing-"))
        .collect::<Vec<_>>();
    assert!(
        growing.iter().any(|node| {
            node.evidence == AnalyzeEvidence::Incomplete
                && node.warnings.contains(&AnalyzeWarningClass::Churn)
                && node.bytes >= INITIAL_GROWTH_BYTES
                && node.bytes <= final_growth_bytes
        }),
        "at least one growing file must be an incomplete lower bound"
    );

    let denied = snapshot
        .nodes
        .iter()
        .find(|node| node.name == "y-denied")
        .expect("denied node");
    assert_eq!(denied.evidence, AnalyzeEvidence::Incomplete);
    assert_eq!(
        denied.bytes, 0,
        "denied content must not count as available"
    );
    assert!(denied.warnings.contains(&AnalyzeWarningClass::AccessDenied));
    assert!(
        !snapshot
            .nodes
            .iter()
            .any(|node| node.name == "excluded.bin")
    );

    for directory in snapshot
        .nodes
        .iter()
        .filter(|node| node.kind == AnalyzeNodeKind::Directory)
    {
        let children = snapshot
            .nodes
            .iter()
            .filter(|node| node.parent_id == Some(directory.id))
            .collect::<Vec<_>>();
        assert_eq!(directory.immediate_count as usize, children.len());
        assert_eq!(
            directory.bytes,
            children.iter().map(|node| node.bytes).sum::<u64>(),
            "directory {} must remain the sum of represented lower bounds",
            directory.name
        );
    }

    drop(denied_guard);
}

#[cfg(all(windows, not(debug_assertions)))]
mod native_gate {
    use super::*;
    use std::fs;
    use std::sync::Arc;

    fn process_private_bytes() -> Option<u64> {
        use std::mem::MaybeUninit;
        use windows_sys::Win32::{
            Foundation::HANDLE,
            System::{
                ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
                Threading::GetCurrentProcess,
            },
        };
        unsafe {
            let mut counters = MaybeUninit::<PROCESS_MEMORY_COUNTERS>::zeroed();
            let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ok =
                GetProcessMemoryInfo(GetCurrentProcess() as HANDLE, counters.as_mut_ptr(), size);
            if ok == 0 {
                None
            } else {
                Some(counters.assume_init().PagefileUsage as u64)
            }
        }
    }

    fn generate_native_50k(root: &Path) -> std::io::Result<()> {
        fs::create_dir_all(root)?;
        for dir in 0..1_000u32 {
            let dir_path = root.join(format!("d{dir:04}"));
            fs::create_dir_all(&dir_path)?;
            for file in 0..50u32 {
                let path = dir_path.join(format!("f{file:02}.dat"));
                fs::File::create(path)?;
            }
        }
        let unicode = root.join("很长的名字分析目录");
        fs::create_dir_all(&unicode)?;
        fs::write(unicode.join("文件.bin"), [])?;
        let _ = create_dir_symlink(&unicode, &root.join("reparse-leaf"));
        let churn = root.join("churn");
        fs::create_dir_all(&churn)?;
        fs::write(churn.join("live.dat"), [1, 2, 3, 4])?;
        Ok(())
    }

    fn assert_denied_branch(snapshot: &AnalyzeSnapshotV1) {
        let denied = snapshot
            .nodes
            .iter()
            .find(|node| node.name == "denied")
            .expect("denied node");
        assert_eq!(denied.evidence, AnalyzeEvidence::Incomplete);
        assert_eq!(denied.bytes, 0);
        assert!(denied.warnings.contains(&AnalyzeWarningClass::AccessDenied));
        assert!(
            !snapshot
                .nodes
                .iter()
                .any(|node| node.name == "excluded.bin")
        );
    }

    #[test]
    fn analysis_native_50k_v1_resource_gate() {
        let fixture = tempfile::TempDir::new().expect("native fixture");
        let root = fixture.path().join("analysis-native-50k-v1");
        generate_native_50k(&root).expect("generate tree");
        let denied_path = root.join("denied");
        let denied_guard = DeniedDirectoryGuard::create(&denied_path).expect("create denied ACL");
        fs::write(denied_path.join("excluded.bin"), vec![9u8; 64 * 1024]).expect("denied content");
        denied_guard
            .deny_read_attributes()
            .expect("deny read attributes");

        let _warmup = analyze_path(&root, None, None).expect("warmup");
        assert_denied_branch(_warmup.snapshot());
        let mut samples = Vec::new();
        for _ in 0..5 {
            let started = Instant::now();
            let mut peak_private = process_private_bytes().unwrap_or(0);
            let sampler_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag = Arc::clone(&sampler_stop);
            let handle = thread::spawn(move || {
                let mut peak = peak_private;
                while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                    if let Some(bytes) = process_private_bytes() {
                        peak = peak.max(bytes);
                    }
                    thread::sleep(Duration::from_millis(200));
                }
                peak
            });
            let outcome = analyze_path(&root, None, None).expect("measured");
            sampler_stop.store(true, std::sync::atomic::Ordering::Relaxed);
            peak_private = handle.join().unwrap_or(peak_private).max(peak_private);
            let snapshot = outcome.snapshot();
            assert_denied_branch(snapshot);
            samples.push(serde_json::json!({
                "nodes": snapshot.nodes.len(),
                "accounted_bytes": snapshot.accounted_owned_bytes,
                "warnings": snapshot.warnings.len(),
                "wall_ms": started.elapsed().as_millis() as u64,
                "private_bytes": peak_private,
                "workers": ANALYZE_WORKERS,
            }));
            assert!(snapshot.nodes.len() >= 50_000);
            assert!(snapshot.accounted_owned_bytes <= MAX_OWNED_BYTES);
        }

        let mut cancel_ms = Vec::new();
        for _ in 0..5 {
            let cancel = Arc::new(FlagCancelObserver::new());
            let walk_root = root.clone();
            let walk_cancel = Arc::clone(&cancel);
            let handle =
                thread::spawn(move || analyze_path(&walk_root, Some(walk_cancel.as_ref()), None));
            thread::sleep(Duration::from_millis(150));
            let cancel_at = Instant::now();
            cancel.request_cancel();
            let outcome = handle
                .join()
                .expect("worker join")
                .expect("canceled analyze");
            let elapsed = cancel_at.elapsed().as_millis() as u64;
            assert_eq!(
                outcome.snapshot().completeness,
                AnalyzeCompleteness::Canceled
            );
            assert!(
                outcome.snapshot().nodes.len() > 1,
                "mid-walk cancel must observe represented nodes"
            );
            cancel_ms.push(elapsed);
        }
        cancel_ms.sort_unstable();
        // nearest-rank p95 for n=5 is ceil(0.95 * 5) = 5, the maximum sample.
        let p95 = *cancel_ms.last().expect("five cancel samples");
        assert!(
            p95 <= 500,
            "p95 cancel-to-join {p95} ms exceeds 500 ms; samples={cancel_ms:?}"
        );

        let evidence = serde_json::json!({
            "fixture": "analysis-native-50k-v1",
            "samples": samples,
            "cancel_to_join_ms": cancel_ms,
            "p95_cancel_to_join_ms": p95,
            "workers": ANALYZE_WORKERS,
            "denied_branch": "access_denied_incomplete_zero_bytes",
        });
        println!("{evidence}");
        drop(denied_guard);
    }
}
