//! Cleanup rule declarations, catalogue presentation, and trusted action
//! reconstruction.
//!
//! Discovery modules consume facts defined here. The private registry resolves
//! persisted intents into trusted actions for plan validation; this module
//! never imports scanner or provider implementations.

mod definitions;
mod registry;

use crate::model::{Ecosystem, RiskLevel, TargetKind};

pub use definitions::PYCACHE_RULE_DOC;
pub(crate) use definitions::{
    CARGO_HOME_RULE_DOC, GLOBAL_CACHE_RULES, GLOBAL_CACHE_RULES_OS, GO_MODCACHE_DIRECTORY_RULE_ID,
    GO_MODCACHE_RULE_DOC, GRADLE_CACHES_RULE_ID, GRADLE_WRAPPER_DISTS_RULE_ID, NPM_CACHE_RULE_DOC,
    NUGET_PACKAGES_RULE_ID, PIP_CACHE_RULE_DOC, PNPM_STORE_RULE_DOC, PROJECT_DIR_RULES,
    RUST_TARGET_METADATA_FALLBACK_RULE_ID, RUST_TARGET_RULE_DOC, YARN_CACHE_CLASSIC_RULE_DOC,
    YARN_CACHE_MODERN_RULE_DOC,
};
pub(crate) use registry::{ActionSpec, intent_from_scan_action, resolve_action};

/// Which project marker gates a project-level directory rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectMarker {
    Node,
    Python,
}

/// A marker-gated project cache directory rule.
#[derive(Debug, Clone)]
pub(crate) struct ProjectDirRule {
    pub id: &'static str,
    pub label: &'static str,
    pub ecosystem: Ecosystem,
    pub marker: ProjectMarker,
    /// Relative to the project directory, `/`-separated.
    pub relative: &'static str,
    pub kind: TargetKind,
    pub risk: RiskLevel,
    pub selected_by_default: bool,
}

/// Action for a known global cache directory that has no official CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KnownCacheAction {
    /// Move the whole directory to the OS trash (reversible).
    Trash,
    /// Never auto-delete; surface for manual inspection only.
    InspectOnly,
}

/// A known home-relative cache directory rule with no official CLI.
#[derive(Debug, Clone)]
pub(crate) struct GlobalCacheRule {
    pub id: &'static str,
    pub label: &'static str,
    pub ecosystem: Ecosystem,
    /// Relative to the user's home directory, `/`-separated.
    pub relative: &'static str,
    pub kind: TargetKind,
    pub risk: RiskLevel,
    pub action: KnownCacheAction,
}

/// Scope of a catalogued rule, for display grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope {
    /// Project-level cleanup rule.
    Project,
    /// Machine-level provider or cache rule.
    Global,
}

/// A flattened, display-facing description of one rule.
#[derive(Debug, Clone)]
pub struct RuleDoc {
    /// Stable built-in rule identifier.
    pub id: &'static str,
    /// Ecosystem associated with the rule.
    pub ecosystem: Ecosystem,
    /// Whether the rule applies to projects or global providers.
    pub scope: RuleScope,
    /// User-facing risk assigned by the rule.
    pub risk: RiskLevel,
    /// Short action label for display.
    pub action: &'static str,
    /// Short user-facing rule description.
    pub summary: &'static str,
}

/// Project directory rules gated by the given marker.
pub(crate) fn project_dir_rules(
    marker: ProjectMarker,
) -> impl Iterator<Item = &'static ProjectDirRule> {
    PROJECT_DIR_RULES
        .iter()
        .filter(move |rule| rule.marker == marker)
}

/// All global cache rules for the current platform.
pub(crate) fn global_cache_rules() -> impl Iterator<Item = &'static GlobalCacheRule> {
    GLOBAL_CACHE_RULES
        .iter()
        .chain(GLOBAL_CACHE_RULES_OS.iter())
}

pub(crate) fn is_known_global_command_rule(rule_id: &str) -> bool {
    rule_id == RUST_TARGET_RULE_DOC.id
        || definitions::AUTHORIZED_GLOBAL_COMMAND_RULE_DOCS
            .iter()
            .any(|doc| doc.id == rule_id)
}

/// One flat, display-facing list of every cleanup rule.
pub fn rule_catalogue() -> Vec<RuleDoc> {
    let mut docs = Vec::new();

    docs.push(RUST_TARGET_RULE_DOC);

    for rule in PROJECT_DIR_RULES {
        docs.push(RuleDoc {
            id: rule.id,
            ecosystem: rule.ecosystem.clone(),
            scope: RuleScope::Project,
            risk: rule.risk.clone(),
            action: "trash",
            summary: rule.label,
        });
    }

    docs.push(PYCACHE_RULE_DOC);
    docs.extend(definitions::PROVIDER_RULE_DOCS.iter().cloned());

    for rule in global_cache_rules() {
        let action = match rule.action {
            KnownCacheAction::Trash => "trash",
            KnownCacheAction::InspectOnly => "inspect only",
        };
        docs.push(RuleDoc {
            id: rule.id,
            ecosystem: rule.ecosystem.clone(),
            scope: RuleScope::Global,
            risk: rule.risk.clone(),
            action,
            summary: rule.label,
        });
    }

    docs.push(definitions::DOCKER_RULE_DOC);
    docs
}

