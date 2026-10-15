//! Pure, non-authoritative logical occupancy projection over completed observations.
//! Paths are normalized lexically; this does not prove live file identity, resolve
//! links, or account for shared physical allocation. Cleanup rows remain unchanged.

use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    filesystem::normalize_absolute_path,
    model::{Scope, TargetKind, UntrustedTarget},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FootprintCategory {
    Artifact,
    Dependency,
    Other,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FootprintAttribution {
    Owned,
    Shared,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectFootprint {
    /// Original report root spelling, one per normalized owner, in stable order.
    pub owner_roots: Vec<PathBuf>,
    /// All report rows retained for explaining a merged or excluded footprint.
    pub target_indices: Vec<usize>,
    pub category: FootprintCategory,
    /// A logical lower bound when incomplete; never promised physical reclaim.
    pub logical_bytes: u64,
    pub size_complete: bool,
    pub attribution: FootprintAttribution,
}

/// Groups equal/nested report paths regardless of cleanup method. Shared/global
/// overlaps and invalid paths remain visible but cannot enter an owner's total.
/// No filesystem access or command execution is performed, including on replay.
pub(crate) fn project_footprints(targets: &[UntrustedTarget]) -> Vec<ProjectFootprint> {
    let mut located = Vec::new();
    let mut result = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        let path = target.path.as_deref().and_then(|path| {
            path.to_str()?;
            normalize_absolute_path(path).ok()
        });
        if let Some(path) = path {
            // Component sorting places ancestors and descendants together even
            // when a sibling's name contains punctuation before '/'.
            located.push((
                path.split('/')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                index,
            ));
        } else if matches!(target.scope, Scope::Project { .. }) {
            let mut footprint = summarize(targets, &[index]);
            footprint.attribution = FootprintAttribution::Unresolved;
            footprint.size_complete = false;
            result.push(footprint);
        }
    }
    located.sort_unstable();
    let mut cursor = 0;
    while cursor < located.len() {
        let root = &located[cursor].0;
        let mut end = cursor + 1;
        while end < located.len() && located[end].0.starts_with(root) {
            end += 1;
        }
        let indices = located[cursor..end]
            .iter()
            .map(|(_, index)| *index)
            .collect::<Vec<_>>();
        if indices
            .iter()
            .any(|index| matches!(targets[*index].scope, Scope::Project { .. }))
        {
            let mut footprint = summarize(targets, &indices);
            let (logical_bytes, complete) = measure_group(targets, &located[cursor..end]);
            footprint.logical_bytes = logical_bytes;
            footprint.size_complete =
                footprint.attribution == FootprintAttribution::Owned && complete;
            result.push(footprint);
        }
        cursor = end;
    }
    result
}

struct Measurement<'a> {
    path: &'a [String],
    observed: u64,
    complete: bool,
    children: u64,
    overflowed: bool,
}

fn finish_measurement(stack: &mut Vec<Measurement<'_>>) -> (u64, bool) {
    let frame = stack.pop().expect("nonempty measurement stack");
    let bytes = frame.observed.max(frame.children);
    if let Some(parent) = stack.last_mut() {
        parent.overflowed |= frame.overflowed || parent.children.checked_add(bytes).is_none();
        parent.children = parent.children.saturating_add(bytes);
    }
    (
        bytes,
        frame.complete && !frame.overflowed && frame.observed >= frame.children,
    )
}

fn measure_group(targets: &[UntrustedTarget], located: &[(Vec<String>, usize)]) -> (u64, bool) {
    let mut stack: Vec<Measurement<'_>> = Vec::new();
    let mut cursor = 0;
    while cursor < located.len() {
        let path = &located[cursor].0;
        while stack
            .last()
            .is_some_and(|parent| !path.starts_with(parent.path))
        {
            finish_measurement(&mut stack);
        }
        let mut end = cursor;
        let mut observed = 0;
        let mut complete_size = None;
        let mut conflicting = false;
        while end < located.len() && &located[end].0 == path {
            let target = &targets[located[end].1];
            observed = observed.max(target.estimated_bytes);
            if target.size_complete {
                conflicting |= complete_size.is_some_and(|size| size != target.estimated_bytes);
                complete_size = Some(target.estimated_bytes);
            }
            end += 1;
        }
        stack.push(Measurement {
            path,
            observed,
            complete: !conflicting && complete_size == Some(observed),
            children: 0,
            overflowed: false,
        });
        cursor = end;
    }
    while stack.len() > 1 {
        finish_measurement(&mut stack);
    }
    finish_measurement(&mut stack)
}

