use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use serde::{Deserialize, Serialize};

mod global;
mod project;
mod ranking;

pub use global::GlobalProviderScanner;
pub use project::ProjectScanner;

pub(crate) use global::resolve_executable;
#[cfg(test)]
use ranking::FRESHNESS_GUARD_RULE_ID;

use crate::model::{CleanupPlan, ScanHealth, ScanReport, TargetId};
use crate::plan::untrusted_plan_from_scan;
use crate::process::{CancelObserver, FlagCancelObserver};

use project::rescan_target_size;
use ranking::rank_cleanup_plan;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Project scan plan and associated health observations.
pub struct ProjectScanOutcome {
    /// Unranked project cleanup targets.
    pub plan: CleanupPlan,
    /// Project discovery and sizing health.
    pub health: ScanHealth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScanPipelineOutcome {
    plan: CleanupPlan,
    health: ScanHealth,
}

/// Injectable project-target scanner used by [`Sweeper`].
pub trait ProjectScan {
    /// Scans project roots while observing cooperative cancellation.
    fn scan_roots_with_cancel(
        &self,
        roots: &[PathBuf],
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<CleanupPlan>;

    /// Scans project roots and returns structured health observations.
    fn scan_roots_with_health(
        &self,
        roots: &[PathBuf],
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ProjectScanOutcome> {
        Ok(ProjectScanOutcome {
            plan: self.scan_roots_with_cancel(roots, cancel)?,
            health: ScanHealth::complete(),
        })
    }
}

/// Injectable global-provider scanner used by [`Sweeper`].
pub trait GlobalScan {
    /// Scans global providers while observing cooperative cancellation.
    fn scan_with_cancel(&self, cancel: Option<&Arc<FlagCancelObserver>>) -> CleanupPlan;
}

impl ProjectScan for ProjectScanner {
    fn scan_roots_with_cancel(
        &self,
        roots: &[PathBuf],
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<CleanupPlan> {
        Ok(ProjectScanner::scan_roots_with_diagnostics_and_cancel(self, roots, cancel)?.plan)
    }

    fn scan_roots_with_health(
        &self,
        roots: &[PathBuf],
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ProjectScanOutcome> {
        let outcome = ProjectScanner::scan_roots_with_diagnostics_and_cancel(self, roots, cancel)?;

        Ok(ProjectScanOutcome {
            plan: outcome.plan,
            health: ScanHealth::new(outcome.completeness, outcome.diagnostics),
        })
    }
}

impl GlobalScan for GlobalProviderScanner {
    fn scan_with_cancel(&self, cancel: Option<&Arc<FlagCancelObserver>>) -> CleanupPlan {
        GlobalProviderScanner::scan_with_cancel(self, cancel)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Scope and roots for one scan request.
pub struct ScanOptions {
    /// Whether to discover project-level targets.
    pub include_projects: bool,
    /// Whether to discover global-provider targets.
    pub include_global: bool,
    /// Project roots to traverse.
    pub roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Major phase of the scan pipeline.
pub enum ScanPhase {
    /// Project discovery and sizing phase.
    Projects,
    /// Global provider discovery phase.
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Cumulative progress snapshot emitted by the scan pipeline.
pub struct ScanProgress {
    /// Phase that emitted the update.
    pub phase: ScanPhase,
    /// Human-readable progress message.
    pub message: String,
    /// Cumulative, already-ranked partial plan for the phases finished so far.
    pub partial: Option<CleanupPlan>,
}

/// Orchestrates project and global scanning, merging, and ranking.
pub struct Sweeper<P = ProjectScanner, G = GlobalProviderScanner> {
    projects: P,
    global: G,
}

impl Default for Sweeper<ProjectScanner, GlobalProviderScanner> {
    fn default() -> Self {
        Self::new(ProjectScanner::new(), GlobalProviderScanner::new())
    }
}

impl<P, G> Sweeper<P, G> {
    /// Creates a sweeper from injectable project and global scanners.
    pub(crate) fn new(projects: P, global: G) -> Self {
        Self { projects, global }
    }
}

impl<P: ProjectScan, G: GlobalScan> Sweeper<P, G> {
    /// Runs the scan pipeline and returns the merged plan. The returned plan is
    /// ranked with the freshness guard applied; callers must not rank again.
    #[cfg(test)]
    pub(crate) fn full_scan(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
    ) -> Result<CleanupPlan> {
        self.full_scan_with_cancel(options, progress, None)
    }

    #[cfg(test)]
    pub(crate) fn full_scan_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<CleanupPlan> {
        Ok(self.full_scan_outcome(options, progress, cancel)?.plan)
    }

    /// Runs the pipeline and returns a non-authoritative report suitable for
    /// JSON export.
    pub fn full_scan_report(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
    ) -> Result<ScanReport> {
        self.full_scan_report_with_cancel(options, progress, None)
    }

    /// Runs the report pipeline while observing cooperative cancellation.
    pub fn full_scan_report_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanReport> {
        let outcome = self.full_scan_outcome(options, progress, cancel)?;
        let plan = untrusted_plan_from_scan(&outcome.plan)?;

        Ok(ScanReport::new(plan, outcome.health))
    }

    /// Runs a normal scan, then re-estimates exactly one discovered target with
    /// the reviewed higher budget. The target ID must be present in this live
    /// scan; callers cannot provide an arbitrary path to the size walker.
    pub fn full_scan_report_rescanning_target(
        &self,
        options: &ScanOptions,
        target_id: &TargetId,
        progress: &mut dyn FnMut(ScanProgress),
    ) -> Result<ScanReport> {
        self.full_scan_report_rescanning_target_with_cancel(options, target_id, progress, None)
    }

    /// Re-estimates one discovered target while observing cancellation.
    pub fn full_scan_report_rescanning_target_with_cancel(
        &self,
        options: &ScanOptions,
        target_id: &TargetId,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanReport> {
        let mut outcome = self.full_scan_outcome(options, progress, cancel)?;
        rescan_target_size(&mut outcome.plan, target_id, cancel)?;
        let plan = untrusted_plan_from_scan(&outcome.plan)?;

        Ok(ScanReport::new(plan, outcome.health))
    }

    fn full_scan_outcome(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanPipelineOutcome> {
        let mut plan = CleanupPlan::empty();
        let mut health = ScanHealth::complete();

        if options.include_projects {
            progress(ScanProgress {
                phase: ScanPhase::Projects,
                message: "Scanning current directory".to_string(),
                partial: None,
            });
            let project_outcome = self
                .projects
                .scan_roots_with_health(&options.roots, cancel)?;
            let count = project_outcome.plan.targets.len();
            plan.targets.extend(project_outcome.plan.targets);
            health.merge(project_outcome.health);
            rank_merged_plan(&mut plan);
            progress(ScanProgress {
                phase: ScanPhase::Projects,
                message: format!("Project scan finished: {count} target(s)"),
                partial: Some(plan.clone()),
            });
        }

        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            health.mark_partial();
            return Ok(ScanPipelineOutcome { plan, health });
        }

        if options.include_global {
            progress(ScanProgress {
                phase: ScanPhase::Global,
                message: "Scanning global providers".to_string(),
                partial: None,
            });
            progress(ScanProgress {
                phase: ScanPhase::Global,
                message: "Estimating global cache sizes".to_string(),
                partial: None,
            });
            let global_plan = self.global.scan_with_cancel(cancel);
            let count = global_plan.targets.len();
            plan.targets.extend(global_plan.targets);
            rank_merged_plan(&mut plan);
            progress(ScanProgress {
                phase: ScanPhase::Global,
                message: format!("Global scan finished: {count} target(s)"),
                partial: Some(plan.clone()),
            });
        }

        Ok(ScanPipelineOutcome { plan, health })
    }
}

/// Sole production call site for ranking: the pipeline owns "ranked exactly
/// once" as an invariant, re-applying it to the cumulative set per phase.
fn rank_merged_plan(plan: &mut CleanupPlan) {
    rank_cleanup_plan(plan);
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::*;
    use crate::model::{
        CLEANUP_PLAN_VERSION, CleanAction, CleanTarget, Ecosystem, Evidence, RiskLevel,
        ScanCompleteness, ScanDiagnostic, ScanDiagnosticOutcome, ScanDiagnosticStage, Scope,
        TargetId, TargetKind,
    };

    #[test]
    fn scan_request_and_progress_types_round_trip_with_snake_case_json() {
        let options = ScanOptions {
            include_projects: true,
            include_global: false,
            roots: vec![PathBuf::from("C:/workspace/app")],
        };
        let progress = ScanProgress {
            phase: ScanPhase::Projects,
            message: "project scan complete".to_string(),
            partial: Some(plan_with(vec![target("cache", 42, None, true)])),
        };

        let options_json = serde_json::to_value(&options).expect("scan options serialize");
        let progress_json = serde_json::to_value(&progress).expect("scan progress serializes");

        assert_eq!(options_json["include_projects"], true);
        assert_eq!(progress_json["phase"], "projects");
        assert_eq!(
            progress_json["partial"]["targets"][0]["action"]["type"],
            "move_to_trash"
        );
        assert_eq!(
            serde_json::from_value::<ScanOptions>(options_json).expect("scan options deserialize"),
            options
        );
        assert_eq!(
            serde_json::from_value::<ScanProgress>(progress_json)
                .expect("scan progress deserializes"),
            progress
        );
    }

    #[test]
    fn full_scan_merges_and_ranks_once() {
        let sweeper = Sweeper::new(
            FakeProjects(vec![target("small", 100, None, false)]),
            FakeGlobal(vec![
                target("middle", 500, None, false),
                target("large", 900, None, false),
            ]),
        );

        let plan = sweeper
            .full_scan(&scan_all(), &mut |_| {})
            .expect("fake scan succeeds");

        assert_eq!(target_ids(&plan), ["large", "middle", "small"]);
    }

    #[test]
    fn full_scan_applies_freshness_guard_exactly_once() {
        let now = SystemTime::now();
        let sweeper = Sweeper::new(
            FakeProjects(vec![target(
                "fresh",
                100,
                Some(now - Duration::from_secs(86_400)),
                true,
            )]),
            FakeGlobal(vec![target("stale", 900, None, true)]),
        );

        let plan = sweeper
            .full_scan(&scan_all(), &mut |_| {})
            .expect("fake scan succeeds");

        let fresh = plan
            .targets
            .iter()
            .find(|target| target.id.as_str() == "fresh")
            .expect("fresh target kept");
        assert!(!fresh.selected_by_default);
        assert_eq!(
            fresh
                .evidence
                .iter()
                .filter(|evidence| matches!(
                    evidence,
                    Evidence::RuleMatched { rule_id } if rule_id == FRESHNESS_GUARD_RULE_ID
                ))
                .count(),
            1
        );
    }

    #[test]
    fn progress_events_follow_staged_sequence() {
        let sweeper = Sweeper::new(
            FakeProjects(vec![target("project", 100, None, false)]),
            FakeGlobal(vec![
                target("global-a", 500, None, false),
                target("global-b", 900, None, false),
            ]),
        );
        let mut events = Vec::new();

        let plan = sweeper
            .full_scan(&scan_all(), &mut |event| events.push(event))
            .expect("fake scan succeeds");

        let expected = [
            (ScanPhase::Projects, "Scanning current directory", false),
            (
                ScanPhase::Projects,
                "Project scan finished: 1 target(s)",
                true,
            ),
            (ScanPhase::Global, "Scanning global providers", false),
            (ScanPhase::Global, "Estimating global cache sizes", false),
            (ScanPhase::Global, "Global scan finished: 2 target(s)", true),
        ];
        assert_eq!(events.len(), expected.len());
        for (event, (phase, message, has_partial)) in events.iter().zip(expected) {
            assert_eq!(event.phase, phase);
            assert_eq!(event.message, message);
            assert_eq!(event.partial.is_some(), has_partial);
        }
        assert_eq!(
            events.last().and_then(|event| event.partial.clone()),
            Some(plan)
        );
    }

    #[test]
    fn scan_options_skip_phases() {
        let projects_only = ScanOptions {
            include_projects: true,
            include_global: false,
            roots: Vec::new(),
        };
        let mut events = Vec::new();
        let plan = sweeper_with_both_sides()
            .full_scan(&projects_only, &mut |event| events.push(event))
            .expect("fake scan succeeds");
        assert!(
            events
                .iter()
                .all(|event| event.phase == ScanPhase::Projects)
        );
        assert_eq!(target_ids(&plan), ["project"]);

        let global_only = ScanOptions {
            include_projects: false,
            include_global: true,
            roots: Vec::new(),
        };
        let mut events = Vec::new();
        let plan = sweeper_with_both_sides()
            .full_scan(&global_only, &mut |event| events.push(event))
            .expect("fake scan succeeds");
        assert!(events.iter().all(|event| event.phase == ScanPhase::Global));
        assert_eq!(target_ids(&plan), ["global"]);
    }

    #[test]
    fn partial_plans_are_cumulative_and_ranked() {
        let sweeper = sweeper_with_both_sides();
        let mut partials = Vec::new();

        sweeper
            .full_scan(&scan_all(), &mut |event| {
                if let Some(partial) = event.partial {
                    partials.push(partial);
                }
            })
            .expect("fake scan succeeds");

        assert_eq!(partials.len(), 2);
        assert_eq!(target_ids(&partials[0]), ["project"]);
        assert_eq!(target_ids(&partials[1]), ["global", "project"]);
    }

    #[test]
    fn full_scan_report_preserves_health_and_separates_size_totals() {
        let mut partial = target("partial", 40, None, false);
        partial.size_complete = false;
        let mut unknown = target("unknown", 0, None, false);
        unknown.size_complete = false;
        let sweeper = Sweeper::new(
            FakeProjectsWithOutcome(ProjectScanOutcome {
                plan: plan_with(vec![target("complete", 100, None, false), partial]),
                health: ScanHealth::new(
                    ScanCompleteness::Partial,
                    vec![ScanDiagnostic {
                        stage: ScanDiagnosticStage::Discovery,
                        path: PathBuf::from("C:/workspace/app/unreadable"),
                        outcome: ScanDiagnosticOutcome::Skipped,
                        detail: "failed to read directory".to_string(),
                        process: None,
                    }],
                ),
            }),
            FakeGlobal(vec![unknown]),
        );

        let report = sweeper
            .full_scan_report(&scan_all(), &mut |_| {})
            .expect("report scan succeeds");

        assert_eq!(report.health.completeness, ScanCompleteness::Partial);
        assert_eq!(report.health.diagnostics.len(), 1);
        assert_eq!(report.health.totals.verified_bytes, 100);
        assert_eq!(report.health.totals.partial_lower_bound_bytes, 40);
        assert_eq!(report.health.totals.unknown_target_count, 1);
        assert_eq!(
            target_ids_from_untrusted(&report.plan),
            ["complete", "partial", "unknown"]
        );
    }

    fn sweeper_with_both_sides() -> Sweeper<FakeProjects, FakeGlobal> {
        Sweeper::new(
            FakeProjects(vec![target("project", 100, None, false)]),
            FakeGlobal(vec![target("global", 900, None, false)]),
        )
    }

    fn scan_all() -> ScanOptions {
        ScanOptions {
            include_projects: true,
            include_global: true,
            roots: Vec::new(),
        }
    }

    struct FakeProjects(Vec<CleanTarget>);

    impl ProjectScan for FakeProjects {
        fn scan_roots_with_cancel(
            &self,
            _roots: &[PathBuf],
            _cancel: Option<&Arc<FlagCancelObserver>>,
        ) -> Result<CleanupPlan> {
            Ok(plan_with(self.0.clone()))
        }
    }

    struct FakeProjectsWithOutcome(ProjectScanOutcome);

    impl ProjectScan for FakeProjectsWithOutcome {
        fn scan_roots_with_cancel(
            &self,
            _roots: &[PathBuf],
            _cancel: Option<&Arc<FlagCancelObserver>>,
        ) -> Result<CleanupPlan> {
            Ok(self.0.plan.clone())
        }

        fn scan_roots_with_health(
            &self,
            _roots: &[PathBuf],
            _cancel: Option<&Arc<FlagCancelObserver>>,
        ) -> Result<ProjectScanOutcome> {
            Ok(self.0.clone())
        }
    }

    struct FakeGlobal(Vec<CleanTarget>);

    impl GlobalScan for FakeGlobal {
        fn scan_with_cancel(&self, _cancel: Option<&Arc<FlagCancelObserver>>) -> CleanupPlan {
            plan_with(self.0.clone())
        }
    }

    fn plan_with(targets: Vec<CleanTarget>) -> CleanupPlan {
        CleanupPlan {
            version: CLEANUP_PLAN_VERSION,
            targets,
        }
    }

    fn target(
        id: &str,
        estimated_bytes: u64,
        last_modified: Option<SystemTime>,
        selected_by_default: bool,
    ) -> CleanTarget {
        CleanTarget {
            id: TargetId::new(id),
            scope: Scope::Project {
                root: PathBuf::from("C:/workspace/app"),
            },
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::ToolCache,
            path: Some(PathBuf::from(format!("C:/workspace/app/{id}"))),
            estimated_bytes,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "node.node_modules".to_string(),
            }],
            action: CleanAction::MoveToTrash {
                path: PathBuf::from(format!("C:/workspace/app/{id}")),
            },
        }
    }

    fn target_ids(plan: &CleanupPlan) -> Vec<&str> {
        plan.targets
            .iter()
            .map(|target| target.id.as_str())
            .collect()
    }

    fn target_ids_from_untrusted(plan: &crate::model::UntrustedPlan) -> Vec<&str> {
        plan.targets
            .iter()
            .map(|target| target.id.as_str())
            .collect()
    }
}
