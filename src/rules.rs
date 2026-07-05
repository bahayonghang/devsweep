//! Declarative catalogue of cleanup rules.
//!
//! devsweep rules come in four shapes. Two are pure data and live here as
//! tables that the scanner and providers consume:
//!
//! - **Project directory rules** ([`PROJECT_DIR_RULES`]): "marker file present
//!   → relative cache directory is trashable" (Node / Python).
//! - **Global cache rules** ([`GLOBAL_CACHE_RULES`]): "known home-relative cache
//!   directory with no official cleanup command" (gradle / maven / go / ...).
//!
//! The other two are procedural and live next to their implementations, which
//! declare their own [`RuleDoc`] constants; [`rule_catalogue`] only aggregates
//! so the `Rules` view lists everything:
//!
//! - **Command project rule**: `cargo clean` (needs a `target/` subdir check),
//!   plus `__pycache__` descent — docs in [`crate::scanner`].
//! - **Command providers**: npm / pip / pnpm / yarn (resolve a tool, run a
//!   command, parse stdout; yarn even branches on version) — docs in
//!   [`crate::providers`].

use crate::model::{Ecosystem, RiskLevel, TargetKind};
use crate::{providers, scanner};

/// Which project marker gates a project-level directory rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectMarker {
    Node,
    Python,
}

