use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent};

use crate::process::FlagCancelObserver;

use super::{
    app::{App, Effect, JobId, UiEvent, WorkerEvent},
    render::render_app,
    shell::{ShellComposition, persist_language},
    terminal::Tui,
};

mod workers;

use devsweep_core::services::{CleanService, InventoryService, ScanService};
pub(super) use devsweep_core::services::{
    ExecutorCleanService, LocalInventoryService, SweepScanService,
};
use workers::{
    run_analyze_worker, run_clean_worker, run_inventory_worker, run_scan_worker,
    run_software_audit_worker, run_software_inventory_worker, run_software_preview_worker,
    run_software_uninstall_worker,
};

struct WorkerRegistration {
    cancel: Arc<FlagCancelObserver>,
    join: thread::JoinHandle<()>,
}

type WorkerRegistry = Arc<Mutex<HashMap<JobId, WorkerRegistration>>>;

pub(super) fn run_event_loop<S: ScanService, I: InventoryService, C: CleanService>(
    terminal: &mut Tui,
    scan: S,
    inventory: I,
    clean: C,
    shell: ShellComposition,
) -> Result<()> {
    let mut app = App::with_shell(shell);
    let (worker_tx, worker_rx) = mpsc::channel();
    let clean_dispatch_in_flight = Arc::new(AtomicBool::new(false));
    let worker_registry: WorkerRegistry = Arc::new(Mutex::new(HashMap::new()));
    let startup_effects = app.startup_effects();
    dispatch_effects(
        startup_effects,
        &worker_tx,
        &scan,
        &inventory,
        &clean,
        &clean_dispatch_in_flight,
        &worker_registry,
    )?;

    while !app.should_quit {
        drain_worker_events(
            &mut app,
            &worker_rx,
            &worker_tx,
            &scan,
            &inventory,
            &clean,
            &clean_dispatch_in_flight,
            &worker_registry,
        )?;
        terminal.draw(|frame| render_app(frame, &app))?;

        if event::poll(Duration::from_millis(100))?
            && let CrosstermEvent::Key(key) = event::read()?
        {
            let effects = app.update(UiEvent::Key(key));
            dispatch_effects(
                effects,
                &worker_tx,
                &scan,
                &inventory,
                &clean,
                &clean_dispatch_in_flight,
                &worker_registry,
            )?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn drain_worker_events<S: ScanService, I: InventoryService, C: CleanService>(
    app: &mut App,
    worker_rx: &Receiver<WorkerEvent>,
    worker_tx: &Sender<WorkerEvent>,
    scan: &S,
    inventory: &I,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    worker_registry: &WorkerRegistry,
) -> Result<()> {
    while let Ok(event) = worker_rx.try_recv() {
        join_terminal_worker(&event, worker_registry)?;
        let effects = app.update(UiEvent::Worker(event));
        dispatch_effects(
            effects,
            worker_tx,
            scan,
            inventory,
            clean,
            clean_dispatch_in_flight,
            worker_registry,
        )?;
    }
    Ok(())
}

fn dispatch_effects<S: ScanService, I: InventoryService, C: CleanService>(
    effects: Vec<Effect>,
    worker_tx: &Sender<WorkerEvent>,
    scan: &S,
    inventory: &I,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    worker_registry: &WorkerRegistry,
) -> Result<()> {
    for effect in effects {
        dispatch_effect(
            effect,
            worker_tx.clone(),
            scan,
            inventory,
            clean,
            clean_dispatch_in_flight,
            worker_registry,
        )?;
    }
    Ok(())
}

fn dispatch_effect<S: ScanService, I: InventoryService, C: CleanService>(
    effect: Effect,
    worker_tx: Sender<WorkerEvent>,
    scan: &S,
    inventory: &I,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    worker_registry: &WorkerRegistry,
) -> Result<()> {
    match effect {
        Effect::StartAnalyze { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned before Analyze start"))?
                .is_empty()
            {
                let _ = worker_tx.send(WorkerEvent::JobFailed {
                    job_id,
                    message: "Analyze dispatch rejected: prior heavy work has not joined"
                        .to_string(),
                });
                return Ok(());
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || run_analyze_worker(job_id, worker_tx, worker_cancel));
            worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned after Analyze start"))?
                .insert(job_id, WorkerRegistration { cancel, join });
        }
        Effect::StartScan { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned before scan start"))?
                .is_empty()
            {
                let _ = worker_tx.send(WorkerEvent::JobFailed {
                    job_id,
                    message: "Scan dispatch rejected: prior heavy work has not joined".to_string(),
                });
                return Ok(());
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let scan = scan.clone();
            let worker_cancel = Arc::clone(&cancel);
            let join =
                thread::spawn(move || run_scan_worker(job_id, worker_tx, scan, worker_cancel));
            worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned after scan start"))?
                .insert(job_id, WorkerRegistration { cancel, join });
        }
        Effect::StartInventory { job_id } => {
            let cancel = Arc::new(FlagCancelObserver::new());
            let inventory = inventory.clone();
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || {
                run_inventory_worker(job_id, worker_tx, inventory, worker_cancel)
            });
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after inventory start")
                })?
                .insert(job_id, WorkerRegistration { cancel, join });
        }
        Effect::StartSoftwareInventory { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Software inventory start")
                })?
                .is_empty()
            {
                let _ = worker_tx.send(WorkerEvent::JobFailed {
                    job_id,
                    message: "Software inventory rejected: prior work has not joined".to_string(),
                });
                return Ok(());
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || {
                run_software_inventory_worker(job_id, worker_tx, worker_cancel)
            });
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Software inventory start")
                })?
                .insert(job_id, WorkerRegistration { cancel, join });
        }
        Effect::StartSoftwarePreview {
            job_id,
            inventory,
            selected_ids,
        } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Software preview start")
                })?
                .is_empty()
            {
                let _ = worker_tx.send(WorkerEvent::JobFailed {
                    job_id,
                    message: "Software preview rejected: prior work has not joined".to_string(),
                });
                return Ok(());
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || {
                run_software_preview_worker(
                    job_id,
                    inventory,
                    selected_ids,
                    worker_tx,
                    worker_cancel,
                )
            });
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Software preview start")
                })?
                .insert(job_id, WorkerRegistration { cancel, join });
        }
        Effect::StartSoftwareUninstall {
            job_id,
            plan,
            preview_digest,
        } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Software uninstall start")
                })?
                .is_empty()
            {
                let _ = worker_tx.send(WorkerEvent::JobFailed {
                    job_id,
                    message: "Software uninstall rejected: prior work has not joined".to_string(),
                });
                return Ok(());
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || {
                run_software_uninstall_worker(
                    job_id,
                    plan,
                    preview_digest,
                    worker_tx,
                    worker_cancel,
                )
            });
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Software uninstall start")
                })?
                .insert(job_id, WorkerRegistration { cancel, join });
        }
        Effect::StartSoftwareAudit { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Software audit start")
                })?
                .is_empty()
            {
                let _ = worker_tx.send(WorkerEvent::JobFailed {
                    job_id,
                    message: "Software audit rejected: prior work has not joined".to_string(),
                });
                return Ok(());
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join =
                thread::spawn(move || run_software_audit_worker(job_id, worker_tx, worker_cancel));
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Software audit start")
                })?
                .insert(job_id, WorkerRegistration { cancel, join });
        }
        Effect::StartClean {
            job_id,
            plan,
            selected,
            plan_digest,
        } => {
            if clean_dispatch_in_flight
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                let _ = worker_tx.send(WorkerEvent::JobFailed {
                    job_id,
                    message: "Cleanup dispatch rejected: another cleanup job is already running"
                        .to_string(),
                });
                return Ok(());
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let clean = clean.clone();
            let clean_dispatch_in_flight = Arc::clone(clean_dispatch_in_flight);
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || {
                run_clean_worker(
                    job_id,
                    plan,
                    selected,
                    plan_digest,
                    worker_tx,
                    clean,
                    clean_dispatch_in_flight,
                    worker_cancel,
                )
            });
            worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned after clean start"))?
                .insert(job_id, WorkerRegistration { cancel, join });
        }
        Effect::CancelJob { job_id } => {
            if let Some(worker) = worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned during cancel"))?
                .get(&job_id)
            {
                worker.cancel.request_cancel();
            }
        }
        Effect::SavePresentationLanguage { request_id, locale } => {
            dispatch_presentation_language(request_id, locale, &worker_tx, persist_language);
        }
        Effect::Quit => {}
    }

    Ok(())
}

