//! Read-only Analyze roots, bounded child lists, and storage classes.
//!
//! These projections do not create a cleanup plan. A class is not disposable.
//! File-lock evidence is a report; this module never shuts a process down.

use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{
    ANALYZE_SNAPSHOT_VERSION, AnalyzeActionError, AnalyzeEvidence, AnalyzeNodeKind, AnalyzeNodeV1,
    AnalyzeSnapshotV1, AnalyzeWarningClass, analyze_node_path,
    walker::{self, FsFail},
};
use crate::filesystem::{PathSafety, SystemPathReparseProbe, inspect_entry_no_follow};

/// Wire version for roots, list, and explain documents.
pub const ANALYZE_OBSERVE_VERSION: u32 = 1;
/// Default page size for one node's children.
pub const ANALYZE_LIST_DEFAULT_LIMIT: u32 = 100;
/// Upper page size for one node's children. No flag raises this cap.
pub const ANALYZE_LIST_MAX_LIMIT: u32 = 1_000;
/// Restart Manager owner rows returned for one path.
const LOCK_OWNER_CAP: u32 = 32;

/// One closed storage class. Size is not a variant and cannot select one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeClass {
    /// `hiberfil.sys`.
    Hibernation,
    /// `pagefile.sys` or `swapfile.sys`.
    PageFile,
    /// WSL `ext4.vhdx` outside a Docker path.
    WslDisk,
    /// Docker Desktop virtual disk.
    DockerDisk,
    /// Model file or known model-store path.
    AiModel,
    /// Windows image backup, File History, or a backup catalog.
    Backup,
    /// Steam, Epic, or Xbox game layout.
    Game,
    /// Cloud Files placeholder, from a cloud reparse tag or recall attribute.
    CloudPlaceholder,
    /// No exact class evidence.
    Unknown,
}

/// Why [`AnalyzeClass`] was selected. There is no size evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeClassEvidence {
    NameHiberfilSys,
    NamePagefileSys,
    NameSwapfileSys,
    PathWslExt4Vhdx,
    PathDockerVhdx,
    ExtensionGguf,
    ExtensionSafetensors,
    PathOllamaModels,
    PathHuggingfaceHub,
    PathWindowsImageBackup,
    PathFileHistory,
    ExtensionWbcat,
    PathSteamappsCommon,
    NameAppManifestAcf,
    PathEpicGames,
    PathXboxGames,
    CloudReparse,
    CloudRecallAttribute,
    None,
}

/// Per-root failure. A missing root stays this root's error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeRootErrorCode {
    /// The path is not present.
    Missing,
    /// The path could not be opened for its volume identity.
    Inaccessible,
}

/// One volume root the snapshot source could see, or the error for that root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeRootRecordV1 {
    pub input: String,
    pub normalized: Option<String>,
    pub volume: Option<String>,
    pub alias_of: Option<String>,
    pub error: Option<AnalyzeRootErrorCode>,
}

/// Volume roots. This document is not a scan and not a cleanup plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeRootsV1 {
    pub version: u32,
    pub roots: Vec<AnalyzeRootRecordV1>,
}

/// One represented child. Bytes are the snapshot lower bound, not a class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeListChildV1 {
    pub id: u32,
    pub name: String,
    pub kind: AnalyzeNodeKind,
    pub bytes: u64,
    pub evidence: AnalyzeEvidence,
    pub immediate_count: u32,
    pub warnings: Vec<AnalyzeWarningClass>,
}

/// One bounded page of children for a snapshot node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeListV1 {
    pub version: u32,
    pub parent_id: u32,
    pub total_children: u32,
    pub offset: u32,
    pub limit: u32,
    pub children: Vec<AnalyzeListChildV1>,
}

/// One process that has the explained path open. The pid is not a kill request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeLockOwnerV1 {
    pub pid: u32,
    pub name: String,
}

/// Lock-owner report. `terminates_process` is always false.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeLockReportV1 {
    pub supported: bool,
    pub terminates_process: bool,
    pub truncated: bool,
    pub owners: Vec<AnalyzeLockOwnerV1>,
}

/// Explanation of one snapshot node. It carries no cleanup plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeExplainV1 {
    pub version: u32,
    pub node_id: u32,
    pub class: AnalyzeClass,
    pub evidence: AnalyzeClassEvidence,
    /// Always false. A class does not make the node disposable.
    pub disposable: bool,
    pub lock: AnalyzeLockReportV1,
}

/// Failure before a read-only projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyzeObserveError {
    UnknownNode(u32),
    InvalidSnapshot,
    RootsUnavailable,
}

