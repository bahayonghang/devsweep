//! Frozen-grammar Status snapshot, live, health, process, and keep-awake handlers.

use std::io;
use std::path::Path;
#[cfg(test)]
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;

use devsweep_core::status::{
    HostProcessControl, KeepAwakeError, KeepAwakeService, KeepAwakeSession, KeepAwakeStatusV1,
    KeepAwakeVerb, LiveRequest, ProcessActionAudit, ProcessActionError, ProcessTerminationPlanV1,
    StatusError, StatusEventV1, StatusSnapshotV1, TerminalReason, bind_process_termination_plan,
    capture_snapshot, explain_health, preview_bound_process, query_keep_awake, spawn_live,
    terminate_bound_process_audited,
};
#[cfg(test)]
use devsweep_core::status::{RecordingBackend, SystemClock};

use super::super::{
    cli::{
        Cli, Command, PlanExecutionCommand, PlanPresentationCommand, StatusCommand,
        StatusHealthCommand, StatusKeepAwakeCommand, StatusKeepAwakeRunCommand,
        StatusKeepAwakeTokenCommand, StatusLiveCommand, StatusProcessCommand,
        StatusProcessPlanCommand, StatusSnapshotCommand,
    },
    output::{
        ApplicationError, ExitClass, OutputSink, ResultFormat, ensure_output_absent,
        from_domain_code, write_json,
    },
    presentation,
};
use crate::i18n::{CatalogueError, Locale, catalogue};

#[cfg(test)]
pub(super) struct Route;

static CTRL_C: AtomicBool = AtomicBool::new(false);

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Status(group)) => match &group.command {
            StatusCommand::Snapshot(command) => run_snapshot(cli, locale, command),
            StatusCommand::Live(command) => run_live(cli, locale, command),
            StatusCommand::Health(command) => run_health(cli, locale, command),
            StatusCommand::Process(group) => (match &group.command {
                StatusProcessCommand::Plan(command) => run_process_plan(command),
                StatusProcessCommand::Preview(command) => run_process_preview(cli, locale, command),
                StatusProcessCommand::Terminate(command) => {
                    run_process_terminate(cli, locale, command)
                }
            })
            .map_err(|error| localize_process_action_error(cli, locale, error)),
            StatusCommand::KeepAwake(group) => match &group.command {
                StatusKeepAwakeCommand::Run(command) => run_keep_awake(cli, locale, command),
                StatusKeepAwakeCommand::Status(command) => {
                    run_keep_awake_query(cli, locale, command, "STATUS")
                }
                StatusKeepAwakeCommand::Stop(command) => {
                    run_keep_awake_query(cli, locale, command, "STOP")
                }
            },
        },
        _ => Err(mode_unavailable(cli, locale)),
    }
}

fn mode_unavailable(cli: &Cli, locale: Locale) -> ApplicationError {
    let command = cli.command_name().unwrap_or("status");
    let message = catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .unwrap_or_else(|_| format!("unavailable: {command}"));
    ApplicationError::unavailable(message)
}

fn run_snapshot(
    cli: &Cli,
    locale: Locale,
    command: &StatusSnapshotCommand,
) -> Result<(), ApplicationError> {
    let snapshot = capture_snapshot(command.process_limit, None).map_err(map_status_error)?;
    emit_snapshot(cli, locale, &snapshot)
}

fn emit_snapshot(
    cli: &Cli,
    locale: Locale,
    snapshot: &StatusSnapshotV1,
) -> Result<(), ApplicationError> {
    let outcome = snapshot.envelope_outcome();
    let warnings = snapshot.envelope_warnings();
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::status::snapshot(locale, snapshot).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)?;
        }
        _ => {
            let mut sink = OutputSink::open(cli.output_path())?;
            write_json(
                &mut sink,
                &serde_json::json!({
                    "schema_version": 1,
                    "command": "status.snapshot",
                    "outcome": outcome,
                    "data": snapshot,
                    "warnings": warnings,
                    "error": serde_json::Value::Null
                }),
            )?;
        }
    }
    if outcome == "partial" {
        return Err(ApplicationError::after_output(
            ExitClass::Partial,
            "status_partial",
            "status snapshot is partial",
        ));
    }
    Ok(())
}

fn run_live(
    cli: &Cli,
    locale: Locale,
    command: &StatusLiveCommand,
) -> Result<(), ApplicationError> {
    install_ctrl_c_handler();
    CTRL_C.store(false, Ordering::SeqCst);
    let interval_ms = command.interval.saturating_mul(1_000);
    let (rx, mut control) = spawn_live(LiveRequest {
        interval_ms,
        process_limit: command.process_limit,
        operation_id: format!("op-{}", unix_now_ms()),
        cancel: None,
    })
    .map_err(map_status_error)?;

    let mut sink = OutputSink::open(cli.output_path())?;
    let ndjson = cli.result_format() == ResultFormat::Ndjson;
    loop {
        if CTRL_C.load(Ordering::SeqCst) {
            control.request_cancel();
        }
        let event = match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                control.join();
                return Ok(());
            }
        };
        match write_live_event(&mut sink, locale, ndjson, &event) {
            Ok(()) => {}
            Err(error) if error.code == "broken_pipe" => {
                control.request_cancel();
                control.join();
                return Ok(());
            }
            Err(error) => {
                control.request_cancel();
                control.join();
                if error.exit == ExitClass::Failed {
                    let _ = write_producer_error_terminal(&mut sink, &event, error.code);
                    return Err(ApplicationError::after_output(
                        ExitClass::Failed,
                        error.code,
                        error.message,
                    ));
                }
                return Err(error);
            }
        }
        if event.is_terminal() {
            control.join();
            return Ok(());
        }
    }
}

