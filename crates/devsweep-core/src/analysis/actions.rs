//! Analyze reveal and confirmed Recycle Bin move over a retained snapshot.
//!
//! The walker and snapshot stay read-only. This module rebuilds node paths
//! from a snapshot that the caller retained for one operation id, launches the
//! Explorer reveal through [`ProcessRunner`], and builds the `analyze.trash`
//! plan that the Clean [`Executor`] runs with the fixed Clean audit journal.
//! Permanent delete is never used.

use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsString,
    fmt,
    path::{MAIN_SEPARATOR_STR, Path, PathBuf},
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::{AnalyzeEvidence, AnalyzeNodeKind, AnalyzeNodeV1, AnalyzeSnapshotV1};
use crate::{
    execution::{
        CommandRunner, ConfirmationDigest, ExecutionReport, ExecutionRequest, Executor,
        OutcomeStatus, TrashRunner, UserProtectionList, confirmation_digest, resolve_home_dir,
    },
    filesystem::{
        PathSafety, SystemPathReparseProbe, inspect_entry_no_follow, normalize_absolute_path,
        path_contains_path, path_is_within, paths_equal,
    },
    model::{
        CleanAction, CleanTarget, Ecosystem, Evidence, RiskLevel, Scope, TargetId, TargetKind,
    },
    plan::{ValidatedPlan, validated_trash_plan},
    process::{
        CancelObserver, CwdPolicy, FlagCancelObserver, NoopCancelObserver, ProcessRequest,
        ProcessResult, ProcessRunner, ProcessStatus,
    },
};

/// Analyze Recycle Bin preview and report wire version.
pub const ANALYZE_TRASH_VERSION: u32 = 1;
/// Rule id of the in-memory Analyze Recycle Bin plan. The rules registry has
/// no entry for it, so a plan file can never carry this authority.
pub const ANALYZE_TRASH_RULE_ID: &str = "analyze.trash";

const EXPLORER_PROGRAM: &str = "explorer.exe";
const EXPLORER_SELECT_ARG: &str = "/select,";
const REVEAL_TIMEOUT: Duration = Duration::from_secs(10);
const SYSTEM_LOCATION_VARS: [&str; 6] = [
    "WINDIR",
    "SystemRoot",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramW6432",
    "ProgramData",
];
/// Profile children that the Clean safety policy treats as exact nodes.
const PROFILE_EXACT_CHILDREN: [&str; 3] = ["Desktop", "Documents", "Downloads"];
/// Profile subtrees that the Clean safety policy never moves.
const PROFILE_PROTECTED_SUBTREES: [&str; 8] = [
    ".ssh",
    ".aws",
    ".gnupg",
    ".docker",
    ".cargo/credentials.toml",
    ".cargo/credentials",
    ".npmrc",
    ".config/gh",
];

/// Why one selected node stays view only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeTrashRefusalCode {
    /// On or overlapping the protection list or a Clean protected subtree.
    Protected,
    /// Overlaps a Windows or program installation location.
    SystemLocation,
    /// A volume root.
    VolumeRoot,
    /// The user profile, a folder that contains it, or a top-level profile folder.
    ProfileRoot,
    /// The analysis root.
    AnalysisRoot,
    /// A reparse point in the snapshot or on disk now.
    ReparsePoint,
    /// Type or modification time differs from the snapshot.
    ChangedSinceSnapshot,
    /// The node id is unknown or the path is missing now.
    NotFound,
}

/// One refused node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeTrashRefusalV1 {
    pub node_id: u32,
    pub reason_code: AnalyzeTrashRefusalCode,
}

/// One accepted node in the Recycle Bin plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeTrashItemV1 {
    pub node_id: u32,
    pub target_id: TargetId,
    /// Display only. The desktop never sends this value back.
    pub path: PathBuf,
    pub kind: AnalyzeNodeKind,
    pub bytes: u64,
    pub evidence: AnalyzeEvidence,
}

/// Preview of one Analyze Recycle Bin plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeTrashPreviewV1 {
    pub version: u32,
    pub operation_id: String,
    pub items: Vec<AnalyzeTrashItemV1>,
    pub refused: Vec<AnalyzeTrashRefusalV1>,
    /// Clean confirmation digest of the accepted items.
    pub digest: ConfirmationDigest,
}

/// Result of one Analyze Recycle Bin move. Bytes are "moved to the Recycle
/// Bin", never freed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeTrashReportV1 {
    pub version: u32,
    pub operation_id: String,
    pub moved_node_ids: Vec<u32>,
    pub report: ExecutionReport,
}

/// Execution request. Confirmation must be explicit.
#[derive(Debug, Clone)]
pub struct AnalyzeTrashExecutionRequest<'a> {
    pub operation_id: &'a str,
    pub snapshot: &'a AnalyzeSnapshotV1,
    pub node_ids: &'a [u32],
    pub expected_digest: &'a str,
    pub confirmed: bool,
    pub cancel: Option<Arc<FlagCancelObserver>>,
}