impl std::fmt::Display for AnalyzeObserveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(formatter, "unknown analysis node {id}"),
            Self::InvalidSnapshot => formatter.write_str("analysis snapshot is invalid"),
            Self::RootsUnavailable => formatter.write_str("analysis volume roots are unavailable"),
        }
    }
}

impl std::error::Error for AnalyzeObserveError {}

impl AnalyzeObserveError {
    /// Stable machine code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownNode(_) => "unknown_node",
            Self::InvalidSnapshot => "invalid_snapshot",
            Self::RootsUnavailable => "roots_unavailable",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloudEvidence {
    None,
    Reparse,
    Recall,
}

trait RootSource {
    fn list_mounts(&self) -> Result<Vec<PathBuf>, ()>;
    fn volume_id(&self, path: &Path) -> Result<String, FsFail>;
}

struct NativeRootSource;

impl RootSource for NativeRootSource {
    fn list_mounts(&self) -> Result<Vec<PathBuf>, ()> {
        logical_mounts()
    }

    fn volume_id(&self, path: &Path) -> Result<String, FsFail> {
        walker::native_volume_id(path)
    }
}

/// Lists volume roots.
///
/// An empty `requested` list asks the snapshot source for mounts it can see.
/// A non-empty list probes only those paths. A missing path is an error for
/// that path. The user profile is not a fallback.
pub fn list_analyze_roots(requested: &[PathBuf]) -> Result<AnalyzeRootsV1, AnalyzeObserveError> {
    list_analyze_roots_with(&NativeRootSource, requested)
}

fn list_analyze_roots_with(
    source: &dyn RootSource,
    requested: &[PathBuf],
) -> Result<AnalyzeRootsV1, AnalyzeObserveError> {
    let mounts = if requested.is_empty() {
        source
            .list_mounts()
            .map_err(|()| AnalyzeObserveError::RootsUnavailable)?
    } else {
        requested.to_vec()
    };
    let mut roots = Vec::with_capacity(mounts.len());
    for mount in mounts {
        roots.push(probe_root(source, &mount));
    }
    qualify_aliases(&mut roots);
    Ok(AnalyzeRootsV1 {
        version: ANALYZE_OBSERVE_VERSION,
        roots,
    })
}

fn probe_root(source: &dyn RootSource, input: &Path) -> AnalyzeRootRecordV1 {
    let input_text = input.to_string_lossy().into_owned();
    let Some(absolute) = absolute_path(input) else {
        return AnalyzeRootRecordV1 {
            input: input_text,
            normalized: None,
            volume: None,
            alias_of: None,
            error: Some(AnalyzeRootErrorCode::Inaccessible),
        };
    };
    if drive_has_no_root(&absolute) {
        return AnalyzeRootRecordV1 {
            input: input_text,
            normalized: None,
            volume: None,
            alias_of: None,
            error: Some(AnalyzeRootErrorCode::Missing),
        };
    }
    let normalized = crate::filesystem::normalize_absolute_path(&absolute).ok();
    match source.volume_id(&absolute) {
        Ok(volume) => AnalyzeRootRecordV1 {
            input: input_text,
            normalized,
            volume: Some(volume),
            alias_of: None,
            error: None,
        },
        Err(FsFail::NotFound) => AnalyzeRootRecordV1 {
            input: input_text,
            normalized: None,
            volume: None,
            alias_of: None,
            error: Some(AnalyzeRootErrorCode::Missing),
        },
        Err(FsFail::AccessDenied | FsFail::Io) => AnalyzeRootRecordV1 {
            input: input_text,
            normalized: None,
            volume: None,
            alias_of: None,
            error: Some(AnalyzeRootErrorCode::Inaccessible),
        },
    }
}

fn absolute_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        std::env::current_dir().ok().map(|cwd| cwd.join(path))
    }
}

fn qualify_aliases(roots: &mut [AnalyzeRootRecordV1]) {
    let mut first_for_volume: HashMap<String, String> = HashMap::new();
    for root in roots.iter_mut() {
        if root.error.is_some() {
            continue;
        }
        let Some(volume) = root.volume.clone() else {
            continue;
        };
        let identity = root
            .normalized
            .clone()
            .unwrap_or_else(|| root.input.clone());
        if let Some(first) = first_for_volume.get(&volume) {
            if first != &identity {
                root.alias_of = Some(first.clone());
            }
        } else {
            first_for_volume.insert(volume, identity);
        }
    }
}

