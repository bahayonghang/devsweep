use std::path::{Path, PathBuf};

use crate::model::{CleanAction, CleanTarget, Evidence};

pub(in crate::scan) fn dedupe_targets(mut targets: Vec<CleanTarget>) -> Vec<CleanTarget> {
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
