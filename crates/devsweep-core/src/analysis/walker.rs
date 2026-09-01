//! Iterative no-follow Analyze traversal with a dedicated two-worker pool.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Condvar, LazyLock, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
    filesystem::{
        PathSafety, SystemPathReparseProbe, inspect_path_no_follow, normalize_absolute_path,
    },
    process::{CancelObserver, FlagCancelObserver, NoopCancelObserver},
};

use super::model::{
    ANALYZE_SNAPSHOT_VERSION, ANALYZE_WORKERS, AnalyzeCompleteness, AnalyzeEvidence,
    AnalyzeNodeKind, AnalyzeNodeV1, AnalyzeProgressV1, AnalyzeRootIdentity, AnalyzeRunOutcome,
    AnalyzeRunStats, AnalyzeSnapshotV1, AnalyzeWarningClass, AnalyzeWarningV1, MAX_LOGICAL_DEPTH,
    MAX_STORED_NODES, MAX_WARNINGS, MemoryBudget, PROGRESS_INTERVAL_MS, PROGRESS_NODE_BATCH,
    PROGRESS_QUEUE_CAP, exact_string, node_record_bytes, string_owned_bytes, vec_owned_bytes,
    warning_record_bytes,
};

static ANALYZE_POOL: LazyLock<Result<ThreadPool, rayon::ThreadPoolBuildError>> =
    LazyLock::new(|| {
        ThreadPoolBuilder::new()
            .num_threads(ANALYZE_WORKERS)
            .thread_name(|index| format!("devsweep-analyze-{index}"))
            .build()
    });
static ANALYZE_JOB: Mutex<()> = Mutex::new(());

const WORK_ITEM_BYTES: u64 = 96;
const OBJECT_ID_ENTRY_BYTES: u64 = 48;
/// Wall-time gap between coarse worker parks. Per-node `Sleep(1)` would inflate
/// in-memory cancel tests; a 10 ms cadence still appears in every 200 ms CPU
/// sample window used by the five-mode walk-window p95 gate.
const WORKER_PARK_INTERVAL: Duration = Duration::from_millis(10);
/// Cooperative park so two dedicated workers plus coordinator stay at
/// process CPU p95 <= 200% (100% = one logical core). Windows may round this
/// up to the timer tick; cancel is checked immediately before parking.
const WORKER_PARK: Duration = Duration::from_millis(1);

#[derive(Clone, Copy)]
enum RootExecutionMode<'a> {
    Dedicated(&'a ThreadPool),
    SerialFallback,
}

fn production_execution_mode() -> RootExecutionMode<'static> {
    match ANALYZE_POOL.as_ref() {
        Ok(pool) => RootExecutionMode::Dedicated(pool),
        Err(_) => RootExecutionMode::SerialFallback,
    }
}

/// Stable object identity used to count hard-linked bytes once.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ObjectId(pub String);

/// Failure from the Analyze filesystem adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FsFail {
    AccessDenied,
    NotFound,
    Io,
}

/// Directory child name without following the child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirChild {
    pub name: String,
    pub name_len: usize,
}

impl DirChild {
    pub(crate) fn named(name: impl Into<String>) -> Self {
        let name = name.into();
        let name_len = name.len();
        Self { name, name_len }
    }

    #[cfg(test)]
    pub(crate) fn huge(name_len: usize) -> Self {
        Self {
            name: String::new(),
            name_len,
        }
    }
}

/// No-follow metadata for one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntryMeta {
    pub kind: AnalyzeNodeKind,
    pub size: u64,
    pub mtime_ms: Option<u64>,
    pub object_id: Option<ObjectId>,
    pub size_unstable: bool,
}

/// Read-only filesystem adapter used by the walker.
pub(crate) trait AnalyzeFs: Send + Sync {
    fn metadata(&self, path: &Path) -> Result<EntryMeta, FsFail>;
    fn read_dir(&self, path: &Path) -> Result<Vec<DirChild>, FsFail>;
    fn volume_id(&self, path: &Path) -> Result<String, FsFail>;
}

/// Native no-follow adapter.
#[derive(Debug, Default)]
pub(crate) struct NativeFs {
    probe: SystemPathReparseProbe,
}

impl AnalyzeFs for NativeFs {
    fn metadata(&self, path: &Path) -> Result<EntryMeta, FsFail> {
        let metadata = std::fs::symlink_metadata(path).map_err(map_io)?;
        let safety = inspect_path_no_follow(path, &metadata, &self.probe);
        let mtime_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64);
        let object_id = capture_object_id_no_follow(path);
        if unverified_open_was_denied(&safety, &object_id) {
            return Err(FsFail::AccessDenied);
        }
        let object_id = object_id.ok();
        match safety {
            PathSafety::ReparsePoint { .. } => Ok(EntryMeta {
                kind: AnalyzeNodeKind::Reparse,
                size: metadata.len(),
                mtime_ms,
                object_id,
                size_unstable: false,
            }),
            PathSafety::Unverified { .. } => Ok(EntryMeta {
                kind: AnalyzeNodeKind::Reparse,
                size: metadata.len(),
                mtime_ms,
                object_id,
                size_unstable: false,
            }),
            PathSafety::Safe if metadata.is_dir() => Ok(EntryMeta {
                kind: AnalyzeNodeKind::Directory,
                size: 0,
                mtime_ms,
                object_id,
                size_unstable: false,
            }),
            PathSafety::Safe => {
                // A second no-follow observation after the handle-based identity
                // capture detects live-file deletion and growth. Keep the smaller
                // observed length so an unstable file is always a lower bound.
                let verified = std::fs::symlink_metadata(path).map_err(map_io)?;
                let size_unstable = verified.len() != metadata.len();
                Ok(EntryMeta {
                    kind: AnalyzeNodeKind::File,
                    size: verified.len().min(metadata.len()),
                    mtime_ms,
                    object_id,
                    size_unstable,
                })
            }
        }
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<DirChild>, FsFail> {
        let entries = std::fs::read_dir(path).map_err(map_io)?;
        let mut children = Vec::new();
        for entry in entries {
            let entry = entry.map_err(map_io)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            children.push(DirChild::named(name));
        }
        children.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(children)
    }

