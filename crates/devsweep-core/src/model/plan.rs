use std::{path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

use super::scan::SizingWarning;

/// Current persisted cleanup plan format version.
pub const CLEANUP_PLAN_VERSION: u32 = 2;
/// Legacy plan version retained only for explicit rejection guidance.
pub const LEGACY_CLEANUP_PLAN_VERSION: u32 = 1;

/// JSON-facing cleanup-plan DTO. It contains observed scan facts and typed
/// intent, but never an executable program, argv, cwd, or authoritative
/// cleanup action. Convert it through `crate::plan` before execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UntrustedPlan {
    /// Persisted plan format version.
    pub version: u32,
    /// Untrusted targets requiring validation before execution.
    pub targets: Vec<UntrustedTarget>,
}

impl UntrustedPlan {
    /// Creates an empty plan using the current format version.
    pub fn empty() -> Self {
        Self {
            version: CLEANUP_PLAN_VERSION,
            targets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Persisted scan observation and typed cleanup intent for one target.
pub struct UntrustedTarget {
    /// Stable target identifier.
    pub id: TargetId,
    /// Built-in rule that produced the target.
    pub rule_id: String,
    /// Filesystem or global scope observed by the scanner.
    pub scope: Scope,
    /// Tool ecosystem associated with the target.
    pub ecosystem: Ecosystem,
    /// Kind of generated artifact or cache.
    pub kind: TargetKind,
    /// Observed target path, when the target is path-backed.
    pub path: Option<PathBuf>,
    /// Estimated reclaimable byte count.
    pub estimated_bytes: u64,
    /// When false, `estimated_bytes` is only a lower bound (or zero is untrusted).
    #[serde(default = "default_size_complete")]
    pub size_complete: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Reasons why the size estimate is incomplete.
    pub sizing_warnings: Vec<SizingWarning>,
    /// Most recent observed modification time.
    pub last_modified: Option<SystemTime>,
    /// Scanner-assigned cleanup risk.
    pub risk: RiskLevel,
    /// Whether the declared cleanup intent is reversible.
    pub reversible: bool,
    /// Conservative scanner recommendation for initial selection.
    pub selected_by_default: bool,
    /// Observations supporting the target classification.
    pub evidence: Vec<Evidence>,
    /// Declarative cleanup intent resolved by the trusted rule registry.
    pub intent: CleanupIntent,
}

fn default_size_complete() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Declarative, non-executable cleanup intent stored in a plan.
pub enum CleanupIntent {
    /// Move a marker-backed project artifact to the OS trash.
    TrashProjectArtifact {
        /// Rule that authorizes the artifact shape.
        rule_id: String,
    },
    /// Run a built-in action reconstructed by the trusted registry.
    RunBuiltInAction {
        /// Built-in provider identifier.
        provider_id: String,
        /// Built-in action identifier.
        action_id: String,
    },
    /// Retain a finding for inspection without cleanup authority.
    InspectOnly {
        /// Rule that produced the inspect-only finding.
        rule_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Valid in-memory cleanup targets produced by the scanner.
pub struct CleanupPlan {
    /// Cleanup plan format version.
    pub version: u32,
    /// Ranked in-memory targets.
    pub targets: Vec<CleanTarget>,
}

impl CleanupPlan {
    /// Creates an empty in-memory plan using the current version.
    pub fn empty() -> Self {
        Self {
            version: CLEANUP_PLAN_VERSION,
            targets: Vec::new(),
        }
    }

    /// Returns targets selected by the scanner in plan order.
    pub fn default_selected_ids(&self) -> Vec<TargetId> {
        self.targets
            .iter()
            .filter(|target| target.selected_by_default)
            .map(|target| target.id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
/// Stable identifier for a cleanup target.
pub struct TargetId(String);

impl TargetId {
    /// Creates a target identifier from its string representation.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Trusted in-memory cleanup target produced by scanning or validation.
pub struct CleanTarget {
    /// Stable target identifier.
    pub id: TargetId,
    /// Filesystem or global target scope.
    pub scope: Scope,
    /// Tool ecosystem associated with the target.
    pub ecosystem: Ecosystem,
    /// Kind of generated artifact or cache.
    pub kind: TargetKind,
    /// Target path when the action is path-backed.
    pub path: Option<PathBuf>,
    /// Estimated reclaimable byte count.
    pub estimated_bytes: u64,
    /// When false, `estimated_bytes` is only a lower bound (or zero is untrusted).
    pub size_complete: bool,
    /// Reasons why the size estimate is incomplete.
    pub sizing_warnings: Vec<SizingWarning>,
    /// Most recent observed modification time.
    pub last_modified: Option<SystemTime>,
    /// Cleanup risk assigned by the rule catalogue.
    pub risk: RiskLevel,
    /// Whether the cleanup action is reversible.
    pub reversible: bool,
    /// Conservative scanner recommendation for initial selection.
    pub selected_by_default: bool,
    /// Observations supporting the target classification.
    pub evidence: Vec<Evidence>,
    /// Trusted action reconstructed from built-in rules.
    pub action: CleanAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Boundary within which a target was discovered.
pub enum Scope {
    /// Machine-level provider or cache scope.
    Global,
    /// Project scope rooted at the given path.
    Project {
        /// Project root that contains the target.
        root: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Tool ecosystem associated with a cleanup target.
pub enum Ecosystem {
    /// Rust and Cargo artifacts.
    Rust,
    /// Node.js package-manager artifacts.
    Node,
    /// Python environment and cache artifacts.
    Python,
    /// Docker storage observations.
    Docker,
    /// Ecosystem-neutral artifacts.
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Semantic category of a cleanup target.
pub enum TargetKind {
    /// Package-manager cache.
    PackageCache,
    /// Compiler or build output.
    BuildArtifacts,
    /// Installed dependency directory.
    DependencyDirectory,
    /// Language virtual environment.
    VirtualEnv,
    /// Test runner cache.
    TestCache,
    /// Tool-specific cache.
    ToolCache,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// User-facing risk assigned to a cleanup target.
pub enum RiskLevel {
    /// Low-risk generated data.
    Low,
    /// Medium-risk data requiring review.
    Medium,
    /// High-risk data requiring deliberate confirmation.
    High,
    /// Dangerous action that is not enabled by default.
    Dangerous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Trusted cleanup action reconstructed from a built-in rule.
pub enum CleanAction {
    /// Run a bounded external command with program and arguments kept separate.
    Command {
        /// Executable program name or path.
        program: String,
        /// Ordered command arguments.
        args: Vec<String>,
        /// Explicit working directory when the rule requires one.
        cwd: Option<PathBuf>,
        /// Whether the command cannot be reversed.
        irreversible: bool,
    },
    /// Move a path to the operating-system trash.
    MoveToTrash {
        /// Exact validated path to move.
        path: PathBuf,
    },
    // Kept as an explicit deny case at the validation, safety, and execution boundaries.
    #[allow(dead_code)]
    /// Disabled permanent-delete deny case.
    DeletePermanently {
        /// Exact path that would be deleted.
        path: PathBuf,
        /// Whether an explicit capability would be required.
        requires_explicit_flag: bool,
    },
    /// Inspect-only target with no cleanup side effect.
    NoopInspectOnly,
}

impl CleanAction {
    /// Returns whether the action can perform cleanup work.
    pub fn is_executable(&self) -> bool {
        matches!(self, Self::Command { .. } | Self::MoveToTrash { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Observation supporting why a cleanup target was discovered.
pub enum Evidence {
    /// Marker file proving a project ecosystem.
    MarkerFile {
        /// Observed marker path.
        path: PathBuf,
    },
    /// Known cache directory derived from a trusted source.
    KnownCacheDir {
        /// Source of the cache path.
        source: String,
        /// Observed cache path.
        path: PathBuf,
    },
    /// Official read-only provider command used for discovery.
    OfficialCommand {
        /// Human-readable command description.
        command: String,
    },
    /// Built-in rule that matched the target.
    RuleMatched {
        /// Matched rule identifier.
        rule_id: String,
    },
    /// Explicit user configuration contributed the observation.
    UserConfigured,
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use super::*;

    #[test]
    fn empty_plan_has_versioned_shape() {
        let json = serde_json::to_value(UntrustedPlan::empty()).expect("empty plan serializes");

        assert_eq!(json["version"], CLEANUP_PLAN_VERSION);
        assert!(
            json["targets"]
                .as_array()
                .expect("targets is an array")
                .is_empty()
        );
    }

    #[test]
    fn default_selected_ids_returns_default_selected_targets_in_plan_order() {
        let plan = CleanupPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![
                minimal_target("a", true),
                minimal_target("b", false),
                minimal_target("c", true),
            ],
        };

        assert_eq!(
            plan.default_selected_ids(),
            vec![TargetId::new("a"), TargetId::new("c")]
        );
    }

    fn minimal_target(id: &str, selected_by_default: bool) -> CleanTarget {
        CleanTarget {
            id: TargetId::new(id),
            scope: Scope::Global,
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::PackageCache,
            path: None,
            estimated_bytes: 0,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default,
            evidence: Vec::new(),
            action: CleanAction::NoopInspectOnly,
        }
    }

    #[test]
    fn untrusted_target_round_trips_through_json_without_executable_fields() {
        let target = UntrustedTarget {
            id: TargetId::new("rust.target:C:/code/app"),
            rule_id: "rust.target".to_string(),
            scope: Scope::Project {
                root: PathBuf::from("C:/code/app"),
            },
            ecosystem: Ecosystem::Rust,
            kind: TargetKind::BuildArtifacts,
            path: Some(PathBuf::from("C:/code/app/target")),
            estimated_bytes: 1024,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: Some(UNIX_EPOCH),
            risk: RiskLevel::Low,
            reversible: false,
            selected_by_default: true,
            evidence: vec![
                Evidence::MarkerFile {
                    path: PathBuf::from("C:/code/app/Cargo.toml"),
                },
                Evidence::OfficialCommand {
                    command: "cargo clean --manifest-path C:/code/app/Cargo.toml".to_string(),
                },
            ],
            intent: CleanupIntent::RunBuiltInAction {
                provider_id: "cargo".to_string(),
                action_id: "clean_manifest".to_string(),
            },
        };

        let json = serde_json::to_string(&target).expect("target serializes");
        let decoded: UntrustedTarget = serde_json::from_str(&json).expect("target deserializes");

        assert_eq!(decoded, target);
        assert_eq!(decoded.id.as_str(), "rust.target:C:/code/app");
        assert!(!json.contains("program"));
        assert!(!json.contains("args"));
    }
}
