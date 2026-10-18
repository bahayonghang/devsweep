//! Frozen-grammar Status snapshot, live, health, process, and keep-awake handlers.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;

use devsweep_core::status::{
    HostProcessControl, KeepAwakeError, KeepAwakeService, KeepAwakeStatusV1, LiveRequest,
    ProcessActionError, ProcessTerminationPlanV1, StatusError, StatusEventV1, StatusSnapshotV1,
    TerminalReason, bind_process_termination_plan, capture_snapshot, explain_health,
    preview_bound_process, spawn_live, terminate_bound_process,
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
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, from_domain_code, write_json},
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
            StatusCommand::Process(group) => match &group.command {
                StatusProcessCommand::Plan(command) => run_process_plan(command),
                StatusProcessCommand::Preview(command) => run_process_preview(cli, locale, command),
                StatusProcessCommand::Terminate(command) => {
                    run_process_terminate(cli, locale, command)
                }
            },
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
    let report = terminate_bound_process(
        &plan,
        &command.preview_digest,
        command.confirm,
        &probe,
        &mut killer,
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
    let code = match error {
        ProcessActionError::Protected { .. } => "protected",
        ProcessActionError::IdentityMismatch | ProcessActionError::NotFound => "invalid_identity",
        ProcessActionError::PermissionDenied => "identity_unavailable",
        ProcessActionError::Unsupported => "unsupported_platform",
        ProcessActionError::ConfirmRequired => "confirmation_required",
        ProcessActionError::StaleDigest => "stale_confirmation",
        ProcessActionError::InvalidPlan => "invalid_plan",
    };
    from_domain_code(code, error.to_string())
}

fn run_keep_awake(
    cli: &Cli,
    locale: Locale,
    command: &StatusKeepAwakeRunCommand,
) -> Result<(), ApplicationError> {
    install_ctrl_c_handler();
    CTRL_C.store(false, Ordering::SeqCst);
    let service = Arc::new(session_service());
    let session = open_keep_awake_session(
        &service,
        &command.owner,
        Duration::from_millis(command.duration_ms),
        command.confirm,
    )?;
    emit_keep_awake(
        cli,
        locale,
        "status.keep-awake.run",
        &session.endpoint,
        &session.lease,
    )?;
    hold_keep_awake_session(session, &CTRL_C);
    Ok(())
}

fn run_keep_awake_query(
    cli: &Cli,
    locale: Locale,
    command: &StatusKeepAwakeTokenCommand,
    verb: &str,
) -> Result<(), ApplicationError> {
    let lease = query_keep_awake(&command.endpoint, verb, &command.token, &command.owner)?;
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

struct KeepAwakeSession {
    service: Arc<KeepAwakeService>,
    listener: TcpListener,
    endpoint: String,
    token: String,
    owner: String,
    lease: KeepAwakeStatusV1,
}

impl Drop for KeepAwakeSession {
    fn drop(&mut self) {
        let _ = self.service.owner_exit(&self.owner);
    }
}

fn open_keep_awake_session(
    service: &Arc<KeepAwakeService>,
    owner: &str,
    duration: Duration,
    confirm: bool,
) -> Result<KeepAwakeSession, ApplicationError> {
    if !service.is_supported() {
        return Err(map_keep_awake(KeepAwakeError::CapabilityGap {
            reason: "platform_unsupported",
        }));
    }
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| {
        ApplicationError::failed(
            "output_open_failed",
            format!("failed to bind keep-awake loopback listener: {error}"),
        )
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        ApplicationError::failed(
            "output_open_failed",
            format!("failed to set keep-awake listener nonblocking: {error}"),
        )
    })?;
    let port = listener
        .local_addr()
        .map_err(|error| {
            ApplicationError::failed(
                "output_open_failed",
                format!("failed to read keep-awake listener address: {error}"),
            )
        })?
        .port();
    let endpoint = format!("127.0.0.1:{port}");
    let lease = service
        .run(owner, duration, confirm)
        .map_err(map_keep_awake)?;
    Ok(KeepAwakeSession {
        service: Arc::clone(service),
        listener,
        endpoint,
        token: lease.token.clone(),
        owner: owner.to_string(),
        lease,
    })
}