    fn volume_id(&self, path: &Path) -> Result<String, FsFail> {
        capture_object_id_no_follow(path)
            .map(|id| volume_from_object_id(&id.0))
            .map_err(map_io)
    }
}

fn unverified_open_was_denied(safety: &PathSafety, object_id: &std::io::Result<ObjectId>) -> bool {
    matches!(safety, PathSafety::Unverified { .. })
        && matches!(
            object_id.as_ref(),
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied
        )
}

#[cfg(test)]
mod native_metadata_tests {
    use super::*;

    #[test]
    fn permission_denied_unverified_open_maps_to_access_denied_path() {
        let safety = PathSafety::Unverified {
            detail: "mocked access denial".to_string(),
        };
        let denied = Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        let unsupported = Err(std::io::Error::from(std::io::ErrorKind::Unsupported));

        assert!(unverified_open_was_denied(&safety, &denied));
        assert!(!unverified_open_was_denied(&safety, &unsupported));
        assert!(!unverified_open_was_denied(&PathSafety::Safe, &denied));
    }
}

fn map_io(error: std::io::Error) -> FsFail {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => FsFail::AccessDenied,
        std::io::ErrorKind::NotFound => FsFail::NotFound,
        _ => FsFail::Io,
    }
}

fn volume_from_object_id(value: &str) -> String {
    value
        .split_once(':')
        .map(|(volume, _)| volume.to_string())
        .unwrap_or_else(|| value.to_string())
}

fn capture_object_id_no_follow(path: &Path) -> std::io::Result<ObjectId> {
    #[cfg(windows)]
    {
        capture_windows_object_id(path)
    }
    #[cfg(unix)]
    {
        let metadata = std::fs::symlink_metadata(path)?;
        use std::os::unix::fs::MetadataExt;
        Ok(ObjectId(format!("{}:{}", metadata.dev(), metadata.ino())))
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "object identity is not implemented on this platform",
        ))
    }
}

#[cfg(windows)]
fn capture_windows_object_id(path: &Path) -> std::io::Result<ObjectId> {
    use std::{
        mem::MaybeUninit,
        os::windows::{
            ffi::OsStrExt,
            io::{AsRawHandle, FromRawHandle, OwnedHandle},
        },
        ptr,
    };

    use windows_sys::Win32::{
        Foundation::{HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, GetFileInformationByHandle, OPEN_EXISTING,
        },
    };

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            core::ptr::null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(raw as _) };
    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    let ok =
        unsafe { GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, info.as_mut_ptr()) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok(ObjectId(format!(
        "{:x}:{index:x}",
        info.dwVolumeSerialNumber
    )))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StopReason {
    None,
    Budget,
    Canceled,
}

struct WorkItem {
    parent_id: u32,
    path: PathBuf,
    name: String,
    depth: u16,
    cost: u64,
}

struct WalkState {
    nodes: Vec<AnalyzeNodeV1>,
    warnings: Vec<AnalyzeWarningV1>,
    budget: MemoryBudget,
    file_owners: HashMap<ObjectId, u32>,
    seen_dirs: HashSet<ObjectId>,
    work: VecDeque<WorkItem>,
    incomplete: HashSet<u32>,
    stop: StopReason,
    in_flight: u32,
    progress: ProgressGate,
}

struct ProgressGate {
    sequence: u64,
    pending: Vec<u32>,
    last_publish: Instant,
    queue: VecDeque<AnalyzeProgressV1>,
    max_depth: u8,
}

impl ProgressGate {
    fn new(now: Instant) -> Self {
        Self {
            sequence: 0,
            pending: Vec::new(),
            last_publish: now,
            queue: VecDeque::with_capacity(PROGRESS_QUEUE_CAP),
            max_depth: 0,
        }
    }

    fn note_changed(&mut self, id: u32) {
        if self.pending.len() < PROGRESS_NODE_BATCH {
            self.pending.push(id);
        }
    }

    fn push_batch(
        &mut self,
        changed_nodes: Vec<AnalyzeNodeV1>,
        stored_nodes: u32,
        accounted: u64,
        now: Instant,
    ) {
        self.sequence += 1;
        self.last_publish = now;
        if self.queue.len() >= PROGRESS_QUEUE_CAP {
            self.queue.pop_front();
        }
        self.queue.push_back(AnalyzeProgressV1 {
            sequence: self.sequence,
            stored_nodes,
            accounted_owned_bytes: accounted,
            changed_nodes,
            queue_depth: 0,
        });
        let depth = self.queue.len() as u8;
        self.max_depth = self.max_depth.max(depth);
        if let Some(last) = self.queue.back_mut() {
            last.queue_depth = depth;
        }
    }

    fn flush(&mut self, nodes: &[AnalyzeNodeV1], accounted: u64, now: Instant) {
        if self.pending.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending);
        let changed_nodes = pending
            .iter()
            .filter_map(|id| nodes.get(*id as usize).cloned())
            .collect::<Vec<_>>();
        self.push_batch(changed_nodes, nodes.len() as u32, accounted, now);
    }
}

