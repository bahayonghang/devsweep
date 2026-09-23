use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use tracing::warn;

mod cargo;
pub(super) mod dedupe;

use cargo::CargoWorkspaceCache;
use dedupe::{dedupe_targets, footprint_depth};

use crate::cargo_metadata::{
    CargoMetadataFailure, CargoMetadataProbe, CargoMetadataProbeResult, SystemCargoMetadataProbe,
};
use crate::filesystem::{
    DEFAULT_SIZE_ENTRY_BUDGET, PathReparseProbe, PathSafety, SystemPathReparseProbe,
    estimate_tree_with_budget_and_cancel_and_probe, inspect_path_no_follow,
};
use crate::model::{
    CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel, ScanDiagnosticOutcome,
    Scope, TargetId, TargetKind,
};
use crate::process::{CancelObserver, FlagCancelObserver};
use crate::rules::{
    PYCACHE_RULE_DOC, ProjectMarker, RUST_TARGET_METADATA_FALLBACK_RULE_ID, RUST_TARGET_RULE_DOC,
    project_dir_rules,
};

use crate::model::{ScanCompleteness, ScanDiagnostic, ScanDiagnosticStage};

type TargetObserver<'a> = &'a mut dyn FnMut(CleanTarget);

struct ProjectScanState<'a> {
    targets: &'a mut Vec<CleanTarget>,
    diagnostics: &'a mut Vec<ScanDiagnostic>,
    cargo_cache: &'a mut CargoWorkspaceCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScanOutcome {
    pub plan: CleanupPlan,
    pub diagnostics: Vec<ScanDiagnostic>,
    pub completeness: ScanCompleteness,
}

/// Production marker-first project artifact scanner.
pub struct ProjectScanner {
    reparse_probe: Arc<dyn PathReparseProbe>,
    cargo_metadata_probe: Arc<dyn CargoMetadataProbe>,
}

impl ProjectScanner {
    pub(crate) fn new() -> Self {
        Self {
            reparse_probe: Arc::new(SystemPathReparseProbe),
            cargo_metadata_probe: Arc::new(SystemCargoMetadataProbe::default()),
        }
    }

    #[cfg(test)]
    fn with_reparse_probe(reparse_probe: Arc<dyn PathReparseProbe>) -> Self {
        Self {
            reparse_probe,
            cargo_metadata_probe: Arc::new(SystemCargoMetadataProbe::default()),
        }
    }

    #[cfg(test)]
    fn with_probes(
        reparse_probe: Arc<dyn PathReparseProbe>,
        cargo_metadata_probe: Arc<dyn CargoMetadataProbe>,
    ) -> Self {
        Self {
            reparse_probe,
            cargo_metadata_probe,
        }
    }

    #[cfg(test)]
    pub(crate) fn scan_roots(&self, roots: &[PathBuf]) -> Result<CleanupPlan> {
        Ok(self.scan_roots_with_diagnostics(roots)?.plan)
    }

    #[cfg(test)]
    pub(crate) fn scan_roots_with_diagnostics(&self, roots: &[PathBuf]) -> Result<ScanOutcome> {
        self.scan_roots_with_diagnostics_and_cancel(roots, None)
    }