fn hold_keep_awake_session(session: KeepAwakeSession, cancel: &AtomicBool) {
    loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = session.service.cancel(&session.token, &session.owner);
            break;
        }
        match session.service.status(&session.token, &session.owner) {
            Ok(status) if status.state == "active" => {}
            _ => break,
        }
        match session.listener.accept() {
            Ok((stream, address)) => {
                if address.ip().is_loopback()
                    && serve_keep_awake_client(&session, stream) == ClientEffect::Release
                {
                    break;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClientEffect {
    Continue,
    Release,
}

fn serve_keep_awake_client(session: &KeepAwakeSession, mut stream: TcpStream) -> ClientEffect {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(400)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(400)));
    let request = match read_session_line(&mut stream) {
        Ok(line) => line,
        Err(_) => return ClientEffect::Continue,
    };
    let Some((verb, token, owner)) = parse_session_line(&request) else {
        let _ = write_session_reply(&mut stream, false, None, Some("invalid_request"), None);
        return ClientEffect::Continue;
    };
    let result = match verb {
        "STATUS" => session.service.status(token, owner),
        "STOP" => session.service.stop(token, owner),
        "CANCEL" => session.service.cancel(token, owner),
        _ => {
            let _ = write_session_reply(&mut stream, false, None, Some("invalid_request"), None);
            return ClientEffect::Continue;
        }
    };
    match result {
        Ok(status) => {
            let release = verb != "STATUS" && status.state != "active";
            let _ = write_session_reply(&mut stream, true, Some(&status), None, None);
            if release {
                ClientEffect::Release
            } else {
                ClientEffect::Continue
            }
        }
        Err(error) => {
            let _ = write_session_reply(
                &mut stream,
                false,
                None,
                Some(keep_awake_code(&error)),
                Some(error.to_string()),
            );
            ClientEffect::Continue
        }
    }
}

fn query_keep_awake(
    endpoint: &str,
    verb: &str,
    token: &str,
    owner: &str,
) -> Result<KeepAwakeStatusV1, ApplicationError> {
    let address = parse_loopback_endpoint(endpoint)?;
    let mut stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(|error| {
            from_domain_code(
                "network_unavailable",
                format!("keep-awake endpoint unavailable: {error}"),
            )
        })?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    stream
        .write_all(format!("{verb} {token} {owner}\n").as_bytes())
        .map_err(|error| {
            ApplicationError::failed(
                "output_write_failed",
                format!("failed to write keep-awake request: {error}"),
            )
        })?;
    let line = read_session_line(&mut stream).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read keep-awake reply: {error}"),
        )
    })?;
    let reply: serde_json::Value = serde_json::from_str(&line)
        .map_err(|_| ApplicationError::usage("invalid_plan", "keep-awake reply is invalid"))?;
    if reply["ok"].as_bool() == Some(true) {
        let status = serde_json::from_value(reply["status"].clone()).map_err(|_| {
            ApplicationError::usage("invalid_plan", "keep-awake reply omitted the lease")
        })?;
        return Ok(status);
    }
    let code = reply["code"].as_str().unwrap_or("invalid_request");
    let message = reply["message"]
        .as_str()
        .unwrap_or("keep-awake request was refused")
        .to_string();
    Err(match code {
        "unknown_token" | "owner_mismatch" => from_domain_code("invalid_identity", message),
        "capability_gap" => from_domain_code("unsupported_platform", message),
        "confirmation_required" => from_domain_code("confirmation_required", message),
        _ => ApplicationError::usage("invalid_plan", message),
    })
}

fn parse_loopback_endpoint(endpoint: &str) -> Result<SocketAddr, ApplicationError> {
    let address: SocketAddr = endpoint.parse().map_err(|_| {
        ApplicationError::usage(
            "invalid_endpoint",
            "keep-awake endpoint must be a loopback socket address",
        )
    })?;
    if !address.ip().is_loopback() {
        return Err(ApplicationError::usage(
            "invalid_endpoint",
            "keep-awake endpoint must be a loopback socket address",
        ));
    }
    Ok(address)
}

fn parse_session_line(line: &str) -> Option<(&str, &str, &str)> {
    let mut parts = line.split_whitespace();
    let verb = parts.next()?;
    let token = parts.next()?;
    let owner = parts.next()?;
    if parts.next().is_some() || !matches!(verb, "STATUS" | "STOP" | "CANCEL") {
        return None;
    }
    Some((verb, token, owner))
}