/// Failures before or around an Analyze action.
#[derive(Debug)]
pub enum AnalyzeActionError {
    UnknownNode(u32),
    InvalidSnapshot(u32),
    EmptySelection,
    NothingToMove,
    ConfirmationRequired,
    ProtectionUnavailable,
    RevealUnavailable,
    Plan(anyhow::Error),
    /// Clean executor error, including a stale confirmation digest.
    Execution(anyhow::Error),
}

impl fmt::Display for AnalyzeActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(formatter, "unknown analysis node {id}"),
            Self::InvalidSnapshot(id) => {
                write!(formatter, "analysis snapshot path for node {id} is invalid")
            }
            Self::EmptySelection => formatter.write_str("analysis selection is empty"),
            Self::NothingToMove => {
                formatter.write_str("no selected analysis node can move to the Recycle Bin")
            }
            Self::ConfirmationRequired => {
                formatter.write_str("the Recycle Bin move requires explicit confirmation")
            }
            Self::ProtectionUnavailable => formatter
                .write_str("protection list is unavailable; the Recycle Bin move is refused"),
            Self::RevealUnavailable => formatter.write_str("Windows Explorer could not start"),
            Self::Plan(error) | Self::Execution(error) => write!(formatter, "{error:#}"),
        }
    }
}

impl std::error::Error for AnalyzeActionError {}

/// Default Analyze root: the system drive (`%SystemDrive%\`) on Windows and
/// `/` elsewhere.
pub fn default_analyze_root() -> PathBuf {
    if cfg!(windows) {
        let mut drive = env::var_os("SystemDrive")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| OsString::from("C:"));
        drive.push(MAIN_SEPARATOR_STR);
        PathBuf::from(drive)
    } else {
        PathBuf::from("/")
    }
}

/// Rebuilds the full path of one node from the snapshot root and node names.
pub fn analyze_node_path(
    snapshot: &AnalyzeSnapshotV1,
    node_id: u32,
) -> Result<PathBuf, AnalyzeActionError> {
    NodeIndex::new(snapshot)?.path(node_id)
}

/// Shows one node in Windows Explorer. Allowed for every node.
pub fn reveal_analyze_node(
    snapshot: &AnalyzeSnapshotV1,
    node_id: u32,
) -> Result<(), AnalyzeActionError> {
    let path = analyze_node_path(snapshot, node_id)?;
    let runner = ProcessRunner::default();
    reveal_with(|request| runner.run(request), &path)
}

/// Builds the Recycle Bin plan for the selected nodes and returns its digest.
pub fn preview_analyze_trash(
    operation_id: &str,
    snapshot: &AnalyzeSnapshotV1,
    node_ids: &[u32],
) -> Result<AnalyzeTrashPreviewV1, AnalyzeActionError> {
    preview_with(
        &TrashEnvironment::production()?,
        operation_id,
        snapshot,
        node_ids,
    )
}

/// Rebuilds the plan, then moves the accepted nodes to the Recycle Bin through
/// the Clean executor when the digest still matches.
pub fn execute_analyze_trash(
    request: AnalyzeTrashExecutionRequest<'_>,
) -> Result<AnalyzeTrashReportV1, AnalyzeActionError> {
    if !request.confirmed {
        return Err(AnalyzeActionError::ConfirmationRequired);
    }
    execute_with(
        &TrashEnvironment::production()?,
        &Executor::default(),
        None,
        request,
    )
}

fn reveal_request<'a>(path: &Path, cancel: &'a dyn CancelObserver) -> ProcessRequest<'a> {
    ProcessRequest {
        program: OsString::from(EXPLORER_PROGRAM),
        args: vec![
            OsString::from(EXPLORER_SELECT_ARG),
            path.as_os_str().to_os_string(),
        ],
        cwd: CwdPolicy::Neutral,
        timeout: Some(REVEAL_TIMEOUT),
        job_deadline: None,
        cancel,
    }
}

/// Explorer exits with code 1 after a successful `/select,`. Only a launch
/// failure is an error.
fn reveal_with(
    run: impl FnOnce(&ProcessRequest<'_>) -> ProcessResult,
    path: &Path,
) -> Result<(), AnalyzeActionError> {
    let noop = NoopCancelObserver;
    match run(&reveal_request(path, &noop)).status {
        ProcessStatus::NotFound | ProcessStatus::InvalidOutput => {
            Err(AnalyzeActionError::RevealUnavailable)
        }
        ProcessStatus::Success
        | ProcessStatus::Exit { .. }
        | ProcessStatus::Timeout
        | ProcessStatus::Canceled => Ok(()),
    }
}

/// Snapshot lookup with the rebuilt root path.
struct NodeIndex<'a> {
    base: PathBuf,
    nodes: HashMap<u32, &'a AnalyzeNodeV1>,
}

impl<'a> NodeIndex<'a> {
    fn new(snapshot: &'a AnalyzeSnapshotV1) -> Result<Self, AnalyzeActionError> {
        Ok(Self {
            base: snapshot_base(snapshot)?,
            nodes: snapshot.nodes.iter().map(|node| (node.id, node)).collect(),
        })
    }