    pub(crate) fn scan_roots_with_diagnostics_and_cancel(
        &self,
        roots: &[PathBuf],
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanOutcome> {
        self.scan_roots_with_diagnostics_and_cancel_and_progress(roots, cancel, &mut |_| {})
    }

    pub(crate) fn scan_roots_with_diagnostics_and_cancel_and_progress(
        &self,
        roots: &[PathBuf],
        cancel: Option<&Arc<FlagCancelObserver>>,
        on_target: TargetObserver<'_>,
    ) -> Result<ScanOutcome> {
        let mut targets = Vec::new();
        let mut diagnostics = Vec::new();
        let mut cargo_cache = CargoWorkspaceCache::default();

        for root in normalize_scan_roots(roots, self.reparse_probe.as_ref(), &mut diagnostics)? {
            if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
                diagnostics.push(scan_diagnostic(
                    ScanDiagnosticStage::Discovery,
                    root.clone(),
                    ScanDiagnosticOutcome::Canceled,
                    "scan canceled",
                ));
                break;
            }
            // Root open failures remain hard errors.
            let metadata = fs::symlink_metadata(&root)
                .with_context(|| format!("failed to inspect scan root {}", root.display()))?;
            if !metadata.is_dir() {
                continue;
            }
            let path_safety = inspect_path_no_follow(&root, &metadata, self.reparse_probe.as_ref());
            if !matches!(path_safety, PathSafety::Safe) {
                diagnostics.push(scan_diagnostic(
                    ScanDiagnosticStage::Discovery,
                    root.clone(),
                    ScanDiagnosticOutcome::Skipped,
                    skipped_path_detail(&path_safety),
                ));
                continue;
            }
            fs::read_dir(&root)
                .with_context(|| format!("failed to read scan root {}", root.display()))?;
            self.scan_dir(
                &root,
                None,
                &mut ProjectScanState {
                    targets: &mut targets,
                    diagnostics: &mut diagnostics,
                    cargo_cache: &mut cargo_cache,
                },
                cancel,
                on_target,
            );
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
        state: &mut ProjectScanState<'_>,
        cancel: Option<&Arc<FlagCancelObserver>>,
        on_target: TargetObserver<'_>,
    ) {
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            state.diagnostics.push(scan_diagnostic(
                ScanDiagnosticStage::Discovery,
                dir.to_path_buf(),
                ScanDiagnosticOutcome::Canceled,
                "scan canceled",
            ));
            return;
        }
        let metadata = match fs::symlink_metadata(dir) {
            Ok(metadata) => metadata,
            Err(error) => {
                state.diagnostics.push(scan_diagnostic(
                    ScanDiagnosticStage::Discovery,
                    dir.to_path_buf(),
                    ScanDiagnosticOutcome::Skipped,
                    format!("failed to inspect: {error}"),
                ));
                return;
            }
        };
        if !metadata.is_dir() {
            return;
        }
        let path_safety = inspect_path_no_follow(dir, &metadata, self.reparse_probe.as_ref());
        if !matches!(path_safety, PathSafety::Safe) {
            state.diagnostics.push(scan_diagnostic(
                ScanDiagnosticStage::Discovery,
                dir.to_path_buf(),
                ScanDiagnosticOutcome::Skipped,
                skipped_path_detail(&path_safety),
            ));
            return;
        }

        if should_stop_descent(dir) {
            if dir.file_name().and_then(|name| name.to_str()) == Some("__pycache__")
                && let Some(context) = python_context
            {
                let target = build_path_target_with_probe(
                    PathTargetInput {
                        rule_id: PYCACHE_RULE_DOC.id,
                        ecosystem: PYCACHE_RULE_DOC.ecosystem.clone(),
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
                    },
                    cancel,
                    self.reparse_probe.as_ref(),
                );
                push_observed_target(state.targets, target, on_target);
            }
            return;
        }

        self.scan_rust_project(
            dir,
            state.targets,
            state.diagnostics,
            state.cargo_cache,
            cancel,
            on_target,
        );
        self.scan_node_project(dir, state.targets, cancel, on_target);

        let local_python_context =
            find_python_marker(dir, self.reparse_probe.as_ref()).map(|marker| PythonContext {
                project_root: dir.to_path_buf(),
                marker,
            });
        let active_python_context = local_python_context.as_ref().or(python_context);
        if let Some(context) = active_python_context {
            self.scan_python_project_dir(dir, context, state.targets, cancel, on_target);
        }

        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) => {
                state.diagnostics.push(scan_diagnostic(
                    ScanDiagnosticStage::Discovery,
                    dir.to_path_buf(),
                    ScanDiagnosticOutcome::Skipped,
                    format!("failed to read: {error}"),
                ));
                return;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    state.diagnostics.push(scan_diagnostic(
                        ScanDiagnosticStage::Discovery,
                        dir.to_path_buf(),
                        ScanDiagnosticOutcome::Skipped,
                        format!("failed to read directory entry: {error}"),
                    ));
                    continue;
                }
            };
            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    state.diagnostics.push(scan_diagnostic(
                        ScanDiagnosticStage::Discovery,
                        path.clone(),
                        ScanDiagnosticOutcome::Skipped,
                        format!("failed to inspect: {error}"),
                    ));
                    continue;
                }
            };
            if metadata.is_dir() {
                self.scan_dir(&path, active_python_context, state, cancel, on_target);
            }
        }
    }

    fn scan_rust_project(
        &self,
        dir: &Path,
        targets: &mut Vec<CleanTarget>,
        diagnostics: &mut Vec<ScanDiagnostic>,
        cargo_cache: &mut CargoWorkspaceCache,
        cancel: Option<&Arc<FlagCancelObserver>>,
        on_target: TargetObserver<'_>,
    ) {
        let manifest = dir.join("Cargo.toml");
        if !is_real_file(&manifest, self.reparse_probe.as_ref()) {
            return;
        }

        match cargo_cache.resolve(
            &manifest,
            dir,
            self.cargo_metadata_probe.as_ref(),
            self.reparse_probe.as_ref(),
        ) {
            CargoMetadataProbeResult::Resolved(scope) => {
                let target_dir = scope.target_directory;
                if !is_real_dir(&target_dir, self.reparse_probe.as_ref()) {
                    return;
                }
                let workspace_root = if scope.workspace_root.as_os_str().is_empty() {
                    dir.to_path_buf()
                } else {
                    scope.workspace_root
                };
                let project_root = if is_real_file(
                    &workspace_root.join("Cargo.toml"),
                    self.reparse_probe.as_ref(),
                ) {
                    workspace_root
                } else {
                    dir.to_path_buf()
                };
                let manifest_path = project_root.join("Cargo.toml");
                let manifest_arg = manifest_path.to_string_lossy().into_owned();
                let target_arg = target_dir.to_string_lossy().into_owned();
                let mut target = build_path_target_with_probe(
                    PathTargetInput {
                        rule_id: RUST_TARGET_RULE_DOC.id,
                        ecosystem: RUST_TARGET_RULE_DOC.ecosystem.clone(),
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
                    },
                    cancel,
                    self.reparse_probe.as_ref(),
                );
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
                push_observed_target(targets, target, on_target);
            }
            CargoMetadataProbeResult::Failed(failure) => {
                let diagnostic = cargo_metadata_diagnostic(&manifest, failure);
                let error = diagnostic.detail.clone();
                diagnostics.push(diagnostic);
                // Metadata failure: downgrade to a local trash candidate only when
                // `<dir>/target` exists as a real directory under the project.
                let local_target = dir.join("target");
                if !is_real_dir(&local_target, self.reparse_probe.as_ref()) {
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
                let mut target = build_path_target_with_probe(
                    PathTargetInput {
                        rule_id: RUST_TARGET_RULE_DOC.id,
                        ecosystem: RUST_TARGET_RULE_DOC.ecosystem.clone(),
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
                                rule_id: RUST_TARGET_METADATA_FALLBACK_RULE_ID.to_string(),
                            },
                        ],
                    },
                    cancel,
                    self.reparse_probe.as_ref(),
                );
                // Keep MoveToTrash from build_path_target; raise risk already set.
                target.reversible = true;
                push_observed_target(targets, target, on_target);
            }
        }
    }

    fn scan_node_project(
        &self,
        dir: &Path,
        targets: &mut Vec<CleanTarget>,
        cancel: Option<&Arc<FlagCancelObserver>>,
        on_target: TargetObserver<'_>,
    ) {
        let Some(marker) = find_node_marker(dir, self.reparse_probe.as_ref()) else {
            return;
        };

        for rule in project_dir_rules(ProjectMarker::Node) {
            let path = dir.join(rule.relative);
            if is_real_dir(&path, self.reparse_probe.as_ref()) {
                let target = build_path_target_with_probe(
                    PathTargetInput {
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
                    },
                    cancel,
                    self.reparse_probe.as_ref(),
                );
                push_observed_target(targets, target, on_target);
            }
        }
    }

    fn scan_python_project_dir(
        &self,
        dir: &Path,
        context: &PythonContext,
        targets: &mut Vec<CleanTarget>,
        cancel: Option<&Arc<FlagCancelObserver>>,
        on_target: TargetObserver<'_>,
    ) {
        for rule in project_dir_rules(ProjectMarker::Python) {
            let path = dir.join(rule.relative);
            if is_real_dir(&path, self.reparse_probe.as_ref()) {
                let target = build_path_target_with_probe(
                    PathTargetInput {
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
                    },
                    cancel,
                    self.reparse_probe.as_ref(),
                );
                push_observed_target(targets, target, on_target);
            }
        }
    }
}