/// Returns one page of represented children. Order is snapshot id order.
pub fn list_analyze_children(
    snapshot: &AnalyzeSnapshotV1,
    parent_id: u32,
    offset: u32,
    limit: u32,
) -> Result<AnalyzeListV1, AnalyzeObserveError> {
    if snapshot.version != ANALYZE_SNAPSHOT_VERSION {
        return Err(AnalyzeObserveError::InvalidSnapshot);
    }
    let mut by_id = HashMap::with_capacity(snapshot.nodes.len());
    for node in &snapshot.nodes {
        if by_id.insert(node.id, node).is_some() {
            return Err(AnalyzeObserveError::InvalidSnapshot);
        }
    }
    if !by_id.contains_key(&parent_id) {
        return Err(AnalyzeObserveError::UnknownNode(parent_id));
    }
    let mut children = snapshot
        .nodes
        .iter()
        .filter(|node| node.parent_id == Some(parent_id))
        .collect::<Vec<_>>();
    children.sort_by_key(|node| node.id);
    let total_children = u32::try_from(children.len()).unwrap_or(u32::MAX);
    let limit = limit.clamp(1, ANALYZE_LIST_MAX_LIMIT);
    let start = usize::try_from(offset).unwrap_or(usize::MAX);
    let page = children.into_iter().skip(start).take(limit as usize);
    Ok(AnalyzeListV1 {
        version: ANALYZE_OBSERVE_VERSION,
        parent_id,
        total_children,
        offset,
        limit,
        children: page.map(child_record).collect(),
    })
}

fn child_record(node: &AnalyzeNodeV1) -> AnalyzeListChildV1 {
    AnalyzeListChildV1 {
        id: node.id,
        name: node.name.clone(),
        kind: node.kind,
        bytes: node.bytes,
        evidence: node.evidence,
        immediate_count: node.immediate_count,
        warnings: node.warnings.clone(),
    }
}

/// Explains one node from path evidence. The node's byte count is not read.
pub fn explain_analyze_node(
    snapshot: &AnalyzeSnapshotV1,
    node_id: u32,
) -> Result<AnalyzeExplainV1, AnalyzeObserveError> {
    if snapshot.version != ANALYZE_SNAPSHOT_VERSION {
        return Err(AnalyzeObserveError::InvalidSnapshot);
    }
    let path = analyze_node_path(snapshot, node_id).map_err(|error| match error {
        AnalyzeActionError::UnknownNode(id) => AnalyzeObserveError::UnknownNode(id),
        AnalyzeActionError::InvalidSnapshot(_) => AnalyzeObserveError::InvalidSnapshot,
        _ => AnalyzeObserveError::InvalidSnapshot,
    })?;
    let (class, evidence) = classify_path(&path, live_cloud_evidence(&path));
    Ok(AnalyzeExplainV1 {
        version: ANALYZE_OBSERVE_VERSION,
        node_id,
        class,
        evidence,
        disposable: false,
        lock: lock_report(&path),
    })
}

pub(crate) fn classify_path(
    path: &Path,
    cloud: CloudEvidence,
) -> (AnalyzeClass, AnalyzeClassEvidence) {
    let (parts, name) = path_signals(path);
    if name == "hiberfil.sys" {
        return (
            AnalyzeClass::Hibernation,
            AnalyzeClassEvidence::NameHiberfilSys,
        );
    }
    if name == "pagefile.sys" {
        return (
            AnalyzeClass::PageFile,
            AnalyzeClassEvidence::NamePagefileSys,
        );
    }
    if name == "swapfile.sys" {
        return (
            AnalyzeClass::PageFile,
            AnalyzeClassEvidence::NameSwapfileSys,
        );
    }
    if is_docker_disk(&parts, &name) {
        return (
            AnalyzeClass::DockerDisk,
            AnalyzeClassEvidence::PathDockerVhdx,
        );
    }
    if is_wsl_disk(&parts, &name) {
        return (AnalyzeClass::WslDisk, AnalyzeClassEvidence::PathWslExt4Vhdx);
    }
    if let Some(evidence) = ai_model_evidence(&parts, &name) {
        return (AnalyzeClass::AiModel, evidence);
    }
    if let Some(evidence) = backup_evidence(&parts, &name) {
        return (AnalyzeClass::Backup, evidence);
    }
    if let Some(evidence) = game_evidence(&parts, &name) {
        return (AnalyzeClass::Game, evidence);
    }
    match cloud {
        CloudEvidence::Reparse => (
            AnalyzeClass::CloudPlaceholder,
            AnalyzeClassEvidence::CloudReparse,
        ),
        CloudEvidence::Recall => (
            AnalyzeClass::CloudPlaceholder,
            AnalyzeClassEvidence::CloudRecallAttribute,
        ),
        CloudEvidence::None => (AnalyzeClass::Unknown, AnalyzeClassEvidence::None),
    }
}