    fn node(&self, node_id: u32) -> Option<&'a AnalyzeNodeV1> {
        self.nodes.get(&node_id).copied()
    }

    fn path(&self, node_id: u32) -> Result<PathBuf, AnalyzeActionError> {
        let mut names = Vec::new();
        let mut seen = HashSet::new();
        let mut cursor = node_id;
        loop {
            if !seen.insert(cursor) {
                return Err(AnalyzeActionError::InvalidSnapshot(node_id));
            }
            let node = self
                .node(cursor)
                .ok_or(AnalyzeActionError::UnknownNode(cursor))?;
            match node.parent_id {
                Some(parent) => {
                    if !plain_name(&node.name) {
                        return Err(AnalyzeActionError::InvalidSnapshot(node_id));
                    }
                    names.push(node.name.as_str());
                    cursor = parent;
                }
                None if node.id == 0 => break,
                None => return Err(AnalyzeActionError::InvalidSnapshot(node_id)),
            }
        }
        let mut path = self.base.clone();
        for name in names.iter().rev() {
            path.push(name);
        }
        Ok(path)
    }

    /// True when a strict ancestor of `node_id` is in `ids`.
    fn has_ancestor_in(&self, node_id: u32, ids: &HashSet<u32>) -> bool {
        let mut seen = HashSet::new();
        let mut cursor = self.node(node_id).and_then(|node| node.parent_id);
        while let Some(id) = cursor {
            if !seen.insert(id) {
                return false;
            }
            if ids.contains(&id) {
                return true;
            }
            cursor = self.node(id).and_then(|node| node.parent_id);
        }
        false
    }
}

/// Uses the caller input when it names the same absolute root, so the
/// original letter case reaches Explorer and the Recycle Bin.
fn snapshot_base(snapshot: &AnalyzeSnapshotV1) -> Result<PathBuf, AnalyzeActionError> {
    let normalized = snapshot.root.normalized.as_str();
    let input = Path::new(&snapshot.root.input);
    if input.is_absolute() && normalize_absolute_path(input).ok().as_deref() == Some(normalized) {
        return Ok(input.to_path_buf());
    }
    let mut base = normalized.replace('/', MAIN_SEPARATOR_STR);
    if base.ends_with(':') {
        base.push_str(MAIN_SEPARATOR_STR);
    }
    let base = PathBuf::from(base);
    if base.is_absolute() {
        Ok(base)
    } else {
        Err(AnalyzeActionError::InvalidSnapshot(0))
    }
}

/// Accepts one path component: no separators and no dot segments.
fn plain_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\'])
}

/// Exclusion inputs for one run.
#[derive(Debug, Clone, Default)]
struct TrashEnvironment {
    system_locations: Vec<PathBuf>,
    profile: Option<PathBuf>,
    protections: Vec<PathBuf>,
}

impl TrashEnvironment {
    fn production() -> Result<Self, AnalyzeActionError> {
        let mut protections = UserProtectionList::load()
            .map_err(|_| AnalyzeActionError::ProtectionUnavailable)?
            .list()
            .to_vec();
        let profile = resolve_home_dir();
        if let Some(home) = &profile {
            protections.extend(
                PROFILE_PROTECTED_SUBTREES
                    .iter()
                    .map(|relative| home.join(relative)),
            );
        }
        let system_locations = SYSTEM_LOCATION_VARS
            .iter()
            .filter_map(|key| env::var_os(key).filter(|value| !value.is_empty()))
            .map(PathBuf::from)
            .collect();
        Ok(Self {
            system_locations,
            profile,
            protections,
        })
    }

    /// Returns why `path` stays view only before any live check.
    fn refusal(&self, path: &Path, node: &AnalyzeNodeV1) -> Option<AnalyzeTrashRefusalCode> {
        let overlaps = |other: &PathBuf| path_is_within(other, path) || path_is_within(path, other);
        if path.parent().is_none() {
            return Some(AnalyzeTrashRefusalCode::VolumeRoot);
        }
        if node.parent_id.is_none() {
            return Some(AnalyzeTrashRefusalCode::AnalysisRoot);
        }
        if let Some(profile) = &self.profile
            && (path_is_within(path, profile)
                || PROFILE_EXACT_CHILDREN
                    .iter()
                    .any(|child| paths_equal(path, &profile.join(child))))
        {
            return Some(AnalyzeTrashRefusalCode::ProfileRoot);
        }
        // Other accounts' profiles (and Public/Default) under the same Users folder.
        if let Some(users) = self.profile.as_deref().and_then(Path::parent)
            && path_is_within(users, path)
            && !self
                .profile
                .as_deref()
                .is_some_and(|profile| path_is_within(profile, path))
        {
            return Some(AnalyzeTrashRefusalCode::ProfileRoot);
        }
        if self.system_locations.iter().any(overlaps) || under_volume_system_folder(path) {
            return Some(AnalyzeTrashRefusalCode::SystemLocation);
        }
        let contains_exe = match env::current_exe() {
            Ok(exe) => path_contains_path(path, &exe),
            Err(_) => true,
        };
        if contains_exe || self.protections.iter().any(overlaps) {
            return Some(AnalyzeTrashRefusalCode::Protected);
        }
        if node.kind == AnalyzeNodeKind::Reparse {
            return Some(AnalyzeTrashRefusalCode::ReparsePoint);
        }
        None
    }
}

