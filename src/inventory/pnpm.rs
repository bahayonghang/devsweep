use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    filesystem::{PathReparseProbe, inspect_path_no_follow, paths_equal},
    model::{ScanDiagnosticStage, ScanHealth},
    process::{
        CwdPolicy, DEFAULT_PROVIDER_PROBE_TIMEOUT, NoopCancelObserver, ProcessRequest,
        ProcessRunner, ProcessStatus,
    },
};

use super::{InventoryClassification, push_diagnostic, unsafe_path_detail};

const MAX_REFERENCE_DEPTH: usize = 4;
const MAX_REFERENCE_FILES: usize = 256;
const MAX_REFERENCE_FILE_BYTES: u64 = 64 * 1024;

/// An inspect-only finding. It has no cleanup intent or action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct OrphanPnpmStoreFinding {
    pub classification: InventoryClassification,
    pub candidate_path: PathBuf,
    pub configured_store: PathBuf,
    pub project_references: Vec<PnpmProjectReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PnpmProjectReference {
    pub path: PathBuf,
    pub references_candidate: bool,
    pub references_configured_store: bool,
}

#[derive(Debug, Default)]
struct PnpmReferenceScan {
    references: Vec<PnpmProjectReference>,
    complete: bool,
}

pub(super) fn inspect_orphan_pnpm_store(
    root: &Path,
    probe: &dyn PathReparseProbe,
    health: &mut ScanHealth,
) -> Option<OrphanPnpmStoreFinding> {
    let candidate_path = root.join(".pnpm-store");
    let metadata = fs::symlink_metadata(&candidate_path).ok()?;
    if !metadata.is_dir()
        || unsafe_path_detail(inspect_path_no_follow(&candidate_path, &metadata, probe)).is_some()
    {
        return None;
    }
    let configured_store = current_pnpm_store()?;
    if paths_equal(&candidate_path, &configured_store) {
        return None;
    }
    let reference_scan =
        collect_pnpm_project_references(root, &candidate_path, &configured_store, probe, health);
    if !reference_scan_supports_orphan_finding(&reference_scan) {
        // Do not claim a store is orphaned without both current configuration
        // and complete, observed project-reference evidence.
        return None;
    }
    Some(OrphanPnpmStoreFinding {
        classification: InventoryClassification::InspectOnly,
        candidate_path,
        configured_store,
        project_references: reference_scan.references,
    })
}

fn reference_scan_supports_orphan_finding(scan: &PnpmReferenceScan) -> bool {
    scan.complete
        && !scan.references.is_empty()
        && scan
            .references
            .iter()
            .all(|reference| !reference.references_candidate)
}

