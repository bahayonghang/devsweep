//! Leftover review after a succeeded uninstall.
//!
//! Discovery is read-only. Removal moves exact reviewed directories to the
//! Recycle Bin only after the Software audit journal holds a succeeded
//! uninstall for the same identity, with a live preview digest and explicit
//! confirmation. Permanent delete is never used.

use std::{
    collections::BTreeSet,
    env, fmt, fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    SoftwareAuditError, SoftwareIdentity, SoftwareInventoryV1, SoftwareSizeBasis,
    SoftwareSizeEvidence, SoftwareSizeSourceCode, SoftwareSupportErrorCode,
    SoftwareSupportOutcomeCode,
    audit::{SoftwareAuditJournal, SoftwareSupportAuditRecordV1},
    software_audit_v1_path, software_id, unix_ms,
};
use crate::{
    execution::{SystemTrashRunner, TrashRunner, UserProtectionList},
    filesystem::{
        DEFAULT_SIZE_ENTRY_BUDGET, PathSafety, SizeEstimate, SystemPathReparseProbe,
        estimate_trees_with_budget_and_cancel, inspect_entry_no_follow, normalize_absolute_path,
        path_is_within,
    },
    process::{CancelObserver, FlagCancelObserver},
};

/// Leftover preview, plan, and report wire version.
pub const SOFTWARE_LEFTOVER_VERSION: u32 = 1;

/// Direct child names of the candidate and system roots that are shared by
/// many products.
const SHARED_DIRECTORY_NAMES: &[&str] = &[
    "microsoft",
    "packages",
    "programs",
    "temp",
    "windows",
    "virtualstore",
    "package cache",
    "crashdumps",
    "common files",
];

/// Where a leftover candidate was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareLeftoverOrigin {
    InstallLocation,
    RoamingAppData,
    LocalAppData,
    ProgramData,
}

/// `certain` candidates come from the product's own `InstallLocation`;
/// `uncertain` candidates only match a directory name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareLeftoverCertainty {
    Certain,
    Uncertain,
}

/// One reviewable leftover directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverCandidateV1 {
    pub id: String,
    pub path: PathBuf,
    pub origin: SoftwareLeftoverOrigin,
    pub certainty: SoftwareLeftoverCertainty,
    /// True only for `certain` candidates.
    pub selected_by_default: bool,
    pub size: SoftwareSizeEvidence,
}

/// Leftover candidates for one inventory entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverAppV1 {
    pub software_id: String,
    pub identity: SoftwareIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    pub app_size: SoftwareSizeEvidence,
    pub candidates: Vec<SoftwareLeftoverCandidateV1>,
}

/// Read-only leftover discovery. It carries no removal authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverPreviewV1 {
    pub version: u32,
    pub apps: Vec<SoftwareLeftoverAppV1>,
}

/// Untrusted desktop request that names one succeeded uninstall and the
/// reviewed candidate ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverSelectionV1 {
    pub software_id: String,
    pub uninstall_operation_id: String,
    pub selected_candidate_ids: Vec<String>,
}

/// Leftover removal plan. Its digest binds identity, names, the uninstall
/// operation, and the exact selected directories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverPlanV1 {
    pub version: u32,
    pub software_id: String,
    pub identity: SoftwareIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    pub uninstall_operation_id: String,
    pub selected_candidate_ids: Vec<String>,
}

/// Live preview of one leftover plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverPlanPreviewV1 {
    pub version: u32,
    pub items: Vec<SoftwareLeftoverCandidateV1>,
    pub digest: String,
}

/// Execution request. Confirmation must be explicit.
#[derive(Debug, Clone)]
pub struct SoftwareLeftoverExecutionRequest<'a> {
    pub plan: &'a SoftwareLeftoverPlanV1,
    pub expected_preview_digest: &'a str,
    pub confirmed: bool,
    pub cancel: Option<Arc<FlagCancelObserver>>,
}

/// Terminal result for one selected leftover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverOutcomeV1 {
    pub candidate_id: String,
    pub certainty: SoftwareLeftoverCertainty,
    pub outcome: SoftwareSupportOutcomeCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<SoftwareSupportErrorCode>,
}

/// Leftover removal report. Bytes are "moved to the Recycle Bin".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverReportV1 {
    pub version: u32,
    pub operation_id: String,
    pub software_id: String,
    pub uninstall_operation_id: String,
    pub outcomes: Vec<SoftwareLeftoverOutcomeV1>,
    /// Sum of known sizes of succeeded moves.
    pub moved_known_bytes: u64,
    /// True when a succeeded move had a partial or unknown size.
    pub lower_bound: bool,
}

/// Failures before any leftover side effect.
#[derive(Debug)]
pub enum SoftwareLeftoverError {
    ConfirmationRequired,
    UnsupportedPlanVersion(u32),
    UnknownSelection(String),
    InvalidIdentity(String),
    EmptySelection,
    DuplicateSelection(String),
    StaleCandidate(String),
    UninstallNotSucceeded(String),
    DigestMismatch,
    ProtectionUnavailable,
    SerializationFailed,
    Audit(SoftwareAuditError),
}