/// Walks `root` with the native adapter.
pub fn analyze_path(
    root: &Path,
    cancel: Option<&FlagCancelObserver>,
    progress: Option<&mut dyn FnMut(AnalyzeProgressV1)>,
) -> Result<AnalyzeRunOutcome> {
    analyze_with_fs(root, &NativeFs::default(), cancel, progress)
}

pub(crate) fn analyze_with_fs(
    root: &Path,
    fs: &dyn AnalyzeFs,
    cancel: Option<&FlagCancelObserver>,
    progress: Option<&mut dyn FnMut(AnalyzeProgressV1)>,
) -> Result<AnalyzeRunOutcome> {
    let _job = ANALYZE_JOB
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(analyze_with_fs_and_stats_inner(root, fs, cancel, progress, None)?.0)
}

fn analyze_with_fs_and_stats_inner(
    root: &Path,
    fs: &dyn AnalyzeFs,
    cancel: Option<&FlagCancelObserver>,
    mut progress: Option<&mut dyn FnMut(AnalyzeProgressV1)>,
    execution: Option<RootExecutionMode<'_>>,
) -> Result<(AnalyzeRunOutcome, AnalyzeRunStats)> {
    let noop = NoopCancelObserver;
    let cancel: &dyn CancelObserver = cancel
        .map(|flag| flag as &dyn CancelObserver)
        .unwrap_or(&noop);
    let mode = match execution {
        Some(mode) => mode,
        None => production_execution_mode(),
    };
    let started = Instant::now();
    let mut cancel_requested_at = None;
    if cancel.is_cancel_requested() {
        cancel_requested_at = Some(Instant::now());
    }

    let input = exact_string(&root.to_string_lossy());
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to resolve current directory")?
            .join(root)
    };
    let normalized = match normalize_absolute_path(&absolute) {
        Ok(value) => value,
        Err(_) => absolute.to_string_lossy().into_owned(),
    };
    let volume = fs
        .volume_id(root)
        .map(|value| exact_string(&value))
        .unwrap_or_else(|_| exact_string("unknown"));

    let mut budget = MemoryBudget::v1();
    let root_cost = string_owned_bytes(input.capacity())
        + string_owned_bytes(normalized.capacity())
        + string_owned_bytes(volume.capacity());
    if !budget.try_charge(root_cost) {
        return Err(anyhow!("analysis root identity exceeds the memory cap"));
    }

    let identity = AnalyzeRootIdentity {
        input,
        normalized,
        volume,
    };

    let mut state = WalkState {
        nodes: Vec::new(),
        warnings: Vec::new(),
        budget,
        file_owners: HashMap::new(),
        seen_dirs: HashSet::new(),
        work: VecDeque::new(),
        incomplete: HashSet::new(),
        stop: if cancel_requested_at.is_some() {
            StopReason::Canceled
        } else {
            StopReason::None
        },
        in_flight: 0,
        progress: ProgressGate::new(started),
    };

    if state.stop == StopReason::None {
        insert_root(root, fs, cancel, &mut state);
    }

    let workers = match mode {
        RootExecutionMode::Dedicated(_) => ANALYZE_WORKERS as u8,
        RootExecutionMode::SerialFallback => 1,
    };
    let serial_fallback = matches!(mode, RootExecutionMode::SerialFallback);

    if state.stop == StopReason::None && !state.work.is_empty() {
        match mode {
            RootExecutionMode::Dedicated(pool) => {
                state = pool.install(|| {
                    let mutex = Mutex::new(state);
                    let condvar = Condvar::new();
                    rayon::scope(|scope| {
                        for _ in 0..ANALYZE_WORKERS {
                            scope.spawn(|_| {
                                worker_loop(&mutex, &condvar, fs, cancel);
                            });
                        }
                    });
                    mutex
                        .into_inner()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                });
            }
            RootExecutionMode::SerialFallback => {
                let mutex = Mutex::new(state);
                let condvar = Condvar::new();
                worker_loop(&mutex, &condvar, fs, cancel);
                state = mutex
                    .into_inner()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
    }

    let joined = Instant::now();
    let cancel_to_join_ms = cancel_requested_at
        .or_else(|| {
            if cancel.is_cancel_requested() {
                Some(started)
            } else {
                None
            }
        })
        .map(|at| joined.duration_since(at).as_millis() as u64);

    if cancel.is_cancel_requested() && state.stop == StopReason::None {
        state.stop = StopReason::Canceled;
    }

    finalize_rollup(&mut state);
    let max_progress_queue_depth = state.progress.max_depth;
    let peak_accounted_owned_bytes = state.budget.peak();
    state
        .progress
        .flush(&state.nodes, state.budget.used(), Instant::now());
    if let Some(emit) = progress.as_mut() {
        for batch in state.progress.queue.drain(..) {
            emit(batch);
        }
    }

    let file_refund = state.file_owners.len() as u64 * OBJECT_ID_ENTRY_BYTES;
    let dir_refund = state.seen_dirs.len() as u64 * OBJECT_ID_ENTRY_BYTES;
    state.budget.refund(file_refund.saturating_add(dir_refund));
    while let Some(item) = state.work.pop_front() {
        state.budget.refund(item.cost);
    }

    let completeness = match state.stop {
        StopReason::Budget => AnalyzeCompleteness::PartialBudget,
        StopReason::Canceled => AnalyzeCompleteness::Canceled,
        StopReason::None => AnalyzeCompleteness::Complete,
    };
    let mut snapshot = AnalyzeSnapshotV1 {
        version: ANALYZE_SNAPSHOT_VERSION,
        root: identity,
        nodes: state.nodes,
        warnings: state.warnings,
        completeness,
        accounted_owned_bytes: 0,
    };
    snapshot.accounted_owned_bytes = snapshot.snapshot_owned_bytes();
    let stats = AnalyzeRunStats {
        workers,
        serial_fallback,
        max_progress_queue_depth,
        peak_accounted_owned_bytes,
        cancel_to_join_ms,
    };
    let outcome = match completeness {
        AnalyzeCompleteness::Canceled => AnalyzeRunOutcome::Canceled { snapshot },
        _ => AnalyzeRunOutcome::Completed { snapshot },
    };
    Ok((outcome, stats))
}

#[cfg(test)]
pub(crate) fn analyze_with_fs_and_stats(
    root: &Path,
    fs: &dyn AnalyzeFs,
    cancel: Option<&FlagCancelObserver>,
    progress: Option<&mut dyn FnMut(AnalyzeProgressV1)>,
    serial: bool,
) -> Result<(AnalyzeRunOutcome, AnalyzeRunStats)> {
    let _job = ANALYZE_JOB
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mode = if serial {
        Some(RootExecutionMode::SerialFallback)
    } else {
        Some(production_execution_mode())
    };
    analyze_with_fs_and_stats_inner(root, fs, cancel, progress, mode)
}

fn insert_root(
    root: &Path,
    fs: &dyn AnalyzeFs,
    cancel: &dyn CancelObserver,
    state: &mut WalkState,
) {
    if cancel.is_cancel_requested() {
        state.stop = StopReason::Canceled;
        return;
    }
    let name = root
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned());
    match observe_path(root, fs, cancel) {
        Observe::Canceled => state.stop = StopReason::Canceled,
        Observe::Meta(meta) => {
            let Some(id) = try_store_node(state, None, &name, meta.kind, meta.mtime_ms) else {
                return;
            };
            apply_meta(state, id, &meta, root);
            if meta.kind == AnalyzeNodeKind::Directory && state.stop == StopReason::None {
                schedule_children(state, fs, cancel, id, root, 0);
            }
        }
        Observe::Fail(fail) => {
            let Some(id) = try_store_node(state, None, &name, AnalyzeNodeKind::Directory, None)
            else {
                return;
            };
            mark_fail(state, id, fail);
        }
    }
}

