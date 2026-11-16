//! Session authorization tests use file hives and injected executors only.
use super::*;
use crate::host_review::{ActionResult, Plan, Prepared};
use crate::workbench::SoftwareSegment;
use devsweep_core::software::{
    SoftwareStartupLocation, SoftwareUpdatePlanV1, SoftwareUpdateRowV1, StartupFixtureClass,
    StartupFixtureEntryV1, StartupFixtureHiveV1,
};
use std::sync::atomic::AtomicU32;

#[derive(Clone)]
struct ProcessFixture(Arc<std::sync::Mutex<ProcessFixtureState>>);

struct ProcessFixtureState {
    live: Option<devsweep_core::status::ObservedProcess>,
    calls: usize,
    fail: bool,
    records: Vec<devsweep_core::status::StatusActionAuditRecordV1>,
    fail_audit_at: usize,
    audit_calls: usize,
}

impl ProcessFixture {
    fn new() -> Self {
        Self(Arc::new(std::sync::Mutex::new(ProcessFixtureState {
            live: Some(devsweep_core::status::ObservedProcess {
                pid: 42,
                creation_time_filetime: 1000,
                name: "fixture.exe".into(),
            }),
            calls: 0,
            fail: false,
            records: Vec::new(),
            fail_audit_at: 0,
            audit_calls: 0,
        })))
    }

    fn session(&self) -> Session {
        let mut session = Session::unbacked();
        session.host_executor = Some(Arc::new(host_review::CoreExecutor::with_process(
            Box::new(self.clone()),
            Box::new(self.clone()),
            Box::new(self.clone()),
        )));
        session.show_mode(Mode::Status);
        session.apply_status_snapshot(process_snapshot("fixture.exe"));
        let row = process_row(session.status_snapshot().expect("snapshot"));
        session.status_toggle_process(&row);
        session
    }
}

impl devsweep_core::status::ProcessProbe for ProcessFixture {
    fn lookup(
        &self,
        _: u32,
    ) -> Result<
        Option<devsweep_core::status::ObservedProcess>,
        devsweep_core::status::ProcessActionError,
    > {
        Ok(self.0.lock().expect("fixture").live.clone())
    }
}

impl devsweep_core::status::ProcessKiller for ProcessFixture {
    fn terminate(
        &mut self,
        pid: u32,
        creation: u64,
    ) -> Result<(), devsweep_core::status::ProcessActionError> {
        use devsweep_core::status::ProcessActionError;
        let mut state = self.0.lock().expect("fixture");
        let live = state.live.as_ref().ok_or(ProcessActionError::NotFound)?;
        if (pid, creation) != (live.pid, live.creation_time_filetime) {
            return Err(ProcessActionError::IdentityMismatch);
        }
        state.calls += 1;
        if state.fail {
            return Err(ProcessActionError::PermissionDenied);
        }
        state.live = None;
        Ok(())
    }
}

impl devsweep_core::status::ProcessActionAudit for ProcessFixture {
    fn append(
        &mut self,
        record: devsweep_core::status::StatusActionAuditRecordV1,
    ) -> Result<(), devsweep_core::status::ProcessActionError> {
        let mut state = self.0.lock().expect("fixture");
        state.audit_calls += 1;
        if state.audit_calls == state.fail_audit_at {
            return Err(devsweep_core::status::ProcessActionError::AuditFailed {
                after_dispatch: false,
            });
        }
        state.records.push(record);
        Ok(())
    }
}

fn process_row(snapshot: &StatusSnapshotV1) -> devsweep_core::status::ProcessV1 {
    match &snapshot.processes {
        devsweep_core::status::AvailabilityV1::Available { value, .. }
        | devsweep_core::status::AvailabilityV1::Partial { value, .. } => value.items[0].clone(),
        _ => panic!("fixture processes"),
    }
}

fn process_snapshot(name: &str) -> StatusSnapshotV1 {
    let mut snapshot: StatusSnapshotV1 = serde_json::from_str(include_str!(
        "../../crates/devsweep-cli/tests/fixtures/status/snapshot.json"
    ))
    .expect("fixture");
    let value = match &mut snapshot.processes {
        devsweep_core::status::AvailabilityV1::Available { value, .. }
        | devsweep_core::status::AvailabilityV1::Partial { value, .. } => value,
        _ => panic!("fixture process group"),
    };
    value.items.truncate(1);
    value.items[0].pid = 42;
    value.items[0].name = name.into();
    snapshot
}

