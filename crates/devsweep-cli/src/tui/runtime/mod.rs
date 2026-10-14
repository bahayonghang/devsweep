use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
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

mod mailbox;
mod workers;
use mailbox::{WorkerReceiver, WorkerSender};

use devsweep_core::services::{CleanService, InventoryService, ScanService};
pub(super) use devsweep_core::services::{
    ExecutorCleanService, LocalInventoryService, SweepScanService,
};
use workers::{
    run_analyze_worker, run_clean_worker, run_inventory_worker, run_optimize_audit_worker,
    run_optimize_list_worker, run_optimize_preview_worker, run_optimize_run_worker,
    run_scan_worker, run_software_audit_worker, run_software_inventory_worker,
    run_software_preview_worker, run_software_uninstall_worker, run_status_live_worker,
    run_status_snapshot_worker,
};

struct WorkerRegistration {
    cancel: Arc<FlagCancelObserver>,
    join: thread::JoinHandle<()>,
    cancel_on_shutdown: bool,
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
    let (worker_tx, worker_rx) = mailbox::channel();
    let clean_dispatch_in_flight = Arc::new(AtomicBool::new(false));
    let worker_registry: WorkerRegistry = Arc::new(Mutex::new(HashMap::new()));
    let outcome = (|| {
        let startup_effects = app.startup_effects();
        dispatch_ui_effects(
            &mut app,
            startup_effects,
            &worker_tx,
            &scan,
            &inventory,
            &clean,
            &clean_dispatch_in_flight,
            &worker_registry,
        )?;

        let mut dirty = true;
        while !app.should_quit {
            dirty |= drain_worker_events(
                &mut app,
                &worker_rx,
                &worker_tx,
                &scan,
                &inventory,
                &clean,
                &clean_dispatch_in_flight,
                &worker_registry,
            )? > 0;
            draw_if_dirty(terminal, &app, &mut dirty)?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    CrosstermEvent::Key(key) => {
                        let effects = app.update(UiEvent::Key(key));
                        dirty = true;
                        dispatch_ui_effects(
                            &mut app,
                            effects,
                            &worker_tx,
                            &scan,
                            &inventory,
                            &clean,
                            &clean_dispatch_in_flight,
                            &worker_registry,
                        )?;
                    }
                    CrosstermEvent::Resize(_, _) => dirty = true,
                    _ => {}
                }
            }
        }

        Ok(())
    })();
    // Close delivery before joining: a worker may be waiting for FIFO space.
    drop(worker_rx);
    join_shutdown_workers(&worker_registry)?;
    outcome
}

fn join_shutdown_workers(worker_registry: &WorkerRegistry) -> Result<()> {
    let workers: Vec<_> = worker_registry
        .lock()
        .map_err(|_| anyhow::anyhow!("worker registry lock poisoned on shutdown"))?
        .drain()
        .map(|(_, worker)| worker)
        .collect();
    for worker in &workers {
        if worker.cancel_on_shutdown {
            worker.cancel.request_cancel();
        }
    }
    for worker in workers {
        let _ = worker.join.join();
    }
    Ok(())
}