/// Volume-root folders that Windows owns on every drive.
const VOLUME_SYSTEM_FOLDERS: [&str; 2] = ["$Recycle.Bin", "System Volume Information"];

/// True when the first folder below the volume root is a Windows system folder.
fn under_volume_system_folder(path: &Path) -> bool {
    path.components()
        .find_map(|component| match component {
            std::path::Component::Normal(name) => Some(name),
            _ => None,
        })
        .is_some_and(|first| {
            VOLUME_SYSTEM_FOLDERS
                .iter()
                .any(|folder| first.to_string_lossy().eq_ignore_ascii_case(folder))
        })
}

/// Compares the live entry with the snapshot node.
fn live_refusal(path: &Path, node: &AnalyzeNodeV1) -> Option<AnalyzeTrashRefusalCode> {
    let entry = match inspect_entry_no_follow(path, &SystemPathReparseProbe) {
        Ok(entry) => entry,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(AnalyzeTrashRefusalCode::NotFound);
        }
        Err(_) => return Some(AnalyzeTrashRefusalCode::ChangedSinceSnapshot),
    };
    if entry.safety != PathSafety::Safe {
        return Some(AnalyzeTrashRefusalCode::ReparsePoint);
    }
    let same_kind = match node.kind {
        AnalyzeNodeKind::File => entry.is_file,
        AnalyzeNodeKind::Directory => entry.is_dir,
        AnalyzeNodeKind::Reparse => false,
    };
    let live_mtime_ms = entry
        .modified
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);
    if !same_kind || (node.mtime_ms.is_some() && live_mtime_ms != node.mtime_ms) {
        return Some(AnalyzeTrashRefusalCode::ChangedSinceSnapshot);
    }
    None
}

/// One accepted node with its rebuilt path.
struct Accepted<'a> {
    node: &'a AnalyzeNodeV1,
    path: PathBuf,
}

fn select<'a>(
    environment: &TrashEnvironment,
    snapshot: &'a AnalyzeSnapshotV1,
    node_ids: &[u32],
) -> Result<(Vec<Accepted<'a>>, Vec<AnalyzeTrashRefusalV1>), AnalyzeActionError> {
    if node_ids.is_empty() {
        return Err(AnalyzeActionError::EmptySelection);
    }
    let index = NodeIndex::new(snapshot)?;
    let mut seen = HashSet::new();
    let mut accepted = Vec::new();
    let mut refused = Vec::new();
    for &node_id in node_ids {
        if !seen.insert(node_id) {
            continue;
        }
        let refuse = |reason_code| AnalyzeTrashRefusalV1 {
            node_id,
            reason_code,
        };
        let (node, path) = match (index.node(node_id), index.path(node_id)) {
            (Some(node), Ok(path)) => (node, path),
            (None, _) | (_, Err(AnalyzeActionError::UnknownNode(_))) => {
                refused.push(refuse(AnalyzeTrashRefusalCode::NotFound));
                continue;
            }
            (_, Err(error)) => return Err(error),
        };
        match environment
            .refusal(&path, node)
            .or_else(|| live_refusal(&path, node))
        {
            Some(code) => refused.push(refuse(code)),
            None => accepted.push(Accepted { node, path }),
        }
    }
    let accepted_ids = accepted
        .iter()
        .map(|item| item.node.id)
        .collect::<HashSet<_>>();
    accepted.retain(|item| !index.has_ancestor_in(item.node.id, &accepted_ids));
    Ok((accepted, refused))
}

fn target_id(operation_id: &str, node_id: u32) -> TargetId {
    TargetId::new(format!("{ANALYZE_TRASH_RULE_ID}:{operation_id}:{node_id}"))
}

/// Builds the trusted in-memory plan. `kind` is a required Clean field; the
/// value never leaves core and carries no meaning for `analyze.trash`.
fn build_plan(
    operation_id: &str,
    accepted: &[Accepted<'_>],
) -> Result<(ValidatedPlan, Vec<TargetId>), AnalyzeActionError> {
    let targets = accepted
        .iter()
        .map(|item| CleanTarget {
            id: target_id(operation_id, item.node.id),
            scope: Scope::Global,
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::ToolCache,
            path: Some(item.path.clone()),
            estimated_bytes: item.node.bytes,
            size_complete: item.node.evidence == AnalyzeEvidence::Complete,
            sizing_warnings: Vec::new(),
            last_modified: item
                .node
                .mtime_ms
                .map(|ms| UNIX_EPOCH + Duration::from_millis(ms)),
            risk: RiskLevel::High,
            reversible: true,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: ANALYZE_TRASH_RULE_ID.to_string(),
            }],
            action: CleanAction::MoveToTrash {
                path: item.path.clone(),
            },
        })
        .collect::<Vec<_>>();
    let selected = targets.iter().map(|target| target.id.clone()).collect();
    let plan =
        validated_trash_plan(targets, ANALYZE_TRASH_RULE_ID).map_err(AnalyzeActionError::Plan)?;
    Ok((plan, selected))
}