#[test]
fn process_review_requires_second_confirmation_and_discloses_identity_and_data_loss_in_both_locales()
 {
    for locale in [Locale::En, Locale::ZhCn] {
        let fixture = ProcessFixture::new();
        let mut session = fixture.session();
        session.set_locale(locale);
        assert!(!session.host_confirm(true));
        assert!(session.status_preview_process_termination());
        assert!(!session.host_confirm(true));
        apply_one(&mut session);
        let lines = session.host_review_lines().join("\n");
        assert!(
            lines.contains("fixture.exe") && lines.contains("PID 42") && lines.contains("1000")
        );
        assert!(lines.contains(crate::copy::pick(locale, "unsaved work", "未保存")));
        assert!(!session.host_confirm(false));
        assert_eq!(fixture.0.lock().expect("fixture").calls, 0);
        assert!(session.host_confirm(true));
        apply_one(&mut session);
        assert_eq!(session.host_review.results[0].code, "terminated");
        let state = fixture.0.lock().expect("fixture");
        assert_eq!(state.calls, 1);
        assert_eq!(state.records.len(), 2);
    }
}

#[test]
fn process_global_enter_and_repeated_review_require_the_explicit_confirm_control() {
    let fixture = ProcessFixture::new();
    let mut session = fixture.session();
    assert!(session.status_preview_process_termination());
    apply_one(&mut session);
    // Root Enter and the search input's propagated Enter share this route.
    for _ in 0..2 {
        session.apply_key(NavKey::Enter);
        assert_eq!(session.host_review.phase, HostPhase::Ready);
        assert!(session.take_jobs().is_empty());
        assert_eq!(fixture.0.lock().expect("fixture").calls, 0);
    }
    // Mouse/UIA/keyboard clicks on Review use preparation, even when Ready.
    assert!(session.status_preview_process_termination());
    apply_one(&mut session);
    assert_eq!(fixture.0.lock().expect("fixture").calls, 0);
    session.status_set_query("fixture");
    assert!(session.status_preview_process_termination());
    apply_one(&mut session);
    session.apply_key(NavKey::Enter);
    assert!(session.take_jobs().is_empty());
    assert_eq!(fixture.0.lock().expect("fixture").calls, 0);
    assert!(session.host_confirm(true));
    apply_one(&mut session);
    assert_eq!(fixture.0.lock().expect("fixture").calls, 1);

    let status = crate::source_lf(include_str!("view/page_status.rs"));
    let detail = status
        .split_once("fn status_process_detail(")
        .expect("detail")
        .1;
    assert!(detail.contains("status-process-review") && detail.contains("on_a11y_action"));
    assert_eq!(
        detail
            .matches("status_preview_process_termination()")
            .count(),
        2
    );
    assert!(!detail.contains("host_confirm("));
    let confirm = crate::source_lf(include_str!("view/host_review.rs"));
    assert!(confirm.contains("Confirm ending process") && confirm.contains("确认结束进程"));
    assert_eq!(confirm.matches("host_confirm(true)").count(), 2);
}

#[test]
fn process_protection_and_unbacked_sessions_refuse_before_any_worker() {
    for (name, _) in devsweep_core::status::protected_process_names() {
        let fixture = ProcessFixture::new();
        let mut session = fixture.session();
        let snapshot = process_snapshot(name);
        let row = process_row(&snapshot);
        session.apply_status_snapshot(snapshot);
        session.status_toggle_process(&row);
        assert_eq!(session.status_process_action_error(), Some("protected"));
        assert!(!session.status_preview_process_termination());
        assert!(session.take_jobs().is_empty());
        assert_eq!(fixture.0.lock().expect("fixture").calls, 0);
    }
    let mut session = Session::unbacked();
    let snapshot = process_snapshot("fixture.exe");
    let row = process_row(&snapshot);
    session.apply_status_snapshot(snapshot);
    session.status_toggle_process(&row);
    assert_eq!(
        session.status_process_action_error(),
        Some("unbacked_session")
    );
    assert!(!session.status_preview_process_termination());
    assert!(session.take_jobs().is_empty());
    let fixture = ProcessFixture::new();
    let mut session = fixture.session();
    assert!(session.install_clean_review_fixture("en:previewed"));
    assert!(!session.status_preview_process_termination());
    assert!(!session.host_confirm(true));
    assert_eq!(fixture.0.lock().expect("fixture").calls, 0);
}

