use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use tracing::warn;

use crate::fs_size::{estimate_tree, is_unsafe_link};
use crate::model::{
    CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel, Scope, TargetId,
    TargetKind,
};
use crate::process_runner::ProcessRunner;
use crate::safety::query_cargo_metadata;

/// Discovery diagnostic for a nested path that could not be fully scanned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanDiagnostic {
    pub stage: ScanDiagnosticStage,
    pub path: PathBuf,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanDiagnosticStage {
    Discovery,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanCompleteness {
    Complete,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOutcome {
    pub plan: CleanupPlan,
    pub diagnostics: Vec<ScanDiagnostic>,
    pub completeness: ScanCompleteness,
}
use crate::rules::{ProjectMarker, RuleDoc, RuleScope, project_dir_rules};

/// Doc for the procedural `cargo clean` rule; the single source of its identity
/// (id / risk / action / summary), consumed by [`crate::rules::rule_catalogue`].
pub(crate) const RUST_TARGET_RULE_DOC: RuleDoc = RuleDoc {
    id: "rust.target",
    ecosystem: Ecosystem::Rust,
    scope: RuleScope::Project,
    risk: RiskLevel::Low,
    action: "cargo clean",
    summary: "Rust build directory (target/) via cargo clean",
};

/// Doc for the procedural `__pycache__` descent rule; single source of identity.
pub(crate) const PYCACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "python.__pycache__",
    ecosystem: Ecosystem::Python,
    scope: RuleScope::Project,
    risk: RiskLevel::Low,
    action: "trash",
    summary: "Python __pycache__ directories",
};

/// All procedural rule docs declared by this scanner. Unlike the provider
/// docs (extended wholesale into the catalogue), these are pushed one by one
/// to keep the catalogue order, so the slice only feeds the aggregation test.
#[cfg(test)]
pub(crate) const SCANNER_RULE_DOCS: &[RuleDoc] = &[RUST_TARGET_RULE_DOC, PYCACHE_RULE_DOC];

pub struct ProjectScanner;

impl ProjectScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_roots(&self, roots: &[PathBuf]) -> Result<CleanupPlan> {
        Ok(self.scan_roots_with_diagnostics(roots)?.plan)
    }

    pub fn scan_roots_with_diagnostics(&self, roots: &[PathBuf]) -> Result<ScanOutcome> {
        let mut targets = Vec::new();
        let mut diagnostics = Vec::new();

        for root in normalize_scan_roots(roots)? {
            // Root open failures remain hard errors.
            let metadata = fs::symlink_metadata(&root)
                .with_context(|| format!("failed to inspect scan root {}", root.display()))?;
            if !metadata.is_dir() || is_unsafe_link(&metadata) {
                continue;
            }
            fs::read_dir(&root)
                .with_context(|| format!("failed to read scan root {}", root.display()))?;
            self.scan_dir(&root, None, true, &mut targets, &mut diagnostics);
        }

        targets = dedupe_targets(targets);
        let completeness = if diagnostics.is_empty() {
            ScanCompleteness::Complete
        } else {
            ScanCompleteness::Partial
        };

        Ok(ScanOutcome {
            plan: CleanupPlan {
                version: crate::model::CLEANUP_PLAN_VERSION,
                targets,
            },
            diagnostics,
            completeness,
        })
    }

    fn scan_dir(
        &self,
        dir: &Path,
        python_context: Option<&PythonContext>,
        is_scan_root: bool,
        targets: &mut Vec<CleanTarget>,
        diagnostics: &mut Vec<ScanDiagnostic>,
    ) {
        let metadata = match fs::symlink_metadata(dir) {
            Ok(metadata) => metadata,
            Err(error) => {
                if is_scan_root {
                    // Caller already validated roots; treat as nested failure.
                }
                diagnostics.push(ScanDiagnostic {
                    stage: ScanDiagnosticStage::Discovery,
                    path: dir.to_path_buf(),
                    detail: format!("failed to inspect: {error}"),
                });
                return;
            }
        };
        if !metadata.is_dir() || is_unsafe_link(&metadata) {
            return;
        }

        if should_stop_descent(dir) {
            if dir.file_name().and_then(|name| name.to_str()) == Some("__pycache__")
                && let Some(context) = python_context
            {
                targets.push(build_path_target(PathTargetInput {
                    rule_id: PYCACHE_RULE_DOC.id,
                    ecosystem: Ecosystem::Python,
                    kind: TargetKind::TestCache,
                    project_root: context.project_root.clone(),
                    path: dir.to_path_buf(),
                    risk: PYCACHE_RULE_DOC.risk,
                    selected_by_default: true,
                    evidence: vec![
                        Evidence::MarkerFile {
                            path: context.marker.clone(),
                        },
                        Evidence::RuleMatched {
                            rule_id: PYCACHE_RULE_DOC.id.to_string(),
                        },
                    ],
                }));
            }
            return;
        }

        self.scan_rust_project(dir, targets);
        self.scan_node_project(dir, targets);

        let local_python_context = find_python_marker(dir).map(|marker| PythonContext {
            project_root: dir.to_path_buf(),
            marker,
        });
        let active_python_context = local_python_context.as_ref().or(python_context);
        if let Some(context) = active_python_context {
            self.scan_python_project_dir(dir, context, targets);
        }

        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) => {
                diagnostics.push(ScanDiagnostic {
                    stage: ScanDiagnosticStage::Discovery,
                    path: dir.to_path_buf(),
                    detail: format!("failed to read: {error}"),
                });
                return;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    diagnostics.push(ScanDiagnostic {
                        stage: ScanDiagnosticStage::Discovery,
                        path: dir.to_path_buf(),
                        detail: format!("failed to read directory entry: {error}"),
                    });
                    continue;
                }
            };
            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    diagnostics.push(ScanDiagnostic {
                        stage: ScanDiagnosticStage::Discovery,
                        path: path.clone(),
                        detail: format!("failed to inspect: {error}"),
                    });
                    continue;
                }
            };
            if metadata.is_dir() && !is_unsafe_link(&metadata) {
                self.scan_dir(&path, active_python_context, false, targets, diagnostics);
            }
        }
    }

    fn scan_rust_project(&self, dir: &Path, targets: &mut Vec<CleanTarget>) {
        let manifest = dir.join("Cargo.toml");
        if !manifest.is_file() {
            return;
        }

        let runner = ProcessRunner::default();
        // Prefer the absolute manifest path so cargo metadata works even when the
        // process runner uses a neutral working directory for other probes.
        match query_cargo_metadata(&runner, dir) {
            Ok(scope) => {
                let target_dir = scope.target_directory;
                if !is_real_dir(&target_dir) {
                    return;
                }
                let workspace_root = if scope.workspace_root.as_os_str().is_empty() {
                    dir.to_path_buf()
                } else {
                    scope.workspace_root
                };
                let project_root = if workspace_root.join("Cargo.toml").is_file() {
                    workspace_root
                } else {
                    dir.to_path_buf()
                };
                let manifest_path = project_root.join("Cargo.toml");
                let manifest_arg = manifest_path.to_string_lossy().into_owned();
                let target_arg = target_dir.to_string_lossy().into_owned();
                let mut target = build_path_target(PathTargetInput {
                    rule_id: RUST_TARGET_RULE_DOC.id,
                    ecosystem: Ecosystem::Rust,
                    kind: TargetKind::BuildArtifacts,
                    project_root,
                    path: target_dir.clone(),
                    risk: RUST_TARGET_RULE_DOC.risk,
                    selected_by_default: true,
                    evidence: vec![
                        Evidence::MarkerFile {
                            path: manifest_path,
                        },
                        Evidence::OfficialCommand {
                            command: format!(
                                "cargo clean --manifest-path {manifest_arg} --target-dir {target_arg}"
                            ),
                        },
                        Evidence::RuleMatched {
                            rule_id: RUST_TARGET_RULE_DOC.id.to_string(),
                        },
                    ],
                });
                target.reversible = false;
                target.action = CleanAction::Command {
                    program: "cargo".to_string(),
                    args: vec![
                        "clean".to_string(),
                        "--manifest-path".to_string(),
                        manifest_arg,
                        "--target-dir".to_string(),
                        target_arg,
                    ],
                    cwd: None,
                    irreversible: true,
                };
                targets.push(target);
            }
            Err(error) => {
                // Metadata failure: downgrade to a local trash candidate only when
                // `<dir>/target` exists as a real directory under the project.
                let local_target = dir.join("target");
                if !is_real_dir(&local_target) {
                    warn!(
                        project = %dir.display(),
                        error = %error,
                        "cargo metadata failed and no local target/ is available"
                    );
                    return;
                }
                warn!(
                    project = %dir.display(),
                    error = %error,
                    "cargo metadata failed; downgrading rust.target to local trash candidate"
                );
                let mut target = build_path_target(PathTargetInput {
                    rule_id: RUST_TARGET_RULE_DOC.id,
                    ecosystem: Ecosystem::Rust,
                    kind: TargetKind::BuildArtifacts,
                    project_root: dir.to_path_buf(),
                    path: local_target,
                    risk: RiskLevel::Medium,
                    selected_by_default: false,
                    evidence: vec![
                        Evidence::MarkerFile {
                            path: manifest.clone(),
                        },
                        Evidence::RuleMatched {
                            rule_id: RUST_TARGET_RULE_DOC.id.to_string(),
                        },
                        Evidence::RuleMatched {
                            rule_id: "rust.target.metadata_fallback_trash".to_string(),
                        },
                    ],
                });
                // Keep MoveToTrash from build_path_target; raise risk already set.
                target.reversible = true;
                targets.push(target);
            }
        }
    }

    fn scan_node_project(&self, dir: &Path, targets: &mut Vec<CleanTarget>) {
        let Some(marker) = find_node_marker(dir) else {
            return;
        };

        for rule in project_dir_rules(ProjectMarker::Node) {
            let path = dir.join(rule.relative);
            if is_real_dir(&path) {
                targets.push(build_path_target(PathTargetInput {
                    rule_id: rule.id,
                    ecosystem: rule.ecosystem.clone(),
                    kind: rule.kind.clone(),
                    project_root: dir.to_path_buf(),
                    path,
                    risk: rule.risk.clone(),
                    selected_by_default: rule.selected_by_default,
                    evidence: vec![
                        Evidence::MarkerFile {
                            path: marker.clone(),
                        },
                        Evidence::RuleMatched {
                            rule_id: rule.id.to_string(),
                        },
                    ],
                }));
            }
        }
    }

    fn scan_python_project_dir(
        &self,
        dir: &Path,
        context: &PythonContext,
        targets: &mut Vec<CleanTarget>,
    ) {
        for rule in project_dir_rules(ProjectMarker::Python) {
            let path = dir.join(rule.relative);
            if is_real_dir(&path) {
                targets.push(build_path_target(PathTargetInput {
                    rule_id: rule.id,
                    ecosystem: rule.ecosystem.clone(),
                    kind: rule.kind.clone(),
                    project_root: context.project_root.clone(),
                    path,
                    risk: rule.risk.clone(),
                    selected_by_default: rule.selected_by_default,
                    evidence: vec![
                        Evidence::MarkerFile {
                            path: context.marker.clone(),
                        },
                        Evidence::RuleMatched {
                            rule_id: rule.id.to_string(),
                        },
                    ],
                }));
            }
        }
    }
}