fn write_live_event(
    sink: &mut OutputSink,
    locale: Locale,
    ndjson: bool,
    event: &StatusEventV1,
) -> Result<(), ApplicationError> {
    if ndjson {
        let mut bytes = serde_json::to_vec(event).map_err(|error| {
            ApplicationError::failed("output_serialize_failed", error.to_string())
        })?;
        bytes.push(b'\n');
        return sink.write_all(&bytes).map_err(classify_write);
    }
    let Some(text) = presentation::status::live_event(locale, event)
        .map_err(|error| ApplicationError::failed("catalogue_render_failed", error.to_string()))?
    else {
        return Ok(());
    };
    let mut bytes = text.into_bytes();
    bytes.push(b'\n');
    sink.write_all(&bytes).map_err(classify_write)
}

fn write_producer_error_terminal(
    sink: &mut OutputSink,
    previous: &StatusEventV1,
    error_code: &'static str,
) -> Result<(), ApplicationError> {
    let event = StatusEventV1::Terminal {
        schema_version: 1,
        operation_id: previous.operation_id().to_string(),
        sequence: previous.sequence(),
        emitted_at_unix_ms: unix_now_ms(),
        data: devsweep_core::status::StatusTerminalV1 {
            reason: TerminalReason::ProducerError,
            error_code: Some(error_code.to_string()),
        },
    };
    let mut bytes = serde_json::to_vec(&event)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes).map_err(classify_write)
}

fn classify_write(error: io::Error) -> ApplicationError {
    if error.kind() == io::ErrorKind::BrokenPipe {
        ApplicationError::new(ExitClass::Success, "broken_pipe", "output pipe closed")
    } else {
        ApplicationError::failed(
            "output_write_failed",
            format!("failed to write output: {error}"),
        )
    }
}

fn write_human(cli: &Cli, text: &str) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(cli.output_path())?;
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(b'\n');
    sink.write_all(&bytes).map_err(classify_write)
}

fn map_status_error(error: StatusError) -> ApplicationError {
    match error {
        StatusError::Canceled => {
            ApplicationError::new(ExitClass::Success, "status_canceled", error.to_string())
        }
        StatusError::CoordinatorRefused => {
            ApplicationError::new(ExitClass::Unavailable, "status_busy", error.to_string())
        }
        StatusError::ChannelClosed => {
            ApplicationError::failed("status_channel_closed", error.to_string())
        }
    }
}

fn run_health(
    cli: &Cli,
    locale: Locale,
    command: &StatusHealthCommand,
) -> Result<(), ApplicationError> {
    let snapshot = capture_snapshot(command.process_limit, None).map_err(map_status_error)?;
    let explanation = explain_health(&snapshot);
    let data = serde_json::to_value(&explanation)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    emit_result(
        cli,
        "status.health",
        "succeeded",
        data,
        presentation::status::health(locale, &explanation),
    )
}

fn run_process_plan(command: &StatusProcessPlanCommand) -> Result<(), ApplicationError> {
    let plan = bind_process_termination_plan(command.pid, command.creation_time, &command.name)
        .map_err(map_process_action)?;
    write_plan(&command.output, &plan)
}

fn run_process_preview(
    cli: &Cli,
    locale: Locale,
    command: &PlanPresentationCommand,
) -> Result<(), ApplicationError> {
    let plan = load_process_plan(&command.plan)?;
    let preview = preview_bound_process(&plan, &HostProcessControl).map_err(map_process_action)?;
    let data = serde_json::to_value(&preview)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    emit_result(
        cli,
        "status.process.preview",
        "succeeded",
        data,
        presentation::status::process_preview(locale, &preview),
    )
}

fn run_process_terminate(
    cli: &Cli,
    locale: Locale,
    command: &PlanExecutionCommand,
) -> Result<(), ApplicationError> {
    let plan = load_process_plan(&command.plan)?;
    let probe = HostProcessControl;
    let mut killer = HostProcessControl;
    run_process_terminate_with(
        cli,
        locale,
        command,
        &plan,
        &probe,
        &mut killer,
        &mut process_audit(),
    )
}

#[cfg(not(test))]
fn process_audit() -> devsweep_core::status::FileProcessActionAudit {
    devsweep_core::status::FileProcessActionAudit::production()
}

#[cfg(test)]
fn process_audit() -> devsweep_core::status::RecordingProcessActionAudit {
    devsweep_core::status::RecordingProcessActionAudit::default()
}

fn run_process_terminate_with(
    cli: &Cli,
    locale: Locale,
    command: &PlanExecutionCommand,
    plan: &ProcessTerminationPlanV1,
    probe: &dyn devsweep_core::status::ProcessProbe,
    killer: &mut dyn devsweep_core::status::ProcessKiller,
    audit: &mut dyn ProcessActionAudit,
) -> Result<(), ApplicationError> {
    ensure_output_absent(cli.output_path())?;
    let report = terminate_bound_process_audited(
        plan,
        &command.preview_digest,
        command.confirm,
        probe,
        killer,
        audit,
    )
    .map_err(map_process_action)?;
    let data = serde_json::to_value(&report)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    emit_result(
        cli,
        "status.process.terminate",
        "succeeded",
        data,
        presentation::status::process_report(locale, &report),
    )
}