fn normalize_scan_roots(
    roots: &[PathBuf],
    reparse_probe: &dyn PathReparseProbe,
    diagnostics: &mut Vec<ScanDiagnostic>,
) -> Result<Vec<PathBuf>> {
    let mut normalized_roots = Vec::with_capacity(roots.len());
    for root in roots {
        let metadata = fs::symlink_metadata(root)
            .with_context(|| format!("failed to inspect scan root {}", root.display()))?;
        if !metadata.is_dir() {
            continue;
        }
        let path_safety = inspect_path_no_follow(root, &metadata, reparse_probe);
        if !matches!(path_safety, PathSafety::Safe) {
            diagnostics.push(scan_diagnostic(
                ScanDiagnosticStage::Discovery,
                root.clone(),
                ScanDiagnosticOutcome::Skipped,
                skipped_path_detail(&path_safety),
            ));
            continue;
        }
        normalized_roots.push(
            root.canonicalize()
                .with_context(|| format!("failed to access scan root {}", root.display()))?,
        );
    }
    let mut roots = normalized_roots;
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

fn scan_diagnostic(
    stage: ScanDiagnosticStage,
    path: PathBuf,
    outcome: ScanDiagnosticOutcome,
    detail: impl Into<String>,
) -> ScanDiagnostic {
    ScanDiagnostic {
        stage,
        path,
        outcome,
        detail: detail.into(),
        process: None,
    }
}

fn cargo_metadata_diagnostic(manifest: &Path, failure: CargoMetadataFailure) -> ScanDiagnostic {
    ScanDiagnostic {
        stage: ScanDiagnosticStage::CargoMetadata,
        path: manifest.to_path_buf(),
        outcome: failure.outcome,
        detail: failure.detail,
        process: failure.process,
    }
}

#[cfg(test)]
fn build_path_target(input: PathTargetInput) -> CleanTarget {
    let probe = SystemPathReparseProbe;
    build_path_target_with_probe(input, None, &probe)
}

fn build_path_target_with_probe(
    input: PathTargetInput,
    cancel: Option<&Arc<FlagCancelObserver>>,
    reparse_probe: &dyn PathReparseProbe,
) -> CleanTarget {
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
    let estimate = estimate_tree_with_budget_and_cancel_and_probe(
        &path,
        DEFAULT_SIZE_ENTRY_BUDGET,
        cancel,
        reparse_probe,
    );
    let selected_by_default = selected_by_default && estimate.complete;
    CleanTarget {
        id: TargetId::new(format!("{rule_id}:{}", path.display())),
        scope: Scope::Project { root: project_root },
        ecosystem,
        kind,
        path: Some(path.clone()),
        estimated_bytes: estimate.display_bytes(),
        size_complete: estimate.complete,
        sizing_warnings: estimate.warnings,
        last_modified: estimate.last_modified,
        risk,
        reversible: true,
        selected_by_default,
        evidence,
        action: CleanAction::MoveToTrash { path },
    }
}

fn push_observed_target(
    targets: &mut Vec<CleanTarget>,
    target: CleanTarget,
    on_target: TargetObserver<'_>,
) {
    targets.push(target.clone());
    on_target(target);
}

/// A reviewed target-size rescan gets a bounded, larger entry budget but never
/// accepts an arbitrary path: the target must come from the current plan.
pub(super) const REVIEW_SIZE_ENTRY_BUDGET: usize = DEFAULT_SIZE_ENTRY_BUDGET * 10;

pub(super) fn rescan_target_size(
    plan: &mut CleanupPlan,
    target_id: &TargetId,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<()> {
    let probe = SystemPathReparseProbe;
    rescan_target_size_with_probe(plan, target_id, REVIEW_SIZE_ENTRY_BUDGET, cancel, &probe)
}

pub(crate) fn rescan_target_size_with_probe(
    plan: &mut CleanupPlan,
    target_id: &TargetId,
    entry_budget: usize,
    cancel: Option<&Arc<FlagCancelObserver>>,
    probe: &dyn PathReparseProbe,
) -> Result<()> {
    if entry_budget <= DEFAULT_SIZE_ENTRY_BUDGET {
        bail!(
            "target size rescan budget must exceed the default {} entries",
            DEFAULT_SIZE_ENTRY_BUDGET
        );
    }

    let target = plan
        .targets
        .iter_mut()
        .find(|target| target.id == *target_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "target {} was not found in the current scan",
                target_id.as_str()
            )
        })?;
    let path = target
        .path
        .clone()
        .ok_or_else(|| anyhow::anyhow!("target {} has no path to rescan", target_id.as_str()))?;
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("failed to inspect rescan target {}", path.display()))?;
    if !metadata.is_dir() && !metadata.is_file() {
        bail!(
            "rescan target {} is not a file or directory",
            path.display()
        );
    }
    let path_safety = inspect_path_no_follow(&path, &metadata, probe);
    if !matches!(path_safety, PathSafety::Safe) {
        bail!(
            "rescan target {} is unsafe: {}",
            path.display(),
            skipped_path_detail(&path_safety)
        );
    }

    let estimate =
        estimate_tree_with_budget_and_cancel_and_probe(&path, entry_budget, cancel, probe);
    target.estimated_bytes = estimate.display_bytes();
    target.size_complete = estimate.complete;
    target.sizing_warnings = estimate.warnings;
    target.last_modified = estimate.last_modified;
    if !target.size_complete {
        target.selected_by_default = false;
    }
    Ok(())
}

