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

#[cfg(test)]
use super::reparse::{InspectedEntry, ReparseProbeResult, reparse_probe_result_from_tag_info};
use super::reparse::{
    PathReparseProbe, PathSafety, SystemPathReparseProbe, inspect_entry_no_follow,
};
#[cfg(all(test, windows))]
use super::reparse::{has_windows_reparse_point, inspect_path_no_follow};

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
const ROOT_CHILD_BATCH_SIZE: usize = 64;
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
                .with_max_len(1)
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

#[cfg(any(windows, test))]
#[derive(Default)]
pub(crate) struct StreamedSizeEstimates {
    pub canceled_after_produce: bool,
    // Carry panics across the joined MTA boundary instead of reclassifying them
    // as an enumeration failure. The inventory caller resumes them after join.
    pub estimates: Vec<(usize, std::thread::Result<SizeEstimate>)>,
}

/// Produces paths on the calling thread and sizes at most two uncollected roots
/// on the existing pool. In particular, a WinRT producer stays on its MTA.
#[cfg(windows)]
pub(crate) fn estimate_streamed_trees<R>(
    entry_budget: usize,
    cancel: Option<&Arc<FlagCancelObserver>>,
    produce: impl FnOnce(&mut dyn FnMut(usize, &Path)) -> R,
) -> (R, StreamedSizeEstimates) {
    estimate_streamed_trees_with_mode_and_probe(
        entry_budget,
        cancel,
        produce,
        &SystemPathReparseProbe,
        production_root_execution_mode(),
    )
}

