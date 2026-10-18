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

use crate::digest_encoding::hex_digest;

use super::{
    SoftwareAuditError, SoftwareIdentity, SoftwareInventoryV1, SoftwareSizeBasis,
    SoftwareSizeEvidence, SoftwareSizeSourceCode, SoftwareSupportErrorCode,
    SoftwareSupportOutcomeCode,
    audit::{
        SoftwareAuditInspection, SoftwareAuditJournal, SoftwareSupportAuditRecordV1,
        SoftwareSupportJournal, SoftwareSupportLifecycleRecordV1, SoftwareSupportOperation,
        SoftwareSupportTerminalResult, SupportAppendFault,
    },
    software_audit_v1_path, software_id, unix_ms,
};
use crate::{
    execution::{SystemTrashRunner, TrashRunner, UserProtectionList},
    filesystem::{
        DEFAULT_SIZE_ENTRY_BUDGET, PathSafety, SizeEstimate, SystemPathReparseProbe,
        capture_path_identity, estimate_trees_with_budget_and_cancel, inspect_entry_no_follow,
        normalize_absolute_path, path_is_within,
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
    /// A directory under a personal-data root. It starts unselected.
    PersonalData,
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

/// Name and publisher matches are untrusted hints. A succeeded uninstall does
/// not raise certainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareLeftoverLabelTrust {
    UntrustedHint,
}

/// Strict leftover preview. `digest` binds the unchanged V1 digest and identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverStrictPreviewV1 {
    pub version: u32,
    pub items: Vec<SoftwareLeftoverCandidateV1>,
    pub v1_digest: String,
    pub digest: String,
    pub label_trust: SoftwareLeftoverLabelTrust,
}

/// Strict leftover result. `unknown` means the terminal audit did not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareLeftoverStrictOutcome {
    Succeeded,
    Failed,
    Unknown,
    Refused,
}

/// Strict leftover removal report. It does not rewrite `software.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareLeftoverStrictReportV1 {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    pub outcome: SoftwareLeftoverStrictOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub outcomes: Vec<SoftwareLeftoverOutcomeV1>,
    pub moved_known_bytes: u64,
    pub lower_bound: bool,
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
    CandidateChanged(String),
    IdentityUnavailable(String),
    AuditUnavailable,
    Canceled,
    ResidueChanged,
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
                "Software audit has no removed uninstall for operation {id}"
            ),
            Self::DigestMismatch => formatter.write_str("leftover preview digest mismatch"),
            Self::ProtectionUnavailable => {
                formatter.write_str("protection list is unavailable; leftovers are not reviewed")
            }
            Self::SerializationFailed => {
                formatter.write_str("leftover canonical serialization failed")
            }
            Self::CandidateChanged(id) => {
                write!(formatter, "leftover candidate {id} changed")
            }
            Self::IdentityUnavailable(id) => {
                write!(formatter, "leftover candidate {id} has no path identity")
            }
            Self::AuditUnavailable => formatter.write_str("leftover support audit is unavailable"),
            Self::Canceled => formatter.write_str("leftover removal was canceled before dispatch"),
            Self::ResidueChanged => {
                formatter.write_str("registry residue bytes changed since the backup")
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

impl SoftwareLeftoverError {
    /// Stable machine code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConfirmationRequired => "confirmation_required",
            Self::UnsupportedPlanVersion(_) => "unsupported_plan_version",
            Self::UnknownSelection(_) => "unknown_selection",
            Self::InvalidIdentity(_) => "invalid_identity",
            Self::EmptySelection => "empty_selection",
            Self::DuplicateSelection(_) => "duplicate_selection",
            Self::StaleCandidate(_) => "stale_candidate",
            Self::UninstallNotSucceeded(_) => "uninstall_not_succeeded",
            Self::DigestMismatch => "digest_mismatch",
            Self::ProtectionUnavailable => "protection_unavailable",
            Self::SerializationFailed => "serialization_failed",
            Self::CandidateChanged(_) => "candidate_changed",
            Self::IdentityUnavailable(_) => "identity_unavailable",
            Self::AuditUnavailable | Self::Audit(_) => "audit_unavailable",
            Self::Canceled => "canceled",
            Self::ResidueChanged => "residue_changed",
        }
    }
}