fn summarize(targets: &[UntrustedTarget], indices: &[usize]) -> ProjectFootprint {
    let mut owners = BTreeMap::new();
    let mut unresolved = false;
    let mut global = false;
    let mut shared_owner = false;
    let mut category = category(&targets[indices[0]].kind);
    let mut logical_bytes = 0;
    for index in indices {
        let target = &targets[*index];
        shared_owner |= target.rule_id == crate::rules::RUST_SHARED_TARGET_RULE_DOC.id;
        match &target.scope {
            Scope::Project { root } => match root
                .to_str()
                .and_then(|_| normalize_absolute_path(root).ok())
            {
                Some(key) => {
                    owners.entry(key).or_insert_with(|| root.clone());
                }
                None => {
                    unresolved = true;
                    owners.insert(format!("unresolved:{index}"), root.clone());
                }
            },
            Scope::Global => global = true,
        }
        if category != self::category(&target.kind) {
            category = FootprintCategory::Mixed;
        }
        logical_bytes = logical_bytes.max(target.estimated_bytes);
    }
    let attribution = if unresolved || category == FootprintCategory::Mixed {
        FootprintAttribution::Unresolved
    } else if global || shared_owner || owners.len() != 1 {
        FootprintAttribution::Shared
    } else {
        FootprintAttribution::Owned
    };
    ProjectFootprint {
        owner_roots: owners.into_values().collect(),
        target_indices: indices.to_vec(),
        category,
        logical_bytes,
        size_complete: false,
        attribution,
    }
}

