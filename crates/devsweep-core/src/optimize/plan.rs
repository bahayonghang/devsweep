//! Untrusted Optimize V1 plan, live preflight, and opaque preview digest.
//!
//! The saved plan contributes only its catalogue version and exactly one
//! operation id. Every executable fact — program, argv, URI, build floor — is
//! reconstructed from the closed catalogue behind a live preflight and sealed
//! into an opaque, non-deserializable validated action.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::LivePreflight;
use super::catalogue::{MaintenanceActionClass, OPTIMIZE_CATALOGUE_VERSION};

/// Saved Optimize plan schema generation.
pub const OPTIMIZE_PLAN_VERSION: u32 = 1;
/// Live Optimize preview schema generation.
pub const OPTIMIZE_PREVIEW_VERSION: u32 = 1;

/// Caller-correctable Optimize plan and preflight failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizePlanError {
    /// The saved plan is not Optimize V1.
    UnsupportedPlanVersion(u32),
    /// The saved plan references a different catalogue generation.
    UnsupportedCatalogueVersion(u32),
    /// The id does not exist in the closed catalogue.
    UnknownOperation(String),
    /// The id is a guidance row and can never be planned or dispatched.
    GuidanceNotExecutable(String),
    /// The confirmed digest does not match the live preview digest.
    DigestMismatch,
    /// Canonical digest serialization failed.
    SerializationFailed,
    /// The live platform is not Windows.
    PlatformUnsupported,
    /// The manifest-independent OS build query failed.
    OsBuildUnavailable,
    /// The live OS build is below the catalogue floor for the id.
    BuildUnsupported { build: u32, floor: u32 },
    /// The native System32 `ipconfig.exe` identity could not be resolved.
    ResolverUnavailable,
}

impl std::fmt::Display for OptimizePlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlanVersion(version) => {
                write!(formatter, "unsupported optimize plan version {version}")
            }
            Self::UnsupportedCatalogueVersion(version) => {
                write!(
                    formatter,
                    "unsupported optimize catalogue version {version}"
                )
            }
            Self::UnknownOperation(id) => write!(formatter, "unknown optimize operation {id}"),
            Self::GuidanceNotExecutable(id) => {
                write!(formatter, "guidance entry {id} is never dispatched")
            }
            Self::DigestMismatch => formatter.write_str("optimize preview digest mismatch"),
            Self::SerializationFailed => {
                formatter.write_str("optimize canonical serialization failed")
            }
            Self::PlatformUnsupported => {
                formatter.write_str("optimize maintenance requires Windows")
            }
            Self::OsBuildUnavailable => {
                formatter.write_str("the Windows OS build query is unavailable")
            }
            Self::BuildUnsupported { build, floor } => {
                write!(
                    formatter,
                    "Windows build {build} is below the required build {floor}"
                )
            }
            Self::ResolverUnavailable => {
                formatter.write_str("the native System32 ipconfig.exe could not be resolved")
            }
        }
    }
}

impl std::error::Error for OptimizePlanError {}

/// A saved Optimize plan. It carries no executable fact: only the catalogue
/// generation and exactly one catalogue id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenancePlanV1 {
    /// Plan schema version.
    pub version: u32,
    /// Catalogue generation the id was selected from.
    pub catalogue_version: u32,
    /// Exactly one closed catalogue id.
    pub operation_id: String,
}

/// A live preview of one resolved maintenance operation. It never contains the
/// resolved program, argv, or URI; those are bound only by the digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenancePreviewV1 {
    /// Preview schema version.
    pub version: u32,
    /// Catalogue generation the id was resolved against.
    pub catalogue_version: u32,
    /// Exactly one closed catalogue id.
    pub operation_id: String,
    /// Closed action class of the resolved row.
    pub action_class: MaintenanceActionClass,
    /// Opaque digest over catalogue version, id, action class, literal
    /// URI/program/argv, and the resolved capability.
    pub digest: String,
}

/// Opaque execution authority resolved from the live preflight. It has no
/// `Serialize` or `Deserialize` implementation, all fields are private, and it
/// cannot be constructed outside this module.
///
/// ```compile_fail
/// use devsweep_core::optimize::{MaintenancePlanV1, ValidatedMaintenanceAction};
/// let _ = ValidatedMaintenanceAction {
///     catalogue_version: 1,
///     operation_id: "forged".into(),
///     action_class: devsweep_core::optimize::MaintenanceActionClass::Execute,
///     resolved: (),
///     preview_digest: "sha256:forged".into(),
/// };
/// ```
///
/// Execution requests cannot inject a caller-constructed validated action:
///
/// ```compile_fail
/// use devsweep_core::optimize::MaintenanceExecutionRequest;
/// let _ = MaintenanceExecutionRequest {
///     plan: todo!(),
///     validated: todo!(),
///     expected_preview_digest: "sha256:forged",
///     confirmed: true,
///     cancel: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedMaintenanceAction {
    pub(super) catalogue_version: u32,
    pub(super) operation_id: String,
    pub(super) action_class: MaintenanceActionClass,
    pub(super) resolved: ResolvedMaintenanceAction,
    pub(super) preview_digest: String,
}

