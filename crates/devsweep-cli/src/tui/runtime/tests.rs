use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use super::*;
use crate::{
    execution::{ExecutionProgress, ExecutionReport, ExecutionRequest, ExecutionTargetStatus},
    inventory::InventoryReport,
    model::{CleanupPlan, ScanHealth},
    plan::validate_scanned_plan,
    scan::{ScanOptions, ScanPhase, ScanProgress},
    tui::{display::compact_target_id, test_support::representative_plan},
};
use devsweep_core::services::{
    CleanService, ExecutorCleanService, ScanServiceOutcome, ScanServiceRunOutcome,
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

#[derive(Clone)]
struct CanceledScanService;

impl ScanService for CanceledScanService {
    fn full_scan_with_cancel(
        &self,
        _options: &ScanOptions,
        _progress: &mut dyn FnMut(ScanProgress),
        _cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceOutcome> {
        unreachable!("the worker uses the explicit terminal API")
    }

    fn full_scan_run_with_cancel(
        &self,
        _options: &ScanOptions,
        _progress: &mut dyn FnMut(ScanProgress),
        _cancel: Option<&Arc<FlagCancelObserver>>,
    ) -> Result<ScanServiceRunOutcome> {
        Ok(ScanServiceRunOutcome::Canceled)
    }
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
        outcomes: Vec::new(),
        notes: Vec::new(),
        estimated_recoverable: Default::default(),
        confirmation_digest: crate::execution::ConfirmationDigest::new("test-digest"),
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

    let request = ExecutionRequest {
        execute: false,
        audit_log: None,
        selected: plan.default_selected_ids(),
        expected_digest: None,
        cancel: None,
    };
    let report = ExecutorCleanService
        .run_plan(&plan, &digest, request.clone(), &mut |_| {})
        .expect("matching digest validates");
    assert!(report.dry_run);
    assert!(
        ExecutorCleanService
            .run_plan(&plan, "different-digest", request, &mut |_| {})
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
fn scan_worker_does_not_promote_an_explicitly_canceled_scan() {
    let (worker_tx, worker_rx) = mpsc::channel();

    run_scan_worker(
        11,
        worker_tx,
        CanceledScanService,
        Arc::new(FlagCancelObserver::new()),
    );

    assert_eq!(
        worker_rx.try_iter().collect::<Vec<_>>(),
        vec![
            WorkerEvent::ScanStarted { job_id: 11 },
            WorkerEvent::JobCanceled { job_id: 11 },
        ]
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