fn category(kind: &TargetKind) -> FootprintCategory {
    match kind {
        TargetKind::BuildArtifacts | TargetKind::TestCache | TargetKind::ToolCache => {
            FootprintCategory::Artifact
        }
        TargetKind::DependencyDirectory | TargetKind::VirtualEnv => FootprintCategory::Dependency,
        TargetKind::PackageCache => FootprintCategory::Other,
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{CleanupIntent, Ecosystem, RiskLevel, TargetId};

    use super::*;

    fn base() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(r"Q:\unavailable-fixture")
        } else {
            PathBuf::from("/unavailable-fixture")
        }
    }

    fn target(id: &str, path: &str, bytes: u64) -> UntrustedTarget {
        UntrustedTarget {
            id: TargetId::new(id),
            rule_id: "rust.target".into(),
            scope: Scope::Project {
                root: base().join("workspace"),
            },
            ecosystem: Ecosystem::Rust,
            kind: TargetKind::BuildArtifacts,
            path: Some(base().join(path)),
            estimated_bytes: bytes,
            size_complete: true,
            sizing_warnings: vec![],
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: false,
            selected_by_default: false,
            evidence: vec![],
            intent: CleanupIntent::InspectOnly {
                rule_id: "rust.target".into(),
            },
        }
    }

    #[test]
    fn counts_equal_paths_once_without_changing_actions_or_selection() {
        let first = target("cargo", "cache/target", 100);
        let mut second = target("other-action", "cache/./target", 100);
        second.intent = CleanupIntent::RunBuiltInAction {
            provider_id: "rust".into(),
            action_id: "clean".into(),
        };
        second.selected_by_default = true;
        let targets = vec![first, second];
        let before = targets.clone();
        let footprints = project_footprints(&targets);
        assert_eq!(targets, before);
        assert_eq!(footprints.len(), 1);
        assert_eq!(footprints[0].logical_bytes, 100);
        assert!(footprints[0].size_complete);
        assert_eq!(footprints[0].target_indices.len(), 2);
        assert_eq!(footprints[0].attribution, FootprintAttribution::Owned);
    }

    #[test]
    fn combines_nested_paths_but_keeps_similar_siblings_separate() {
        let targets = vec![
            target("nested", "cache/target/debug", 80),
            target("sibling", "cache/target-2", 30),
            target("outer", "cache/target", 100),
        ];
        let footprints = project_footprints(&targets);
        assert_eq!(footprints.len(), 2);
        assert_eq!(
            footprints.iter().map(|row| row.logical_bytes).sum::<u64>(),
            130
        );
        assert!(footprints.iter().all(|row| row.size_complete));
    }

    #[test]
    fn disjoint_children_supply_lower_bound_without_claiming_complete_parent() {
        let mut outer = target("outer", "cache", 0);
        outer.size_complete = false;
        let targets = vec![
            outer,
            target("a", "cache/a", 80),
            target("b", "cache/b", 70),
        ];
        let footprint = &project_footprints(&targets)[0];
        assert_eq!(footprint.logical_bytes, 150);
        assert!(!footprint.size_complete);
        let mut contradictory = targets.clone();
        contradictory[0].size_complete = true;
        contradictory[0].estimated_bytes = 100;
        assert!(!project_footprints(&contradictory)[0].size_complete);
    }

    #[test]
    fn conflicting_duplicate_measurements_are_partial_in_either_order() {
        let mut targets = vec![target("a", "cache", 100), target("b", "cache", 80)];
        for _ in 0..2 {
            let footprint = &project_footprints(&targets)[0];
            assert_eq!(footprint.logical_bytes, 100);
            assert!(!footprint.size_complete);
            targets.reverse();
        }
    }

    #[test]
    fn complete_duplicate_can_cover_a_matching_partial_measurement() {
        let mut partial = target("partial", "cache", 30);
        partial.size_complete = false;
        let footprint = &project_footprints(&[partial, target("complete", "cache", 100)])[0];
        assert_eq!(footprint.logical_bytes, 100);
        assert!(footprint.size_complete);
    }

    #[test]
    fn two_workspace_owners_or_global_overlap_are_never_attributed_to_each_project() {
        let first = target("a", "cache", 100);
        let mut second = target("b", "cache/debug", 80);
        second.scope = Scope::Project {
            root: base().join("other-workspace"),
        };
        let footprint = &project_footprints(&[first.clone(), second.clone()])[0];
        assert_eq!(footprint.attribution, FootprintAttribution::Shared);
        assert_eq!(footprint.owner_roots.len(), 2);
        assert!(!footprint.size_complete);
        second.scope = Scope::Global;
        let footprint = &project_footprints(&[first, second])[0];
        assert_eq!(footprint.attribution, FootprintAttribution::Shared);
        assert_eq!(footprint.owner_roots.len(), 1);
        let mut known_shared = target("known-shared", "alias/cache", 100);
        known_shared.rule_id = crate::rules::RUST_SHARED_TARGET_RULE_DOC.id.to_string();
        let footprint = &project_footprints(&[known_shared])[0];
        assert_eq!(footprint.attribution, FootprintAttribution::Shared);
        assert!(!footprint.size_complete);
    }

    #[test]
    fn global_shared_storage_is_omitted_and_dependencies_stay_separate() {
        let mut global = target("shared", "actions", 1_000);
        global.scope = Scope::Global;
        let mut dependency = target("dependency", "workspace/node_modules", 200);
        dependency.kind = TargetKind::DependencyDirectory;
        let footprints =
            project_footprints(&[global, dependency, target("build", "workspace/dist", 50)]);
        assert_eq!(footprints.len(), 2);
        assert_eq!(
            footprints
                .iter()
                .filter(|row| row.category == FootprintCategory::Artifact)
                .map(|row| row.logical_bytes)
                .sum::<u64>(),
            50
        );
        assert!(
            footprints
                .iter()
                .any(|row| row.category == FootprintCategory::Dependency)
        );
    }

    #[test]
    fn ambiguous_category_and_unlocated_paths_do_not_claim_exact_occupancy() {
        let first = target("a", "cache", 100);
        let mut dependency = target("b", "cache/deps", 50);
        dependency.kind = TargetKind::VirtualEnv;
        let footprint = &project_footprints(&[first.clone(), dependency])[0];
        assert_eq!(footprint.category, FootprintCategory::Mixed);
        assert_eq!(footprint.attribution, FootprintAttribution::Unresolved);
        for path in [None, Some(PathBuf::from("relative/path"))] {
            let mut unresolved = first.clone();
            unresolved.path = path;
            assert_eq!(
                project_footprints(&[unresolved])[0].attribution,
                FootprintAttribution::Unresolved
            );
        }
    }

    #[test]
    fn many_report_rows_remain_linear_after_sorting() {
        let targets = (0..10_000)
            .map(|index| target(&index.to_string(), &format!("cache/{index}"), 1))
            .collect::<Vec<_>>();
        let footprints = project_footprints(&targets);
        assert_eq!(footprints.len(), 10_000);
        assert_eq!(
            footprints.iter().map(|row| row.logical_bytes).sum::<u64>(),
            10_000
        );
    }

    #[test]
    fn volume_root_footprint_contains_descendants_on_the_current_platform() {
        let mut outer = target("outer", "unused", 100);
        outer.path = base().ancestors().last().map(PathBuf::from);
        let footprints = project_footprints(&[outer, target("child", "cache", 40)]);
        assert_eq!(footprints.len(), 1);
        assert_eq!(footprints[0].logical_bytes, 100);
        assert!(footprints[0].size_complete);
    }

    #[test]
    fn overflowing_disjoint_children_cannot_claim_a_complete_total() {
        let outer = target("outer", "cache", u64::MAX);
        let footprints = project_footprints(&[
            outer,
            target("a", "cache/a", u64::MAX),
            target("b", "cache/b", 1),
        ]);
        assert_eq!(footprints[0].logical_bytes, u64::MAX);
        assert!(!footprints[0].size_complete);
    }
}
