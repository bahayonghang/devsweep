//! Saved Analyze Recycle Bin plans.
//!
//! The plan embeds one snapshot and frozen path-identity bindings. Execution
//! calls the Clean executor only after the strict digest and those bindings
//! still match. This module does not redefine the Clean V1 digest.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    ANALYZE_SNAPSHOT_VERSION, ANALYZE_TRASH_VERSION, AnalyzeEvidence, AnalyzeNodeKind,
    AnalyzeNodeV1, AnalyzeSnapshotV1, AnalyzeTrashItemV1, AnalyzeTrashRefusalV1, MAX_LOGICAL_DEPTH,
    MAX_OWNED_BYTES, MAX_STORED_NODES, MAX_WARNINGS,
    actions::{
        Accepted, AnalyzeActionError, NodeIndex, TrashEnvironment, build_plan, select, target_id,
    },
};
use crate::{
    digest_encoding::hex_digest,
    execution::{
        ConfirmationDigest, ExecutionReport, ExecutionRequest, Executor, confirmation_digest,
    },
    filesystem::{
        PathIdentity, PathSafety, SystemPathReparseProbe, capture_path_identity,
        inspect_entry_no_follow, normalize_absolute_path, path_is_within,
    },
    process::{CancelObserver, FlagCancelObserver},
};

/// Saved Analyze trash-plan version.
pub const ANALYZE_TRASH_PLAN_VERSION: u32 = 1;
/// Maximum accepted plan document size before deserialize.
pub const ANALYZE_TRASH_PLAN_MAX_BYTES: u64 = 268_435_456;

/// Frozen identity of one selected node. The hash is private `PathIdentity`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeSelectedBindingV1 {
    pub node_id: u32,
    pub kind: AnalyzeNodeKind,
    pub mtime_ms: u64,
    pub identity_sha256: String,
}

/// Closed saved Analyze trash plan. It is not a Clean plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeTrashPlanV1 {
    pub version: u32,
    pub operation_id: String,
    pub snapshot: AnalyzeSnapshotV1,
    pub selected_node_ids: Vec<u32>,
    pub root_identity_sha256: String,
    pub selected_bindings: Vec<AnalyzeSelectedBindingV1>,
}

/// Strict preview. `digest` binds the saved plan and the live Clean digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeTrashStrictPreviewV1 {
    pub version: u32,
    pub operation_id: String,
    pub items: Vec<AnalyzeTrashItemV1>,
    pub refused: Vec<AnalyzeTrashRefusalV1>,
    pub collapsed_node_ids: Vec<u32>,
    pub digest: String,
}

/// Strict execution report. `report` is absent when nothing was dispatched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeTrashStrictReportV1 {
    pub version: u32,
    pub operation_id: String,
    pub moved_node_ids: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ExecutionReport>,
}

/// Failures before an Analyze trash side effect, plus post-dispatch executor errors.
#[derive(Debug)]
pub enum AnalyzeSavedError {
    UnsupportedPlanVersion(u32),
    UnsupportedSnapshotVersion(u32),
    InvalidPlan,
    InvalidSnapshot,
    DuplicateNodeId(u32),
    DuplicateSelection(u32),
    SnapshotLimitExceeded,
    SnapshotRootMismatch,
    PathOutsideRoot(u32),
    IncompleteObservation(u32),
    IdentityUnavailable(u32),
    ChangedSinceSnapshot(u32),
    StaleConfirmation,
    EmptySelection,
    ConfirmationRequired,
    ProtectionUnavailable,
    RevealUnavailable,
    Unsupported,
    Canceled,
    Execution(anyhow::Error),
}

impl std::fmt::Display for AnalyzeSavedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlanVersion(version) => {
                write!(
                    formatter,
                    "unsupported analyze trash plan version {version}"
                )
            }
            Self::UnsupportedSnapshotVersion(version) => {
                write!(formatter, "unsupported analysis snapshot version {version}")
            }
            Self::InvalidPlan => formatter.write_str("analyze trash plan is invalid"),
            Self::InvalidSnapshot => formatter.write_str("analysis snapshot is invalid"),
            Self::DuplicateNodeId(id) => write!(formatter, "duplicate analysis node {id}"),
            Self::DuplicateSelection(id) => write!(formatter, "duplicate analysis selection {id}"),
            Self::SnapshotLimitExceeded => {
                formatter.write_str("analysis snapshot exceeds the byte limit")
            }
            Self::SnapshotRootMismatch => {
                formatter.write_str("analysis snapshot root does not match its input")
            }
            Self::PathOutsideRoot(id) => write!(formatter, "analysis node {id} escapes the root"),
            Self::IncompleteObservation(id) => {
                write!(formatter, "analysis node {id} is an incomplete observation")
            }
            Self::IdentityUnavailable(id) => {
                write!(formatter, "analysis node {id} has no path identity")
            }
            Self::ChangedSinceSnapshot(id) => {
                write!(formatter, "analysis node {id} changed since the snapshot")
            }
            Self::StaleConfirmation => formatter.write_str("analyze trash preview digest is stale"),
            Self::EmptySelection => formatter.write_str("analysis selection is empty"),
            Self::ConfirmationRequired => {
                formatter.write_str("the Recycle Bin move requires explicit confirmation")
            }
            Self::ProtectionUnavailable => formatter
                .write_str("protection list is unavailable; the Recycle Bin move is refused"),
            Self::RevealUnavailable => formatter.write_str("Windows Explorer could not start"),
            Self::Unsupported => formatter.write_str("analyze trash requires Windows"),
            Self::Canceled => formatter.write_str("analyze trash was canceled before dispatch"),
            Self::Execution(error) => write!(formatter, "{error:#}"),
        }
    }
}

