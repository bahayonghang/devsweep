use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use crossterm::event::{self, Event as CrosstermEvent};

use crate::{
    executor::{ExecutionProgress, ExecutionReport, ExecutionRequest, Executor},
    inventory::{InventoryReport, inventory_root_with_cancel},
    model::{CleanupPlan, ScanHealth, ScanReport, TargetId},
    plan::{ValidatedPlan, validate_plan, validate_scanned_plan},
    process_runner::{CancelObserver, FlagCancelObserver},
    sweep::{ScanOptions, ScanProgress, Sweeper},
};

type CancelRegistry = Arc<Mutex<HashMap<JobId, Arc<FlagCancelObserver>>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScanServiceOutcome {
    pub(super) plan: CleanupPlan,
    pub(super) health: ScanHealth,
}

use super::{
    app::{App, Effect, JobId, UiEvent, WorkerEvent},
    render::{compact_target_id, render_app},
    terminal::Tui,
};

pub(super) trait ScanService: Send + Clone + 'static {
    fn full_scan_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceOutcome>;
}

pub(super) trait CleanService: Send + Clone + 'static {
    fn run_plan(
        &self,
        plan: &CleanupPlan,
        expected_digest: &str,
        request: ExecutionRequest,
        on_progress: &mut dyn FnMut(ExecutionProgress),
    ) -> Result<ExecutionReport>;
}

pub(super) trait InventoryService: Send + Clone + 'static {
    fn inventory_root_with_cancel(
        &self,
        root: &Path,
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<InventoryReport>;
}

#[derive(Clone)]
pub(super) struct SweepScanService;

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
pub(super) struct LocalInventoryService;

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
pub(super) struct ExecutorCleanService;

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

fn validate_confirmed_plan(plan: &CleanupPlan, expected_digest: &str) -> Result<ValidatedPlan> {
    let validated = validate_scanned_plan(plan)?;
    if validated.digest() != expected_digest {
        bail!("confirmed cleanup plan digest does not match the validated plan")
    }
    Ok(validated)
}

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

