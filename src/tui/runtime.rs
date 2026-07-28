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

use anyhow::{Context, Result, bail};
use crossterm::event::{self, Event as CrosstermEvent};

use crate::{
    executor::{ExecutionProgress, ExecutionReport, ExecutionRequest, Executor},
    model::{CleanupPlan, TargetId},
    plan_validation::{ValidatedPlan, validate_scanned_plan},
    process_runner::{CancelObserver, FlagCancelObserver},
    sweep::{ScanOptions, ScanProgress, Sweeper},
};

type CancelRegistry = Arc<Mutex<HashMap<JobId, Arc<FlagCancelObserver>>>>;

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
    ) -> Result<CleanupPlan>;
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

#[derive(Clone)]
pub(super) struct SweepScanService;

impl ScanService for SweepScanService {
    fn full_scan_with_cancel(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<CleanupPlan> {
        Sweeper::default().full_scan_with_cancel(options, progress, cancel)
    }
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

pub(super) fn run_event_loop<S: ScanService, C: CleanService>(
    terminal: &mut Tui,
    scan: S,
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
                &clean,
                &clean_dispatch_in_flight,
                &cancel_registry,
            )?;
        }
    }

    Ok(())
}

fn drain_worker_events<S: ScanService, C: CleanService>(
    app: &mut App,
    worker_rx: &Receiver<WorkerEvent>,
    worker_tx: &Sender<WorkerEvent>,
    scan: &S,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    cancel_registry: &CancelRegistry,
) -> Result<()> {
    while let Ok(event) = worker_rx.try_recv() {
        if let WorkerEvent::CleanFinished { job_id, .. }
        | WorkerEvent::JobFailed { job_id, .. }
        | WorkerEvent::JobCanceled { job_id }
        | WorkerEvent::ScanFinished { job_id, .. } = &event
        {
            if let Ok(mut guard) = cancel_registry.lock() {
                guard.remove(job_id);
            }
        }
        let effects = app.update(UiEvent::Worker(event));
        dispatch_effects(
            effects,
            worker_tx,
            scan,
            clean,
            clean_dispatch_in_flight,
            cancel_registry,
        )?;
    }
    Ok(())
}

fn dispatch_effects<S: ScanService, C: CleanService>(
    effects: Vec<Effect>,
    worker_tx: &Sender<WorkerEvent>,
    scan: &S,
    clean: &C,
    clean_dispatch_in_flight: &Arc<AtomicBool>,
    cancel_registry: &CancelRegistry,
) -> Result<()> {
    for effect in effects {
        dispatch_effect(
            effect,
            worker_tx.clone(),
            scan,
            clean,
            clean_dispatch_in_flight,
            cancel_registry,
        )?;
    }
    Ok(())
}

fn dispatch_effect<S: ScanService, C: CleanService>(
    effect: Effect,
    worker_tx: Sender<WorkerEvent>,
    scan: &S,
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
        Ok(plan) if cancel.is_cancel_requested() => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
            let _ = worker_tx.send(WorkerEvent::ScanFinished { job_id, plan });
        }
        Ok(plan) => {
            let _ = worker_tx.send(WorkerEvent::ScanFinished { job_id, plan });
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
        outcome: Result<CleanupPlan, String>,
    }

    impl ScanService for FakeScanService {
        fn full_scan_with_cancel(
            &self,
            _options: &ScanOptions,
            progress: &mut dyn FnMut(ScanProgress),
            _cancel: Option<&Arc<FlagCancelObserver>>,
        ) -> Result<CleanupPlan> {
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
                outcome: Ok(full.clone()),
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
                    plan: full
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
            outcome: Ok(CleanupPlan::empty()),
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
            outcome: Ok(CleanupPlan::empty()),
        };
        let clean = FakeCleanService {
            progress: Vec::new(),
            outcome: Ok(successful_report()),
            requests: Arc::new(Mutex::new(Vec::new())),
        };
        let clean_dispatch_in_flight = Arc::new(AtomicBool::new(false));

        dispatch_effect(
            Effect::CancelJob { job_id: 7 },
            worker_tx,
            &scan,
            &clean,
            &clean_dispatch_in_flight,
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .expect("cancel effect dispatches");

        assert!(worker_rx.try_recv().is_err());
    }
}