enum Observe {
    Meta(EntryMeta),
    Fail(FsFail),
    Canceled,
}

fn observe_path(path: &Path, fs: &dyn AnalyzeFs, cancel: &dyn CancelObserver) -> Observe {
    if cancel.is_cancel_requested() {
        return Observe::Canceled;
    }
    match fs.metadata(path) {
        Ok(meta) => Observe::Meta(meta),
        Err(fail) => Observe::Fail(fail),
    }
}

fn try_store_node(
    state: &mut WalkState,
    parent_id: Option<u32>,
    name: &str,
    kind: AnalyzeNodeKind,
    mtime_ms: Option<u64>,
) -> Option<u32> {
    if state.nodes.len() as u32 >= MAX_STORED_NODES {
        hit_budget(state, parent_id);
        return None;
    }
    let name = exact_string(name);
    let node_cost = node_record_bytes() + string_owned_bytes(name.capacity());
    let growth = vec_growth_cost(
        size_of_val_node(),
        state.nodes.len(),
        state.nodes.capacity(),
    );
    if !state.budget.try_charge(node_cost.saturating_add(growth)) {
        hit_budget(state, parent_id);
        return None;
    }
    let id = state.nodes.len() as u32;
    state.nodes.push(AnalyzeNodeV1 {
        id,
        parent_id,
        kind,
        name,
        bytes: 0,
        immediate_count: 0,
        recursive_count: 0,
        evidence: AnalyzeEvidence::Complete,
        warnings: Vec::new(),
        mtime_ms,
    });
    if let Some(parent) = parent_id
        && let Some(parent_node) = state.nodes.get_mut(parent as usize)
    {
        parent_node.immediate_count = parent_node.immediate_count.saturating_add(1);
    }
    state.progress.note_changed(id);
    Some(id)
}

fn size_of_val_node() -> usize {
    std::mem::size_of::<AnalyzeNodeV1>()
}

fn vec_growth_cost(element_size: usize, len: usize, capacity: usize) -> u64 {
    if len < capacity {
        0
    } else if capacity == 0 {
        vec_owned_bytes(element_size, 4)
    } else {
        vec_owned_bytes(element_size, capacity)
    }
}