/// Candidate roots and exclusions for one run.
#[derive(Debug, Clone)]
struct LeftoverEnvironment {
    roaming: Option<PathBuf>,
    local: Option<PathBuf>,
    program_data: Option<PathBuf>,
    /// Documents, Desktop, Downloads, Pictures, Music, and Videos when present.
    personal: Vec<PathBuf>,
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
            personal: personal_roots(var("USERPROFILE")),
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
        .chain(
            self.personal
                .iter()
                .map(|root| (root.as_path(), SoftwareLeftoverOrigin::PersonalData)),
        )
        .collect()
    }

    fn personal_candidate(&self, path: &Path) -> bool {
        self.personal
            .iter()
            .any(|root| path != root.as_path() && path_is_within(root, path))
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
                .chain(self.personal.iter())
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
    if journal.uninstall_removed(operation_id, identity) {
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
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
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
        .map(|((key, path, origin, certainty), size)| {
            let origin = if environment.personal_candidate(&path) {
                SoftwareLeftoverOrigin::PersonalData
            } else {
                origin
            };
            SoftwareLeftoverCandidateV1 {
                id: candidate_id(subject.software_id, origin, &key),
                selected_by_default: origin != SoftwareLeftoverOrigin::PersonalData
                    && certainty == SoftwareLeftoverCertainty::Certain,
                size: size_evidence(&size, observed_at_unix_ms),
                path,
                origin,
                certainty,
            }
        })
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

fn personal_roots(profile: Option<PathBuf>) -> Vec<PathBuf> {
    let Some(profile) = profile else {
        return Vec::new();
    };
    [
        "Desktop",
        "Documents",
        "Downloads",
        "Music",
        "Pictures",
        "Videos",
    ]
    .into_iter()
    .map(|name| profile.join(name))
    .filter(|path| path.is_dir())
    .collect()
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
    let digest = hex_digest(Sha256::digest(format!(
        "devsweep.software.leftover.v1\0{software_id}\0{origin}\0{key}"
    )));
    format!("leftover:v1:{digest}")
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

/// Read-only strict preview. It does not create the audit directory, lock, or file.
pub fn preview_software_leftover_plan(
    inventory: &SoftwareInventoryV1,
    plan: &SoftwareLeftoverPlanV1,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<SoftwareLeftoverStrictPreviewV1, SoftwareLeftoverError> {
    preview_strict_with(
        &LeftoverEnvironment::production()?,
        &software_audit_v1_path()?,
        inventory,
        plan,
        install_location,
        cancel,
    )
}

/// Strict leftover removal. The public entry does not accept an audit path.
pub fn execute_software_leftover_plan(
    inventory: &SoftwareInventoryV1,
    plan: &SoftwareLeftoverPlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<SoftwareLeftoverStrictReportV1, SoftwareLeftoverError> {
    execute_strict_with(StrictLeftoverExecute {
        environment: &LeftoverEnvironment::production()?,
        audit_path: &software_audit_v1_path()?,
        trash: &SystemTrashRunner,
        locate: install_location,
        inventory,
        plan,
        expected_preview_digest,
        confirmed,
        cancel,
        fault: SupportAppendFault::NONE,
    })
}

struct StrictLeftoverExecute<'a> {
    environment: &'a LeftoverEnvironment,
    audit_path: &'a Path,
    trash: &'a dyn TrashRunner,
    locate: fn(&SoftwareIdentity) -> Option<PathBuf>,
    inventory: &'a SoftwareInventoryV1,
    plan: &'a SoftwareLeftoverPlanV1,
    expected_preview_digest: &'a str,
    confirmed: bool,
    cancel: Option<&'a Arc<FlagCancelObserver>>,
    fault: SupportAppendFault,
}

struct BoundLeftovers {
    items: Vec<SoftwareLeftoverCandidateV1>,
    v1_digest: String,
    identities: Vec<(String, String)>,
}

fn preview_strict_with(
    environment: &LeftoverEnvironment,
    audit_path: &Path,
    inventory: &SoftwareInventoryV1,
    plan: &SoftwareLeftoverPlanV1,
    locate: fn(&SoftwareIdentity) -> Option<PathBuf>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<SoftwareLeftoverStrictPreviewV1, SoftwareLeftoverError> {
    let bound = strict_candidates(environment, audit_path, inventory, plan, locate, cancel)?;
    Ok(SoftwareLeftoverStrictPreviewV1 {
        version: SOFTWARE_LEFTOVER_VERSION,
        digest: strict_digest(&bound.v1_digest, &bound.identities)?,
        v1_digest: bound.v1_digest,
        label_trust: SoftwareLeftoverLabelTrust::UntrustedHint,
        items: bound.items,
    })
}

fn execute_strict_with(
    request: StrictLeftoverExecute<'_>,
) -> Result<SoftwareLeftoverStrictReportV1, SoftwareLeftoverError> {
    let StrictLeftoverExecute {
        environment,
        audit_path,
        trash,
        locate,
        inventory,
        plan,
        expected_preview_digest,
        confirmed,
        cancel,
        fault,
    } = request;
    if !confirmed {
        return Err(SoftwareLeftoverError::ConfirmationRequired);
    }
    if cancel
        .as_ref()
        .is_some_and(|flag| flag.is_cancel_requested())
    {
        return Err(SoftwareLeftoverError::Canceled);
    }
    let bound = strict_candidates(environment, audit_path, inventory, plan, locate, cancel)?;
    let digest = strict_digest(&bound.v1_digest, &bound.identities)?;
    if digest != expected_preview_digest {
        if expected_preview_digest == bound.v1_digest {
            return Err(SoftwareLeftoverError::DigestMismatch);
        }
        return Err(SoftwareLeftoverError::CandidateChanged(
            bound
                .items
                .first()
                .map(|item| item.id.clone())
                .unwrap_or_default(),
        ));
    }
    let mut journal = SoftwareSupportJournal::open(audit_path).map_err(map_leftover_audit)?;
    if !journal.uninstall_removed(&plan.uninstall_operation_id, &plan.identity) {
        return Err(SoftwareLeftoverError::UninstallNotSucceeded(
            plan.uninstall_operation_id.clone(),
        ));
    }
    let rebound = bind_candidates(environment, plan, locate, cancel)?;
    if rebound.v1_digest != bound.v1_digest || rebound.identities != bound.identities {
        return Err(SoftwareLeftoverError::CandidateChanged(
            rebound
                .items
                .first()
                .map(|item| item.id.clone())
                .unwrap_or_default(),
        ));
    }
    let operation_id = next_operation_id();
    let plan_identity = leftover_plan_identity(plan)?;
    journal
        .append_lifecycle(
            SoftwareSupportLifecycleRecordV1::started(
                SoftwareSupportOperation::LeftoverMove,
                &operation_id,
                unix_ms(SystemTime::now()),
                &plan_identity,
                &digest,
            ),
            fault,
        )
        .map_err(|_| SoftwareLeftoverError::AuditUnavailable)?;
    let rechecked = match bind_candidates(environment, plan, locate, cancel) {
        Ok(value) => value,
        Err(error) => {
            return finish_leftover_refused(
                &mut journal,
                fault,
                &operation_id,
                &plan_identity,
                &digest,
                error,
            );
        }
    };
    let items = rechecked.items;
    if rechecked.v1_digest != bound.v1_digest || rechecked.identities != bound.identities {
        return finish_leftover_refused(
            &mut journal,
            fault,
            &operation_id,
            &plan_identity,
            &digest,
            SoftwareLeftoverError::CandidateChanged(
                items
                    .first()
                    .map(|item| item.id.clone())
                    .unwrap_or_default(),
            ),
        );
    }
    let mut outcomes = Vec::with_capacity(items.len());
    let mut moved_known_bytes = 0_u64;
    let mut lower_bound = false;
    let mut any_failed = false;
    for item in &items {
        let (outcome, estimated_bytes, error_code) =
            if let Some(code) = live_refusal(environment, &item.path) {
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
                    Err(_) => {
                        any_failed = true;
                        (
                            SoftwareSupportOutcomeCode::Failed,
                            None,
                            Some(SoftwareSupportErrorCode::TrashFailed),
                        )
                    }
                }
            };
        outcomes.push(SoftwareLeftoverOutcomeV1 {
            candidate_id: item.id.clone(),
            certainty: item.certainty,
            outcome,
            estimated_bytes,
            error_code,
        });
    }
    let (terminal, outcome, error_code) = if any_failed {
        (
            SoftwareSupportTerminalResult::Failed,
            SoftwareLeftoverStrictOutcome::Failed,
            Some("trash_failed"),
        )
    } else {
        (
            SoftwareSupportTerminalResult::Succeeded,
            SoftwareLeftoverStrictOutcome::Succeeded,
            None,
        )
    };
    if journal
        .append_lifecycle(
            SoftwareSupportLifecycleRecordV1::terminal(
                SoftwareSupportOperation::LeftoverMove,
                &operation_id,
                unix_ms(SystemTime::now()),
                &plan_identity,
                &digest,
                terminal,
                error_code,
            ),
            fault,
        )
        .is_err()
    {
        return Ok(SoftwareLeftoverStrictReportV1 {
            version: SOFTWARE_LEFTOVER_VERSION,
            operation_id: Some(operation_id),
            outcome: SoftwareLeftoverStrictOutcome::Unknown,
            error_code: Some("audit_unavailable".to_string()),
            outcomes,
            moved_known_bytes,
            lower_bound,
        });
    }
    Ok(SoftwareLeftoverStrictReportV1 {
        version: SOFTWARE_LEFTOVER_VERSION,
        operation_id: Some(operation_id),
        outcome,
        error_code: error_code.map(str::to_string),
        outcomes,
        moved_known_bytes,
        lower_bound,
    })
}

fn finish_leftover_refused(
    journal: &mut SoftwareSupportJournal,
    fault: SupportAppendFault,
    operation_id: &str,
    plan_identity: &str,
    digest: &str,
    error: SoftwareLeftoverError,
) -> Result<SoftwareLeftoverStrictReportV1, SoftwareLeftoverError> {
    let recorded = journal.append_lifecycle(
        SoftwareSupportLifecycleRecordV1::terminal(
            SoftwareSupportOperation::LeftoverMove,
            operation_id,
            unix_ms(SystemTime::now()),
            plan_identity,
            digest,
            SoftwareSupportTerminalResult::Refused,
            Some(error.code()),
        ),
        fault,
    );
    if recorded.is_err() {
        return Ok(SoftwareLeftoverStrictReportV1 {
            version: SOFTWARE_LEFTOVER_VERSION,
            operation_id: Some(operation_id.to_string()),
            outcome: SoftwareLeftoverStrictOutcome::Unknown,
            error_code: Some("audit_unavailable".to_string()),
            outcomes: Vec::new(),
            moved_known_bytes: 0,
            lower_bound: false,
        });
    }
    Err(error)
}

fn strict_candidates(
    environment: &LeftoverEnvironment,
    audit_path: &Path,
    inventory: &SoftwareInventoryV1,
    plan: &SoftwareLeftoverPlanV1,
    locate: fn(&SoftwareIdentity) -> Option<PathBuf>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<BoundLeftovers, SoftwareLeftoverError> {
    if plan.version != SOFTWARE_LEFTOVER_VERSION {
        return Err(SoftwareLeftoverError::UnsupportedPlanVersion(plan.version));
    }
    verify_software_id(&plan.software_id, &plan.identity)?;
    let entry = inventory
        .entries
        .iter()
        .find(|entry| entry.id == plan.software_id)
        .ok_or_else(|| SoftwareLeftoverError::UnknownSelection(plan.software_id.clone()))?;
    if entry.identity != plan.identity {
        return Err(SoftwareLeftoverError::InvalidIdentity(
            plan.software_id.clone(),
        ));
    }
    let removed = SoftwareAuditInspection::read(audit_path)
        .map_err(map_leftover_audit)?
        .uninstall_removed(&plan.uninstall_operation_id, &plan.identity);
    if !removed {
        return Err(SoftwareLeftoverError::UninstallNotSucceeded(
            plan.uninstall_operation_id.clone(),
        ));
    }
    bind_candidates(environment, plan, locate, cancel)
}

fn bind_candidates(
    environment: &LeftoverEnvironment,
    plan: &SoftwareLeftoverPlanV1,
    locate: fn(&SoftwareIdentity) -> Option<PathBuf>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<BoundLeftovers, SoftwareLeftoverError> {
    let candidates = discover_candidates(
        environment,
        &LeftoverSubject {
            software_id: &plan.software_id,
            display_name: plan.display_name.as_deref(),
            publisher: plan.publisher.as_deref(),
            install_location: locate(&plan.identity),
        },
        cancel,
    );
    let items = select_candidates(&candidates, &plan.selected_candidate_ids)?;
    let v1_digest = plan_digest(plan, &items)?;
    let mut identities = Vec::with_capacity(items.len());
    for item in &items {
        identities.push((
            item.id.clone(),
            capture_candidate_identity(&item.id, &item.path)?,
        ));
    }
    Ok(BoundLeftovers {
        items,
        v1_digest,
        identities,
    })
}

fn capture_candidate_identity(id: &str, path: &Path) -> Result<String, SoftwareLeftoverError> {
    let entry = inspect_entry_no_follow(path, &SystemPathReparseProbe)
        .map_err(|_| SoftwareLeftoverError::IdentityUnavailable(id.to_string()))?;
    if !entry.is_dir || entry.safety != PathSafety::Safe {
        return Err(SoftwareLeftoverError::IdentityUnavailable(id.to_string()));
    }
    let identity = capture_path_identity(path)
        .map_err(|_| SoftwareLeftoverError::IdentityUnavailable(id.to_string()))?;
    let kind = serde_json::to_string(&identity.kind)
        .map_err(|_| SoftwareLeftoverError::SerializationFailed)?;
    let mut hasher = Sha256::new();
    hasher.update(b"devsweep.software.leftover.identity.v1");
    hasher.update([0]);
    hasher.update(kind.as_bytes());
    hasher.update([0]);
    hasher.update(identity.value.as_bytes());
    Ok(hex_digest(hasher.finalize()))
}

fn strict_digest(
    v1_digest: &str,
    identities: &[(String, String)],
) -> Result<String, SoftwareLeftoverError> {
    let canonical = serde_json::to_vec(&(
        "devsweep.software.leftover.strict.v1",
        v1_digest,
        identities,
    ))
    .map_err(|_| SoftwareLeftoverError::SerializationFailed)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
}

fn leftover_plan_identity(plan: &SoftwareLeftoverPlanV1) -> Result<String, SoftwareLeftoverError> {
    let canonical = serde_json::to_vec(&("devsweep.software.leftover.plan-identity.v1", plan))
        .map_err(|_| SoftwareLeftoverError::SerializationFailed)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
}

/// File name of the one approved registry-residue fixture key.
pub const REGISTRY_RESIDUE_KEY_NAME: &str = "DevSweepResidue";

/// Registry-residue plan wire version.
pub const REGISTRY_RESIDUE_VERSION: u32 = 1;

const RESIDUE_POST_CHANGE: &[u8] = b"removed\n";

/// Saved residue plan. It names the closed key, not a vendor prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareRegistryResiduePlanV1 {
    pub version: u32,
    pub key_name: String,
    pub before_sha256: String,
}

/// Live preview of one residue plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareRegistryResiduePreviewV1 {
    pub version: u32,
    pub key_name: String,
    pub before_sha256: String,
    pub digest: String,
}

/// Result of one residue change. `backup_sha256` is the exact pre-change bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareRegistryResidueReportV1 {
    pub version: u32,
    pub key_name: String,
    pub backup_sha256: String,
    pub post_change_sha256: String,
}

/// Conditional restore of one residue key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareRegistryResidueRestorePlanV1 {
    pub version: u32,
    pub key_name: String,
    pub backup_sha256: String,
    pub post_change_sha256: String,
}

/// Live preview of one residue restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareRegistryResidueRestorePreviewV1 {
    pub version: u32,
    pub digest: String,
}

/// Reads one closed fixture key. It does not scan names or vendor prefixes.
pub fn plan_registry_residue(
    key_path: &Path,
) -> Result<SoftwareRegistryResiduePlanV1, SoftwareLeftoverError> {
    let bytes = read_closed_residue(key_path)?;
    Ok(SoftwareRegistryResiduePlanV1 {
        version: REGISTRY_RESIDUE_VERSION,
        key_name: REGISTRY_RESIDUE_KEY_NAME.to_string(),
        before_sha256: sha256_prefixed(&bytes),
    })
}

/// Re-reads the key and binds its current bytes to `plan`.
pub fn preview_registry_residue(
    key_path: &Path,
    plan: &SoftwareRegistryResiduePlanV1,
) -> Result<SoftwareRegistryResiduePreviewV1, SoftwareLeftoverError> {
    let bytes = read_closed_residue(key_path)?;
    validate_residue_plan(plan, &sha256_prefixed(&bytes))?;
    let digest = residue_digest(
        "devsweep.software.residue.preview.v1",
        &[&plan.key_name, &plan.before_sha256],
    )?;
    Ok(SoftwareRegistryResiduePreviewV1 {
        version: REGISTRY_RESIDUE_VERSION,
        key_name: plan.key_name.clone(),
        before_sha256: plan.before_sha256.clone(),
        digest,
    })
}

/// Writes the exact key bytes to `backup_path`, then changes the key.
pub fn execute_registry_residue(
    key_path: &Path,
    backup_path: &Path,
    plan: &SoftwareRegistryResiduePlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
) -> Result<SoftwareRegistryResidueReportV1, SoftwareLeftoverError> {
    if !confirmed {
        return Err(SoftwareLeftoverError::ConfirmationRequired);
    }
    if key_path == backup_path {
        return Err(SoftwareLeftoverError::InvalidIdentity(
            REGISTRY_RESIDUE_KEY_NAME.to_string(),
        ));
    }
    let preview = preview_registry_residue(key_path, plan)?;
    if preview.digest != expected_preview_digest {
        return Err(SoftwareLeftoverError::DigestMismatch);
    }
    let bytes = read_closed_residue(key_path)?;
    if sha256_prefixed(&bytes) != plan.before_sha256 {
        return Err(SoftwareLeftoverError::ResidueChanged);
    }
    fs::write(backup_path, &bytes).map_err(|_| SoftwareLeftoverError::AuditUnavailable)?;
    if fs::read(backup_path).ok().as_deref() != Some(bytes.as_slice()) {
        return Err(SoftwareLeftoverError::AuditUnavailable);
    }
    fs::write(key_path, RESIDUE_POST_CHANGE)
        .map_err(|_| SoftwareLeftoverError::AuditUnavailable)?;
    Ok(SoftwareRegistryResidueReportV1 {
        version: REGISTRY_RESIDUE_VERSION,
        key_name: REGISTRY_RESIDUE_KEY_NAME.to_string(),
        backup_sha256: sha256_prefixed(&bytes),
        post_change_sha256: sha256_prefixed(RESIDUE_POST_CHANGE),
    })
}

/// Copies the execute report into a restore plan. It does not write the key.
pub fn plan_registry_residue_restore(
    report: &SoftwareRegistryResidueReportV1,
) -> Result<SoftwareRegistryResidueRestorePlanV1, SoftwareLeftoverError> {
    if report.version != REGISTRY_RESIDUE_VERSION
        || report.key_name != REGISTRY_RESIDUE_KEY_NAME
        || !super::audit::is_sha256_digest(&report.backup_sha256)
        || !super::audit::is_sha256_digest(&report.post_change_sha256)
    {
        return Err(SoftwareLeftoverError::InvalidIdentity(
            report.key_name.clone(),
        ));
    }
    Ok(SoftwareRegistryResidueRestorePlanV1 {
        version: REGISTRY_RESIDUE_VERSION,
        key_name: report.key_name.clone(),
        backup_sha256: report.backup_sha256.clone(),
        post_change_sha256: report.post_change_sha256.clone(),
    })
}

/// Re-reads the key and the backup. A byte change is a refusal, not a write.
pub fn preview_registry_residue_restore(
    key_path: &Path,
    backup_path: &Path,
    plan: &SoftwareRegistryResidueRestorePlanV1,
) -> Result<SoftwareRegistryResidueRestorePreviewV1, SoftwareLeftoverError> {
    bind_residue_restore(key_path, backup_path, plan)?;
    Ok(SoftwareRegistryResidueRestorePreviewV1 {
        version: REGISTRY_RESIDUE_VERSION,
        digest: residue_digest(
            "devsweep.software.residue.restore.preview.v1",
            &[
                &plan.key_name,
                &plan.backup_sha256,
                &plan.post_change_sha256,
            ],
        )?,
    })
}

/// Restores the backup only when the live key still matches the post-change bytes.
pub fn execute_registry_residue_restore(
    key_path: &Path,
    backup_path: &Path,
    plan: &SoftwareRegistryResidueRestorePlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
) -> Result<(), SoftwareLeftoverError> {
    if !confirmed {
        return Err(SoftwareLeftoverError::ConfirmationRequired);
    }
    let preview = preview_registry_residue_restore(key_path, backup_path, plan)?;
    if preview.digest != expected_preview_digest {
        return Err(SoftwareLeftoverError::DigestMismatch);
    }
    let backup = bind_residue_restore(key_path, backup_path, plan)?;
    fs::write(key_path, backup).map_err(|_| SoftwareLeftoverError::AuditUnavailable)
}

fn bind_residue_restore(
    key_path: &Path,
    backup_path: &Path,
    plan: &SoftwareRegistryResidueRestorePlanV1,
) -> Result<Vec<u8>, SoftwareLeftoverError> {
    if plan.version != REGISTRY_RESIDUE_VERSION || plan.key_name != REGISTRY_RESIDUE_KEY_NAME {
        return Err(SoftwareLeftoverError::InvalidIdentity(
            plan.key_name.clone(),
        ));
    }
    let live = read_closed_residue(key_path)?;
    if sha256_prefixed(&live) != plan.post_change_sha256 {
        return Err(SoftwareLeftoverError::ResidueChanged);
    }
    let backup = fs::read(backup_path)
        .map_err(|_| SoftwareLeftoverError::StaleCandidate(plan.key_name.clone()))?;
    if sha256_prefixed(&backup) != plan.backup_sha256 {
        return Err(SoftwareLeftoverError::ResidueChanged);
    }
    Ok(backup)
}

fn read_closed_residue(key_path: &Path) -> Result<Vec<u8>, SoftwareLeftoverError> {
    let name = key_path.file_name().and_then(|value| value.to_str());
    if name != Some(REGISTRY_RESIDUE_KEY_NAME) {
        return Err(SoftwareLeftoverError::InvalidIdentity(
            REGISTRY_RESIDUE_KEY_NAME.to_string(),
        ));
    }
    fs::read(key_path)
        .map_err(|_| SoftwareLeftoverError::StaleCandidate(REGISTRY_RESIDUE_KEY_NAME.to_string()))
}

fn validate_residue_plan(
    plan: &SoftwareRegistryResiduePlanV1,
    before_sha256: &str,
) -> Result<(), SoftwareLeftoverError> {
    if plan.version != REGISTRY_RESIDUE_VERSION
        || plan.key_name != REGISTRY_RESIDUE_KEY_NAME
        || plan.before_sha256 != before_sha256
        || !super::audit::is_sha256_digest(&plan.before_sha256)
    {
        return Err(SoftwareLeftoverError::ResidueChanged);
    }
    Ok(())
}

fn residue_digest(label: &str, parts: &[&str]) -> Result<String, SoftwareLeftoverError> {
    let canonical = serde_json::to_vec(&(label, parts))
        .map_err(|_| SoftwareLeftoverError::SerializationFailed)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    format!("sha256:{}", hex_digest(Sha256::digest(bytes)))
}

fn map_leftover_audit(error: SoftwareAuditError) -> SoftwareLeftoverError {
    match error {
        SoftwareAuditError::LocalAppDataUnavailable => SoftwareLeftoverError::Audit(error),
        _ => SoftwareLeftoverError::AuditUnavailable,
    }
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
                personal: Vec::new(),
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

    fn name_plan(
        fixture: &Fixture,
    ) -> (
        SoftwareInventoryV1,
        SoftwareLeftoverPlanV1,
        SoftwareLeftoverCandidateV1,
    ) {
        let local = fixture.environment.local.clone().unwrap();
        write(&local.join("Contoso Tools").join("cache.bin"), 8);
        let stored = inventory(msix_identity(), "Contoso Tools", "Contoso");
        record_uninstall(&fixture.audit_path, "software-op-removed", true);
        let selection = SoftwareLeftoverSelectionV1 {
            software_id: stored.entries[0].id.clone(),
            uninstall_operation_id: "software-op-removed".to_string(),
            selected_candidate_ids: Vec::new(),
        };
        let discovered = discover_with(
            &fixture.environment,
            &stored,
            &[stored.entries[0].id.clone()],
            no_location,
            None,
        )
        .unwrap();
        let candidate = discovered.apps[0].candidates[0].clone();
        assert_eq!(candidate.certainty, SoftwareLeftoverCertainty::Uncertain);
        assert!(!candidate.selected_by_default);
        let mut selection = selection;
        selection.selected_candidate_ids = vec![candidate.id.clone()];
        let (plan, _) = plan_with(
            &fixture.environment,
            &fixture.audit_path,
            &stored,
            &selection,
            no_location,
            None,
        )
        .unwrap();
        (stored, plan, candidate)
    }

    #[test]
    fn strict_preview_does_not_create_an_audit_store() {
        let fixture = fixture();
        let missing = fixture.root.join("absent").join("software.jsonl");
        let stored = inventory(msix_identity(), "Contoso Tools", "Contoso");
        let plan = SoftwareLeftoverPlanV1 {
            version: SOFTWARE_LEFTOVER_VERSION,
            software_id: stored.entries[0].id.clone(),
            identity: msix_identity(),
            display_name: Some("Contoso Tools".to_string()),
            publisher: Some("Contoso".to_string()),
            uninstall_operation_id: "software-op-removed".to_string(),
            selected_candidate_ids: vec!["leftover:v1:missing".to_string()],
        };
        let error = preview_strict_with(
            &fixture.environment,
            &missing,
            &stored,
            &plan,
            no_location,
            None,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            SoftwareLeftoverError::UninstallNotSucceeded(_)
        ));
        assert!(!fixture.root.join("absent").exists());

        let (stored, plan, _) = name_plan(&fixture);
        let lock = fixture.audit_path.parent().unwrap().join("software.lock");
        assert!(lock.exists());
        fs::remove_file(&lock).unwrap();
        let preview = preview_strict_with(
            &fixture.environment,
            &fixture.audit_path,
            &stored,
            &plan,
            no_location,
            None,
        )
        .unwrap();
        assert_eq!(
            preview.label_trust,
            SoftwareLeftoverLabelTrust::UntrustedHint
        );
        assert!(
            preview
                .items
                .iter()
                .all(|item| item.certainty == SoftwareLeftoverCertainty::Uncertain)
        );
        assert!(!lock.exists());
        assert!(
            !fixture
                .audit_path
                .parent()
                .unwrap()
                .join("software-support.jsonl")
                .exists()
        );
    }

    // The fixture proves replacement by unlinking and recreating the reviewed
    // directory, which only yields a new object identity on Windows. On Unix the
    // identity is `dev:ino`, and ext4 reuses the freed inode, so the recreated
    // directory is indistinguishable from the reviewed one.
    #[cfg(windows)]
    #[test]
    fn strict_execute_refuses_a_replaced_directory_and_keeps_v1_digest() {
        let fixture = fixture();
        let (stored, plan, candidate) = name_plan(&fixture);
        let preview = preview_strict_with(
            &fixture.environment,
            &fixture.audit_path,
            &stored,
            &plan,
            no_location,
            None,
        )
        .unwrap();
        let (v1_plan, v1_preview) = plan_with(
            &fixture.environment,
            &fixture.audit_path,
            &stored,
            &SoftwareLeftoverSelectionV1 {
                software_id: plan.software_id.clone(),
                uninstall_operation_id: plan.uninstall_operation_id.clone(),
                selected_candidate_ids: plan.selected_candidate_ids.clone(),
            },
            no_location,
            None,
        )
        .unwrap();
        assert_eq!(preview.v1_digest, v1_preview.digest);
        assert_eq!(v1_plan.selected_candidate_ids, plan.selected_candidate_ids);
        fs::remove_dir_all(&candidate.path).unwrap();
        fs::create_dir_all(&candidate.path).unwrap();
        let trash = RecordingTrash::default();
        let error = execute_strict_with(StrictLeftoverExecute {
            environment: &fixture.environment,
            audit_path: &fixture.audit_path,
            trash: &trash,
            locate: no_location,
            inventory: &stored,
            plan: &plan,
            expected_preview_digest: &preview.digest,
            confirmed: true,
            cancel: None,
            fault: SupportAppendFault::NONE,
        })
        .unwrap_err();
        assert!(
            matches!(error, SoftwareLeftoverError::CandidateChanged(_)),
            "{error}"
        );
        assert!(trash.moved.lock().unwrap().is_empty());
        assert!(candidate.path.exists());
        let journal = fs::read_to_string(&fixture.audit_path).unwrap();
        assert!(!journal.contains("leftover_moved"));
    }

    #[test]
    fn strict_execute_moves_once_and_records_support_lifecycle() {
        let fixture = fixture();
        let (stored, plan, candidate) = name_plan(&fixture);
        let preview = preview_strict_with(
            &fixture.environment,
            &fixture.audit_path,
            &stored,
            &plan,
            no_location,
            None,
        )
        .unwrap();
        let trash = RecordingTrash::default();
        let fault = SupportAppendFault {
            fail_started: true,
            fail_terminal: false,
        };
        let error = execute_strict_with(StrictLeftoverExecute {
            environment: &fixture.environment,
            audit_path: &fixture.audit_path,
            trash: &trash,
            locate: no_location,
            inventory: &stored,
            plan: &plan,
            expected_preview_digest: &preview.digest,
            confirmed: true,
            cancel: None,
            fault,
        })
        .unwrap_err();
        assert!(matches!(error, SoftwareLeftoverError::AuditUnavailable));
        assert!(trash.moved.lock().unwrap().is_empty());
        assert!(candidate.path.exists());

        let report = execute_strict_with(StrictLeftoverExecute {
            environment: &fixture.environment,
            audit_path: &fixture.audit_path,
            trash: &trash,
            locate: no_location,
            inventory: &stored,
            plan: &plan,
            expected_preview_digest: &preview.digest,
            confirmed: true,
            cancel: None,
            fault: SupportAppendFault::NONE,
        })
        .unwrap();
        assert_eq!(report.outcome, SoftwareLeftoverStrictOutcome::Succeeded);
        assert_eq!(trash.moved.lock().unwrap().len(), 1);
        let support = fs::read_to_string(
            fixture
                .audit_path
                .parent()
                .unwrap()
                .join("software-support.jsonl"),
        )
        .unwrap();
        assert!(
            support.contains("\"phase\":\"started\"") || support.contains("\"phase\": \"started\"")
        );
        assert!(support.contains("leftover_move"));
        assert!(!support.contains(&candidate.path.display().to_string()));
        let v1 = fs::read_to_string(&fixture.audit_path).unwrap();
        assert!(!v1.contains("leftover_moved"));

        let terminal_fault = SupportAppendFault {
            fail_started: false,
            fail_terminal: true,
        };
        let again = fixture.root.join("Local").join("Contoso Tools");
        fs::create_dir_all(&again).unwrap();
        let second_digest = preview_strict_with(
            &fixture.environment,
            &fixture.audit_path,
            &stored,
            &plan,
            no_location,
            None,
        )
        .unwrap()
        .digest;
        let second = execute_strict_with(StrictLeftoverExecute {
            environment: &fixture.environment,
            audit_path: &fixture.audit_path,
            trash: &trash,
            locate: no_location,
            inventory: &stored,
            plan: &plan,
            expected_preview_digest: &second_digest,
            confirmed: true,
            cancel: None,
            fault: terminal_fault,
        })
        .unwrap();
        assert_eq!(second.outcome, SoftwareLeftoverStrictOutcome::Unknown);
        assert_eq!(trash.moved.lock().unwrap().len(), 2);
    }

    fn record_outcome(path: &Path, operation_id: &str, outcome: SoftwareExecutionOutcome) {
        let identity = msix_identity();
        let mut journal = SoftwareAuditJournal::open(path).unwrap();
        let append =
            |journal: &mut SoftwareAuditJournal, transition, status, reboot, installed, adapter| {
                journal
                    .append(SoftwareAuditRecordV1::new(
                        operation_id,
                        1,
                        &identity,
                        &format!("sha256:{}", "1".repeat(64)),
                        &format!("sha256:{}", "2".repeat(64)),
                        SoftwareAuditEvent {
                            transition,
                            status_code: status,
                            error_code: None,
                            reboot_evidence: reboot,
                            installed_state: installed,
                            adapter_outcome: adapter,
                        },
                    ))
                    .unwrap();
            };
        append(
            &mut journal,
            SoftwareAuditTransition::Validated,
            SoftwareAuditStatusCode::Validated,
            SoftwareRebootEvidence::None,
            None,
            None,
        );
        if outcome == SoftwareExecutionOutcome::CanceledBeforeStart {
            append(
                &mut journal,
                SoftwareAuditTransition::Terminal { outcome },
                SoftwareAuditStatusCode::CanceledBeforeStart,
                SoftwareRebootEvidence::None,
                None,
                None,
            );
            return;
        }
        append(
            &mut journal,
            SoftwareAuditTransition::DispatchStarted,
            SoftwareAuditStatusCode::DispatchStarted,
            SoftwareRebootEvidence::None,
            None,
            None,
        );
        match outcome {
            SoftwareExecutionOutcome::Failed => {
                append(
                    &mut journal,
                    SoftwareAuditTransition::AdapterCompleted,
                    SoftwareAuditStatusCode::AdapterFailed,
                    SoftwareRebootEvidence::None,
                    None,
                    Some(AdapterOutcome::Failure),
                );
                append(
                    &mut journal,
                    SoftwareAuditTransition::RequeryObserved,
                    SoftwareAuditStatusCode::RequeryPresent,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    None,
                );
                append(
                    &mut journal,
                    SoftwareAuditTransition::Terminal { outcome },
                    SoftwareAuditStatusCode::Failed,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    Some(AdapterOutcome::Failure),
                );
            }
            SoftwareExecutionOutcome::StillPresent => {
                append(
                    &mut journal,
                    SoftwareAuditTransition::AdapterCompleted,
                    SoftwareAuditStatusCode::AdapterSucceeded,
                    SoftwareRebootEvidence::None,
                    None,
                    Some(AdapterOutcome::Success),
                );
                append(
                    &mut journal,
                    SoftwareAuditTransition::RequeryObserved,
                    SoftwareAuditStatusCode::RequeryPresent,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    None,
                );
                append(
                    &mut journal,
                    SoftwareAuditTransition::Terminal { outcome },
                    SoftwareAuditStatusCode::StillPresent,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    Some(AdapterOutcome::Success),
                );
            }
            SoftwareExecutionOutcome::RebootRequired => {
                append(
                    &mut journal,
                    SoftwareAuditTransition::AdapterCompleted,
                    SoftwareAuditStatusCode::AdapterRebootRequired,
                    SoftwareRebootEvidence::Required,
                    None,
                    Some(AdapterOutcome::RebootRequired),
                );
                append(
                    &mut journal,
                    SoftwareAuditTransition::RequeryObserved,
                    SoftwareAuditStatusCode::RequeryAbsent,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Absent),
                    None,
                );
                append(
                    &mut journal,
                    SoftwareAuditTransition::Terminal { outcome },
                    SoftwareAuditStatusCode::RebootRequired,
                    SoftwareRebootEvidence::Required,
                    Some(SoftwareInstalledState::Absent),
                    Some(AdapterOutcome::RebootRequired),
                );
            }
            SoftwareExecutionOutcome::UnknownAfterDispatch => {
                append(
                    &mut journal,
                    SoftwareAuditTransition::RequeryObserved,
                    SoftwareAuditStatusCode::RequeryPresent,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    None,
                );
                append(
                    &mut journal,
                    SoftwareAuditTransition::Terminal { outcome },
                    SoftwareAuditStatusCode::UnknownAfterDispatch,
                    SoftwareRebootEvidence::None,
                    Some(SoftwareInstalledState::Present),
                    Some(AdapterOutcome::Unfinished),
                );
            }
            SoftwareExecutionOutcome::Removed | SoftwareExecutionOutcome::CanceledBeforeStart => {}
        }
    }

    #[test]
    fn ac3_non_removed_uninstall_does_not_execute_leftovers() {
        for (operation_id, outcome) in [
            ("software-op-failed", SoftwareExecutionOutcome::Failed),
            (
                "software-op-cancel",
                SoftwareExecutionOutcome::CanceledBeforeStart,
            ),
            ("software-op-still", SoftwareExecutionOutcome::StillPresent),
            (
                "software-op-reboot",
                SoftwareExecutionOutcome::RebootRequired,
            ),
            (
                "software-op-unknown",
                SoftwareExecutionOutcome::UnknownAfterDispatch,
            ),
        ] {
            let fixture = fixture();
            let local = fixture.environment.local.clone().unwrap();
            let dir = local.join("Contoso Tools");
            write(&dir.join("keep.bin"), 4);
            let marker = fs::read(dir.join("keep.bin")).unwrap();
            record_outcome(&fixture.audit_path, operation_id, outcome);
            let stored = inventory(msix_identity(), "Contoso Tools", "Contoso");
            let plan = SoftwareLeftoverPlanV1 {
                version: SOFTWARE_LEFTOVER_VERSION,
                software_id: stored.entries[0].id.clone(),
                identity: stored.entries[0].identity.clone(),
                display_name: stored.entries[0].display_name.clone(),
                publisher: stored.entries[0].publisher.clone(),
                uninstall_operation_id: operation_id.to_string(),
                selected_candidate_ids: vec!["leftover:v1:unused".to_string()],
            };
            let trash = RecordingTrash::default();
            let error = execute_strict_with(StrictLeftoverExecute {
                environment: &fixture.environment,
                audit_path: &fixture.audit_path,
                trash: &trash,
                locate: no_location,
                inventory: &stored,
                plan: &plan,
                expected_preview_digest: &format!("sha256:{}", "9".repeat(64)),
                confirmed: true,
                cancel: None,
                fault: SupportAppendFault::NONE,
            })
            .unwrap_err();
            assert!(matches!(
                error,
                SoftwareLeftoverError::UninstallNotSucceeded(_)
            ));
            assert!(trash.moved.lock().unwrap().is_empty());
            assert_eq!(fs::read(dir.join("keep.bin")).unwrap(), marker);
        }
    }

    #[test]
    fn ac3_personal_rows_start_unselected_and_shared_folder_is_protected() {
        let mut fixture = fixture();
        let documents = fixture.root.join("Documents");
        let personal = documents.join("ContosoTools");
        let named = documents.join("Contoso Tools");
        let shared = documents.join("Programs");
        write(&personal.join("note.txt"), 3);
        write(&named.join("a.bin"), 3);
        write(&shared.join("secret.bin"), 3);
        fixture.environment.personal = vec![documents];
        let candidates = discover_candidates(
            &fixture.environment,
            &LeftoverSubject {
                software_id: "software:v1:msix:fixture",
                display_name: Some("Contoso Tools"),
                publisher: None,
                install_location: Some(personal.clone()),
            },
            None,
        );
        let install = candidates
            .iter()
            .find(|candidate| candidate.path == personal)
            .expect("personal install location");
        assert_eq!(install.origin, SoftwareLeftoverOrigin::PersonalData);
        assert_eq!(install.certainty, SoftwareLeftoverCertainty::Certain);
        assert!(!install.selected_by_default);
        let name_match = candidates
            .iter()
            .find(|candidate| candidate.path == named)
            .expect("personal name match");
        assert_eq!(name_match.origin, SoftwareLeftoverOrigin::PersonalData);
        assert!(!name_match.selected_by_default);
        assert!(candidates.iter().all(|candidate| candidate.path != shared));
    }

    #[test]
    fn ac3_registry_residue_backup_and_conditional_restore() {
        let temp = TempDir::new().unwrap();
        let key = temp.path().join(REGISTRY_RESIDUE_KEY_NAME);
        let original = b"before\n".to_vec();
        fs::write(&key, &original).unwrap();
        let other = temp.path().join("VendorPrefix");
        fs::write(&other, b"vendor").unwrap();
        assert!(matches!(
            plan_registry_residue(&other),
            Err(SoftwareLeftoverError::InvalidIdentity(_))
        ));
        assert_eq!(fs::read(&other).unwrap(), b"vendor");
        let backup = temp.path().join("backup.bin");
        let plan = plan_registry_residue(&key).unwrap();
        assert_eq!(plan.key_name, REGISTRY_RESIDUE_KEY_NAME);
        let preview = preview_registry_residue(&key, &plan).unwrap();
        assert!(matches!(
            execute_registry_residue(&key, &key, &plan, &preview.digest, true),
            Err(SoftwareLeftoverError::InvalidIdentity(_))
        ));
        assert_eq!(fs::read(&key).unwrap(), original);
        let report = execute_registry_residue(&key, &backup, &plan, &preview.digest, true).unwrap();
        assert_eq!(fs::read(&backup).unwrap(), original);
        assert_eq!(fs::read(&key).unwrap(), b"removed\n");
        assert_eq!(report.backup_sha256, plan.before_sha256);
        let restore = plan_registry_residue_restore(&report).unwrap();
        let restore_preview = preview_registry_residue_restore(&key, &backup, &restore).unwrap();
        execute_registry_residue_restore(&key, &backup, &restore, &restore_preview.digest, true)
            .unwrap();
        assert_eq!(fs::read(&key).unwrap(), original);
        let external = b"external\n".to_vec();
        fs::write(&key, &external).unwrap();
        let error = execute_registry_residue_restore(
            &key,
            &backup,
            &restore,
            &restore_preview.digest,
            true,
        )
        .unwrap_err();
        assert!(matches!(error, SoftwareLeftoverError::ResidueChanged));
        assert_eq!(fs::read(&key).unwrap(), external);
        assert_eq!(fs::read(&backup).unwrap(), original);
    }
}