/// One display row shared by the CLI and TUI rule views.
pub fn rule_row(doc: &RuleDoc) -> String {
    format!(
        "{:<22} {:<9} {:<16} {}",
        doc.id,
        risk_label(&doc.risk),
        doc.action,
        doc.summary
    )
}

/// Human-facing label for a risk level.
pub fn risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "Low",
        RiskLevel::Medium => "Medium",
        RiskLevel::High => "High",
        RiskLevel::Dangerous => "Dangerous",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn catalogue_ids_are_unique() {
        let mut seen = HashSet::new();
        for doc in rule_catalogue() {
            assert!(seen.insert(doc.id), "duplicate catalogue id {}", doc.id);
        }
    }

    #[test]
    fn catalogue_has_no_empty_fields() {
        for doc in rule_catalogue() {
            assert!(!doc.id.is_empty(), "empty id");
            assert!(!doc.action.is_empty(), "empty action for {}", doc.id);
            assert!(!doc.summary.is_empty(), "empty summary for {}", doc.id);
        }
    }

    #[test]
    fn catalogue_covers_all_declared_docs() {
        let ids: HashSet<_> = rule_catalogue().into_iter().map(|doc| doc.id).collect();
        let declared = definitions::PROJECT_PROCEDURAL_RULE_DOCS
            .iter()
            .chain(definitions::PROVIDER_RULE_DOCS)
            .map(|doc| doc.id)
            .chain(PROJECT_DIR_RULES.iter().map(|rule| rule.id))
            .chain(global_cache_rules().map(|rule| rule.id));
        for id in declared {
            assert!(ids.contains(id), "catalogue missing declared rule {id}");
        }
    }

    #[test]
    fn project_dir_rules_filter_by_marker() {
        assert!(
            project_dir_rules(ProjectMarker::Node).all(|rule| rule.marker == ProjectMarker::Node)
        );
        assert!(
            project_dir_rules(ProjectMarker::Python)
                .all(|rule| rule.marker == ProjectMarker::Python)
        );
        assert_eq!(project_dir_rules(ProjectMarker::Node).count(), 4);
        assert_eq!(project_dir_rules(ProjectMarker::Python).count(), 6);
    }

    #[test]
    fn global_cache_rules_are_unique_and_cover_new_ecosystems() {
        let mut seen = HashSet::new();
        for rule in global_cache_rules() {
            assert!(!rule.label.is_empty(), "empty label for {}", rule.id);
            assert!(
                seen.insert(rule.id),
                "duplicate global cache id {}",
                rule.id
            );
        }
        for expected in ["gradle.caches", "maven.repository", "go.mod_cache"] {
            assert!(
                seen.contains(expected),
                "missing new global cache {expected}"
            );
        }
    }

    #[test]
    fn trash_rules_are_reversible_by_design_and_inspect_rules_are_high_risk() {
        for rule in global_cache_rules() {
            if rule.action == KnownCacheAction::InspectOnly {
                assert!(
                    matches!(rule.risk, RiskLevel::High | RiskLevel::Dangerous),
                    "{} is inspect-only but not high risk",
                    rule.id
                );
            }
        }
    }

    #[test]
    fn maven_jetbrains_and_go_are_not_whole_tree_trash() {
        for rule in global_cache_rules() {
            if matches!(
                rule.id,
                "maven.repository" | "jetbrains.caches" | "go.mod_cache"
            ) {
                assert_eq!(
                    rule.action,
                    KnownCacheAction::InspectOnly,
                    "{} must not trash a coarse vendor/repo tree",
                    rule.id
                );
            }
        }
    }

    #[test]
    fn provider_docs_include_yarn_classic_and_modern_variants() {
        let ids: HashSet<_> = definitions::PROVIDER_RULE_DOCS
            .iter()
            .map(|doc| doc.id)
            .collect();
        assert!(ids.contains(YARN_CACHE_CLASSIC_RULE_DOC.id));
        assert!(ids.contains(YARN_CACHE_MODERN_RULE_DOC.id));
        assert!(ids.contains(GO_MODCACHE_RULE_DOC.id));
        assert_eq!(
            PNPM_STORE_RULE_DOC.risk,
            RiskLevel::Medium,
            "pnpm catalogue risk is the single source"
        );
    }

    #[test]
    fn os_cache_roots_are_platform_specific() {
        let jetbrains = global_cache_rules()
            .find(|rule| rule.id == "jetbrains.caches")
            .expect("jetbrains rule");
        #[cfg(windows)]
        assert!(
            jetbrains.relative.contains("AppData"),
            "windows jetbrains path: {}",
            jetbrains.relative
        );
        #[cfg(target_os = "macos")]
        assert!(
            jetbrains.relative.contains("Library/Caches"),
            "macos jetbrains path: {}",
            jetbrains.relative
        );
        #[cfg(all(not(windows), not(target_os = "macos")))]
        assert!(
            jetbrains.relative.starts_with(".cache/"),
            "linux jetbrains path: {}",
            jetbrains.relative
        );
    }
}
