use std::{
    fs,
    io::ErrorKind,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    time::SystemTime,
};

use anyhow::Context;

use crate::{
    execution::ExecutionRequest,
    model::{CleanupPlan, TargetId},
    process::{CancelObserver, FlagCancelObserver},
    scan::{ScanOptions, ScanProgress},
};

use crate::tui::{
    app::{JobId, WorkerEvent},
    display::compact_target_id,
};
use devsweep_core::services::{CleanService, InventoryService, ScanService, ScanServiceRunOutcome};
use devsweep_core::{
    analysis::analyze_path,
    optimize::{
        MaintenanceExecutionRequest, MaintenanceExecutor, MaintenancePlanV1,
        OPTIMIZE_AUDIT_VERSION, catalogue_entries, optimize_audit_v1_path, plan_operation,
        preview_maintenance_plan_live,
    },
    software::{
        SOFTWARE_EXECUTION_VERSION, SoftwareExecutionReportV1, SoftwareExecutionRequest,
        SoftwareExecutor, SoftwareInventorySource, SoftwareInventoryV1, SoftwareSelectionPlanV1,
        build_selection_plan, inventory_software, preview_selection_plan_live,
    },
};

pub(super) fn run_analyze_worker(
    job_id: JobId,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    let _ = worker_tx.send(WorkerEvent::AnalyzeStarted { job_id });
    let result = std::env::current_dir()
        .context("failed to get current directory for Analyze")
        .and_then(|root| {
            let progress_tx = worker_tx.clone();
            let mut progress = move |batch| {
                let _ = progress_tx.send(WorkerEvent::AnalyzeProgress {
                    job_id,
                    progress: batch,
                });
            };
            analyze_path(&root, Some(cancel.as_ref()), Some(&mut progress))
        });
    match result {
        Ok(outcome) => {
            let _ = worker_tx.send(WorkerEvent::AnalyzeFinished { job_id, outcome });
        }
        Err(_error) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

pub(super) fn run_software_inventory_worker(
    job_id: JobId,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    let result = inventory_software(SoftwareInventorySource::All, Some(&cancel));
    match result {
        Ok(_inventory) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Ok(inventory) => {
            let _ = worker_tx.send(WorkerEvent::SoftwareInventoryFinished { job_id, inventory });
        }
        Err(_error) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

pub(super) fn run_software_preview_worker(
    job_id: JobId,
    inventory: SoftwareInventoryV1,
    selected_ids: Vec<String>,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    let result = (|| {
        let plan = build_selection_plan(&inventory, &selected_ids, unix_ms_now())?;
        if cancel.is_cancel_requested() {
            return Ok(None);
        }
        let live = inventory_software(SoftwareInventorySource::Msix, Some(&cancel))?;
        if cancel.is_cancel_requested() {
            return Ok(None);
        }
        let preview = preview_selection_plan_live(&plan, &live, unix_ms_now())?;
        Ok::<_, anyhow::Error>(Some((plan, preview)))
    })();
    match result {
        Ok(Some((plan, preview))) => {
            let _ = worker_tx.send(WorkerEvent::SoftwarePreviewFinished {
                job_id,
                plan,
                preview,
            });
        }
        Ok(None) => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(_) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

pub(super) fn run_software_uninstall_worker(
    job_id: JobId,
    plan: SoftwareSelectionPlanV1,
    preview_digest: String,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    let result = SoftwareExecutor::default().execute(SoftwareExecutionRequest {
        plan: &plan,
        expected_preview_digest: &preview_digest,
        confirmed: true,
        cancel: Some(Arc::clone(&cancel)),
    });
    match result {
        Ok(report) => {
            let _ = worker_tx.send(WorkerEvent::SoftwareUninstallFinished { job_id, report });
        }
        Err(_error) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

pub(super) fn run_software_audit_worker(
    job_id: JobId,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    if cancel.is_cancel_requested() {
        let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        return;
    }
    match SoftwareExecutor::default().recover_startup() {
        Ok(_outcomes) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Ok(outcomes) => {
            let _ = worker_tx.send(WorkerEvent::SoftwareAuditFinished {
                job_id,
                report: SoftwareExecutionReportV1 {
                    version: SOFTWARE_EXECUTION_VERSION,
                    irreversible: true,
                    outcomes,
                },
            });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

pub(super) fn run_optimize_list_worker(
    job_id: JobId,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    if cancel.is_cancel_requested() {
        let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        return;
    }
    let _ = worker_tx.send(WorkerEvent::OptimizeListFinished {
        job_id,
        entries: catalogue_entries().to_vec(),
    });
}

pub(super) fn run_optimize_preview_worker(
    job_id: JobId,
    catalogue_id: String,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    let result = (|| {
        let plan = plan_operation(&catalogue_id)?;
        if cancel.is_cancel_requested() {
            return Ok(None);
        }
        let preview = preview_maintenance_plan_live(&plan)?;
        Ok::<_, anyhow::Error>(Some((plan, preview)))
    })();
    match result {
        Ok(Some((plan, preview))) => {
            let _ = worker_tx.send(WorkerEvent::OptimizePreviewFinished {
                job_id,
                plan,
                preview,
            });
        }
        Ok(None) => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(_) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

pub(super) fn run_optimize_run_worker(
    job_id: JobId,
    plan: MaintenancePlanV1,
    preview_digest: String,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    let result = MaintenanceExecutor::default().execute(MaintenanceExecutionRequest {
        plan: &plan,
        expected_preview_digest: &preview_digest,
        confirmed: true,
        cancel: Some(Arc::clone(&cancel)),
    });
    match result {
        Ok(report) => {
            let _ = worker_tx.send(WorkerEvent::OptimizeRunFinished { job_id, report });
        }
        Err(_error) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

pub(super) fn run_optimize_audit_worker(
    job_id: JobId,
    worker_tx: Sender<WorkerEvent>,
    cancel: Arc<FlagCancelObserver>,
) {
    if cancel.is_cancel_requested() {
        let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        return;
    }
    match (|| {
        let recovered = MaintenanceExecutor::default().recover_startup()?;
        let records = read_optimize_audit_records()?;
        Ok::<_, anyhow::Error>((recovered, records))
    })() {
        Ok((_recovered, _records)) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Ok((recovered, records)) => {
            let _ = worker_tx.send(WorkerEvent::OptimizeAuditFinished {
                job_id,
                recovered,
                records,
            });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

fn read_optimize_audit_records()
-> anyhow::Result<Vec<devsweep_core::optimize::OptimizeAuditRecordV1>> {
    let path = optimize_audit_v1_path()?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: devsweep_core::optimize::OptimizeAuditRecordV1 = serde_json::from_str(line)
            .with_context(|| format!("invalid Optimize audit record on line {}", index + 1))?;
        if record.schema_version != OPTIMIZE_AUDIT_VERSION || record.domain != "optimize" {
            anyhow::bail!("unsupported Optimize audit record on line {}", index + 1);
        }
        records.push(record);
    }
    Ok(records)
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

pub(super) fn run_scan_worker<S: ScanService>(
    job_id: JobId,
    worker_tx: Sender<WorkerEvent>,
    scan: S,
    cancel: Arc<FlagCancelObserver>,
) {
    let _ = worker_tx.send(WorkerEvent::ScanStarted { job_id });

    let result = std::env::current_dir()
        .context("failed to get current directory")
        .and_then(|current_dir| {
            let options = ScanOptions {
                include_projects: true,
                include_global: true,
                roots: vec![current_dir],
            };
            scan.full_scan_run_with_cancel(
                &options,
                &mut |progress: ScanProgress| {
                    let _ = worker_tx.send(WorkerEvent::ScanProgress {
                        job_id,
                        phase: progress.phase,
                        message: progress.message,
                        plan: progress.partial,
                    });
                },
                Some(&cancel),
            )
        });
    match result {
        Ok(ScanServiceRunOutcome::Completed(outcome)) => {
            let _ = worker_tx.send(WorkerEvent::ScanFinished {
                job_id,
                plan: outcome.plan,
                health: outcome.health,
            });
        }
        Ok(ScanServiceRunOutcome::Canceled) => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(_error) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

pub(super) fn run_inventory_worker<I: InventoryService>(
    job_id: JobId,
    worker_tx: Sender<WorkerEvent>,
    inventory: I,
    cancel: Arc<FlagCancelObserver>,
) {
    let _ = worker_tx.send(WorkerEvent::InventoryStarted { job_id });

    let result = std::env::current_dir()
        .context("failed to get current directory")
        .and_then(|current_dir| inventory.inventory_root_with_cancel(&current_dir, Some(&cancel)));
    match result {
        Ok(report) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
            let _ = worker_tx.send(WorkerEvent::InventoryFinished {
                job_id,
                report: Box::new(report),
            });
        }
        Ok(report) => {
            let _ = worker_tx.send(WorkerEvent::InventoryFinished {
                job_id,
                report: Box::new(report),
            });
        }
        Err(_error) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_clean_worker<C: CleanService>(
    job_id: JobId,
    plan: CleanupPlan,
    selected: Vec<TargetId>,
    plan_digest: String,
    worker_tx: Sender<WorkerEvent>,
    clean: C,
    clean_dispatch_in_flight: Arc<AtomicBool>,
    cancel: Arc<FlagCancelObserver>,
) {
    let _ = worker_tx.send(WorkerEvent::JobProgress {
        job_id,
        message: "Executing selected cleanup plan".to_string(),
    });

    let result = clean.run_plan(
        &plan,
        &plan_digest,
        ExecutionRequest {
            execute: true,
            audit_log: None,
            selected,
            expected_digest: None,
            cancel: Some(Arc::clone(&cancel)),
        },
        &mut |progress| {
            let target_id = progress.target_id;
            let detail = progress.message;
            let _ = worker_tx.send(WorkerEvent::CleanProgress {
                job_id,
                target_id: target_id.clone(),
                status: progress.status,
                message: format!("{}: {}", compact_target_id(&target_id), detail),
                detail,
                completed: progress.completed,
                total: progress.total,
            });
        },
    );

    match result {
        Ok(_report) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Ok(report) => {
            let _ = worker_tx.send(WorkerEvent::CleanFinished { job_id, report });
        }
        Err(_error) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }

    clean_dispatch_in_flight.store(false, Ordering::Release);
}
