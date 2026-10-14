use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use devsweep_core::{
    model::{ScanPreviewSnapshot, ScanReport, TargetId},
    process::{CancelObserver, FlagCancelObserver},
    scan::{
        ScanOptions, ScanPhase, ScanProgress, ScanProgressPolicy, ScanReportRunOutcome, Sweeper,
        scan_preview_from_plan, validate_scan_preview_plan,
    },
};
use serde::Serialize;

use crate::error::CommandError;

pub(crate) const PREVIEW_EMISSION_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopScanProgress {
    pub scan_id: String,
    pub sequence: u64,
    pub phase: ScanPhase,
    pub message: String,
    pub preview: Option<ScanPreviewSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DesktopScanResult {
    Completed { scan_id: String, report: ScanReport },
    Canceled { scan_id: String },
}

pub(crate) trait ScanRunner: Send + Sync + 'static {
    fn run(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: &Arc<FlagCancelObserver>,
    ) -> Result<ScanReportRunOutcome>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CoreScanRunner;

impl ScanRunner for CoreScanRunner {
    fn run(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: &Arc<FlagCancelObserver>,
    ) -> Result<ScanReportRunOutcome> {
        Sweeper::default()
            .with_progress_policy(ScanProgressPolicy::Interactive)
            .full_scan_report_run_with_cancel(options, progress, Some(cancel))
    }
}

pub(crate) struct CoreRescanRunner(pub TargetId);

impl ScanRunner for CoreRescanRunner {
    fn run(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: &Arc<FlagCancelObserver>,
    ) -> Result<ScanReportRunOutcome> {
        let result = Sweeper::default()
            .with_progress_policy(ScanProgressPolicy::Interactive)
            .full_scan_report_rescanning_target_with_cancel(
                options,
                &self.0,
                progress,
                Some(cancel),
            );
        if cancel.is_cancel_requested() {
            Ok(ScanReportRunOutcome::Canceled)
        } else {
            Ok(ScanReportRunOutcome::Completed(result?))
        }
    }
}

#[derive(Clone)]
struct RunningScan {
    scan_id: String,
    cancel: Arc<FlagCancelObserver>,
    delivery: Arc<ProgressDelivery>,
}

#[derive(Default)]
struct DeliverySequence {
    sent: u64,
    acknowledged: u64,
}

/// One progress packet may be retained by Tauri until the validated consumer
/// acknowledges it. The worker, never the event loop, waits for consumption.
pub(crate) struct ProgressDelivery {
    sequence: Mutex<DeliverySequence>,
    changed: Condvar,
    cancel: Arc<FlagCancelObserver>,
}

impl ProgressDelivery {
    pub(crate) fn send(
        &self,
        progress: DesktopScanProgress,
        emit: impl FnOnce(DesktopScanProgress) -> Result<()>,
    ) -> Result<()> {
        {
            let mut sequence = self
                .sequence
                .lock()
                .map_err(|_| anyhow!("scan delivery lock poisoned"))?;
            if self.cancel.is_cancel_requested() {
                return Ok(());
            }
            sequence.sent = progress.sequence;
        }
        emit(progress)?;
        let mut sequence = self
            .sequence
            .lock()
            .map_err(|_| anyhow!("scan delivery lock poisoned"))?;
        while sequence.acknowledged < sequence.sent && !self.cancel.is_cancel_requested() {
            sequence = self
                .changed
                .wait(sequence)
                .map_err(|_| anyhow!("scan delivery lock poisoned"))?;
        }
        Ok(())
    }

    fn acknowledge(&self, received: u64) -> Result<(), CommandError> {
        let mut sequence = self.sequence.lock().map_err(CommandError::io)?;
        if received == 0 || received > sequence.sent {
            return Err(CommandError::scan_failed(anyhow!(
                "invalid scan progress acknowledgment"
            )));
        }
        if received <= sequence.acknowledged {
            return Ok(());
        }
        if received != sequence.sent {
            return Err(CommandError::scan_failed(anyhow!(
                "unexpected scan progress acknowledgment"
            )));
        }
        sequence.acknowledged = received;
        self.changed.notify_all();
        Ok(())
    }

    fn cancel(&self) {
        // Hold the same mutex as wait so cancellation cannot lose a wakeup.
        let _sequence = self.sequence.lock();
        self.cancel.request_cancel();
        self.changed.notify_all();
    }
}

#[derive(Clone, Default)]
pub(crate) struct ScanCoordinator {
    running: Arc<Mutex<Option<RunningScan>>>,
}

impl ScanCoordinator {
    pub(crate) fn begin(&self, scan_id: String) -> Result<Arc<FlagCancelObserver>, CommandError> {
        if scan_id.trim().is_empty() {
            return Err(CommandError::scan_failed(anyhow!(
                "scan id must not be empty"
            )));
        }
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running.is_some() {
            return Err(CommandError::ScanAlreadyRunning);
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        let delivery = Arc::new(ProgressDelivery {
            sequence: Mutex::default(),
            changed: Condvar::new(),
            cancel: Arc::clone(&cancel),
        });
        *running = Some(RunningScan {
            scan_id,
            cancel: Arc::clone(&cancel),
            delivery,
        });
        Ok(cancel)
    }

    pub(crate) fn cancel(&self, scan_id: &str) -> Result<(), CommandError> {
        if let Some(running) = self.running.lock().map_err(CommandError::io)?.as_ref()
            && running.scan_id == scan_id
        {
            running.delivery.cancel();
        }
        Ok(())
    }

    pub(crate) fn progress_delivery(
        &self,
        scan_id: &str,
        cancel: &Arc<FlagCancelObserver>,
    ) -> Result<Arc<ProgressDelivery>, CommandError> {
        self.running
            .lock()
            .map_err(CommandError::io)?
            .as_ref()
            .filter(|running| running.scan_id == scan_id && Arc::ptr_eq(&running.cancel, cancel))
            .map(|running| Arc::clone(&running.delivery))
            .ok_or_else(|| CommandError::scan_failed(anyhow!("scan delivery is no longer active")))
    }

    pub(crate) fn acknowledge(&self, scan_id: &str, sequence: u64) -> Result<(), CommandError> {
        if let Some(running) = self.running.lock().map_err(CommandError::io)?.as_ref()
            && running.scan_id == scan_id
        {
            running.delivery.acknowledge(sequence)?;
        }
        Ok(())
    }

    /// Renderer destruction cannot leave a worker waiting for an absent ack.
    pub(crate) fn cancel_active(&self) {
        if let Ok(running) = self.running.lock()
            && let Some(running) = running.as_ref()
        {
            running.delivery.cancel();
        }
    }

    pub(crate) fn finish(
        &self,
        scan_id: &str,
        completed: &Arc<FlagCancelObserver>,
    ) -> Result<(), CommandError> {
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running.as_ref().is_some_and(|current| {
            current.scan_id == scan_id && Arc::ptr_eq(&current.cancel, completed)
        }) {
            *running = None;
        }
        Ok(())
    }
}

struct ProgressForwarder<S, N> {
    scan_id: String,
    sequence: u64,
    send: S,
    now: N,
    last_preview_at: Option<Duration>,
    pending_preview: Option<ScanProgress>,
    first_error: Option<anyhow::Error>,
    #[cfg(test)]
    projected_previews: usize,
}

impl<S, N> ProgressForwarder<S, N>
where
    S: FnMut(DesktopScanProgress) -> Result<()>,
    N: FnMut() -> Duration,
{
    fn new(scan_id: String, send: S, now: N) -> Self {
        Self {
            scan_id,
            sequence: 0,
            send,
            now,
            last_preview_at: None,
            pending_preview: None,
            first_error: None,
            #[cfg(test)]
            projected_previews: 0,
        }
    }

    fn forward(&mut self, progress: ScanProgress) {
        if self.first_error.is_some() {
            return;
        }

        if let Err(error) = progress
            .partial
            .as_ref()
            .map(validate_scan_preview_plan)
            .transpose()
        {
            self.first_error = Some(error);
            return;
        }

        if progress.partial.is_none() {
            self.flush_pending();
            self.send_now(progress);
            return;
        }

        let now = (self.now)();
        let may_send = self
            .last_preview_at
            .is_none_or(|last| now.saturating_sub(last) >= PREVIEW_EMISSION_INTERVAL);
        if may_send {
            self.pending_preview = None;
            self.send_now(progress);
        } else {
            self.pending_preview = Some(progress);
        }
    }

    fn finish(mut self) -> Result<()> {
        self.flush_pending();
        if let Some(error) = self.first_error {
            return Err(error);
        }
        Ok(())
    }

    fn flush_pending(&mut self) {
        if let Some(event) = self.pending_preview.take() {
            self.send_now(event);
        }
    }

    fn send_now(&mut self, progress: ScanProgress) {
        if self.first_error.is_some() {
            return;
        }
        let preview = match progress
            .partial
            .as_ref()
            .map(scan_preview_from_plan)
            .transpose()
        {
            Ok(preview) => preview,
            Err(error) => {
                self.first_error = Some(error);
                return;
            }
        };
        let is_preview = preview.is_some();
        #[cfg(test)]
        if is_preview {
            self.projected_previews += 1;
        }
        self.sequence = self.sequence.saturating_add(1);
        let event = DesktopScanProgress {
            scan_id: self.scan_id.clone(),
            sequence: self.sequence,
            phase: progress.phase,
            message: progress.message,
            preview,
        };
        if let Err(error) = (self.send)(event) {
            self.first_error = Some(error);
            return;
        }
        if is_preview {
            self.last_preview_at = Some((self.now)());
        }
    }
}

pub(crate) fn run_scan_job<R, E>(
    runner: &R,
    options: &ScanOptions,
    scan_id: String,
    cancel: &Arc<FlagCancelObserver>,
    emit: E,
) -> Result<DesktopScanResult>
where
    R: ScanRunner,
    E: FnMut(DesktopScanProgress) -> Result<()>,
{
    let started = Instant::now();
    let mut forwarder = ProgressForwarder::new(scan_id.clone(), emit, || started.elapsed());
    let outcome = runner.run(options, &mut |progress| forwarder.forward(progress), cancel);
    forwarder.finish()?;
    let outcome = outcome?;
    Ok(match outcome {
        ScanReportRunOutcome::Completed(report) => DesktopScanResult::Completed { scan_id, report },
        ScanReportRunOutcome::Canceled => DesktopScanResult::Canceled { scan_id },
    })
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        path::PathBuf,
        rc::Rc,
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    use devsweep_core::{
        model::{
            CLEANUP_PLAN_VERSION, CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence,
            RiskLevel, ScanHealth, Scope, TargetId, TargetKind, UntrustedPlan,
        },
        process::CancelObserver,
        scan::ScanPhase,
    };

    use super::*;

    #[test]
    fn reviewed_rescan_uses_discovered_identity_and_maps_cancellation_to_terminal_canceled() {
        let fixture = tempfile::tempdir().unwrap();
        std::fs::write(fixture.path().join("package.json"), b"{}").unwrap();
        let modules = fixture.path().join("node_modules");
        std::fs::create_dir(&modules).unwrap();
        std::fs::write(modules.join("fixture.bin"), b"fixture").unwrap();
        let options = ScanOptions {
            include_projects: true,
            include_global: false,
            roots: vec![fixture.path().to_path_buf()],
        };
        let cancel = Arc::new(FlagCancelObserver::new());
        let report = match CoreScanRunner.run(&options, &mut |_| {}, &cancel).unwrap() {
            ScanReportRunOutcome::Completed(report) => report,
            ScanReportRunOutcome::Canceled => panic!("fixture scan unexpectedly canceled"),
        };
        let target = report
            .plan
            .targets
            .first()
            .expect("discovered node dependency target");
        assert!(matches!(
            CoreRescanRunner(target.id.clone())
                .run(&options, &mut |_| {}, &cancel)
                .unwrap(),
            ScanReportRunOutcome::Completed(_)
        ));
        assert!(
            CoreRescanRunner(TargetId::new("caller-supplied-path"))
                .run(&options, &mut |_| {}, &cancel)
                .is_err()
        );
        cancel.request_cancel();
        assert!(matches!(
            CoreRescanRunner(target.id.clone())
                .run(&options, &mut |_| {}, &cancel)
                .unwrap(),
            ScanReportRunOutcome::Canceled
        ));
        assert_eq!(
            std::fs::read(modules.join("fixture.bin")).unwrap(),
            b"fixture"
        );
    }

    struct SlowRunner;

    struct FixedRunner;

    impl ScanRunner for FixedRunner {
        fn run(
            &self,
            _options: &ScanOptions,
            progress: &mut dyn FnMut(ScanProgress),
            _cancel: &Arc<FlagCancelObserver>,
        ) -> Result<ScanReportRunOutcome> {
            for message in ["first", "second"] {
                progress(ScanProgress {
                    phase: ScanPhase::Projects,
                    message: message.to_string(),
                    partial: None,
                });
            }
            Ok(ScanReportRunOutcome::Completed(ScanReport::new(
                UntrustedPlan::empty(),
                ScanHealth::complete(),
            )))
        }
    }

    impl ScanRunner for SlowRunner {
        fn run(
            &self,
            _options: &ScanOptions,
            progress: &mut dyn FnMut(ScanProgress),
            cancel: &Arc<FlagCancelObserver>,
        ) -> Result<ScanReportRunOutcome> {
            while !cancel.is_cancel_requested() {
                progress(ScanProgress {
                    phase: ScanPhase::Projects,
                    message: "working".to_string(),
                    partial: None,
                });
                thread::sleep(Duration::from_millis(10));
            }
            Ok(ScanReportRunOutcome::Canceled)
        }
    }

    fn options() -> ScanOptions {
        ScanOptions {
            include_projects: true,
            include_global: false,
            roots: Vec::new(),
        }
    }

    fn delivery_event(sequence: u64) -> DesktopScanProgress {
        DesktopScanProgress {
            scan_id: "delivery".to_string(),
            sequence,
            phase: ScanPhase::Projects,
            message: "progress".to_string(),
            preview: None,
        }
    }

    #[test]
    fn progress_delivery_waits_for_each_exact_ack_before_sending_or_settling() {
        let coordinator = ScanCoordinator::default();
        let cancel = coordinator.begin("delivery".to_string()).unwrap();
        let delivery = coordinator.progress_delivery("delivery", &cancel).unwrap();
        let (sent, received) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        let worker = thread::spawn(move || {
            for sequence in 1..=3 {
                delivery
                    .send(delivery_event(sequence), |event| {
                        sent.send(event.sequence)?;
                        Ok(())
                    })
                    .unwrap();
            }
            done.send(()).unwrap();
        });
        assert_eq!(received.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
        assert!(coordinator.acknowledge("delivery", 0).is_err());
        assert!(coordinator.acknowledge("delivery", 2).is_err());
        assert!(received.recv_timeout(Duration::from_millis(30)).is_err());
        coordinator.acknowledge("delivery", 1).unwrap();
        assert_eq!(received.recv_timeout(Duration::from_secs(1)).unwrap(), 2);
        coordinator.acknowledge("delivery", 1).unwrap();
        coordinator.acknowledge("stale", 2).unwrap();
        assert!(received.recv_timeout(Duration::from_millis(30)).is_err());
        coordinator.acknowledge("delivery", 2).unwrap();
        assert_eq!(received.recv_timeout(Duration::from_secs(1)).unwrap(), 3);
        assert!(completed.recv_timeout(Duration::from_millis(30)).is_err());
        coordinator.acknowledge("delivery", 3).unwrap();
        completed.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.join().unwrap();
        coordinator.finish("delivery", &cancel).unwrap();
        coordinator.acknowledge("delivery", 3).unwrap();
    }

    #[test]
    fn cancel_or_renderer_loss_wakes_unacknowledged_delivery_without_later_send() {
        for renderer_lost in [false, true] {
            let coordinator = ScanCoordinator::default();
            let cancel = coordinator.begin("delivery".to_string()).unwrap();
            let delivery = coordinator.progress_delivery("delivery", &cancel).unwrap();
            let (sent, received) = mpsc::channel();
            let (done, completed) = mpsc::channel();
            let worker = thread::spawn(move || {
                for sequence in 1..=2 {
                    delivery
                        .send(delivery_event(sequence), |event| {
                            sent.send(event.sequence)?;
                            Ok(())
                        })
                        .unwrap();
                }
                done.send(()).unwrap();
            });
            assert_eq!(received.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
            if renderer_lost {
                coordinator.cancel_active();
            } else {
                coordinator.cancel("delivery").unwrap();
            }
            completed.recv_timeout(Duration::from_secs(1)).unwrap();
            worker.join().unwrap();
            assert!(received.try_recv().is_err());
            assert!(cancel.is_cancel_requested());
            coordinator.finish("delivery", &cancel).unwrap();
            coordinator.begin("next".to_string()).unwrap();
        }
    }

    #[test]
    fn delivery_send_failure_does_not_wait_for_an_ack() {
        let coordinator = ScanCoordinator::default();
        let cancel = coordinator.begin("delivery".to_string()).unwrap();
        let delivery = coordinator.progress_delivery("delivery", &cancel).unwrap();
        assert!(
            delivery
                .send(delivery_event(1), |_| Err(anyhow!("closed channel")))
                .is_err()
        );
        coordinator.finish("delivery", &cancel).unwrap();
    }

    #[test]
    fn concurrent_scan_is_rejected_immediately() {
        let coordinator = ScanCoordinator::default();
        let running = coordinator
            .begin("first".to_string())
            .expect("first scan reserves slot");

        assert_eq!(
            coordinator
                .begin("second".to_string())
                .expect_err("second scan is rejected"),
            CommandError::ScanAlreadyRunning
        );
        coordinator
            .finish("first", &running)
            .expect("scan slot releases");
        coordinator
            .begin("second".to_string())
            .expect("next scan can start");
    }

    #[test]
    fn worker_owned_coordinator_releases_shared_slot() {
        let coordinator = ScanCoordinator::default();
        let worker_state = coordinator.clone();
        let running = coordinator
            .begin("shared".to_string())
            .expect("scan reserves slot");

        worker_state
            .finish("shared", &running)
            .expect("worker releases slot");

        coordinator
            .begin("next".to_string())
            .expect("shared slot is available");
    }

    #[test]
    fn cancellation_is_idempotent_and_stops_fixture_within_five_seconds() {
        let coordinator = Arc::new(ScanCoordinator::default());
        let cancel = coordinator
            .begin("slow".to_string())
            .expect("scan reserves slot");
        let worker_cancel = Arc::clone(&cancel);
        let (events_tx, events_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            run_scan_job(
                &SlowRunner,
                &options(),
                "slow".to_string(),
                &worker_cancel,
                |event| {
                    events_tx.send(event).expect("event receiver stays open");
                    Ok(())
                },
            )
        });

        events_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("fixture forwards progress");
        let started = Instant::now();
        coordinator
            .cancel("stale")
            .expect("stale cancellation is harmless");
        coordinator
            .cancel("slow")
            .expect("first cancellation succeeds");
        coordinator
            .cancel("slow")
            .expect("repeated cancellation succeeds");
        let outcome = worker
            .join()
            .expect("scan thread joins")
            .expect("scan returns");

        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            outcome,
            DesktopScanResult::Canceled {
                scan_id: "slow".to_string()
            }
        );
    }

    #[test]
    fn scan_job_forwards_every_progress_event() {
        let cancel = Arc::new(FlagCancelObserver::new());
        let mut events = Vec::new();

        run_scan_job(
            &FixedRunner,
            &options(),
            "fixed".to_string(),
            &cancel,
            |event| {
                events.push(event);
                Ok(())
            },
        )
        .expect("fixture scan returns");

        assert_eq!(
            events
                .iter()
                .map(|event| event.message.as_str())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
    }

    #[test]
    fn ipc_progress_projects_display_facts_without_cleanup_authority() {
        let progress = ScanProgress {
            phase: ScanPhase::Projects,
            message: "partial".to_string(),
            partial: Some(CleanupPlan {
                version: CLEANUP_PLAN_VERSION,
                targets: vec![CleanTarget {
                    id: TargetId::new("fixture.command"),
                    scope: Scope::Global,
                    ecosystem: Ecosystem::Node,
                    kind: TargetKind::PackageCache,
                    path: None,
                    estimated_bytes: 1,
                    size_complete: true,
                    sizing_warnings: Vec::new(),
                    last_modified: None,
                    risk: RiskLevel::Medium,
                    reversible: false,
                    selected_by_default: false,
                    evidence: vec![Evidence::UserConfigured],
                    action: CleanAction::Command {
                        program: "authority-fixture.exe".to_string(),
                        args: vec!["--dangerous-fixture".to_string()],
                        cwd: Some(PathBuf::from("C:/authority-fixture")),
                        irreversible: true,
                    },
                }],
            }),
        };
        let internal_json = serde_json::to_string(&progress).expect("internal progress serializes");

        let events = Rc::new(RefCell::new(Vec::new()));
        let emitted = Rc::clone(&events);
        let mut forwarder = ProgressForwarder::new(
            "safe".to_string(),
            move |event| {
                emitted.borrow_mut().push(event);
                Ok(())
            },
            || Duration::ZERO,
        );
        forwarder.forward(progress);
        forwarder.finish().expect("preview projection succeeds");
        let events = events.borrow();
        let projected = events.first().expect("preview event emitted");
        let ipc_json = serde_json::to_string(projected).expect("IPC progress serializes");

        assert!(internal_json.contains("authority-fixture.exe"));
        assert!(projected.preview.is_some());
        assert!(!ipc_json.contains("authority-fixture.exe"));
        assert!(!ipc_json.contains("--dangerous-fixture"));
        assert!(!ipc_json.contains("selected_by_default"));
        assert!(!ipc_json.contains("intent"));
        assert!(!ipc_json.contains("action"));
        assert!(!ipc_json.contains("cwd"));
        assert_eq!(projected.message, "partial");
        assert_eq!(projected.scan_id, "safe");
        assert_eq!(projected.sequence, 1);
    }

    #[test]
    fn preview_delivery_coalesces_bursts_and_flushes_the_latest_snapshot() {
        let now = Rc::new(Cell::new(Duration::ZERO));
        let clock = Rc::clone(&now);
        let events = Rc::new(RefCell::new(Vec::new()));
        let emitted = Rc::clone(&events);
        let mut forwarder = ProgressForwarder::new(
            "burst".to_string(),
            move |event| {
                emitted.borrow_mut().push(event);
                Ok(())
            },
            move || clock.get(),
        );

        for index in 0..100 {
            let message = if index == 0 { "first" } else { "latest" };
            forwarder.forward(ScanProgress {
                phase: ScanPhase::Projects,
                message: message.to_string(),
                partial: Some(CleanupPlan {
                    version: CLEANUP_PLAN_VERSION,
                    targets: vec![preview_target(&format!("target-{index}"))],
                }),
            });
        }
        assert_eq!(forwarder.projected_previews, 1);
        assert_eq!(
            forwarder
                .pending_preview
                .as_ref()
                .unwrap()
                .partial
                .as_ref()
                .unwrap()
                .targets[0]
                .id,
            TargetId::new("target-99"),
        );
        forwarder.flush_pending();
        assert_eq!(forwarder.projected_previews, 2);
        assert!(forwarder.pending_preview.is_none());
        forwarder.finish().expect("terminal flush succeeds");

        let events = events.borrow();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "first");
        assert_eq!(events[1].message, "latest");
        assert_eq!(events[1].sequence, 2);
    }

    #[test]
    fn a_newer_immediate_preview_supersedes_pending_before_phase_or_terminal_flush() {
        for flush_phase in [false, true] {
            let now = Cell::new(Duration::ZERO);
            let events = RefCell::new(Vec::new());
            let mut forwarder = ProgressForwarder::new(
                "ordered".to_string(),
                |event| {
                    events.borrow_mut().push(event);
                    Ok(())
                },
                || now.get(),
            );
            for (index, message) in ["first", "superseded", "newest"].into_iter().enumerate() {
                if index == 2 {
                    now.set(PREVIEW_EMISSION_INTERVAL);
                }
                forwarder.forward(ScanProgress {
                    phase: ScanPhase::Projects,
                    message: message.to_string(),
                    partial: Some(CleanupPlan {
                        version: CLEANUP_PLAN_VERSION,
                        targets: vec![preview_target(message)],
                    }),
                });
            }
            assert!(forwarder.pending_preview.is_none());
            if flush_phase {
                forwarder.forward(ScanProgress {
                    phase: ScanPhase::Projects,
                    message: "phase".to_string(),
                    partial: None,
                });
            }
            forwarder.finish().unwrap();
            let events = events.borrow();
            let messages: Vec<_> = events.iter().map(|event| event.message.as_str()).collect();
            assert_eq!(
                messages,
                if flush_phase {
                    vec!["first", "newest", "phase"]
                } else {
                    vec!["first", "newest"]
                }
            );
            assert!(
                events
                    .iter()
                    .enumerate()
                    .all(|(index, event)| event.sequence == index as u64 + 1)
            );
        }
    }

    #[test]
    fn an_invalid_superseded_preview_still_fails_the_scan() {
        let events = RefCell::new(Vec::new());
        let mut forwarder = ProgressForwarder::new(
            "invalid-pending".to_string(),
            |event| {
                events.borrow_mut().push(event);
                Ok(())
            },
            || Duration::ZERO,
        );
        for targets in [
            vec![preview_target("first")],
            vec![preview_target("duplicate"), preview_target("duplicate")],
            vec![preview_target("replacement")],
        ] {
            forwarder.forward(ScanProgress {
                phase: ScanPhase::Projects,
                message: "preview".to_string(),
                partial: Some(CleanupPlan {
                    version: CLEANUP_PLAN_VERSION,
                    targets,
                }),
            });
        }
        let error = forwarder
            .finish()
            .expect_err("invalid input cannot be coalesced away");
        assert!(
            error
                .to_string()
                .contains("duplicate scan preview target id")
        );
        assert_eq!(events.borrow().len(), 1);
    }

    #[test]
    fn scan_failure_flushes_the_latest_pending_preview() {
        struct FailingPreviewRunner;

        impl ScanRunner for FailingPreviewRunner {
            fn run(
                &self,
                _options: &ScanOptions,
                progress: &mut dyn FnMut(ScanProgress),
                _cancel: &Arc<FlagCancelObserver>,
            ) -> Result<ScanReportRunOutcome> {
                for (index, message) in ["first", "latest"].into_iter().enumerate() {
                    progress(ScanProgress {
                        phase: ScanPhase::Projects,
                        message: message.to_string(),
                        partial: Some(CleanupPlan {
                            version: CLEANUP_PLAN_VERSION,
                            targets: vec![preview_target(&format!("target-{index}"))],
                        }),
                    });
                }
                Err(anyhow!("fixture scan failure"))
            }
        }

        let cancel = Arc::new(FlagCancelObserver::new());
        let mut events = Vec::new();
        let error = run_scan_job(
            &FailingPreviewRunner,
            &options(),
            "failing".to_string(),
            &cancel,
            |event| {
                events.push(event);
                Ok(())
            },
        )
        .expect_err("fixture scan fails");

        assert!(error.to_string().contains("fixture scan failure"));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "first");
        assert_eq!(events[1].message, "latest");
    }

    #[test]
    fn preview_projection_failure_stops_delivery_and_fails_closed() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let emitted = Rc::clone(&events);
        let mut forwarder = ProgressForwarder::new(
            "invalid".to_string(),
            move |event| {
                emitted.borrow_mut().push(event);
                Ok(())
            },
            || Duration::ZERO,
        );
        let duplicate = preview_target("duplicate");
        forwarder.forward(ScanProgress {
            phase: ScanPhase::Projects,
            message: "invalid preview".to_string(),
            partial: Some(CleanupPlan {
                version: CLEANUP_PLAN_VERSION,
                targets: vec![duplicate.clone(), duplicate],
            }),
        });

        let error = forwarder.finish().expect_err("duplicate ids fail closed");
        assert!(
            error
                .to_string()
                .contains("duplicate scan preview target id")
        );
        assert!(events.borrow().is_empty());
    }

    fn preview_target(id: &str) -> CleanTarget {
        CleanTarget {
            id: TargetId::new(id),
            scope: Scope::Global,
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::ToolCache,
            path: Some(PathBuf::from(format!("C:/fixture/{id}"))),
            estimated_bytes: 1,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default: true,
            evidence: vec![Evidence::UserConfigured],
            action: CleanAction::MoveToTrash {
                path: PathBuf::from(format!("C:/fixture/{id}")),
            },
        }
    }
}
