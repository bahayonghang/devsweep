use std::{path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

pub const CLEANUP_PLAN_VERSION: u32 = 2;
pub const LEGACY_CLEANUP_PLAN_VERSION: u32 = 1;

/// JSON-facing cleanup-plan DTO. It contains observed scan facts and typed
/// intent, but never an executable program, argv, cwd, or authoritative
/// cleanup action. Convert it through `plan_validation` before execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UntrustedPlan {
    pub version: u32,
    pub targets: Vec<UntrustedTarget>,
}

impl UntrustedPlan {
    pub fn empty() -> Self {
        Self {
            version: CLEANUP_PLAN_VERSION,
            targets: Vec::new(),
        }
    }

    pub fn default_selected_ids(&self) -> Vec<TargetId> {
        self.targets
            .iter()
            .filter(|target| target.selected_by_default)
            .map(|target| target.id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UntrustedTarget {
    pub id: TargetId,
    pub rule_id: String,
    pub scope: Scope,
    pub ecosystem: Ecosystem,
    pub kind: TargetKind,
    pub path: Option<PathBuf>,
    pub estimated_bytes: u64,
    pub last_modified: Option<SystemTime>,
    pub risk: RiskLevel,
    pub reversible: bool,
    pub selected_by_default: bool,
    pub evidence: Vec<Evidence>,
    pub intent: CleanupIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CleanupIntent {
    TrashProjectArtifact {
        rule_id: String,
    },
    RunBuiltInAction {
        provider_id: String,
        action_id: String,
    },
    InspectOnly {
        rule_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupPlan {
    pub version: u32,
    pub targets: Vec<CleanTarget>,
}

impl CleanupPlan {
    pub fn empty() -> Self {
        Self {
            version: CLEANUP_PLAN_VERSION,
            targets: Vec::new(),
        }
    }

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
pub struct TargetId(String);

impl TargetId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanTarget {
    pub id: TargetId,
    pub scope: Scope,
    pub ecosystem: Ecosystem,
    pub kind: TargetKind,
    pub path: Option<PathBuf>,
    pub estimated_bytes: u64,
    pub last_modified: Option<SystemTime>,
    pub risk: RiskLevel,
    pub reversible: bool,
    pub selected_by_default: bool,
    pub evidence: Vec<Evidence>,
    pub action: CleanAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scope {
    Global,
    Project { root: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecosystem {
    Rust,
    Node,
    Python,
    Docker,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    PackageCache,
    BuildArtifacts,
    DependencyDirectory,
    VirtualEnv,
    TestCache,
    ToolCache,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Dangerous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanAction {
    Command {
        program: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
        irreversible: bool,
    },
    MoveToTrash {
        path: PathBuf,
    },
    DeletePermanently {
        path: PathBuf,
        requires_explicit_flag: bool,
    },
    NoopInspectOnly,
}

impl CleanAction {
    pub(crate) fn is_executable(&self) -> bool {
        matches!(self, Self::Command { .. } | Self::MoveToTrash { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Evidence {
    MarkerFile { path: PathBuf },
    KnownCacheDir { source: String, path: PathBuf },
    OfficialCommand { command: String },
    RuleMatched { rule_id: String },
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