impl fmt::Display for SoftwareLeftoverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfirmationRequired => {
                formatter.write_str("leftover removal requires explicit confirmation")
            }
            Self::UnsupportedPlanVersion(version) => {
                write!(formatter, "unsupported leftover plan version {version}")
            }
            Self::UnknownSelection(id) => write!(formatter, "unknown software selection {id}"),
            Self::InvalidIdentity(id) => {
                write!(formatter, "software id does not match its identity: {id}")
            }
            Self::EmptySelection => formatter.write_str("leftover selection is empty"),
            Self::DuplicateSelection(id) => write!(formatter, "duplicate leftover selection {id}"),
            Self::StaleCandidate(id) => {
                write!(formatter, "leftover candidate {id} is no longer present")
            }
            Self::UninstallNotSucceeded(id) => write!(
                formatter,
                "Software audit has no succeeded uninstall for operation {id}"
            ),
            Self::DigestMismatch => formatter.write_str("leftover preview digest mismatch"),
            Self::ProtectionUnavailable => {
                formatter.write_str("protection list is unavailable; leftovers are not reviewed")
            }
            Self::SerializationFailed => {
                formatter.write_str("leftover canonical serialization failed")
            }
            Self::Audit(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SoftwareLeftoverError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Audit(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SoftwareAuditError> for SoftwareLeftoverError {
    fn from(value: SoftwareAuditError) -> Self {
        Self::Audit(value)
    }
}

/// Candidate roots and exclusions for one run.
#[derive(Debug, Clone)]
struct LeftoverEnvironment {
    roaming: Option<PathBuf>,
    local: Option<PathBuf>,
    program_data: Option<PathBuf>,
    /// A candidate may not equal or contain any of these paths.
    system_roots: Vec<PathBuf>,
    /// A candidate may not sit inside any of these trees.
    system_trees: Vec<PathBuf>,
    protections: Vec<PathBuf>,
}

impl LeftoverEnvironment {
    fn production() -> Result<Self, SoftwareLeftoverError> {
        let var = |key: &str| {
            env::var_os(key)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        let protections = UserProtectionList::load()
            .map_err(|_| SoftwareLeftoverError::ProtectionUnavailable)?
            .list()
            .to_vec();
        let mut system_roots = Vec::new();
        let mut system_trees = Vec::new();
        for key in ["WINDIR", "SystemRoot"] {
            if let Some(path) = var(key) {
                system_trees.push(path);
            }
        }
        for key in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
            if let Some(path) = var(key) {
                system_trees.push(path.join("WindowsApps"));
                system_roots.push(path);
            }
        }
        for key in [
            "ProgramData",
            "APPDATA",
            "LOCALAPPDATA",
            "USERPROFILE",
            "PUBLIC",
            "HOME",
        ] {
            if let Some(path) = var(key) {
                system_roots.push(path);
            }
        }
        if let Some(drive) = env::var_os("SystemDrive").filter(|value| !value.is_empty()) {
            let mut root = drive;
            root.push(std::path::MAIN_SEPARATOR_STR);
            system_roots.push(PathBuf::from(root));
        }
        Ok(Self {
            roaming: var("APPDATA"),
            local: var("LOCALAPPDATA"),
            program_data: var("ProgramData"),
            system_roots,
            system_trees,
            protections,
        })
    }

    fn name_roots(&self) -> Vec<(&Path, SoftwareLeftoverOrigin)> {
        [
            (&self.roaming, SoftwareLeftoverOrigin::RoamingAppData),
            (&self.local, SoftwareLeftoverOrigin::LocalAppData),
            (&self.program_data, SoftwareLeftoverOrigin::ProgramData),
        ]
        .into_iter()
        .filter_map(|(root, origin)| root.as_deref().map(|root| (root, origin)))
        .collect()
    }

    /// Returns why `path` can never be a candidate, if it cannot.
    fn exclusion(&self, path: &Path) -> Option<SoftwareSupportErrorCode> {
        if !path.is_absolute() || normalize_absolute_path(path).is_err() {
            return Some(SoftwareSupportErrorCode::UnsafePath);
        }
        let roots = || {
            self.system_roots
                .iter()
                .chain(self.roaming.iter())
                .chain(self.local.iter())
                .chain(self.program_data.iter())
        };
        for root in roots() {
            // The candidate equals or contains a system or candidate root.
            if path_is_within(path, root) {
                return Some(SoftwareSupportErrorCode::UnsafePath);
            }
        }
        // A shared folder directly under a root (for example
        // `%LOCALAPPDATA%\Programs`) is never a candidate, even as an
        // `InstallLocation`.
        let shared = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| SHARED_DIRECTORY_NAMES.contains(&name.to_lowercase().as_str()));
        if shared
            && path.parent().is_some_and(|parent| {
                roots().any(|root| path_is_within(parent, root) && path_is_within(root, parent))
            })
        {
            return Some(SoftwareSupportErrorCode::UnsafePath);
        }
        if self
            .system_trees
            .iter()
            .any(|tree| path_is_within(tree, path))
        {
            return Some(SoftwareSupportErrorCode::UnsafePath);
        }
        if self
            .protections
            .iter()
            .any(|protected| path_is_within(protected, path) || path_is_within(path, protected))
        {
            return Some(SoftwareSupportErrorCode::Protected);
        }
        match env::current_exe() {
            Ok(exe) if !path_is_within(path, &exe) => None,
            _ => Some(SoftwareSupportErrorCode::Protected),
        }
    }
}

/// Discovery input for one app.
struct LeftoverSubject<'a> {
    software_id: &'a str,
    display_name: Option<&'a str>,
    publisher: Option<&'a str>,
    install_location: Option<PathBuf>,
}