fn path_signals(path: &Path) -> (Vec<String>, String) {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                parts.push(value.to_string_lossy().to_ascii_lowercase());
            }
            Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().to_ascii_lowercase());
            }
            Component::RootDir | Component::CurDir | Component::ParentDir => {}
        }
    }
    let name = parts.last().cloned().unwrap_or_default();
    (parts, name)
}

fn parents(parts: &[String]) -> &[String] {
    parts.split_last().map_or(parts, |(_, parents)| parents)
}

fn is_docker_disk(parts: &[String], name: &str) -> bool {
    if name == "docker_data.vhdx" {
        return true;
    }
    name.ends_with(".vhdx")
        && parents(parts)
            .iter()
            .any(|part| part == "docker" || part == "docker-desktop")
}

fn is_wsl_disk(parts: &[String], name: &str) -> bool {
    if name != "ext4.vhdx" {
        return false;
    }
    parents(parts)
        .iter()
        .any(|part| part == "wsl" || part.contains("wsl$") || part.contains("wsl.localhost"))
}

fn ai_model_evidence(parts: &[String], name: &str) -> Option<AnalyzeClassEvidence> {
    if name.ends_with(".gguf") {
        return Some(AnalyzeClassEvidence::ExtensionGguf);
    }
    if name.ends_with(".safetensors") {
        return Some(AnalyzeClassEvidence::ExtensionSafetensors);
    }
    let parent = parents(parts);
    let ollama = parent
        .iter()
        .any(|part| part == "ollama" || part == ".ollama");
    let huggingface = parent
        .iter()
        .any(|part| part == "huggingface" || part == ".huggingface");
    let store = parent
        .iter()
        .any(|part| part == "models" || part == "blobs" || part == "hub");
    if ollama && store {
        return Some(AnalyzeClassEvidence::PathOllamaModels);
    }
    if huggingface && store && (name.ends_with(".bin") || name.ends_with(".onnx")) {
        return Some(AnalyzeClassEvidence::PathHuggingfaceHub);
    }
    None
}

fn backup_evidence(parts: &[String], name: &str) -> Option<AnalyzeClassEvidence> {
    if parents(parts)
        .iter()
        .any(|part| part == "windowsimagebackup")
    {
        return Some(AnalyzeClassEvidence::PathWindowsImageBackup);
    }
    if parents(parts).iter().any(|part| part == "filehistory") {
        return Some(AnalyzeClassEvidence::PathFileHistory);
    }
    if name.ends_with(".wbcat") {
        return Some(AnalyzeClassEvidence::ExtensionWbcat);
    }
    None
}

fn game_evidence(parts: &[String], name: &str) -> Option<AnalyzeClassEvidence> {
    let parent = parents(parts);
    let steam = parent.iter().any(|part| part == "steamapps");
    if steam && parent.iter().any(|part| part == "common") {
        return Some(AnalyzeClassEvidence::PathSteamappsCommon);
    }
    if steam && name.starts_with("appmanifest_") && name.ends_with(".acf") {
        return Some(AnalyzeClassEvidence::NameAppManifestAcf);
    }
    if parent.iter().any(|part| part == "epic games") {
        return Some(AnalyzeClassEvidence::PathEpicGames);
    }
    if parent.iter().any(|part| part == "xboxgames") {
        return Some(AnalyzeClassEvidence::PathXboxGames);
    }
    None
}

fn live_cloud_evidence(path: &Path) -> CloudEvidence {
    if cloud_reparse_tag(path) {
        return CloudEvidence::Reparse;
    }
    if recall_attribute(path) {
        return CloudEvidence::Recall;
    }
    CloudEvidence::None
}

fn cloud_reparse_tag(path: &Path) -> bool {
    match inspect_entry_no_follow(path, &SystemPathReparseProbe) {
        Ok(entry) => matches!(
            entry.safety,
            PathSafety::ReparsePoint { tag: Some(tag) } if is_cloud_reparse_tag(tag)
        ),
        Err(_) => false,
    }
}

fn is_cloud_reparse_tag(tag: u32) -> bool {
    const MASK: u32 = 0x0000_F000;
    const BASE: u32 = 0x9000_001A;
    (tag & !MASK) == BASE
}

fn lock_report(path: &Path) -> AnalyzeLockReportV1 {
    if path.symlink_metadata().is_err() {
        return empty_lock_report(false);
    }
    query_lock_owners(path)
}

