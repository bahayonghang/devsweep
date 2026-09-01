use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::SystemTime,
};

use rayon::{ThreadPool, ThreadPoolBuilder, prelude::*};

use crate::{
    model::{SizingWarning, SizingWarningKind},
    process::{CancelObserver, FlagCancelObserver},
};

#[cfg(all(test, windows))]
use super::reparse::has_windows_reparse_point;
use super::reparse::{
    PathReparseProbe, PathSafety, SystemPathReparseProbe, inspect_path_no_follow,
};
#[cfg(test)]
use super::reparse::{ReparseProbeResult, reparse_probe_result_from_tag_info};

/// Size walk result that distinguishes a verified empty tree from a failed walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SizeEstimate {
    /// Lower bound of observed logical bytes. `None` means no trustworthy total.
    pub logical_bytes: Option<u64>,
    pub complete: bool,
    pub last_modified: Option<SystemTime>,
    pub warnings: Vec<SizingWarning>,
}

impl SizeEstimate {
    pub(crate) fn trusted(bytes: u64, last_modified: Option<SystemTime>) -> Self {
        Self {
            logical_bytes: Some(bytes),
            complete: true,
            last_modified,
            warnings: Vec::new(),
        }
    }

    pub(crate) fn display_bytes(&self) -> u64 {
        self.logical_bytes.unwrap_or(0)
    }

