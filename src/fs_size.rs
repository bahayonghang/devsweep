use std::{fs, path::Path, sync::Arc, time::SystemTime};

use rayon::prelude::*;

use crate::{
    model::{SizingWarning, SizingWarningKind},
    process_runner::{CancelObserver, FlagCancelObserver},
};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// Result of a no-follow reparse-point probe for one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReparseProbeResult {
    NotReparsePoint,
    ReparsePoint { tag: u32 },
    Unsupported { detail: String },
    Error { detail: String },
}

/// Platform boundary for path-level reparse checks.
///
/// Windows callers must not infer reparse safety from `Metadata` alone: some
/// Cloud Files providers do not report the reparse attribute through Rust's
/// metadata view. Implementations inspect the path without following it.
pub(crate) trait PathReparseProbe: Send + Sync {
    fn probe(&self, path: &Path) -> ReparseProbeResult;
}

/// Path-level result used by traversal and live authorization callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathSafety {
    Safe,
    ReparsePoint { tag: Option<u32> },
    Unverified { detail: String },
}

/// Native platform probe used outside tests.
#[derive(Debug, Default)]
pub(crate) struct SystemPathReparseProbe;

impl PathReparseProbe for SystemPathReparseProbe {
    fn probe(&self, path: &Path) -> ReparseProbeResult {
        #[cfg(windows)]
        {
            probe_windows_reparse_point(path)
        }

        #[cfg(not(windows))]
        {
            let _ = path;
            ReparseProbeResult::NotReparsePoint
        }
    }
}

/// Combines portable symlink detection with the path-level reparse probe.
///
/// A probe failure is deliberately not treated as a normal path. Callers must
/// skip traversal or deny authority for [`PathSafety::Unverified`].
pub(crate) fn inspect_path_no_follow(
    path: &Path,
    metadata: &fs::Metadata,
    probe: &dyn PathReparseProbe,
) -> PathSafety {
    let metadata_reports_link = is_unsafe_link(metadata);
    match probe.probe(path) {
        ReparseProbeResult::NotReparsePoint if metadata_reports_link => {
            PathSafety::ReparsePoint { tag: None }
        }
        ReparseProbeResult::NotReparsePoint => PathSafety::Safe,
        ReparseProbeResult::ReparsePoint { tag } => PathSafety::ReparsePoint { tag: Some(tag) },
        ReparseProbeResult::Unsupported { .. } | ReparseProbeResult::Error { .. }
            if metadata_reports_link =>
        {
            PathSafety::ReparsePoint { tag: None }
        }
        ReparseProbeResult::Unsupported { detail } => PathSafety::Unverified {
            detail: format!("reparse probe unsupported: {detail}"),
        },
        ReparseProbeResult::Error { detail } => PathSafety::Unverified {
            detail: format!("reparse probe failed: {detail}"),
        },
    }
}

fn reparse_probe_result_from_tag_info(
    file_attributes: u32,
    reparse_tag: u32,
) -> ReparseProbeResult {
    if reparse_tag != 0 || file_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        ReparseProbeResult::ReparsePoint { tag: reparse_tag }
    } else {
        ReparseProbeResult::NotReparsePoint
    }
}

/// Size walk result that distinguishes a verified empty tree from a failed walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeEstimate {
    /// Lower bound of observed logical bytes. `None` means no trustworthy total.
    pub logical_bytes: Option<u64>,
    pub complete: bool,
    pub last_modified: Option<SystemTime>,
    pub warnings: Vec<SizingWarning>,
}

impl SizeEstimate {
    pub fn trusted(bytes: u64, last_modified: Option<SystemTime>) -> Self {
        Self {
            logical_bytes: Some(bytes),
            complete: true,
            last_modified,
            warnings: Vec::new(),
        }
    }

    pub fn display_bytes(&self) -> u64 {
        self.logical_bytes.unwrap_or(0)
    }

    pub fn merge(mut self, other: Self) -> Self {
        let bytes = match (self.logical_bytes, other.logical_bytes) {
            (Some(left), Some(right)) => Some(left + right),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        };
        self.logical_bytes = bytes;
        self.complete = self.complete && other.complete;
        self.last_modified = max_mtime(self.last_modified, other.last_modified);
        self.warnings.extend(other.warnings);
        self
    }
}

/// Default entry budget for a single size walk root.
pub const DEFAULT_SIZE_ENTRY_BUDGET: usize = 50_000;

pub fn estimate_tree(path: &Path) -> SizeEstimate {
    estimate_tree_with_budget(path, DEFAULT_SIZE_ENTRY_BUDGET)
}