impl std::error::Error for AnalyzeSavedError {}

impl AnalyzeSavedError {
    /// Stable machine code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedPlanVersion(_) => "unsupported_plan_version",
            Self::UnsupportedSnapshotVersion(_) => "unsupported_snapshot_version",
            Self::InvalidPlan => "invalid_plan",
            Self::InvalidSnapshot => "invalid_snapshot",
            Self::DuplicateNodeId(_) => "duplicate_node_id",
            Self::DuplicateSelection(_) => "duplicate_selection",
            Self::SnapshotLimitExceeded => "snapshot_limit_exceeded",
            Self::SnapshotRootMismatch => "snapshot_root_mismatch",
            Self::PathOutsideRoot(_) => "path_outside_root",
            Self::IncompleteObservation(_) => "incomplete_observation",
            Self::IdentityUnavailable(_) => "identity_unavailable",
            Self::ChangedSinceSnapshot(_) => "changed_since_snapshot",
            Self::StaleConfirmation => "stale_confirmation",
            Self::EmptySelection => "empty_selection",
            Self::ConfirmationRequired => "confirmation_required",
            Self::ProtectionUnavailable => "protection_unavailable",
            Self::RevealUnavailable => "reveal_unavailable",
            Self::Unsupported => "unsupported_platform",
            Self::Canceled => "canceled",
            Self::Execution(_) => "execution_failed",
        }
    }
}

/// Decodes a saved plan. The byte limit is applied before deserialize.
pub fn decode_analyze_trash_plan(bytes: &[u8]) -> Result<AnalyzeTrashPlanV1, AnalyzeSavedError> {
    if bytes.len() as u64 > ANALYZE_TRASH_PLAN_MAX_BYTES {
        return Err(AnalyzeSavedError::SnapshotLimitExceeded);
    }
    serde_json::from_slice(bytes).map_err(|_| AnalyzeSavedError::InvalidPlan)
}

/// Observes the live snapshot selection and returns a saved plan. No write.
pub fn plan_analyze_trash(
    snapshot: &AnalyzeSnapshotV1,
    node_ids: &[u32],
) -> Result<AnalyzeTrashPlanV1, AnalyzeSavedError> {
    if !cfg!(windows) {
        return Err(AnalyzeSavedError::Unsupported);
    }
    plan_saved_with(snapshot, node_ids)
}

/// Re-observes a saved plan and returns the strict digest.
pub fn preview_analyze_trash_plan(
    plan: &AnalyzeTrashPlanV1,
) -> Result<AnalyzeTrashStrictPreviewV1, AnalyzeSavedError> {
    if !cfg!(windows) {
        return Err(AnalyzeSavedError::Unsupported);
    }
    let environment = TrashEnvironment::production().map_err(map_action)?;
    preview_saved_with(&environment, plan).map(|(preview, _)| preview)
}

/// Re-checks the strict digest, then moves accepted nodes through the Clean executor.
pub fn execute_analyze_trash_plan(
    plan: &AnalyzeTrashPlanV1,
    expected_digest: &str,
    confirmed: bool,
    cancel: Option<std::sync::Arc<FlagCancelObserver>>,
) -> Result<AnalyzeTrashStrictReportV1, AnalyzeSavedError> {
    if !cfg!(windows) {
        return Err(AnalyzeSavedError::Unsupported);
    }
    let environment = TrashEnvironment::production().map_err(map_action)?;
    execute_saved_with(
        &environment,
        &Executor::default(),
        None,
        plan,
        expected_digest,
        confirmed,
        cancel,
    )
}

/// Reveals one node from a structurally valid snapshot.
pub fn reveal_saved_analyze_node(
    snapshot: &AnalyzeSnapshotV1,
    node_id: u32,
) -> Result<(), AnalyzeSavedError> {
    let _path = prepare_reveal(snapshot, node_id)?;
    if !cfg!(windows) {
        return Err(AnalyzeSavedError::Unsupported);
    }
    super::reveal_analyze_node(snapshot, node_id).map_err(map_action)
}