fn join_terminal_worker(event: &WorkerEvent, worker_registry: &WorkerRegistry) -> Result<()> {
    let job_id = match event {
        WorkerEvent::CleanFinished { job_id, .. }
        | WorkerEvent::AnalyzeFinished { job_id, .. }
        | WorkerEvent::SoftwareInventoryFinished { job_id, .. }
        | WorkerEvent::SoftwarePreviewFinished { job_id, .. }
        | WorkerEvent::SoftwareUninstallFinished { job_id, .. }
        | WorkerEvent::SoftwareAuditFinished { job_id, .. }
        | WorkerEvent::JobFailed { job_id, .. }
        | WorkerEvent::JobCanceled { job_id }
        | WorkerEvent::ScanFinished { job_id, .. }
        | WorkerEvent::InventoryFinished { job_id, .. } => *job_id,
        _ => return Ok(()),
    };

    let worker = worker_registry
        .lock()
        .map_err(|_| anyhow::anyhow!("worker registry lock poisoned before join"))?
        .remove(&job_id);
    if let Some(worker) = worker {
        worker
            .join
            .join()
            .map_err(|_| anyhow::anyhow!("worker {job_id} panicked before join"))?;
    }
    Ok(())
}

fn dispatch_presentation_language<F>(
    request_id: u64,
    locale: crate::i18n::Locale,
    worker_tx: &Sender<WorkerEvent>,
    persist: F,
) where
    F: FnOnce(crate::i18n::Locale) -> Result<()>,
{
    let event = match persist(locale) {
        Ok(()) => WorkerEvent::PresentationLanguageSaved { request_id, locale },
        Err(_error) => WorkerEvent::PresentationLanguageSaveFailed { request_id },
    };
    let _ = worker_tx.send(event);
}

#[cfg(test)]
mod tests;
