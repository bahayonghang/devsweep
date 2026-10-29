use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    filesystem::normalize_absolute_path,
    model::{CleanAction, CleanTarget, Evidence, Scope, TargetId},
    rules::{RUST_SHARED_TARGET_RULE_DOC, RUST_TARGET_RULE_DOC},
};

pub(in crate::scan) fn dedupe_targets(mut targets: Vec<CleanTarget>) -> Vec<CleanTarget> {
    mark_shared_rust_targets(&mut targets);
    // Canonicalize once. The old scan compared every kept target to every later
    // target and called `canonicalize` inside those comparisons, so 10k sibling
    // caches did not finish a release scan in 10 minutes.
    let footprints: Vec<Option<PathBuf>> = targets
        .iter()
        .map(|target| target.path.as_deref().map(canonical_footprint))
        .collect();
    let mut order: Vec<usize> = (0..targets.len()).collect();
    order.sort_by(|&left, &right| {
        let left_depth = footprints[left].as_ref().map(|path| path.components().count());
        let right_depth = footprints[right]
            .as_ref()
            .map(|path| path.components().count());
        left_depth.cmp(&right_depth).then_with(|| {
            targets[left]
                .id
                .as_str()
                .cmp(targets[right].id.as_str())
        })
    });

    let mut slots: Vec<Option<CleanTarget>> = targets.into_iter().map(Some).collect();
    let mut kept_indices = Vec::new();
    let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
    for index in order {
        let key = action_bucket_key(slots[index].as_ref().expect("slot"));
        let bucket = buckets.entry(key).or_default();
        if let Some(&existing) = bucket.iter().find(|&&existing| {
            same_cached_footprint(
                &footprints[existing],
                &footprints[index],
                slots[existing].as_ref().expect("kept slot"),
                slots[index].as_ref().expect("candidate slot"),
            )
        }) {
            let evidence = std::mem::take(&mut slots[index].as_mut().expect("candidate").evidence);
            merge_unique_evidence(slots[existing].as_mut().expect("kept"), evidence);
            continue;
        }
        if bucket.iter().any(|&existing| {
            parent_cached_covers(&footprints[existing], &footprints[index])
        }) {
            continue;
        }
        bucket.push(index);
        kept_indices.push(index);
    }

    kept_indices
        .into_iter()
        .map(|index| slots[index].take().expect("kept target"))
        .collect()
}

/// Partition key equivalent to [`same_action_identity`]. Targets in different
/// buckets cannot merge or cover each other.
fn action_bucket_key(target: &CleanTarget) -> String {
    let owner = match &target.scope {
        Scope::Global => "global".to_string(),
        Scope::Project { root } => normalize_absolute_path(root)
            .unwrap_or_else(|_| root.to_string_lossy().into_owned()),
    };
    let action = match &target.action {
        CleanAction::MoveToTrash { .. } => "trash".to_string(),
        CleanAction::NoopInspectOnly => "noop".to_string(),
        CleanAction::DeletePermanently {
            requires_explicit_flag, ..
        } => format!("delete:{requires_explicit_flag}"),
        CleanAction::Command {
            program,
            args,
            cwd,
            irreversible,
        } => {
            let cwd = cwd
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!(
                "cmd\0{program}\0{}\0{cwd}\0{irreversible}",
                args.join("\0")
            )
        }
    };
    format!("{owner}\0{action}")
}

pub(super) fn footprint_depth(path: &Path) -> usize {
    canonical_footprint(path).components().count()
}

fn canonical_footprint(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg_attr(not(test), allow(dead_code))]
fn same_action_identity(left: &CleanTarget, right: &CleanTarget) -> bool {
    // Preserve competing owners for the observational footprint
    // projection. A matching cleanup method does not prove exclusive ownership.
    let same_owner = match (&left.scope, &right.scope) {
        (Scope::Global, Scope::Global) => true,
        (Scope::Project { root: left }, Scope::Project { root: right }) => {
            left == right
                || matches!((normalize_absolute_path(left), normalize_absolute_path(right)),
                (Ok(left), Ok(right)) if left == right)
        }
        _ => false,
    };
    if !same_owner {
        return false;
    }
    match (&left.action, &right.action) {
        (CleanAction::MoveToTrash { .. }, CleanAction::MoveToTrash { .. })
        | (CleanAction::NoopInspectOnly, CleanAction::NoopInspectOnly) => true,
        (
            CleanAction::DeletePermanently {
                requires_explicit_flag: left_flag,
                ..
            },
            CleanAction::DeletePermanently {
                requires_explicit_flag: right_flag,
                ..
            },
        ) => left_flag == right_flag,
        (
            CleanAction::Command {
                program: left_program,
                args: left_args,
                cwd: left_cwd,
                irreversible: left_irreversible,
            },
            CleanAction::Command {
                program: right_program,
                args: right_args,
                cwd: right_cwd,
                irreversible: right_irreversible,
            },
        ) => {
            left_program == right_program
                && left_args == right_args
                && left_cwd == right_cwd
                && left_irreversible == right_irreversible
        }
        _ => false,
    }
}