fn plan_saved_with(
    snapshot: &AnalyzeSnapshotV1,
    node_ids: &[u32],
) -> Result<AnalyzeTrashPlanV1, AnalyzeSavedError> {
    validate_selection(node_ids)?;
    validate_saved_snapshot(snapshot)?;
    require_root_agreement(snapshot)?;
    let index = NodeIndex::new(snapshot).map_err(map_action)?;
    let root_identity_sha256 = capture_required(&index, 0)?;
    let mut selected_bindings = Vec::new();
    for &node_id in node_ids {
        let node = index
            .node(node_id)
            .ok_or(AnalyzeSavedError::InvalidSnapshot)?;
        if node.evidence != AnalyzeEvidence::Complete {
            return Err(AnalyzeSavedError::IncompleteObservation(node_id));
        }
        let Some(mtime_ms) = node.mtime_ms else {
            return Err(AnalyzeSavedError::IdentityUnavailable(node_id));
        };
        let identity_sha256 = match capture_live(&index, node_id)? {
            LiveIdentity::Missing => {
                return Err(AnalyzeSavedError::IdentityUnavailable(node_id));
            }
            LiveIdentity::Present {
                hash,
                mtime_ms: live_mtime,
            } => {
                if live_mtime != mtime_ms {
                    return Err(AnalyzeSavedError::ChangedSinceSnapshot(node_id));
                }
                hash
            }
        };
        selected_bindings.push(AnalyzeSelectedBindingV1 {
            node_id,
            kind: node.kind,
            mtime_ms,
            identity_sha256,
        });
    }
    Ok(AnalyzeTrashPlanV1 {
        version: ANALYZE_TRASH_PLAN_VERSION,
        operation_id: next_operation_id(),
        snapshot: snapshot.clone(),
        selected_node_ids: node_ids.to_vec(),
        root_identity_sha256,
        selected_bindings,
    })
}

fn preview_saved_with(
    environment: &TrashEnvironment,
    plan: &AnalyzeTrashPlanV1,
) -> Result<(AnalyzeTrashStrictPreviewV1, Option<String>), AnalyzeSavedError> {
    let view = observe_plan(environment, plan)?;
    let digest = strict_digest(plan, &view)?;
    Ok((
        AnalyzeTrashStrictPreviewV1 {
            version: ANALYZE_TRASH_VERSION,
            operation_id: plan.operation_id.clone(),
            items: view.items,
            refused: view.refused,
            collapsed_node_ids: view.collapsed_node_ids,
            digest,
        },
        view.clean_digest,
    ))
}

fn execute_saved_with<C, T>(
    environment: &TrashEnvironment,
    executor: &Executor<C, T>,
    audit_log: Option<std::path::PathBuf>,
    plan: &AnalyzeTrashPlanV1,
    expected_digest: &str,
    confirmed: bool,
    cancel: Option<std::sync::Arc<FlagCancelObserver>>,
) -> Result<AnalyzeTrashStrictReportV1, AnalyzeSavedError>
where
    C: crate::execution::CommandRunner,
    T: crate::execution::TrashRunner,
{
    if !confirmed {
        return Err(AnalyzeSavedError::ConfirmationRequired);
    }
    if cancel
        .as_ref()
        .is_some_and(|flag| flag.is_cancel_requested())
    {
        return Err(AnalyzeSavedError::Canceled);
    }
    let (preview, clean_digest) = preview_saved_with(environment, plan)?;
    if preview.digest != expected_digest {
        return Err(AnalyzeSavedError::StaleConfirmation);
    }
    if preview.items.is_empty() {
        return Ok(AnalyzeTrashStrictReportV1 {
            version: ANALYZE_TRASH_VERSION,
            operation_id: plan.operation_id.clone(),
            moved_node_ids: Vec::new(),
            report: None,
        });
    }
    let index = NodeIndex::new(&plan.snapshot).map_err(map_action)?;
    let accepted = preview
        .items
        .iter()
        .map(|item| {
            Ok(Accepted {
                node: index
                    .node(item.node_id)
                    .ok_or(AnalyzeSavedError::InvalidSnapshot)?,
                path: item.path.clone(),
            })
        })
        .collect::<Result<Vec<_>, AnalyzeSavedError>>()?;
    let (clean_plan, selected) = build_plan(&plan.operation_id, &accepted).map_err(map_action)?;
    let clean_digest = clean_digest.ok_or(AnalyzeSavedError::InvalidPlan)?;
    let report = executor
        .run_plan(
            &clean_plan,
            ExecutionRequest {
                execute: true,
                audit_log,
                selected,
                expected_digest: Some(ConfirmationDigest::new(clean_digest)),
                cancel,
            },
        )
        .map_err(AnalyzeSavedError::Execution)?;
    let moved_node_ids = preview
        .items
        .iter()
        .filter(|item| {
            report.outcomes.iter().any(|outcome| {
                outcome.target_id == item.target_id
                    && outcome.status == crate::execution::OutcomeStatus::Succeeded
            })
        })
        .map(|item| item.node_id)
        .collect();
    Ok(AnalyzeTrashStrictReportV1 {
        version: ANALYZE_TRASH_VERSION,
        operation_id: plan.operation_id.clone(),
        moved_node_ids,
        report: Some(report),
    })
}

