//! Opt-in classification for Claude Code, Codex, and Cursor homes.
//!
//! Scan, plan, preview, recovery listing, and restore planning do not mutate
//! the home. Execute is the only mutation: a recovery bundle, then one
//! move-to-trash. Worktree removal that cannot keep Git metadata consistent
//! refuses with `deletion_policy` before that write.

mod execute;
mod plan;
mod scan;
mod worktree;

#[cfg(test)]
mod tests;

use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use execute::{execute_ai, recovery_list, restore_plan};
pub use plan::{plan_ai, preview_ai};
pub use scan::{default_home_path, scan_ai};

/// Session directory entries examined before the observation is incomplete.
pub(crate) const SESSION_ENUM_LIMIT: usize = 4096;

const SCHEMA_OBSERVATION: &str = "devsweep.ai.observation.v1";
const SCHEMA_PLAN: &str = "devsweep.ai.plan.v1";
const SCHEMA_PREVIEW: &str = "devsweep.ai.preview.v1";
const SCHEMA_RECOVERY: &str = "devsweep.ai.recovery.v1";
const SCHEMA_RECOVERY_LIST: &str = "devsweep.ai.recovery-list.v1";
const SCHEMA_EXECUTE: &str = "devsweep.ai.execute.v1";

const PROTECTED_NAMES: &[&str] = &[
    "settings",
    "credentials",
    "memory",
    "skills",
    "mcp",
    "models",
    "projects",
];

/// Closed provider set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProvider {
    /// `%USERPROFILE%\.claude` when `--home` is omitted.
    #[serde(rename = "claude-code")]
    ClaudeCode,
    /// `%USERPROFILE%\.codex` when `--home` is omitted.
    #[serde(rename = "codex")]
    Codex,
    /// `%APPDATA%\Cursor` when `--home` is omitted.
    #[serde(rename = "cursor")]
    Cursor,
}

impl AiProvider {
    /// Stable English id.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
        }
    }

    pub(crate) const ALL: [Self; 3] = [Self::ClaudeCode, Self::Codex, Self::Cursor];
}

/// Closed class ids. `installed_tool` is never a cleanup target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiClass {
    /// Direct children of `cache/` named `regen-*`.
    RegenerableCache,
    /// `versions/<name>` other than the single line in `versions/current.txt`.
    SupersededVersion,
    /// Closed version-1 inactive session files.
    SessionArchive,
    /// Directories under `worktrees/`.
    Worktree,
    /// Typed install identity. Never selected.
    InstalledTool,
}

impl AiClass {
    /// Stable English id.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RegenerableCache => "regenerable_cache",
            Self::SupersededVersion => "superseded_version",
            Self::SessionArchive => "session_archive",
            Self::Worktree => "worktree",
            Self::InstalledTool => "installed_tool",
        }
    }
}

/// Why a row is or is not a target. Age is not a reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiDisposition {
    /// Selectable once a plan names the class.
    Candidate,
    /// Observed and protected from mutation.
    InspectOnly,
    /// Bytes are not the closed supported shape.
    UnsupportedVersion,
    /// Settings, credentials, memory, or another denied name.
    Protected,
}

/// Exit class for an AI operation. The CLI maps these to process statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiExit {
    /// Usage. Process exit 2.
    Usage,
    /// Changed identity. Process exit 3.
    Stale,
    /// Source or permission unavailable. Process exit 4.
    Unavailable,
    /// Truthful partial result. Process exit 5.
    Partial,
    /// Failed or unknown after dispatch. Process exit 6.
    Failed,
    /// Canceled before dispatch. Process exit 130.
    Canceled,
}

/// Stable AI failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiError {
    /// Process exit class.
    pub exit: AiExit,
    /// Locale-neutral code.
    pub code: &'static str,
    /// English diagnostic. JSON envelopes do not copy this into `error.message`.
    pub message: String,
}

impl AiError {
    pub(crate) fn new(exit: AiExit, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            exit,
            code,
            message: message.into(),
        }
    }

    pub(crate) fn usage(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(AiExit::Usage, code, message)
    }

    pub(crate) fn stale(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(AiExit::Stale, code, message)
    }

    pub(crate) fn unavailable(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(AiExit::Unavailable, code, message)
    }

    pub(crate) fn partial(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(AiExit::Partial, code, message)
    }

    pub(crate) fn failed(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(AiExit::Failed, code, message)
    }

    pub(crate) fn canceled() -> Self {
        Self::new(AiExit::Canceled, "canceled", "canceled before dispatch")
    }
}

impl std::fmt::Display for AiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for AiError {}

/// One provider home. A missing production default is `absent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProviderObservation {
    /// Provider id.
    pub provider: AiProvider,
    /// `present` or `absent`.
    pub state: String,
    /// Normalized absolute home, when present.
    pub home: Option<String>,
    /// Typed install identity. Never a deletion target.
    pub installed_tool: Option<AiInstalledTool>,
    /// Classified rows. None are selected by scan.
    pub rows: Vec<AiRow>,
    /// True when session enumeration stopped at [`SESSION_ENUM_LIMIT`].
    pub session_bounded: bool,
}

/// `{provider, version, install_kind}` from `install.json` or `unknown`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiInstalledTool {
    /// Provider id.
    pub provider: AiProvider,
    /// Version token, or `unknown`.
    pub version: String,
    /// `install_json` or `unknown`.
    pub install_kind: String,
}