/// Discovers leftover candidates for the selected inventory entries. Read-only.
pub fn discover_software_leftovers(
    inventory: &SoftwareInventoryV1,
    software_ids: &[String],
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<SoftwareLeftoverPreviewV1, SoftwareLeftoverError> {
    discover_with(
        &LeftoverEnvironment::production()?,
        inventory,
        software_ids,
        install_location,
        cancel,
    )
}

/// Builds a removal plan and its live digest for one succeeded uninstall.
pub fn plan_software_leftovers(
    inventory: &SoftwareInventoryV1,
    selection: &SoftwareLeftoverSelectionV1,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<(SoftwareLeftoverPlanV1, SoftwareLeftoverPlanPreviewV1), SoftwareLeftoverError> {
    plan_with(
        &LeftoverEnvironment::production()?,
        &software_audit_v1_path()?,
        inventory,
        selection,
        install_location,
        cancel,
    )
}

/// Moves the exact reviewed directories to the Recycle Bin.
pub fn execute_software_leftovers(
    request: SoftwareLeftoverExecutionRequest<'_>,
) -> Result<SoftwareLeftoverReportV1, SoftwareLeftoverError> {
    if !request.confirmed {
        return Err(SoftwareLeftoverError::ConfirmationRequired);
    }
    execute_with(
        &LeftoverEnvironment::production()?,
        &software_audit_v1_path()?,
        &SystemTrashRunner,
        install_location,
        request,
    )
}

fn install_location(identity: &SoftwareIdentity) -> Option<PathBuf> {
    match identity {
        SoftwareIdentity::Arp { hive, view, subkey } => {
            super::arp::install_location(*hive, *view, subkey)
        }
        // MSIX package directories are owned by the deployment service and
        // live under WindowsApps; MSI identities carry no install location.
        SoftwareIdentity::Msi { .. } | SoftwareIdentity::Msix { .. } => None,
    }
}

fn discover_with(
    environment: &LeftoverEnvironment,
    inventory: &SoftwareInventoryV1,
    software_ids: &[String],
    locate: fn(&SoftwareIdentity) -> Option<PathBuf>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<SoftwareLeftoverPreviewV1, SoftwareLeftoverError> {
    let mut apps = Vec::with_capacity(software_ids.len());
    let mut seen = BTreeSet::new();
    for id in software_ids {
        if !seen.insert(id) {
            return Err(SoftwareLeftoverError::DuplicateSelection(id.clone()));
        }
        let entry = inventory
            .entries
            .iter()
            .find(|entry| &entry.id == id)
            .ok_or_else(|| SoftwareLeftoverError::UnknownSelection(id.clone()))?;
        verify_software_id(&entry.id, &entry.identity)?;
        let candidates = discover_candidates(
            environment,
            &LeftoverSubject {
                software_id: &entry.id,
                display_name: entry.display_name.as_deref(),
                publisher: entry.publisher.as_deref(),
                install_location: locate(&entry.identity),
            },
            cancel,
        );
        apps.push(SoftwareLeftoverAppV1 {
            software_id: entry.id.clone(),
            identity: entry.identity.clone(),
            display_name: entry.display_name.clone(),
            publisher: entry.publisher.clone(),
            app_size: entry.size.clone(),
            candidates,
        });
    }
    Ok(SoftwareLeftoverPreviewV1 {
        version: SOFTWARE_LEFTOVER_VERSION,
        apps,
    })
}

fn plan_with(
    environment: &LeftoverEnvironment,
    audit_path: &Path,
    inventory: &SoftwareInventoryV1,
    selection: &SoftwareLeftoverSelectionV1,
    locate: fn(&SoftwareIdentity) -> Option<PathBuf>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<(SoftwareLeftoverPlanV1, SoftwareLeftoverPlanPreviewV1), SoftwareLeftoverError> {
    let entry = inventory
        .entries
        .iter()
        .find(|entry| entry.id == selection.software_id)
        .ok_or_else(|| SoftwareLeftoverError::UnknownSelection(selection.software_id.clone()))?;
    verify_software_id(&entry.id, &entry.identity)?;
    require_succeeded_uninstall(
        &SoftwareAuditJournal::open(audit_path)?,
        &selection.uninstall_operation_id,
        &entry.identity,
    )?;
    let candidates = discover_candidates(
        environment,
        &LeftoverSubject {
            software_id: &entry.id,
            display_name: entry.display_name.as_deref(),
            publisher: entry.publisher.as_deref(),
            install_location: locate(&entry.identity),
        },
        cancel,
    );
    let items = select_candidates(&candidates, &selection.selected_candidate_ids)?;
    let plan = SoftwareLeftoverPlanV1 {
        version: SOFTWARE_LEFTOVER_VERSION,
        software_id: entry.id.clone(),
        identity: entry.identity.clone(),
        display_name: entry.display_name.clone(),
        publisher: entry.publisher.clone(),
        uninstall_operation_id: selection.uninstall_operation_id.clone(),
        selected_candidate_ids: items.iter().map(|item| item.id.clone()).collect(),
    };
    let digest = plan_digest(&plan, &items)?;
    Ok((
        plan,
        SoftwareLeftoverPlanPreviewV1 {
            version: SOFTWARE_LEFTOVER_VERSION,
            items,
            digest,
        },
    ))
}

fn execute_with(
    environment: &LeftoverEnvironment,
    audit_path: &Path,
    trash: &dyn TrashRunner,
    locate: fn(&SoftwareIdentity) -> Option<PathBuf>,
    request: SoftwareLeftoverExecutionRequest<'_>,
) -> Result<SoftwareLeftoverReportV1, SoftwareLeftoverError> {
    if !request.confirmed {
        return Err(SoftwareLeftoverError::ConfirmationRequired);
    }
    let plan = request.plan;
    if plan.version != SOFTWARE_LEFTOVER_VERSION {
        return Err(SoftwareLeftoverError::UnsupportedPlanVersion(plan.version));
    }
    verify_software_id(&plan.software_id, &plan.identity)?;
    let mut journal = SoftwareAuditJournal::open(audit_path)?;
    require_succeeded_uninstall(&journal, &plan.uninstall_operation_id, &plan.identity)?;
    let candidates = discover_candidates(
        environment,
        &LeftoverSubject {
            software_id: &plan.software_id,
            display_name: plan.display_name.as_deref(),
            publisher: plan.publisher.as_deref(),
            install_location: locate(&plan.identity),
        },
        request.cancel.as_ref(),
    );
    let items = select_candidates(&candidates, &plan.selected_candidate_ids)?;
    if plan_digest(plan, &items)? != request.expected_preview_digest {
        return Err(SoftwareLeftoverError::DigestMismatch);
    }

    let operation_id = next_operation_id();
    let mut outcomes = Vec::with_capacity(items.len());
    let mut moved_known_bytes = 0_u64;
    let mut lower_bound = false;
    for item in &items {
        let canceled = request
            .cancel
            .as_ref()
            .is_some_and(|flag| flag.is_cancel_requested());
        let (outcome, estimated_bytes, error_code) = if canceled {
            (
                SoftwareSupportOutcomeCode::Skipped,
                None,
                Some(SoftwareSupportErrorCode::Canceled),
            )
        } else if let Some(code) = live_refusal(environment, &item.path) {
            (SoftwareSupportOutcomeCode::Skipped, None, Some(code))
        } else {
            match trash.move_to_trash(&item.path) {
                Ok(()) => {
                    let (bytes, partial) = size_bytes(&item.size);
                    if let Some(bytes) = bytes {
                        moved_known_bytes = moved_known_bytes.saturating_add(bytes);
                    }
                    lower_bound |= partial || bytes.is_none();
                    (SoftwareSupportOutcomeCode::Succeeded, bytes, None)
                }
                Err(_) => (
                    SoftwareSupportOutcomeCode::Failed,
                    None,
                    Some(SoftwareSupportErrorCode::TrashFailed),
                ),
            }
        };
        journal.append_support(SoftwareSupportAuditRecordV1::leftover_moved(
            &operation_id,
            unix_ms(SystemTime::now()),
            &plan.uninstall_operation_id,
            &plan.identity,
            &item.id,
            item.certainty,
            outcome,
            estimated_bytes,
            error_code,
        ))?;
        outcomes.push(SoftwareLeftoverOutcomeV1 {
            candidate_id: item.id.clone(),
            certainty: item.certainty,
            outcome,
            estimated_bytes,
            error_code,
        });
    }
    Ok(SoftwareLeftoverReportV1 {
        version: SOFTWARE_LEFTOVER_VERSION,
        operation_id,
        software_id: plan.software_id.clone(),
        uninstall_operation_id: plan.uninstall_operation_id.clone(),
        outcomes,
        moved_known_bytes,
        lower_bound,
    })
}

fn verify_software_id(id: &str, identity: &SoftwareIdentity) -> Result<(), SoftwareLeftoverError> {
    let expected = software_id(identity).map_err(|_| SoftwareLeftoverError::SerializationFailed)?;
    if expected == id {
        Ok(())
    } else {
        Err(SoftwareLeftoverError::InvalidIdentity(id.to_string()))
    }
}

fn require_succeeded_uninstall(
    journal: &SoftwareAuditJournal,
    operation_id: &str,
    identity: &SoftwareIdentity,
) -> Result<(), SoftwareLeftoverError> {
    if journal.uninstall_succeeded(operation_id, identity) {
        Ok(())
    } else {
        Err(SoftwareLeftoverError::UninstallNotSucceeded(
            operation_id.to_string(),
        ))
    }
}

fn select_candidates(
    candidates: &[SoftwareLeftoverCandidateV1],
    selected: &[String],
) -> Result<Vec<SoftwareLeftoverCandidateV1>, SoftwareLeftoverError> {
    if selected.is_empty() {
        return Err(SoftwareLeftoverError::EmptySelection);
    }
    let mut seen = BTreeSet::new();
    for id in selected {
        if !seen.insert(id) {
            return Err(SoftwareLeftoverError::DuplicateSelection(id.clone()));
        }
        if !candidates.iter().any(|candidate| &candidate.id == id) {
            return Err(SoftwareLeftoverError::StaleCandidate(id.clone()));
        }
    }
    Ok(candidates
        .iter()
        .filter(|candidate| seen.contains(&candidate.id))
        .cloned()
        .collect())
}

fn plan_digest(
    plan: &SoftwareLeftoverPlanV1,
    items: &[SoftwareLeftoverCandidateV1],
) -> Result<String, SoftwareLeftoverError> {
    let items = items
        .iter()
        .map(|item| {
            normalize_absolute_path(&item.path)
                .map(|path| (&item.id, path, item.origin, item.certainty))
                .map_err(|_| SoftwareLeftoverError::SerializationFailed)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let canonical = serde_json::to_vec(&("devsweep.software.leftover.plan.v1", plan, items))
        .map_err(|_| SoftwareLeftoverError::SerializationFailed)?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

fn discover_candidates(
    environment: &LeftoverEnvironment,
    subject: &LeftoverSubject<'_>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Vec<SoftwareLeftoverCandidateV1> {
    let mut found: Vec<(PathBuf, SoftwareLeftoverOrigin, SoftwareLeftoverCertainty)> = Vec::new();
    if let Some(location) = &subject.install_location {
        found.push((
            location.clone(),
            SoftwareLeftoverOrigin::InstallLocation,
            SoftwareLeftoverCertainty::Certain,
        ));
    }
    let display = subject.display_name.and_then(path_component);
    let publisher = subject.publisher.and_then(path_component);
    if let Some(display) = display {
        for (root, origin) in environment.name_roots() {
            if !SHARED_DIRECTORY_NAMES.contains(&display.to_lowercase().as_str())
                && let Some(path) = child_named(root, display)
            {
                found.push((path, origin, SoftwareLeftoverCertainty::Uncertain));
            }
            if let Some(publisher) = publisher
                && let Some(vendor) = child_named(root, publisher)
                && let Some(path) = child_named(&vendor, display)
            {
                found.push((path, origin, SoftwareLeftoverCertainty::Uncertain));
            }
        }
    }

    let mut seen = BTreeSet::new();
    let mut accepted = Vec::new();
    for (path, origin, certainty) in found {
        let Ok(key) = normalize_absolute_path(&path).map(|path| path.to_lowercase()) else {
            continue;
        };
        if !seen.insert(key.clone()) {
            continue;
        }
        if live_refusal(environment, &path).is_some() {
            continue;
        }
        accepted.push((key, path, origin, certainty));
    }

    let paths = accepted
        .iter()
        .map(|(_, path, ..)| path.clone())
        .collect::<Vec<_>>();
    let sizes = estimate_trees_with_budget_and_cancel(&paths, DEFAULT_SIZE_ENTRY_BUDGET, cancel);
    let observed_at_unix_ms = unix_ms(SystemTime::now());
    accepted
        .into_iter()
        .zip(sizes)
        .map(
            |((key, path, origin, certainty), size)| SoftwareLeftoverCandidateV1 {
                id: candidate_id(subject.software_id, origin, &key),
                path,
                origin,
                certainty,
                selected_by_default: certainty == SoftwareLeftoverCertainty::Certain,
                size: size_evidence(&size, observed_at_unix_ms),
            },
        )
        .collect()
}

/// Rejects paths that are excluded or are not plain, safe directories now.
fn live_refusal(
    environment: &LeftoverEnvironment,
    path: &Path,
) -> Option<SoftwareSupportErrorCode> {
    if let Some(code) = environment.exclusion(path) {
        return Some(code);
    }
    let probe = SystemPathReparseProbe;
    match inspect_entry_no_follow(path, &probe) {
        Ok(entry) if entry.is_dir && entry.safety == PathSafety::Safe => None,
        Ok(_) => Some(SoftwareSupportErrorCode::UnsafePath),
        Err(_) => Some(SoftwareSupportErrorCode::NotPresent),
    }
}

/// Accepts one plain directory name: no separators, drive, or dot segments.
fn path_component(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    let valid = !trimmed.is_empty()
        && trimmed != "."
        && trimmed != ".."
        && !trimmed.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|'])
        && !trimmed.chars().any(char::is_control);
    valid.then_some(trimmed)
}

/// Finds a direct child directory whose name equals `name` case-insensitively.
fn child_named(root: &Path, name: &str) -> Option<PathBuf> {
    let wanted = name.to_lowercase();
    fs::read_dir(root).ok()?.flatten().find_map(|item| {
        let file_name = item.file_name();
        let matches = file_name
            .to_str()
            .is_some_and(|value| value.to_lowercase() == wanted);
        (matches && item.file_type().is_ok_and(|kind| kind.is_dir())).then(|| item.path())
    })
}

fn candidate_id(software_id: &str, origin: SoftwareLeftoverOrigin, key: &str) -> String {
    let origin = serde_json::to_string(&origin).unwrap_or_default();
    let digest = Sha256::digest(format!(
        "devsweep.software.leftover.v1\0{software_id}\0{origin}\0{key}"
    ));
    format!("leftover:v1:{digest:x}")
}

fn size_evidence(size: &SizeEstimate, observed_at_unix_ms: u64) -> SoftwareSizeEvidence {
    match (size.logical_bytes, size.complete) {
        (Some(value_bytes), true) => SoftwareSizeEvidence::Available {
            value_bytes,
            basis: SoftwareSizeBasis::MeasuredDirectory,
            source_code: SoftwareSizeSourceCode::LeftoverDirectory,
            observed_at_unix_ms,
        },
        (Some(lower_bound_bytes), false) => SoftwareSizeEvidence::Partial {
            lower_bound_bytes,
            basis: SoftwareSizeBasis::MeasuredDirectory,
            source_code: SoftwareSizeSourceCode::LeftoverDirectory,
            reason_code: size
                .warnings
                .first()
                .and_then(|warning| serde_json::to_value(warning.kind).ok())
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "measurement_incomplete".to_string()),
            observed_at_unix_ms,
        },
        (None, _) => SoftwareSizeEvidence::Unknown {
            reason_code: "measurement_unavailable".to_string(),
        },
    }
}

/// Returns `(known bytes, partial)` for one size evidence value.
fn size_bytes(size: &SoftwareSizeEvidence) -> (Option<u64>, bool) {
    match size {
        SoftwareSizeEvidence::Available { value_bytes, .. } => (Some(*value_bytes), false),
        SoftwareSizeEvidence::Partial {
            lower_bound_bytes, ..
        } => (Some(*lower_bound_bytes), true),
        SoftwareSizeEvidence::Unknown { .. } => (None, true),
    }
}

fn next_operation_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let sequence = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!(
        "software-leftover-op-{:016x}-{:08x}-{sequence:016x}",
        unix_ms(SystemTime::now()),
        std::process::id()
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use tempfile::TempDir;

    use super::*;
    use crate::software::{
        SoftwareAuditStatusCode, SoftwareAuditTransition, SoftwareExecutionOutcome,
        SoftwareInstalledState, SoftwareRebootEvidence,
        audit::{SoftwareAuditEvent, SoftwareAuditRecordV1},
        execution::AdapterOutcome,
    };

    struct Fixture {
        _temp: TempDir,
        root: PathBuf,
        environment: LeftoverEnvironment,
        audit_path: PathBuf,
    }

    fn fixture() -> Fixture {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let roaming = root.join("Roaming");
        let local = root.join("Local");
        let program_data = root.join("ProgramData");
        for dir in [&roaming, &local, &program_data, &root.join("Windows")] {
            fs::create_dir_all(dir).unwrap();
        }
        Fixture {
            environment: LeftoverEnvironment {
                roaming: Some(roaming),
                local: Some(local),
                program_data: Some(program_data),
                system_roots: vec![root.clone()],
                system_trees: vec![root.join("Windows")],
                protections: Vec::new(),
            },
            audit_path: root.join("audit").join("software.jsonl"),
            root,
            _temp: temp,
        }
    }

    fn msix_identity() -> SoftwareIdentity {
        SoftwareIdentity::Msix {
            package_full_name: "Contoso.Tools_1.0.0.0_x64__8wekyb3d8bbwe".to_string(),
        }
    }

    fn inventory(identity: SoftwareIdentity, name: &str, publisher: &str) -> SoftwareInventoryV1 {
        let id = software_id(&identity).unwrap();
        serde_json::from_value(serde_json::json!({
            "version": 1,
            "observed_at_unix_ms": 1,
            "sources": [],
            "entries": [{
                "id": id,
                "identity": identity,
                "scope": "current_user",
                "display_name": name,
                "publisher": publisher,
                "provenance": [],
                "eligibility": {"state": "selectable", "reason": "eligible_current_user_msix"},
                "size": {"state": "available", "value_bytes": 4096, "basis": "measured_installed_location", "source_code": "msix_installed_path", "observed_at_unix_ms": 1},
                "last_used": {"state": "unknown", "reason_code": "no_supported_exact_source"}
            }],
            "fingerprint": format!("sha256:{}", "0".repeat(64)),
        }))
        .unwrap()
    }

    fn write(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![7_u8; bytes]).unwrap();
    }

    fn uninstall_record(
        operation_id: &str,
        identity: &SoftwareIdentity,
        transition: SoftwareAuditTransition,
        status_code: SoftwareAuditStatusCode,
        installed_state: Option<SoftwareInstalledState>,
        adapter_outcome: Option<AdapterOutcome>,
    ) -> SoftwareAuditRecordV1 {
        SoftwareAuditRecordV1::new(
            operation_id,
            1,
            identity,
            &format!("sha256:{}", "1".repeat(64)),
            &format!("sha256:{}", "2".repeat(64)),
            SoftwareAuditEvent {
                transition,
                status_code,
                error_code: None,
                reboot_evidence: SoftwareRebootEvidence::None,
                installed_state,
                adapter_outcome,
            },
        )
    }

    /// Writes a complete uninstall with the given terminal to the journal.
    fn record_uninstall(path: &Path, operation_id: &str, removed: bool) {
        let identity = msix_identity();
        let mut journal = SoftwareAuditJournal::open(path).unwrap();
        let installed = if removed {
            SoftwareInstalledState::Absent
        } else {
            SoftwareInstalledState::Present
        };
        for record in [
            uninstall_record(
                operation_id,
                &identity,
                SoftwareAuditTransition::Validated,
                SoftwareAuditStatusCode::Validated,
                None,
                None,
            ),
            uninstall_record(
                operation_id,
                &identity,
                SoftwareAuditTransition::DispatchStarted,
                SoftwareAuditStatusCode::DispatchStarted,
                None,
                None,
            ),
            uninstall_record(
                operation_id,
                &identity,
                SoftwareAuditTransition::AdapterCompleted,
                SoftwareAuditStatusCode::AdapterSucceeded,
                None,
                Some(AdapterOutcome::Success),
            ),
            uninstall_record(
                operation_id,
                &identity,
                SoftwareAuditTransition::RequeryObserved,
                if removed {
                    SoftwareAuditStatusCode::RequeryAbsent
                } else {
                    SoftwareAuditStatusCode::RequeryPresent
                },
                Some(installed),
                None,
            ),
            uninstall_record(
                operation_id,
                &identity,
                SoftwareAuditTransition::Terminal {
                    outcome: if removed {
                        SoftwareExecutionOutcome::Removed
                    } else {
                        SoftwareExecutionOutcome::StillPresent
                    },
                },
                if removed {
                    SoftwareAuditStatusCode::Removed
                } else {
                    SoftwareAuditStatusCode::StillPresent
                },
                Some(installed),
                Some(AdapterOutcome::Success),
            ),
        ] {
            journal.append(record).unwrap();
        }
    }

    #[derive(Default)]
    struct RecordingTrash {
        moved: Mutex<Vec<PathBuf>>,
    }

    impl TrashRunner for RecordingTrash {
        fn move_to_trash(&self, path: &Path) -> anyhow::Result<()> {
            self.moved.lock().unwrap().push(path.to_path_buf());
            fs::remove_dir_all(path)?;
            Ok(())
        }
    }

    fn no_location(_: &SoftwareIdentity) -> Option<PathBuf> {
        None
    }

    #[test]
    fn certain_install_location_is_selected_and_name_matches_are_not() {
        let fixture = fixture();
        let local = fixture.environment.local.clone().unwrap();
        let roaming = fixture.environment.roaming.clone().unwrap();
        write(&local.join("contoso tools").join("cache.bin"), 2048);
        write(
            &roaming.join("Contoso").join("Contoso Tools").join("a.json"),
            10,
        );
        write(&roaming.join("Unrelated").join("b"), 1);
        let install = fixture.root.join("Apps").join("ContosoTools");
        write(&install.join("tool.exe"), 100);
        let subject = LeftoverSubject {
            software_id: "software:v1:msix:fixture",
            display_name: Some("Contoso Tools"),
            publisher: Some("Contoso"),
            install_location: Some(install.clone()),
        };
        let candidates = discover_candidates(&fixture.environment, &subject, None);
        assert_eq!(candidates.len(), 3);
        let certain = &candidates[0];
        assert_eq!(certain.path, install);
        assert_eq!(certain.certainty, SoftwareLeftoverCertainty::Certain);
        assert!(certain.selected_by_default);
        assert!(matches!(
            certain.size,
            SoftwareSizeEvidence::Available {
                value_bytes: 100,
                basis: SoftwareSizeBasis::MeasuredDirectory,
                source_code: SoftwareSizeSourceCode::LeftoverDirectory,
                ..
            }
        ));
        for uncertain in &candidates[1..] {
            assert_eq!(uncertain.certainty, SoftwareLeftoverCertainty::Uncertain);
            assert!(!uncertain.selected_by_default);
        }
        assert!(candidates.iter().any(|candidate| {
            candidate.origin == SoftwareLeftoverOrigin::LocalAppData
                && candidate.path.ends_with("contoso tools")
                && matches!(
                    candidate.size,
                    SoftwareSizeEvidence::Available {
                        value_bytes: 2048,
                        ..
                    }
                )
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate
                .path
                .ends_with(Path::new("Contoso").join("Contoso Tools"))
        }));
    }

    #[test]
    fn protected_paths_system_roots_and_shared_names_are_never_candidates() {
        let mut fixture = fixture();
        let local = fixture.environment.local.clone().unwrap();
        let roaming = fixture.environment.roaming.clone().unwrap();
        write(&local.join("Contoso Tools").join("x"), 1);
        write(&roaming.join("Contoso Tools").join("y"), 1);
        write(&local.join("Microsoft").join("z"), 1);
        fixture.environment.protections = vec![roaming.join("Contoso Tools")];
        let windows_child = fixture.root.join("Windows").join("Contoso Tools");
        fs::create_dir_all(&windows_child).unwrap();
        let shared_programs = local.join("Programs");
        fs::create_dir_all(&shared_programs).unwrap();

        for install_location in [
            Some(fixture.root.clone()),
            Some(local.clone()),
            Some(windows_child),
            Some(shared_programs),
            Some(PathBuf::from("relative\\path")),
        ] {
            let subject = LeftoverSubject {
                software_id: "software:v1:msix:fixture",
                display_name: Some("Contoso Tools"),
                publisher: None,
                install_location,
            };
            let candidates = discover_candidates(&fixture.environment, &subject, None);
            assert_eq!(candidates.len(), 1, "only the local name match survives");
            assert_eq!(candidates[0].origin, SoftwareLeftoverOrigin::LocalAppData);
            assert_eq!(
                candidates[0].certainty,
                SoftwareLeftoverCertainty::Uncertain
            );
        }

        let shared = LeftoverSubject {
            software_id: "software:v1:msix:fixture",
            display_name: Some("Microsoft"),
            publisher: None,
            install_location: None,
        };
        assert!(discover_candidates(&fixture.environment, &shared, None).is_empty());
        for name in ["..", "a\\b", "C:", "   "] {
            let subject = LeftoverSubject {
                software_id: "software:v1:msix:fixture",
                display_name: Some(name),
                publisher: Some(name),
                install_location: None,
            };
            assert!(discover_candidates(&fixture.environment, &subject, None).is_empty());
        }
    }

    #[test]
    fn size_evidence_distinguishes_available_partial_and_unknown() {
        let complete = SizeEstimate::trusted(10, None);
        assert!(matches!(
            size_evidence(&complete, 1),
            SoftwareSizeEvidence::Available {
                value_bytes: 10,
                ..
            }
        ));
        let partial = SizeEstimate {
            logical_bytes: Some(5),
            complete: false,
            last_modified: None,
            warnings: Vec::new(),
        };
        assert!(matches!(
            size_evidence(&partial, 1),
            SoftwareSizeEvidence::Partial {
                lower_bound_bytes: 5,
                ..
            }
        ));
        let unknown = SizeEstimate {
            logical_bytes: None,
            complete: false,
            last_modified: None,
            warnings: Vec::new(),
        };
        assert!(matches!(
            size_evidence(&unknown, 1),
            SoftwareSizeEvidence::Unknown { .. }
        ));
        assert_eq!(size_bytes(&size_evidence(&partial, 1)), (Some(5), true));
        assert_eq!(size_bytes(&size_evidence(&unknown, 1)), (None, true));
    }

    #[test]
    fn removal_requires_succeeded_uninstall_live_digest_and_confirmation() {
        let fixture = fixture();
        let local = fixture.environment.local.clone().unwrap();
        write(&local.join("Contoso Tools").join("cache.bin"), 64);
        let inventory = inventory(msix_identity(), "Contoso Tools", "Contoso");
        let software = inventory.entries[0].id.clone();
        let discovered = discover_with(
            &fixture.environment,
            &inventory,
            std::slice::from_ref(&software),
            no_location,
            None,
        )
        .unwrap();
        let candidate = discovered.apps[0].candidates[0].clone();
        let selection = SoftwareLeftoverSelectionV1 {
            software_id: software.clone(),
            uninstall_operation_id: "software-op-removed".to_string(),
            selected_candidate_ids: vec![candidate.id.clone()],
        };

        // No uninstall terminal in the journal yet.
        let error = plan_with(
            &fixture.environment,
            &fixture.audit_path,
            &inventory,
            &selection,
            no_location,
            None,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            SoftwareLeftoverError::UninstallNotSucceeded(_)
        ));

        // A still-present terminal is not a succeeded uninstall.
        record_uninstall(&fixture.audit_path, "software-op-present", false);
        let still_present = SoftwareLeftoverSelectionV1 {
            uninstall_operation_id: "software-op-present".to_string(),
            ..selection.clone()
        };
        assert!(matches!(
            plan_with(
                &fixture.environment,
                &fixture.audit_path,
                &inventory,
                &still_present,
                no_location,
                None,
            ),
            Err(SoftwareLeftoverError::UninstallNotSucceeded(_))
        ));

        record_uninstall(&fixture.audit_path, "software-op-removed", true);
        let (plan, preview) = plan_with(
            &fixture.environment,
            &fixture.audit_path,
            &inventory,
            &selection,
            no_location,
            None,
        )
        .unwrap();
        assert_eq!(preview.items.len(), 1);
        let trash = RecordingTrash::default();

        let unconfirmed = execute_with(
            &fixture.environment,
            &fixture.audit_path,
            &trash,
            no_location,
            SoftwareLeftoverExecutionRequest {
                plan: &plan,
                expected_preview_digest: &preview.digest,
                confirmed: false,
                cancel: None,
            },
        );
        assert!(matches!(
            unconfirmed,
            Err(SoftwareLeftoverError::ConfirmationRequired)
        ));
        let stale = execute_with(
            &fixture.environment,
            &fixture.audit_path,
            &trash,
            no_location,
            SoftwareLeftoverExecutionRequest {
                plan: &plan,
                expected_preview_digest: &format!("sha256:{}", "9".repeat(64)),
                confirmed: true,
                cancel: None,
            },
        );
        assert!(matches!(stale, Err(SoftwareLeftoverError::DigestMismatch)));
        let forged = SoftwareLeftoverPlanV1 {
            uninstall_operation_id: "software-op-present".to_string(),
            ..plan.clone()
        };
        assert!(matches!(
            execute_with(
                &fixture.environment,
                &fixture.audit_path,
                &trash,
                no_location,
                SoftwareLeftoverExecutionRequest {
                    plan: &forged,
                    expected_preview_digest: &preview.digest,
                    confirmed: true,
                    cancel: None,
                },
            ),
            Err(SoftwareLeftoverError::UninstallNotSucceeded(_))
        ));
        assert!(trash.moved.lock().unwrap().is_empty());

        let report = execute_with(
            &fixture.environment,
            &fixture.audit_path,
            &trash,
            no_location,
            SoftwareLeftoverExecutionRequest {
                plan: &plan,
                expected_preview_digest: &preview.digest,
                confirmed: true,
                cancel: None,
            },
        )
        .unwrap();
        assert_eq!(*trash.moved.lock().unwrap(), vec![candidate.path.clone()]);
        assert_eq!(
            report.outcomes[0].outcome,
            SoftwareSupportOutcomeCode::Succeeded
        );
        assert_eq!(report.moved_known_bytes, 64);
        assert!(!report.lower_bound);

        let journal = fs::read_to_string(&fixture.audit_path).unwrap();
        let leftover = journal
            .lines()
            .find(|line| line.contains("leftover_moved"))
            .unwrap();
        assert!(leftover.contains("\"estimated_bytes\":64"));
        assert!(!leftover.contains("\"path\""));
        // The journal still reopens with mixed record families.
        SoftwareAuditJournal::open(&fixture.audit_path).unwrap();

        // The moved directory is gone, so the same plan is now stale.
        assert!(matches!(
            execute_with(
                &fixture.environment,
                &fixture.audit_path,
                &trash,
                no_location,
                SoftwareLeftoverExecutionRequest {
                    plan: &plan,
                    expected_preview_digest: &preview.digest,
                    confirmed: true,
                    cancel: None,
                },
            ),
            Err(SoftwareLeftoverError::StaleCandidate(_))
        ));
    }

    #[test]
    fn mismatched_software_id_and_unknown_candidate_are_rejected() {
        let fixture = fixture();
        let mut inventory = inventory(msix_identity(), "Contoso Tools", "Contoso");
        let genuine = inventory.entries[0].id.clone();
        inventory.entries[0].id = "software:v1:msix:forged".to_string();
        assert!(matches!(
            discover_with(
                &fixture.environment,
                &inventory,
                &["software:v1:msix:forged".to_string()],
                no_location,
                None,
            ),
            Err(SoftwareLeftoverError::InvalidIdentity(_))
        ));
        inventory.entries[0].id = genuine.clone();
        record_uninstall(&fixture.audit_path, "software-op-removed", true);
        let selection = SoftwareLeftoverSelectionV1 {
            software_id: genuine,
            uninstall_operation_id: "software-op-removed".to_string(),
            selected_candidate_ids: vec!["leftover:v1:unknown".to_string()],
        };
        assert!(matches!(
            plan_with(
                &fixture.environment,
                &fixture.audit_path,
                &inventory,
                &selection,
                no_location,
                None,
            ),
            Err(SoftwareLeftoverError::StaleCandidate(_))
        ));
    }
}