struct PlanView {
    items: Vec<AnalyzeTrashItemV1>,
    refused: Vec<AnalyzeTrashRefusalV1>,
    collapsed_node_ids: Vec<u32>,
    clean_digest: Option<String>,
    live_root_identity_sha256: String,
    live_selected: Vec<(u32, String, u64)>,
}

fn observe_plan(
    environment: &TrashEnvironment,
    plan: &AnalyzeTrashPlanV1,
) -> Result<PlanView, AnalyzeSavedError> {
    validate_plan_shape(plan)?;
    validate_saved_snapshot(&plan.snapshot)?;
    require_root_agreement(&plan.snapshot)?;
    let index = NodeIndex::new(&plan.snapshot).map_err(map_action)?;
    let live_root_identity_sha256 = match capture_live(&index, 0)? {
        LiveIdentity::Present { hash, .. } => hash,
        LiveIdentity::Missing => {
            return Err(AnalyzeSavedError::IdentityUnavailable(0));
        }
    };
    if plan.snapshot.root.volume == "unknown" {
        return Err(AnalyzeSavedError::IdentityUnavailable(0));
    }
    let live_volume = live_volume(&index)?;
    if live_volume != plan.snapshot.root.volume
        || live_root_identity_sha256 != plan.root_identity_sha256
    {
        return Err(AnalyzeSavedError::ChangedSinceSnapshot(0));
    }
    let mut live_selected = Vec::new();
    for binding in &plan.selected_bindings {
        let node = index
            .node(binding.node_id)
            .ok_or(AnalyzeSavedError::InvalidSnapshot)?;
        if node.evidence != AnalyzeEvidence::Complete {
            return Err(AnalyzeSavedError::IncompleteObservation(binding.node_id));
        }
        if node.kind != binding.kind || node.mtime_ms != Some(binding.mtime_ms) {
            return Err(AnalyzeSavedError::ChangedSinceSnapshot(binding.node_id));
        }
        match capture_live(&index, binding.node_id)? {
            LiveIdentity::Missing => {
                live_selected.push((binding.node_id, "missing".to_string(), 0));
            }
            LiveIdentity::Present { hash, mtime_ms } => {
                if hash != binding.identity_sha256 || mtime_ms != binding.mtime_ms {
                    return Err(AnalyzeSavedError::ChangedSinceSnapshot(binding.node_id));
                }
                live_selected.push((binding.node_id, hash, mtime_ms));
            }
        }
    }
    let (accepted, mut refused) =
        select(environment, &plan.snapshot, &plan.selected_node_ids).map_err(map_action)?;
    refused.sort_by_key(|refusal| refusal.node_id);
    let accepted_ids = accepted
        .iter()
        .map(|item| item.node.id)
        .collect::<HashSet<_>>();
    let refused_ids = refused
        .iter()
        .map(|refusal| refusal.node_id)
        .collect::<HashSet<_>>();
    let mut collapsed_node_ids = plan
        .selected_node_ids
        .iter()
        .copied()
        .filter(|id| !accepted_ids.contains(id) && !refused_ids.contains(id))
        .collect::<Vec<_>>();
    collapsed_node_ids.sort_unstable();
    collapsed_node_ids.dedup();
    let clean_digest = if accepted.is_empty() {
        None
    } else {
        let (clean_plan, selected) =
            build_plan(&plan.operation_id, &accepted).map_err(map_action)?;
        Some(
            confirmation_digest(&clean_plan, &selected)
                .map_err(|_| AnalyzeSavedError::InvalidPlan)?
                .as_str()
                .to_string(),
        )
    };
    let items = accepted
        .iter()
        .map(|item| AnalyzeTrashItemV1 {
            node_id: item.node.id,
            target_id: target_id(&plan.operation_id, item.node.id),
            path: item.path.clone(),
            kind: item.node.kind,
            bytes: item.node.bytes,
            evidence: item.node.evidence,
        })
        .collect();
    Ok(PlanView {
        items,
        refused,
        collapsed_node_ids,
        clean_digest,
        live_root_identity_sha256,
        live_selected,
    })
}