pub fn estimate_tree_with_budget(path: &Path, entry_budget: usize) -> SizeEstimate {
    estimate_tree_with_budget_and_cancel(path, entry_budget, None)
}

pub fn estimate_tree_with_budget_and_cancel(
    path: &Path,
    entry_budget: usize,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> SizeEstimate {
    let probe = SystemPathReparseProbe;
    estimate_tree_with_budget_and_cancel_and_probe(path, entry_budget, cancel, &probe)
}

pub(crate) fn estimate_tree_with_budget_and_cancel_and_probe(
    path: &Path,
    entry_budget: usize,
    cancel: Option<&Arc<FlagCancelObserver>>,
    probe: &dyn PathReparseProbe,
) -> SizeEstimate {
    let mut remaining = entry_budget;
    estimate_tree_bounded(
        path,
        &mut remaining,
        0,
        64,
        cancel.map(|flag| flag.as_ref()),
        probe,
    )
}

fn estimate_tree_bounded(
    path: &Path,
    remaining: &mut usize,
    depth: usize,
    max_depth: usize,
    cancel: Option<&FlagCancelObserver>,
    probe: &dyn PathReparseProbe,
) -> SizeEstimate {
    if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
        return SizeEstimate {
            logical_bytes: None,
            complete: false,
            last_modified: None,
            warnings: vec![sizing_warning(
                SizingWarningKind::Canceled,
                format!("size walk canceled at {}", path.display()),
            )],
        };
    }
    if *remaining == 0 {
        return SizeEstimate {
            logical_bytes: None,
            complete: false,
            last_modified: None,
            warnings: vec![sizing_warning(
                SizingWarningKind::EntryBudgetExhausted,
                format!("size entry budget exhausted at {}", path.display()),
            )],
        };
    }
    *remaining = remaining.saturating_sub(1);

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return SizeEstimate {
                logical_bytes: None,
                complete: false,
                last_modified: None,
                warnings: vec![sizing_warning(
                    SizingWarningKind::MetadataUnavailable,
                    format!("failed to inspect {}: {error}", path.display()),
                )],
            };
        }
    };
    match inspect_path_no_follow(path, &metadata, probe) {
        PathSafety::Safe => {}
        PathSafety::ReparsePoint { .. } => {
            return SizeEstimate::trusted(0, metadata.modified().ok());
        }
        PathSafety::Unverified { detail } => {
            return SizeEstimate {
                logical_bytes: None,
                complete: false,
                last_modified: metadata.modified().ok(),
                warnings: vec![sizing_warning(
                    SizingWarningKind::ReparseSafetyUnverified,
                    format!(
                        "could not verify reparse safety for {}: {detail}",
                        path.display()
                    ),
                )],
            };
        }
    }
    if metadata.is_file() {
        return SizeEstimate::trusted(metadata.len(), metadata.modified().ok());
    }
    if !metadata.is_dir() {
        return SizeEstimate::trusted(0, metadata.modified().ok());
    }
    if depth >= max_depth {
        return SizeEstimate {
            logical_bytes: Some(0),
            complete: false,
            last_modified: metadata.modified().ok(),
            warnings: vec![sizing_warning(
                SizingWarningKind::MaxDepthReached,
                format!("max depth reached at {}", path.display()),
            )],
        };
    }

    let self_mtime = metadata.modified().ok();
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            return SizeEstimate {
                logical_bytes: None,
                complete: false,
                last_modified: self_mtime,
                warnings: vec![sizing_warning(
                    SizingWarningKind::DirectoryReadFailed,
                    format!("failed to read {}: {error}", path.display()),
                )],
            };
        }
    };

    let mut children = Vec::new();
    let mut warnings = Vec::new();
    let mut entry_errors = false;
    let mut budget_hit = false;
    for entry in entries {
        if *remaining == 0 {
            budget_hit = true;
            warnings.push(sizing_warning(
                SizingWarningKind::EntryBudgetExhausted,
                format!("size entry budget exhausted under {}", path.display()),
            ));
            break;
        }
        match entry {
            Ok(entry) => children.push(entry.path()),
            Err(error) => {
                entry_errors = true;
                warnings.push(sizing_warning(
                    SizingWarningKind::DirectoryEntryReadFailed,
                    format!(
                        "failed to read directory entry under {}: {error}",
                        path.display()
                    ),
                ));
            }
        }
    }

    // Top-level fan-out may use rayon; nested walks stay sequential to avoid
    // task explosion on deep trees.
    let child_estimate = if depth == 0 && children.len() > 1 {
        children
            .par_iter()
            .map(|child| {
                let mut local = (*remaining).min(DEFAULT_SIZE_ENTRY_BUDGET);
                estimate_tree_bounded(child, &mut local, depth + 1, max_depth, cancel, probe)
            })
            .reduce(
                || SizeEstimate::trusted(0, None),
                |left, right| left.merge(right),
            )
    } else {
        let mut acc = SizeEstimate::trusted(0, None);
        for child in &children {
            if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
                acc.complete = false;
                acc.warnings.push(sizing_warning(
                    SizingWarningKind::Canceled,
                    format!("size walk canceled under {}", path.display()),
                ));
                break;
            }
            acc = acc.merge(estimate_tree_bounded(
                child,
                remaining,
                depth + 1,
                max_depth,
                cancel,
                probe,
            ));
        }
        acc
    };

    let mut estimate = child_estimate;
    estimate.last_modified = max_mtime(estimate.last_modified, self_mtime);
    estimate.warnings.extend(warnings);
    if entry_errors || budget_hit {
        estimate.complete = false;
    }
    estimate
}