fn apply_meta(state: &mut WalkState, id: u32, meta: &EntryMeta, path: &Path) {
    if meta.size_unstable {
        attach_warning(state, id, AnalyzeWarningClass::Churn);
        state.incomplete.insert(id);
    }
    match meta.kind {
        AnalyzeNodeKind::Reparse => {
            attach_warning(state, id, AnalyzeWarningClass::Reparse);
            if let Some(node) = state.nodes.get_mut(id as usize) {
                node.bytes = meta.size;
                node.kind = AnalyzeNodeKind::Reparse;
                node.mtime_ms = meta.mtime_ms;
            }
        }
        AnalyzeNodeKind::File => {
            if let Some(node) = state.nodes.get_mut(id as usize) {
                node.kind = AnalyzeNodeKind::File;
                node.mtime_ms = meta.mtime_ms;
            }
            if let Some(object_id) = &meta.object_id {
                if let Some(&owner) = state.file_owners.get(object_id) {
                    attach_warning(state, id, AnalyzeWarningClass::DuplicateLink);
                    let _ = owner;
                } else if state.budget.try_charge(OBJECT_ID_ENTRY_BYTES) {
                    state.file_owners.insert(object_id.clone(), id);
                    if let Some(node) = state.nodes.get_mut(id as usize) {
                        node.bytes = meta.size;
                    }
                } else {
                    hit_budget(state, Some(id));
                }
            } else if let Some(node) = state.nodes.get_mut(id as usize) {
                node.bytes = meta.size;
            }
        }
        AnalyzeNodeKind::Directory => {
            if let Some(node) = state.nodes.get_mut(id as usize) {
                node.kind = AnalyzeNodeKind::Directory;
                node.mtime_ms = meta.mtime_ms;
            }
            if let Some(object_id) = &meta.object_id {
                if !state.seen_dirs.insert(object_id.clone()) {
                    attach_warning(state, id, AnalyzeWarningClass::Cycle);
                    state.incomplete.insert(id);
                    if let Some(node) = state.nodes.get_mut(id as usize) {
                        node.evidence = AnalyzeEvidence::Incomplete;
                    }
                    return;
                }
                if !state.budget.try_charge(OBJECT_ID_ENTRY_BYTES) {
                    state.seen_dirs.remove(object_id);
                    hit_budget(state, Some(id));
                    return;
                }
            }
            let _ = path;
        }
    }
}

fn mark_fail(state: &mut WalkState, id: u32, fail: FsFail) {
    let class = match fail {
        FsFail::AccessDenied => AnalyzeWarningClass::AccessDenied,
        FsFail::NotFound => AnalyzeWarningClass::Churn,
        FsFail::Io => AnalyzeWarningClass::IoError,
    };
    attach_warning(state, id, class);
    state.incomplete.insert(id);
    if let Some(node) = state.nodes.get_mut(id as usize) {
        node.evidence = match fail {
            FsFail::NotFound => AnalyzeEvidence::Unknown,
            _ => AnalyzeEvidence::Incomplete,
        };
    }
}

fn attach_warning(state: &mut WalkState, node_id: u32, class: AnalyzeWarningClass) {
    if let Some(node) = state.nodes.get_mut(node_id as usize)
        && !node.warnings.contains(&class)
    {
        let warn_growth = if node.warnings.len() == node.warnings.capacity() {
            vec_growth_cost(
                std::mem::size_of::<AnalyzeWarningClass>(),
                node.warnings.len(),
                node.warnings.capacity(),
            )
        } else {
            0
        };
        if !state.budget.try_charge(warn_growth) {
            hit_budget(state, Some(node_id));
            return;
        }
        node.warnings.push(class);
    }
    if state.warnings.len() as u32 >= MAX_WARNINGS {
        if class == AnalyzeWarningClass::PartialBudget {
            return;
        }
        hit_budget(state, Some(node_id));
        return;
    }
    let growth = vec_growth_cost(
        std::mem::size_of::<AnalyzeWarningV1>(),
        state.warnings.len(),
        state.warnings.capacity(),
    );
    if !state
        .budget
        .try_charge(warning_record_bytes().saturating_add(growth))
    {
        hit_budget(state, Some(node_id));
        return;
    }
    state.warnings.push(AnalyzeWarningV1 {
        class,
        node_id: Some(node_id),
    });
}

fn hit_budget(state: &mut WalkState, nearest: Option<u32>) {
    if state.stop == StopReason::None {
        state.stop = StopReason::Budget;
    }
    if let Some(id) = nearest {
        mark_incomplete_chain(state, id);
        if !state
            .warnings
            .iter()
            .any(|warning| warning.class == AnalyzeWarningClass::PartialBudget)
            && (state.warnings.len() as u32) < MAX_WARNINGS
        {
            let growth = vec_growth_cost(
                std::mem::size_of::<AnalyzeWarningV1>(),
                state.warnings.len(),
                state.warnings.capacity(),
            );
            if state
                .budget
                .try_charge(warning_record_bytes().saturating_add(growth))
            {
                state.warnings.push(AnalyzeWarningV1 {
                    class: AnalyzeWarningClass::PartialBudget,
                    node_id: Some(id),
                });
                if let Some(node) = state.nodes.get_mut(id as usize)
                    && !node.warnings.contains(&AnalyzeWarningClass::PartialBudget)
                {
                    node.warnings.push(AnalyzeWarningClass::PartialBudget);
                }
            }
        }
    }
    state.work.clear();
}

fn mark_incomplete_chain(state: &mut WalkState, mut id: u32) {
    loop {
        state.incomplete.insert(id);
        if let Some(node) = state.nodes.get_mut(id as usize) {
            if node.evidence == AnalyzeEvidence::Complete {
                node.evidence = AnalyzeEvidence::Incomplete;
            }
            match node.parent_id {
                Some(parent) => id = parent,
                None => break,
            }
        } else {
            break;
        }
    }
}