fn strict_digest(plan: &AnalyzeTrashPlanV1, view: &PlanView) -> Result<String, AnalyzeSavedError> {
    let snapshot_bytes =
        serde_json::to_vec(&plan.snapshot).map_err(|_| AnalyzeSavedError::InvalidPlan)?;
    let snapshot_sha256 = hex_digest(Sha256::digest(&snapshot_bytes));
    let refused = view
        .refused
        .iter()
        .map(|refusal| (refusal.node_id, refusal.reason_code))
        .collect::<Vec<_>>();
    let accepted = view
        .items
        .iter()
        .map(|item| item.node_id)
        .collect::<Vec<_>>();
    let canonical = serde_json::to_vec(&(
        "devsweep.analyze.trash.strict.v1",
        plan.version,
        &plan.operation_id,
        snapshot_sha256,
        &plan.selected_node_ids,
        &plan.root_identity_sha256,
        &plan.selected_bindings,
        &view.live_root_identity_sha256,
        &view.live_selected,
        accepted,
        refused,
        &view.collapsed_node_ids,
        view.clean_digest.as_deref().unwrap_or(""),
    ))
    .map_err(|_| AnalyzeSavedError::InvalidPlan)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
}

fn prepare_reveal(
    snapshot: &AnalyzeSnapshotV1,
    node_id: u32,
) -> Result<std::path::PathBuf, AnalyzeSavedError> {
    validate_saved_snapshot(snapshot)?;
    super::analyze_node_path(snapshot, node_id).map_err(map_action)
}

/// Structural snapshot checks. Stored `accounted_owned_bytes` is ignored.
pub(crate) fn validate_saved_snapshot(
    snapshot: &AnalyzeSnapshotV1,
) -> Result<(), AnalyzeSavedError> {
    if snapshot.version != ANALYZE_SNAPSHOT_VERSION {
        return Err(AnalyzeSavedError::UnsupportedSnapshotVersion(
            snapshot.version,
        ));
    }
    if snapshot.nodes.is_empty()
        || snapshot.nodes.len() as u32 > MAX_STORED_NODES
        || snapshot.warnings.len() as u32 > MAX_WARNINGS
    {
        return Err(AnalyzeSavedError::InvalidSnapshot);
    }
    if snapshot.snapshot_owned_bytes() > MAX_OWNED_BYTES {
        return Err(AnalyzeSavedError::SnapshotLimitExceeded);
    }
    let root = &snapshot.nodes[0];
    if root.id != 0 || root.parent_id.is_some() || root.kind != AnalyzeNodeKind::Directory {
        return Err(AnalyzeSavedError::InvalidSnapshot);
    }
    if root.name.is_empty() || root.name.contains('\0') {
        return Err(AnalyzeSavedError::InvalidSnapshot);
    }
    let mut seen = HashSet::new();
    let mut by_id = HashMap::new();
    for node in &snapshot.nodes {
        if !seen.insert(node.id) {
            return Err(AnalyzeSavedError::DuplicateNodeId(node.id));
        }
        if node.warnings.len() as u32 > MAX_WARNINGS {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        }
        if node.id != 0 && !child_name_ok(&node.name) {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        }
        by_id.insert(node.id, node);
    }
    if snapshot
        .nodes
        .iter()
        .filter(|node| node.parent_id.is_none())
        .count()
        != 1
    {
        return Err(AnalyzeSavedError::InvalidSnapshot);
    }
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for node in &snapshot.nodes {
        if node.id == 0 {
            continue;
        }
        let Some(parent_id) = node.parent_id else {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        };
        let Some(parent) = by_id.get(&parent_id) else {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        };
        if parent.kind != AnalyzeNodeKind::Directory {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        }
        children.entry(parent_id).or_default().push(node.id);
    }
    let mut visiting = HashSet::new();
    let mut counts = HashMap::new();
    fn walk(
        id: u32,
        children: &HashMap<u32, Vec<u32>>,
        by_id: &HashMap<u32, &AnalyzeNodeV1>,
        visiting: &mut HashSet<u32>,
        counts: &mut HashMap<u32, (u32, u32)>,
        depth: u16,
    ) -> Result<(), AnalyzeSavedError> {
        if depth > MAX_LOGICAL_DEPTH {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        }
        if !visiting.insert(id) {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        }
        let kids = children.get(&id).map(Vec::as_slice).unwrap_or(&[]);
        let mut recursive = kids.len() as u32;
        for &child in kids {
            walk(
                child,
                children,
                by_id,
                visiting,
                counts,
                depth.saturating_add(1),
            )?;
            recursive =
                recursive.saturating_add(counts.get(&child).map(|(_, rec)| *rec).unwrap_or(0));
        }
        visiting.remove(&id);
        counts.insert(id, (kids.len() as u32, recursive));
        let _ = by_id;
        Ok(())
    }
    walk(0, &children, &by_id, &mut visiting, &mut counts, 0)?;
    for node in &snapshot.nodes {
        let Some((immediate, recursive)) = counts.get(&node.id) else {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        };
        if node.immediate_count != *immediate || node.recursive_count != *recursive {
            return Err(AnalyzeSavedError::InvalidSnapshot);
        }
    }
    Ok(())
}