fn current_pnpm_store() -> Option<PathBuf> {
    let pnpm = crate::scan::resolve_executable("pnpm")?;
    let cancel = NoopCancelObserver;
    let result = ProcessRunner::default().run(&ProcessRequest {
        program: pnpm.into_os_string(),
        args: vec!["store".into(), "path".into()],
        cwd: CwdPolicy::Neutral,
        timeout: Some(DEFAULT_PROVIDER_PROBE_TIMEOUT),
        job_deadline: None,
        cancel: &cancel,
    });
    if result.status != ProcessStatus::Success || result.output.stdout_truncated {
        return None;
    }
    String::from_utf8_lossy(&result.output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn collect_pnpm_project_references(
    root: &Path,
    candidate_path: &Path,
    configured_store: &Path,
    probe: &dyn PathReparseProbe,
    health: &mut ScanHealth,
) -> PnpmReferenceScan {
    let mut scan = PnpmReferenceScan {
        complete: true,
        ..PnpmReferenceScan::default()
    };
    collect_pnpm_project_references_at(
        root,
        candidate_path,
        configured_store,
        probe,
        health,
        0,
        &mut scan,
    );
    scan.references
        .sort_by(|left, right| left.path.cmp(&right.path));
    scan
}

#[allow(clippy::too_many_arguments)]
fn collect_pnpm_project_references_at(
    dir: &Path,
    candidate_path: &Path,
    configured_store: &Path,
    probe: &dyn PathReparseProbe,
    health: &mut ScanHealth,
    depth: usize,
    scan: &mut PnpmReferenceScan,
) {
    if depth > MAX_REFERENCE_DEPTH || scan.references.len() >= MAX_REFERENCE_FILES {
        scan.complete = false;
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            push_diagnostic(
                health,
                dir,
                ScanDiagnosticStage::Discovery,
                format!("failed to inspect pnpm project references: {error}"),
            );
            scan.complete = false;
            return;
        }
    };
    for entry in entries {
        if scan.references.len() >= MAX_REFERENCE_FILES {
            scan.complete = false;
            return;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                push_diagnostic(
                    health,
                    dir,
                    ScanDiagnosticStage::Discovery,
                    format!("failed to read pnpm project reference entry: {error}"),
                );
                scan.complete = false;
                continue;
            }
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                push_diagnostic(
                    health,
                    &path,
                    ScanDiagnosticStage::Discovery,
                    format!("failed to inspect pnpm reference path: {error}"),
                );
                scan.complete = false;
                continue;
            }
        };
        if let Some(detail) = unsafe_path_detail(inspect_path_no_follow(&path, &metadata, probe)) {
            push_diagnostic(health, &path, ScanDiagnosticStage::Discovery, detail);
            scan.complete = false;
            continue;
        }
        if metadata.is_dir() {
            if !skip_reference_descent(&path) {
                collect_pnpm_project_references_at(
                    &path,
                    candidate_path,
                    configured_store,
                    probe,
                    health,
                    depth + 1,
                    scan,
                );
            }
            continue;
        }
        if !is_pnpm_reference_file(&path) {
            continue;
        }
        if metadata.len() > MAX_REFERENCE_FILE_BYTES {
            push_diagnostic(
                health,
                &path,
                ScanDiagnosticStage::Discovery,
                format!(
                    "pnpm project reference exceeds {} byte inspection limit",
                    MAX_REFERENCE_FILE_BYTES
                ),
            );
            scan.complete = false;
            continue;
        }
        let Ok(text) = read_bounded_text(&path) else {
            push_diagnostic(
                health,
                &path,
                ScanDiagnosticStage::Discovery,
                "failed to read pnpm project reference".to_string(),
            );
            scan.complete = false;
            continue;
        };
        let candidate_text = candidate_path.to_string_lossy();
        let configured_text = configured_store.to_string_lossy();
        scan.references.push(PnpmProjectReference {
            path,
            references_candidate: text.contains(candidate_text.as_ref()),
            references_configured_store: text.contains(configured_text.as_ref()),
        });
    }
}

fn skip_reference_descent(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | ".pnpm-store" | "node_modules" | "target")
    )
}

fn is_pnpm_reference_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".npmrc" | "pnpm-workspace.yaml" | "pnpm-lock.yaml" | "package.json")
    )
}

fn read_bounded_text(path: &Path) -> Result<String> {
    let file = fs::File::open(path)
        .with_context(|| format!("failed to open pnpm reference {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_REFERENCE_FILE_BYTES)
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read pnpm reference {}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orphan_finding_requires_complete_non_candidate_reference_evidence() {
        let references = vec![PnpmProjectReference {
            path: PathBuf::from("C:/inventory/app/.npmrc"),
            references_candidate: false,
            references_configured_store: true,
        }];
        let scan = PnpmReferenceScan {
            references: references.clone(),
            complete: true,
        };

        assert!(reference_scan_supports_orphan_finding(&scan));
        assert!(!reference_scan_supports_orphan_finding(
            &PnpmReferenceScan {
                references,
                complete: false,
            }
        ));
        assert!(!reference_scan_supports_orphan_finding(
            &PnpmReferenceScan {
                references: vec![PnpmProjectReference {
                    path: PathBuf::from("C:/inventory/app/.npmrc"),
                    references_candidate: true,
                    references_configured_store: false,
                }],
                complete: true,
            }
        ));
    }
}
