use super::*;
use crate::model::{
    CleanupIntent, Ecosystem, Evidence, RiskLevel, ScanCoverage, ScanHealth, UntrustedPlan,
    UntrustedTarget,
};

fn native_path(value: &str) -> PathBuf {
    if cfg!(windows) {
        value.into()
    } else {
        value.strip_prefix("C:").unwrap_or(value).into()
    }
}

fn target(
    id: &str,
    root: &str,
    suffix: &str,
    bytes: u64,
    complete: bool,
    kind: TargetKind,
) -> UntrustedTarget {
    UntrustedTarget {
        id: TargetId::new(id),
        rule_id: "node.next_cache".into(),
        scope: Scope::Project {
            root: native_path(root),
        },
        ecosystem: Ecosystem::Node,
        kind,
        path: Some(native_path(&format!("{root}/{suffix}"))),
        estimated_bytes: bytes,
        size_complete: complete,
        sizing_warnings: vec![],
        last_modified: None,
        risk: RiskLevel::Low,
        reversible: true,
        selected_by_default: false,
        evidence: vec![Evidence::RuleMatched {
            rule_id: "node.next_cache".into(),
        }],
        intent: CleanupIntent::TrashProjectArtifact {
            rule_id: "node.next_cache".into(),
        },
    }
}
fn coverage(root: &str, status: ScanCoverageStatus, count: usize) -> ScanCoverage {
    ScanCoverage {
        id: root.into(),
        scope: ScanCoverageScope::Project,
        path: Some(native_path(root)),
        status,
        target_count: count,
        detail: "isolated observation".into(),
    }
}
fn report(targets: Vec<UntrustedTarget>) -> ScanReport {
    let mut health = ScanHealth::complete();
    for root in ["C:/projects/one", "C:/projects/two"] {
        health.record_coverage(coverage(root, ScanCoverageStatus::Complete, targets.len()));
    }
    ScanReport::new(
        UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets,
        },
        health,
    )
}
fn artifact(id: &str, bytes: u64, complete: bool) -> UntrustedTarget {
    target(
        id,
        "C:/projects/one",
        id,
        bytes,
        complete,
        TargetKind::BuildArtifacts,
    )
}

#[test]
fn inclusive_threshold_and_smaller_artifacts_sum_without_dependencies() {
    for (bytes, assessment) in [
        (
            DEFAULT_THRESHOLD_BYTES - 1,
            CapacityAssessment::BelowThreshold,
        ),
        (DEFAULT_THRESHOLD_BYTES, CapacityAssessment::Oversized),
        (DEFAULT_THRESHOLD_BYTES + 1, CapacityAssessment::Oversized),
    ] {
        let output =
            project_capacity_advisories(&report(vec![artifact("build", bytes, true)]), 5).unwrap();
        assert_eq!(output.projects[0].assessment, assessment);
        assert_eq!(
            output.projects[0].artifacts.confidence,
            CapacityConfidence::Exact
        );
    }
    let input = report(vec![
        artifact("build", 3 * GIB, true),
        target(
            "tests",
            "C:/projects/one",
            "tests",
            2 * GIB,
            true,
            TargetKind::TestCache,
        ),
        target(
            "dependencies",
            "C:/projects/one",
            "node_modules",
            20 * GIB,
            true,
            TargetKind::DependencyDirectory,
        ),
        target(
            "second",
            "C:/projects/two",
            "build",
            GIB,
            true,
            TargetKind::ToolCache,
        ),
    ]);
    let output = project_capacity_advisories(&input, 5).unwrap();
    assert_eq!(output.projects.len(), 2);
    assert_eq!(output.projects[0].artifacts.known_bytes, 5 * GIB);
    assert_eq!(output.projects[0].dependencies.known_bytes, 20 * GIB);
    assert_eq!(
        output.projects[1].assessment,
        CapacityAssessment::BelowThreshold
    );
}

#[test]
fn partial_above_threshold_warns_below_and_unknown_never_claim_healthy() {
    for (bytes, assessment, confidence) in [
        (
            5 * GIB,
            CapacityAssessment::Oversized,
            CapacityConfidence::LowerBound,
        ),
        (
            GIB,
            CapacityAssessment::Incomplete,
            CapacityConfidence::LowerBound,
        ),
        (
            0,
            CapacityAssessment::Incomplete,
            CapacityConfidence::Unknown,
        ),
    ] {
        let output =
            project_capacity_advisories(&report(vec![artifact("partial", bytes, false)]), 5)
                .unwrap();
        assert_eq!(output.projects[0].assessment, assessment);
        assert_eq!(output.projects[0].artifacts.confidence, confidence);
    }
}