fn require_root_agreement(snapshot: &AnalyzeSnapshotV1) -> Result<(), AnalyzeSavedError> {
    let input = Path::new(&snapshot.root.input);
    let normalized =
        normalize_absolute_path(input).map_err(|_| AnalyzeSavedError::SnapshotRootMismatch)?;
    if normalized != snapshot.root.normalized {
        Err(AnalyzeSavedError::SnapshotRootMismatch)
    } else {
        Ok(())
    }
}

fn validate_plan_shape(plan: &AnalyzeTrashPlanV1) -> Result<(), AnalyzeSavedError> {
    if plan.version != ANALYZE_TRASH_PLAN_VERSION {
        return Err(AnalyzeSavedError::UnsupportedPlanVersion(plan.version));
    }
    if !valid_operation_id(&plan.operation_id)
        || !is_hex64(&plan.root_identity_sha256)
        || plan.selected_node_ids.is_empty()
    {
        return Err(AnalyzeSavedError::InvalidPlan);
    }
    validate_selection(&plan.selected_node_ids)?;
    if plan.selected_bindings.len() != plan.selected_node_ids.len() {
        return Err(AnalyzeSavedError::InvalidPlan);
    }
    for (binding, &node_id) in plan.selected_bindings.iter().zip(&plan.selected_node_ids) {
        if binding.node_id != node_id || !is_hex64(&binding.identity_sha256) {
            return Err(AnalyzeSavedError::InvalidPlan);
        }
    }
    Ok(())
}

fn validate_selection(node_ids: &[u32]) -> Result<(), AnalyzeSavedError> {
    if node_ids.is_empty() {
        return Err(AnalyzeSavedError::EmptySelection);
    }
    let mut seen = HashSet::new();
    for &node_id in node_ids {
        if !seen.insert(node_id) {
            return Err(AnalyzeSavedError::DuplicateSelection(node_id));
        }
    }
    Ok(())
}

fn child_name_ok(name: &str) -> bool {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':', '\0']) {
        return false;
    }
    !name.starts_with("\\\\") && !name.starts_with("//")
}

enum LiveIdentity {
    Missing,
    Present { hash: String, mtime_ms: u64 },
}

fn capture_required(index: &NodeIndex<'_>, node_id: u32) -> Result<String, AnalyzeSavedError> {
    match capture_live(index, node_id)? {
        LiveIdentity::Present { hash, .. } => Ok(hash),
        LiveIdentity::Missing => Err(AnalyzeSavedError::IdentityUnavailable(node_id)),
    }
}

fn capture_live(index: &NodeIndex<'_>, node_id: u32) -> Result<LiveIdentity, AnalyzeSavedError> {
    let path = index.path(node_id).map_err(map_action)?;
    if node_id != 0 {
        let root = index.path(0).map_err(map_action)?;
        if !path_is_within(&root, &path) {
            return Err(AnalyzeSavedError::PathOutsideRoot(node_id));
        }
    }
    let entry = match inspect_entry_no_follow(&path, &SystemPathReparseProbe) {
        Ok(entry) => entry,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LiveIdentity::Missing);
        }
        Err(_) => return Err(AnalyzeSavedError::IdentityUnavailable(node_id)),
    };
    if entry.safety != PathSafety::Safe {
        return Err(AnalyzeSavedError::ChangedSinceSnapshot(node_id));
    }
    let identity = capture_path_identity(&path)
        .map_err(|_| AnalyzeSavedError::IdentityUnavailable(node_id))?;
    let mtime_ms = entry
        .modified
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .ok_or(AnalyzeSavedError::IdentityUnavailable(node_id))?;
    Ok(LiveIdentity::Present {
        hash: hash_identity(&identity),
        mtime_ms,
    })
}

fn live_volume(index: &NodeIndex<'_>) -> Result<String, AnalyzeSavedError> {
    let path = index.path(0).map_err(map_action)?;
    let identity =
        capture_path_identity(&path).map_err(|_| AnalyzeSavedError::IdentityUnavailable(0))?;
    identity
        .value
        .split_once(':')
        .map(|(volume, _)| volume.to_string())
        .filter(|volume| !volume.is_empty())
        .ok_or(AnalyzeSavedError::IdentityUnavailable(0))
}

fn hash_identity(identity: &PathIdentity) -> String {
    let kind = serde_json::to_string(&identity.kind).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(b"devsweep.analyze.path.v1");
    hasher.update([0]);
    hasher.update(kind.as_bytes());
    hasher.update([0]);
    hasher.update(identity.value.as_bytes());
    hex_digest(hasher.finalize())
}