fn schedule_children(
    state: &mut WalkState,
    fs: &dyn AnalyzeFs,
    cancel: &dyn CancelObserver,
    parent_id: u32,
    path: &Path,
    depth: u16,
) {
    if cancel.is_cancel_requested() {
        state.stop = StopReason::Canceled;
        mark_incomplete_chain(state, parent_id);
        return;
    }
    if depth >= MAX_LOGICAL_DEPTH {
        mark_incomplete_chain(state, parent_id);
        return;
    }
    if state.stop != StopReason::None {
        mark_incomplete_chain(state, parent_id);
        return;
    }
    match fs.read_dir(path) {
        Ok(children) => {
            if children.is_empty() {
                return;
            }
            for child in children {
                if state.stop != StopReason::None {
                    mark_incomplete_chain(state, parent_id);
                    return;
                }
                let name_cost = string_owned_bytes(child.name_len);
                if child.name_len > 0 && child.name.is_empty() {
                    if !state.budget.try_charge(name_cost) {
                        hit_budget(state, Some(parent_id));
                        return;
                    }
                    state.budget.refund(name_cost);
                    hit_budget(state, Some(parent_id));
                    return;
                }
                let child_path = path.join(&child.name);
                let path_cost = string_owned_bytes(child_path.as_os_str().len());
                let item_cost = WORK_ITEM_BYTES + name_cost + path_cost;
                if !state.budget.try_charge(item_cost) {
                    hit_budget(state, Some(parent_id));
                    return;
                }
                state.work.push_back(WorkItem {
                    parent_id,
                    path: child_path,
                    name: exact_string(&child.name),
                    depth: depth.saturating_add(1),
                    cost: item_cost,
                });
            }
        }
        Err(fail) => mark_fail(state, parent_id, fail),
    }
}

fn request_worker_cancel(mutex: &Mutex<WalkState>, condvar: &Condvar) {
    let mut state = mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.stop = StopReason::Canceled;
    state.work.clear();
    condvar.notify_all();
}

fn maybe_park_busy_worker(last_park: &mut Instant, cancel: &dyn CancelObserver) {
    if cancel.is_cancel_requested() {
        return;
    }
    if last_park.elapsed() < WORKER_PARK_INTERVAL {
        return;
    }
    std::thread::sleep(WORKER_PARK);
    *last_park = Instant::now();
}