#[test]
fn overlapping_aliases_count_once_and_shared_or_unresolved_bytes_are_excluded() {
    let mut alias = artifact("alias", 3 * GIB, true);
    alias.path = Some(native_path("C:/projects/one/build/."));
    let input = report(vec![
        artifact("build", 3 * GIB, true),
        alias,
        artifact("build/child", GIB, true),
    ]);
    let output = project_capacity_advisories(&input, 5).unwrap();
    assert_eq!(output.projects[0].artifacts.known_bytes, 3 * GIB);
    assert_eq!(output.projects[0].artifacts.target_ids.len(), 3);
    let mut shared = artifact("shared-owner", 3 * GIB, true);
    shared.path = Some(native_path("C:/projects/one/build"));
    shared.scope = Scope::Project {
        root: native_path("C:/projects/two"),
    };
    let input = report(vec![artifact("build", 3 * GIB, true), shared]);
    let output = project_capacity_advisories(&input, 5).unwrap();
    for project in output.projects {
        assert_eq!(project.artifacts.known_bytes, 0);
        assert_eq!(project.assessment, CapacityAssessment::Incomplete);
        assert_eq!(project.excluded_target_ids.len(), 2);
    }
    let mut unlocated = artifact("unknown-path", 6 * GIB, true);
    unlocated.path = None;
    let output = project_capacity_advisories(&report(vec![unlocated]), 5).unwrap();
    assert_eq!(output.projects[0].artifacts.known_bytes, 0);
    assert_eq!(
        output.projects[0].assessment,
        CapacityAssessment::Incomplete
    );
}

#[test]
fn only_successfully_covered_empty_roots_are_exact_zero() {
    for status in [
        ScanCoverageStatus::Complete,
        ScanCoverageStatus::Empty,
        ScanCoverageStatus::Failed,
        ScanCoverageStatus::Skipped,
        ScanCoverageStatus::Unsupported,
        ScanCoverageStatus::Absent,
    ] {
        let mut input = ScanReport::new(UntrustedPlan::empty(), ScanHealth::complete());
        input
            .health
            .record_coverage(coverage("C:/projects/one", status, 0));
        let output = project_capacity_advisories(&input, 5).unwrap();
        let expected = if matches!(
            status,
            ScanCoverageStatus::Complete | ScanCoverageStatus::Empty
        ) {
            CapacityAssessment::BelowThreshold
        } else {
            CapacityAssessment::Incomplete
        };
        assert_eq!(output.projects[0].assessment, expected);
        assert_eq!(output.projects[0].artifacts.known_bytes, 0);
    }
    let mut input = report(vec![artifact("build", GIB, true)]);
    input.health.coverage.clear();
    assert_eq!(
        project_capacity_advisories(&input, 5).unwrap().projects[0].assessment,
        CapacityAssessment::Incomplete
    );
    input
        .health
        .record_coverage(coverage("C:/projects", ScanCoverageStatus::Skipped, 1));
    assert_eq!(
        project_capacity_advisories(&input, 5).unwrap().projects[0].assessment,
        CapacityAssessment::Incomplete
    );
}

#[test]
fn threshold_changes_preserve_report_and_observation_identity() {
    let input = report(vec![artifact("build", 5 * GIB, true)]);
    let before = serde_json::to_vec(&input).unwrap();
    let five = project_capacity_advisories(&input, 5).unwrap();
    let six = project_capacity_advisories(&input, 6).unwrap();
    assert_eq!(five.report_id, six.report_id);
    assert_eq!(five.projects[0].assessment, CapacityAssessment::Oversized);
    assert_eq!(
        six.projects[0].assessment,
        CapacityAssessment::BelowThreshold
    );
    assert_eq!(before, serde_json::to_vec(&input).unwrap());
    assert!(!input.plan.targets[0].selected_by_default);
}

#[test]
fn volume_root_coverage_includes_descendants_but_sibling_prefixes_do_not() {
    let mut input = report(vec![artifact("build", GIB, true)]);
    input.health.coverage = vec![coverage("C:/", ScanCoverageStatus::Complete, 1)];
    assert_eq!(
        project_capacity_advisories(&input, 5).unwrap().projects[0].assessment,
        CapacityAssessment::BelowThreshold
    );
    input.health.coverage = vec![coverage("C:/projects/on", ScanCoverageStatus::Complete, 1)];
    assert_eq!(
        project_capacity_advisories(&input, 5).unwrap().projects[0].assessment,
        CapacityAssessment::Incomplete
    );
    #[cfg(unix)]
    {
        input.plan.targets[0].scope = Scope::Project {
            root: "/projects/one".into(),
        };
        input.plan.targets[0].path = Some("/projects/one/build".into());
        input.health.coverage = vec![coverage("/", ScanCoverageStatus::Complete, 1)];
        assert_eq!(
            project_capacity_advisories(&input, 5).unwrap().projects[0].assessment,
            CapacityAssessment::BelowThreshold
        );
    }
}

#[test]
fn unsafe_integer_totals_are_capped_lower_bounds_and_bad_inputs_fail_closed() {
    let input = report(vec![
        artifact("build", u64::MAX, true),
        artifact("other", 1, true),
    ]);
    let output = project_capacity_advisories(&input, MAX_THRESHOLD_GIB).unwrap();
    assert_eq!(output.projects[0].artifacts.known_bytes, MAX_SAFE_INTEGER);
    assert_eq!(
        output.projects[0].artifacts.confidence,
        CapacityConfidence::LowerBound
    );
    assert_eq!(output.projects[0].assessment, CapacityAssessment::Oversized);
    let mut input = report(vec![
        artifact("duplicate", 1, true),
        artifact("duplicate", 1, true),
    ]);
    assert!(project_capacity_advisories(&input, 5).is_err());
    input.plan.targets.clear();
    input.version = 2;
    assert!(project_capacity_advisories(&input, 5).is_err());
}