fn normalize_scan_roots(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut roots: Vec<PathBuf> = roots
        .iter()
        .map(|root| {
            root.canonicalize()
                .with_context(|| format!("failed to access scan root {}", root.display()))
        })
        .collect::<Result<_>>()?;
    roots.sort_by(|left, right| {
        footprint_depth(left)
            .cmp(&footprint_depth(right))
            .then_with(|| left.cmp(right))
    });
    roots.dedup();

    let mut covered = Vec::new();
    for root in roots {
        if !covered
            .iter()
            .any(|parent: &PathBuf| root.starts_with(parent))
        {
            covered.push(root);
        }
    }
    Ok(covered)
}

impl Default for ProjectScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct PythonContext {
    project_root: PathBuf,
    marker: PathBuf,
}

struct PathTargetInput {
    rule_id: &'static str,
    ecosystem: Ecosystem,
    kind: TargetKind,
    project_root: PathBuf,
    path: PathBuf,
    risk: RiskLevel,
    selected_by_default: bool,
    evidence: Vec<Evidence>,
}

fn build_path_target(input: PathTargetInput) -> CleanTarget {
    let PathTargetInput {
        rule_id,
        ecosystem,
        kind,
        project_root,
        path,
        risk,
        selected_by_default,
        evidence,
    } = input;
    let estimate = estimate_tree(&path);
    let selected_by_default = selected_by_default && estimate.complete;
    CleanTarget {
        id: TargetId::new(format!("{rule_id}:{}", path.display())),
        scope: Scope::Project { root: project_root },
        ecosystem,
        kind,
        path: Some(path.clone()),
        estimated_bytes: estimate.display_bytes(),
        size_complete: estimate.complete,
        last_modified: estimate.last_modified,
        risk,
        reversible: true,
        selected_by_default,
        evidence,
        action: CleanAction::MoveToTrash { path },
    }
}