/// A "marker file present → relative cache directory is trashable" rule.
///
/// Consumed by the project scanner; one entry per former inline branch.
#[derive(Debug, Clone)]
pub struct ProjectDirRule {
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
pub enum KnownCacheAction {
    /// Move the whole directory to the OS trash (reversible).
    Trash,
    /// Never auto-delete; surface for manual inspection only.
    InspectOnly,
}

/// A "known home-relative cache directory" rule (no official cleanup command).
#[derive(Debug, Clone)]
pub struct GlobalCacheRule {
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
    Project,
    Global,
}

/// A flattened, display-facing description of one rule.
///
/// Aggregates all four rule shapes for the `Rules` tab and `devsweep rules`.
#[derive(Debug, Clone)]
pub struct RuleDoc {
    pub id: &'static str,
    pub ecosystem: Ecosystem,
    pub scope: RuleScope,
    pub risk: RiskLevel,
    pub action: &'static str,
    pub summary: &'static str,
}

/// Project-level directory rules (Node + Python), formerly inline in the scanner.
pub const PROJECT_DIR_RULES: &[ProjectDirRule] = &[
    ProjectDirRule {
        id: "node.node_modules",
        label: "Node dependency directory",
        ecosystem: Ecosystem::Node,
        marker: ProjectMarker::Node,
        relative: "node_modules",
        kind: TargetKind::DependencyDirectory,
        risk: RiskLevel::Medium,
        selected_by_default: false,
    },
    ProjectDirRule {
        id: "node.next_cache",
        label: "Next.js build cache",
        ecosystem: Ecosystem::Node,
        marker: ProjectMarker::Node,
        relative: ".next/cache",
        kind: TargetKind::BuildArtifacts,
        risk: RiskLevel::Low,
        selected_by_default: true,
    },
    ProjectDirRule {
        id: "node.turbo",
        label: "Turborepo cache",
        ecosystem: Ecosystem::Node,
        marker: ProjectMarker::Node,
        relative: ".turbo",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::Low,
        selected_by_default: true,
    },
    ProjectDirRule {
        id: "node.parcel_cache",
        label: "Parcel cache",
        ecosystem: Ecosystem::Node,
        marker: ProjectMarker::Node,
        relative: ".parcel-cache",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::Low,
        selected_by_default: true,
    },
    ProjectDirRule {
        id: "python.venv_dot",
        label: "Python virtualenv (.venv)",
        ecosystem: Ecosystem::Python,
        marker: ProjectMarker::Python,
        relative: ".venv",
        kind: TargetKind::VirtualEnv,
        risk: RiskLevel::Medium,
        selected_by_default: false,
    },
    ProjectDirRule {
        id: "python.venv",
        label: "Python virtualenv (venv)",
        ecosystem: Ecosystem::Python,
        marker: ProjectMarker::Python,
        relative: "venv",
        kind: TargetKind::VirtualEnv,
        risk: RiskLevel::Medium,
        selected_by_default: false,
    },
    ProjectDirRule {
        id: "python.pytest_cache",
        label: "pytest cache",
        ecosystem: Ecosystem::Python,
        marker: ProjectMarker::Python,
        relative: ".pytest_cache",
        kind: TargetKind::TestCache,
        risk: RiskLevel::Low,
        selected_by_default: true,
    },
    ProjectDirRule {
        id: "python.mypy_cache",
        label: "mypy cache",
        ecosystem: Ecosystem::Python,
        marker: ProjectMarker::Python,
        relative: ".mypy_cache",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::Low,
        selected_by_default: true,
    },
    ProjectDirRule {
        id: "python.ruff_cache",
        label: "Ruff cache",
        ecosystem: Ecosystem::Python,
        marker: ProjectMarker::Python,
        relative: ".ruff_cache",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::Low,
        selected_by_default: true,
    },
    ProjectDirRule {
        id: "python.tox",
        label: "tox environments",
        ecosystem: Ecosystem::Python,
        marker: ProjectMarker::Python,
        relative: ".tox",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::Medium,
        selected_by_default: false,
    },
];

/// Cross-platform global caches (home-relative on both unix and Windows).
///
/// All are re-downloadable package caches with no official cleanup command, so
/// they trash the whole directory (reversible). Never auto-selected.
pub const GLOBAL_CACHE_RULES: &[GlobalCacheRule] = &[
    GlobalCacheRule {
        id: "gradle.caches",
        label: "gradle caches",
        ecosystem: Ecosystem::Generic,
        relative: ".gradle/caches",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
    GlobalCacheRule {
        id: "gradle.wrapper_dists",
        label: "gradle wrapper distributions",
        ecosystem: Ecosystem::Generic,
        relative: ".gradle/wrapper/dists",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
    GlobalCacheRule {
        id: "maven.repository",
        label: "maven local repository",
        ecosystem: Ecosystem::Generic,
        relative: ".m2/repository",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
    GlobalCacheRule {
        id: "go.mod_cache",
        label: "go module cache",
        ecosystem: Ecosystem::Generic,
        relative: "go/pkg/mod",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
    GlobalCacheRule {
        id: "ivy.cache",
        label: "ivy cache",
        ecosystem: Ecosystem::Generic,
        relative: ".ivy2/cache",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
    GlobalCacheRule {
        id: "nuget.packages",
        label: "NuGet global packages",
        ecosystem: Ecosystem::Generic,
        relative: ".nuget/packages",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
];

/// OS-specific global caches. Same ids per platform; only one set compiles.
#[cfg(windows)]
pub const GLOBAL_CACHE_RULES_OS: &[GlobalCacheRule] = &[
    GlobalCacheRule {
        id: "jetbrains.caches",
        label: "JetBrains IDE caches",
        ecosystem: Ecosystem::Generic,
        relative: "AppData/Local/JetBrains",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
    GlobalCacheRule {
        id: "huggingface.hub",
        label: "HuggingFace hub models",
        ecosystem: Ecosystem::Generic,
        relative: "AppData/Local/huggingface",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::High,
        action: KnownCacheAction::InspectOnly,
    },
];

/// OS-specific global caches. Same ids per platform; only one set compiles.
#[cfg(not(windows))]
pub const GLOBAL_CACHE_RULES_OS: &[GlobalCacheRule] = &[
    GlobalCacheRule {
        id: "jetbrains.caches",
        label: "JetBrains IDE caches",
        ecosystem: Ecosystem::Generic,
        relative: ".cache/JetBrains",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
    GlobalCacheRule {
        id: "huggingface.hub",
        label: "HuggingFace hub models",
        ecosystem: Ecosystem::Generic,
        relative: ".cache/huggingface",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::High,
        action: KnownCacheAction::InspectOnly,
    },
];

/// Project directory rules gated by the given marker.
pub fn project_dir_rules(marker: ProjectMarker) -> impl Iterator<Item = &'static ProjectDirRule> {
    PROJECT_DIR_RULES
        .iter()
        .filter(move |rule| rule.marker == marker)
}

/// All global cache rules for the current platform.
pub fn global_cache_rules() -> impl Iterator<Item = &'static GlobalCacheRule> {
    GLOBAL_CACHE_RULES
        .iter()
        .chain(GLOBAL_CACHE_RULES_OS.iter())
}

/// One flat, display-facing list of every cleanup rule (all four shapes).
///
/// Pure aggregation: project rules first, then global. Table-driven rules
/// (A/C) are derived from their tables; procedural rules (B/D) contribute the
/// [`RuleDoc`] constants declared next to their implementations.
pub fn rule_catalogue() -> Vec<RuleDoc> {
    let mut docs = Vec::new();

    // B: command project rule, declared in the scanner.
    docs.push(scanner::RUST_TARGET_RULE_DOC);

    // A: project directory rules, derived from the table.
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

    // Discovered during directory descent, not via the marker table.
    docs.push(scanner::PYCACHE_RULE_DOC);

    // D: command providers (procedural), declared next to their implementations.
    docs.extend(providers::PROVIDER_RULE_DOCS.iter().cloned());

    // C: known global cache rules, derived from the table.
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

    // Reserved: Docker support is planned but not yet scanned.
    docs.push(RuleDoc {
        id: "docker",
        ecosystem: Ecosystem::Docker,
        scope: RuleScope::Global,
        risk: RiskLevel::High,
        action: "deferred",
        summary: "Docker prune — planned, not yet scanned",
    });

    docs
}

/// One display row for a rule doc; shared by `devsweep rules` and the TUI
/// Rules tab (the two adapters only differ in where the row is written).
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
        let declared = scanner::SCANNER_RULE_DOCS
            .iter()
            .chain(providers::PROVIDER_RULE_DOCS)
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
}