    pub(crate) fn merge(mut self, other: Self) -> Self {
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
pub(crate) const DEFAULT_SIZE_ENTRY_BUDGET: usize = 50_000;

/// Hard ceiling for filesystem-sizing work owned by this module.
const SIZE_WALK_WORKERS: usize = 2;
const MAX_SIZE_WALK_WORKERS: usize = 4;
const _: () = assert!(SIZE_WALK_WORKERS <= MAX_SIZE_WALK_WORKERS);

static SIZE_WALK_POOL: OnceLock<Result<ThreadPool, rayon::ThreadPoolBuildError>> = OnceLock::new();

#[derive(Clone, Copy)]
enum RootExecutionMode<'a> {
    Dedicated(&'a ThreadPool),
    SerialFallback,
}

fn production_root_execution_mode() -> RootExecutionMode<'static> {
    match SIZE_WALK_POOL.get_or_init(|| {
        ThreadPoolBuilder::new()
            .num_threads(SIZE_WALK_WORKERS)
            .thread_name(|index| format!("devsweep-size-{index}"))
            .build()
    }) {
        Ok(pool) => RootExecutionMode::Dedicated(pool),
        Err(_) => RootExecutionMode::SerialFallback,
    }
}

#[cfg(test)]
fn estimate_tree(path: &Path) -> SizeEstimate {
    estimate_tree_with_budget(path, DEFAULT_SIZE_ENTRY_BUDGET)
}

#[cfg(test)]
fn estimate_tree_with_budget(path: &Path, entry_budget: usize) -> SizeEstimate {
    estimate_tree_with_budget_and_cancel(path, entry_budget, None)
}

pub(crate) fn estimate_tree_with_budget_and_cancel(
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
    estimate_tree_with_limits_and_mode_and_probe(
        path,
        entry_budget,
        64,
        cancel,
        probe,
        production_root_execution_mode(),
    )
}

/// Sizes many independent roots on the existing two-worker pool.
///
/// Walks queue on [`SIZE_WALK_POOL`]; this does not create another Rayon pool
/// or raise the worker ceiling. Each root stays bounded, cancelable, and
/// no-follow.
pub(crate) fn estimate_trees_with_budget_and_cancel(
    paths: &[PathBuf],
    entry_budget: usize,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Vec<SizeEstimate> {
    if paths.is_empty() {
        return Vec::new();
    }
    let probe = SystemPathReparseProbe;
    match production_root_execution_mode() {
        RootExecutionMode::Dedicated(pool) => pool.install(|| {
            paths
                .par_iter()
                .map(|path| {
                    estimate_tree_with_limits_and_mode_and_probe(
                        path,
                        entry_budget,
                        64,
                        cancel,
                        &probe,
                        RootExecutionMode::Dedicated(pool),
                    )
                })
                .collect()
        }),
        RootExecutionMode::SerialFallback => paths
            .iter()
            .map(|path| {
                estimate_tree_with_limits_and_mode_and_probe(
                    path,
                    entry_budget,
                    64,
                    cancel,
                    &probe,
                    RootExecutionMode::SerialFallback,
                )
            })
            .collect(),
    }
}

fn estimate_tree_with_limits_and_mode_and_probe(
    path: &Path,
    entry_budget: usize,
    max_depth: usize,
    cancel: Option<&Arc<FlagCancelObserver>>,
    probe: &dyn PathReparseProbe,
    root_execution: RootExecutionMode<'_>,
) -> SizeEstimate {
    let mut remaining = entry_budget;
    estimate_tree_bounded(
        path,
        &mut remaining,
        0,
        max_depth,
        cancel.map(|flag| flag.as_ref()),
        probe,
        root_execution,
    )
}

fn estimate_tree_bounded(
    path: &Path,
    remaining: &mut usize,
    depth: usize,
    max_depth: usize,
    cancel: Option<&FlagCancelObserver>,
    probe: &dyn PathReparseProbe,
    root_execution: RootExecutionMode<'_>,
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

    // Top-level fan-out uses only the module-owned pool. Nested walks stay
    // sequential to avoid task explosion on deep trees.
    let child_estimate = if depth == 0 && children.len() > 1 {
        estimate_root_children(
            path,
            &children,
            *remaining,
            depth + 1,
            max_depth,
            cancel,
            probe,
            root_execution,
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
                root_execution,
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

#[allow(clippy::too_many_arguments)]
fn estimate_root_children(
    root: &Path,
    children: &[std::path::PathBuf],
    root_remaining_at_fanout: usize,
    child_depth: usize,
    max_depth: usize,
    cancel: Option<&FlagCancelObserver>,
    probe: &dyn PathReparseProbe,
    root_execution: RootExecutionMode<'_>,
) -> SizeEstimate {
    let estimate_child = |child: &Path| {
        let mut local_remaining = root_remaining_at_fanout.min(DEFAULT_SIZE_ENTRY_BUDGET);
        estimate_tree_bounded(
            child,
            &mut local_remaining,
            child_depth,
            max_depth,
            cancel,
            probe,
            root_execution,
        )
    };

    match root_execution {
        RootExecutionMode::Dedicated(pool) => pool.install(|| {
            children
                .par_iter()
                .map(|child| estimate_child(child))
                .reduce(
                    || SizeEstimate::trusted(0, None),
                    |left, right| left.merge(right),
                )
        }),
        RootExecutionMode::SerialFallback => {
            let mut estimate = SizeEstimate::trusted(0, None);
            for child in children {
                if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
                    estimate.complete = false;
                    estimate.warnings.push(sizing_warning(
                        SizingWarningKind::Canceled,
                        format!("size walk canceled under {}", root.display()),
                    ));
                    break;
                }
                estimate = estimate.merge(estimate_child(child));
            }
            estimate
        }
    }
}

fn max_mtime(left: Option<SystemTime>, right: Option<SystemTime>) -> Option<SystemTime> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn sizing_warning(kind: SizingWarningKind, detail: impl Into<String>) -> SizingWarning {
    SizingWarning {
        kind,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        path::{Path, PathBuf},
        sync::{
            Arc, Condvar, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
        time::{Duration, Instant, SystemTime},
    };

    use tempfile::TempDir;

    use super::*;

    const CLOUD_FILES_REPARSE_TAG: u32 = 0x9000_701A;

    #[derive(Clone, Copy)]
    enum TestRootMode {
        Dedicated,
        Serial,
    }

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

        fn unsupported(path: &Path) -> Self {
            let mut results = HashMap::new();
            results.insert(
                path.to_path_buf(),
                ReparseProbeResult::Unsupported {
                    detail: "fixture probe unsupported".to_string(),
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

        assert_estimates_equivalent(
            &estimate_tree(&fixture.path("root")),
            &serial_estimate_tree(&fixture.path("root")),
        );
    }

    #[test]
    fn dedicated_pool_enforces_the_worker_ceiling() {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        for child in 0..(SIZE_WALK_WORKERS * 2) {
            fixture.file(&format!("root/file-{child}.txt"), "payload");
        }
        let probe = Arc::new(WorkerGateProbe::new(root.clone()));
        let worker_probe = Arc::clone(&probe);
        let pool = test_pool();
        assert_eq!(pool.current_num_threads(), SIZE_WALK_WORKERS);

        let handle = thread::spawn(move || {
            estimate_tree_with_limits_and_mode_and_probe(
                &root,
                DEFAULT_SIZE_ENTRY_BUDGET,
                64,
                None,
                worker_probe.as_ref(),
                RootExecutionMode::Dedicated(&pool),
            )
        });

        probe.wait_for_workers(SIZE_WALK_WORKERS);
        probe.release();
        let estimate = handle.join().expect("dedicated size walk joins");
        assert!(estimate.complete, "warnings: {:?}", estimate.warnings);
        assert_eq!(probe.peak.load(Ordering::SeqCst), SIZE_WALK_WORKERS);
        assert!(probe.peak.load(Ordering::SeqCst) <= SIZE_WALK_WORKERS);
    }

    #[test]
    fn estimate_trees_preserve_order_and_match_single_root_walks() {
        let fixture = Fixture::new();
        fixture.file("left/a.txt", "aa");
        fixture.file("right/b.txt", "bbbb");
        let paths = vec![fixture.path("left"), fixture.path("right")];
        let estimates =
            estimate_trees_with_budget_and_cancel(&paths, DEFAULT_SIZE_ENTRY_BUDGET, None);
        assert_eq!(estimates.len(), 2);
        assert_eq!(
            estimates[0].logical_bytes,
            estimate_tree(&paths[0]).logical_bytes
        );
        assert_eq!(
            estimates[1].logical_bytes,
            estimate_tree(&paths[1]).logical_bytes
        );
        assert!(estimates.iter().all(|estimate| estimate.complete));
        assert!(
            estimate_trees_with_budget_and_cancel(&[], DEFAULT_SIZE_ENTRY_BUDGET, None).is_empty()
        );
    }

    #[test]
    fn estimate_tree_stops_promptly_when_cancel_is_requested() {
        let fixture = Fixture::new();
        for i in 0..200 {
            fixture.file(&format!("root/file-{i}.txt"), "payload");
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();
        for mode in [TestRootMode::Dedicated, TestRootMode::Serial] {
            let started = Instant::now();
            let estimate = estimate_with_test_mode(
                &fixture.path("root"),
                50_000,
                64,
                Some(&cancel),
                &SystemPathReparseProbe,
                mode,
            );
            let elapsed = started.elapsed();
            assert_canceled(&estimate);
            assert!(
                elapsed < Duration::from_millis(250),
                "cancel should abort size walk quickly, took {elapsed:?}"
            );
        }
    }

    #[test]
    fn dedicated_and_serial_modes_cancel_mid_walk_without_deadlock() {
        for mode in [TestRootMode::Dedicated, TestRootMode::Serial] {
            assert_mid_walk_cancel(mode);
        }
    }

    #[test]
    fn dedicated_and_serial_modes_preserve_fresh_root_child_budgets() {
        let fixture = Fixture::new();
        for child in 0..4 {
            fixture.file(&format!("root/dir-{child}/payload.bin"), "payload");
        }
        let root = fixture.path("root");
        let dedicated = estimate_with_test_mode(
            &root,
            2,
            64,
            None,
            &SystemPathReparseProbe,
            TestRootMode::Dedicated,
        );
        let serial = estimate_with_test_mode(
            &root,
            2,
            64,
            None,
            &SystemPathReparseProbe,
            TestRootMode::Serial,
        );

        assert_estimates_equivalent(&dedicated, &serial);
        assert!(!dedicated.complete);
        assert_eq!(
            warning_count(&dedicated, SizingWarningKind::EntryBudgetExhausted),
            4,
            "every root child must receive its own local budget"
        );
    }

    #[test]
    fn low_max_depth_is_incomplete_in_both_root_modes() {
        let fixture = Fixture::new();
        fixture.file("root/left/nested/payload.bin", "left");
        fixture.file("root/right/nested/payload.bin", "right");
        let root = fixture.path("root");

        let dedicated = estimate_with_test_mode(
            &root,
            DEFAULT_SIZE_ENTRY_BUDGET,
            1,
            None,
            &SystemPathReparseProbe,
            TestRootMode::Dedicated,
        );
        let serial = estimate_with_test_mode(
            &root,
            DEFAULT_SIZE_ENTRY_BUDGET,
            1,
            None,
            &SystemPathReparseProbe,
            TestRootMode::Serial,
        );

        assert_estimates_equivalent(&dedicated, &serial);
        assert!(!dedicated.complete);
        assert_eq!(
            warning_count(&dedicated, SizingWarningKind::MaxDepthReached),
            2
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

        let serial = estimate_with_test_mode(
            &root,
            DEFAULT_SIZE_ENTRY_BUDGET,
            64,
            None,
            &probe,
            TestRootMode::Serial,
        );
        assert_estimates_equivalent(&estimate, &serial);
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

    #[test]
    fn estimate_tree_fails_closed_when_reparse_probe_is_unsupported() {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        fixture.file("root/visible.txt", "hello");
        let probe = FixtureReparseProbe::unsupported(&root);

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

    fn test_pool() -> ThreadPool {
        ThreadPoolBuilder::new()
            .num_threads(SIZE_WALK_WORKERS)
            .build()
            .expect("dedicated test pool")
    }

    fn estimate_with_test_mode(
        path: &Path,
        entry_budget: usize,
        max_depth: usize,
        cancel: Option<&Arc<FlagCancelObserver>>,
        probe: &dyn PathReparseProbe,
        mode: TestRootMode,
    ) -> SizeEstimate {
        match mode {
            TestRootMode::Dedicated => {
                let pool = test_pool();
                estimate_tree_with_limits_and_mode_and_probe(
                    path,
                    entry_budget,
                    max_depth,
                    cancel,
                    probe,
                    RootExecutionMode::Dedicated(&pool),
                )
            }
            TestRootMode::Serial => estimate_tree_with_limits_and_mode_and_probe(
                path,
                entry_budget,
                max_depth,
                cancel,
                probe,
                RootExecutionMode::SerialFallback,
            ),
        }
    }

    fn serial_estimate_tree(path: &Path) -> SizeEstimate {
        estimate_with_test_mode(
            path,
            DEFAULT_SIZE_ENTRY_BUDGET,
            64,
            None,
            &SystemPathReparseProbe,
            TestRootMode::Serial,
        )
    }

    fn warning_kinds(estimate: &SizeEstimate) -> Vec<String> {
        let mut kinds = estimate
            .warnings
            .iter()
            .map(|warning| format!("{:?}", warning.kind))
            .collect::<Vec<_>>();
        kinds.sort();
        kinds
    }

    fn warning_count(estimate: &SizeEstimate, kind: SizingWarningKind) -> usize {
        estimate
            .warnings
            .iter()
            .filter(|warning| warning.kind == kind)
            .count()
    }

    fn assert_estimates_equivalent(left: &SizeEstimate, right: &SizeEstimate) {
        assert_eq!(left.logical_bytes, right.logical_bytes);
        assert_eq!(left.complete, right.complete);
        assert_eq!(left.last_modified, right.last_modified);
        assert_eq!(warning_kinds(left), warning_kinds(right));
    }

    fn assert_canceled(estimate: &SizeEstimate) {
        assert!(!estimate.complete);
        assert!(
            estimate
                .warnings
                .iter()
                .any(|warning| warning.kind == SizingWarningKind::Canceled),
            "expected cancel warning: {:?}",
            estimate.warnings
        );
    }

    fn assert_mid_walk_cancel(mode: TestRootMode) {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        for child in 0..8 {
            fixture.file(&format!("root/file-{child}.txt"), "payload");
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        let probe = Arc::new(CancelGateProbe::new(root.clone()));
        let worker_cancel = Arc::clone(&cancel);
        let worker_probe = Arc::clone(&probe);
        let handle = thread::spawn(move || {
            estimate_with_test_mode(
                &root,
                DEFAULT_SIZE_ENTRY_BUDGET,
                64,
                Some(&worker_cancel),
                worker_probe.as_ref(),
                mode,
            )
        });

        probe.wait_until_entered();
        cancel.request_cancel();
        probe.release();
        let released = Instant::now();
        let estimate = handle.join().expect("canceled size walk joins");
        let elapsed = released.elapsed();
        assert_canceled(&estimate);
        assert!(
            elapsed < Duration::from_millis(250),
            "mid-walk cancellation took {elapsed:?} after release"
        );
    }

    #[derive(Default)]
    struct GateState {
        entered: usize,
        released: bool,
    }

    struct WorkerGateProbe {
        root: PathBuf,
        state: Mutex<GateState>,
        changed: Condvar,
        current: AtomicUsize,
        peak: AtomicUsize,
    }

    impl WorkerGateProbe {
        fn new(root: PathBuf) -> Self {
            Self {
                root,
                state: Mutex::new(GateState::default()),
                changed: Condvar::new(),
                current: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
            }
        }

        fn wait_for_workers(&self, expected: usize) {
            let state = self.state.lock().expect("worker gate lock");
            let (state, timeout) = self
                .changed
                .wait_timeout_while(state, Duration::from_secs(5), |state| {
                    state.entered < expected
                })
                .expect("worker gate wait");
            assert!(!timeout.timed_out(), "workers did not reach gate");
            assert!(state.entered >= expected);
        }

        fn release(&self) {
            let mut state = self.state.lock().expect("worker gate lock");
            state.released = true;
            self.changed.notify_all();
        }
    }

    impl PathReparseProbe for WorkerGateProbe {
        fn probe(&self, path: &Path) -> ReparseProbeResult {
            if path.parent() != Some(self.root.as_path()) {
                return ReparseProbeResult::NotReparsePoint;
            }
            let current = self.current.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(current, Ordering::SeqCst);
            let mut state = self.state.lock().expect("worker gate lock");
            state.entered += 1;
            self.changed.notify_all();
            let (state, timeout) = self
                .changed
                .wait_timeout_while(state, Duration::from_secs(5), |state| !state.released)
                .expect("worker release wait");
            assert!(!timeout.timed_out(), "worker gate was not released");
            drop(state);
            self.current.fetch_sub(1, Ordering::SeqCst);
            ReparseProbeResult::NotReparsePoint
        }
    }

    struct CancelGateProbe {
        root: PathBuf,
        state: Mutex<GateState>,
        changed: Condvar,
    }

    impl CancelGateProbe {
        fn new(root: PathBuf) -> Self {
            Self {
                root,
                state: Mutex::new(GateState::default()),
                changed: Condvar::new(),
            }
        }

        fn wait_until_entered(&self) {
            let state = self.state.lock().expect("cancel gate lock");
            let (state, timeout) = self
                .changed
                .wait_timeout_while(state, Duration::from_secs(5), |state| state.entered == 0)
                .expect("cancel gate wait");
            assert!(!timeout.timed_out(), "size walk did not reach cancel gate");
            assert!(state.entered > 0);
        }

        fn release(&self) {
            let mut state = self.state.lock().expect("cancel gate lock");
            state.released = true;
            self.changed.notify_all();
        }
    }

    impl PathReparseProbe for CancelGateProbe {
        fn probe(&self, path: &Path) -> ReparseProbeResult {
            if path.parent() != Some(self.root.as_path()) {
                return ReparseProbeResult::NotReparsePoint;
            }
            let mut state = self.state.lock().expect("cancel gate lock");
            state.entered += 1;
            self.changed.notify_all();
            let (state, timeout) = self
                .changed
                .wait_timeout_while(state, Duration::from_secs(5), |state| !state.released)
                .expect("cancel release wait");
            assert!(!timeout.timed_out(), "cancel gate was not released");
            drop(state);
            ReparseProbeResult::NotReparsePoint
        }
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
