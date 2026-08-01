use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    fs_size::{
        DEFAULT_SIZE_ENTRY_BUDGET, PathReparseProbe, PathSafety, SystemPathReparseProbe,
        estimate_tree_with_budget_and_cancel_and_probe, inspect_path_no_follow,
    },
    model::{
        ScanDiagnostic, ScanDiagnosticOutcome, ScanDiagnosticStage, ScanHealth, ScanTotals,
        SizingWarning,
    },
    path_identity::paths_equal,
    process_runner::{
        CancelObserver, CwdPolicy, FlagCancelObserver, NoopCancelObserver, ProcessRequest,
        ProcessRunner, ProcessStatus,
    },
};

pub const INVENTORY_REPORT_VERSION: u32 = 1;
pub const DEFAULT_INVENTORY_ENTRY_BUDGET: usize = DEFAULT_SIZE_ENTRY_BUDGET * 2;

const MAX_REFERENCE_DEPTH: usize = 4;
const MAX_REFERENCE_FILES: usize = 256;
const MAX_REFERENCE_FILE_BYTES: u64 = 64 * 1024;

/// Read-only capacity observations. These values never enter a cleanup plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryReport {
    pub version: u32,
    pub root: PathBuf,
    pub observations: Vec<CapacityObservation>,
    pub health: ScanHealth,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan_pnpm_store: Option<OrphanPnpmStoreFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapacityObservation {
    pub path: PathBuf,
    pub classification: InventoryClassification,
    pub estimated_bytes: u64,
    pub size_complete: bool,
    pub warnings: Vec<SizingWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryClassification {
    InventoryOnly,
    InspectOnly,
}

/// An inspect-only finding. It has no cleanup intent or action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrphanPnpmStoreFinding {
    pub classification: InventoryClassification,
    pub candidate_path: PathBuf,
    pub configured_store: PathBuf,
    pub project_references: Vec<PnpmProjectReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PnpmProjectReference {
    pub path: PathBuf,
    pub references_candidate: bool,
    pub references_configured_store: bool,
}

#[derive(Debug, Default)]
struct PnpmReferenceScan {
    references: Vec<PnpmProjectReference>,
    complete: bool,
}

pub fn inventory_root(root: &Path) -> Result<InventoryReport> {
    inventory_root_with_cancel(root, None)
}

/// Produces read-only observations while honoring a TUI cancellation request.
pub fn inventory_root_with_cancel(
    root: &Path,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<InventoryReport> {
    let probe = SystemPathReparseProbe;
    inventory_root_with_probe_and_cancel(root, &probe, cancel)
}

#[cfg(test)]
fn inventory_root_with_probe(root: &Path, probe: &dyn PathReparseProbe) -> Result<InventoryReport> {
    inventory_root_with_probe_and_cancel(root, probe, None)
}

fn inventory_root_with_probe_and_cancel(
    root: &Path,
    probe: &dyn PathReparseProbe,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> Result<InventoryReport> {
    let mut health = ScanHealth::complete();
    if cancel_requested(cancel) {
        push_canceled_diagnostic(&mut health, root);
        return Ok(InventoryReport {
            version: INVENTORY_REPORT_VERSION,
            root: root.to_path_buf(),
            observations: Vec::new(),
            health,
            orphan_pnpm_store: None,
        });
    }
    let metadata = fs::symlink_metadata(root)
        .with_context(|| format!("failed to inspect inventory root {}", root.display()))?;
    if !metadata.is_dir() {
        anyhow::bail!("inventory root is not a directory: {}", root.display());
    }

    if let Some(detail) = unsafe_path_detail(inspect_path_no_follow(root, &metadata, probe)) {
        push_diagnostic(&mut health, root, ScanDiagnosticStage::Discovery, detail);
        return Ok(InventoryReport {
            version: INVENTORY_REPORT_VERSION,
            root: root.to_path_buf(),
            observations: Vec::new(),
            health,
            orphan_pnpm_store: None,
        });
    }

    let entries = fs::read_dir(root)
        .with_context(|| format!("failed to read inventory root {}", root.display()))?;
    let mut observations = Vec::new();
    let mut canceled = false;
    for entry in entries {
        if cancel_requested(cancel) {
            canceled = true;
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                push_diagnostic(
                    &mut health,
                    root,
                    ScanDiagnosticStage::Discovery,
                    format!("failed to read inventory entry: {error}"),
                );
                continue;
            }
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                push_diagnostic(
                    &mut health,
                    &path,
                    ScanDiagnosticStage::Discovery,
                    format!("failed to inspect inventory entry: {error}"),
                );
                continue;
            }
        };
        if let Some(detail) = unsafe_path_detail(inspect_path_no_follow(&path, &metadata, probe)) {
            push_diagnostic(&mut health, &path, ScanDiagnosticStage::Discovery, detail);
            continue;
        }
        if !metadata.is_dir() && !metadata.is_file() {
            continue;
        }

        let estimate = estimate_tree_with_budget_and_cancel_and_probe(
            &path,
            DEFAULT_INVENTORY_ENTRY_BUDGET,
            cancel,
            probe,
        );
        let estimated_bytes = estimate.display_bytes();
        if !estimate.complete {
            health.mark_partial();
        }
        for warning in &estimate.warnings {
            push_diagnostic(
                &mut health,
                &path,
                ScanDiagnosticStage::Sizing,
                warning.detail.clone(),
            );
        }
        observations.push(CapacityObservation {
            path,
            classification: InventoryClassification::InventoryOnly,
            estimated_bytes,
            size_complete: estimate.complete,
            warnings: estimate.warnings,
        });
        if cancel_requested(cancel) {
            canceled = true;
            break;
        }
    }

    if canceled {
        push_canceled_diagnostic(&mut health, root);
    }

    observations.sort_by(|left, right| {
        right
            .estimated_bytes
            .cmp(&left.estimated_bytes)
            .then_with(|| left.path.cmp(&right.path))
    });
    health.totals = totals_from_observations(&observations);
    let orphan_pnpm_store = if canceled || cancel_requested(cancel) {
        if !canceled {
            push_canceled_diagnostic(&mut health, root);
        }
        None
    } else {
        inspect_orphan_pnpm_store(root, probe, &mut health)
    };

    Ok(InventoryReport {
        version: INVENTORY_REPORT_VERSION,
        root: root.to_path_buf(),
        observations,
        health,
        orphan_pnpm_store,
    })
}

fn cancel_requested(cancel: Option<&Arc<FlagCancelObserver>>) -> bool {
    cancel.is_some_and(|flag| flag.is_cancel_requested())
}

fn totals_from_observations(observations: &[CapacityObservation]) -> ScanTotals {
    let mut totals = ScanTotals::default();
    for observation in observations {
        if observation.size_complete {
            totals.verified_bytes = totals
                .verified_bytes
                .saturating_add(observation.estimated_bytes);
        } else if observation.estimated_bytes == 0 {
            totals.unknown_target_count = totals.unknown_target_count.saturating_add(1);
        } else {
            totals.partial_lower_bound_bytes = totals
                .partial_lower_bound_bytes
                .saturating_add(observation.estimated_bytes);
        }
    }
    totals
}

fn inspect_orphan_pnpm_store(
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
    let pnpm = crate::providers::resolve_executable("pnpm")?;
    let cancel = NoopCancelObserver;
    let result = ProcessRunner::default().run(&ProcessRequest {
        program: pnpm.into_os_string(),
        args: vec!["store".into(), "path".into()],
        cwd: CwdPolicy::Neutral,
        timeout: Some(crate::process_runner::DEFAULT_PROVIDER_PROBE_TIMEOUT),
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

fn unsafe_path_detail(path_safety: PathSafety) -> Option<String> {
    match path_safety {
        PathSafety::Safe => None,
        PathSafety::ReparsePoint { tag: Some(tag) } => {
            Some(format!("skipped reparse point with tag 0x{tag:08x}"))
        }
        PathSafety::ReparsePoint { tag: None } => {
            Some("skipped symlink, junction, or reparse point".to_string())
        }
        PathSafety::Unverified { detail } => Some(format!(
            "skipped path because reparse safety could not be verified: {detail}"
        )),
    }
}

fn push_diagnostic(
    health: &mut ScanHealth,
    path: &Path,
    stage: ScanDiagnosticStage,
    detail: String,
) {
    health.mark_partial();
    health.diagnostics.push(ScanDiagnostic {
        stage,
        path: path.to_path_buf(),
        outcome: ScanDiagnosticOutcome::Skipped,
        detail,
        process: None,
    });
}

fn push_canceled_diagnostic(health: &mut ScanHealth, path: &Path) {
    health.mark_partial();
    health.diagnostics.push(ScanDiagnostic {
        stage: ScanDiagnosticStage::Sizing,
        path: path.to_path_buf(),
        outcome: ScanDiagnosticOutcome::Canceled,
        detail: "inventory canceled before all capacity observations completed".to_string(),
        process: None,
    });
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        sync::{Arc, Mutex},
    };

    use tempfile::TempDir;

    use super::*;
    use crate::{
        fs_size::ReparseProbeResult,
        model::{ScanCompleteness, ScanDiagnosticOutcome},
        process_runner::FlagCancelObserver,
    };

    #[derive(Default)]
    struct FixtureProbe {
        tagged: Mutex<HashMap<PathBuf, u32>>,
    }

    impl FixtureProbe {
        fn tagged(path: &Path) -> Self {
            Self {
                tagged: Mutex::new(HashMap::from([(path.to_path_buf(), 0x9000_701a)])),
            }
        }
    }

    impl PathReparseProbe for FixtureProbe {
        fn probe(&self, path: &Path) -> ReparseProbeResult {
            self.tagged
                .lock()
                .expect("fixture probe lock")
                .get(path)
                .copied()
                .map(|tag| ReparseProbeResult::ReparsePoint { tag })
                .unwrap_or(ReparseProbeResult::NotReparsePoint)
        }
    }

    #[test]
    fn inventory_reports_read_only_top_level_observations() {
        let fixture = TempDir::new().expect("temp dir");
        fs::create_dir_all(fixture.path().join("small")).expect("small dir");
        fs::write(fixture.path().join("small/data.bin"), b"small").expect("small file");
        fs::create_dir_all(fixture.path().join("large")).expect("large dir");
        fs::write(fixture.path().join("large/data.bin"), vec![b'x'; 128]).expect("large file");

        let report = inventory_root(fixture.path()).expect("inventory succeeds");

        assert_eq!(report.version, INVENTORY_REPORT_VERSION);
        assert_eq!(report.observations.len(), 2);
        assert!(report.observations[0].estimated_bytes >= report.observations[1].estimated_bytes);
        assert!(report.observations.iter().all(
            |observation| observation.classification == InventoryClassification::InventoryOnly
        ));
        assert!(report.orphan_pnpm_store.is_none());
    }

    #[test]
    fn canceled_inventory_returns_partial_read_only_report() {
        let fixture = TempDir::new().expect("temp dir");
        fs::write(fixture.path().join("observed.bin"), b"observed")
            .expect("inventory fixture writes");
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();

        let report = inventory_root_with_cancel(fixture.path(), Some(&cancel))
            .expect("canceled inventory returns report");

        assert!(report.observations.is_empty());
        assert_eq!(report.health.completeness, ScanCompleteness::Partial);
        assert_eq!(
            report.health.diagnostics[0].outcome,
            ScanDiagnosticOutcome::Canceled
        );
    }

    #[test]
    fn inventory_skips_injected_cloud_files_reparse_root_without_an_observation() {
        let fixture = TempDir::new().expect("temp dir");
        let root = fixture.path().join("cloud-root");
        fs::create_dir_all(&root).expect("root dir");
        let probe = Arc::new(FixtureProbe::tagged(&root));

        let report = inventory_root_with_probe(&root, probe.as_ref()).expect("inventory succeeds");

        assert!(report.observations.is_empty());
        assert_eq!(report.health.completeness, ScanCompleteness::Partial);
        assert!(report.health.diagnostics[0].detail.contains("0x9000701a"));
    }

    #[test]
    fn inventory_estimation_uses_the_injected_reparse_probe_for_nested_children() {
        let fixture = TempDir::new().expect("temp dir");
        let project = fixture.path().join("project");
        let cloud_child = project.join("cloud");
        fs::create_dir_all(&cloud_child).expect("project directory");
        fs::write(project.join("visible.bin"), b"visible").expect("visible file");
        fs::write(cloud_child.join("hidden.bin"), vec![b'x'; 128]).expect("hidden file");
        let probe = FixtureProbe::tagged(&cloud_child);

        let report = inventory_root_with_probe(fixture.path(), &probe).expect("inventory succeeds");
        let observation = report
            .observations
            .iter()
            .find(|observation| observation.path == project)
            .expect("project observation");

        assert_eq!(observation.estimated_bytes, 7);
        assert!(observation.size_complete);
    }

    #[test]
    fn orphan_pnpm_finding_requires_complete_non_candidate_reference_evidence() {
        let candidate = PathBuf::from("C:/inventory/.pnpm-store");
        let configured = PathBuf::from("C:/Users/test/AppData/Local/pnpm/store");
        let references = vec![PnpmProjectReference {
            path: PathBuf::from("C:/inventory/app/.npmrc"),
            references_candidate: false,
            references_configured_store: true,
        }];
        let scan = PnpmReferenceScan {
            references: references.clone(),
            complete: true,
        };

        let finding = OrphanPnpmStoreFinding {
            classification: InventoryClassification::InspectOnly,
            candidate_path: candidate.clone(),
            configured_store: configured.clone(),
            project_references: references.clone(),
        };

        assert_eq!(finding.classification, InventoryClassification::InspectOnly);
        assert_ne!(finding.candidate_path, finding.configured_store);
        assert!(!finding.project_references.is_empty());
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

    #[test]
    fn inventory_json_has_no_cleanup_authority_fields() {
        let report = InventoryReport {
            version: INVENTORY_REPORT_VERSION,
            root: PathBuf::from("C:/inventory"),
            observations: vec![CapacityObservation {
                path: PathBuf::from("C:/inventory/media"),
                classification: InventoryClassification::InventoryOnly,
                estimated_bytes: 1024,
                size_complete: true,
                warnings: Vec::new(),
            }],
            health: ScanHealth::complete(),
            orphan_pnpm_store: None,
        };

        let json = serde_json::to_string(&report).expect("inventory serializes");

        assert!(!json.contains("\"intent\""));
        assert!(!json.contains("\"action\""));
        assert!(!json.contains("\"selected_by_default\""));
    }
}
