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
pub(in crate::tui) struct ScanServiceOutcome {
    pub(super) plan: CleanupPlan,
    pub(super) health: ScanHealth,
}

pub(in crate::tui) trait ScanService: Send + Clone + 'static {
    fn full_scan_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceOutcome>;
}

pub(in crate::tui) trait CleanService: Send + Clone + 'static {
    fn run_plan(
        &self,
        plan: &CleanupPlan,
        expected_digest: &str,
        request: ExecutionRequest,
        on_progress: &mut dyn FnMut(ExecutionProgress),
    ) -> Result<ExecutionReport>;
}

pub(in crate::tui) trait InventoryService: Send + Clone + 'static {
    fn inventory_root_with_cancel(
        &self,
        root: &Path,
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<InventoryReport>;
}

#[derive(Clone)]
pub(in crate::tui) struct SweepScanService;

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
pub(in crate::tui) struct LocalInventoryService;

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
pub(in crate::tui) struct ExecutorCleanService;

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

pub(super) fn validate_confirmed_plan(
    plan: &CleanupPlan,
    expected_digest: &str,
) -> Result<ValidatedPlan> {
    let validated = validate_scanned_plan(plan)?;
    if validated.digest() != expected_digest {
        bail!("confirmed cleanup plan digest does not match the validated plan")
    }
    Ok(validated)
}