fn run_scan_worker<S: ScanService>(
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

fn run_inventory_worker<I: InventoryService>(
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
fn run_clean_worker<C: CleanService>(
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

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use super::*;
    use crate::{
        executor::ExecutionTargetStatus, sweep::ScanPhase, tui::test_support::representative_plan,
    };

    #[derive(Clone)]
    struct FakeScanService {
        partial: Option<CleanupPlan>,
        outcome: Result<ScanServiceOutcome, String>,
    }

    impl ScanService for FakeScanService {
        fn full_scan_with_cancel(
            &self,
            _options: &ScanOptions,
            progress: &mut dyn FnMut(ScanProgress),
            _cancel: Option<&Arc<FlagCancelObserver>>,
        ) -> Result<ScanServiceOutcome> {
            progress(ScanProgress {
                phase: ScanPhase::Projects,
                message: "Fake project scan".to_string(),
                partial: self.partial.clone(),
            });
            self.outcome
                .clone()
                .map_err(|message| anyhow::anyhow!(message))
        }
    }

    #[derive(Clone)]
    struct FakeInventoryService {
        outcome: Result<InventoryReport, String>,
    }

    impl InventoryService for FakeInventoryService {
        fn inventory_root_with_cancel(
            &self,
            _root: &Path,
            _cancel: Option<&Arc<FlagCancelObserver>>,
        ) -> Result<InventoryReport> {
            self.outcome
                .clone()
                .map_err(|message| anyhow::anyhow!(message))
        }
    }

    #[derive(Clone)]
    struct FakeCleanService {
        progress: Vec<ExecutionProgress>,
        outcome: Result<ExecutionReport, String>,
        requests: Arc<Mutex<Vec<ExecutionRequest>>>,
    }

    impl CleanService for FakeCleanService {
        fn run_plan(
            &self,
            _plan: &CleanupPlan,
            _expected_digest: &str,
            request: ExecutionRequest,
            on_progress: &mut dyn FnMut(ExecutionProgress),
        ) -> Result<ExecutionReport> {
            self.requests.lock().expect("requests lock").push(request);
            for progress in &self.progress {
                on_progress(progress.clone());
            }
            self.outcome
                .clone()
                .map_err(|message| anyhow::anyhow!(message))
        }
    }

    #[derive(Clone)]
    struct BlockingCleanService {
        invocations: Arc<AtomicUsize>,
        started_tx: Sender<()>,
        release_rx: Arc<Mutex<Receiver<()>>>,
    }

    impl CleanService for BlockingCleanService {
        fn run_plan(
            &self,
            _plan: &CleanupPlan,
            _expected_digest: &str,
            _request: ExecutionRequest,
            _on_progress: &mut dyn FnMut(ExecutionProgress),
        ) -> Result<ExecutionReport> {
            self.invocations.fetch_add(1, Ordering::SeqCst);
            self.started_tx.send(()).expect("worker start signal");
            self.release_rx
                .lock()
                .expect("release lock")
                .recv()
                .expect("worker release signal");
            Ok(successful_report())
        }
    }

    fn successful_report() -> ExecutionReport {
        ExecutionReport {
            dry_run: false,
            selected: 1,
            attempted: 1,
            succeeded: 1,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
            audit_log: None,
        }
    }

    fn complete_scan_outcome(plan: CleanupPlan) -> ScanServiceOutcome {
        ScanServiceOutcome {
            plan,
            health: ScanHealth::complete(),
        }
    }

    fn empty_inventory_report() -> InventoryReport {
        InventoryReport {
            version: crate::inventory::INVENTORY_REPORT_VERSION,
            root: std::env::temp_dir().join("devsweep-inventory-fixture"),
            observations: Vec::new(),
            health: ScanHealth::complete(),
            orphan_pnpm_store: None,
        }
    }

    #[test]
    fn confirmed_digest_must_match_the_revalidated_snapshot() {
        let mut plan = representative_plan();
        plan.targets.truncate(1);
        let digest = validate_scanned_plan(&plan)
            .expect("representative project target validates")
            .digest()
            .to_string();

        assert_eq!(
            validate_confirmed_plan(&plan, &digest)
                .expect("matching digest validates")
                .digest(),
            digest
        );
        assert!(
            validate_confirmed_plan(&plan, "different-digest")
                .expect_err("mismatched digest rejects")
                .to_string()
                .contains("does not match")
        );
    }

    #[test]
    fn scan_worker_emits_started_progress_finished() {
        let (worker_tx, worker_rx) = mpsc::channel();
        let partial = representative_plan();
        let full = representative_plan();

        run_scan_worker(
            7,
            worker_tx,
            FakeScanService {
                partial: Some(partial.clone()),
                outcome: Ok(complete_scan_outcome(full.clone())),
            },
            Arc::new(FlagCancelObserver::new()),
        );

        let events: Vec<WorkerEvent> = worker_rx.try_iter().collect();
        assert_eq!(
            events,
            vec![
                WorkerEvent::ScanStarted { job_id: 7 },
                WorkerEvent::ScanProgress {
                    job_id: 7,
                    phase: ScanPhase::Projects,
                    message: "Fake project scan".to_string(),
                    plan: Some(partial),
                },
                WorkerEvent::ScanFinished {
                    job_id: 7,
                    plan: full,
                    health: ScanHealth::complete(),
                },
            ]
        );
    }

    #[test]
    fn scan_worker_reports_failure_as_job_failed() {
        let (worker_tx, worker_rx) = mpsc::channel();

        run_scan_worker(
            9,
            worker_tx,
            FakeScanService {
                partial: None,
                outcome: Err("scan exploded".to_string()),
            },
            Arc::new(FlagCancelObserver::new()),
        );

        let events: Vec<WorkerEvent> = worker_rx.try_iter().collect();
        assert_eq!(
            events.first(),
            Some(&WorkerEvent::ScanStarted { job_id: 9 })
        );
        assert_eq!(
            events.last(),
            Some(&WorkerEvent::JobFailed {
                job_id: 9,
                message: "scan exploded".to_string(),
            })
        );
    }

    #[test]
    fn inventory_worker_emits_read_only_report_events() {
        let (worker_tx, worker_rx) = mpsc::channel();
        let report = empty_inventory_report();

        run_inventory_worker(
            10,
            worker_tx,
            FakeInventoryService {
                outcome: Ok(report.clone()),
            },
            Arc::new(FlagCancelObserver::new()),
        );

        let events: Vec<WorkerEvent> = worker_rx.try_iter().collect();
        assert_eq!(
            events,
            vec![
                WorkerEvent::InventoryStarted { job_id: 10 },
                WorkerEvent::InventoryFinished {
                    job_id: 10,
                    report: Box::new(report),
                },
            ]
        );
    }

    #[test]
    fn clean_worker_translates_execution_progress() {
        let (worker_tx, worker_rx) = mpsc::channel();
        let plan = representative_plan();
        let target_id = plan.targets[0].id.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));

        run_clean_worker(
            3,
            plan,
            vec![target_id.clone()],
            "fixture-digest".to_string(),
            worker_tx,
            FakeCleanService {
                progress: vec![ExecutionProgress {
                    completed: 1,
                    total: 2,
                    target_id: target_id.clone(),
                    status: ExecutionTargetStatus::Succeeded,
                    message: "moved to trash".to_string(),
                }],
                outcome: Ok(successful_report()),
                requests: requests.clone(),
            },
            Arc::new(AtomicBool::new(false)),
            Arc::new(FlagCancelObserver::new()),
        );

        let events: Vec<WorkerEvent> = worker_rx.try_iter().collect();
        assert_eq!(
            events[0],
            WorkerEvent::JobProgress {
                job_id: 3,
                message: "Executing selected cleanup plan".to_string(),
            }
        );
        assert_eq!(
            events[1],
            WorkerEvent::CleanProgress {
                job_id: 3,
                target_id: target_id.clone(),
                status: ExecutionTargetStatus::Succeeded,
                message: format!("{}: moved to trash", compact_target_id(&target_id)),
                detail: "moved to trash".to_string(),
                completed: 1,
                total: 2,
            }
        );

        let recorded = requests.lock().expect("requests lock");
        assert_eq!(recorded.len(), 1);
        assert!(recorded[0].execute);
        assert!(recorded[0].audit_log.is_none());
        assert_eq!(recorded[0].selected, vec![target_id]);
    }

    #[test]
    fn clean_worker_reports_finish_and_failure() {
        let (worker_tx, worker_rx) = mpsc::channel();
        run_clean_worker(
            4,
            representative_plan(),
            representative_plan().default_selected_ids(),
            "fixture-digest".to_string(),
            worker_tx,
            FakeCleanService {
                progress: Vec::new(),
                outcome: Ok(successful_report()),
                requests: Arc::new(Mutex::new(Vec::new())),
            },
            Arc::new(AtomicBool::new(false)),
            Arc::new(FlagCancelObserver::new()),
        );
        let events: Vec<WorkerEvent> = worker_rx.try_iter().collect();
        assert_eq!(
            events.last(),
            Some(&WorkerEvent::CleanFinished {
                job_id: 4,
                report: successful_report(),
            })
        );

        let (worker_tx, worker_rx) = mpsc::channel();
        run_clean_worker(
            5,
            representative_plan(),
            representative_plan().default_selected_ids(),
            "fixture-digest".to_string(),
            worker_tx,
            FakeCleanService {
                progress: Vec::new(),
                outcome: Err("clean exploded".to_string()),
                requests: Arc::new(Mutex::new(Vec::new())),
            },
            Arc::new(AtomicBool::new(false)),
            Arc::new(FlagCancelObserver::new()),
        );
        let events: Vec<WorkerEvent> = worker_rx.try_iter().collect();
        assert_eq!(
            events.last(),
            Some(&WorkerEvent::JobFailed {
                job_id: 5,
                message: "clean exploded".to_string(),
            })
        );
    }

    #[test]
    fn duplicate_clean_dispatch_starts_only_one_worker() {
        let (worker_tx, worker_rx) = mpsc::channel();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let invocations = Arc::new(AtomicUsize::new(0));
        let clean_dispatch_in_flight = Arc::new(AtomicBool::new(false));
        let clean = BlockingCleanService {
            invocations: invocations.clone(),
            started_tx,
            release_rx: Arc::new(Mutex::new(release_rx)),
        };
        let scan = FakeScanService {
            partial: None,
            outcome: Ok(complete_scan_outcome(CleanupPlan::empty())),
        };
        let inventory = FakeInventoryService {
            outcome: Ok(empty_inventory_report()),
        };
        let plan = representative_plan();
        let selected = plan.default_selected_ids();

        dispatch_effects(
            vec![
                Effect::StartClean {
                    job_id: 1,
                    plan: plan.clone(),
                    selected: selected.clone(),
                    plan_digest: "fixture-digest".to_string(),
                },
                Effect::StartClean {
                    job_id: 2,
                    plan,
                    selected,
                    plan_digest: "fixture-digest".to_string(),
                },
            ],
            &worker_tx,
            &scan,
            &inventory,
            &clean,
            &clean_dispatch_in_flight,
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .expect("effects dispatch");

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("first clean worker starts");
        assert_eq!(invocations.load(Ordering::SeqCst), 1);
        assert!(clean_dispatch_in_flight.load(Ordering::Acquire));
        let early_events: Vec<WorkerEvent> = worker_rx.try_iter().collect();
        assert!(early_events.iter().any(|event| {
            matches!(
                event,
                WorkerEvent::JobFailed { job_id: 2, message }
                    if message.contains("another cleanup job is already running")
            )
        }));

        release_tx.send(()).expect("release first worker");
        let mut saw_terminal_event = false;
        for _ in 0..2 {
            let event = worker_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("worker emits terminal event");
            if matches!(event, WorkerEvent::CleanFinished { job_id: 1, .. }) {
                saw_terminal_event = true;
                break;
            }
        }
        assert!(saw_terminal_event);
        for _ in 0..100 {
            if !clean_dispatch_in_flight.load(Ordering::Acquire) {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(!clean_dispatch_in_flight.load(Ordering::Acquire));
        assert_eq!(invocations.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cancel_effect_does_not_fabricate_a_cancellation_event() {
        let (worker_tx, worker_rx) = mpsc::channel();
        let scan = FakeScanService {
            partial: None,
            outcome: Ok(complete_scan_outcome(CleanupPlan::empty())),
        };
        let clean = FakeCleanService {
            progress: Vec::new(),
            outcome: Ok(successful_report()),
            requests: Arc::new(Mutex::new(Vec::new())),
        };
        let inventory = FakeInventoryService {
            outcome: Ok(empty_inventory_report()),
        };
        let clean_dispatch_in_flight = Arc::new(AtomicBool::new(false));

        dispatch_effect(
            Effect::CancelJob { job_id: 7 },
            worker_tx,
            &scan,
            &inventory,
            &clean,
            &clean_dispatch_in_flight,
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .expect("cancel effect dispatches");

        assert!(worker_rx.try_recv().is_err());
    }
}
