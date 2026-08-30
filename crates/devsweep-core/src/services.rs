use std::{path::Path, sync::Arc};

use anyhow::{Context, Result, bail};

use crate::{
    execution::{ExecutionProgress, ExecutionReport, ExecutionRequest, Executor},
    inventory::{InventoryReport, inventory_root_with_cancel},
    model::{CleanupPlan, ScanHealth, ScanReport},
    plan::{ValidatedPlan, validate_plan, validate_scanned_plan},
    process::FlagCancelObserver,
    scan::{ScanOptions, ScanProgress, ScanReportRunOutcome, Sweeper},
};

#[derive(Debug, Clone, PartialEq, Eq)]
/// In-memory scan result used by interactive frontends.
pub struct ScanServiceOutcome {
    /// Trusted in-memory cleanup plan for presentation.
    pub plan: CleanupPlan,
    /// Non-authoritative scan health observations.
    pub health: ScanHealth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Explicit interactive scan terminal result.
pub enum ScanServiceRunOutcome {
    /// The scan completed and may be promoted to review state.
    Completed(ScanServiceOutcome),
    /// The scan stopped cooperatively and must remain preview-only.
    Canceled,
}

/// Runs cleanup scans for an interactive frontend.
pub trait ScanService: Send + Clone + 'static {
    /// Runs a scan while forwarding progress and observing cancellation.
    fn full_scan_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceOutcome>;

    /// Runs a scan while preserving explicit cancellation semantics. Injected
    /// services keep the legacy completed behavior unless they override it.
    fn full_scan_run_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceRunOutcome> {
        self.full_scan_with_cancel(options, progress, cancel)
            .map(ScanServiceRunOutcome::Completed)
    }
}

/// Revalidates and executes a confirmed cleanup plan.
pub trait CleanService: Send + Clone + 'static {
    /// Revalidates a confirmed plan and runs the requested cleanup mode.
    fn run_plan(
        &self,
        plan: &CleanupPlan,
        expected_digest: &str,
        request: ExecutionRequest,
        on_progress: &mut dyn FnMut(ExecutionProgress),
    ) -> Result<ExecutionReport>;
}

/// Produces read-only capacity inventory reports.
pub trait InventoryService: Send + Clone + 'static {
    /// Inventories one root while observing cooperative cancellation.
    fn inventory_root_with_cancel(
        &self,
        root: &Path,
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<InventoryReport>;
}

#[derive(Clone)]
/// Production scan service backed by [`Sweeper`].
pub struct SweepScanService;

impl ScanService for SweepScanService {
    fn full_scan_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceOutcome> {
        let report = Sweeper::default().full_scan_report_with_cancel(options, progress, cancel)?;
        scan_service_outcome_from_report(report)
    }

    fn full_scan_run_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceRunOutcome> {
        match Sweeper::default().full_scan_report_run_with_cancel(options, progress, cancel)? {
            ScanReportRunOutcome::Completed(report) => {
                scan_service_outcome_from_report(report).map(ScanServiceRunOutcome::Completed)
            }
            ScanReportRunOutcome::Canceled => Ok(ScanServiceRunOutcome::Canceled),
        }
    }
}

#[derive(Clone)]
/// Production inventory service for local filesystem roots.
pub struct LocalInventoryService;

impl InventoryService for LocalInventoryService {
    fn inventory_root_with_cancel(
        &self,
        root: &Path,
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<InventoryReport> {
        inventory_root_with_cancel(root, cancel)
    }
}

fn scan_service_outcome_from_report(report: ScanReport) -> Result<ScanServiceOutcome> {
    let validated = validate_plan(&report.plan)
        .context("scan report plan failed validation before TUI presentation")?;
    Ok(ScanServiceOutcome {
        plan: CleanupPlan {
            version: report.plan.version,
            targets: validated
                .targets()
                .iter()
                .map(|target| target.target().clone())
                .collect(),
        },
        health: report.health,
    })
}

#[derive(Clone)]
/// Production cleanup service backed by [`Executor`].
pub struct ExecutorCleanService;

impl CleanService for ExecutorCleanService {
    fn run_plan(
        &self,
        plan: &CleanupPlan,
        expected_digest: &str,
        request: ExecutionRequest,
        on_progress: &mut dyn FnMut(ExecutionProgress),
    ) -> Result<ExecutionReport> {
        let validated = validate_confirmed_plan(plan, expected_digest)?;
        Executor::default().run_plan_with_progress(&validated, request, on_progress)
    }
}

/// Frontend-neutral Clean workbench adapter over scan and execution services.
#[derive(Clone)]
pub struct CleanModeAdapter {
    scan: SweepScanService,
    clean: ExecutorCleanService,
}

impl Default for CleanModeAdapter {
    fn default() -> Self {
        Self {
            scan: SweepScanService,
            clean: ExecutorCleanService,
        }
    }
}

impl ScanService for CleanModeAdapter {
    fn full_scan_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceOutcome> {
        self.scan.full_scan_with_cancel(options, progress, cancel)
    }

    fn full_scan_run_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceRunOutcome> {
        self.scan
            .full_scan_run_with_cancel(options, progress, cancel)
    }
}

impl CleanService for CleanModeAdapter {
    fn run_plan(
        &self,
        plan: &CleanupPlan,
        expected_digest: &str,
        request: ExecutionRequest,
        on_progress: &mut dyn FnMut(ExecutionProgress),
    ) -> Result<ExecutionReport> {
        self.clean
            .run_plan(plan, expected_digest, request, on_progress)
    }
}

pub(crate) fn validate_confirmed_plan(
    plan: &CleanupPlan,
    expected_digest: &str,
) -> Result<ValidatedPlan> {
    let validated = validate_scanned_plan(plan)?;
    if validated.digest() != expected_digest {
        bail!("confirmed cleanup plan digest does not match the validated plan")
    }
    Ok(validated)
}
