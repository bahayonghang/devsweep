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
    terminal::Tui,
};

mod services;
mod workers;

use services::{CleanService, InventoryService, ScanService};
pub(super) use services::{ExecutorCleanService, LocalInventoryService, SweepScanService};
use workers::{run_clean_worker, run_inventory_worker, run_scan_worker};

type CancelRegistry = Arc<Mutex<HashMap<JobId, Arc<FlagCancelObserver>>>>;

pub(super) fn run_event_loop<S: ScanService, I: InventoryService, C: CleanService>(
    terminal: &mut Tui,
    scan: S,
    inventory: I,
    clean: C,
) -> Result<()> {
    let mut app = App::new();
    let (worker_tx, worker_rx) = mpsc::channel();
    let clean_dispatch_in_flight = Arc::new(AtomicBool::new(false));
    let cancel_registry: CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
    let startup_effects = app.startup_effects();
    dispatch_effects(
        startup_effects,
        &worker_tx,
        &scan,
        &inventory,
        &clean,
        &clean_dispatch_in_flight,
        &cancel_registry,
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
            &cancel_registry,
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
                &cancel_registry,
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
    cancel_registry: &CancelRegistry,
) -> Result<()> {
    while let Ok(event) = worker_rx.try_recv() {
        if let WorkerEvent::CleanFinished { job_id, .. }
        | WorkerEvent::JobFailed { job_id, .. }
        | WorkerEvent::JobCanceled { job_id }
        | WorkerEvent::ScanFinished { job_id, .. }
        | WorkerEvent::InventoryFinished { job_id, .. } = &event
            && let Ok(mut guard) = cancel_registry.lock()
        {
            guard.remove(job_id);
        }
        let effects = app.update(UiEvent::Worker(event));
        dispatch_effects(
            effects,
            worker_tx,
            scan,
            inventory,
            clean,
            clean_dispatch_in_flight,
            cancel_registry,
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
    cancel_registry: &CancelRegistry,
) -> Result<()> {
    for effect in effects {
        dispatch_effect(
            effect,
            worker_tx.clone(),
            scan,
            inventory,
            clean,
            clean_dispatch_in_flight,
            cancel_registry,
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
    cancel_registry: &CancelRegistry,
) -> Result<()> {
    match effect {
        Effect::StartScan { job_id } => {
            let cancel = Arc::new(FlagCancelObserver::new());
            if let Ok(mut guard) = cancel_registry.lock() {
                guard.insert(job_id, Arc::clone(&cancel));
            }
            let scan = scan.clone();
            thread::spawn(move || run_scan_worker(job_id, worker_tx, scan, cancel));
        }
        Effect::StartInventory { job_id } => {
            let cancel = Arc::new(FlagCancelObserver::new());
            if let Ok(mut guard) = cancel_registry.lock() {
                guard.insert(job_id, Arc::clone(&cancel));
            }
            let inventory = inventory.clone();
            thread::spawn(move || run_inventory_worker(job_id, worker_tx, inventory, cancel));
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
            if let Ok(mut guard) = cancel_registry.lock() {
                guard.insert(job_id, Arc::clone(&cancel));
            }
            let clean = clean.clone();
            let clean_dispatch_in_flight = Arc::clone(clean_dispatch_in_flight);
            thread::spawn(move || {
                run_clean_worker(
                    job_id,
                    plan,
                    selected,
                    plan_digest,
                    worker_tx,
                    clean,
                    clean_dispatch_in_flight,
                    cancel,
                )
            });
        }
        Effect::CancelJob { job_id } => {
            if let Ok(guard) = cancel_registry.lock()
                && let Some(flag) = guard.get(&job_id)
            {
                flag.request_cancel();
            }
        }
        Effect::Quit => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests;