/// One classified path. Scan does not select it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiRow {
    /// Stable id `ai:<provider>:<class>:<relative>`.
    pub id: String,
    /// Class id.
    pub class: AiClass,
    /// `/`-separated path relative to the home.
    pub relative_path: String,
    /// Whether a later plan may name this row.
    pub disposition: AiDisposition,
    /// Locale-neutral reason.
    pub reason: String,
    /// Content identity when the row is a candidate.
    pub content_sha256: Option<String>,
}

/// Non-mutating observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiObservationV1 {
    /// Schema id.
    pub schema: String,
    /// Schema version.
    pub version: u32,
    /// One entry per requested provider.
    pub providers: Vec<AiProviderObservation>,
}

impl AiObservationV1 {
    pub(crate) fn new(providers: Vec<AiProviderObservation>) -> Self {
        Self {
            schema: SCHEMA_OBSERVATION.to_string(),
            version: 1,
            providers,
        }
    }
}

/// Explicit home or the production default. Tests pass an explicit absolute home.
#[derive(Debug, Clone)]
pub struct AiScanRequest {
    /// `None` with no home observes all three production defaults.
    pub provider: Option<AiProvider>,
    /// `None` uses the production default. An explicit path must be absolute.
    pub home: Option<std::path::PathBuf>,
}

/// Cleanup or restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiPlanKind {
    /// Move selected targets to the Recycle Bin after a recovery bundle.
    Cleanup,
    /// Write a recovery bundle back when the destination is absent.
    Restore,
}

impl AiPlanKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Cleanup => "cleanup",
            Self::Restore => "restore",
        }
    }
}

/// One planned path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiPlanTarget {
    /// Stable target id.
    pub id: String,
    /// `/`-separated path relative to the home.
    pub relative_path: String,
    /// Content identity captured for the plan.
    pub content_sha256: String,
    /// `move_to_trash` or `restore_bytes`.
    pub action: String,
}

/// A path the plan names but does not select.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiRefused {
    /// `/`-separated path relative to the home.
    pub relative_path: String,
    /// Locale-neutral reason.
    pub reason: String,
}

/// Saved plan. Writing the file does not mutate the home.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiPlanV1 {
    /// Schema id.
    pub schema: String,
    /// Schema version.
    pub version: u32,
    /// Cleanup or restore.
    pub kind: AiPlanKind,
    /// Provider id.
    pub provider: AiProvider,
    /// Normalized absolute home.
    pub home: String,
    /// Class id.
    pub class: AiClass,
    /// Recovery bundle id for a restore plan.
    pub bundle_id: Option<String>,
    /// Selectable targets.
    pub targets: Vec<AiPlanTarget>,
    /// Same-class paths that are not targets.
    pub refused: Vec<AiRefused>,
}

/// Live preview. The digest is `sha256:` plus 64 lowercase hex characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiPreviewV1 {
    /// Schema id.
    pub schema: String,
    /// Schema version.
    pub version: u32,
    /// Cleanup or restore.
    pub kind: AiPlanKind,
    /// Provider id.
    pub provider: AiProvider,
    /// Normalized absolute home.
    pub home: String,
    /// Class id.
    pub class: AiClass,
    /// Live digest.
    pub digest: String,
    /// Targets bound into the digest.
    pub targets: Vec<AiPlanTarget>,
    /// Same-class paths that are not targets.
    pub refused: Vec<AiRefused>,
}

/// One recovery bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiRecoveryBundle {
    /// Bundle id.
    pub id: String,
    /// Provider id.
    pub provider: AiProvider,
    /// Class id.
    pub class: AiClass,
    /// Original relative path.
    pub relative_path: String,
    /// sha256 of the original bytes.
    pub content_sha256: String,
}

/// Recovery listing. This does not restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiRecoveryListV1 {
    /// Schema id.
    pub schema: String,
    /// Schema version.
    pub version: u32,
    /// Normalized home.
    pub home: String,
    /// Bundles under `<home>/.devsweep-recovery`.
    pub bundles: Vec<AiRecoveryBundle>,
}

/// Execute report. A failed move is not retried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiExecuteReportV1 {
    /// Schema id.
    pub schema: String,
    /// Schema version.
    pub version: u32,
    /// `succeeded`, `partial`, `failed`, or `unknown`.
    pub outcome: String,
    /// Published recovery bundle ids.
    pub bundle_ids: Vec<String>,
    /// Target ids moved to trash.
    pub moved: Vec<String>,
    /// Locale-neutral code when the outcome is not success.
    pub code: Option<String>,
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(Sha256::digest(bytes).as_slice())
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub(crate) fn normalize_home(path: &Path) -> Result<String, AiError> {
    crate::filesystem::normalize_absolute_path(path).map_err(|_| {
        AiError::usage(
            "invalid_home",
            format!("--home must be absolute: {}", path.display()),
        )
    })
}

pub(crate) fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        .unwrap_or(false)
}

pub(crate) fn protected_component(name: &str) -> bool {
    let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name);
    PROTECTED_NAMES.contains(&name) || PROTECTED_NAMES.contains(&stem)
}

pub(crate) fn io_unavailable(error: io::Error, message: impl Into<String>) -> AiError {
    if matches!(
        error.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound
    ) {
        AiError::unavailable("source_unavailable", message)
    } else {
        AiError::failed("source_unavailable", message)
    }
}

pub(crate) fn check_cancel(cancel: &dyn crate::process::CancelObserver) -> Result<(), AiError> {
    if cancel.is_cancel_requested() {
        Err(AiError::canceled())
    } else {
        Ok(())
    }
}
