use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use devsweep_core::{
    model::{ScanPreviewSnapshot, ScanReport},
    process::FlagCancelObserver,
    scan::{
        ScanOptions, ScanPhase, ScanProgress, ScanReportRunOutcome, Sweeper, scan_preview_from_plan,
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
        Sweeper::default().full_scan_report_run_with_cancel(options, progress, Some(cancel))
    }
}

#[derive(Clone)]
struct RunningScan {
    scan_id: String,
    cancel: Arc<FlagCancelObserver>,
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
        *running = Some(RunningScan {
            scan_id,
            cancel: Arc::clone(&cancel),
        });
        Ok(cancel)
    }

    pub(crate) fn cancel(&self, scan_id: &str) -> Result<(), CommandError> {
        if let Some(running) = self.running.lock().map_err(CommandError::io)?.as_ref()
            && running.scan_id == scan_id
        {
            running.cancel.request_cancel();
        }
        Ok(())
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
    pending_preview: Option<DesktopScanProgress>,
    first_error: Option<anyhow::Error>,
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
        }
    }

    fn forward(&mut self, progress: ScanProgress) {
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
        let event = DesktopScanProgress {
            scan_id: self.scan_id.clone(),
            sequence: 0,
            phase: progress.phase,
            message: progress.message,
            preview,
        };

        if event.preview.is_none() {
            self.flush_pending();
            self.send_now(event, false);
            return;
        }

        let now = (self.now)();
        let may_send = self
            .last_preview_at
            .is_none_or(|last| now.saturating_sub(last) >= PREVIEW_EMISSION_INTERVAL);
        if may_send {
            self.send_now(event, true);
        } else {
            self.pending_preview = Some(event);
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
            self.send_now(event, true);
        }
    }

    fn send_now(&mut self, mut event: DesktopScanProgress, preview: bool) {
        if self.first_error.is_some() {
            return;
        }
        self.sequence = self.sequence.saturating_add(1);
        event.sequence = self.sequence;
        if let Err(error) = (self.send)(event) {
            self.first_error = Some(error);
            return;
        }
        if preview {
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

        for (index, message) in ["first", "second", "latest"].into_iter().enumerate() {
            forwarder.forward(ScanProgress {
                phase: ScanPhase::Projects,
                message: message.to_string(),
                partial: Some(CleanupPlan {
                    version: CLEANUP_PLAN_VERSION,
                    targets: vec![preview_target(&format!("target-{index}"))],
                }),
            });
        }
        forwarder.finish().expect("terminal flush succeeds");

        let events = events.borrow();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "first");
        assert_eq!(events[1].message, "latest");
        assert_eq!(events[1].sequence, 2);
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