fn empty_lock_report(supported: bool) -> AnalyzeLockReportV1 {
    AnalyzeLockReportV1 {
        supported,
        terminates_process: false,
        truncated: false,
        owners: Vec::new(),
    }
}

#[cfg(windows)]
fn logical_mounts() -> Result<Vec<PathBuf>, ()> {
    use std::os::windows::ffi::OsStringExt;

    use windows_sys::Win32::Storage::FileSystem::GetLogicalDriveStringsW;

    let required = unsafe { GetLogicalDriveStringsW(0, std::ptr::null_mut()) };
    if required == 0 {
        return Err(());
    }
    let mut buffer = vec![0_u16; required as usize];
    // SAFETY: `buffer` is writable for `required` UTF-16 units, including the final extra NUL.
    let written = unsafe { GetLogicalDriveStringsW(required, buffer.as_mut_ptr()) };
    if written == 0 || (written as usize) >= buffer.len() {
        return Err(());
    }
    let mut mounts = Vec::new();
    let mut offset = 0;
    while offset < written as usize {
        let Some(end) = buffer[offset..].iter().position(|unit| *unit == 0) else {
            break;
        };
        if end == 0 {
            break;
        }
        let mount = std::ffi::OsString::from_wide(&buffer[offset..offset + end]);
        mounts.push(PathBuf::from(mount));
        offset += end + 1;
    }
    Ok(mounts)
}

#[cfg(not(windows))]
fn logical_mounts() -> Result<Vec<PathBuf>, ()> {
    let root = PathBuf::from("/");
    if root.symlink_metadata().is_ok() {
        Ok(vec![root])
    } else {
        Ok(Vec::new())
    }
}

#[cfg(windows)]
fn drive_has_no_root(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;

    const DRIVE_NO_ROOT_DIR: u32 = 1;
    let text = path.to_string_lossy();
    let drive_root = text.len() <= 3 && text.as_bytes().get(1) == Some(&b':');
    if !drive_root {
        return false;
    }
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    // SAFETY: `wide` is a NUL-terminated path.
    let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
    kind == DRIVE_NO_ROOT_DIR
}

#[cfg(not(windows))]
fn drive_has_no_root(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
fn recall_attribute(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN, GetFileAttributesW,
        INVALID_FILE_ATTRIBUTES,
    };

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    // SAFETY: `wide` is a NUL-terminated path. The call only reads attributes.
    let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
    attributes != INVALID_FILE_ATTRIBUTES
        && attributes & (FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS | FILE_ATTRIBUTE_RECALL_ON_OPEN) != 0
}

#[cfg(not(windows))]
fn recall_attribute(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
fn query_lock_owners(path: &Path) -> AnalyzeLockReportV1 {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::{
        Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS},
        System::RestartManager::{
            CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
            RmStartSession,
        },
    };

    struct Session(u32);

    impl Drop for Session {
        fn drop(&mut self) {
            // SAFETY: the handle was returned by RmStartSession in this process.
            // RmEndSession releases the session. It does not shut processes down.
            unsafe {
                let _ = RmEndSession(self.0);
            }
        }
    }

    let mut session = 0_u32;
    let mut key = [0_u16; (CCH_RM_SESSION_KEY as usize) + 1];
    // SAFETY: `session` and `key` are valid outputs. This starts a query session only.
    let started = unsafe { RmStartSession(&mut session, 0, key.as_mut_ptr()) };
    if started != ERROR_SUCCESS {
        return empty_lock_report(false);
    }
    let session = Session(session);
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let name = wide.as_ptr();
    // SAFETY: `name` points at a NUL-terminated path that outlives the call.
    let registered = unsafe {
        RmRegisterResources(
            session.0,
            1,
            &name,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        )
    };
    if registered != ERROR_SUCCESS {
        return empty_lock_report(false);
    }
    let mut needed = 0_u32;
    let mut count = 0_u32;
    let mut reasons = 0_u32;
    // SAFETY: a zero count and a null process buffer asks for the required size only.
    let listed = unsafe {
        RmGetList(
            session.0,
            &mut needed,
            &mut count,
            std::ptr::null_mut(),
            &mut reasons,
        )
    };
    if listed != ERROR_SUCCESS && listed != ERROR_MORE_DATA {
        return empty_lock_report(false);
    }
    if needed == 0 {
        return empty_lock_report(true);
    }
    let truncated = needed > LOCK_OWNER_CAP;
    let rows = needed.min(LOCK_OWNER_CAP);
    let mut infos = vec![RM_PROCESS_INFO::default(); rows as usize];
    count = rows;
    // SAFETY: `infos` is writable for `count` records. RmGetList does not terminate processes.
    let listed = unsafe {
        RmGetList(
            session.0,
            &mut needed,
            &mut count,
            infos.as_mut_ptr(),
            &mut reasons,
        )
    };
    if listed != ERROR_SUCCESS && listed != ERROR_MORE_DATA {
        return AnalyzeLockReportV1 {
            supported: true,
            terminates_process: false,
            truncated,
            owners: Vec::new(),
        };
    }
    let owners = infos
        .iter()
        .take(count as usize)
        .map(|info| AnalyzeLockOwnerV1 {
            pid: info.Process.dwProcessId,
            name: wide_lossy(&info.strAppName),
        })
        .collect();
    AnalyzeLockReportV1 {
        supported: true,
        terminates_process: false,
        truncated: truncated || listed == ERROR_MORE_DATA,
        owners,
    }
}

