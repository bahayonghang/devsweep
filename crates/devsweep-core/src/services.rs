use std::{path::Path, sync::Arc};

use anyhow::{Context, Result, bail};

use crate::{
    execution::{ExecutionProgress, ExecutionReport, ExecutionRequest, Executor},
    inventory::{InventoryReport, inventory_root_with_cancel},
    model::{CleanupPlan, ScanHealth, ScanReport},
    plan::{ValidatedPlan, validate_plan, validate_scanned_plan},
    process::FlagCancelObserver,
    scan::{ScanOptions, ScanProgress, Sweeper},
};

#[derive(Debug, Clone, PartialEq, Eq)]
/// In-memory scan result used by interactive frontends.
pub struct ScanServiceOutcome {
    /// Trusted in-memory cleanup plan for presentation.
    pub plan: CleanupPlan,
    /// Non-authoritative scan health observations.
    pub health: ScanHealth,
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
