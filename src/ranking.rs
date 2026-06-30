use std::{
    cmp::Ordering,
    time::{Duration, SystemTime},
};

use crate::model::{CleanTarget, CleanupPlan, Evidence};

pub const DEFAULT_FRESHNESS_FLOOR: Duration = Duration::from_secs(7 * 86_400);
pub const FRESHNESS_GUARD_RULE_ID: &str = "ranking.freshness_guard.7d";

pub fn rank_cleanup_plan(plan: &mut CleanupPlan) {
    rank_cleanup_plan_at(plan, SystemTime::now(), DEFAULT_FRESHNESS_FLOOR);
}

pub fn target_score(target: &CleanTarget, now: SystemTime) -> f64 {
    let Some(age) = target_age(target, now) else {
        return 0.0;
    };
    let size_mib = target.estimated_bytes as f64 / 1_048_576.0;
    let age_days = age.as_secs_f64() / 86_400.0;
    size_mib * age_days
}

pub(crate) fn rank_cleanup_plan_at(plan: &mut CleanupPlan, now: SystemTime, floor: Duration) {
    for target in &mut plan.targets {
        apply_freshness_guard(target, now, floor);
    }
    sort_targets(&mut plan.targets, now);
}

fn apply_freshness_guard(target: &mut CleanTarget, now: SystemTime, floor: Duration) {
    if !target.selected_by_default {
        return;
    }
    let Some(age) = target_age(target, now) else {
        return;
    };
    if age >= floor {
        return;
    }

    target.selected_by_default = false;
    if !target.evidence.iter().any(is_freshness_guard_evidence) {
        target.evidence.push(Evidence::RuleMatched {
            rule_id: FRESHNESS_GUARD_RULE_ID.to_string(),
        });
    }
}

fn is_freshness_guard_evidence(evidence: &Evidence) -> bool {
    matches!(
        evidence,
        Evidence::RuleMatched { rule_id } if rule_id == FRESHNESS_GUARD_RULE_ID
    )
}

fn target_age(target: &CleanTarget, now: SystemTime) -> Option<Duration> {
    let last_modified = target.last_modified?;
    Some(now.duration_since(last_modified).unwrap_or(Duration::ZERO))
}

fn sort_targets(targets: &mut [CleanTarget], now: SystemTime) {
    targets.sort_by(|left, right| compare_targets(left, right, now));
}

fn compare_targets(left: &CleanTarget, right: &CleanTarget, now: SystemTime) -> Ordering {
    right
        .estimated_bytes
        .cmp(&left.estimated_bytes)
        .then_with(|| {
            target_score(right, now)
                .partial_cmp(&target_score(left, now))
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| compare_last_modified(left, right))
        .then_with(|| left.id.as_str().cmp(right.id.as_str()))
}

fn compare_last_modified(left: &CleanTarget, right: &CleanTarget) -> Ordering {
    match (left.last_modified, right.last_modified) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{Duration, UNIX_EPOCH},
    };

    use crate::model::{
        CleanAction, CleanTarget, CleanupPlan, Ecosystem, RiskLevel, Scope, TargetId, TargetKind,
    };

    use super::*;

    #[test]
    fn sorts_targets_by_size_descending() {
        let now = UNIX_EPOCH + Duration::from_secs(30 * 86_400);
        let mut plan = plan(vec![
            target(
                "small",
                100,
                Some(now - Duration::from_secs(30 * 86_400)),
                true,
            ),
            target(
                "large",
                900,
                Some(now - Duration::from_secs(2 * 86_400)),
                true,
            ),
            target(
                "middle",
                500,
                Some(now - Duration::from_secs(20 * 86_400)),
                true,
            ),
        ]);

        rank_cleanup_plan_at(&mut plan, now, DEFAULT_FRESHNESS_FLOOR);

        assert_eq!(target_ids(&plan), ["large", "middle", "small"]);
    }

    #[test]
    fn equal_sizes_use_score_age_and_id_tie_breakers() {
        let now = UNIX_EPOCH + Duration::from_secs(30 * 86_400);
        let mut plan = plan(vec![
            target("gamma", 1000, None, false),
            target("alpha", 1000, None, false),
            target(
                "recent",
                1000,
                Some(now - Duration::from_secs(10 * 86_400)),
                false,
            ),
            target(
                "old",
                1000,
                Some(now - Duration::from_secs(20 * 86_400)),
                false,
            ),
        ]);

        rank_cleanup_plan_at(&mut plan, now, DEFAULT_FRESHNESS_FLOOR);

        assert_eq!(target_ids(&plan), ["old", "recent", "alpha", "gamma"]);
    }

    #[test]
    fn score_is_size_mib_times_age_days() {
        let now = UNIX_EPOCH + Duration::from_secs(10 * 86_400);
        let target = target(
            "cache",
            2 * 1_048_576,
            Some(now - Duration::from_secs(3 * 86_400)),
            false,
        );

        assert_eq!(target_score(&target, now), 6.0);
    }

    #[test]
    fn score_returns_zero_for_missing_or_future_mtime() {
        let now = UNIX_EPOCH + Duration::from_secs(10 * 86_400);
        let missing = target("missing", 2 * 1_048_576, None, false);
        let future = target(
            "future",
            2 * 1_048_576,
            Some(now + Duration::from_secs(86_400)),
            false,
        );

        assert_eq!(target_score(&missing, now), 0.0);
        assert_eq!(target_score(&future, now), 0.0);
    }

    #[test]
    fn freshness_guard_deselects_recent_default_targets_once() {
        let now = UNIX_EPOCH + Duration::from_secs(30 * 86_400);
        let mut plan = plan(vec![target(
            "recent",
            100,
            Some(now - Duration::from_secs(2 * 86_400)),
            true,
        )]);

        rank_cleanup_plan_at(&mut plan, now, DEFAULT_FRESHNESS_FLOOR);
        rank_cleanup_plan_at(&mut plan, now, DEFAULT_FRESHNESS_FLOOR);

        let target = &plan.targets[0];
        assert!(!target.selected_by_default);
        assert_eq!(
            target
                .evidence
                .iter()
                .filter(|evidence| is_freshness_guard_evidence(evidence))
                .count(),
            1
        );
    }

    #[test]
    fn freshness_guard_preserves_stale_missing_and_unselected_targets() {
        let now = UNIX_EPOCH + Duration::from_secs(30 * 86_400);
        let mut plan = plan(vec![
            target(
                "stale",
                100,
                Some(now - Duration::from_secs(10 * 86_400)),
                true,
            ),
            target("missing", 100, None, true),
            target("future", 100, Some(now + Duration::from_secs(86_400)), true),
            target(
                "unselected",
                100,
                Some(now - Duration::from_secs(2 * 86_400)),
                false,
            ),
        ]);

        rank_cleanup_plan_at(&mut plan, now, DEFAULT_FRESHNESS_FLOOR);

        let selected_by_id: Vec<_> = plan
            .targets
            .iter()
            .map(|target| (target.id.as_str(), target.selected_by_default))
            .collect();
        assert!(selected_by_id.contains(&("stale", true)));
        assert!(selected_by_id.contains(&("missing", true)));
        assert!(selected_by_id.contains(&("future", false)));
        assert!(selected_by_id.contains(&("unselected", false)));
    }

    fn plan(targets: Vec<CleanTarget>) -> CleanupPlan {
        CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
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