#[test]
fn process_identity_drift_failure_and_audit_uncertainty_never_become_success() {
    for case in 0..6 {
        let fixture = ProcessFixture::new();
        let mut session = fixture.session();
        assert!(session.status_preview_process_termination());
        apply_one(&mut session);
        {
            let mut state = fixture.0.lock().expect("fixture");
            match case {
                0 => state.live = None,
                1 => state.live.as_mut().expect("live").creation_time_filetime += 1,
                2 => state.live.as_mut().expect("live").name = "replacement.exe".into(),
                3 => state.fail = true,
                4 => state.fail_audit_at = 1,
                5 => state.fail_audit_at = 2,
                _ => unreachable!(),
            }
        }
        assert!(session.host_confirm(true));
        apply_one(&mut session);
        let expected = match case {
            0..=2 => "invalid_identity",
            3 => "identity_unavailable",
            4 => "audit_unavailable",
            _ => "unknown_after_dispatch",
        };
        assert_eq!(session.host_review.results[0].code, expected);
        assert_eq!(
            fixture.0.lock().expect("fixture").calls,
            usize::from(case == 3 || case == 5)
        );
    }
}

#[test]
fn process_telemetry_is_observation_while_selection_sort_search_and_disappearance_invalidate_review()
 {
    for change in 0..5 {
        let fixture = ProcessFixture::new();
        let mut session = fixture.session();
        assert!(session.status_preview_process_termination());
        apply_one(&mut session);
        session.status_receive_live(process_snapshot("fixture.exe"));
        assert_eq!(
            session.host_review.phase,
            HostPhase::Ready,
            "same identity telemetry keeps the reviewed plan"
        );
        match change {
            0 => session.status_clear_process(),
            1 => session.status_cycle_sort(),
            2 => session.status_set_query("other"),
            3 => session.apply_status_snapshot(process_snapshot("replacement.exe")),
            4 => session.show_mode(Mode::Software),
            _ => unreachable!(),
        }
        assert!(!session.host_confirm(true));
        assert_eq!(fixture.0.lock().expect("fixture").calls, 0);
    }
}

#[test]
fn process_late_preview_or_worker_result_never_transfers_authority_or_unlocks_early() {
    let fixture = ProcessFixture::new();
    let mut session = fixture.session();
    assert!(session.status_preview_process_termination());
    let preview = session.take_jobs().pop().expect("preview");
    session.status_clear_process();
    assert!(session.host_action_busy());
    assert!(!session.apply_job(preview.run()));
    assert!(!session.host_confirm(true));
    assert!(!session.host_action_busy());
    let row = process_row(session.status_snapshot().expect("snapshot"));
    session.status_toggle_process(&row);
    assert!(session.status_preview_process_termination());
    apply_one(&mut session);
    assert!(session.host_confirm(true));
    let terminal = session.take_jobs().pop().expect("execute").run();
    assert_eq!(fixture.0.lock().expect("fixture").calls, 1);
    session.status_clear_process();
    assert!(session.host_action_busy());
    assert!(!session.host_preview(vec![update("other")]));
    assert!(!session.apply_job(terminal));
    assert!(!session.host_action_busy());
    assert!(session.host_review.results.is_empty());
}

#[derive(Default)]
struct FakeExecutor {
    calls: AtomicU32,
}

fn update(id: &str) -> HostIntent {
    HostIntent::Update {
        row: SoftwareUpdateRowV1 {
            id: id.into(),
            name: id.into(),
            installed_version: "1".into(),
            available_version: "2".into(),
            source: "winget".into(),
            name_truncated: false,
            matched_software_ids: Vec::new(),
        },
    }
}