fn read_session_line(stream: &mut TcpStream) -> io::Result<String> {
    let mut data = Vec::new();
    let mut buffer = [0_u8; 256];
    loop {
        if data.len() >= 512 || data.contains(&b'\n') {
            break;
        }
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => data.extend_from_slice(&buffer[..count]),
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    || error.kind() == io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(error) => return Err(error),
        }
    }
    let text = String::from_utf8_lossy(&data);
    Ok(text.trim().to_string())
}

fn write_session_reply(
    stream: &mut TcpStream,
    ok: bool,
    status: Option<&KeepAwakeStatusV1>,
    code: Option<&str>,
    message: Option<String>,
) -> io::Result<()> {
    let reply = serde_json::json!({
        "ok": ok,
        "status": status,
        "code": code,
        "message": message,
    });
    let mut bytes = serde_json::to_vec(&reply).unwrap_or_else(|_| b"{}".to_vec());
    bytes.push(b'\n');
    stream.write_all(&bytes)
}

fn keep_awake_code(error: &KeepAwakeError) -> &'static str {
    match error {
        KeepAwakeError::CapabilityGap { .. } => "capability_gap",
        KeepAwakeError::ConfirmRequired => "confirmation_required",
        KeepAwakeError::DurationNotFinite => "duration_not_finite",
        KeepAwakeError::InvalidOwner => "invalid_owner",
        KeepAwakeError::UnknownToken => "unknown_token",
        KeepAwakeError::OwnerMismatch => "owner_mismatch",
    }
}