fn dedupe_targets(mut targets: Vec<CleanTarget>) -> Vec<CleanTarget> {
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

fn footprint_depth(path: &Path) -> usize {
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

fn find_node_marker(dir: &Path) -> Option<PathBuf> {
    [
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
    ]
    .into_iter()
    .map(|name| dir.join(name))
    .find(|path| path.is_file())
}

fn find_python_marker(dir: &Path) -> Option<PathBuf> {
    [
        "pyproject.toml",
        "requirements.txt",
        "setup.py",
        "setup.cfg",
        "tox.ini",
    ]
    .into_iter()
    .map(|name| dir.join(name))
    .find(|path| path.is_file())
}

fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !is_unsafe_link(&metadata))
        .unwrap_or(false)
}

fn should_stop_descent(dir: &Path) -> bool {
    match dir.file_name().and_then(|name| name.to_str()) {
        // Always skip VCS metadata trees — they are huge and never cleanup roots.
        Some(".git" | ".hg" | ".svn") => true,
        // Known cleanup footprints: stop descent after the target itself is
        // considered. Bare name "cache" is intentionally NOT listed so a
        // directory named cache that contains real projects remains visible.
        Some(
            "target" | "node_modules" | ".turbo" | ".parcel-cache" | ".next" | ".venv" | "venv"
            | "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".tox",
        ) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn scanner_finds_marker_backed_project_targets() {
        let fixture = Fixture::new();
        fixture.file(
            "rust-app/Cargo.toml",
            "[package]\nname = \"rust-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        fixture.file("rust-app/src/lib.rs", "");
        fixture.file("rust-app/target/debug/app.bin", "binary");
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/node_modules/pkg/index.js", "module");
        fixture.file("node-app/.next/cache/blob", "cache");
        fixture.file("node-app/.turbo/state", "turbo");
        fixture.file("node-app/.parcel-cache/state", "parcel");
        fixture.file(
            "python-app/pyproject.toml",
            "[project]\nname = \"python-app\"\n",
        );
        fixture.file("python-app/.venv/pyvenv.cfg", "home = python");
        fixture.file("python-app/pkg/__pycache__/mod.pyc", "pyc");
        fixture.file("python-app/.pytest_cache/CACHEDIR.TAG", "pytest");
        fixture.file("python-app/.mypy_cache/state", "mypy");
        fixture.file("python-app/.ruff_cache/state", "ruff");
        fixture.file("python-app/.tox/state", "tox");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");
        let ids = target_ids(&plan);

        for rule in [
            "rust.target",
            "node.node_modules",
            "node.next_cache",
            "node.turbo",
            "node.parcel_cache",
            "python.venv_dot",
            "python.__pycache__",
            "python.pytest_cache",
            "python.mypy_cache",
            "python.ruff_cache",
            "python.tox",
        ] {
            assert!(ids.iter().any(|id| id.starts_with(rule)), "missing {rule}");
        }
    }

    #[test]
    fn scanner_ignores_markerless_cleanup_names() {
        let fixture = Fixture::new();
        fixture.file("loose/target/debug/file", "target");
        fixture.file("loose/build/file", "build");
        fixture.file("loose/dist/file", "dist");
        fixture.file("loose/node_modules/pkg/index.js", "module");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");

        assert!(plan.targets.is_empty());
    }

    #[test]
    fn scanner_does_not_follow_symlinked_project_directories() {
        let fixture = Fixture::new();
        fixture.file("outside/package.json", "{}");
        fixture.file("outside/node_modules/pkg/index.js", "module");
        fs::create_dir_all(fixture.path().join("scan")).expect("scan directory");
        let outside = fixture.path().join("outside");
        let link = fixture.path().join("scan/link-app");

        if create_dir_symlink(&outside, &link).is_err() {
            return;
        }

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().join("scan")])
            .expect("fixture scans");

        assert!(
            plan.targets.is_empty(),
            "scanner must not follow symlinked cleanup projects"
        );
    }

    #[cfg(windows)]
    #[test]
    fn scanner_skips_windows_reparse_point_directories() {
        let fixture = Fixture::new();
        fixture.file("outside/package.json", "{}");
        fixture.file("outside/node_modules/pkg/index.js", "module");
        fs::create_dir_all(fixture.path().join("scan")).expect("scan directory");
        let outside = fixture.path().join("outside");
        let link = fixture.path().join("scan/reparse-app");

        if create_dir_symlink(&outside, &link).is_err() {
            return;
        }

        let metadata = fs::symlink_metadata(&link).expect("link metadata");
        assert!(crate::fs_size::is_unsafe_link(&metadata));

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().join("scan")])
            .expect("fixture scans");

        assert!(
            plan.targets.is_empty(),
            "scanner must not follow Windows reparse point directories"
        );
    }

    #[test]
    fn scanner_output_has_plan_contract_fields_and_keeps_files() {
        let fixture = Fixture::new();
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/node_modules/pkg/index.js", "module");
        let module_file = fixture.path().join("node-app/node_modules/pkg/index.js");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");
        let target = plan.targets.first().expect("target discovered");

        assert!(module_file.exists(), "scan must not mutate fixtures");
        assert!(target.estimated_bytes > 0);
        assert!(!target.evidence.is_empty());
        assert!(matches!(target.risk, RiskLevel::Medium));
        assert!(!target.selected_by_default);
        assert!(matches!(target.action, CleanAction::MoveToTrash { .. }));

        let json = serde_json::to_value(
            crate::plan_validation::untrusted_plan_from_scan(&plan).expect("v2 plan converts"),
        )
        .expect("plan serializes");
        let first = &json["targets"][0];
        for key in [
            "risk",
            "evidence",
            "selected_by_default",
            "intent",
            "rule_id",
            "estimated_bytes",
        ] {
            assert!(!first[key].is_null(), "missing JSON field {key}");
        }
        assert!(first["action"].is_null());
    }

    #[test]
    fn rust_target_uses_cargo_clean_with_manifest_path() {
        let fixture = Fixture::new();
        fixture.file(
            "rust-app/Cargo.toml",
            "[package]\nname = \"rust-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        fixture.file("rust-app/src/lib.rs", "");
        fixture.file("rust-app/target/debug/app.bin", "binary");
        let manifest = fixture.path().join("rust-app/Cargo.toml");
        let manifest = manifest.canonicalize().expect("manifest canonicalizes");
        let target_dir = fixture
            .path()
            .join("rust-app/target")
            .canonicalize()
            .expect("target canonicalizes");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");
        let target = plan
            .targets
            .iter()
            .find(|target| target.id.as_str().starts_with("rust.target"))
            .expect("rust target discovered");

        match &target.action {
            CleanAction::Command {
                program,
                args,
                cwd: None,
                irreversible: true,
            } => {
                assert_eq!(program, "cargo");
                assert_eq!(args[0], "clean");
                assert_eq!(args[1], "--manifest-path");
                assert_eq!(PathBuf::from(&args[2]), manifest);
                assert_eq!(args[3], "--target-dir");
                assert_eq!(
                    PathBuf::from(&args[4]).canonicalize().expect("target-dir"),
                    target_dir
                );
                assert_eq!(
                    target
                        .path
                        .as_ref()
                        .and_then(|path| path.canonicalize().ok()),
                    Some(target_dir)
                );
            }
            action => panic!("unexpected rust target action: {action:?}"),
        }
    }

    #[test]
    fn scan_roots_reduces_duplicate_and_nested_roots() {
        let fixture = Fixture::new();
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/node_modules/pkg/index.js", "module");

        let root = fixture.path().to_path_buf();
        let plan = ProjectScanner::new()
            .scan_roots(&[root.clone(), root.clone(), root.join("node-app")])
            .expect("overlapping roots scan once");

        assert_eq!(plan.targets.len(), 1);
        assert!(plan.targets[0].id.as_str().starts_with("node.node_modules"));
    }

    #[cfg(windows)]
    #[test]
    fn scan_roots_collapses_case_and_separator_equivalent_roots() {
        let fixture = Fixture::new();
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/node_modules/pkg/index.js", "module");

        let root = fixture
            .path()
            .canonicalize()
            .expect("fixture root canonicalizes");
        let forward_slash_root = PathBuf::from(root.display().to_string().replace('\\', "/"));
        let uppercase_root = PathBuf::from(root.to_string_lossy().to_ascii_uppercase());

        let plan = ProjectScanner::new()
            .scan_roots(&[root, forward_slash_root, uppercase_root])
            .expect("equivalent roots scan once");

        assert_eq!(plan.targets.len(), 1);
        assert!(plan.targets[0].id.as_str().starts_with("node.node_modules"));
    }

    #[test]
    fn dedupe_targets_merges_evidence_for_exact_footprint_and_action() {
        let fixture = Fixture::new();
        fixture.file("cache/item", "payload");
        let root = fixture.path().to_path_buf();
        let path = root.join("cache");
        let target = build_path_target(PathTargetInput {
            rule_id: "node.next_cache",
            ecosystem: Ecosystem::Node,
            kind: TargetKind::BuildArtifacts,
            project_root: root,
            path,
            risk: RiskLevel::Low,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "node.next_cache".to_string(),
            }],
        });
        let mut duplicate = target.clone();
        duplicate.evidence.push(Evidence::UserConfigured);

        let deduped = dedupe_targets(vec![target, duplicate]);

        assert_eq!(deduped.len(), 1);
        assert!(deduped[0].evidence.contains(&Evidence::RuleMatched {
            rule_id: "node.next_cache".to_string(),
        }));
        assert!(deduped[0].evidence.contains(&Evidence::UserConfigured));
    }

    #[test]
    fn dedupe_targets_keeps_same_footprint_with_distinct_actions() {
        let fixture = Fixture::new();
        fixture.file("cache/item", "payload");
        let root = fixture.path().to_path_buf();
        let path = root.join("cache");
        let trash_target = build_path_target(PathTargetInput {
            rule_id: "node.next_cache",
            ecosystem: Ecosystem::Node,
            kind: TargetKind::BuildArtifacts,
            project_root: root,
            path,
            risk: RiskLevel::Low,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "node.next_cache".to_string(),
            }],
        });
        let mut command_target = trash_target.clone();
        command_target.id = TargetId::new("node.next_cache.command");
        command_target.action = CleanAction::Command {
            program: "npm".to_string(),
            args: vec!["cache".to_string(), "clean".to_string()],
            cwd: None,
            irreversible: true,
        };

        let deduped = dedupe_targets(vec![trash_target, command_target]);

        assert_eq!(deduped.len(), 2);
        assert!(
            deduped
                .iter()
                .any(|target| matches!(target.action, CleanAction::MoveToTrash { .. }))
        );
        assert!(deduped.iter().any(|target| {
            matches!(
                target.action,
                CleanAction::Command {
                    ref program,
                    ref args,
                    ..
                } if program == "npm"
                    && args.iter().map(String::as_str).eq(["cache", "clean"])
            )
        }));
    }

    #[test]
    fn dedupe_targets_removes_nested_cleanup_paths() {
        let root = PathBuf::from("C:/workspace/app");
        let parent = build_path_target(PathTargetInput {
            rule_id: "python.venv_dot",
            ecosystem: Ecosystem::Python,
            kind: TargetKind::VirtualEnv,
            project_root: root.clone(),
            path: root.join(".venv"),
            risk: RiskLevel::Medium,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "python.venv_dot".to_string(),
            }],
        });
        let child = build_path_target(PathTargetInput {
            rule_id: "python.__pycache__",
            ecosystem: Ecosystem::Python,
            kind: TargetKind::TestCache,
            project_root: root.clone(),
            path: root.join(".venv/Lib/site-packages/__pycache__"),
            risk: RiskLevel::Low,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "python.__pycache__".to_string(),
            }],
        });

        let deduped = dedupe_targets(vec![child, parent]);

        assert_eq!(deduped.len(), 1);
        assert!(deduped[0].id.as_str().starts_with("python.venv_dot"));
    }

    #[test]
    fn scan_roots_keeps_sibling_cache_targets_after_dedupe() {
        let fixture = Fixture::new();
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/.turbo/small.bin", "small");
        fixture.file("node-app/.parcel-cache/large.bin", "larger payload");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");

        let ids = target_ids(&plan);
        assert_eq!(ids.len(), 2, "both cache targets kept: {ids:?}");
        assert!(
            ids.iter().any(|id| id.starts_with("node.parcel_cache")),
            "parcel cache target kept: {ids:?}"
        );
        assert!(
            ids.iter().any(|id| id.starts_with("node.turbo")),
            "turbo cache target kept: {ids:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn unreadable_nested_child_keeps_sibling_targets_and_is_partial() {
        let fixture = Fixture::new();
        fixture.file("app/package.json", "{}");
        fixture.file("app/node_modules/pkg/index.js", "module");
        let denied = fixture.path().join("app/secret");
        fs::create_dir_all(&denied).expect("denied dir");
        fs::write(denied.join("hidden.bin"), "x").expect("hidden file");
        if deny_directory_read(&denied).is_err() {
            return;
        }
        assert!(
            fs::read_dir(&denied).is_err(),
            "fixture must deny read_dir before asserting partial scan"
        );

        let outcome = ProjectScanner::new()
            .scan_roots_with_diagnostics(&[fixture.path().to_path_buf()])
            .expect("partial scan succeeds");
        let _ = restore_directory_read(&denied);

        assert_eq!(outcome.completeness, ScanCompleteness::Partial);
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|diag| diag.path.ends_with("secret")),
            "diagnostic names denied child: {:?}",
            outcome.diagnostics
        );
        assert!(
            outcome
                .plan
                .targets
                .iter()
                .any(|target| target.id.as_str().starts_with("node.node_modules")),
            "sibling target kept: {:?}",
            target_ids(&outcome.plan)
        );
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_nested_child_keeps_sibling_targets_and_is_partial() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new();
        fixture.file("app/package.json", "{}");
        fixture.file("app/node_modules/pkg/index.js", "module");
        let denied = fixture.path().join("app/secret");
        fs::create_dir_all(&denied).expect("denied dir");
        fs::write(denied.join("hidden.bin"), "x").expect("hidden file");
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o000)).expect("chmod");
        assert!(fs::read_dir(&denied).is_err());

        let outcome = ProjectScanner::new()
            .scan_roots_with_diagnostics(&[fixture.path().to_path_buf()])
            .expect("partial scan succeeds");
        let _ = fs::set_permissions(&denied, fs::Permissions::from_mode(0o755));

        assert_eq!(outcome.completeness, ScanCompleteness::Partial);
        assert!(!outcome.diagnostics.is_empty());
        assert!(
            outcome
                .plan
                .targets
                .iter()
                .any(|target| target.id.as_str().starts_with("node.node_modules"))
        );
    }

    #[test]
    fn cache_named_directory_still_discovers_nested_projects() {
        let fixture = Fixture::new();
        fixture.file("cache/nested-app/package.json", "{}");
        fixture.file("cache/nested-app/node_modules/pkg/index.js", "module");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("cache-named root still scans");
        assert!(
            plan.targets
                .iter()
                .any(|target| target.id.as_str().contains("node_modules")),
            "project under bare cache/ name must be discovered: {:?}",
            target_ids(&plan)
        );
    }

    #[test]
    fn unreadable_root_remains_a_hard_error() {
        let fixture = Fixture::new();
        let missing = fixture.path().join("does-not-exist");
        let error = ProjectScanner::new()
            .scan_roots(&[missing])
            .expect_err("missing root is hard error");
        assert!(
            error.to_string().contains("failed to")
                || error
                    .chain()
                    .any(|cause| cause.to_string().contains("failed to")),
            "root failure keeps context: {error:#}"
        );
    }

    fn target_ids(plan: &CleanupPlan) -> Vec<&str> {
        plan.targets
            .iter()
            .map(|target| target.id.as_str())
            .collect()
    }

    struct Fixture {
        temp: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                temp: TempDir::new().expect("temp dir"),
            }
        }

        fn path(&self) -> &Path {
            self.temp.path()
        }

        fn file(&self, relative: &str, content: &str) {
            let path = self.temp.path().join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture directory");
            }
            fs::write(path, content).expect("fixture file");
        }
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(windows)]
    fn deny_directory_read(path: &Path) -> std::io::Result<()> {
        use std::process::Command;
        let output = Command::new("icacls")
            .arg(path)
            .arg("/deny")
            .arg(format!("{}:(OI)(CI)(R,X)", current_user()?))
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(String::from_utf8_lossy(
                &output.stderr,
            )))
        }
    }

    #[cfg(windows)]
    fn restore_directory_read(path: &Path) -> std::io::Result<()> {
        use std::process::Command;
        let _ = Command::new("icacls")
            .arg(path)
            .arg("/remove:d")
            .arg(current_user()?)
            .status();
        Ok(())
    }

    #[cfg(windows)]
    fn current_user() -> std::io::Result<String> {
        std::env::var("USERNAME").map_err(|_| std::io::Error::other("USERNAME missing"))
    }
}
