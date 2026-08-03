use crate::model::{Ecosystem, RiskLevel, TargetKind};

use super::{GlobalCacheRule, KnownCacheAction, ProjectDirRule, ProjectMarker, RuleDoc, RuleScope};

pub(crate) const GRADLE_CACHES_RULE_ID: &str = "gradle.caches";
pub(crate) const GRADLE_WRAPPER_DISTS_RULE_ID: &str = "gradle.wrapper_dists";
pub(crate) const GO_MODCACHE_DIRECTORY_RULE_ID: &str = "go.mod_cache";
pub(crate) const NUGET_PACKAGES_RULE_ID: &str = "nuget.packages";
pub(crate) const RUST_TARGET_METADATA_FALLBACK_RULE_ID: &str =
    "rust.target.metadata_fallback_trash";

pub(crate) const RUST_TARGET_RULE_DOC: RuleDoc = RuleDoc {
    id: "rust.target",
    ecosystem: Ecosystem::Rust,
    scope: RuleScope::Project,
    risk: RiskLevel::Low,
    action: "cargo clean",
    summary: "Rust build directory (target/) via cargo clean",
};

pub(crate) const PYCACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "python.__pycache__",
    ecosystem: Ecosystem::Python,
    scope: RuleScope::Project,
    risk: RiskLevel::Low,
    action: "trash",
    summary: "Python __pycache__ directories",
};

pub(crate) const NPM_CACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "npm.cache.clean",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "npm cache via npm cache clean --force",
};

pub(crate) const PIP_CACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "pip.cache.purge",
    ecosystem: Ecosystem::Python,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "pip cache via pip cache purge",
};

pub(crate) const PNPM_STORE_RULE_DOC: RuleDoc = RuleDoc {
    id: "pnpm.store.prune",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "pnpm store via pnpm store prune",
};

/// Base yarn doc; the catalogue also lists classic/modern variants explicitly.
pub(crate) const YARN_CACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "yarn.cache.clean",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "yarn cache via yarn cache clean (see classic/modern variants)",
};

pub(crate) const YARN_CACHE_CLASSIC_RULE_DOC: RuleDoc = RuleDoc {
    id: "yarn.cache.clean.classic",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "yarn classic cache via yarn cache clean",
};

pub(crate) const YARN_CACHE_MODERN_RULE_DOC: RuleDoc = RuleDoc {
    id: "yarn.cache.clean.modern",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "yarn berry cache via yarn cache clean --mirror",
};

pub(crate) const CARGO_HOME_RULE_DOC: RuleDoc = RuleDoc {
    id: "cargo.home.inspect",
    ecosystem: Ecosystem::Rust,
    scope: RuleScope::Global,
    risk: RiskLevel::High,
    action: "inspect only",
    summary: "Cargo home (~/.cargo) — inspect only, never auto-deleted",
};

pub(crate) const GO_MODCACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "go.mod_cache.clean",
    ecosystem: Ecosystem::Generic,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "Go module cache via go clean -modcache",
};

pub(crate) const DOCKER_RULE_DOC: RuleDoc = RuleDoc {
    id: "docker",
    ecosystem: Ecosystem::Docker,
    scope: RuleScope::Global,
    risk: RiskLevel::High,
    action: "deferred",
    summary: "Docker prune — planned, not yet scanned",
};

#[cfg(test)]
pub(crate) const PROJECT_PROCEDURAL_RULE_DOCS: &[RuleDoc] =
    &[RUST_TARGET_RULE_DOC, PYCACHE_RULE_DOC];

pub(crate) const PROVIDER_RULE_DOCS: &[RuleDoc] = &[
    NPM_CACHE_RULE_DOC,
    PIP_CACHE_RULE_DOC,
    PNPM_STORE_RULE_DOC,
    YARN_CACHE_RULE_DOC,
    YARN_CACHE_CLASSIC_RULE_DOC,
    YARN_CACHE_MODERN_RULE_DOC,
    CARGO_HOME_RULE_DOC,
    GO_MODCACHE_RULE_DOC,
];

pub(crate) const AUTHORIZED_GLOBAL_COMMAND_RULE_DOCS: &[RuleDoc] = &[
    NPM_CACHE_RULE_DOC,
    PIP_CACHE_RULE_DOC,
    PNPM_STORE_RULE_DOC,
    YARN_CACHE_CLASSIC_RULE_DOC,
    YARN_CACHE_MODERN_RULE_DOC,
    CARGO_HOME_RULE_DOC,
];

pub(crate) const PROJECT_DIR_RULES: &[ProjectDirRule] = &[
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

pub(crate) const GLOBAL_CACHE_RULES: &[GlobalCacheRule] = &[
    GlobalCacheRule {
        id: GRADLE_CACHES_RULE_ID,
        label: "gradle caches",
        ecosystem: Ecosystem::Generic,
        relative: ".gradle/caches",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
    GlobalCacheRule {
        id: GRADLE_WRAPPER_DISTS_RULE_ID,
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
        risk: RiskLevel::High,
        action: KnownCacheAction::InspectOnly,
    },
    GlobalCacheRule {
        id: GO_MODCACHE_DIRECTORY_RULE_ID,
        label: "go module cache",
        ecosystem: Ecosystem::Generic,
        relative: "go/pkg/mod",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::High,
        action: KnownCacheAction::InspectOnly,
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
        id: NUGET_PACKAGES_RULE_ID,
        label: "NuGet global packages",
        ecosystem: Ecosystem::Generic,
        relative: ".nuget/packages",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::Medium,
        action: KnownCacheAction::Trash,
    },
];

#[cfg(windows)]
pub(crate) const GLOBAL_CACHE_RULES_OS: &[GlobalCacheRule] = &[
    GlobalCacheRule {
        id: "jetbrains.caches",
        label: "JetBrains IDE caches (vendor root inspect)",
        ecosystem: Ecosystem::Generic,
        relative: "AppData/Local/JetBrains",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::High,
        action: KnownCacheAction::InspectOnly,
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

#[cfg(target_os = "macos")]
pub(crate) const GLOBAL_CACHE_RULES_OS: &[GlobalCacheRule] = &[
    GlobalCacheRule {
        id: "jetbrains.caches",
        label: "JetBrains IDE caches (vendor root inspect)",
        ecosystem: Ecosystem::Generic,
        relative: "Library/Caches/JetBrains",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::High,
        action: KnownCacheAction::InspectOnly,
    },
    GlobalCacheRule {
        id: "huggingface.hub",
        label: "HuggingFace hub models",
        ecosystem: Ecosystem::Generic,
        relative: "Library/Caches/huggingface",
        kind: TargetKind::PackageCache,
        risk: RiskLevel::High,
        action: KnownCacheAction::InspectOnly,
    },
];

#[cfg(all(not(windows), not(target_os = "macos")))]
pub(crate) const GLOBAL_CACHE_RULES_OS: &[GlobalCacheRule] = &[
    GlobalCacheRule {
        id: "jetbrains.caches",
        label: "JetBrains IDE caches (vendor root inspect)",
        ecosystem: Ecosystem::Generic,
        relative: ".cache/JetBrains",
        kind: TargetKind::ToolCache,
        risk: RiskLevel::High,
        action: KnownCacheAction::InspectOnly,
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