fn worker_loop(
    mutex: &Mutex<WalkState>,
    condvar: &Condvar,
    fs: &dyn AnalyzeFs,
    cancel: &dyn CancelObserver,
) {
    let mut last_park = Instant::now();
    loop {
        if cancel.is_cancel_requested() {
            request_worker_cancel(mutex, condvar);
            return;
        }
        let item = {
            let mut state = mutex
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            loop {
                if state.stop != StopReason::None {
                    return;
                }
                if let Some(item) = state.work.pop_front() {
                    state.in_flight += 1;
                    break item;
                }
                if state.in_flight == 0 {
                    condvar.notify_all();
                    return;
                }
                state = condvar
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };
        process_item(mutex, condvar, fs, cancel, item);
        maybe_park_busy_worker(&mut last_park, cancel);
    }
}

fn process_item(
    mutex: &Mutex<WalkState>,
    condvar: &Condvar,
    fs: &dyn AnalyzeFs,
    cancel: &dyn CancelObserver,
    item: WorkItem,
) {
    if cancel.is_cancel_requested() {
        let mut state = mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.stop = StopReason::Canceled;
        state.budget.refund(item.cost);
        state.in_flight = state.in_flight.saturating_sub(1);
        condvar.notify_all();
        return;
    }
    let observed = observe_path(&item.path, fs, cancel);
    let mut state = mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.budget.refund(item.cost);
    match observed {
        Observe::Canceled => {
            state.stop = StopReason::Canceled;
            mark_incomplete_chain(&mut state, item.parent_id);
        }
        Observe::Fail(fail) => {
            if state.stop == StopReason::None
                && let Some(id) = try_store_node(
                    &mut state,
                    Some(item.parent_id),
                    &item.name,
                    AnalyzeNodeKind::File,
                    None,
                )
            {
                mark_fail(&mut state, id, fail);
            } else if state.stop != StopReason::None {
                mark_incomplete_chain(&mut state, item.parent_id);
            }
        }
        Observe::Meta(meta) => {
            if state.stop == StopReason::None
                && let Some(id) = try_store_node(
                    &mut state,
                    Some(item.parent_id),
                    &item.name,
                    meta.kind,
                    meta.mtime_ms,
                )
            {
                apply_meta(&mut state, id, &meta, &item.path);
                if meta.kind == AnalyzeNodeKind::Directory
                    && !state.incomplete.contains(&id)
                    && state.stop == StopReason::None
                {
                    schedule_children(&mut state, fs, cancel, id, &item.path, item.depth);
                }
            } else {
                mark_incomplete_chain(&mut state, item.parent_id);
            }
        }
    }
    let accounted = state.budget.used();
    let now = Instant::now();
    let stored_nodes = state.nodes.len() as u32;
    let due = state.progress.pending.len() >= PROGRESS_NODE_BATCH
        || now.duration_since(state.progress.last_publish)
            >= Duration::from_millis(PROGRESS_INTERVAL_MS);
    if due && !state.progress.pending.is_empty() {
        let ids = std::mem::take(&mut state.progress.pending);
        let changed_nodes = ids
            .iter()
            .filter_map(|id| state.nodes.get(*id as usize).cloned())
            .collect::<Vec<_>>();
        state
            .progress
            .push_batch(changed_nodes, stored_nodes, accounted, now);
    }
    state.in_flight = state.in_flight.saturating_sub(1);
    condvar.notify_all();
}

fn finalize_rollup(state: &mut WalkState) {
    for id in (0..state.nodes.len()).rev() {
        let parent_id = state.nodes[id].parent_id;
        let bytes = state.nodes[id].bytes;
        let recursive = state.nodes[id].recursive_count;
        if state.incomplete.contains(&(id as u32)) {
            state.nodes[id].evidence = match state.nodes[id].evidence {
                AnalyzeEvidence::Unknown => AnalyzeEvidence::Unknown,
                _ => AnalyzeEvidence::Incomplete,
            };
        }
        if let Some(parent) = parent_id
            && let Some(parent_node) = state.nodes.get_mut(parent as usize)
        {
            parent_node.bytes = parent_node.bytes.saturating_add(bytes);
            parent_node.recursive_count = parent_node
                .recursive_count
                .saturating_add(1)
                .saturating_add(recursive);
            if state.incomplete.contains(&(id as u32)) {
                state.incomplete.insert(parent);
            }
        }
    }
    for id in &state.incomplete {
        if let Some(node) = state.nodes.get_mut(*id as usize)
            && node.evidence == AnalyzeEvidence::Complete
        {
            node.evidence = AnalyzeEvidence::Incomplete;
        }
    }
}

#[cfg(test)]
pub(crate) use test_fs::{Fake250k, MapFs, dir_entry, file_entry, reparse_entry};

#[cfg(test)]
mod test_fs {
    use super::*;

    #[derive(Debug, Clone)]
    pub(crate) struct Fake250k {
        extra_overflow_node: bool,
        huge_name_len: Option<usize>,
    }

    impl Fake250k {
        pub(crate) fn analysis_250k_v1() -> Self {
            Self {
                extra_overflow_node: false,
                huge_name_len: None,
            }
        }

        pub(crate) fn with_overflow_node() -> Self {
            Self {
                extra_overflow_node: true,
                huge_name_len: None,
            }
        }

        pub(crate) fn with_huge_name(name_len: usize) -> Self {
            Self {
                extra_overflow_node: false,
                huge_name_len: Some(name_len),
            }
        }

        pub(crate) fn root() -> PathBuf {
            PathBuf::from("analyze-250k-v1")
        }
    }

    const WIDE_CHILDREN: u32 = 10_000;
    const DEEP_DEPTH: u16 = 1_024;
    const FILL_BUCKETS: u32 = 1_000;

    fn special_node_count() -> u32 {
        1 + 1 + WIDE_CHILDREN + u32::from(DEEP_DEPTH) + 1 + 2 + 1 + 1 + 1
    }

    fn fill_file_count() -> u32 {
        MAX_STORED_NODES - special_node_count() - 1 - FILL_BUCKETS
    }

    impl AnalyzeFs for Fake250k {
        fn metadata(&self, path: &Path) -> Result<EntryMeta, FsFail> {
            let parts = path_parts(path);
            if parts.is_empty() {
                return Err(FsFail::Io);
            }
            match classify(&parts) {
                FakeKind::Root
                | FakeKind::Wide
                | FakeKind::Deep(_)
                | FakeKind::Links
                | FakeKind::ChurnDir
                | FakeKind::Fill
                | FakeKind::FillBucket(_)
                | FakeKind::Denied => Ok(dir_meta(&parts)),
                FakeKind::WideChild(_) | FakeKind::FillFile { .. } | FakeKind::Overflow => {
                    Ok(file_meta(&parts, 1))
                }
                FakeKind::DeepLeaf => Ok(file_meta(&parts, 2)),
                FakeKind::LinkA | FakeKind::LinkB => Ok(EntryMeta {
                    kind: AnalyzeNodeKind::File,
                    size: 4096,
                    mtime_ms: Some(1),
                    object_id: Some(ObjectId("vol:link".to_string())),
                    size_unstable: false,
                }),
                FakeKind::ChurnFile => Ok(EntryMeta {
                    kind: AnalyzeNodeKind::File,
                    size: 8,
                    mtime_ms: Some(1),
                    object_id: Some(ObjectId(format!("vol:{}", parts.join("/")))),
                    size_unstable: true,
                }),
                FakeKind::Unknown => Err(FsFail::NotFound),
            }
        }

        fn read_dir(&self, path: &Path) -> Result<Vec<DirChild>, FsFail> {
            let parts = path_parts(path);
            match classify(&parts) {
                FakeKind::Denied => Err(FsFail::AccessDenied),
                FakeKind::Root => {
                    let mut children = vec![
                        DirChild::named("wide"),
                        DirChild::named("deep"),
                        DirChild::named("links"),
                        DirChild::named("denied"),
                        DirChild::named("churn"),
                        DirChild::named("fill"),
                    ];
                    if let Some(len) = self.huge_name_len {
                        children.push(DirChild::huge(len));
                    }
                    Ok(children)
                }
                FakeKind::Wide => Ok((0..WIDE_CHILDREN)
                    .map(|index| DirChild::named(format!("c{index:05}")))
                    .collect()),
                FakeKind::Deep(depth) if depth < DEEP_DEPTH => {
                    Ok(vec![DirChild::named(format!("d{}", depth + 1))])
                }
                FakeKind::Deep(_) => Ok(Vec::new()),
                FakeKind::Links => Ok(vec![DirChild::named("a"), DirChild::named("b")]),
                FakeKind::ChurnDir => Ok(vec![DirChild::named("file")]),
                FakeKind::Fill => {
                    let mut children = (0..FILL_BUCKETS)
                        .map(|index| DirChild::named(format!("b{index:04}")))
                        .collect::<Vec<_>>();
                    if self.extra_overflow_node {
                        children.push(DirChild::named("overflow"));
                    }
                    Ok(children)
                }
                FakeKind::FillBucket(bucket) => {
                    let total = fill_file_count();
                    let base = total / FILL_BUCKETS;
                    let extra = total % FILL_BUCKETS;
                    let count = base + u32::from(bucket < extra);
                    Ok((0..count)
                        .map(|index| DirChild::named(format!("f{index:04}")))
                        .collect())
                }
                _ => Err(FsFail::Io),
            }
        }

        fn volume_id(&self, _path: &Path) -> Result<String, FsFail> {
            Ok("vol".to_string())
        }
    }

    fn path_parts(path: &Path) -> Vec<String> {
        path.iter()
            .map(|component| component.to_string_lossy().into_owned())
            .collect()
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FakeKind {
        Root,
        Wide,
        WideChild(u32),
        Deep(u16),
        DeepLeaf,
        Links,
        LinkA,
        LinkB,
        Denied,
        ChurnDir,
        ChurnFile,
        Fill,
        FillBucket(u32),
        FillFile { bucket: u32, file: u32 },
        Overflow,
        Unknown,
    }

    fn classify(parts: &[String]) -> FakeKind {
        if parts.first().map(String::as_str) != Some("analyze-250k-v1") {
            return FakeKind::Unknown;
        }
        match parts.len() {
            1 => FakeKind::Root,
            2 => match parts[1].as_str() {
                "wide" => FakeKind::Wide,
                "deep" => FakeKind::Deep(1),
                "links" => FakeKind::Links,
                "denied" => FakeKind::Denied,
                "churn" => FakeKind::ChurnDir,
                "fill" => FakeKind::Fill,
                _ => FakeKind::Unknown,
            },
            3 => match parts[1].as_str() {
                "wide" => parts[2]
                    .strip_prefix('c')
                    .and_then(|value| value.parse().ok())
                    .map(FakeKind::WideChild)
                    .unwrap_or(FakeKind::Unknown),
                "deep" => FakeKind::Deep(2),
                "links" if parts[2] == "a" => FakeKind::LinkA,
                "links" if parts[2] == "b" => FakeKind::LinkB,
                "churn" if parts[2] == "file" => FakeKind::ChurnFile,
                "fill" if parts[2] == "overflow" => FakeKind::Overflow,
                "fill" => parts[2]
                    .strip_prefix('b')
                    .and_then(|value| value.parse().ok())
                    .map(FakeKind::FillBucket)
                    .unwrap_or(FakeKind::Unknown),
                _ => FakeKind::Unknown,
            },
            n if parts[1] == "deep" && n <= usize::from(DEEP_DEPTH) + 1 => {
                FakeKind::Deep((n - 1) as u16)
            }
            n if parts[1] == "deep" && n == usize::from(DEEP_DEPTH) + 2 => FakeKind::DeepLeaf,
            4 if parts[1] == "fill" => {
                let bucket = parts[2]
                    .strip_prefix('b')
                    .and_then(|value| value.parse().ok());
                let file = parts[3]
                    .strip_prefix('f')
                    .and_then(|value| value.parse().ok());
                match (bucket, file) {
                    (Some(bucket), Some(file)) => FakeKind::FillFile { bucket, file },
                    _ => FakeKind::Unknown,
                }
            }
            _ => FakeKind::Unknown,
        }
    }

    fn dir_meta(parts: &[String]) -> EntryMeta {
        EntryMeta {
            kind: AnalyzeNodeKind::Directory,
            size: 0,
            mtime_ms: Some(1),
            object_id: Some(ObjectId(format!("dir:{}", parts.join("/")))),
            size_unstable: false,
        }
    }

    fn file_meta(parts: &[String], size: u64) -> EntryMeta {
        EntryMeta {
            kind: AnalyzeNodeKind::File,
            size,
            mtime_ms: Some(1),
            object_id: Some(ObjectId(format!("file:{}", parts.join("/")))),
            size_unstable: false,
        }
    }

    #[derive(Debug, Default)]
    pub(crate) struct MapFs {
        pub meta: HashMap<PathBuf, Result<EntryMeta, FsFail>>,
        pub dirs: HashMap<PathBuf, Result<Vec<DirChild>, FsFail>>,
        pub volume: String,
    }

    impl AnalyzeFs for MapFs {
        fn metadata(&self, path: &Path) -> Result<EntryMeta, FsFail> {
            self.meta
                .get(path)
                .cloned()
                .unwrap_or(Err(FsFail::NotFound))
        }

        fn read_dir(&self, path: &Path) -> Result<Vec<DirChild>, FsFail> {
            self.dirs
                .get(path)
                .cloned()
                .unwrap_or(Err(FsFail::NotFound))
        }

        fn volume_id(&self, _path: &Path) -> Result<String, FsFail> {
            Ok(self.volume.clone())
        }
    }

    pub(crate) fn file_entry(size: u64, id: &str) -> EntryMeta {
        EntryMeta {
            kind: AnalyzeNodeKind::File,
            size,
            mtime_ms: Some(1_700_000_000_000),
            object_id: Some(ObjectId(id.to_string())),
            size_unstable: false,
        }
    }

    pub(crate) fn dir_entry(id: &str) -> EntryMeta {
        EntryMeta {
            kind: AnalyzeNodeKind::Directory,
            size: 0,
            mtime_ms: Some(1_700_000_000_000),
            object_id: Some(ObjectId(id.to_string())),
            size_unstable: false,
        }
    }

    pub(crate) fn reparse_entry(size: u64, id: &str) -> EntryMeta {
        EntryMeta {
            kind: AnalyzeNodeKind::Reparse,
            size,
            mtime_ms: Some(1),
            object_id: Some(ObjectId(id.to_string())),
            size_unstable: false,
        }
    }
}