fn map_keep_awake(error: KeepAwakeError) -> ApplicationError {
    let code = match error {
        KeepAwakeError::CapabilityGap { .. } => "unsupported_platform",
        KeepAwakeError::ConfirmRequired => "confirmation_required",
        KeepAwakeError::DurationNotFinite | KeepAwakeError::InvalidOwner => "invalid_plan",
        KeepAwakeError::UnknownToken | KeepAwakeError::OwnerMismatch => "invalid_identity",
    };
    from_domain_code(code, error.to_string())
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
        for code in ["vram", "fan", "smart", "physical_disk_activity"] {
            assert!(codes.contains(&code), "missing {code}");
        }
        for (group, code) in [("gpu", "gpu_utilization"), ("thermal", "thermal")] {
            let state = en_doc["data"][group]["state"].as_str().unwrap();
            let measured = state == "available" || state == "partial";
            assert_eq!(codes.contains(&code), !measured, "{group} {state}");
        }
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
        let clock = Arc::new(devsweep_core::status::FakeClock::new());
        let service = Arc::new(KeepAwakeService::new(
            Arc::clone(&clock) as Arc<dyn devsweep_core::status::KeepAwakeClock>,
            Box::new(backend),
            false,
        ));
        let error =
            match open_keep_awake_session(&service, "owner-a", Duration::from_millis(50), true) {
                Err(error) => error,
                Ok(_) => panic!("unsupported platform stored a lease"),
            };
        assert_eq!(error.code, "unsupported_platform");
        assert_eq!(error.exit, ExitClass::Unavailable);
        assert_eq!(service.stored_lease_count(), 0);
        assert!(log.lock().expect("log").is_empty());
    }

    #[test]
    fn keep_awake_lease_expiry_token_ownership_cancellation_and_owner_exit() {
        use devsweep_core::status::{BackendCall, FakeClock, KeepAwakeClock};

        let clock = Arc::new(FakeClock::new());
        let (backend, log) = RecordingBackend::pair();
        let service = Arc::new(KeepAwakeService::new(
            Arc::clone(&clock) as Arc<dyn KeepAwakeClock>,
            Box::new(backend),
            true,
        ));
        let refused =
            match open_keep_awake_session(&service, "owner-a", Duration::from_millis(0), true) {
                Err(error) => error,
                Ok(_) => panic!("zero duration stored a lease"),
            };
        assert_eq!(refused.code, "invalid_plan");
        assert_eq!(service.stored_lease_count(), 0);

        let session = open_keep_awake_session(&service, "owner-a", Duration::from_millis(50), true)
            .expect("lease");
        assert!(!session.lease.power_settings_changed);
        let exited_token = session.token.clone();
        drop(session);
        assert_eq!(
            service
                .status(&exited_token, "owner-a")
                .expect("owner exit")
                .state,
            "owner_exited"
        );

        let left = open_keep_awake_session(&service, "owner-a", Duration::from_secs(30), true)
            .expect("left");
        let right = open_keep_awake_session(&service, "owner-b", Duration::from_secs(30), true)
            .expect("right");
        let right_token = right.token.clone();
        let right_endpoint = right.endpoint.clone();
        let left_token = left.token.clone();
        let left_endpoint = left.endpoint.clone();
        let left_flag = Arc::new(AtomicBool::new(false));
        let right_flag = Arc::new(AtomicBool::new(false));
        let right_cancel = Arc::clone(&right_flag);
        let left_thread = thread::spawn(move || hold_keep_awake_session(left, &left_flag));
        let right_thread = thread::spawn(move || hold_keep_awake_session(right, &right_cancel));
        let wait_status = |endpoint: &str, token: &str, owner: &str| {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            let mut last = String::new();
            while std::time::Instant::now() < deadline {
                match query_keep_awake(endpoint, "STATUS", token, owner) {
                    Ok(status) => return status,
                    Err(error) if error.code == "network_unavailable" => {
                        last = error.message;
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(error) => panic!("{} {}", error.code, error.message),
                }
            }
            panic!("keep-awake status timed out: {last}");
        };

        let active = wait_status(&left_endpoint, &left_token, "owner-a");
        assert_eq!(active.state, "active");
        assert!(!active.power_settings_changed);
        let wrong = query_keep_awake(&left_endpoint, "STOP", &left_token, "owner-b");
        assert_eq!(wrong.expect_err("owner").code, "invalid_identity");
        assert_eq!(
            wait_status(&right_endpoint, &right_token, "owner-b").state,
            "active"
        );
        let stopped =
            query_keep_awake(&left_endpoint, "STOP", &left_token, "owner-a").expect("stop left");
        assert_eq!(stopped.state, "stopped");
        left_thread.join().expect("left hold");
        assert_eq!(
            service
                .status(&right_token, "owner-b")
                .expect("right after left stop")
                .state,
            "active"
        );
        let _ = query_keep_awake(&right_endpoint, "STOP", &right_token, "owner-b");
        right_flag.store(true, Ordering::SeqCst);
        right_thread.join().expect("right hold");

        let short = open_keep_awake_session(&service, "owner-d", Duration::from_millis(50), true)
            .expect("short lease");
        let short_token = short.token.clone();
        let short_endpoint = short.endpoint.clone();
        let short_flag = Arc::new(AtomicBool::new(false));
        let short_cancel = Arc::clone(&short_flag);
        let short_thread = thread::spawn(move || hold_keep_awake_session(short, &short_cancel));
        assert_eq!(
            wait_status(&short_endpoint, &short_token, "owner-d").state,
            "active"
        );
        clock.advance(Duration::from_millis(50));
        assert_eq!(
            service
                .status(&short_token, "owner-d")
                .expect("expiry")
                .state,
            "expired"
        );
        short_flag.store(true, Ordering::SeqCst);
        short_thread.join().expect("short hold");

        let fresh = open_keep_awake_session(&service, "owner-c", Duration::from_secs(30), true)
            .expect("cancel lease");
        let fresh_token = fresh.token.clone();
        let flag = Arc::new(AtomicBool::new(true));
        hold_keep_awake_session(fresh, &flag);
        assert_eq!(
            service
                .status(&fresh_token, "owner-c")
                .expect("canceled")
                .state,
            "canceled"
        );

        let releases = log
            .lock()
            .expect("log")
            .iter()
            .filter(|call| matches!(call, BackendCall::Release(_)))
            .count();
        assert!(releases >= 1);
        let endpoint_error = query_keep_awake("8.8.8.8:9", "STATUS", "ka-1-1", "owner-a");
        assert_eq!(
            endpoint_error.expect_err("endpoint").code,
            "invalid_endpoint"
        );
    }

    #[cfg(windows)]
    #[test]
    fn fixture_owned_process_is_the_only_native_termination_target() {
        use devsweep_core::status::ProcessProbe;
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
}
