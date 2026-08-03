use std::sync::{Arc, Mutex};

use anyhow::Result;
use devsweep_core::{
    model::ScanReport,
    process::FlagCancelObserver,
    scan::{ScanOptions, ScanProgress, Sweeper},
};

use crate::error::CommandError;

pub(crate) const SCAN_PROGRESS_EVENT: &str = "scan://progress";

pub(crate) fn progress_for_ipc(mut progress: ScanProgress) -> ScanProgress {
    // The core partial contains trusted CleanAction details. The webview only
    // needs phase/message progress and must never receive execution authority.
    progress.partial = None;
    progress
}

pub(crate) trait ScanRunner: Send + Sync + 'static {
    fn run(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: &Arc<FlagCancelObserver>,
    ) -> Result<ScanReport>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CoreScanRunner;

impl ScanRunner for CoreScanRunner {
    fn run(
        &self,
        options: &ScanOptions,
        progress: &mut dyn FnMut(ScanProgress),
        cancel: &Arc<FlagCancelObserver>,
    ) -> Result<ScanReport> {
        Sweeper::default().full_scan_report_with_cancel(options, progress, Some(cancel))
    }
}

#[derive(Clone, Default)]
pub(crate) struct ScanCoordinator {
    running: Arc<Mutex<Option<Arc<FlagCancelObserver>>>>,
}

impl ScanCoordinator {
    pub(crate) fn begin(&self) -> Result<Arc<FlagCancelObserver>, CommandError> {
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running.is_some() {
            return Err(CommandError::ScanAlreadyRunning);
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        *running = Some(Arc::clone(&cancel));
        Ok(cancel)
    }

    pub(crate) fn cancel(&self) -> Result<(), CommandError> {
        if let Some(cancel) = self.running.lock().map_err(CommandError::io)?.as_ref() {
            cancel.request_cancel();
        }
        Ok(())
    }

    pub(crate) fn finish(&self, completed: &Arc<FlagCancelObserver>) -> Result<(), CommandError> {
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, completed))
        {
            *running = None;
        }
        Ok(())
    }
}

pub(crate) fn run_scan_job<R, E>(
    runner: &R,
    options: &ScanOptions,
    cancel: &Arc<FlagCancelObserver>,
    mut emit: E,
) -> Result<ScanReport>
where
    R: ScanRunner,
    E: FnMut(ScanProgress),
{
    runner.run(options, &mut emit, cancel)
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
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
        ) -> Result<ScanReport> {
            for message in ["first", "second"] {
                progress(ScanProgress {
                    phase: ScanPhase::Projects,
                    message: message.to_string(),
                    partial: None,
                });
            }
            Ok(ScanReport::new(
                UntrustedPlan::empty(),
                ScanHealth::complete(),
            ))
        }
    }

    impl ScanRunner for SlowRunner {
        fn run(
            &self,
            _options: &ScanOptions,
            progress: &mut dyn FnMut(ScanProgress),
            cancel: &Arc<FlagCancelObserver>,
        ) -> Result<ScanReport> {
            while !cancel.is_cancel_requested() {
                progress(ScanProgress {
                    phase: ScanPhase::Projects,
                    message: "working".to_string(),
                    partial: None,
                });
                thread::sleep(Duration::from_millis(10));
            }
            Ok(ScanReport::new(
                UntrustedPlan::empty(),
                ScanHealth::complete(),
            ))
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
        let running = coordinator.begin().expect("first scan reserves slot");

        assert_eq!(
            coordinator.begin().expect_err("second scan is rejected"),
            CommandError::ScanAlreadyRunning
        );
        coordinator.finish(&running).expect("scan slot releases");
        coordinator.begin().expect("next scan can start");
    }

    #[test]
    fn worker_owned_coordinator_releases_shared_slot() {
        let coordinator = ScanCoordinator::default();
        let worker_state = coordinator.clone();
        let running = coordinator.begin().expect("scan reserves slot");

        worker_state.finish(&running).expect("worker releases slot");

        coordinator.begin().expect("shared slot is available");
    }

    #[test]
    fn cancellation_is_idempotent_and_stops_fixture_within_five_seconds() {
        let coordinator = Arc::new(ScanCoordinator::default());
        let cancel = coordinator.begin().expect("scan reserves slot");
        let worker_cancel = Arc::clone(&cancel);
        let (events_tx, events_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            run_scan_job(&SlowRunner, &options(), &worker_cancel, |event| {
                events_tx.send(event).expect("event receiver stays open");
            })
        });

        events_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("fixture forwards progress");
        let started = Instant::now();
        coordinator.cancel().expect("first cancellation succeeds");
        coordinator
            .cancel()
            .expect("repeated cancellation succeeds");
        worker
            .join()
            .expect("scan thread joins")
            .expect("scan returns");

        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn scan_job_forwards_every_progress_event() {
        let cancel = Arc::new(FlagCancelObserver::new());
        let mut events = Vec::new();

        run_scan_job(&FixedRunner, &options(), &cancel, |event| {
            events.push(event)
        })
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
    fn ipc_progress_redacts_trusted_partial_plan() {
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

        let projected = progress_for_ipc(progress);
        let ipc_json = serde_json::to_string(&projected).expect("IPC progress serializes");

        assert!(internal_json.contains("authority-fixture.exe"));
        assert!(projected.partial.is_none());
        assert!(!ipc_json.contains("authority-fixture.exe"));
        assert!(!ipc_json.contains("--dangerous-fixture"));
        assert_eq!(projected.message, "partial");
    }
}