#[cfg(any(windows, test))]
fn estimate_streamed_trees_with_mode_and_probe<R>(
    entry_budget: usize,
    cancel: Option<&Arc<FlagCancelObserver>>,
    produce: impl FnOnce(&mut dyn FnMut(usize, &Path)) -> R,
    probe: &dyn PathReparseProbe,
    mode: RootExecutionMode<'_>,
) -> (R, StreamedSizeEstimates) {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::mpsc::{RecvTimeoutError, sync_channel},
        time::Duration,
    };

    let canceled = || cancel.is_some_and(|flag| flag.is_cancel_requested());
    let measure = |path: &Path| {
        catch_unwind(AssertUnwindSafe(|| {
            estimate_tree_with_limits_and_mode_and_probe(
                path,
                entry_budget,
                64,
                cancel,
                probe,
                mode,
            )
        }))
    };
    let mut sizes = StreamedSizeEstimates::default();
    let RootExecutionMode::Dedicated(pool) = mode else {
        let mut paths = Vec::new();
        let result = produce(&mut |index, path| paths.push((index, path.to_path_buf())));
        sizes.canceled_after_produce = canceled();
        for (index, path) in paths {
            let estimate = measure(&path);
            let panicked = estimate.is_err();
            sizes.estimates.push((index, estimate));
            if panicked {
                break;
            }
        }
        return (result, sizes);
    };

    let (sender, receiver) =
        sync_channel::<(usize, std::thread::Result<SizeEstimate>)>(SIZE_WALK_WORKERS);
    let mut pending = 0;
    let mut panicked = false;
    let result = pool.in_place_scope(|scope| {
        let result = produce(&mut |index, path| {
            while pending == SIZE_WALK_WORKERS && !canceled() && !panicked {
                match receiver.recv_timeout(Duration::from_millis(10)) {
                    Ok((index, estimate)) => {
                        pending -= 1;
                        panicked |= estimate.is_err();
                        sizes.estimates.push((index, estimate));
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {
                        unreachable!("the producer retains the completion sender")
                    }
                }
            }
            if canceled() || panicked {
                return;
            }
            pending += 1;
            let path = path.to_path_buf();
            let sender = sender.clone();
            let measure = &measure;
            scope.spawn(move |_| {
                // Every pending job sends exactly once, including after panic.
                // At most two remain uncollected, so this never blocks a worker
                // behind the producer, even during scope unwind or cancellation.
                sender
                    .try_send((index, measure(&path)))
                    .expect("one reserved completion slot per pending size job");
            });
        });
        // Capture this before in_place_scope waits for the sizing tail.
        sizes.canceled_after_produce = canceled();
        result
    });
    sizes.estimates.extend(receiver.try_iter());
    (result, sizes)
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

    let entry = match inspect_entry_no_follow(path, probe) {
        Ok(entry) => entry,
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
    match entry.safety {
        PathSafety::Safe => {}
        PathSafety::ReparsePoint { .. } => {
            return SizeEstimate::trusted(0, entry.modified);
        }
        PathSafety::Unverified { detail } => {
            return SizeEstimate {
                logical_bytes: None,
                complete: false,
                last_modified: entry.modified,
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
    if entry.is_file {
        return SizeEstimate::trusted(entry.len, entry.modified);
    }
    if !entry.is_dir {
        return SizeEstimate::trusted(0, entry.modified);
    }
    if depth >= max_depth {
        return SizeEstimate {
            logical_bytes: Some(0),
            complete: false,
            last_modified: entry.modified,
            warnings: vec![sizing_warning(
                SizingWarningKind::MaxDepthReached,
                format!("max depth reached at {}", path.display()),
            )],
        };
    }

    let self_mtime = entry.modified;
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

    let mut estimate = estimate_directory_children(
        path,
        entries.map(|entry| entry.map(|entry| entry.path())),
        remaining,
        depth,
        max_depth,
        cancel,
        probe,
        root_execution,
    );
    estimate.last_modified = max_mtime(estimate.last_modified, self_mtime);
    estimate
}

#[allow(clippy::too_many_arguments)]
fn estimate_directory_children(
    path: &Path,
    mut entries: impl Iterator<Item = std::io::Result<PathBuf>>,
    remaining: &mut usize,
    depth: usize,
    max_depth: usize,
    cancel: Option<&FlagCancelObserver>,
    probe: &dyn PathReparseProbe,
    root_execution: RootExecutionMode<'_>,
) -> SizeEstimate {
    let initial_remaining = *remaining;
    let mut children = Vec::new();
    let mut root_fanout = false;
    let mut estimate = SizeEstimate::trusted(0, None);
    let mut warnings = Vec::new();
    let mut entry_errors = false;
    let mut budget_hit = false;
    loop {
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            estimate.complete = false;
            warnings.push(sizing_warning(
                SizingWarningKind::Canceled,
                format!("size walk canceled under {}", path.display()),
            ));
            break;
        }
        let Some(entry) = entries.next() else { break };
        // Before streaming, enumeration preceded all visits. Preserve its
        // warning semantics when nested visits exhaust the shared budget.
        if initial_remaining == 0 {
            budget_hit = true;
            warnings.push(sizing_warning(
                SizingWarningKind::EntryBudgetExhausted,
                format!("size entry budget exhausted under {}", path.display()),
            ));
            break;
        }
        match entry {
            Ok(child) if depth == 0 => {
                children.push(child);
                root_fanout |= children.len() > 1;
                if children.len() == ROOT_CHILD_BATCH_SIZE {
                    estimate = estimate.merge(estimate_root_children(
                        path,
                        &children,
                        initial_remaining,
                        depth + 1,
                        max_depth,
                        cancel,
                        probe,
                        root_execution,
                    ));
                    children.clear();
                }
            }
            Ok(child) => {
                estimate = estimate.merge(estimate_tree_bounded(
                    &child,
                    remaining,
                    depth + 1,
                    max_depth,
                    cancel,
                    probe,
                    root_execution,
                ));
            }
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

    // Only root chunks enter the existing pool. A root with exactly one child
    // retains its larger reviewed budget; every fan-out child gets a fresh
    // local budget, including the final singleton chunk.
    // Let the cancel-aware walkers mark skipped pending bytes incomplete, even
    // when cancellation arrived during the final iterator read.
    if !children.is_empty() {
        estimate = estimate.merge(if root_fanout {
            estimate_root_children(
                path,
                &children,
                initial_remaining,
                depth + 1,
                max_depth,
                cancel,
                probe,
                root_execution,
            )
        } else {
            estimate_tree_bounded(
                &children[0],
                remaining,
                depth + 1,
                max_depth,
                cancel,
                probe,
                root_execution,
            )
        });
    }
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
                .with_max_len(1)
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

    #[derive(Default)]
    struct CountingFileProbe(AtomicUsize);

    impl PathReparseProbe for CountingFileProbe {
        fn probe(&self, _path: &Path) -> ReparseProbeResult {
            panic!("combined inspection must serve the fixture");
        }

        fn inspect_entry(&self, _path: &Path) -> Option<InspectedEntry> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Some(InspectedEntry {
                is_file: true,
                is_dir: false,
                len: 1,
                modified: None,
                safety: PathSafety::Safe,
            })
        }
    }

    #[test]
    fn wide_enumeration_retains_only_a_root_chunk_or_one_nested_name() {
        for (depth, limit) in [(0, ROOT_CHILD_BATCH_SIZE), (1, 1)] {
            let probe = CountingFileProbe::default();
            let peak = AtomicUsize::new(0);
            let entries = (0..1000).map(|index| {
                peak.fetch_max(index + 1 - probe.0.load(Ordering::SeqCst), Ordering::SeqCst);
                Ok(PathBuf::from(format!("file-{index}")))
            });
            let estimate = estimate_directory_children(
                Path::new("root"),
                entries,
                &mut 1000,
                depth,
                64,
                None,
                &probe,
                RootExecutionMode::SerialFallback,
            );
            assert!(estimate.complete);
            assert_eq!(estimate.logical_bytes, Some(1000));
            assert_eq!(probe.0.load(Ordering::SeqCst), 1000);
            assert_eq!(peak.load(Ordering::SeqCst), limit);
        }
    }

    #[test]
    fn cancellation_during_file_only_enumeration_stops_reading_names() {
        for depth in [0, 1] {
            let cancel = FlagCancelObserver::new();
            let yielded = AtomicUsize::new(0);
            let probe = CountingFileProbe::default();
            let entries = (0..10_000).map(|index| {
                if yielded.fetch_add(1, Ordering::SeqCst) == 6 {
                    cancel.request_cancel();
                }
                Ok(PathBuf::from(format!("file-{index}")))
            });
            let estimate = estimate_directory_children(
                Path::new("root"),
                entries,
                &mut 10_000,
                depth,
                64,
                Some(&cancel),
                &probe,
                RootExecutionMode::SerialFallback,
            );
            assert_eq!(yielded.load(Ordering::SeqCst), 7);
            assert!(probe.0.load(Ordering::SeqCst) <= 6);
            assert!(!estimate.complete);
            assert!(
                estimate
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == SizingWarningKind::Canceled)
            );
        }
    }

    #[test]
    fn cancellation_at_enumeration_end_keeps_pending_root_bytes_incomplete() {
        let pool = ThreadPoolBuilder::new().num_threads(2).build().unwrap();
        for mode in [
            RootExecutionMode::Dedicated(&pool),
            RootExecutionMode::SerialFallback,
        ] {
            for count in [1, 2, ROOT_CHILD_BATCH_SIZE + 1, ROOT_CHILD_BATCH_SIZE + 2] {
                let cancel = FlagCancelObserver::new();
                let probe = CountingFileProbe::default();
                let mut index = 0;
                let entries = std::iter::from_fn(|| {
                    if index == count {
                        cancel.request_cancel();
                        return None;
                    }
                    let entry = Ok(PathBuf::from(format!("file-{index}")));
                    index += 1;
                    Some(entry)
                });
                let mut remaining = count;
                let estimate = estimate_directory_children(
                    Path::new("root"),
                    entries,
                    &mut remaining,
                    0,
                    64,
                    Some(&cancel),
                    &probe,
                    mode,
                );
                let completed = count / ROOT_CHILD_BATCH_SIZE * ROOT_CHILD_BATCH_SIZE;
                assert_eq!(probe.0.load(Ordering::SeqCst), completed);
                assert_eq!(estimate.logical_bytes, Some(completed as u64));
                assert!(!estimate.complete, "pending bytes must not be trusted");
                assert!(
                    estimate
                        .warnings
                        .iter()
                        .any(|warning| warning.kind == SizingWarningKind::Canceled)
                );
            }
        }
    }

    #[test]
    fn chunk_boundaries_preserve_fresh_child_budgets_and_nested_exhaustion_warnings() {
        let probe = CountingFileProbe::default();
        for depth in [0, 1] {
            let entries = (0..ROOT_CHILD_BATCH_SIZE + 1)
                .map(|index| Ok(PathBuf::from(format!("file-{index}"))));
            let estimate = estimate_directory_children(
                Path::new("root"),
                entries,
                &mut 1,
                depth,
                64,
                None,
                &probe,
                RootExecutionMode::SerialFallback,
            );
            if depth == 0 {
                assert!(estimate.complete);
                assert_eq!(
                    estimate.logical_bytes,
                    Some((ROOT_CHILD_BATCH_SIZE + 1) as u64)
                );
                assert!(estimate.warnings.is_empty());
            } else {
                assert!(!estimate.complete);
                assert_eq!(estimate.logical_bytes, Some(1));
                assert_eq!(
                    warning_count(&estimate, SizingWarningKind::EntryBudgetExhausted),
                    ROOT_CHILD_BATCH_SIZE
                );
            }
        }
    }

    #[test]
    fn a_single_root_child_uses_the_shared_reviewed_budget() {
        let probe = CountingFileProbe::default();
        let mut remaining = DEFAULT_SIZE_ENTRY_BUDGET + 10;
        let estimate = estimate_directory_children(
            Path::new("root"),
            std::iter::once(Ok(PathBuf::from("only-child"))),
            &mut remaining,
            0,
            64,
            None,
            &probe,
            RootExecutionMode::SerialFallback,
        );
        assert!(estimate.complete);
        assert_eq!(estimate.logical_bytes, Some(1));
        assert_eq!(remaining, DEFAULT_SIZE_ENTRY_BUDGET + 9);
    }

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
    fn blocked_root_child_does_not_hide_unvisited_siblings_from_idle_worker() {
        #[derive(Default)]
        struct Progress {
            blocked_entered: bool,
            other_visits: usize,
            released: bool,
            gate_timed_out: bool,
        }

        #[derive(Default)]
        struct SiblingProbe {
            progress: Mutex<Progress>,
            changed: Condvar,
        }

        impl PathReparseProbe for SiblingProbe {
            fn probe(&self, _path: &Path) -> ReparseProbeResult {
                panic!("combined inspection must serve the fixture");
            }

            fn inspect_entry(&self, path: &Path) -> Option<InspectedEntry> {
                let mut progress = self.progress.lock().expect("sibling progress lock");
                if path == Path::new("child-0") {
                    progress.blocked_entered = true;
                    self.changed.notify_all();
                    // Bound the fixture even if the observing test thread fails.
                    let (mut progress, timeout) = self
                        .changed
                        .wait_timeout_while(progress, Duration::from_secs(10), |progress| {
                            !progress.released
                        })
                        .expect("blocked child release");
                    progress.gate_timed_out = timeout.timed_out();
                } else {
                    progress.other_visits += 1;
                    self.changed.notify_all();
                }
                Some(InspectedEntry {
                    is_file: true,
                    is_dir: false,
                    len: 1,
                    modified: None,
                    safety: PathSafety::Safe,
                })
            }
        }

        let probe = Arc::new(SiblingProbe::default());
        let worker_probe = Arc::clone(&probe);
        let handle = thread::spawn(move || {
            let pool = test_pool();
            let children = (0..ROOT_CHILD_BATCH_SIZE)
                .map(|index| PathBuf::from(format!("child-{index}")))
                .collect::<Vec<_>>();
            estimate_root_children(
                Path::new("root"),
                &children,
                DEFAULT_SIZE_ENTRY_BUDGET - 1,
                1,
                64,
                None,
                worker_probe.as_ref(),
                RootExecutionMode::Dedicated(&pool),
            )
        });
        let progress = probe.progress.lock().expect("sibling progress lock");
        let (mut progress, _) = probe
            .changed
            .wait_timeout_while(progress, Duration::from_secs(2), |progress| {
                !progress.blocked_entered || progress.other_visits < ROOT_CHILD_BATCH_SIZE - 1
            })
            .expect("available sibling progress");
        let blocked_entered = progress.blocked_entered;
        let visits_before_release = progress.other_visits;
        // Release and join before any assertion, including the timeout failure.
        progress.released = true;
        probe.changed.notify_all();
        drop(progress);
        let estimate = handle
            .join()
            .expect("size walk joined after releasing gate");
        assert!(
            !probe
                .progress
                .lock()
                .expect("sibling progress lock")
                .gate_timed_out,
            "controlled slow child timed out before explicit release"
        );
        assert!(estimate.complete);
        assert_eq!(estimate.logical_bytes, Some(ROOT_CHILD_BATCH_SIZE as u64));
        assert!(blocked_entered, "controlled slow child was never entered");
        assert_eq!(
            visits_before_release,
            ROOT_CHILD_BATCH_SIZE - 1,
            "an idle worker must visit every unblocked sibling before the slow child is released"
        );
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

    #[cfg(windows)]
    #[test]
    fn one_handle_entry_inspection_matches_metadata_plus_probe() {
        let fixture = Fixture::new();
        let file = fixture.file("root/data.bin", "twelve bytes");
        fixture.file("outside/big.bin", "not counted");
        let mut paths = vec![file, fixture.path("root")];
        let link = fixture.path("root/link");
        if create_dir_symlink(&fixture.path("outside"), &link).is_ok() {
            paths.push(link);
        }

        let probe = SystemPathReparseProbe;
        for path in &paths {
            let one_handle = probe
                .inspect_entry(path)
                .expect("one-handle inspection serves a local path");
            let metadata = fs::symlink_metadata(path).expect("metadata");
            let two_step = InspectedEntry {
                is_file: metadata.is_file(),
                is_dir: metadata.is_dir(),
                len: metadata.len(),
                modified: metadata.modified().ok(),
                safety: inspect_path_no_follow(path, &metadata, &probe),
            };
            assert_eq!(one_handle, two_step, "{}", path.display());
        }

        let missing = fixture.path("missing");
        assert!(probe.inspect_entry(&missing).is_none());
        assert!(inspect_entry_no_follow(&missing, &probe).is_err());
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

        deny_directory_read(&denied).expect("deny disposable fixture listing");
        struct RestoreDirectory(PathBuf);
        impl Drop for RestoreDirectory {
            fn drop(&mut self) {
                let _ = restore_directory_read(&self.0);
            }
        }
        let restore = RestoreDirectory(denied.clone());
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
        restore_directory_read(&denied).expect("restore disposable fixture listing");
        assert!(fs::read_dir(&denied).is_ok());
        drop(restore);
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
            .arg("*S-1-1-0:(RD)")
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
        let output = Command::new("icacls")
            .arg(path)
            .arg("/remove:d")
            .arg("*S-1-1-0")
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(String::from_utf8_lossy(
                &output.stderr,
            )))
        }
    }

    mod streaming {
        use super::*;
        use std::{
            rc::Rc,
            sync::{atomic::AtomicBool, mpsc},
        };

        #[derive(Default)]
        struct Gate {
            opened: Mutex<bool>,
            changed: Condvar,
            timed_out: AtomicBool,
        }

        impl Gate {
            fn wait(&self) {
                let open = self.opened.lock().expect("gate lock");
                let (_open, timeout) = self
                    .changed
                    .wait_timeout_while(open, Duration::from_secs(10), |open| !*open)
                    .expect("gate wait");
                self.timed_out
                    .fetch_or(timeout.timed_out(), Ordering::SeqCst);
            }

            fn release(&self) {
                *self.opened.lock().expect("gate lock") = true;
                self.changed.notify_all();
            }
        }

        struct Probe<F>(F);

        impl<F: Fn(&Path) -> u64 + Send + Sync> PathReparseProbe for Probe<F> {
            fn probe(&self, _: &Path) -> ReparseProbeResult {
                panic!("combined fixture inspection required");
            }

            fn inspect_entry(&self, path: &Path) -> Option<InspectedEntry> {
                Some(InspectedEntry {
                    is_file: true,
                    is_dir: false,
                    len: (self.0)(path),
                    modified: None,
                    safety: PathSafety::Safe,
                })
            }
        }

        fn pool() -> ThreadPool {
            ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .expect("two sizing workers")
        }

        #[test]
        fn roots_overlap_production_without_moving_the_producer_or_losing_indices() {
            let pool = pool();
            let gate = Gate::default();
            let (sent, received) = mpsc::channel();
            let producer_thread = thread::current().id();
            let not_send = Rc::new(7);
            let probe = Probe(|path: &Path| {
                let slow = path == Path::new("slow");
                if slow {
                    gate.wait();
                } else {
                    sent.send(thread::current().id())
                        .expect("probe observation");
                }
                if slow { 10 } else { 20 }
            });
            let (observation, sizes) = estimate_streamed_trees_with_mode_and_probe(
                100,
                None,
                |submit| {
                    let caller = (thread::current().id(), *not_send);
                    submit(9, Path::new("slow"));
                    submit(3, Path::new("fast"));
                    let worker = received.recv_timeout(Duration::from_secs(2));
                    // Always release before scope join or any assertion.
                    gate.release();
                    (caller, worker)
                },
                &probe,
                RootExecutionMode::Dedicated(&pool),
            );
            assert!(!gate.timed_out.load(Ordering::SeqCst));
            assert_eq!(observation.0, (producer_thread, 7));
            assert_ne!(
                observation.1.expect("sizing before producer completion"),
                producer_thread
            );
            let mut sizes: Vec<_> = sizes
                .estimates
                .into_iter()
                .map(|(index, size)| (index, size.expect("size result").logical_bytes))
                .collect();
            sizes.sort_by_key(|row| row.0);
            assert_eq!(sizes, vec![(3, Some(20)), (9, Some(10))]);
        }

        #[test]
        fn full_pipeline_cancel_and_worker_panic_release_producer_before_worker_join() {
            for panic_worker in [false, true] {
                let gates = Arc::new([Gate::default(), Gate::default()]);
                let entered = Arc::new(AtomicUsize::new(0));
                let cancel = Arc::new(FlagCancelObserver::new());
                let (started_tx, started_rx) = mpsc::channel();
                let (full_tx, full_rx) = mpsc::channel();
                let (produced_tx, produced_rx) = mpsc::channel();
                let worker_gates = Arc::clone(&gates);
                let worker_entered = Arc::clone(&entered);
                let worker_cancel = Arc::clone(&cancel);
                let handle = thread::spawn(move || {
                    let pool = pool();
                    let probe = Probe(|path: &Path| {
                        let index = path
                            .to_str()
                            .expect("numeric path")
                            .parse::<usize>()
                            .expect("path index");
                        worker_entered.fetch_add(1, Ordering::SeqCst);
                        started_tx.send(index).expect("started probe");
                        if index < 2 {
                            worker_gates[index].wait();
                        }
                        assert!(!(panic_worker && index == 0), "controlled size panic");
                        1
                    });
                    estimate_streamed_trees_with_mode_and_probe(
                        100,
                        Some(&worker_cancel),
                        |submit| {
                            submit(0, Path::new("0"));
                            submit(1, Path::new("1"));
                            full_tx.send(()).expect("two pending roots");
                            submit(2, Path::new("2"));
                            produced_tx.send(()).expect("producer resumed");
                        },
                        &probe,
                        RootExecutionMode::Dedicated(&pool),
                    )
                    .1
                });
                let both_started = started_rx.recv_timeout(Duration::from_secs(2)).is_ok()
                    && started_rx.recv_timeout(Duration::from_secs(2)).is_ok();
                let full = full_rx.recv_timeout(Duration::from_secs(2)).is_ok();
                let blocked = produced_rx.recv_timeout(Duration::from_millis(50)).is_err();
                if panic_worker {
                    gates[0].release();
                } else {
                    cancel.request_cancel();
                }
                let resumed = produced_rx.recv_timeout(Duration::from_secs(2)).is_ok();
                let joined_early = handle.is_finished();
                gates[0].release();
                gates[1].release();
                let sizes = handle.join().expect("released pipeline joined");
                assert!(
                    gates
                        .iter()
                        .all(|gate| !gate.timed_out.load(Ordering::SeqCst))
                );
                assert!(
                    both_started && full && blocked && resumed,
                    "full pipeline failed to resume"
                );
                assert!(!joined_early, "pending healthy root must be joined");
                assert_eq!(entered.load(Ordering::SeqCst), 2, "third root must not run");
                assert_eq!(sizes.canceled_after_produce, !panic_worker);
                assert_eq!(sizes.estimates.len(), 2);
                assert_eq!(
                    sizes
                        .estimates
                        .iter()
                        .filter(|(_, result)| result.is_err())
                        .count(),
                    usize::from(panic_worker)
                );
            }
        }

        #[test]
        fn producer_panic_waits_for_every_submitted_root() {
            let gate = Arc::new(Gate::default());
            let completed = Arc::new(AtomicUsize::new(0));
            let (started_tx, started_rx) = mpsc::channel();
            let worker_gate = Arc::clone(&gate);
            let worker_completed = Arc::clone(&completed);
            let handle = thread::spawn(move || {
                let pool = pool();
                let probe = Probe(|_: &Path| {
                    started_tx.send(()).expect("root started");
                    worker_gate.wait();
                    worker_completed.fetch_add(1, Ordering::SeqCst);
                    1
                });
                estimate_streamed_trees_with_mode_and_probe(
                    100,
                    None,
                    |submit| {
                        submit(0, Path::new("a"));
                        submit(1, Path::new("b"));
                        panic!("controlled enumeration panic");
                    },
                    &probe,
                    RootExecutionMode::Dedicated(&pool),
                );
            });
            let both_started = started_rx.recv_timeout(Duration::from_secs(2)).is_ok()
                && started_rx.recv_timeout(Duration::from_secs(2)).is_ok();
            let joined_early = handle.is_finished();
            gate.release();
            let outcome = handle.join();
            assert!(!gate.timed_out.load(Ordering::SeqCst));
            assert!(both_started && !joined_early);
            assert!(outcome.is_err());
            assert_eq!(completed.load(Ordering::SeqCst), 2);
        }

        #[test]
        fn streamed_walks_match_existing_walks_for_budget_reparse_and_missing_roots() {
            let fixture = Fixture::new();
            fixture.file("wide/a/file", "1234");
            fixture.file("wide/b/file", "123456");
            fixture.file("tagged/retained", "never inspected");
            let paths = [
                fixture.path("wide"),
                fixture.path("tagged"),
                fixture.path("absent"),
                fixture.path("wide"),
            ];
            let probe = FixtureReparseProbe::tagged(&paths[1]);
            let pool = pool();
            for mode in [
                RootExecutionMode::Dedicated(&pool),
                RootExecutionMode::SerialFallback,
            ] {
                let (_, sizes) = estimate_streamed_trees_with_mode_and_probe(
                    2,
                    None,
                    |submit| {
                        for (index, path) in paths.iter().enumerate() {
                            submit(index, path);
                        }
                    },
                    &probe,
                    mode,
                );
                assert!(!sizes.canceled_after_produce);
                assert_eq!(sizes.estimates.len(), paths.len());
                for (index, estimate) in sizes.estimates {
                    let expected = estimate_tree_with_limits_and_mode_and_probe(
                        &paths[index],
                        2,
                        64,
                        None,
                        &probe,
                        mode,
                    );
                    assert_estimates_equivalent(&estimate.expect("streamed estimate"), &expected);
                }
            }
        }
    }
}
