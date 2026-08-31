//! Frozen-grammar Status snapshot and live handlers.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use devsweep_core::status::{
    LiveRequest, StatusError, StatusEventV1, StatusSnapshotV1, TerminalReason, capture_snapshot,
    spawn_live,
};

use super::super::{
    cli::{Cli, Command, StatusCommand, StatusLiveCommand, StatusSnapshotCommand},
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, write_json},
    presentation,
};
use crate::i18n::{Locale, catalogue};

#[cfg(test)]
pub(super) struct Route;

static CTRL_C: AtomicBool = AtomicBool::new(false);

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Status(group)) => match &group.command {
            StatusCommand::Snapshot(command) => run_snapshot(cli, locale, command),
            StatusCommand::Live(command) => run_live(cli, locale, command),
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
        assert_eq!(
            en_doc["data"]["unsupported_capabilities"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
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
}
