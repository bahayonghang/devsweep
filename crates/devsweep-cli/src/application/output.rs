#![allow(dead_code)]

use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub(super) enum ExitClass {
    Success = 0,
    Usage = 2,
    InvalidAuthority = 3,
    Unavailable = 4,
    Partial = 5,
    Failed = 6,
    CanceledBeforeDispatch = 130,
}

impl ExitClass {
    pub(super) const fn code(self) -> i32 {
        self as i32
    }
}

#[derive(Debug)]
pub(super) struct ApplicationError {
    pub(super) exit: ExitClass,
    pub(super) code: &'static str,
    pub(super) message: String,
}

impl ApplicationError {
    pub(super) fn new(exit: ExitClass, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            exit,
            code,
            message: message.into(),
        }
    }

    pub(super) fn usage(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ExitClass::Usage, code, message)
    }

    pub(super) fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ExitClass::Unavailable, "mode_unavailable", message)
    }

    pub(super) fn failed(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(ExitClass::Failed, code, message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResultFormat {
    Human,
    Json,
    Ndjson,
    PlanJson,
}

#[derive(Debug)]
pub(super) enum OutputSink {
    Stdout(io::Stdout),
    CreateNew { file: std::fs::File, path: PathBuf },
}

impl OutputSink {
    pub(super) fn open(path: Option<&Path>) -> Result<Self, ApplicationError> {
        match path {
            None => Ok(Self::Stdout(io::stdout())),
            Some(path) => {
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .map_err(|error| {
                        if error.kind() == io::ErrorKind::AlreadyExists {
                            ApplicationError::failed(
                                "output_exists",
                                format!("output already exists: {}", path.display()),
                            )
                        } else {
                            ApplicationError::failed(
                                "output_open_failed",
                                format!("failed to create output {}: {error}", path.display()),
                            )
                        }
                    })?;
                Ok(Self::CreateNew {
                    file,
                    path: path.to_path_buf(),
                })
            }
        }
    }

    pub(super) fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        match self {
            Self::Stdout(stdout) => {
                let mut lock = stdout.lock();
                lock.write_all(bytes)?;
                lock.flush()
            }
            Self::CreateNew { file, .. } => {
                file.write_all(bytes)?;
                file.flush()?;
                file.sync_all()
            }
        }
    }

    pub(super) fn path(&self) -> Option<&Path> {
        match self {
            Self::Stdout(_) => None,
            Self::CreateNew { path, .. } => Some(path),
        }
    }
}

pub(super) fn error_envelope(command: &str, error: &ApplicationError) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "command": command,
        "outcome": "failed",
        "data": null,
        "warnings": [],
        "error": {
            "code": error.code,
            "message": null
        }
    })
}

pub(super) fn write_json(
    sink: &mut OutputSink,
    document: &serde_json::Value,
) -> Result<(), ApplicationError> {
    let mut bytes = serde_json::to_vec(document).map_err(|error| {
        ApplicationError::failed(
            "output_serialize_failed",
            format!("failed to serialize machine output: {error}"),
        )
    })?;
    bytes.push(b'\n');
    sink.write_all(&bytes).map_err(classify_write_error)
}