fn load_process_plan(path: &Path) -> Result<ProcessTerminationPlanV1, ApplicationError> {
    let bytes = std::fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read process plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ApplicationError::usage("invalid_plan", "process termination plan is invalid"))
}

fn write_plan(path: &Path, plan: &ProcessTerminationPlanV1) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(Some(path))?;
    let mut bytes = serde_json::to_vec_pretty(plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn map_process_action(error: ProcessActionError) -> ApplicationError {
    from_domain_code(error.code(), error.to_string())
}

fn localize_process_action_error(
    cli: &Cli,
    locale: Locale,
    mut error: ApplicationError,
) -> ApplicationError {
    if cli.result_format() != ResultFormat::Human {
        return error;
    }
    let key = match error.code {
        "protected" => "status.v1.process.error.protected",
        "invalid_identity" | "stale_confirmation" => "status.v1.process.error.identity",
        "confirmation_required" => "status.v1.process.error.confirmation",
        "audit_unavailable" => "status.v1.process.error.audit",
        "unknown_after_dispatch" => "status.v1.process.error.unknown",
        _ => "status.v1.process.error.unavailable",
    };
    error.message = catalogue(locale)
        .render(key, &[("detail", &error.message)], None)
        .unwrap_or(error.message);
    error
}

fn run_keep_awake(
    cli: &Cli,
    locale: Locale,
    command: &StatusKeepAwakeRunCommand,
) -> Result<(), ApplicationError> {
    install_ctrl_c_handler();
    CTRL_C.store(false, Ordering::SeqCst);
    let session = KeepAwakeSession::open(
        session_service(),
        &command.owner,
        Duration::from_millis(command.duration_ms),
        command.confirm,
    )
    .map_err(|error| localize_keep_awake_error(cli, locale, map_keep_awake(error)))?;
    let lease = session
        .status()
        .map_err(|error| localize_keep_awake_error(cli, locale, map_keep_awake(error)))?;
    emit_keep_awake(
        cli,
        locale,
        "status.keep-awake.run",
        session.endpoint(),
        &lease,
    )?;
    hold_keep_awake_session(session, &CTRL_C)
        .map_err(|error| keep_awake_after_output(localize_keep_awake_error(cli, locale, error)))
}

fn keep_awake_after_output(error: ApplicationError) -> ApplicationError {
    // The initial lease document is already complete. Preserve its single-document
    // contract while still reporting later session/cleanup failures and their exit.
    eprintln!("{}: {}", error.code, error.message);
    ApplicationError::after_output(error.exit, error.code, error.message)
}

fn run_keep_awake_query(
    cli: &Cli,
    locale: Locale,
    command: &StatusKeepAwakeTokenCommand,
    verb: &str,
) -> Result<(), ApplicationError> {
    let protocol_verb = if verb == "STOP" {
        KeepAwakeVerb::Stop
    } else {
        KeepAwakeVerb::Status
    };
    let lease = query_keep_awake(
        &command.endpoint,
        protocol_verb,
        &command.token,
        &command.owner,
    )
    .map_err(|error| localize_keep_awake_error(cli, locale, map_keep_awake(error)))?;
    let command_name = if verb == "STOP" {
        "status.keep-awake.stop"
    } else {
        "status.keep-awake.status"
    };
    emit_keep_awake(cli, locale, command_name, &command.endpoint, &lease)
}

/// Test builds record the lease and do not request a thread execution state.
fn session_service() -> KeepAwakeService {
    #[cfg(test)]
    {
        let (backend, _) = RecordingBackend::pair();
        KeepAwakeService::new(Arc::new(SystemClock), Box::new(backend), true)
    }
    #[cfg(not(test))]
    {
        KeepAwakeService::production()
    }
}

fn hold_keep_awake_session(
    mut session: KeepAwakeSession,
    cancel: &AtomicBool,
) -> Result<(), ApplicationError> {
    let outcome = loop {
        if cancel.load(Ordering::SeqCst) {
            break session.cancel().map(|_| ()).map_err(map_keep_awake);
        }
        match session.status() {
            Ok(status) if status.state == "active" => thread::sleep(Duration::from_millis(20)),
            Ok(_) => break Ok(()),
            Err(error) => break Err(map_keep_awake(error)),
        }
    };
    let closed = session.shutdown().map_err(map_keep_awake);
    outcome?;
    closed?;
    Ok(())
}

fn map_keep_awake(error: KeepAwakeError) -> ApplicationError {
    let code = match error {
        KeepAwakeError::CapabilityGap { .. } => "unsupported_platform",
        KeepAwakeError::ConfirmRequired => "confirmation_required",
        KeepAwakeError::DurationNotFinite | KeepAwakeError::InvalidOwner => "invalid_plan",
        KeepAwakeError::UnknownToken | KeepAwakeError::OwnerMismatch => "invalid_identity",
        KeepAwakeError::BackendFailed { .. } => "keep_awake_backend_failed",
        KeepAwakeError::WorkerUnavailable => "keep_awake_worker_unavailable",
        KeepAwakeError::SessionIo { .. } => "output_open_failed",
        KeepAwakeError::InvalidEndpoint => "invalid_endpoint",
        KeepAwakeError::InvalidRequest | KeepAwakeError::InvalidReply => "invalid_plan",
        KeepAwakeError::NetworkUnavailable { .. } => "network_unavailable",
        KeepAwakeError::StaleConfirmation => "stale_confirmation",
        KeepAwakeError::AuditFailed {
            after_dispatch: false,
            ..
        } => "audit_unavailable",
        KeepAwakeError::AuditFailed {
            after_dispatch: true,
            ..
        } => "unknown_after_dispatch",
    };
    if code == "invalid_endpoint" {
        return ApplicationError::usage(code, error.to_string());
    }
    from_domain_code(code, error.to_string())
}

fn localize_keep_awake_error(
    cli: &Cli,
    locale: Locale,
    mut error: ApplicationError,
) -> ApplicationError {
    if cli.result_format() != ResultFormat::Human {
        return error;
    }
    let key = match error.code {
        "unsupported_platform" => "status.v1.keep_awake.error.platform",
        "confirmation_required" => "status.v1.keep_awake.error.confirmation",
        "invalid_plan" => "status.v1.keep_awake.error.request",
        "invalid_identity" => "status.v1.keep_awake.error.identity",
        "invalid_endpoint" => "status.v1.keep_awake.error.endpoint",
        "network_unavailable" => "status.v1.keep_awake.error.network",
        "stale_confirmation" => "status.v1.keep_awake.error.stale",
        "audit_unavailable" => "status.v1.keep_awake.error.audit",
        "unknown_after_dispatch" => "status.v1.keep_awake.error.unknown",
        "keep_awake_backend_failed" => "status.v1.keep_awake.error.backend",
        "keep_awake_worker_unavailable" => "status.v1.keep_awake.error.worker",
        "output_open_failed" => "status.v1.keep_awake.error.io",
        _ => return error,
    };
    error.message = catalogue(locale)
        .render(key, &[("detail", &error.message)], None)
        .unwrap_or(error.message);
    error
}

fn emit_keep_awake(
    cli: &Cli,
    locale: Locale,
    command: &str,
    endpoint: &str,
    lease: &KeepAwakeStatusV1,
) -> Result<(), ApplicationError> {
    let data = serde_json::json!({
        "endpoint": endpoint,
        "lease": lease,
    });
    emit_result(
        cli,
        command,
        "succeeded",
        data,
        presentation::status::keep_awake(locale, endpoint, lease),
    )
}

fn emit_result(
    cli: &Cli,
    command: &str,
    outcome: &str,
    data: serde_json::Value,
    human: Result<String, CatalogueError>,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text = human.map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(cli, command, outcome, data),
    }
}