fn preview_with(
    environment: &TrashEnvironment,
    operation_id: &str,
    snapshot: &AnalyzeSnapshotV1,
    node_ids: &[u32],
) -> Result<AnalyzeTrashPreviewV1, AnalyzeActionError> {
    let (accepted, refused) = select(environment, snapshot, node_ids)?;
    let (plan, selected) = build_plan(operation_id, &accepted)?;
    let digest = confirmation_digest(&plan, &selected).map_err(AnalyzeActionError::Plan)?;
    Ok(AnalyzeTrashPreviewV1 {
        version: ANALYZE_TRASH_VERSION,
        operation_id: operation_id.to_string(),
        items: accepted
            .iter()
            .map(|item| AnalyzeTrashItemV1 {
                node_id: item.node.id,
                target_id: target_id(operation_id, item.node.id),
                path: item.path.clone(),
                kind: item.node.kind,
                bytes: item.node.bytes,
                evidence: item.node.evidence,
            })
            .collect(),
        refused,
        digest,
    })
}

fn execute_with<C, T>(
    environment: &TrashEnvironment,
    executor: &Executor<C, T>,
    audit_log: Option<PathBuf>,
    request: AnalyzeTrashExecutionRequest<'_>,
) -> Result<AnalyzeTrashReportV1, AnalyzeActionError>
where
    C: CommandRunner,
    T: TrashRunner,
{
    if !request.confirmed {
        return Err(AnalyzeActionError::ConfirmationRequired);
    }
    let (accepted, _) = select(environment, request.snapshot, request.node_ids)?;
    if accepted.is_empty() {
        return Err(AnalyzeActionError::NothingToMove);
    }
    let (plan, selected) = build_plan(request.operation_id, &accepted)?;
    let report = executor
        .run_plan(
            &plan,
            ExecutionRequest {
                execute: true,
                audit_log,
                selected,
                expected_digest: Some(ConfirmationDigest::new(request.expected_digest)),
                cancel: request.cancel,
            },
        )
        .map_err(AnalyzeActionError::Execution)?;
    let succeeded = report
        .outcomes
        .iter()
        .filter(|outcome| outcome.status == OutcomeStatus::Succeeded)
        .map(|outcome| outcome.target_id.clone())
        .collect::<HashSet<_>>();
    Ok(AnalyzeTrashReportV1 {
        version: ANALYZE_TRASH_VERSION,
        operation_id: request.operation_id.to_string(),
        moved_node_ids: accepted
            .iter()
            .map(|item| item.node.id)
            .filter(|node_id| succeeded.contains(&target_id(request.operation_id, *node_id)))
            .collect(),
        report,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::Mutex,
        time::{Duration, SystemTime},
    };

    use tempfile::TempDir;

    use super::*;
    use crate::{
        analysis::analyze_path,
        execution::{ExecutionError, ProcessCommandRunner},
        model::{CleanupIntent, UntrustedPlan, UntrustedTarget},
        plan::validate_plan,
        process::ProcessOutput,
    };

    struct Fixture {
        _temp: TempDir,
        root: PathBuf,
        environment: TrashEnvironment,
    }

    /// root/{keep.bin, docs/note.txt, windows/sys.dll, users/me/Desktop,
    /// protected/secret.txt}
    fn fixture() -> Fixture {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap().join("root");
        for dir in ["docs", "windows", "users/me/Desktop", "protected"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        for (file, bytes) in [
            ("keep.bin", 4096_usize),
            ("docs/note.txt", 10),
            ("windows/sys.dll", 20),
            ("protected/secret.txt", 30),
        ] {
            fs::write(root.join(file), vec![7_u8; bytes]).unwrap();
        }
        Fixture {
            environment: TrashEnvironment {
                system_locations: vec![root.join("windows")],
                profile: Some(root.join("users").join("me")),
                protections: vec![root.join("protected")],
            },
            root,
            _temp: temp,
        }
    }

    fn snapshot(root: &Path) -> AnalyzeSnapshotV1 {
        analyze_path(root, None, None).unwrap().snapshot().clone()
    }

    fn id_of(snapshot: &AnalyzeSnapshotV1, relative: &str) -> u32 {
        let wanted = snapshot_base(snapshot).unwrap().join(relative);
        snapshot
            .nodes
            .iter()
            .map(|node| node.id)
            .find(|id| {
                analyze_node_path(snapshot, *id).is_ok_and(|path| paths_equal(&path, &wanted))
            })
            .unwrap_or_else(|| panic!("no node for {relative}"))
    }

    fn refusals(preview: &AnalyzeTrashPreviewV1) -> Vec<(u32, AnalyzeTrashRefusalCode)> {
        preview
            .refused
            .iter()
            .map(|refusal| (refusal.node_id, refusal.reason_code))
            .collect()
    }

    #[derive(Default, Clone)]
    struct RecordingTrash {
        moved: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl TrashRunner for RecordingTrash {
        fn move_to_trash(&self, path: &Path) -> anyhow::Result<()> {
            self.moved.lock().unwrap().push(path.to_path_buf());
            fs::remove_file(path).or_else(|_| fs::remove_dir_all(path))?;
            Ok(())
        }
    }

    #[test]
    fn node_path_rebuilds_from_names_and_rejects_unknown_ids_and_cycles() {
        let fixture = fixture();
        let snapshot = snapshot(&fixture.root);
        let note = id_of(&snapshot, "docs/note.txt");
        assert!(paths_equal(
            &analyze_node_path(&snapshot, note).unwrap(),
            &fixture.root.join("docs").join("note.txt")
        ));
        assert!(paths_equal(
            &analyze_node_path(&snapshot, 0).unwrap(),
            &fixture.root
        ));
        assert!(matches!(
            analyze_node_path(&snapshot, 999_999),
            Err(AnalyzeActionError::UnknownNode(999_999))
        ));

        let mut cyclic = snapshot.clone();
        let docs = id_of(&snapshot, "docs");
        for node in &mut cyclic.nodes {
            if node.id == docs {
                node.parent_id = Some(note);
            }
        }
        assert!(matches!(
            analyze_node_path(&cyclic, note),
            Err(AnalyzeActionError::InvalidSnapshot(_))
        ));

        let mut escaping = snapshot.clone();
        for node in &mut escaping.nodes {
            if node.id == docs {
                node.name = "..".to_string();
            }
        }
        assert!(matches!(
            analyze_node_path(&escaping, note),
            Err(AnalyzeActionError::InvalidSnapshot(_))
        ));
    }

    #[test]
    fn each_refusal_class_is_reported() {
        let fixture = fixture();
        let mut snapshot = snapshot(&fixture.root);
        let root = 0;
        let windows = id_of(&snapshot, "windows/sys.dll");
        let profile = id_of(&snapshot, "users/me");
        let desktop = id_of(&snapshot, "users/me/Desktop");
        let protected = id_of(&snapshot, "protected/secret.txt");
        let changed = id_of(&snapshot, "keep.bin");
        let missing = id_of(&snapshot, "docs/note.txt");
        let reparse = id_of(&snapshot, "docs");
        for node in &mut snapshot.nodes {
            if node.id == reparse {
                node.kind = AnalyzeNodeKind::Reparse;
            }
        }
        let file = fs::File::options()
            .write(true)
            .open(fixture.root.join("keep.bin"))
            .unwrap();
        file.set_modified(SystemTime::now() + Duration::from_secs(3600))
            .unwrap();
        drop(file);
        fs::remove_file(fixture.root.join("docs/note.txt")).unwrap();

        let preview = preview_with(
            &fixture.environment,
            "op",
            &snapshot,
            &[
                root, windows, profile, desktop, protected, changed, missing, reparse, 999_999,
            ],
        )
        .unwrap();
        assert!(preview.items.is_empty());
        assert_eq!(
            refusals(&preview),
            vec![
                (root, AnalyzeTrashRefusalCode::AnalysisRoot),
                (windows, AnalyzeTrashRefusalCode::SystemLocation),
                (profile, AnalyzeTrashRefusalCode::ProfileRoot),
                (desktop, AnalyzeTrashRefusalCode::ProfileRoot),
                (protected, AnalyzeTrashRefusalCode::Protected),
                (changed, AnalyzeTrashRefusalCode::ChangedSinceSnapshot),
                (missing, AnalyzeTrashRefusalCode::NotFound),
                (reparse, AnalyzeTrashRefusalCode::ReparsePoint),
                (999_999, AnalyzeTrashRefusalCode::NotFound),
            ]
        );
    }

    #[test]
    fn other_profiles_and_volume_system_folders_are_refused() {
        let files = fixture();
        let users = files.root.join("users");
        let node = AnalyzeNodeV1 {
            id: 1,
            parent_id: Some(0),
            kind: AnalyzeNodeKind::Directory,
            name: String::new(),
            bytes: 0,
            immediate_count: 0,
            recursive_count: 0,
            evidence: AnalyzeEvidence::Complete,
            warnings: Vec::new(),
            mtime_ms: None,
        };
        let refusal = |path: &Path| files.environment.refusal(path, &node);
        assert_eq!(
            refusal(&users.join("other").join("AppData")),
            Some(AnalyzeTrashRefusalCode::ProfileRoot)
        );
        let volume = files.root.ancestors().last().unwrap().to_path_buf();
        for folder in ["$Recycle.Bin", "system volume information"] {
            assert_eq!(
                refusal(&volume.join(folder).join("S-1-5-21")),
                Some(AnalyzeTrashRefusalCode::SystemLocation)
            );
        }
        assert_eq!(
            refusal(&files.root.join("users").join("me").join("cache")),
            None
        );
    }

    #[test]
    fn volume_root_and_containing_folders_are_refused() {
        let files = fixture();
        let mut rooted = snapshot(&files.root);
        let volume = files.root.ancestors().last().unwrap().to_path_buf();
        rooted.root.input = volume.to_string_lossy().into_owned();
        rooted.root.normalized = normalize_absolute_path(&volume).unwrap();
        let preview = preview_with(&files.environment, "op", &rooted, &[0]).unwrap();
        assert_eq!(
            refusals(&preview),
            vec![(0, AnalyzeTrashRefusalCode::VolumeRoot)]
        );

        let observed = snapshot(&files.root);
        let environment = TrashEnvironment {
            profile: Some(files.root.join("docs").join("user")),
            ..files.environment.clone()
        };
        let docs = id_of(&observed, "docs");
        let preview = preview_with(&environment, "op", &observed, &[docs]).unwrap();
        assert_eq!(
            refusals(&preview),
            vec![(docs, AnalyzeTrashRefusalCode::ProfileRoot)]
        );
    }

    #[test]
    fn accepted_node_yields_a_move_to_trash_plan_with_a_digest() {
        let fixture = fixture();
        let snapshot = snapshot(&fixture.root);
        let keep = id_of(&snapshot, "keep.bin");
        let preview = preview_with(&fixture.environment, "op", &snapshot, &[keep]).unwrap();
        assert!(preview.refused.is_empty());
        assert_eq!(preview.items.len(), 1);
        assert_eq!(preview.items[0].node_id, keep);
        assert_eq!(preview.items[0].bytes, 4096);
        assert_eq!(
            preview.items[0].target_id.as_str(),
            format!("analyze.trash:op:{keep}")
        );
        assert_eq!(preview.digest.as_str().len(), 64);
        assert!(
            preview
                .digest
                .as_str()
                .chars()
                .all(|value| value.is_ascii_hexdigit())
        );

        let (accepted, _) = select(&fixture.environment, &snapshot, &[keep]).unwrap();
        let (plan, selected) = build_plan("op", &accepted).unwrap();
        assert_eq!(selected, vec![target_id("op", keep)]);
        let target = plan.targets()[0].target();
        assert!(matches!(
            &target.action,
            CleanAction::MoveToTrash { path } if paths_equal(path, &fixture.root.join("keep.bin"))
        ));
        assert_eq!(plan.targets()[0].rule_id(), ANALYZE_TRASH_RULE_ID);

        // A second preview of the same live state has the same digest.
        let again = preview_with(&fixture.environment, "op", &snapshot, &[keep]).unwrap();
        assert_eq!(again.digest, preview.digest);
    }

    #[test]
    fn plan_files_cannot_carry_the_analyze_trash_rule() {
        let fixture = fixture();
        let path = fixture.root.join("keep.bin");
        let plan = UntrustedPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![UntrustedTarget {
                id: TargetId::new("analyze.trash:op:1"),
                rule_id: ANALYZE_TRASH_RULE_ID.to_string(),
                scope: Scope::Global,
                ecosystem: Ecosystem::Generic,
                kind: TargetKind::ToolCache,
                path: Some(path),
                estimated_bytes: 1,
                size_complete: true,
                sizing_warnings: Vec::new(),
                last_modified: None,
                risk: RiskLevel::High,
                reversible: true,
                selected_by_default: true,
                evidence: Vec::new(),
                intent: CleanupIntent::TrashProjectArtifact {
                    rule_id: ANALYZE_TRASH_RULE_ID.to_string(),
                },
            }],
        };
        assert!(validate_plan(&plan).is_err());
    }

    #[test]
    fn nested_selection_keeps_only_the_ancestor() {
        let fixture = fixture();
        let snapshot = snapshot(&fixture.root);
        let docs = id_of(&snapshot, "docs");
        let note = id_of(&snapshot, "docs/note.txt");
        let preview =
            preview_with(&fixture.environment, "op", &snapshot, &[note, docs, note]).unwrap();
        assert_eq!(
            preview
                .items
                .iter()
                .map(|item| item.node_id)
                .collect::<Vec<_>>(),
            vec![docs]
        );
        assert!(preview.refused.is_empty());
        assert!(matches!(
            preview_with(&fixture.environment, "op", &snapshot, &[]),
            Err(AnalyzeActionError::EmptySelection)
        ));
    }

    #[test]
    fn stale_digest_or_operation_mismatch_is_refused_before_any_move() {
        let fixture = fixture();
        let snapshot = snapshot(&fixture.root);
        let keep = id_of(&snapshot, "keep.bin");
        let nodes = [keep];
        let preview = preview_with(&fixture.environment, "op-a", &snapshot, &nodes).unwrap();
        let audit = fixture
            .root
            .parent()
            .unwrap()
            .join("audit")
            .join("clean.jsonl");
        let trash = RecordingTrash::default();
        let executor = Executor::new(ProcessCommandRunner::new(), trash.clone());
        let request = |operation_id, expected_digest, confirmed| AnalyzeTrashExecutionRequest {
            operation_id,
            snapshot: &snapshot,
            node_ids: &nodes,
            expected_digest,
            confirmed,
            cancel: None,
        };
        let digest = preview.digest.as_str();
        for stale in [request("op-b", digest, true), request("op-a", "0", true)] {
            let error = execute_with(&fixture.environment, &executor, Some(audit.clone()), stale)
                .unwrap_err();
            let AnalyzeActionError::Execution(error) = error else {
                panic!("expected a stale confirmation, got {error}");
            };
            assert!(matches!(
                error.downcast_ref::<ExecutionError>(),
                Some(ExecutionError::StaleConfirmation { .. })
            ));
        }
        assert!(matches!(
            execute_with(
                &fixture.environment,
                &executor,
                Some(audit.clone()),
                request("op-a", digest, false)
            ),
            Err(AnalyzeActionError::ConfirmationRequired)
        ));
        assert!(trash.moved.lock().unwrap().is_empty());
        assert!(!audit.exists());
        assert!(fixture.root.join("keep.bin").exists());
    }

    #[test]
    fn execution_moves_through_the_clean_executor_and_writes_clean_audit() {
        let fixture = fixture();
        let snapshot = snapshot(&fixture.root);
        let keep = id_of(&snapshot, "keep.bin");
        let windows = id_of(&snapshot, "windows/sys.dll");
        let nodes = [keep, windows];
        let preview = preview_with(&fixture.environment, "op", &snapshot, &nodes).unwrap();
        let audit = fixture
            .root
            .parent()
            .unwrap()
            .join("audit")
            .join("clean.jsonl");
        let trash = RecordingTrash::default();
        let executor = Executor::new(ProcessCommandRunner::new(), trash.clone());
        let report = execute_with(
            &fixture.environment,
            &executor,
            Some(audit.clone()),
            AnalyzeTrashExecutionRequest {
                operation_id: "op",
                snapshot: &snapshot,
                node_ids: &nodes,
                expected_digest: preview.digest.as_str(),
                confirmed: true,
                cancel: None,
            },
        )
        .unwrap();
        assert_eq!(report.moved_node_ids, vec![keep]);
        assert_eq!(report.report.succeeded, 1);
        assert!(!report.report.dry_run);
        let moved = trash.moved.lock().unwrap().clone();
        assert_eq!(moved.len(), 1);
        assert!(paths_equal(&moved[0], &fixture.root.join("keep.bin")));
        assert!(fixture.root.join("windows/sys.dll").exists());

        let journal = fs::read_to_string(&audit).unwrap();
        let lines = journal.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(
            lines
                .iter()
                .all(|line| line.contains("\"domain\":\"clean\"")
                    && line.contains("\"record_kind\":\"execution_transition\"")
                    && line.contains("\"action_kind\":\"move_to_trash\""))
        );
        assert!(lines[0].contains("\"outcome_code\":\"started\""));
        assert!(lines[1].contains("\"outcome_code\":\"succeeded\""));
        assert!(lines[1].contains("\"estimated_bytes\":4096"));
        assert!(!journal.contains("keep.bin"));
        assert!(!journal.contains("delete"));
    }

    #[test]
    fn reveal_uses_explorer_with_separate_argv_and_neutral_cwd() {
        let path = PathBuf::from(if cfg!(windows) {
            r"C:\Users\someone\a folder\file.txt"
        } else {
            "/home/someone/a folder/file.txt"
        });
        let mut captured = None;
        reveal_with(
            |request| {
                captured = Some((
                    request.program.clone(),
                    request.args.clone(),
                    request.cwd.clone(),
                    request.timeout,
                ));
                ProcessResult {
                    status: ProcessStatus::Exit { code: Some(1) },
                    output: ProcessOutput::default(),
                }
            },
            &path,
        )
        .expect("exit code 1 is a successful reveal");
        let (program, args, cwd, timeout) = captured.expect("reveal went through ProcessRequest");
        assert_eq!(program, OsString::from("explorer.exe"));
        assert_eq!(
            args,
            vec![OsString::from("/select,"), path.clone().into_os_string()]
        );
        assert_eq!(cwd, CwdPolicy::Neutral);
        assert_eq!(timeout, Some(REVEAL_TIMEOUT));
    }

    #[test]
    fn reveal_treats_only_a_launch_failure_as_an_error() {
        let result = |status| ProcessResult {
            status,
            output: ProcessOutput::default(),
        };
        for status in [
            ProcessStatus::Success,
            ProcessStatus::Exit { code: Some(1) },
            ProcessStatus::Timeout,
        ] {
            assert!(reveal_with(|_| result(status.clone()), Path::new("C:/x")).is_ok());
        }
        for status in [ProcessStatus::NotFound, ProcessStatus::InvalidOutput] {
            assert!(matches!(
                reveal_with(|_| result(status.clone()), Path::new("C:/x")),
                Err(AnalyzeActionError::RevealUnavailable)
            ));
        }
    }
}