fn classify_write_error(error: io::Error) -> ApplicationError {
    if error.kind() == io::ErrorKind::BrokenPipe {
        ApplicationError::new(ExitClass::Success, "broken_pipe", "output pipe closed")
    } else {
        ApplicationError::failed(
            "output_write_failed",
            format!("failed to write output: {error}"),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StreamTerminal {
    pub(super) operation_id: String,
    pub(super) sequence: u64,
    pub(super) reason: &'static str,
    pub(super) error_code: Option<&'static str>,
    pub(super) producer_joined: bool,
}

pub(super) trait StreamProducerControl {
    fn cancel(&mut self);
    fn join(&mut self);
}

#[derive(Debug)]
pub(super) struct StreamWriteFailure {
    pub(super) error: Box<ApplicationError>,
    pub(super) terminal: StreamTerminal,
    pub(super) terminal_serialized: bool,
}

pub(super) fn write_ndjson_with_control<I, C>(
    writer: &mut dyn Write,
    events: I,
    operation_id: &str,
    mut next_sequence: u64,
    control: &mut C,
) -> Result<StreamTerminal, StreamWriteFailure>
where
    I: IntoIterator<Item = serde_json::Value>,
    C: StreamProducerControl,
{
    for event in events {
        let mut line = match serde_json::to_vec(&event) {
            Ok(line) => line,
            Err(error) => {
                return producer_failure(
                    writer,
                    operation_id,
                    next_sequence,
                    control,
                    ApplicationError::failed(
                        "output_serialize_failed",
                        format!("failed to serialize stream event: {error}"),
                    ),
                );
            }
        };
        line.push(b'\n');
        if let Err(error) = writer.write_all(&line) {
            control.cancel();
            control.join();
            if error.kind() == io::ErrorKind::BrokenPipe {
                return Ok(StreamTerminal {
                    operation_id: operation_id.to_string(),
                    sequence: next_sequence,
                    reason: "broken_pipe",
                    error_code: None,
                    producer_joined: true,
                });
            }
            return producer_failure_after_join(
                writer,
                operation_id,
                next_sequence,
                ApplicationError::failed(
                    "output_write_failed",
                    format!("failed to write stream event: {error}"),
                ),
            );
        }
        next_sequence += 1;
    }

    let terminal = terminal_event(operation_id, next_sequence, "completed", None);
    let mut line = serde_json::to_vec(&terminal).expect("terminal event is always serializable");
    line.push(b'\n');
    if let Err(error) = writer.write_all(&line) {
        control.cancel();
        control.join();
        if error.kind() == io::ErrorKind::BrokenPipe {
            return Ok(StreamTerminal {
                operation_id: operation_id.to_string(),
                sequence: next_sequence,
                reason: "broken_pipe",
                error_code: None,
                producer_joined: true,
            });
        }
        return producer_failure_after_join(
            writer,
            operation_id,
            next_sequence,
            ApplicationError::failed(
                "output_write_failed",
                format!("failed to write terminal event: {error}"),
            ),
        );
    }
    control.join();
    Ok(StreamTerminal {
        operation_id: operation_id.to_string(),
        sequence: next_sequence,
        reason: "completed",
        error_code: None,
        producer_joined: true,
    })
}

fn producer_failure<C: StreamProducerControl>(
    writer: &mut dyn Write,
    operation_id: &str,
    sequence: u64,
    control: &mut C,
    error: ApplicationError,
) -> Result<StreamTerminal, StreamWriteFailure> {
    control.cancel();
    control.join();
    producer_failure_after_join(writer, operation_id, sequence, error)
}

fn producer_failure_after_join(
    writer: &mut dyn Write,
    operation_id: &str,
    sequence: u64,
    error: ApplicationError,
) -> Result<StreamTerminal, StreamWriteFailure> {
    let terminal = StreamTerminal {
        operation_id: operation_id.to_string(),
        sequence,
        reason: "producer_error",
        error_code: Some(error.code),
        producer_joined: true,
    };
    let event = terminal_event(operation_id, sequence, terminal.reason, terminal.error_code);
    let mut line = serde_json::to_vec(&event).expect("terminal event is always serializable");
    line.push(b'\n');
    let terminal_serialized = writer.write_all(&line).is_ok();
    Err(StreamWriteFailure {
        error: Box::new(error),
        terminal,
        terminal_serialized,
    })
}

fn terminal_event(
    operation_id: &str,
    sequence: u64,
    reason: &str,
    error_code: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "event": "status_terminal",
        "operation_id": operation_id,
        "sequence": sequence,
        "emitted_at_unix_ms": 0,
        "data": { "reason": reason, "error_code": error_code }
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, io};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn output_files_use_exclusive_create_new_without_truncation() {
        let directory = TempDir::new().expect("temporary output directory");
        let path = directory.path().join("result.json");
        fs::write(&path, b"keep me").expect("existing output fixture");

        let error = OutputSink::open(Some(&path)).expect_err("existing output rejects");
        assert_eq!(error.exit, ExitClass::Failed);
        assert_eq!(error.code, "output_exists");
        assert_eq!(fs::read(&path).unwrap(), b"keep me");
    }

    struct BrokenPipeAfterOne {
        writes: usize,
        bytes: Vec<u8>,
    }

    impl Write for BrokenPipeAfterOne {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.writes == 1 {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
            }
            self.writes += 1;
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestControl {
        canceled: bool,
        joined: bool,
    }

    impl StreamProducerControl for TestControl {
        fn cancel(&mut self) {
            self.canceled = true;
        }

        fn join(&mut self) {
            self.joined = true;
        }
    }

    #[test]
    fn broken_pipe_cancels_and_joins_without_another_write() {
        let mut writer = BrokenPipeAfterOne {
            writes: 0,
            bytes: Vec::new(),
        };
        let mut control = TestControl::default();
        let terminal = write_ndjson_with_control(
            &mut writer,
            [
                serde_json::json!({"schema_version":1,"event":"status_started","operation_id":"op","sequence":0}),
                serde_json::json!({"schema_version":1,"event":"status_snapshot","operation_id":"op","sequence":1}),
            ],
            "op",
            0,
            &mut control,
        )
        .expect("broken pipe is graceful");

        assert_eq!(writer.writes, 1);
        assert!(
            String::from_utf8(writer.bytes)
                .unwrap()
                .contains("status_started")
        );
        assert_eq!(terminal.reason, "broken_pipe");
        assert!(terminal.producer_joined);
        assert!(control.canceled);
        assert!(control.joined);
    }

    struct FailOnceThenRecover {
        writes: usize,
        bytes: Vec<u8>,
    }

    impl Write for FailOnceThenRecover {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            if self.writes == 2 {
                return Err(io::Error::other("fixture write failure"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn non_broken_write_failure_records_producer_error_and_joins() {
        let mut writer = FailOnceThenRecover {
            writes: 0,
            bytes: Vec::new(),
        };
        let mut control = TestControl::default();
        let failure = write_ndjson_with_control(
            &mut writer,
            [
                serde_json::json!({"schema_version":1,"event":"status_started"}),
                serde_json::json!({"schema_version":1,"event":"status_snapshot"}),
            ],
            "op",
            0,
            &mut control,
        )
        .expect_err("non-broken write errors fail");

        assert_eq!(failure.error.exit, ExitClass::Failed);
        assert_eq!(failure.error.code, "output_write_failed");
        assert_eq!(failure.terminal.reason, "producer_error");
        assert_eq!(failure.terminal.error_code, Some("output_write_failed"));
        assert!(failure.terminal.producer_joined);
        assert!(failure.terminal_serialized);
        assert!(control.canceled);
        assert!(control.joined);
        let serialized = String::from_utf8(writer.bytes).unwrap();
        assert!(serialized.contains("status_started"));
        assert!(serialized.contains("producer_error"));
        assert!(!serialized.contains("status_snapshot"));
    }

    #[cfg(windows)]
    #[test]
    fn closed_os_pipe_is_classified_as_graceful_broken_pipe() {
        use std::process::{Command, Stdio};

        let mut child = Command::new("cmd.exe")
            .args(["/D", "/C", "exit", "0"])
            .stdin(Stdio::piped())
            .spawn()
            .expect("system pipe child starts");
        let mut pipe = child.stdin.take().expect("child stdin pipe");
        assert!(child.wait().expect("system pipe child exits").success());

        let mut control = TestControl::default();
        let terminal = write_ndjson_with_control(
            &mut pipe,
            [serde_json::json!({"schema_version":1,"event":"status_started"})],
            "os-pipe",
            0,
            &mut control,
        )
        .expect("closed system pipe is graceful");

        assert_eq!(terminal.reason, "broken_pipe");
        assert!(control.canceled);
        assert!(control.joined);
    }

    #[test]
    fn machine_error_envelope_has_stable_locale_neutral_fields() {
        let error = ApplicationError::unavailable("localized display text");
        let envelope = error_envelope("status.snapshot", &error);
        assert_eq!(envelope["schema_version"], 1);
        assert_eq!(envelope["command"], "status.snapshot");
        assert_eq!(envelope["error"]["code"], "mode_unavailable");
        assert!(envelope["error"]["message"].is_null());
    }
}
