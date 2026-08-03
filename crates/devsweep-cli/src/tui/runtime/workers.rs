use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
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
use devsweep_core::services::{CleanService, InventoryService, ScanService};

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
            scan.full_scan_with_cancel(
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
        Ok(outcome) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
            let _ = worker_tx.send(WorkerEvent::ScanFinished {
                job_id,
                plan: outcome.plan,
                health: outcome.health,
            });
        }
        Ok(outcome) => {
            let _ = worker_tx.send(WorkerEvent::ScanFinished {
                job_id,
                plan: outcome.plan,
                health: outcome.health,
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