pub(crate) fn is_unsafe_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || has_windows_reparse_point(metadata)
}

fn max_mtime(left: Option<SystemTime>, right: Option<SystemTime>) -> Option<SystemTime> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

#[cfg(windows)]
fn has_windows_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn sizing_warning(kind: SizingWarningKind, detail: impl Into<String>) -> SizingWarning {
    SizingWarning {
        kind,
        detail: detail.into(),
    }
}

#[cfg(windows)]
fn probe_windows_reparse_point(path: &Path) -> ReparseProbeResult {
    use std::{
        mem::{MaybeUninit, size_of},
        os::windows::ffi::OsStrExt,
    };

    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, FileAttributeTagInfo, GetFileInformationByHandleEx, OPEN_EXISTING,
        },
    };

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return classify_windows_probe_error("could not open path without following it");
    }

    let mut tag_info = MaybeUninit::<FILE_ATTRIBUTE_TAG_INFO>::zeroed();
    let queried = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            tag_info.as_mut_ptr().cast(),
            size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    };
    let error = if queried == 0 {
        Some(std::io::Error::last_os_error())
    } else {
        None
    };
    unsafe {
        let _ = CloseHandle(handle);
    }

    if let Some(error) = error {
        return classify_windows_probe_error_with("could not read path reparse tag", error);
    }

    let tag_info = unsafe { tag_info.assume_init() };
    reparse_probe_result_from_tag_info(tag_info.FileAttributes, tag_info.ReparseTag)
}

#[cfg(windows)]
fn classify_windows_probe_error(operation: &str) -> ReparseProbeResult {
    classify_windows_probe_error_with(operation, std::io::Error::last_os_error())
}

#[cfg(windows)]
fn classify_windows_probe_error_with(operation: &str, error: std::io::Error) -> ReparseProbeResult {
    let detail = format!("{operation}: {error}");
    match error.raw_os_error() {
        // These indicate that the filesystem or OS cannot service the tag query.
        Some(1 | 50 | 87) => ReparseProbeResult::Unsupported { detail },
        _ => ReparseProbeResult::Error { detail },
    }
}

