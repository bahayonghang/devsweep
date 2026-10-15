use std::path::{Path, PathBuf};

use crate::{
    filesystem::normalize_absolute_path,
    model::{CleanAction, CleanTarget, Evidence, Scope, TargetId},
    rules::{RUST_SHARED_TARGET_RULE_DOC, RUST_TARGET_RULE_DOC},
};

pub(in crate::scan) fn dedupe_targets(mut targets: Vec<CleanTarget>) -> Vec<CleanTarget> {
    mark_shared_rust_targets(&mut targets);
    targets.sort_by(|left, right| {
        let left_path = left.path.as_deref().map(footprint_depth);
        let right_path = right.path.as_deref().map(footprint_depth);
        left_path
            .cmp(&right_path)
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });

    let mut kept: Vec<CleanTarget> = Vec::new();
    for target in targets {
        if let Some(existing) = kept.iter_mut().find(|existing| {
            same_action_identity(existing, &target) && same_footprint(existing, &target)
        }) {
            merge_unique_evidence(existing, target.evidence);
            continue;
        }

        let is_nested_with_same_action = kept.iter().any(|existing| {
            same_action_identity(existing, &target) && parent_footprint_covers(existing, &target)
        });
        if is_nested_with_same_action {
            continue;
        }

        kept.push(target);
    }

    kept
}

pub(super) fn footprint_depth(path: &Path) -> usize {
    canonical_footprint(path).components().count()
}

fn canonical_footprint(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

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

fn same_footprint(left: &CleanTarget, right: &CleanTarget) -> bool {
    match (&left.path, &right.path) {
        (Some(left), Some(right)) => canonical_footprint(left) == canonical_footprint(right),
        (None, None) => left.id == right.id,
        _ => false,
    }
}

fn parent_footprint_covers(parent: &CleanTarget, child: &CleanTarget) -> bool {
    match (&parent.path, &child.path) {
        (Some(parent), Some(child)) => {
            let parent = canonical_footprint(parent);
            let child = canonical_footprint(child);
            child != parent && child.starts_with(parent)
        }
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