impl ActionExecutor for FakeExecutor {
    fn prepare(&self, intent: HostIntent) -> Result<Prepared, String> {
        Ok(Prepared {
            intent,
            plan: Plan::Update(SoftwareUpdatePlanV1 {
                version: 1,
                package_id: "fixture".into(),
                source: "winget".into(),
                installed_version: "1".into(),
                available_version: "2".into(),
                provider_fingerprint: format!("sha256:{}", "a".repeat(64)),
            }),
            digest: format!("sha256:{}", "b".repeat(64)),
        })
    }
    fn execute(&self, item: &Prepared, confirmed: bool) -> Result<ActionResult, String> {
        assert!(confirmed);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ActionResult {
            intent: item.intent.clone(),
            code: "updated".into(),
            reverse: None,
        })
    }
}

fn apply_one(session: &mut Session) {
    let jobs = session.take_jobs();
    assert_eq!(jobs.len(), 1);
    for job in jobs {
        assert!(session.apply_job(job.run()));
    }
}

#[test]
fn unbacked_and_clean_fixture_sessions_never_schedule_host_actions() {
    let mut session = Session::unbacked();
    assert!(!session.keep_awake_preview_toggle());
    assert!(!session.host_confirm(true));
    assert!(session.take_jobs().is_empty());
    let fake = Arc::new(FakeExecutor::default());
    session.host_executor = Some(fake.clone());
    assert!(session.install_clean_review_fixture("en:previewed"));
    session.show_mode(Mode::Software);
    session.software_open_segment(SoftwareSegment::Updates);
    session.activate_mode_row(Mode::Software, "updates");
    assert!(!session.host_preview(vec![update("a")]));
    assert!(!session.keep_awake_preview_toggle());
    assert!(!session.queue_awake_poll());
    assert!(session.take_jobs().is_empty());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn changed_selection_canceled_preview_and_old_execution_delivery_do_not_replace_current_state() {
    let fake = Arc::new(FakeExecutor::default());
    let mut session = Session::unbacked();
    session.host_executor = Some(fake.clone());
    session.show_mode(Mode::Software);
    session.select_mode_row(Mode::Software, "a");
    assert!(session.host_preview(vec![update("a")]));
    let preview = session.take_jobs().pop().expect("preview");
    session.select_mode_row(Mode::Software, "b");
    assert!(!session.apply_job(preview.run()));
    assert!(session.host_review.prepared.is_empty());
    assert!(!session.host_confirm(true));
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    assert!(session.host_preview(vec![update("b")]));
    apply_one(&mut session);
    assert!(!session.host_confirm(false));
    assert!(session.host_confirm(true));
    assert!(!session.host_preview(vec![update("c")]));
    let execute = session.take_jobs().pop().expect("execution");
    session.select_mode_row(Mode::Software, "c");
    let result = execute.run();
    assert!(!session.apply_job(result));
    assert!(!session.host_action_busy());
    assert_eq!(session.shell.selection(Mode::Software), Some("c"));
    assert_eq!(
        fake.calls.load(Ordering::SeqCst),
        0,
        "cancel before queued dispatch"
    );
}

#[test]
fn cancellation_after_dispatch_keeps_the_lock_until_the_obsolete_worker_terminal() {
    let fake = Arc::new(FakeExecutor::default());
    let mut session = Session::unbacked();
    session.host_executor = Some(fake.clone());
    assert!(session.host_preview(vec![update("a")]));
    apply_one(&mut session);
    assert!(session.host_confirm(true));
    let job = session.take_jobs().pop().expect("execution");
    let terminal = job.run();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    session.host_cancel();
    assert!(session.host_action_busy());
    assert!(!session.host_preview(vec![update("b")]));
    assert!(!session.apply_job(terminal));
    assert!(!session.host_action_busy());
    assert!(session.host_review.results.is_empty());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert!(session.host_preview(vec![update("b")]));
}

#[test]
fn startup_hive_keyboard_flow_and_restore_require_new_preview_and_confirmation() {
    let root = std::env::temp_dir().join(format!("devsweep-host-review-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("fixture directory");
    let path = root.join("startup.json");
    let hive = StartupFixtureHiveV1 {
        version: 1,
        entries: vec![StartupFixtureEntryV1 {
            name: "Fixture".into(),
            location: SoftwareStartupLocation::CurrentUserRun,
            class: StartupFixtureClass::Ordinary,
            run_value: b"fixture".to_vec(),
            approval: None,
        }],
    };
    std::fs::write(&path, serde_json::to_vec(&hive).expect("hive")).expect("write hive");
    let before = std::fs::read(&path).expect("before");
    let mut session = Session::unbacked();
    session.show_mode(Mode::Software);
    session.software_review.open_hive(&path, Locale::En);
    session.sync_software_rows();
    let id = session.software_review.filtered_ids()[0].clone();
    session.activate_mode_row(Mode::Software, id.clone());
    assert_eq!(session.host_review.phase, HostPhase::Preparing);
    assert_eq!(std::fs::read(&path).expect("read"), before);
    apply_one(&mut session);
    let digest = session.host_review.prepared[0].digest.clone();
    assert!(!session.host_confirm(false));
    assert_eq!(std::fs::read(&path).expect("read"), before);
    session.apply_key(NavKey::Enter);
    apply_one(&mut session);
    assert_eq!(session.host_review.results[0].code, "succeeded");
    assert_eq!(
        session.software_review.item(&id).expect("row").version,
        "disabled"
    );
    assert!(session.host_preview_restore());
    assert!(
        !session.host_confirm(true),
        "restore is not ready before live preview"
    );
    apply_one(&mut session);
    assert_ne!(session.host_review.prepared[0].digest, digest);
    assert!(session.host_confirm(true));
    apply_one(&mut session);
    assert_eq!(
        session.software_review.item(&id).expect("row").version,
        "enabled"
    );
    std::fs::remove_file(&path).expect("remove owned fixture");
    std::fs::remove_dir(&root).expect("remove owned directory");
}

#[test]
fn refresh_and_scan_root_changes_clear_confirmation_without_dispatch() {
    let fake = Arc::new(FakeExecutor::default());
    let mut session = Session::unbacked();
    session.host_executor = Some(fake.clone());
    session.host_preview(vec![update("a")]);
    apply_one(&mut session);
    session.software_refresh();
    assert!(session.host_review.prepared.is_empty());
    assert!(!session.host_confirm(true));
    session.host_preview(vec![update("a")]);
    apply_one(&mut session);
    session.set_scan_roots(Vec::new());
    assert!(session.host_review.prepared.is_empty());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn session_drop_explicitly_joins_awake_owner_even_when_a_poll_holds_an_executor_arc() {
    use devsweep_core::status::{BackendCall, FakeClock, KeepAwakeService, RecordingBackend};
    let (backend, log) = RecordingBackend::pair();
    let executor = Arc::new(host_review::CoreExecutor::with_awake(
        KeepAwakeService::new(Arc::new(FakeClock::new()), Box::new(backend), true),
    ));
    let mut session = Session::unbacked();
    session.host_executor = Some(executor.clone());
    assert!(session.keep_awake_preview_toggle());
    apply_one(&mut session);
    assert!(session.host_confirm(true));
    apply_one(&mut session);
    assert_eq!(
        session.awake_snapshot.as_ref().expect("cache").state,
        "active"
    );
    drop(session);
    let log = log.lock().expect("recording backend");
    assert_eq!(
        log.iter()
            .filter(|call| matches!(call, BackendCall::Acquire(_)))
            .count(),
        1
    );
    assert_eq!(
        log.iter()
            .filter(|call| matches!(call, BackendCall::Release(_)))
            .count(),
        1
    );
    drop(log);
    drop(executor);
}

#[test]
fn keep_awake_confirmation_rejects_a_status_poll_sampled_before_the_action() {
    use devsweep_core::status::{FakeClock, KeepAwakeService, RecordingBackend};
    let (backend, _) = RecordingBackend::pair();
    let executor = Arc::new(host_review::CoreExecutor::with_awake(
        KeepAwakeService::new(Arc::new(FakeClock::new()), Box::new(backend), true),
    ));
    let mut session = Session::unbacked();
    session.host_executor = Some(executor);
    for _ in 0..2 {
        assert!(session.keep_awake_preview_toggle());
        apply_one(&mut session);
        assert!(session.host_confirm(true));
        apply_one(&mut session);
    }
    assert_eq!(
        session.awake_snapshot.as_ref().expect("cache").state,
        "stopped"
    );
    assert!(session.queue_awake_poll());
    let stale = session.take_jobs().pop().expect("poll").run();
    assert!(session.keep_awake_preview_toggle());
    apply_one(&mut session);
    assert!(session.host_confirm(true));
    apply_one(&mut session);
    assert_eq!(
        session.awake_snapshot.as_ref().expect("cache").state,
        "active"
    );
    assert!(!session.apply_job(stale), "old stopped sample is obsolete");
    assert_eq!(
        session.awake_snapshot.as_ref().expect("cache").state,
        "active"
    );
}

#[test]
fn failed_keep_awake_clear_stays_unknown_and_allows_reviewed_stop_retry() {
    use devsweep_core::status::{AwakeBackend, FakeClock, KeepAwakeError, KeepAwakeService};
    struct FailClearOnce(bool);
    impl AwakeBackend for FailClearOnce {
        fn acquire(&mut self, _: &str) -> Result<(), KeepAwakeError> {
            Ok(())
        }
        fn release(&mut self, _: &str) -> Result<(), KeepAwakeError> {
            if self.0 {
                self.0 = false;
                Err(KeepAwakeError::BackendFailed {
                    operation: "clear",
                    message: "fixture".into(),
                })
            } else {
                Ok(())
            }
        }
    }
    let executor = Arc::new(host_review::CoreExecutor::with_awake(
        KeepAwakeService::new(
            Arc::new(FakeClock::new()),
            Box::new(FailClearOnce(true)),
            true,
        ),
    ));
    let mut session = Session::unbacked();
    session.host_executor = Some(executor);
    session.keep_awake_preview_toggle();
    apply_one(&mut session);
    session.host_confirm(true);
    apply_one(&mut session);
    session.keep_awake_preview_toggle();
    apply_one(&mut session);
    session.host_confirm(true);
    apply_one(&mut session);
    assert_eq!(
        session.awake_error.as_deref(),
        Some("keep_awake_clear_failed")
    );
    assert!(session.keep_awake_line().contains("unknown"));
    assert!(
        session.keep_awake_preview_toggle(),
        "explicit retry creates a new Stop review"
    );
    assert!(matches!(
        session.host_review.intents.as_slice(),
        [HostIntent::KeepAwake { enabled: false }]
    ));
    apply_one(&mut session);
    session.apply_awake_status(Err("keep_awake_clear_failed".to_string()));
    assert_eq!(
        session.host_review.phase,
        HostPhase::Ready,
        "an unchanged poll error must not erase a reviewed Stop retry"
    );
    assert!(!session.host_confirm(false));
    assert!(session.host_confirm(true));
    apply_one(&mut session);
    assert!(session.awake_error.is_none());
    assert_eq!(
        session.awake_snapshot.as_ref().expect("cache").state,
        "stopped"
    );
}

#[test]
fn failed_startup_result_cannot_keep_a_stale_enabled_badge() {
    use devsweep_core::software::{
        SoftwareScope, SoftwareStartupEntryV1, SoftwareStartupListV1, SoftwareStartupState,
        SoftwareStartupToggle,
    };
    let mut session = Session::unbacked();
    session.software_review.apply_host_load(
        crate::software_review::SoftwareHostLoad::Startup(SoftwareStartupListV1 {
            version: 1,
            observed_at_unix_ms: 1,
            sources: Vec::new(),
            entries: vec![SoftwareStartupEntryV1 {
                id: "fixture".into(),
                location: SoftwareStartupLocation::CurrentUserRun,
                scope: SoftwareScope::CurrentUser,
                name: "Fixture".into(),
                state: SoftwareStartupState::Enabled,
                toggle: SoftwareStartupToggle::Allowed,
            }],
        }),
        Locale::En,
    );
    session.software_review.apply_host_action(
        &ActionResult {
            intent: HostIntent::Startup {
                source: host_review::StartupSource::Host,
                id: "fixture".into(),
                name: "Fixture".into(),
                enabled: false,
            },
            code: "unknown_after_dispatch".into(),
            reverse: None,
        },
        Locale::En,
    );
    assert_eq!(
        session
            .software_review
            .item("fixture")
            .expect("row")
            .version,
        "unknown"
    );
}