/// The one resolved dispatch identity per live preflight. Guidance rows have no
/// variant, so they can never enter dispatch by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedMaintenanceAction {
    /// The exact native System32 `ipconfig.exe` identity. The argv is always
    /// the fixed `["/flushdns"]` and never stored per plan.
    DnsFlush { program: PathBuf },
    /// One of the three literal Settings URIs plus the observed OS build.
    SettingsHandoff {
        uri: &'static str,
        observed_build: u32,
    },
}

/// Creates an untrusted exact-id plan from the closed catalogue. Guidance rows
/// and unknown ids fail closed before any file is written.
pub fn plan_operation(operation_id: &str) -> Result<MaintenancePlanV1, OptimizePlanError> {
    let entry = super::catalogue::catalogue_entry(operation_id)
        .ok_or_else(|| OptimizePlanError::UnknownOperation(operation_id.to_string()))?;
    if entry.action_class == MaintenanceActionClass::Guidance {
        return Err(OptimizePlanError::GuidanceNotExecutable(
            operation_id.to_string(),
        ));
    }
    Ok(MaintenancePlanV1 {
        version: OPTIMIZE_PLAN_VERSION,
        catalogue_version: OPTIMIZE_CATALOGUE_VERSION,
        operation_id: entry.id.to_string(),
    })
}

/// Revalidates a saved plan against the closed catalogue and a live preflight,
/// resolves one non-deserializable action variant, and seals the preview digest.
pub(super) fn preview_maintenance_plan(
    plan: &MaintenancePlanV1,
    preflight: &dyn LivePreflight,
) -> Result<MaintenancePreviewV1, OptimizePlanError> {
    let action = construct_validated_action(plan, preflight)?;
    Ok(MaintenancePreviewV1 {
        version: OPTIMIZE_PREVIEW_VERSION,
        catalogue_version: action.catalogue_version,
        operation_id: action.operation_id.clone(),
        action_class: action.action_class,
        digest: action.preview_digest.clone(),
    })
}

/// Recomputes and checks an expected preview digest before any dispatch.
pub fn validate_preview_digest(
    preview: &MaintenancePreviewV1,
    expected_digest: &str,
) -> Result<(), OptimizePlanError> {
    if preview.version != OPTIMIZE_PREVIEW_VERSION || !is_sha256(&preview.digest) {
        return Err(OptimizePlanError::DigestMismatch);
    }
    if preview.digest != expected_digest {
        return Err(OptimizePlanError::DigestMismatch);
    }
    Ok(())
}

/// Resolves the validated action for one saved plan behind the live preflight.
pub(super) fn construct_validated_action(
    plan: &MaintenancePlanV1,
    preflight: &dyn LivePreflight,
) -> Result<ValidatedMaintenanceAction, OptimizePlanError> {
    if plan.version != OPTIMIZE_PLAN_VERSION {
        return Err(OptimizePlanError::UnsupportedPlanVersion(plan.version));
    }
    if plan.catalogue_version != OPTIMIZE_CATALOGUE_VERSION {
        return Err(OptimizePlanError::UnsupportedCatalogueVersion(
            plan.catalogue_version,
        ));
    }
    let entry = super::catalogue::catalogue_entry(&plan.operation_id)
        .ok_or_else(|| OptimizePlanError::UnknownOperation(plan.operation_id.clone()))?;
    if entry.action_class == MaintenanceActionClass::Guidance {
        return Err(OptimizePlanError::GuidanceNotExecutable(
            plan.operation_id.clone(),
        ));
    }
    let resolved = preflight.resolve(entry)?;
    let preview_digest = preview_digest(
        OPTIMIZE_CATALOGUE_VERSION,
        entry.id,
        entry.action_class,
        &resolved,
    )?;
    Ok(ValidatedMaintenanceAction {
        catalogue_version: OPTIMIZE_CATALOGUE_VERSION,
        operation_id: entry.id.to_string(),
        action_class: entry.action_class,
        resolved,
        preview_digest,
    })
}

/// The literal identity facts bound by the preview digest.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ResolvedIdentityLiterals<'a> {
    DnsFlush {
        program: &'a str,
        argv: [&'static str; 1],
    },
    SettingsHandoff {
        uri: &'static str,
        observed_build: u32,
    },
}

fn preview_digest(
    catalogue_version: u32,
    operation_id: &str,
    action_class: MaintenanceActionClass,
    resolved: &ResolvedMaintenanceAction,
) -> Result<String, OptimizePlanError> {
    let literals = match resolved {
        ResolvedMaintenanceAction::DnsFlush { program } => {
            let program = program
                .to_str()
                .ok_or(OptimizePlanError::SerializationFailed)?;
            ResolvedIdentityLiterals::DnsFlush {
                program,
                argv: [super::windows::DNS_FLUSH_ARG],
            }
        }
        ResolvedMaintenanceAction::SettingsHandoff {
            uri,
            observed_build,
        } => ResolvedIdentityLiterals::SettingsHandoff {
            uri,
            observed_build: *observed_build,
        },
    };
    let canonical = serde_json::to_vec(&(
        "devsweep.optimize.preview.v1",
        catalogue_version,
        operation_id,
        action_class,
        literals,
    ))
    .map_err(|_| OptimizePlanError::SerializationFailed)?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}
