use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::plan::{CleanupPlan, UntrustedPlan};

/// Current JSON scan report format version.
pub const SCAN_REPORT_VERSION: u32 = 1;

/// JSON-facing scan document. Its health observations are informational only;
/// cleanup authority remains the embedded [`UntrustedPlan`] after validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanReport {
    /// Scan report format version.
    pub version: u32,
    /// Embedded untrusted cleanup plan.
    pub plan: UntrustedPlan,
    /// Non-authoritative scan health observations.
    pub health: ScanHealth,
}

impl ScanReport {
    /// Creates a report and derives its capacity totals from the plan.
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

    /// Returns whether this report uses the supported format version.
    pub fn has_supported_version(&self) -> bool {
        self.version == SCAN_REPORT_VERSION
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Completeness, diagnostics, and capacity totals for a scan.
pub struct ScanHealth {
    /// Overall scan completeness.
    pub completeness: ScanCompleteness,
    /// Structured observations for incomplete or failed work.
    pub diagnostics: Vec<ScanDiagnostic>,
    /// Capacity totals derived from discovered targets.
    pub totals: ScanTotals,
}

impl ScanHealth {
    /// Creates complete scan health with no diagnostics.
    pub fn complete() -> Self {
        Self::new(ScanCompleteness::Complete, Vec::new())
    }

    /// Creates scan health and marks it partial when diagnostics are present.
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

    /// Returns whether the scan completed without partial observations.
    pub fn is_complete(&self) -> bool {
        self.completeness.is_complete()
    }

    /// Merges another phase's health into this value.
    pub(crate) fn merge(&mut self, other: Self) {
        if !other.is_complete() {
            self.completeness = ScanCompleteness::Partial;
        }
        self.diagnostics.extend(other.diagnostics);
    }

    /// Marks the scan as partial.
    pub fn mark_partial(&mut self) {
        self.completeness = ScanCompleteness::Partial;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Overall completeness of a scan result.
pub enum ScanCompleteness {
    /// All requested work completed with trusted size observations.
    Complete,
    /// Some work or size observations were incomplete.
    Partial,
}

impl ScanCompleteness {
    /// Returns whether the value is [`Self::Complete`].
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Returns a stable human-readable label.
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
    /// Scan stage that produced the diagnostic.
    pub stage: ScanDiagnosticStage,
    /// Path associated with the diagnostic.
    pub path: PathBuf,
    /// Typed diagnostic outcome.
    pub outcome: ScanDiagnosticOutcome,
    /// Sanitized presentation detail.
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Optional bounded process metadata.
    pub process: Option<ScanProcessProbe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Stage of the scan pipeline that produced a diagnostic.
pub enum ScanDiagnosticStage {
    /// Filesystem target discovery.
    Discovery,
    /// Bounded target sizing.
    Sizing,
    /// Cargo metadata probing.
    CargoMetadata,
    /// Global provider probing.
    Provider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Outcome recorded by a scan diagnostic.
pub enum ScanDiagnosticOutcome {
    /// Work was safely skipped.
    Skipped,
    /// Work failed.
    Failed,
    /// Work stopped after cooperative cancellation.
    Canceled,
    /// Bounded process output exceeded its capture limit.
    OutputTruncated,
}

/// Serializable process metadata for a scan diagnostic. It deliberately omits
/// command text and captured output, keeping reports observational and safe to
/// export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanProcessProbe {
    /// Typed process completion status.
    pub status: ScanProcessStatus,
    /// Bounded stdout metadata.
    pub stdout: ScanProcessOutput,
    /// Bounded stderr metadata.
    pub stderr: ScanProcessOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
/// Serializable status of a read-only scan process probe.
pub enum ScanProcessStatus {
    /// Process completed successfully.
    Success,
    /// Program was not found.
    NotFound,
    /// Process exceeded its timeout.
    Timeout,
    /// Process exited unsuccessfully.
    Exit {
        /// Platform exit code when available.
        code: Option<i32>,
    },
    /// Process output could not be interpreted safely.
    InvalidOutput,
    /// Process stopped after cooperative cancellation.
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Bounded output-size metadata without captured content.
pub struct ScanProcessOutput {
    /// Whether output exceeded the retained byte cap.
    pub truncated: bool,
    /// Number of bytes retained for diagnostics.
    pub retained_bytes: u64,
    /// Total number of output bytes observed.
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Capacity totals split by estimate confidence.
pub struct ScanTotals {
    /// Bytes from complete size estimates.
    pub verified_bytes: u64,
    /// Lower-bound bytes from partial estimates.
    pub partial_lower_bound_bytes: u64,
    /// Number of targets without a usable estimate.
    pub unknown_target_count: u64,
}

impl ScanTotals {
    /// Derives totals from an untrusted persisted plan.
    pub(crate) fn from_untrusted_plan(plan: &UntrustedPlan) -> Self {
        Self::from_sizes(
            plan.targets
                .iter()
                .map(|target| (target.estimated_bytes, target.size_complete)),
        )
    }

    /// Derives totals from an in-memory cleanup plan.
    pub fn from_cleanup_plan(plan: &CleanupPlan) -> Self {
        Self::from_sizes(
            plan.targets
                .iter()
                .map(|target| (target.estimated_bytes, target.size_complete)),
        )
    }

    pub(crate) fn from_sizes(sizes: impl IntoIterator<Item = (u64, bool)>) -> Self {
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
    /// Typed cause of the incomplete estimate.
    pub kind: SizingWarningKind,
    /// Sanitized warning detail.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Reason a target size is incomplete or unknown.
pub enum SizingWarningKind {
    /// Sizing stopped after cooperative cancellation.
    Canceled,
    /// The bounded entry budget was exhausted.
    EntryBudgetExhausted,
    /// Required filesystem metadata was unavailable.
    MetadataUnavailable,
    /// Reparse or symlink safety could not be verified.
    ReparseSafetyUnverified,
    /// The bounded traversal depth was reached.
    MaxDepthReached,
    /// A directory could not be read.
    DirectoryReadFailed,
    /// A directory entry could not be read.
    DirectoryEntryReadFailed,
    /// A path could not be resolved safely.
    PathUnresolved,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CLEANUP_PLAN_VERSION, CleanupIntent, Ecosystem, Evidence, RiskLevel, Scope, TargetId,
        TargetKind, UntrustedTarget,
    };

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