#[cfg(not(windows))]
fn has_windows_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        path::{Path, PathBuf},
        time::SystemTime,
    };

    use tempfile::TempDir;

    use super::*;

    const CLOUD_FILES_REPARSE_TAG: u32 = 0x9000_701A;

    struct FixtureReparseProbe {
        results: HashMap<PathBuf, ReparseProbeResult>,
    }

    impl FixtureReparseProbe {
        fn tagged(path: &Path) -> Self {
            let mut results = HashMap::new();
            results.insert(
                path.to_path_buf(),
                ReparseProbeResult::ReparsePoint {
                    tag: CLOUD_FILES_REPARSE_TAG,
                },
            );
            Self { results }
        }

        fn failing(path: &Path) -> Self {
            let mut results = HashMap::new();
            results.insert(
                path.to_path_buf(),
                ReparseProbeResult::Error {
                    detail: "fixture probe failure".to_string(),
                },
            );
            Self { results }
        }
    }

    impl PathReparseProbe for FixtureReparseProbe {
        fn probe(&self, path: &Path) -> ReparseProbeResult {
            self.results
                .get(path)
                .cloned()
                .unwrap_or(ReparseProbeResult::NotReparsePoint)
        }
    }

    #[test]
    fn estimate_tree_counts_file_bytes_and_latest_mtime() {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        let nested = fixture.path("root/nested");
        let first_file = fixture.file("root/a.txt", "abcd");
        let second_file = fixture.file("root/nested/b.txt", "123456");
        let third_file = fixture.file("root/nested/latest.txt", "xy");

        let estimate = estimate_tree(&root);

        assert_eq!(estimate.logical_bytes, Some(12));
        assert!(estimate.complete);
        assert_eq!(
            estimate.last_modified,
            latest_of([&root, &nested, &first_file, &second_file, &third_file])
        );
    }

    #[test]
    fn parallel_estimate_matches_serial_reference() {
        let fixture = Fixture::new();
        for dir in 0..8 {
            for file in 0..8 {
                fixture.file(&format!("root/dir-{dir}/file-{file}.txt"), "payload");
            }
        }
        fixture.file("root/empty/.keep", "");

        assert_eq!(
            estimate_tree(&fixture.path("root")),
            serial_estimate_tree(&fixture.path("root"))
        );
    }

    #[test]
    fn estimate_tree_stops_promptly_when_cancel_is_requested() {
        use crate::process_runner::FlagCancelObserver;
        use std::sync::Arc;

        let fixture = Fixture::new();
        for i in 0..200 {
            fixture.file(&format!("root/file-{i}.txt"), "payload");
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();
        let started = std::time::Instant::now();
        let estimate =
            estimate_tree_with_budget_and_cancel(&fixture.path("root"), 50_000, Some(&cancel));
        let elapsed = started.elapsed();
        assert!(!estimate.complete);
        assert!(
            estimate
                .warnings
                .iter()
                .any(|warning| warning.kind == SizingWarningKind::Canceled),
            "expected cancel warning: {:?}",
            estimate.warnings
        );
        assert!(
            elapsed.as_millis() < 250,
            "cancel should abort size walk quickly, took {elapsed:?}"
        );
    }

    #[test]
    fn estimate_tree_marks_missing_path_unknown() {
        let fixture = Fixture::new();

        let estimate = estimate_tree(&fixture.path("missing"));
        assert_eq!(estimate.logical_bytes, None);
        assert!(!estimate.complete);
        assert!(!estimate.warnings.is_empty());
    }

    #[test]
    fn estimate_tree_marks_empty_directory_complete_zero() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.path("empty")).expect("empty dir");

        let estimate = estimate_tree(&fixture.path("empty"));
        assert_eq!(estimate.logical_bytes, Some(0));
        assert!(estimate.complete);
        assert!(estimate.warnings.is_empty());
    }

    #[test]
    fn estimate_tree_does_not_follow_symlinked_directories() {
        let fixture = Fixture::new();
        fixture.file("outside/big.bin", "not counted");
        fs::create_dir_all(fixture.path("root")).expect("root directory");
        let link = fixture.path("root/link");

        if create_dir_symlink(&fixture.path("outside"), &link).is_err() {
            return;
        }

        let root = fixture.path("root");
        let link_mtime = fs::symlink_metadata(&link)
            .expect("link metadata")
            .modified()
            .ok();
        let root_mtime = fs::metadata(&root).and_then(|m| m.modified()).ok();

        let estimate = estimate_tree(&root);
        assert_eq!(estimate.logical_bytes, Some(0));
        assert!(estimate.complete);
        assert_eq!(estimate.last_modified, max_mtime(root_mtime, link_mtime));
    }

    #[test]
    fn path_probe_treats_a_nonzero_tag_as_reparse_without_the_attribute_bit() {
        assert_eq!(
            reparse_probe_result_from_tag_info(0x80030, CLOUD_FILES_REPARSE_TAG),
            ReparseProbeResult::ReparsePoint {
                tag: CLOUD_FILES_REPARSE_TAG,
            }
        );
    }

    #[test]
    fn estimate_tree_excludes_injected_cloud_files_reparse_child() {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        fixture.file("root/visible.txt", "hello");
        fixture.file("root/cloud/hidden.txt", "not counted");
        let cloud_child = root.join("cloud");
        let probe = FixtureReparseProbe::tagged(&cloud_child);

        let estimate = estimate_tree_with_budget_and_cancel_and_probe(
            &root,
            DEFAULT_SIZE_ENTRY_BUDGET,
            None,
            &probe,
        );

        assert_eq!(estimate.logical_bytes, Some(5));
        assert!(estimate.complete);
    }

    #[test]
    fn estimate_tree_fails_closed_when_reparse_probe_cannot_verify_root() {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        fixture.file("root/visible.txt", "hello");
        let probe = FixtureReparseProbe::failing(&root);

        let estimate = estimate_tree_with_budget_and_cancel_and_probe(
            &root,
            DEFAULT_SIZE_ENTRY_BUDGET,
            None,
            &probe,
        );

        assert_eq!(estimate.logical_bytes, None);
        assert!(!estimate.complete);
        assert!(
            estimate
                .warnings
                .iter()
                .any(|warning| warning.kind == SizingWarningKind::ReparseSafetyUnverified),
            "expected fail-closed warning: {:?}",
            estimate.warnings
        );
    }

    #[cfg(windows)]
    #[test]
    fn estimate_tree_does_not_follow_windows_reparse_points() {
        let fixture = Fixture::new();
        fixture.file("outside/big.bin", "not counted");
        fs::create_dir_all(fixture.path("root")).expect("root directory");
        let link = fixture.path("root/reparse");

        if create_dir_symlink(&fixture.path("outside"), &link).is_err() {
            return;
        }

        let metadata = fs::symlink_metadata(&link).expect("link metadata");
        assert!(has_windows_reparse_point(&metadata));

        let root = fixture.path("root");
        let link_mtime = metadata.modified().ok();
        let root_mtime = fs::metadata(&root).and_then(|m| m.modified()).ok();
        let estimate = estimate_tree(&root);
        assert_eq!(estimate.logical_bytes, Some(0));
        assert!(estimate.complete);
        assert_eq!(estimate.last_modified, max_mtime(root_mtime, link_mtime));
    }

    #[cfg(windows)]
    #[test]
    fn estimate_tree_marks_denied_child_incomplete() {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        let denied = fixture.path("root/denied");
        let visible = fixture.file("root/visible.txt", "hello");
        fs::create_dir_all(&denied).expect("denied dir");
        fs::write(denied.join("secret.bin"), "secret").expect("secret file");

        if deny_directory_read(&denied).is_err() {
            return;
        }
        assert!(
            fs::read_dir(&denied).is_err(),
            "fixture must actually deny read_dir"
        );

        let estimate = estimate_tree(&root);
        assert!(!estimate.complete, "denied child must mark incomplete");
        assert_eq!(
            estimate.logical_bytes,
            Some(5),
            "visible sibling bytes remain as lower bound"
        );
        assert!(
            estimate
                .warnings
                .iter()
                .any(|warning| warning.detail.contains("denied")),
            "warning names the failed path: {:?}",
            estimate.warnings
        );
        let _ = visible;
        let _ = restore_directory_read(&denied);
    }

    #[cfg(unix)]
    #[test]
    fn estimate_tree_marks_denied_child_incomplete() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new();
        let root = fixture.path("root");
        fixture.file("root/visible.txt", "hello");
        let denied = fixture.path("root/denied");
        fs::create_dir_all(&denied).expect("denied dir");
        fs::write(denied.join("secret.bin"), "secret").expect("secret file");
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o000)).expect("chmod");

        assert!(
            fs::read_dir(&denied).is_err(),
            "fixture must actually deny read_dir"
        );

        let estimate = estimate_tree(&root);
        let _ = fs::set_permissions(&denied, fs::Permissions::from_mode(0o755));
        assert!(!estimate.complete);
        assert_eq!(estimate.logical_bytes, Some(5));
        assert!(!estimate.warnings.is_empty());
    }

    fn latest_of<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> Option<SystemTime> {
        paths
            .into_iter()
            .filter_map(|path| {
                fs::metadata(path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
            })
            .reduce(|left, right| left.max(right))
    }

    fn serial_estimate_tree(path: &Path) -> SizeEstimate {
        estimate_tree(path)
    }

    struct Fixture {
        temp: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                temp: TempDir::new().expect("temp dir"),
            }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.temp.path().join(relative)
        }

        fn file(&self, relative: &str, content: &str) -> PathBuf {
            let path = self.path(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture directory");
            }
            fs::write(&path, content).expect("fixture file");
            path
        }
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(windows)]
    fn deny_directory_read(path: &Path) -> std::io::Result<()> {
        use std::process::Command;
        let output = Command::new("icacls")
            .arg(path)
            .arg("/deny")
            .arg(format!("{}:(OI)(CI)(R,X)", current_user()?))
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(String::from_utf8_lossy(
                &output.stderr,
            )))
        }
    }

    #[cfg(windows)]
    fn restore_directory_read(path: &Path) -> std::io::Result<()> {
        use std::process::Command;
        let _ = Command::new("icacls")
            .arg(path)
            .arg("/remove:d")
            .arg(current_user()?)
            .status();
        Ok(())
    }

    #[cfg(windows)]
    fn current_user() -> std::io::Result<String> {
        std::env::var("USERNAME").map_err(|_| std::io::Error::other("USERNAME missing"))
    }
}
