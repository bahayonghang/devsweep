use std::{path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

use crate::process_runner::{ProcessResult, ProcessStatus};

pub const CLEANUP_PLAN_VERSION: u32 = 2;
pub const LEGACY_CLEANUP_PLAN_VERSION: u32 = 1;
pub const SCAN_REPORT_VERSION: u32 = 1;

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

/// JSON-facing scan document. Its health observations are informational only;
/// cleanup authority remains the embedded [`UntrustedPlan`] after validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanReport {
    pub version: u32,
    pub plan: UntrustedPlan,
    pub health: ScanHealth,
}

impl ScanReport {
    pub fn new(plan: UntrustedPlan, mut health: ScanHealth) -> Self {
        health.totals = ScanTotals::from_untrusted_plan(&plan);
        if plan.targets.iter().any(|target| !target.size_complete) {
            health.mark_partial();
        }
        Self {
            version: SCAN_REPORT_VERSION,
            plan,
            health,
        }
    }

    pub fn has_supported_version(&self) -> bool {
        self.version == SCAN_REPORT_VERSION
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanHealth {
    pub completeness: ScanCompleteness,
    pub diagnostics: Vec<ScanDiagnostic>,
    pub totals: ScanTotals,
}

impl ScanHealth {
    pub fn complete() -> Self {
        Self::new(ScanCompleteness::Complete, Vec::new())
    }

    pub fn new(completeness: ScanCompleteness, diagnostics: Vec<ScanDiagnostic>) -> Self {
        Self {
            completeness: if diagnostics.is_empty() {
                completeness
            } else {
                ScanCompleteness::Partial
            },
            diagnostics,
            totals: ScanTotals::default(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.completeness.is_complete()
    }

    pub fn merge(&mut self, other: Self) {
        if !other.is_complete() {
            self.completeness = ScanCompleteness::Partial;
        }
        self.diagnostics.extend(other.diagnostics);
    }

    pub fn mark_partial(&mut self) {
        self.completeness = ScanCompleteness::Partial;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanCompleteness {
    Complete,
    Partial,
}

impl ScanCompleteness {
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
        }
    }
}

/// Structured scan observation. `detail` is sanitized presentation context;
/// callers must use the typed stage, outcome, and optional process metadata
/// for programmatic handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanDiagnostic {
    pub stage: ScanDiagnosticStage,
    pub path: PathBuf,
    pub outcome: ScanDiagnosticOutcome,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<ScanProcessProbe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanDiagnosticStage {
    Discovery,
    Sizing,
    CargoMetadata,
    Provider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanDiagnosticOutcome {
    Skipped,
    Failed,
    Canceled,
    OutputTruncated,
}

/// Serializable process metadata for a scan diagnostic. It deliberately omits
/// command text and captured output, keeping reports observational and safe to
/// export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanProcessProbe {
    pub status: ScanProcessStatus,
    pub stdout: ScanProcessOutput,
    pub stderr: ScanProcessOutput,
}

impl From<&ProcessResult> for ScanProcessProbe {
    fn from(result: &ProcessResult) -> Self {
        Self {
            status: ScanProcessStatus::from(&result.status),
            stdout: ScanProcessOutput {
                truncated: result.output.stdout_truncated,
                retained_bytes: result.output.stdout.len() as u64,
                total_bytes: result.output.total_stdout_bytes,
            },
            stderr: ScanProcessOutput {
                truncated: result.output.stderr_truncated,
                retained_bytes: result.output.stderr.len() as u64,
                total_bytes: result.output.total_stderr_bytes,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScanProcessStatus {
    Success,
    NotFound,
    Timeout,
    Exit { code: Option<i32> },
    InvalidOutput,
    Canceled,
}

impl From<&ProcessStatus> for ScanProcessStatus {
    fn from(status: &ProcessStatus) -> Self {
        match status {
            ProcessStatus::Success => Self::Success,
            ProcessStatus::NotFound => Self::NotFound,
            ProcessStatus::Timeout => Self::Timeout,
            ProcessStatus::Exit { code } => Self::Exit { code: *code },
            ProcessStatus::InvalidOutput => Self::InvalidOutput,
            ProcessStatus::Canceled => Self::Canceled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanProcessOutput {
    pub truncated: bool,
    pub retained_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanTotals {
    pub verified_bytes: u64,
    pub partial_lower_bound_bytes: u64,
    pub unknown_target_count: u64,
}

impl ScanTotals {
    pub fn from_untrusted_plan(plan: &UntrustedPlan) -> Self {
        Self::from_sizes(
            plan.targets
                .iter()
                .map(|target| (target.estimated_bytes, target.size_complete)),
        )
    }

    pub fn from_cleanup_plan(plan: &CleanupPlan) -> Self {
        Self::from_sizes(
            plan.targets
                .iter()
                .map(|target| (target.estimated_bytes, target.size_complete)),
        )
    }

    fn from_sizes(sizes: impl IntoIterator<Item = (u64, bool)>) -> Self {
        let mut totals = Self::default();
        for (estimated_bytes, size_complete) in sizes {
            if size_complete {
                totals.verified_bytes = totals.verified_bytes.saturating_add(estimated_bytes);
            } else if estimated_bytes == 0 {
                totals.unknown_target_count = totals.unknown_target_count.saturating_add(1);
            } else {
                totals.partial_lower_bound_bytes = totals
                    .partial_lower_bound_bytes
                    .saturating_add(estimated_bytes);
            }
        }
        totals
    }
}

/// A non-authoritative reason why a target size is incomplete or lower-bound
/// only. These observations are retained for review but are never used to
/// reconstruct or authorize a cleanup action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizingWarning {
    pub kind: SizingWarningKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizingWarningKind {
    Canceled,
    EntryBudgetExhausted,
    MetadataUnavailable,
    ReparseSafetyUnverified,
    MaxDepthReached,
    DirectoryReadFailed,
    DirectoryEntryReadFailed,
    PathUnresolved,
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
    /// When false, `estimated_bytes` is only a lower bound (or zero is untrusted).
    #[serde(default = "default_size_complete")]
    pub size_complete: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sizing_warnings: Vec<SizingWarning>,
    pub last_modified: Option<SystemTime>,
    pub risk: RiskLevel,
    pub reversible: bool,
    pub selected_by_default: bool,
    pub evidence: Vec<Evidence>,
    pub intent: CleanupIntent,
}

fn default_size_complete() -> bool {
    true
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
    /// When false, `estimated_bytes` is only a lower bound (or zero is untrusted).
    pub size_complete: bool,
    pub sizing_warnings: Vec<SizingWarning>,
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

    #[test]
    fn scan_report_round_trips_with_structured_health() {
        let plan = UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![untrusted_target("complete", 1024, true)],
        };
        let diagnostic = ScanDiagnostic {
            stage: ScanDiagnosticStage::CargoMetadata,
            path: PathBuf::from("C:/code/app/Cargo.toml"),
            outcome: ScanDiagnosticOutcome::OutputTruncated,
            detail: "cargo metadata output was truncated".to_string(),
            process: Some(ScanProcessProbe {
                status: ScanProcessStatus::Success,
                stdout: ScanProcessOutput {
                    truncated: true,
                    retained_bytes: 1024,
                    total_bytes: 4096,
                },
                stderr: ScanProcessOutput {
                    truncated: false,
                    retained_bytes: 0,
                    total_bytes: 0,
                },
            }),
        };
        let report = ScanReport::new(
            plan.clone(),
            ScanHealth::new(ScanCompleteness::Partial, vec![diagnostic]),
        );

        let json = serde_json::to_string(&report).expect("report serializes");
        let decoded: ScanReport = serde_json::from_str(&json).expect("report deserializes");

        assert_eq!(decoded, report);
        assert_eq!(decoded.version, SCAN_REPORT_VERSION);
        assert_eq!(decoded.plan, plan);
        assert_eq!(decoded.health.totals.verified_bytes, 1024);
        assert!(!json.contains("\"program\""));
        assert!(!json.contains("\"args\""));
    }

    #[test]
    fn scan_report_rejects_untrusted_executable_fields() {
        let report = ScanReport::new(UntrustedPlan::empty(), ScanHealth::complete());
        let mut document = serde_json::to_value(report).expect("report serializes");
        document["program"] = serde_json::json!("powershell.exe");

        assert!(
            serde_json::from_value::<ScanReport>(document).is_err(),
            "reports must reject unexpected executable fields"
        );
    }

    #[test]
    fn embedded_plan_rejects_untrusted_executable_fields() {
        let report = ScanReport::new(UntrustedPlan::empty(), ScanHealth::complete());
        let mut document = serde_json::to_value(report).expect("report serializes");
        document["plan"]["program"] = serde_json::json!("powershell.exe");

        assert!(
            serde_json::from_value::<ScanReport>(document).is_err(),
            "the embedded plan must reject unexpected executable fields"
        );
    }

    #[test]
    fn diagnostics_force_partial_scan_health() {
        let health = ScanHealth::new(
            ScanCompleteness::Complete,
            vec![ScanDiagnostic {
                stage: ScanDiagnosticStage::Discovery,
                path: PathBuf::from("C:/code/app/unreadable"),
                outcome: ScanDiagnosticOutcome::Skipped,
                detail: "failed to read directory".to_string(),
                process: None,
            }],
        );

        assert_eq!(health.completeness, ScanCompleteness::Partial);
    }

    #[test]
    fn scan_totals_keep_verified_partial_and_unknown_observations_separate() {
        let plan = UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![
                untrusted_target("complete", 100, true),
                untrusted_target("partial", 40, false),
                untrusted_target("unknown", 0, false),
            ],
        };

        let totals = ScanTotals::from_untrusted_plan(&plan);

        assert_eq!(totals.verified_bytes, 100);
        assert_eq!(totals.partial_lower_bound_bytes, 40);
        assert_eq!(totals.unknown_target_count, 1);
        assert_eq!(
            ScanReport::new(plan, ScanHealth::complete()).health.totals,
            totals
        );
    }

    #[test]
    fn incomplete_target_warnings_are_typed_and_make_a_report_partial() {
        let mut target = untrusted_target("partial", 40, false);
        target.sizing_warnings.push(SizingWarning {
            kind: SizingWarningKind::EntryBudgetExhausted,
            detail: "size entry budget exhausted under C:/code/app/partial".to_string(),
        });
        let report = ScanReport::new(
            UntrustedPlan {
                version: CLEANUP_PLAN_VERSION,
                targets: vec![target],
            },
            ScanHealth::complete(),
        );

        let json = serde_json::to_value(&report).expect("report serializes");

        assert_eq!(report.health.completeness, ScanCompleteness::Partial);
        assert_eq!(
            json["plan"]["targets"][0]["sizing_warnings"][0]["kind"],
            "entry_budget_exhausted"
        );
    }

    fn untrusted_target(id: &str, estimated_bytes: u64, size_complete: bool) -> UntrustedTarget {
        UntrustedTarget {
            id: TargetId::new(format!("node.node_modules:C:/code/app/{id}")),
            rule_id: "node.node_modules".to_string(),
            scope: Scope::Project {
                root: PathBuf::from("C:/code/app"),
            },
            ecosystem: Ecosystem::Node,
            kind: TargetKind::DependencyDirectory,
            path: Some(PathBuf::from(format!("C:/code/app/{id}"))),
            estimated_bytes,
            size_complete,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Medium,
            reversible: true,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "node.node_modules".to_string(),
            }],
            intent: CleanupIntent::TrashProjectArtifact {
                rule_id: "node.node_modules".to_string(),
            },
        }
    }
}