fn draw_if_dirty<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &App,
    dirty: &mut bool,
) -> std::result::Result<bool, B::Error> {
    if !*dirty {
        return Ok(false);
    }
    terminal.draw(|frame| render_app(frame, app))?;
    *dirty = false;
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn drain_worker_events<S: ScanService, I: InventoryService, C: CleanService>(
    app: &mut App,
    worker_rx: &WorkerReceiver,
    worker_tx: &WorkerSender,
    scan: &S,
    inventory: &I,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    worker_registry: &WorkerRegistry,
) -> Result<usize> {
    let mut drained = 0;
    while drained < 32 {
        let Ok(event) = worker_rx.try_recv() else {
            break;
        };
        drained += 1;
        join_terminal_worker(&event, worker_registry)?;
        let effects = app.update(UiEvent::Worker(event));
        dispatch_ui_effects(
            app,
            effects,
            worker_tx,
            scan,
            inventory,
            clean,
            clean_dispatch_in_flight,
            worker_registry,
        )?;
    }
    Ok(drained)
}

#[allow(clippy::too_many_arguments)]
fn dispatch_ui_effects<S: ScanService, I: InventoryService, C: CleanService>(
    app: &mut App,
    mut effects: Vec<Effect>,
    worker_tx: &WorkerSender,
    scan: &S,
    inventory: &I,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    worker_registry: &WorkerRegistry,
) -> Result<()> {
    while !effects.is_empty() {
        let immediate = dispatch_effects(
            effects,
            worker_tx,
            scan,
            inventory,
            clean,
            clean_dispatch_in_flight,
            worker_registry,
        )?;
        effects = immediate
            .into_iter()
            .flat_map(|event| app.update(UiEvent::Worker(event)))
            .collect();
    }
    Ok(())
}

fn dispatch_effects<S: ScanService, I: InventoryService, C: CleanService>(
    effects: Vec<Effect>,
    worker_tx: &WorkerSender,
    scan: &S,
    inventory: &I,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    worker_registry: &WorkerRegistry,
) -> Result<Vec<WorkerEvent>> {
    let mut immediate = Vec::new();
    for effect in effects {
        if let Some(event) = dispatch_effect(
            effect,
            worker_tx.clone(),
            scan,
            inventory,
            clean,
            clean_dispatch_in_flight,
            worker_registry,
        )? {
            immediate.push(event);
        }
    }
    Ok(immediate)
}

fn dispatch_effect<S: ScanService, I: InventoryService, C: CleanService>(
    effect: Effect,
    worker_tx: WorkerSender,
    scan: &S,
    inventory: &I,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    worker_registry: &WorkerRegistry,
) -> Result<Option<WorkerEvent>> {
    // Unexpected presentation failure must not cancel an in-flight mutation.
    let cancel_on_shutdown = !matches!(
        &effect,
        Effect::StartClean { .. }
            | Effect::StartSoftwareUninstall { .. }
            | Effect::StartOptimizeRun { .. }
    );
    match effect {
        Effect::StartAnalyze { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned before Analyze start"))?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Analyze dispatch rejected: prior heavy work has not joined"
                        .to_string(),
                }));
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || run_analyze_worker(job_id, worker_tx, worker_cancel));
            worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned after Analyze start"))?
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartScan { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned before scan start"))?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Scan dispatch rejected: prior heavy work has not joined".to_string(),
                }));
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let scan = scan.clone();
            let worker_cancel = Arc::clone(&cancel);
            let join =
                thread::spawn(move || run_scan_worker(job_id, worker_tx, scan, worker_cancel));
            worker_registry
                .lock()
                .map_err(|_| anyhow::anyhow!("worker registry lock poisoned after scan start"))?
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
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
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartSoftwareInventory { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Software inventory start")
                })?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Software inventory rejected: prior work has not joined".to_string(),
                }));
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
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
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
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Software preview rejected: prior work has not joined".to_string(),
                }));
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
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
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
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Software uninstall rejected: prior work has not joined".to_string(),
                }));
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
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartSoftwareAudit { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Software audit start")
                })?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Software audit rejected: prior work has not joined".to_string(),
                }));
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
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartOptimizeList { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Optimize list start")
                })?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Optimize list rejected: prior work has not joined".to_string(),
                }));
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join =
                thread::spawn(move || run_optimize_list_worker(job_id, worker_tx, worker_cancel));
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Optimize list start")
                })?
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartOptimizePreview {
            job_id,
            catalogue_id,
        } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Optimize preview start")
                })?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Optimize preview rejected: prior work has not joined".to_string(),
                }));
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || {
                run_optimize_preview_worker(job_id, catalogue_id, worker_tx, worker_cancel)
            });
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Optimize preview start")
                })?
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartOptimizeRun {
            job_id,
            plan,
            preview_digest,
        } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Optimize run start")
                })?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Optimize run rejected: prior work has not joined".to_string(),
                }));
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || {
                run_optimize_run_worker(job_id, plan, preview_digest, worker_tx, worker_cancel)
            });
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Optimize run start")
                })?
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartOptimizeAudit { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Optimize audit start")
                })?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Optimize audit rejected: prior work has not joined".to_string(),
                }));
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join =
                thread::spawn(move || run_optimize_audit_worker(job_id, worker_tx, worker_cancel));
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Optimize audit start")
                })?
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartStatusSnapshot { job_id } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Status snapshot start")
                })?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Status snapshot rejected: prior work has not joined".to_string(),
                }));
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join =
                thread::spawn(move || run_status_snapshot_worker(job_id, worker_tx, worker_cancel));
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Status snapshot start")
                })?
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
        }
        Effect::StartStatusLive {
            job_id,
            interval_ms,
            process_limit,
        } => {
            if !worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned before Status live start")
                })?
                .is_empty()
            {
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Status live rejected: prior work has not joined".to_string(),
                }));
            }
            let cancel = Arc::new(FlagCancelObserver::new());
            let worker_cancel = Arc::clone(&cancel);
            let join = thread::spawn(move || {
                run_status_live_worker(job_id, interval_ms, process_limit, worker_tx, worker_cancel)
            });
            worker_registry
                .lock()
                .map_err(|_| {
                    anyhow::anyhow!("worker registry lock poisoned after Status live start")
                })?
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
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
                return Ok(Some(WorkerEvent::JobFailed {
                    job_id,
                    message: "Cleanup dispatch rejected: another cleanup job is already running"
                        .to_string(),
                }));
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
                .insert(
                    job_id,
                    WorkerRegistration {
                        cancel,
                        join,
                        cancel_on_shutdown,
                    },
                );
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
            return Ok(Some(dispatch_presentation_language(
                request_id,
                locale,
                persist_language,
            )));
        }
        Effect::Quit => {}
    }

    Ok(None)
}

