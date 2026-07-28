use std::path::PathBuf;

use anyhow::Result;

use crate::model::CleanupPlan;
use crate::providers::GlobalProviderScanner;
use crate::ranking::rank_cleanup_plan;
use crate::scanner::ProjectScanner;

pub trait ProjectScan {
    fn scan_roots(&self, roots: &[PathBuf]) -> Result<CleanupPlan>;
}

pub trait GlobalScan {
    fn scan(&self) -> CleanupPlan;
}

impl ProjectScan for ProjectScanner {
    fn scan_roots(&self, roots: &[PathBuf]) -> Result<CleanupPlan> {
        ProjectScanner::scan_roots(self, roots)
    }
}

impl GlobalScan for GlobalProviderScanner {
    fn scan(&self) -> CleanupPlan {
        GlobalProviderScanner::scan(self)
    }
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub include_projects: bool,
    pub include_global: bool,
    pub roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPhase {
    Projects,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanProgress {
    pub phase: ScanPhase,
    pub message: String,
    /// Cumulative, already-ranked partial plan for the phases finished so far.
    pub partial: Option<CleanupPlan>,
}

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
    pub fn new(projects: P, global: G) -> Self {
        Self { projects, global }
    }
}

impl<P: ProjectScan, G: GlobalScan> Sweeper<P, G> {
    /// Runs the scan pipeline and returns the merged plan. The returned plan is
    /// ranked with the freshness guard applied; callers must not rank again.
    pub fn full_scan(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
    ) -> Result<CleanupPlan> {
        let mut plan = CleanupPlan::empty();

        if options.include_projects {
            progress(ScanProgress {
                phase: ScanPhase::Projects,
                message: "Scanning current directory".to_string(),
                partial: None,
            });
            let project_plan = self.projects.scan_roots(&options.roots)?;
            let count = project_plan.targets.len();
            plan.targets.extend(project_plan.targets);
            rank_merged_plan(&mut plan);
            progress(ScanProgress {
                phase: ScanPhase::Projects,
                message: format!("Project scan finished: {count} target(s)"),
                partial: Some(plan.clone()),
            });
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
            let global_plan = self.global.scan();
            let count = global_plan.targets.len();
            plan.targets.extend(global_plan.targets);
            rank_merged_plan(&mut plan);
            progress(ScanProgress {
                phase: ScanPhase::Global,
                message: format!("Global scan finished: {count} target(s)"),
                partial: Some(plan.clone()),
            });
        }

        Ok(plan)
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

    use crate::model::{
        CLEANUP_PLAN_VERSION, CleanAction, CleanTarget, Ecosystem, Evidence, RiskLevel, Scope,
        TargetId, TargetKind,
    };
    use crate::ranking::FRESHNESS_GUARD_RULE_ID;

    use super::*;

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
        fn scan_roots(&self, _roots: &[PathBuf]) -> Result<CleanupPlan> {
            Ok(plan_with(self.0.clone()))
        }
    }

    struct FakeGlobal(Vec<CleanTarget>);

    impl GlobalScan for FakeGlobal {
        fn scan(&self) -> CleanupPlan {
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
            last_modified,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default,
            evidence: Vec::new(),
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
}