fn write_envelope(
    cli: &Cli,
    command: &str,
    outcome: &str,
    data: serde_json::Value,
) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(cli.output_path())?;
    write_json(
        &mut sink,
        &serde_json::json!({
            "schema_version": 1,
            "command": command,
            "outcome": outcome,
            "data": data,
            "warnings": [],
            "error": null
        }),
    )
}

fn unix_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn install_ctrl_c_handler() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn SetConsoleCtrlHandler(
                handler: Option<unsafe extern "system" fn(u32) -> i32>,
                add: i32,
            ) -> i32;
        }
        // SAFETY: the handler only stores an atomic flag.
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(ctrl_handler), 1);
        }
    }
    #[cfg(unix)]
    {
        let _ = &CTRL_C;
    }
}

#[cfg(windows)]
unsafe extern "system" fn ctrl_handler(control: u32) -> i32 {
    if matches!(control, 0..=2) {
        CTRL_C.store(true, Ordering::SeqCst);
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("status CLI parses")
    }

    struct ProcessProbeFixture;
    impl devsweep_core::status::ProcessProbe for ProcessProbeFixture {
        fn lookup(
            &self,
            _: u32,
        ) -> Result<Option<devsweep_core::status::ObservedProcess>, ProcessActionError> {
            Ok(Some(devsweep_core::status::ObservedProcess {
                pid: 42,
                creation_time_filetime: 1000,
                name: "fixture.exe".into(),
            }))
        }
    }
    #[derive(Default)]
    struct ProcessKillerFixture(usize);
    impl devsweep_core::status::ProcessKiller for ProcessKillerFixture {
        fn terminate(&mut self, pid: u32, creation: u64) -> Result<(), ProcessActionError> {
            assert_eq!((pid, creation), (42, 1000));
            self.0 += 1;
            Ok(())
        }
    }

    #[test]
    fn process_fixture_execution_keeps_json_locale_invariant_and_requires_confirmation_and_live_digest()
     {
        use devsweep_core::status::RecordingProcessActionAudit;
        let root = TempDir::new().expect("isolated outputs");
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        let digest = preview_bound_process(&plan, &ProcessProbeFixture)
            .expect("preview")
            .digest;
        let mut envelopes = Vec::new();
        for (index, locale) in [Locale::En, Locale::ZhCn].into_iter().enumerate() {
            let output = root.path().join(format!("{index}.json"));
            let args = [
                "devsweep",
                "status",
                "process",
                "terminate",
                "--plan",
                "fixture.json",
                "--preview-digest",
                &digest,
                "--confirm",
                "--format",
                "json",
                "--output",
                output.to_str().expect("path"),
            ];
            let cli = parse(&args);
            let Some(Command::Status(group)) = &cli.command else {
                panic!("status");
            };
            let StatusCommand::Process(group) = &group.command else {
                panic!("process");
            };
            let StatusProcessCommand::Terminate(command) = &group.command else {
                panic!("terminate");
            };
            let mut killer = ProcessKillerFixture::default();
            let mut audit = RecordingProcessActionAudit::default();
            run_process_terminate_with(
                &cli,
                locale,
                command,
                &plan,
                &ProcessProbeFixture,
                &mut killer,
                &mut audit,
            )
            .expect("fake termination");
            assert_eq!(killer.0, 1);
            assert_eq!(audit.0.len(), 2);
            envelopes.push(
                serde_json::from_slice::<serde_json::Value>(
                    &std::fs::read(&output).expect("output"),
                )
                .expect("JSON"),
            );
            let mut refused_killer = ProcessKillerFixture::default();
            let mut refused_audit = RecordingProcessActionAudit::default();
            assert_eq!(
                run_process_terminate_with(
                    &cli,
                    locale,
                    command,
                    &plan,
                    &ProcessProbeFixture,
                    &mut refused_killer,
                    &mut refused_audit,
                )
                .expect_err("existing output refuses before termination")
                .code,
                "output_exists"
            );
            assert_eq!(refused_killer.0, 0);
            assert!(refused_audit.0.is_empty());
            std::fs::remove_file(&output).expect("remove owned output for refusal cases");
            let mut stale =
                bind_process_termination_plan(42, 1001, "fixture.exe").expect("reused plan");
            let mut never = ProcessKillerFixture::default();
            let mut never_audit = RecordingProcessActionAudit::default();
            assert_eq!(
                run_process_terminate_with(
                    &cli,
                    locale,
                    command,
                    &stale,
                    &ProcessProbeFixture,
                    &mut never,
                    &mut never_audit
                )
                .expect_err("PID reused")
                .code,
                "invalid_identity"
            );
            stale.creation_time_filetime = "1000".into();
            stale.name = "different.exe".into();
            assert!(
                run_process_terminate_with(
                    &cli,
                    locale,
                    command,
                    &stale,
                    &ProcessProbeFixture,
                    &mut never,
                    &mut never_audit
                )
                .is_err()
            );
            assert_eq!(never.0, 0);
            assert!(never_audit.0.is_empty());
        }
        assert_eq!(envelopes[0], envelopes[1]);
        assert_eq!(envelopes[0]["data"]["outcome"], "terminated");
        assert!(
            Cli::try_parse_from([
                "devsweep",
                "status",
                "process",
                "terminate",
                "--plan",
                "fixture.json",
                "--preview-digest",
                &digest
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "devsweep",
                "status",
                "process",
                "terminate",
                "--plan",
                "fixture.json",
                "--confirm"
            ])
            .is_err()
        );
    }

    #[test]
    fn process_bilingual_preview_results_and_errors_keep_machine_identity_unchanged() {
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        let preview = preview_bound_process(&plan, &ProcessProbeFixture).expect("preview");
        let en = presentation::status::process_preview(Locale::En, &preview).expect("English");
        let zh = presentation::status::process_preview(Locale::ZhCn, &preview).expect("Chinese");
        assert!(en.contains("unsaved work"));
        assert!(zh.contains("未保存") && !zh.contains("unsaved work"));
        for line in [&en, &zh] {
            assert!(
                line.contains("fixture.exe")
                    && line.contains("42")
                    && line.contains(&preview.digest)
            );
        }
        let cli = parse(&[
            "devsweep",
            "status",
            "process",
            "preview",
            "--plan",
            "fixture.json",
        ]);
        let json = parse(&[
            "devsweep",
            "status",
            "process",
            "preview",
            "--plan",
            "fixture.json",
            "--format",
            "json",
        ]);
        let error = || {
            map_process_action(ProcessActionError::AuditFailed {
                after_dispatch: true,
            })
        };
        let en = localize_process_action_error(&cli, Locale::En, error());
        let zh = localize_process_action_error(&cli, Locale::ZhCn, error());
        assert!(en.message.contains("unknown"));
        assert!(zh.message.contains("结果未知"));
        assert_eq!(en.code, zh.code);
        assert_eq!(
            localize_process_action_error(&json, Locale::En, error()).message,
            localize_process_action_error(&json, Locale::ZhCn, error()).message
        );
        let report = devsweep_core::status::ProcessTerminationReportV1 {
            version: 1,
            pid: 42,
            creation_time_filetime: "1000".into(),
            name: "fixture.exe".into(),
            outcome: "terminated".into(),
        };
        assert!(
            presentation::status::process_report(Locale::En, &report)
                .expect("English result")
                .contains("requested")
        );
        assert!(
            presentation::status::process_report(Locale::ZhCn, &report)
                .expect("Chinese result")
                .contains("已请求")
        );
    }

    #[test]
    fn snapshot_json_is_locale_invariant_and_privacy_limited() {
        let temp = TempDir::new().unwrap();
        let en_path = temp.path().join("en.json");
        let zh_path = temp.path().join("zh.json");
        let en = parse(&[
            "devsweep",
            "status",
            "snapshot",
            "--format",
            "json",
            "--output",
            en_path.to_str().unwrap(),
        ]);
        let zh = parse(&[
            "devsweep",
            "status",
            "snapshot",
            "--format",
            "json",
            "--output",
            zh_path.to_str().unwrap(),
        ]);
        let first = run(&en, Locale::En);
        let second = run(&zh, Locale::ZhCn);
        assert!(
            first.is_ok()
                || first
                    .as_ref()
                    .is_err_and(|error| error.exit == ExitClass::Partial)
        );
        assert!(
            second.is_ok()
                || second
                    .as_ref()
                    .is_err_and(|error| error.exit == ExitClass::Partial)
        );
        let en_bytes = std::fs::read_to_string(&en_path).unwrap();
        let zh_bytes = std::fs::read_to_string(&zh_path).unwrap();
        let en_doc: serde_json::Value = serde_json::from_str(&en_bytes).unwrap();
        let zh_doc: serde_json::Value = serde_json::from_str(&zh_bytes).unwrap();
        assert_eq!(en_doc["command"], "status.snapshot");
        assert_eq!(zh_doc["command"], "status.snapshot");
        assert_eq!(en_doc["schema_version"], 1);
        assert!(en_doc["error"].is_null());
        for text in [&en_bytes, &zh_bytes] {
            assert!(!text.contains("cmdline"));
            assert!(!text.contains("command_line"));
            assert!(!text.contains("environment"));
            assert!(!text.contains("CleanupPlan"));
        }
        let codes = en_doc["data"]["unsupported_capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|capability| capability["code"].as_str().unwrap())
            .collect::<Vec<_>>();
        for code in ["vram", "fan", "smart"] {
            assert!(codes.contains(&code), "missing {code}");
        }
        for (group, code) in [
            ("gpu", "gpu_utilization"),
            ("thermal", "thermal"),
            ("disk_activity", "physical_disk_activity"),
        ] {
            let state = en_doc["data"][group]["state"].as_str().unwrap();
            let measured = state == "available" || state == "partial";
            assert_eq!(codes.contains(&code), !measured, "{group} {state}");
        }
        for (group, field, code) in [
            ("cpu", "frequency_mhz", "cpu_frequency"),
            ("power", "full_charge_basis_points", "battery_health"),
            ("power", "cycle_count", "battery_cycle_count"),
            ("power", "rate_milliwatts", "battery_rate"),
        ] {
            let value = &en_doc["data"][group]["value"][field];
            assert_eq!(
                codes.contains(&code),
                value.is_null(),
                "{group}.{field} {value}"
            );
        }
        assert!(en_doc["data"]["host"]["state"].is_string());
    }

    #[test]
    fn coordinator_refusal_is_unavailable() {
        assert_eq!(
            map_status_error(StatusError::CoordinatorRefused).code,
            "status_busy"
        );
        assert_eq!(
            map_status_error(StatusError::CoordinatorRefused).exit,
            ExitClass::Unavailable
        );
    }

    #[test]
    fn broken_pipe_classifier_is_success() {
        let error = classify_write(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
        assert_eq!(error.exit, ExitClass::Success);
        assert_eq!(error.code, "broken_pipe");
    }

    #[test]
    fn protected_critical_shared_and_system_process_names_refuse_before_a_plan_file() {
        let temp = TempDir::new().unwrap();
        for name in ["lsass.exe", "svchost.exe", "System"] {
            let path = temp.path().join(format!("{name}.json"));
            let cli = parse(&[
                "devsweep",
                "status",
                "process",
                "plan",
                "--pid",
                "424242",
                "--creation-time",
                "1000",
                "--name",
                name,
                "--output",
                path.to_str().unwrap(),
            ]);
            let error = run(&cli, Locale::En).expect_err("protected name");
            assert_eq!(error.code, "protected");
            assert_eq!(error.exit, ExitClass::Unavailable);
            assert!(!path.exists(), "{name} plan file exists");
        }
    }

    #[test]
    fn status_health_names_evidence_without_an_opaque_score() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("health.json");
        let cli = parse(&[
            "devsweep",
            "status",
            "health",
            "--format",
            "json",
            "--output",
            path.to_str().unwrap(),
        ]);
        run(&cli, Locale::En).expect("health succeeds even when sensors are gaps");
        let text = std::fs::read_to_string(&path).unwrap();
        let document: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(document["command"], "status.health");
        assert_eq!(document["outcome"], "succeeded");
        assert!(document["data"].get("score").is_none());
        assert!(document["data"]["evidence"].as_array().unwrap().len() > 1);
        let cpu = document["data"]["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["subject"] == "cpu")
            .expect("cpu evidence");
        let cpu_text = cpu["text"].as_str().unwrap();
        if cpu_text.contains("system fault") {
            assert!(cpu_text.contains("CPU load is not a system fault"));
        }
        if cpu["state"] == "unavailable" {
            assert!(!cpu_text.contains("0 basis"));
        }
    }

    #[test]
    fn keep_awake_unsupported_platform_returns_a_capability_gap_and_changes_nothing() {
        let (backend, log) = RecordingBackend::pair();
        let service = KeepAwakeService::new(Arc::new(SystemClock), Box::new(backend), false);
        let error = match KeepAwakeSession::open(service, "owner-a", Duration::from_secs(1), true) {
            Err(error) => map_keep_awake(error),
            Ok(_) => panic!("unsupported platform stored a lease"),
        };
        assert_eq!(error.code, "unsupported_platform");
        assert_eq!(error.exit, ExitClass::Unavailable);
        assert!(log.lock().expect("log").is_empty());
    }

    #[test]
    fn keep_awake_shared_protocol_preserves_cli_stop_cancel_expiry_and_error_classes() {
        use devsweep_core::status::{BackendCall, FakeClock};
        let clock = Arc::new(FakeClock::new());
        let (backend, log) = RecordingBackend::pair();
        let service = KeepAwakeService::new(clock.clone(), Box::new(backend), true);
        let session = KeepAwakeSession::open(service, "cli-a", Duration::from_millis(50), true)
            .expect("session");
        let endpoint = session.endpoint().to_string();
        let token = session.token().to_string();
        let wrong = query_keep_awake(&endpoint, KeepAwakeVerb::Stop, &token, "other-owner")
            .expect_err("owner mismatch");
        assert_eq!(map_keep_awake(wrong).code, "invalid_identity");
        assert_eq!(
            query_keep_awake(&endpoint, KeepAwakeVerb::Status, &token, "cli-a")
                .expect("active")
                .state,
            "active"
        );
        clock.advance(Duration::from_millis(50));
        hold_keep_awake_session(session, &AtomicBool::new(false)).expect("expiry exits CLI hold");
        let closed = query_keep_awake(&endpoint, KeepAwakeVerb::Status, &token, "cli-a")
            .expect_err("closed endpoint remains unknown");
        assert_eq!(map_keep_awake(closed).exit, ExitClass::Unavailable);
        assert!(
            log.lock()
                .expect("log")
                .iter()
                .any(|call| matches!(call, BackendCall::Release(_)))
        );

        let service = session_service();
        let session = KeepAwakeSession::open(service, "cli-b", Duration::from_secs(30), true)
            .expect("session");
        let tuple = (session.endpoint().to_string(), session.token().to_string());
        let hold = thread::spawn(move || hold_keep_awake_session(session, &AtomicBool::new(false)));
        assert_eq!(
            query_keep_awake(&tuple.0, KeepAwakeVerb::Stop, &tuple.1, "cli-b")
                .expect("stop")
                .state,
            "stopped"
        );
        hold.join()
            .expect("hold joined")
            .expect("stop exits CLI hold");

        let session =
            KeepAwakeSession::open(session_service(), "cli-c", Duration::from_secs(30), true)
                .expect("session");
        hold_keep_awake_session(session, &AtomicBool::new(true)).expect("cancel exits CLI hold");
        assert_eq!(
            map_keep_awake(KeepAwakeError::InvalidEndpoint).code,
            "invalid_endpoint"
        );
        assert_eq!(
            map_keep_awake(KeepAwakeError::InvalidEndpoint).exit,
            ExitClass::Usage
        );
        assert_eq!(
            map_keep_awake(KeepAwakeError::BackendFailed {
                operation: "release",
                message: "fixture".into(),
            })
            .exit,
            ExitClass::Failed
        );
    }

    #[cfg(windows)]
    #[test]
    fn fixture_owned_process_is_the_only_native_termination_target() {
        use devsweep_core::status::{ProcessKiller, ProcessProbe};
        use std::process::{Command, Stdio};
        use std::time::Instant;

        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let ping = std::path::PathBuf::from(root)
            .join("System32")
            .join("ping.exe");
        let child = Command::new(ping)
            .args(["-n", "30", "127.0.0.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn ping");
        struct Guard(Option<std::process::Child>);
        impl Drop for Guard {
            fn drop(&mut self) {
                if let Some(child) = self.0.as_mut() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
        let mut guard = Guard(Some(child));
        let child = guard.0.as_mut().expect("child");
        let observed = HostProcessControl
            .lookup(child.id())
            .expect("lookup")
            .expect("spawned process is visible");
        assert!(observed.name.eq_ignore_ascii_case("ping.exe"));
        assert_eq!(
            HostProcessControl.terminate(
                observed.pid,
                observed.creation_time_filetime.wrapping_add(1)
            ),
            Err(ProcessActionError::IdentityMismatch),
            "the actual kill handle must reject a changed creation time"
        );
        assert!(child.try_wait().expect("owned child still live").is_none());
        let temp = TempDir::new().unwrap();
        let plan_path = temp.path().join("plan.json");
        let preview_path = temp.path().join("preview.json");
        let report_path = temp.path().join("report.json");
        let creation = observed.creation_time_filetime.to_string();
        let pid = observed.pid.to_string();
        let plan_cli = parse(&[
            "devsweep",
            "status",
            "process",
            "plan",
            "--pid",
            &pid,
            "--creation-time",
            &creation,
            "--name",
            &observed.name,
            "--output",
            plan_path.to_str().unwrap(),
        ]);
        run(&plan_cli, Locale::En).expect("plan");
        let preview_cli = parse(&[
            "devsweep",
            "status",
            "process",
            "preview",
            "--plan",
            plan_path.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            preview_path.to_str().unwrap(),
        ]);
        run(&preview_cli, Locale::En).expect("preview");
        let preview: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&preview_path).unwrap()).unwrap();
        let data_loss = preview["data"]["data_loss_text"].as_str().unwrap();
        assert!(data_loss.contains("discard unsaved work and in-memory data"));
        let digest = preview["data"]["digest"].as_str().unwrap().to_string();

        let mut drifted: ProcessTerminationPlanV1 =
            serde_json::from_str(&std::fs::read_to_string(&plan_path).unwrap()).unwrap();
        drifted.creation_time_filetime =
            observed.creation_time_filetime.wrapping_add(1).to_string();
        let drifted_path = temp.path().join("drifted.json");
        std::fs::write(&drifted_path, serde_json::to_vec_pretty(&drifted).unwrap()).unwrap();
        let drift_cli = parse(&[
            "devsweep",
            "status",
            "process",
            "terminate",
            "--plan",
            drifted_path.to_str().unwrap(),
            "--preview-digest",
            &digest,
            "--confirm",
            "--format",
            "json",
            "--output",
            temp.path().join("drift.json").to_str().unwrap(),
        ]);
        let mismatch = run(&drift_cli, Locale::En).expect_err("identity drift");
        assert_eq!(mismatch.code, "invalid_identity");
        assert_eq!(mismatch.exit, ExitClass::InvalidAuthority);
        assert!(child.try_wait().expect("poll").is_none());

        let terminate_cli = parse(&[
            "devsweep",
            "status",
            "process",
            "terminate",
            "--plan",
            plan_path.to_str().unwrap(),
            "--preview-digest",
            &digest,
            "--confirm",
            "--format",
            "json",
            "--output",
            report_path.to_str().unwrap(),
        ]);
        run(&terminate_cli, Locale::En).expect("terminate spawned process");
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
        assert_eq!(report["data"]["pid"], observed.pid);
        assert_eq!(report["data"]["outcome"], "terminated");
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if child.try_wait().expect("wait").is_some() {
                break;
            }
            if Instant::now() >= deadline {
                panic!("spawned process was not terminated");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
    #[test]
    fn keep_awake_errors_localize_human_details_and_keep_machine_messages_unchanged() {
        let human = Cli::try_parse_from([
            "devsweep",
            "status",
            "keep-awake",
            "status",
            "--endpoint",
            "127.0.0.1:1",
            "--token",
            "test",
            "--owner",
            "test",
        ])
        .expect("human CLI");
        let json = Cli::try_parse_from([
            "devsweep",
            "status",
            "keep-awake",
            "status",
            "--endpoint",
            "127.0.0.1:1",
            "--token",
            "test",
            "--owner",
            "test",
            "--format",
            "json",
        ])
        .expect("JSON CLI");
        let failure = || {
            map_keep_awake(KeepAwakeError::BackendFailed {
                operation: "release",
                message: "fixture native error 123".into(),
            })
        };
        let native = failure();
        let english = localize_keep_awake_error(&human, Locale::En, failure());
        let chinese = localize_keep_awake_error(&human, Locale::ZhCn, failure());
        assert!(english.message.contains("operating-system"));
        assert!(chinese.message.contains("操作系统"));
        for localized in [&english, &chinese] {
            assert!(localized.message.contains("fixture native error 123"));
            assert_eq!(localized.code, native.code);
            assert_eq!(localized.exit, ExitClass::Failed);
        }
        let machine = localize_keep_awake_error(&json, Locale::ZhCn, failure());
        assert_eq!(machine.message, native.message);
    }

    #[test]
    fn keep_awake_late_cleanup_failure_keeps_the_emitted_json_and_real_exit() {
        struct FailingClear;
        impl devsweep_core::status::AwakeBackend for FailingClear {
            fn acquire(&mut self, _: &str) -> Result<(), KeepAwakeError> {
                Ok(())
            }
            fn release(&mut self, _: &str) -> Result<(), KeepAwakeError> {
                Err(KeepAwakeError::BackendFailed {
                    operation: "release",
                    message: "fixture clear failure".into(),
                })
            }
        }
        let isolated = tempfile::TempDir::new().expect("isolated output");
        let path = isolated.path().join("lease.json");
        let cli = Cli::try_parse_from([
            "devsweep",
            "status",
            "keep-awake",
            "run",
            "--owner",
            "fixture",
            "--duration-ms",
            "30000",
            "--confirm",
            "--format",
            "json",
            "--output",
            path.to_str().expect("path"),
        ])
        .expect("run CLI");
        let service = KeepAwakeService::new(Arc::new(SystemClock), Box::new(FailingClear), true);
        let session = KeepAwakeSession::open(service, "fixture", Duration::from_secs(30), true)
            .expect("fake lease");
        emit_keep_awake(
            &cli,
            Locale::En,
            "status.keep-awake.run",
            session.endpoint(),
            &session.status().expect("lease"),
        )
        .expect("single document");
        let before = std::fs::read(&path).expect("output");
        let error = keep_awake_after_output(
            hold_keep_awake_session(session, &AtomicBool::new(true)).expect_err("clear failure"),
        );
        assert!(
            error.output_already_emitted,
            "top-level must not open the output again"
        );
        assert_eq!(error.exit, ExitClass::Failed);
        assert_eq!(error.code, "keep_awake_backend_failed");
        assert!(error.message.contains("fixture clear failure"));
        assert_eq!(std::fs::read(&path).expect("unchanged"), before);
        serde_json::from_slice::<serde_json::Value>(&before).expect("exactly one JSON document");
    }
}