fn map_action(error: AnalyzeActionError) -> AnalyzeSavedError {
    match error {
        AnalyzeActionError::EmptySelection => AnalyzeSavedError::EmptySelection,
        AnalyzeActionError::ProtectionUnavailable => AnalyzeSavedError::ProtectionUnavailable,
        AnalyzeActionError::UnknownNode(_) | AnalyzeActionError::InvalidSnapshot(_) => {
            AnalyzeSavedError::InvalidSnapshot
        }
        AnalyzeActionError::ConfirmationRequired => AnalyzeSavedError::ConfirmationRequired,
        AnalyzeActionError::RevealUnavailable => AnalyzeSavedError::RevealUnavailable,
        AnalyzeActionError::NothingToMove
        | AnalyzeActionError::Plan(_)
        | AnalyzeActionError::Execution(_) => AnalyzeSavedError::InvalidPlan,
    }
}

fn next_operation_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let sequence = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!(
        "analyze-trash-op-{now:016x}-{:08x}-{sequence:016x}",
        std::process::id()
    )
}

fn valid_operation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

fn is_hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Mutex};

    use super::*;
    use crate::{analysis::analyze_path, execution::TrashRunner, filesystem::paths_equal};

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf, TrashEnvironment) {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap().join("root");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("keep.bin"), vec![7_u8; 32]).unwrap();
        fs::write(root.join("docs").join("note.txt"), b"note").unwrap();
        let environment = TrashEnvironment {
            system_locations: vec![root.join("windows")],
            profile: None,
            protections: Vec::new(),
        };
        (temp, root, environment)
    }

    fn snapshot(root: &Path) -> AnalyzeSnapshotV1 {
        analyze_path(root, None, None).unwrap().snapshot().clone()
    }

    fn id_of(snapshot: &AnalyzeSnapshotV1, relative: &str) -> u32 {
        let wanted = Path::new(&snapshot.root.input).join(relative);
        snapshot
            .nodes
            .iter()
            .map(|node| node.id)
            .find(|id| {
                crate::analysis::analyze_node_path(snapshot, *id)
                    .is_ok_and(|path| paths_equal(&path, &wanted))
            })
            .unwrap_or_else(|| panic!("missing {relative}"))
    }

    #[derive(Default, Clone)]
    struct RecordingTrash {
        moved: std::sync::Arc<Mutex<Vec<std::path::PathBuf>>>,
    }

    impl TrashRunner for RecordingTrash {
        fn move_to_trash(&self, path: &Path) -> anyhow::Result<()> {
            self.moved.lock().unwrap().push(path.to_path_buf());
            fs::remove_file(path).or_else(|_| fs::remove_dir_all(path))?;
            Ok(())
        }
    }

    #[test]
    fn forged_accounted_bytes_do_not_bypass_structure_checks() {
        let (_temp, root, _) = fixture();
        let mut stored = snapshot(&root);
        stored.accounted_owned_bytes = 0;
        validate_saved_snapshot(&stored).unwrap();
        stored.nodes.push(stored.nodes[0].clone());
        assert!(matches!(
            validate_saved_snapshot(&stored),
            Err(AnalyzeSavedError::DuplicateNodeId(0))
        ));
        stored.nodes.pop();
        stored.version = 99;
        assert!(matches!(
            validate_saved_snapshot(&stored),
            Err(AnalyzeSavedError::UnsupportedSnapshotVersion(99))
        ));
    }

    #[test]
    fn child_escape_and_duplicate_selection_fail_closed() {
        let (_temp, root, _) = fixture();
        let mut stored = snapshot(&root);
        let docs = id_of(&stored, "docs");
        for node in &mut stored.nodes {
            if node.id == docs {
                node.name = "..".to_string();
            }
        }
        assert!(matches!(
            validate_saved_snapshot(&stored),
            Err(AnalyzeSavedError::InvalidSnapshot)
        ));
        assert!(matches!(
            validate_selection(&[1, 1]),
            Err(AnalyzeSavedError::DuplicateSelection(1))
        ));
        let mut rooted = snapshot(&root);
        rooted.root.normalized = format!("{}/other", rooted.root.normalized);
        assert!(matches!(
            require_root_agreement(&rooted),
            Err(AnalyzeSavedError::SnapshotRootMismatch)
        ));
    }

    #[test]
    fn byte_limit_runs_before_deserialize() {
        assert!(matches!(
            decode_analyze_trash_plan(b"{"),
            Err(AnalyzeSavedError::InvalidPlan)
        ));
        let huge = vec![b' '; ANALYZE_TRASH_PLAN_MAX_BYTES as usize + 1];
        assert!(matches!(
            decode_analyze_trash_plan(&huge),
            Err(AnalyzeSavedError::SnapshotLimitExceeded)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn strict_digest_binds_identity_and_refuses_a_replacement_before_trash() {
        let (_temp, root, environment) = fixture();
        let stored = snapshot(&root);
        let keep = id_of(&stored, "keep.bin");
        let plan = plan_saved_with(&stored, &[keep]).unwrap();
        let (preview, _) = preview_saved_with(&environment, &plan).unwrap();
        assert!(preview.refused.is_empty());
        assert!(preview.digest.starts_with("sha256:"));
        let encoded = serde_json::to_string(&plan).unwrap();
        assert!(!encoded.contains("MoveToTrash"));
        assert!(encoded.contains("snapshot"));

        let path = root.join("keep.bin");
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        fs::remove_file(&path).unwrap();
        fs::write(&path, vec![9_u8; 32]).unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
        let error = preview_saved_with(&environment, &plan).unwrap_err();
        assert!(
            matches!(error, AnalyzeSavedError::ChangedSinceSnapshot(id) if id == keep),
            "{error}"
        );
        assert!(path.exists());

        let stored = snapshot(&root);
        let keep = id_of(&stored, "keep.bin");
        let plan = plan_saved_with(&stored, &[keep]).unwrap();
        let (preview, _) = preview_saved_with(&environment, &plan).unwrap();
        let protected = TrashEnvironment {
            protections: vec![path.clone()],
            ..environment.clone()
        };
        let (protected_preview, _) = preview_saved_with(&protected, &plan).unwrap();
        assert!(protected_preview.refused.iter().any(|refusal| {
            refusal.node_id == keep
                && refusal.reason_code == super::super::AnalyzeTrashRefusalCode::Protected
        }));
        assert_ne!(protected_preview.digest, preview.digest);
        let trash = RecordingTrash::default();
        let executor = Executor::new(crate::execution::ProcessCommandRunner::new(), trash.clone());
        let audit = root.join("clean.jsonl");
        let error = execute_saved_with(
            &protected,
            &executor,
            Some(audit.clone()),
            &plan,
            &preview.digest,
            true,
            None,
        )
        .unwrap_err();
        assert!(
            matches!(error, AnalyzeSavedError::StaleConfirmation),
            "{error}"
        );
        assert!(trash.moved.lock().unwrap().is_empty());
        assert!(!audit.exists());
        assert!(path.exists());

        let report = execute_saved_with(
            &environment,
            &executor,
            Some(audit.clone()),
            &plan,
            &preview.digest,
            true,
            None,
        )
        .unwrap();
        assert_eq!(report.moved_node_ids, vec![keep]);
        assert!(trash.moved.lock().unwrap().len() == 1);
        assert!(audit.exists());
        assert!(!path.exists());
    }

    #[cfg(windows)]
    #[test]
    fn root_identity_change_refuses_preview_and_keeps_the_replacement() {
        let (_temp, root, _) = fixture();
        let stored = snapshot(&root);
        let keep = id_of(&stored, "keep.bin");
        let plan = plan_analyze_trash(&stored, &[keep]).unwrap();

        let parent = root.parent().expect("temp parent");
        let retired = parent.join("retired-root");
        let incoming = parent.join("incoming-root");
        fs::create_dir(&incoming).unwrap();
        fs::write(incoming.join("replacement.bin"), b"replacement").unwrap();
        fs::rename(&root, &retired).unwrap();
        fs::rename(&incoming, &root).unwrap();
        let replacement = root.join("replacement.bin");

        let error = preview_analyze_trash_plan(&plan).unwrap_err();
        assert!(
            matches!(error, AnalyzeSavedError::ChangedSinceSnapshot(0)),
            "{error}"
        );
        assert!(replacement.exists());
    }

    #[cfg(windows)]
    #[test]
    fn empty_acceptance_does_not_call_the_executor_and_ancestor_collapse_is_visible() {
        let (_temp, root, environment) = fixture();
        let stored = snapshot(&root);
        let docs = id_of(&stored, "docs");
        let note = id_of(&stored, "docs/note.txt");
        let plan = plan_saved_with(&stored, &[note, docs]).unwrap();
        let (preview, _) = preview_saved_with(&environment, &plan).unwrap();
        assert_eq!(
            preview
                .items
                .iter()
                .map(|item| item.node_id)
                .collect::<Vec<_>>(),
            vec![docs]
        );
        assert_eq!(preview.collapsed_node_ids, vec![note]);
        let mut incomplete = stored.clone();
        for node in &mut incomplete.nodes {
            if node.id == note {
                node.evidence = AnalyzeEvidence::Incomplete;
            }
        }
        assert!(matches!(
            plan_saved_with(&incomplete, &[note]),
            Err(AnalyzeSavedError::IncompleteObservation(_))
        ));
    }

    #[test]
    fn reveal_rejects_a_malformed_snapshot_before_a_platform_check() {
        let (_temp, root, _) = fixture();
        let mut stored = snapshot(&root);
        stored.nodes.clear();
        assert!(matches!(
            prepare_reveal(&stored, 0),
            Err(AnalyzeSavedError::InvalidSnapshot)
        ));
    }
}