#[cfg(windows)]
fn wide_lossy(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

#[cfg(not(windows))]
fn query_lock_owners(_path: &Path) -> AnalyzeLockReportV1 {
    empty_lock_report(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{ANALYZE_SNAPSHOT_VERSION, AnalyzeRootIdentity, analyze_path};

    struct FakeSource {
        mounts: Vec<PathBuf>,
        volumes: HashMap<PathBuf, Result<String, FsFail>>,
        list_calls: std::cell::Cell<u32>,
    }

    impl RootSource for FakeSource {
        fn list_mounts(&self) -> Result<Vec<PathBuf>, ()> {
            self.list_calls.set(self.list_calls.get() + 1);
            Ok(self.mounts.clone())
        }

        fn volume_id(&self, path: &Path) -> Result<String, FsFail> {
            self.volumes
                .get(path)
                .cloned()
                .unwrap_or(Err(FsFail::NotFound))
        }
    }

    #[test]
    fn missing_requested_root_is_not_replaced_by_a_profile() {
        let profile = PathBuf::from(r"C:\Users\profile");
        let missing = PathBuf::from(r"C:\missing-root");
        let source = FakeSource {
            mounts: vec![profile.clone()],
            volumes: HashMap::from([(profile, Ok("profile-volume".to_string()))]),
            list_calls: std::cell::Cell::new(0),
        };
        let report =
            list_analyze_roots_with(&source, std::slice::from_ref(&missing)).expect("roots");
        assert_eq!(source.list_calls.get(), 0);
        assert_eq!(report.roots.len(), 1);
        assert_eq!(report.roots[0].error, Some(AnalyzeRootErrorCode::Missing));
        assert_eq!(report.roots[0].input, missing.to_string_lossy());
        assert!(report.roots[0].volume.is_none());
        assert!(!report.roots[0].input.contains("profile"));
    }

    #[test]
    fn empty_mount_list_does_not_invent_a_profile() {
        let source = FakeSource {
            mounts: Vec::new(),
            volumes: HashMap::new(),
            list_calls: std::cell::Cell::new(0),
        };
        let report = list_analyze_roots_with(&source, &[]).expect("roots");
        assert_eq!(source.list_calls.get(), 1);
        assert!(report.roots.is_empty());
    }

    #[test]
    fn shared_volume_is_an_alias_and_not_a_second_scan() {
        let first = PathBuf::from(r"C:\mount-one");
        let subst = PathBuf::from(r"C:\mount-two");
        let source = FakeSource {
            mounts: vec![first.clone(), subst.clone()],
            volumes: HashMap::from([
                (first.clone(), Ok("same".to_string())),
                (subst.clone(), Ok("same".to_string())),
            ]),
            list_calls: std::cell::Cell::new(0),
        };
        let report = list_analyze_roots_with(&source, &[]).expect("roots");
        assert!(report.roots[0].alias_of.is_none());
        assert!(report.roots[0].error.is_none());
        assert_eq!(report.roots[1].volume.as_deref(), Some("same"));
        assert!(report.roots[1].alias_of.is_some());
        assert_ne!(
            report.roots[1].alias_of.as_deref(),
            Some(r"C:\Users\profile")
        );
    }

    #[test]
    fn source_failure_is_not_a_profile_scan() {
        struct Down;
        impl RootSource for Down {
            fn list_mounts(&self) -> Result<Vec<PathBuf>, ()> {
                Err(())
            }
            fn volume_id(&self, _path: &Path) -> Result<String, FsFail> {
                Err(FsFail::NotFound)
            }
        }
        let error = list_analyze_roots_with(&Down, &[]).expect_err("unavailable");
        assert_eq!(error, AnalyzeObserveError::RootsUnavailable);
    }

    #[test]
    fn classes_use_exact_evidence_and_size_cannot_select_one() {
        let cases = [
            (r"C:\hiberfil.sys", AnalyzeClass::Hibernation),
            (r"C:\pagefile.sys", AnalyzeClass::PageFile),
            (r"C:\swapfile.sys", AnalyzeClass::PageFile),
            (r"C:\wsl\ext4.vhdx", AnalyzeClass::WslDisk),
            (
                r"C:\Users\x\AppData\Local\Docker\wsl\data\ext4.vhdx",
                AnalyzeClass::DockerDisk,
            ),
            (
                r"C:\Docker\wsl\disk\docker_data.vhdx",
                AnalyzeClass::DockerDisk,
            ),
            (r"C:\ext4.vhdx", AnalyzeClass::Unknown),
            (r"C:\models\demo.gguf", AnalyzeClass::AiModel),
            (r"C:\models\demo.safetensors", AnalyzeClass::AiModel),
            (
                r"C:\Users\x\.ollama\models\blobs\sha",
                AnalyzeClass::AiModel,
            ),
            (
                r"C:\Users\x\.huggingface\hub\snapshots\weight.bin",
                AnalyzeClass::AiModel,
            ),
            (r"C:\models\weight.bin", AnalyzeClass::Unknown),
            (r"C:\WindowsImageBackup\notes.txt", AnalyzeClass::Backup),
            (r"C:\FileHistory\file.bin", AnalyzeClass::Backup),
            (r"C:\backup.wbcat", AnalyzeClass::Backup),
            (r"C:\backup.zip", AnalyzeClass::Unknown),
            (
                r"C:\Steam\steamapps\common\Game\data.bin",
                AnalyzeClass::Game,
            ),
            (r"C:\Steam\steamapps\appmanifest_10.acf", AnalyzeClass::Game),
            (r"C:\Epic Games\Title\game.bin", AnalyzeClass::Game),
            (r"C:\XboxGames\Title", AnalyzeClass::Game),
            (r"C:\Games\huge.bin", AnalyzeClass::Unknown),
            (r"C:\notes.bin", AnalyzeClass::Unknown),
        ];
        for (path, class) in cases {
            let (found, evidence) = classify_path(Path::new(path), CloudEvidence::None);
            assert_eq!(found, class, "{path}");
            if class == AnalyzeClass::Unknown {
                assert_eq!(evidence, AnalyzeClassEvidence::None, "{path}");
            } else {
                assert_ne!(evidence, AnalyzeClassEvidence::None, "{path}");
            }
        }
        let (cloud, evidence) = classify_path(Path::new(r"C:\notes.bin"), CloudEvidence::Reparse);
        assert_eq!(cloud, AnalyzeClass::CloudPlaceholder);
        assert_eq!(evidence, AnalyzeClassEvidence::CloudReparse);
        let (recall, _) = classify_path(Path::new(r"C:\notes.bin"), CloudEvidence::Recall);
        assert_eq!(recall, AnalyzeClass::CloudPlaceholder);
        assert!(!is_cloud_reparse_tag(0xA000_0003));
        assert!(is_cloud_reparse_tag(0x9000_001A));
        assert!(is_cloud_reparse_tag(0x9000_F01A));
    }

    #[test]
    fn explain_ignores_byte_count_and_is_not_a_cleanup_plan() {
        let wsl = explain_node(&["wsl", "ext4.vhdx"], 1);
        assert_eq!(wsl.class, AnalyzeClass::WslDisk);
        assert!(!wsl.disposable);
        assert!(!wsl.lock.terminates_process);
        let model = explain_node(&["weights.gguf"], u64::MAX);
        assert_eq!(model.class, AnalyzeClass::AiModel);
        assert!(!model.disposable);
        let unknown = explain_node(&["notes.bin"], u64::MAX);
        assert_eq!(unknown.class, AnalyzeClass::Unknown);
        assert_eq!(unknown.evidence, AnalyzeClassEvidence::None);
        assert!(!unknown.disposable);
        let value = serde_json::to_value(&unknown).expect("json");
        assert_eq!(value["disposable"], false);
        assert!(value.get("intent").is_none());
        assert!(value.get("digest").is_none());
        assert!(value.get("action").is_none());
        assert!(!value.to_string().contains("CleanupPlan"));
    }

    #[test]
    fn list_is_bounded_by_id_order_and_rejects_an_unknown_node() {
        let snapshot = tree_snapshot(&[
            ("a.bin", 50),
            ("b.bin", 1),
            ("c.bin", u64::MAX),
            ("d.bin", 3),
        ]);
        let page = list_analyze_children(&snapshot, 0, 1, 2).expect("page");
        assert_eq!(page.total_children, 4);
        assert_eq!(page.children.len(), 2);
        assert_eq!(page.children[0].name, "b.bin");
        assert_eq!(page.children[1].name, "c.bin");
        assert_eq!(page.children[1].bytes, u64::MAX);
        let error = list_analyze_children(&snapshot, 99, 0, 10).expect_err("missing");
        assert_eq!(error, AnalyzeObserveError::UnknownNode(99));
        let file =
            list_analyze_children(&snapshot, 1, 0, ANALYZE_LIST_DEFAULT_LIMIT).expect("file");
        assert_eq!(file.total_children, 0);
        assert!(file.children.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn explain_lock_report_does_not_terminate_the_current_process() {
        let fixture = tempfile::TempDir::new().expect("temp");
        let root = fixture.path().join("root");
        std::fs::create_dir_all(&root).expect("root");
        let file = root.join("locked.bin");
        std::fs::write(&file, [1, 2, 3, 4]).expect("file");
        let hold = std::fs::File::open(&file).expect("hold");
        let outcome = analyze_path(&root, None, None).expect("analyze");
        let node = outcome
            .snapshot()
            .nodes
            .iter()
            .find(|node| node.name == "locked.bin")
            .expect("node");
        let pid = std::process::id();
        let explained = explain_analyze_node(outcome.snapshot(), node.id).expect("explain");
        assert!(!explained.lock.terminates_process);
        assert_eq!(std::process::id(), pid);
        assert!(file.symlink_metadata().is_ok());
        drop(hold);
    }

    fn explain_node(names: &[&str], bytes: u64) -> AnalyzeExplainV1 {
        let snapshot = chain_snapshot(names, bytes);
        let id = snapshot.nodes.last().expect("node").id;
        explain_analyze_node(&snapshot, id).expect("explain")
    }

    fn tree_snapshot(children: &[(&str, u64)]) -> AnalyzeSnapshotV1 {
        let mut nodes = vec![directory(0, None, "classify-root")];
        for (index, (name, bytes)) in children.iter().enumerate() {
            nodes.push(file((index + 1) as u32, 0, name, *bytes));
        }
        snapshot(nodes)
    }

    fn chain_snapshot(names: &[&str], bytes: u64) -> AnalyzeSnapshotV1 {
        let mut nodes = vec![directory(0, None, "classify-root")];
        for (index, name) in names.iter().enumerate() {
            let id = (index + 1) as u32;
            let parent = index as u32;
            if index + 1 == names.len() {
                nodes.push(file(id, parent, name, bytes));
            } else {
                nodes.push(directory(id, Some(parent), name));
            }
        }
        snapshot(nodes)
    }

    fn snapshot(nodes: Vec<AnalyzeNodeV1>) -> AnalyzeSnapshotV1 {
        let (input, normalized) = if cfg!(windows) {
            ("C:/classify-root", "c:/classify-root")
        } else {
            ("/classify-root", "/classify-root")
        };
        AnalyzeSnapshotV1 {
            version: ANALYZE_SNAPSHOT_VERSION,
            root: AnalyzeRootIdentity {
                input: input.to_string(),
                normalized: normalized.to_string(),
                volume: "vol".to_string(),
            },
            nodes,
            warnings: Vec::new(),
            completeness: crate::analysis::AnalyzeCompleteness::Complete,
            accounted_owned_bytes: 0,
        }
    }

    fn directory(id: u32, parent_id: Option<u32>, name: &str) -> AnalyzeNodeV1 {
        AnalyzeNodeV1 {
            id,
            parent_id,
            kind: AnalyzeNodeKind::Directory,
            name: name.to_string(),
            bytes: 0,
            immediate_count: 0,
            recursive_count: 0,
            evidence: AnalyzeEvidence::Complete,
            warnings: Vec::new(),
            mtime_ms: None,
        }
    }

    fn file(id: u32, parent_id: u32, name: &str, bytes: u64) -> AnalyzeNodeV1 {
        AnalyzeNodeV1 {
            id,
            parent_id: Some(parent_id),
            kind: AnalyzeNodeKind::File,
            name: name.to_string(),
            bytes,
            immediate_count: 0,
            recursive_count: 0,
            evidence: AnalyzeEvidence::Complete,
            warnings: Vec::new(),
            mtime_ms: None,
        }
    }
}