fn join_terminal_worker(event: &WorkerEvent, worker_registry: &WorkerRegistry) -> Result<()> {
    let job_id = match event {
        WorkerEvent::CleanFinished { job_id, .. }
        | WorkerEvent::AnalyzeFinished { job_id, .. }
        | WorkerEvent::SoftwareInventoryFinished { job_id, .. }
        | WorkerEvent::SoftwarePreviewFinished { job_id, .. }
        | WorkerEvent::SoftwareUninstallFinished { job_id, .. }
        | WorkerEvent::SoftwareAuditFinished { job_id, .. }
        | WorkerEvent::OptimizeListFinished { job_id, .. }
        | WorkerEvent::OptimizePreviewFinished { job_id, .. }
        | WorkerEvent::OptimizeRunFinished { job_id, .. }
        | WorkerEvent::OptimizeAuditFinished { job_id, .. }
        | WorkerEvent::StatusSnapshotFinished { job_id, .. }
        | WorkerEvent::JobFailed { job_id, .. }
        | WorkerEvent::JobCanceled { job_id }
        | WorkerEvent::ScanFinished { job_id, .. }
        | WorkerEvent::InventoryFinished { job_id, .. } => *job_id,
        WorkerEvent::StatusLiveEvent { job_id, event } if event.is_terminal() => *job_id,
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
    persist: F,
) -> WorkerEvent
where
    F: FnOnce(crate::i18n::Locale) -> Result<()>,
{
    match persist(locale) {
        Ok(()) => WorkerEvent::PresentationLanguageSaved { request_id, locale },
        Err(_error) => WorkerEvent::PresentationLanguageSaveFailed { request_id },
    }
}

#[cfg(test)]
mod tests;