fn find_node_marker(dir: &Path, reparse_probe: &dyn PathReparseProbe) -> Option<PathBuf> {
    [
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
    ]
    .into_iter()
    .map(|name| dir.join(name))
    .find(|path| is_real_file(path, reparse_probe))
}

fn find_python_marker(dir: &Path, reparse_probe: &dyn PathReparseProbe) -> Option<PathBuf> {
    [
        "pyproject.toml",
        "requirements.txt",
        "setup.py",
        "setup.cfg",
        "tox.ini",
    ]
    .into_iter()
    .map(|name| dir.join(name))
    .find(|path| is_real_file(path, reparse_probe))
}

fn is_real_file(path: &Path, reparse_probe: &dyn PathReparseProbe) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| {
            metadata.is_file()
                && matches!(
                    inspect_path_no_follow(path, &metadata, reparse_probe),
                    PathSafety::Safe
                )
        })
        .unwrap_or(false)
}

fn is_real_dir(path: &Path, reparse_probe: &dyn PathReparseProbe) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| {
            metadata.is_dir()
                && matches!(
                    inspect_path_no_follow(path, &metadata, reparse_probe),
                    PathSafety::Safe
                )
        })
        .unwrap_or(false)
}

fn skipped_path_detail(path_safety: &PathSafety) -> String {
    match path_safety {
        PathSafety::Safe => "path is safe".to_string(),
        PathSafety::ReparsePoint { tag: Some(tag) } => {
            format!("skipped reparse point with tag 0x{tag:08x}")
        }
        PathSafety::ReparsePoint { tag: None } => {
            "skipped symlink, junction, or reparse point".to_string()
        }
        PathSafety::Unverified { detail } => {
            format!("skipped path because reparse safety could not be verified: {detail}")
        }
    }
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
    use std::{
        collections::{HashMap, HashSet},
        fs,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use tempfile::TempDir;

    use super::*;
    use crate::{cargo_metadata::CargoMetadataScope, filesystem::ReparseProbeResult};

    const CLOUD_FILES_REPARSE_TAG: u32 = 0x9000_701A;

    struct FixtureReparseProbe {
        results: HashMap<PathBuf, ReparseProbeResult>,
    }

    impl FixtureReparseProbe {
        fn tagged(path: &Path) -> Self {
            let mut results = HashMap::new();
            let tagged = ReparseProbeResult::ReparsePoint {
                tag: CLOUD_FILES_REPARSE_TAG,
            };
            results.insert(path.to_path_buf(), tagged.clone());
            if let Ok(canonical) = path.canonicalize() {
                results.insert(canonical, tagged);
            }
            Self { results }
        }
    }

    impl PathReparseProbe for FixtureReparseProbe {
        fn probe(&self, path: &Path) -> ReparseProbeResult {
            self.results
                .get(path)
                .cloned()
                .unwrap_or(ReparseProbeResult::NotReparsePoint)
        }
    }

    #[derive(Clone)]
    struct RecordingCargoMetadataProbe {
        results: Arc<Mutex<HashMap<PathBuf, CargoMetadataProbeResult>>>,
        calls: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl RecordingCargoMetadataProbe {
        fn new(results: impl IntoIterator<Item = (PathBuf, CargoMetadataProbeResult)>) -> Self {
            let mut indexed = HashMap::new();
            for (manifest_dir, result) in results {
                indexed.insert(manifest_dir.clone(), result.clone());
                if let Ok(canonical) = manifest_dir.canonicalize() {
                    indexed.insert(canonical, result);
                }
            }
            Self {
                results: Arc::new(Mutex::new(indexed)),
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn calls(&self) -> Vec<PathBuf> {
            self.calls.lock().expect("calls lock").clone()
        }
    }

    impl CargoMetadataProbe for RecordingCargoMetadataProbe {
        fn probe(&self, manifest_dir: &Path) -> CargoMetadataProbeResult {
            self.calls
                .lock()
                .expect("calls lock")
                .push(manifest_dir.to_path_buf());
            self.results
                .lock()
                .expect("results lock")
                .get(manifest_dir)
                .cloned()
                .unwrap_or_else(|| {
                    CargoMetadataProbeResult::Failed(CargoMetadataFailure {
                        outcome: ScanDiagnosticOutcome::Failed,
                        detail: format!(
                            "test cargo metadata probe has no result for {}",
                            manifest_dir.display()
                        ),
                        process: None,
                    })
                })
        }
    }

    fn metadata_scope(
        workspace_root: &Path,
        target_directory: &Path,
        member_manifests: Vec<PathBuf>,
    ) -> CargoMetadataProbeResult {
        CargoMetadataProbeResult::Resolved(CargoMetadataScope {
            workspace_root: workspace_root.to_path_buf(),
            target_directory: target_directory.to_path_buf(),
            member_manifests,
        })
    }

    fn metadata_failure(detail: &str) -> CargoMetadataProbeResult {
        CargoMetadataProbeResult::Failed(CargoMetadataFailure {
            outcome: ScanDiagnosticOutcome::Failed,
            detail: detail.to_string(),
            process: None,
        })
    }

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
    fn project_scanner_reports_targets_as_they_are_constructed() {
        let fixture = Fixture::new();
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/node_modules/pkg/index.js", "module");
        fixture.file("node-app/.next/cache/blob", "cache");
        let mut observed = Vec::new();

        let outcome = ProjectScanner::new()
            .scan_roots_with_diagnostics_and_cancel_and_progress(
                &[fixture.path().to_path_buf()],
                None,
                &mut |target| observed.push(target.id.clone()),
            )
            .expect("fixture scans with checkpoints");

        assert!(
            observed.len() >= 2,
            "same-phase targets arrive incrementally"
        );
        assert_eq!(
            observed.into_iter().collect::<HashSet<_>>(),
            outcome
                .plan
                .targets
                .iter()
                .map(|target| target.id.clone())
                .collect()
        );
    }

    #[test]
    fn ordinary_target_sizing_observes_cancellation() {
        let fixture = Fixture::new();
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/node_modules/pkg/index.js", "module");
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();

        let target = build_path_target_with_probe(
            PathTargetInput {
                rule_id: "node.node_modules",
                ecosystem: Ecosystem::Node,
                kind: TargetKind::DependencyDirectory,
                project_root: fixture.path().join("node-app"),
                path: fixture.path().join("node-app/node_modules"),
                risk: RiskLevel::Medium,
                selected_by_default: true,
                evidence: vec![Evidence::UserConfigured],
            },
            Some(&cancel),
            &SystemPathReparseProbe,
        );

        assert!(!target.size_complete);
        assert!(!target.selected_by_default);
        assert!(
            target
                .sizing_warnings
                .iter()
                .any(|warning| warning.kind == crate::model::SizingWarningKind::Canceled)
        );
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

    #[test]
    fn scanner_skips_injected_cloud_files_reparse_root() {
        let fixture = Fixture::new();
        fixture.file("app/package.json", "{}");
        fixture.file("app/node_modules/pkg/index.js", "module");
        let root = fixture.path().to_path_buf();
        let scanner =
            ProjectScanner::with_reparse_probe(Arc::new(FixtureReparseProbe::tagged(&root)));

        let outcome = scanner
            .scan_roots_with_diagnostics(&[root])
            .expect("fixture scans");

        assert!(outcome.plan.targets.is_empty());
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.detail.contains("0x9000701a") })
        );
    }

    #[test]
    fn scanner_skips_injected_cloud_files_reparse_child() {
        let fixture = Fixture::new();
        fixture.file("cloud-app/package.json", "{}");
        fixture.file("cloud-app/node_modules/pkg/index.js", "module");
        let child = fixture.path().join("cloud-app");
        let scanner =
            ProjectScanner::with_reparse_probe(Arc::new(FixtureReparseProbe::tagged(&child)));

        let plan = scanner
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");

        assert!(plan.targets.is_empty());
    }

    #[test]
    fn scanner_skips_injected_cloud_files_reparse_target() {
        let fixture = Fixture::new();
        fixture.file("app/package.json", "{}");
        fixture.file("app/node_modules/pkg/index.js", "module");
        let target = fixture.path().join("app/node_modules");
        let scanner =
            ProjectScanner::with_reparse_probe(Arc::new(FixtureReparseProbe::tagged(&target)));

        let plan = scanner
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");

        assert!(plan.targets.is_empty());
    }

    #[test]
    fn scanner_skips_injected_cloud_files_reparse_marker() {
        let fixture = Fixture::new();
        fixture.file("app/package.json", "{}");
        fixture.file("app/node_modules/pkg/index.js", "module");
        let marker = fixture.path().join("app/package.json");
        let scanner =
            ProjectScanner::with_reparse_probe(Arc::new(FixtureReparseProbe::tagged(&marker)));

        let plan = scanner
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");

        assert!(plan.targets.is_empty());
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
        assert!(crate::filesystem::is_unsafe_link(&metadata));

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
            crate::plan::untrusted_plan_from_scan(&plan).expect("v2 plan converts"),
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
    fn cargo_workspace_member_resolution_is_reused_once_per_scan() {
        let fixture = Fixture::new();
        fixture.file("workspace/Cargo.toml", "[workspace]\n");
        fixture.file("workspace/member-a/Cargo.toml", "[package]\nname = \"a\"\n");
        fixture.file("workspace/member-b/Cargo.toml", "[package]\nname = \"b\"\n");
        fixture.file("workspace/target/debug/app.bin", "binary");

        let workspace = fixture.path().join("workspace");
        let member_a = workspace.join("member-a");
        let member_b = workspace.join("member-b");
        let scope = metadata_scope(
            &workspace,
            &workspace.join("target"),
            vec![member_a.join("Cargo.toml"), member_b.join("Cargo.toml")],
        );
        let probe = RecordingCargoMetadataProbe::new([
            (member_a.clone(), scope.clone()),
            (member_b.clone(), scope),
        ]);
        let scanner =
            ProjectScanner::with_probes(Arc::new(SystemPathReparseProbe), Arc::new(probe.clone()));

        let plan = scanner
            .scan_roots(&[member_a, member_b])
            .expect("workspace members scan");

        assert_eq!(
            probe.calls().len(),
            1,
            "metadata result is reused for member"
        );
        assert_eq!(plan.targets.len(), 1, "shared Cargo target is deduplicated");
    }

    #[test]
    fn unproven_nested_manifest_runs_its_own_cargo_metadata_probe() {
        let fixture = Fixture::new();
        fixture.file("parent/Cargo.toml", "[package]\nname = \"parent\"\n");
        fixture.file("parent/target/debug/parent.bin", "binary");
        fixture.file("parent/nested/Cargo.toml", "[package]\nname = \"nested\"\n");
        fixture.file("parent/nested/target/debug/nested.bin", "binary");

        let parent = fixture.path().join("parent");
        let nested = parent.join("nested");
        let probe = RecordingCargoMetadataProbe::new([
            (
                parent.clone(),
                metadata_scope(
                    &parent,
                    &parent.join("target"),
                    vec![parent.join("Cargo.toml")],
                ),
            ),
            (
                nested.clone(),
                metadata_scope(
                    &nested,
                    &nested.join("target"),
                    vec![nested.join("Cargo.toml")],
                ),
            ),
        ]);
        let scanner =
            ProjectScanner::with_probes(Arc::new(SystemPathReparseProbe), Arc::new(probe.clone()));

        scanner
            .scan_roots(&[parent])
            .expect("nested workspaces scan");

        assert_eq!(
            probe.calls().len(),
            2,
            "nested manifest is not reused without explicit workspace membership"
        );
    }

    #[test]
    fn cargo_metadata_failures_are_replayed_only_for_the_exact_manifest() {
        let fixture = Fixture::new();
        fixture.file("app/Cargo.toml", "[package]\nname = \"app\"\n");
        let app = fixture.path().join("app");
        let manifest = app.join("Cargo.toml");
        let expected = metadata_failure("fixture metadata failure");
        let probe = RecordingCargoMetadataProbe::new([(app.clone(), expected.clone())]);
        let mut cache = CargoWorkspaceCache::with_limit(8);
        let reparse_probe = SystemPathReparseProbe;

        let first = cache.resolve(&manifest, &app, &probe, &reparse_probe);
        let second = cache.resolve(&manifest, &app, &probe, &reparse_probe);

        assert_eq!(first, expected);
        assert_eq!(second, expected);
        assert_eq!(probe.calls().len(), 1, "exact failure is replayed");
    }

    #[test]
    fn cargo_metadata_cache_entry_limit_prevents_unbounded_failure_growth() {
        let fixture = Fixture::new();
        fixture.file("app-a/Cargo.toml", "[package]\nname = \"a\"\n");
        fixture.file("app-b/Cargo.toml", "[package]\nname = \"b\"\n");
        let app_a = fixture.path().join("app-a");
        let app_b = fixture.path().join("app-b");
        let probe = RecordingCargoMetadataProbe::new([
            (app_a.clone(), metadata_failure("a failed")),
            (app_b.clone(), metadata_failure("b failed")),
        ]);
        let mut cache = CargoWorkspaceCache::with_limit(1);
        let reparse_probe = SystemPathReparseProbe;

        cache.resolve(&app_a.join("Cargo.toml"), &app_a, &probe, &reparse_probe);
        cache.resolve(&app_b.join("Cargo.toml"), &app_b, &probe, &reparse_probe);
        cache.resolve(&app_b.join("Cargo.toml"), &app_b, &probe, &reparse_probe);

        assert_eq!(cache.entry_count(), 1);
        assert_eq!(probe.calls().len(), 3, "uncached entry is probed again");
    }

    #[test]
    fn truncated_cargo_metadata_failure_reaches_scan_health_with_process_metadata() {
        let fixture = Fixture::new();
        fixture.file("app/Cargo.toml", "[package]\nname = \"app\"\n");
        fixture.file("app/target/debug/app.bin", "binary");
        let app = fixture.path().join("app");
        let probe = RecordingCargoMetadataProbe::new([(
            app.clone(),
            CargoMetadataProbeResult::Failed(CargoMetadataFailure {
                outcome: ScanDiagnosticOutcome::OutputTruncated,
                detail: "cargo metadata output was truncated before JSON parsing".to_string(),
                process: Some(crate::model::ScanProcessProbe {
                    status: crate::model::ScanProcessStatus::Success,
                    stdout: crate::model::ScanProcessOutput {
                        truncated: true,
                        retained_bytes: 32,
                        total_bytes: 2_048,
                    },
                    stderr: crate::model::ScanProcessOutput {
                        truncated: false,
                        retained_bytes: 0,
                        total_bytes: 0,
                    },
                }),
            }),
        )]);
        let scanner =
            ProjectScanner::with_probes(Arc::new(SystemPathReparseProbe), Arc::new(probe));

        let outcome = scanner
            .scan_roots_with_diagnostics(&[app])
            .expect("truncated metadata scan");
        let diagnostic = outcome
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.stage == ScanDiagnosticStage::CargoMetadata)
            .expect("cargo metadata diagnostic");

        assert_eq!(diagnostic.outcome, ScanDiagnosticOutcome::OutputTruncated);
        let process = diagnostic.process.as_ref().expect("process metadata");
        assert!(process.stdout.truncated);
        assert_eq!(process.stdout.retained_bytes, 32);
        assert_eq!(process.stdout.total_bytes, 2_048);
        assert!(outcome.plan.targets.iter().any(|target| {
            target.id.as_str().starts_with("rust.target")
                && matches!(target.action, CleanAction::MoveToTrash { .. })
        }));
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
    fn dedupe_targets_merges_repeated_unresolved_command_target() {
        let fixture = Fixture::new();
        fixture.file("cache/item", "payload");
        let root = fixture.path().to_path_buf();
        let mut target = build_path_target(PathTargetInput {
            rule_id: "pip.cache.purge",
            ecosystem: Ecosystem::Python,
            kind: TargetKind::PackageCache,
            project_root: root.clone(),
            path: root.join("cache"),
            risk: RiskLevel::Low,
            selected_by_default: false,
            evidence: Vec::new(),
        });
        target.id = TargetId::new("pip.cache.purge:unresolved");
        target.path = None;
        target.action = CleanAction::Command {
            program: "python".to_string(),
            args: vec!["-m".to_string(), "pip".to_string()],
            cwd: None,
            irreversible: true,
        };

        let deduped = dedupe_targets(vec![target.clone(), target]);

        assert_eq!(deduped.len(), 1);
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
    fn scan_honors_cancel_flag_without_full_tree_walk() {
        use crate::process::FlagCancelObserver;
        use std::sync::Arc;

        let fixture = Fixture::new();
        for i in 0..50 {
            fixture.file(&format!("p{i}/package.json"), "{}");
            fixture.file(&format!("p{i}/node_modules/x/index.js"), "m");
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();
        let started = std::time::Instant::now();
        let outcome = ProjectScanner::new()
            .scan_roots_with_diagnostics_and_cancel(&[fixture.path().to_path_buf()], Some(&cancel))
            .expect("canceled scan still returns");
        let elapsed = started.elapsed();
        assert_eq!(outcome.completeness, ScanCompleteness::Partial);
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|d| d.detail.contains("canceled")),
            "expected cancel diagnostic: {:?}",
            outcome.diagnostics
        );
        assert!(
            elapsed.as_millis() < 250,
            "cancel should stop discovery quickly, took {elapsed:?}"
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

    #[test]
    fn reviewed_rescan_replaces_only_the_selected_target_observation() {
        let fixture = Fixture::new();
        fixture.file("app/cache/first.bin", "payload");
        fixture.file("app/other/second.bin", "unchanged");
        let root = fixture.path().join("app");
        let target = build_path_target(PathTargetInput {
            rule_id: "node.next_cache",
            ecosystem: Ecosystem::Node,
            kind: TargetKind::BuildArtifacts,
            project_root: root.clone(),
            path: root.join("cache"),
            risk: RiskLevel::Low,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "node.next_cache".to_string(),
            }],
        });
        let sibling = build_path_target(PathTargetInput {
            rule_id: "node.turbo",
            ecosystem: Ecosystem::Node,
            kind: TargetKind::ToolCache,
            project_root: root,
            path: fixture.path().join("app/other"),
            risk: RiskLevel::Low,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "node.turbo".to_string(),
            }],
        });
        let target_id = target.id.clone();
        let sibling_bytes = sibling.estimated_bytes;
        let mut plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target, sibling],
        };
        let reviewed = plan.targets.first_mut().expect("reviewed target");
        reviewed.estimated_bytes = 0;
        reviewed.size_complete = false;
        reviewed.sizing_warnings = vec![crate::model::SizingWarning {
            kind: crate::model::SizingWarningKind::EntryBudgetExhausted,
            detail: "normal scan hit its entry budget".to_string(),
        }];

        let probe = SystemPathReparseProbe;
        rescan_target_size_with_probe(
            &mut plan,
            &target_id,
            REVIEW_SIZE_ENTRY_BUDGET,
            None,
            &probe,
        )
        .expect("reviewed rescan succeeds");

        assert_eq!(plan.targets[0].estimated_bytes, 7);
        assert!(plan.targets[0].size_complete);
        assert!(plan.targets[0].sizing_warnings.is_empty());
        assert_eq!(plan.targets[1].estimated_bytes, sibling_bytes);
    }

    #[test]
    fn reviewed_rescan_rejects_an_injected_cloud_files_target() {
        let fixture = Fixture::new();
        fixture.file("app/cache/data.bin", "payload");
        let root = fixture.path().join("app");
        let path = root.join("cache");
        let target = build_path_target(PathTargetInput {
            rule_id: "node.next_cache",
            ecosystem: Ecosystem::Node,
            kind: TargetKind::BuildArtifacts,
            project_root: root,
            path: path.clone(),
            risk: RiskLevel::Low,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "node.next_cache".to_string(),
            }],
        });
        let target_id = target.id.clone();
        let mut plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target],
        };
        let probe = FixtureReparseProbe::tagged(&path);

        let error = rescan_target_size_with_probe(
            &mut plan,
            &target_id,
            REVIEW_SIZE_ENTRY_BUDGET,
            None,
            &probe,
        )
        .expect_err("reparse target rejects");

        assert!(error.to_string().contains("reparse point"));
    }

    #[test]
    fn reviewed_rescan_honors_cancellation_and_deselects_an_incomplete_target() {
        let fixture = Fixture::new();
        fixture.file("app/cache/data.bin", "payload");
        let root = fixture.path().join("app");
        let target = build_path_target(PathTargetInput {
            rule_id: "node.next_cache",
            ecosystem: Ecosystem::Node,
            kind: TargetKind::BuildArtifacts,
            project_root: root.clone(),
            path: root.join("cache"),
            risk: RiskLevel::Low,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "node.next_cache".to_string(),
            }],
        });
        let target_id = target.id.clone();
        let mut plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target],
        };
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();
        let probe = SystemPathReparseProbe;

        rescan_target_size_with_probe(
            &mut plan,
            &target_id,
            REVIEW_SIZE_ENTRY_BUDGET,
            Some(&cancel),
            &probe,
        )
        .expect("canceled rescan returns an incomplete observation");

        assert!(!plan.targets[0].size_complete);
        assert!(!plan.targets[0].selected_by_default);
        assert!(
            plan.targets[0]
                .sizing_warnings
                .iter()
                .any(|warning| warning.kind == crate::model::SizingWarningKind::Canceled)
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