fn mark_shared_rust_targets(targets: &mut [CleanTarget]) {
    let mut located = targets
        .iter()
        .enumerate()
        .filter_map(|(index, target)| {
            let Scope::Project { root } = &target.scope else {
                return None;
            };
            if !target.evidence.iter().any(|item| {
                matches!(item, Evidence::RuleMatched { rule_id }
            if rule_id == RUST_TARGET_RULE_DOC.id || rule_id == RUST_SHARED_TARGET_RULE_DOC.id)
            }) {
                return None;
            }
            let path =
                normalize_absolute_path(&canonical_footprint(target.path.as_deref()?)).ok()?;
            let owner = normalize_absolute_path(root).ok()?;
            Some((
                path.split('/')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                owner,
                index,
            ))
        })
        .collect::<Vec<_>>();
    located.sort_unstable();
    let mut cursor = 0;
    while cursor < located.len() {
        let mut end = cursor + 1;
        while end < located.len() && located[end].0.starts_with(&located[cursor].0) {
            end += 1;
        }
        if located[cursor..end]
            .iter()
            .any(|(_, owner, _)| owner != &located[cursor].1)
        {
            for (path, owner, index) in &located[cursor..end] {
                let target = &mut targets[*index];
                target.id = TargetId::new(format!(
                    "{}:{}:{owner}{}",
                    RUST_SHARED_TARGET_RULE_DOC.id,
                    owner.len(),
                    path.join("/")
                ));
                target.action = CleanAction::NoopInspectOnly;
                target.risk = RUST_SHARED_TARGET_RULE_DOC.risk;
                target.reversible = true;
                target.selected_by_default = false;
                target.evidence.retain(|item| {
                    !matches!(item,
                        Evidence::RuleMatched { rule_id } if rule_id == RUST_TARGET_RULE_DOC.id
                    ) && !matches!(item, Evidence::OfficialCommand { .. })
                });
                let matched = Evidence::RuleMatched {
                    rule_id: RUST_SHARED_TARGET_RULE_DOC.id.into(),
                };
                if !target.evidence.contains(&matched) {
                    target.evidence.insert(0, matched);
                }
            }
        }
        cursor = end;
    }
}

fn same_cached_footprint(
    left: &Option<PathBuf>,
    right: &Option<PathBuf>,
    left_target: &CleanTarget,
    right_target: &CleanTarget,
) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left == right,
        (None, None) => left_target.id == right_target.id,
        _ => false,
    }
}

fn parent_cached_covers(parent: &Option<PathBuf>, child: &Option<PathBuf>) -> bool {
    match (parent, child) {
        (Some(parent), Some(child)) => child != parent && child.starts_with(parent),
        _ => false,
    }
}

fn merge_unique_evidence(existing: &mut CleanTarget, evidence: Vec<Evidence>) {
    for item in evidence {
        if !existing.evidence.contains(&item) {
            existing.evidence.push(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::model::{
        CleanAction, CleanTarget, Ecosystem, RiskLevel, Scope, TargetId, TargetKind,
    };

    fn sibling(index: usize) -> CleanTarget {
        let root = PathBuf::from(format!("C:\\isolated\\p{index:05}"));
        let path = root.join(".pytest_cache");
        CleanTarget {
            id: TargetId::new(format!("python.pytest_cache:{index}")),
            scope: Scope::Project { root },
            ecosystem: Ecosystem::Python,
            kind: TargetKind::TestCache,
            path: Some(path.clone()),
            estimated_bytes: 1,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default: true,
            evidence: Vec::new(),
            action: CleanAction::MoveToTrash { path },
        }
    }

    #[test]
    fn action_bucket_key_matches_same_action_identity() {
        let left = sibling(1);
        let right = sibling(2);
        assert!(!same_action_identity(&left, &right));
        assert_ne!(action_bucket_key(&left), action_bucket_key(&right));
        let mut same = sibling(1);
        same.id = TargetId::new("other-id");
        assert!(same_action_identity(&left, &same));
        assert_eq!(action_bucket_key(&left), action_bucket_key(&same));
    }

    #[test]
    fn distinct_project_caches_stay_separate_without_a_quadratic_scan() {
        let targets: Vec<_> = (0..4_000).map(sibling).collect();
        let started = Instant::now();
        let deduped = dedupe_targets(targets);
        let elapsed = started.elapsed();
        assert_eq!(deduped.len(), 4_000);
        assert!(
            elapsed < Duration::from_secs(2),
            "4000 distinct project caches took {elapsed:?}"
        );
    }
}
