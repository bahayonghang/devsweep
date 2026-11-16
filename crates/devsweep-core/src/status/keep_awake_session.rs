//! Explicit loopback transport shared by the CLI and in-process desktop owners.
//! The endpoint has no discovery or persistent receipt; unreachable means unknown.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::keep_awake::{
    KeepAwakeAction, KeepAwakeError, KeepAwakePlan, KeepAwakePreviewV1, KeepAwakeService,
    KeepAwakeStatusV1, MAX_KEEP_AWAKE_MS,
};

/// Existing finite-session protocol verbs. Starting a lease is never a network verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAwakeVerb {
    Status,
    Stop,
    Cancel,
}

impl KeepAwakeVerb {
    fn as_str(self) -> &'static str {
        match self {
            Self::Status => "STATUS",
            Self::Stop => "STOP",
            Self::Cancel => "CANCEL",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "STATUS" => Some(Self::Status),
            "STOP" => Some(Self::Stop),
            "CANCEL" => Some(Self::Cancel),
            _ => None,
        }
    }
}

/// Owns the endpoint and a joined service worker. Stop/expiry retains the endpoint
/// until shutdown. Callers can report explicit shutdown errors before dropping it.
pub struct KeepAwakeSession {
    service: Arc<KeepAwakeService>,
    endpoint: String,
    token: String,
    owner: String,
    closing: Arc<AtomicBool>,
    transport: Option<JoinHandle<()>>,
}

impl KeepAwakeSession {
    pub fn open(
        service: KeepAwakeService,
        owner: &str,
        duration: Duration,
        confirm: bool,
    ) -> Result<Self, KeepAwakeError> {
        if !service.is_supported() {
            return Err(KeepAwakeError::CapabilityGap {
                reason: "platform_unsupported",
            });
        }
        if !confirm {
            return Err(KeepAwakeError::ConfirmRequired);
        }
        let plan = service.plan_start(owner, duration)?;
        let preview = service.preview(&plan)?;
        Self::start(service, &plan, &preview.digest, confirm)
    }

    /// Desktop start requires the digest returned by the explicit live review.
    pub fn start(
        service: KeepAwakeService,
        plan: &KeepAwakePlan,
        expected_digest: &str,
        confirmed: bool,
    ) -> Result<Self, KeepAwakeError> {
        if !confirmed {
            return Err(KeepAwakeError::ConfirmRequired);
        }
        let listener =
            TcpListener::bind("127.0.0.1:0").map_err(|error| session_io("bind", error))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| session_io("configure", error))?;
        let endpoint = listener
            .local_addr()
            .map_err(|error| session_io("address", error))?
            .to_string();
        let lease = service.execute_start(plan, expected_digest, confirmed)?;
        let service = Arc::new(service);
        let closing = Arc::new(AtomicBool::new(false));
        let worker_service = Arc::clone(&service);
        let worker_closing = Arc::clone(&closing);
        let transport = thread::Builder::new()
            .name("keep-awake-endpoint".into())
            .spawn(move || {
                while !worker_closing.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, address)) if address.ip().is_loopback() => {
                            serve_client(&worker_service, stream);
                        }
                        Ok(_) => {}
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(20));
                        }
                        Err(error) => {
                            tracing::error!(%error, "keep-awake endpoint accept failed");
                            break;
                        }
                    }
                }
            })
            .map_err(|error| session_io("spawn", error))?;
        Ok(Self {
            service,
            endpoint,
            token: lease.token,
            owner: lease.owner,
            closing,
            transport: Some(transport),
        })
    }

    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn status(&self) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.service.status(&self.token, &self.owner)
    }

    pub fn stop(&self) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.service.stop(&self.token, &self.owner)
    }

    pub fn cancel(&self) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.service.cancel(&self.token, &self.owner)
    }

    pub fn plan_start(&self, duration: Duration) -> Result<KeepAwakePlan, KeepAwakeError> {
        self.service.plan_existing(
            &self.token,
            &self.owner,
            KeepAwakeAction::Start,
            Some(duration),
        )
    }

    pub fn plan_stop(&self) -> Result<KeepAwakePlan, KeepAwakeError> {
        self.service
            .plan_existing(&self.token, &self.owner, KeepAwakeAction::Stop, None)
    }

    pub fn preview(&self, plan: &KeepAwakePlan) -> Result<KeepAwakePreviewV1, KeepAwakeError> {
        self.service.preview(plan)
    }

    pub fn execute_start(
        &self,
        plan: &KeepAwakePlan,
        expected_digest: &str,
        confirmed: bool,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.service.execute_start(plan, expected_digest, confirmed)
    }

    pub fn execute_stop(
        &self,
        plan: &KeepAwakePlan,
        expected_digest: &str,
        confirmed: bool,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.service.execute_stop(plan, expected_digest, confirmed)
    }

    /// Closes the endpoint, clears on the owner thread, and joins both workers.
    /// Even a failed clear joins; that failure never reports a terminal/off lease.
    pub fn shutdown(&mut self) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.closing.store(true, Ordering::SeqCst);
        let transport = self.transport.take().map(|worker| worker.join());
        let terminal = self
            .service
            .owner_exit(&self.owner)
            .and_then(|_| self.status());
        let closed = self.service.shutdown();
        if transport.is_some_and(|result| result.is_err()) {
            return Err(KeepAwakeError::WorkerUnavailable);
        }
        let status = terminal?;
        closed?;
        Ok(status)
    }
}

impl Drop for KeepAwakeSession {
    fn drop(&mut self) {
        if self.transport.is_some()
            && let Err(error) = self.shutdown()
        {
            tracing::error!(%error, "keep-awake session shutdown failed; release is unconfirmed");
        }
    }
}

/// Queries only the explicit session tuple. Connection/read failures are unknown,
/// never evidence of an off state. The wire format matches the original CLI.
pub fn query_keep_awake(
    endpoint: &str,
    verb: KeepAwakeVerb,
    token: &str,
    owner: &str,
) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
    let address: SocketAddr = endpoint
        .parse()
        .map_err(|_| KeepAwakeError::InvalidEndpoint)?;
    if !address.ip().is_loopback() {
        return Err(KeepAwakeError::InvalidEndpoint);
    }
    if !valid_part(token) || !valid_part(owner) {
        return Err(KeepAwakeError::InvalidRequest);
    }
    let mut stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(network)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(network)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(network)?;
    stream
        .write_all(format!("{} {token} {owner}\n", verb.as_str()).as_bytes())
        .map_err(network)?;
    let line = read_line(&mut stream, Duration::from_secs(2)).map_err(network)?;
    let reply: Reply = serde_json::from_str(&line).map_err(|_| KeepAwakeError::InvalidReply)?;
    if reply.ok {
        let status = reply.status.ok_or(KeepAwakeError::InvalidReply)?;
        if status.token != token
            || status.owner != owner
            || status.power_settings_changed
            || status.duration_ms == 0
            || status.duration_ms > MAX_KEEP_AWAKE_MS
            || status.remaining_ms > status.duration_ms
            || !matches!(
                status.state.as_str(),
                "active" | "stopped" | "expired" | "canceled" | "owner_exited"
            )
            || (status.state != "active" && status.remaining_ms != 0)
        {
            return Err(KeepAwakeError::InvalidReply);
        }
        return Ok(status);
    }
    Err(match reply.code.as_deref() {
        Some("unknown_token") => KeepAwakeError::UnknownToken,
        Some("owner_mismatch") => KeepAwakeError::OwnerMismatch,
        Some("capability_gap") => KeepAwakeError::CapabilityGap {
            reason: "platform_unsupported",
        },
        Some("confirmation_required") => KeepAwakeError::ConfirmRequired,
        Some("backend_failed") => KeepAwakeError::BackendFailed {
            operation: "remote request",
            message: reply.message.unwrap_or_default(),
        },
        Some("worker_unavailable") => KeepAwakeError::WorkerUnavailable,
        Some("audit_unavailable") => KeepAwakeError::AuditFailed {
            after_dispatch: false,
            message: reply.message.unwrap_or_default(),
        },
        Some("unknown_after_dispatch") => KeepAwakeError::AuditFailed {
            after_dispatch: true,
            message: reply.message.unwrap_or_default(),
        },
        Some("stale_confirmation") => KeepAwakeError::StaleConfirmation,
        _ => KeepAwakeError::InvalidRequest,
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Reply {
    ok: bool,
    status: Option<KeepAwakeStatusV1>,
    code: Option<String>,
    message: Option<String>,
}

fn serve_client(service: &KeepAwakeService, mut stream: TcpStream) {
    let result = (|| {
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_millis(400)))?;
        stream.set_write_timeout(Some(Duration::from_millis(400)))?;
        let line = read_line(&mut stream, Duration::from_millis(400))?;
        let result = parse_line(&line)
            .ok_or(KeepAwakeError::InvalidRequest)
            .and_then(|(verb, token, owner)| match verb {
                KeepAwakeVerb::Status => service.status(token, owner),
                KeepAwakeVerb::Stop => service.stop(token, owner),
                KeepAwakeVerb::Cancel => service.cancel(token, owner),
            });
        let reply = match result {
            Ok(status) => Reply {
                ok: true,
                status: Some(status),
                code: None,
                message: None,
            },
            Err(error) => Reply {
                ok: false,
                status: None,
                code: Some(wire_code(&error).into()),
                message: Some(error.to_string()),
            },
        };
        let mut bytes = serde_json::to_vec(&reply)?;
        bytes.push(b'\n');
        stream.write_all(&bytes)
    })();
    // A disconnected/malformed client does not end or clear its owner's session.
    let _: io::Result<()> = result;
}

fn parse_line(line: &str) -> Option<(KeepAwakeVerb, &str, &str)> {
    let mut parts = line.split_whitespace();
    let verb = KeepAwakeVerb::parse(parts.next()?)?;
    let token = parts.next()?;
    let owner = parts.next()?;
    if parts.next().is_some() || !valid_part(token) || !valid_part(owner) {
        return None;
    }
    Some((verb, token, owner))
}

fn valid_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn read_line(stream: &mut TcpStream, timeout: Duration) -> io::Result<String> {
    let deadline = Instant::now() + timeout;
    let mut data = Vec::new();
    let mut buffer = [0_u8; 256];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        stream.set_read_timeout(Some(remaining))?;
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        data.extend_from_slice(&buffer[..count]);
        if data.len() > 2048 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if let Some(newline) = data.iter().position(|byte| *byte == b'\n') {
            if newline + 1 != data.len() {
                return Err(io::ErrorKind::InvalidData.into());
            }
            data.pop();
            return String::from_utf8(data).map_err(|_| io::ErrorKind::InvalidData.into());
        }
    }
}

fn wire_code(error: &KeepAwakeError) -> &'static str {
    match error {
        KeepAwakeError::CapabilityGap { .. } => "capability_gap",
        KeepAwakeError::ConfirmRequired => "confirmation_required",
        KeepAwakeError::DurationNotFinite => "duration_not_finite",
        KeepAwakeError::InvalidOwner => "invalid_owner",
        KeepAwakeError::UnknownToken => "unknown_token",
        KeepAwakeError::OwnerMismatch => "owner_mismatch",
        KeepAwakeError::BackendFailed { .. } => "backend_failed",
        KeepAwakeError::WorkerUnavailable => "worker_unavailable",
        KeepAwakeError::AuditFailed {
            after_dispatch: false,
            ..
        } => "audit_unavailable",
        KeepAwakeError::AuditFailed {
            after_dispatch: true,
            ..
        } => "unknown_after_dispatch",
        KeepAwakeError::StaleConfirmation => "stale_confirmation",
        _ => "invalid_request",
    }
}

fn session_io(operation: &'static str, error: io::Error) -> KeepAwakeError {
    KeepAwakeError::SessionIo {
        operation,
        message: error.to_string(),
    }
}

fn network(error: io::Error) -> KeepAwakeError {
    KeepAwakeError::NetworkUnavailable {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{AwakeBackend, FakeClock};
    use std::sync::Mutex;
    use std::thread::ThreadId;

    type ThreadCall = (&'static str, String, ThreadId);

    #[derive(Clone, Default)]
    struct Control {
        calls: Arc<Mutex<Vec<ThreadCall>>>,
        fail_acquire: Arc<AtomicBool>,
        fail_release: Arc<AtomicBool>,
    }

    impl AwakeBackend for Control {
        fn acquire(&mut self, token: &str) -> Result<(), KeepAwakeError> {
            self.record("acquire", token, &self.fail_acquire)
        }

        fn release(&mut self, token: &str) -> Result<(), KeepAwakeError> {
            self.record("release", token, &self.fail_release)
        }
    }

    impl Control {
        fn record(
            &self,
            operation: &'static str,
            token: &str,
            fail: &AtomicBool,
        ) -> Result<(), KeepAwakeError> {
            self.calls.lock().expect("calls").push((
                operation,
                token.into(),
                thread::current().id(),
            ));
            if fail.load(Ordering::SeqCst) {
                return Err(KeepAwakeError::BackendFailed {
                    operation,
                    message: "fixture failure".into(),
                });
            }
            Ok(())
        }

        fn service(&self, clock: &Arc<FakeClock>) -> KeepAwakeService {
            KeepAwakeService::new(clock.clone(), Box::new(self.clone()), true)
        }

        fn assert_one_worker(&self) {
            let calls = self.calls.lock().expect("calls");
            let owner = calls.first().expect("backend called").2;
            assert_ne!(owner, thread::current().id());
            assert!(
                calls.iter().all(|call| call.2 == owner),
                "cross-thread backend calls: {calls:?}"
            );
        }
    }

    fn open(control: &Control, clock: &Arc<FakeClock>, owner: &str) -> KeepAwakeSession {
        KeepAwakeSession::open(control.service(clock), owner, Duration::from_secs(30), true)
            .expect("fake session")
    }

    fn query(
        session: &KeepAwakeSession,
        verb: KeepAwakeVerb,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        query_keep_awake(session.endpoint(), verb, session.token(), session.owner())
    }

    #[test]
    fn keep_awake_transport_deadline_is_not_extended_by_partial_bytes() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("isolated listener");
        let address = listener.local_addr().expect("address");
        let feeder = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).expect("connect");
            for _ in 0..50 {
                if stream.write_all(b"x").is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        let (mut stream, _) = listener.accept().expect("accept");
        let started = Instant::now();
        let error = read_line(&mut stream, Duration::from_millis(100))
            .expect_err("partial bytes cannot renew deadline");
        let elapsed = started.elapsed();
        drop(stream);
        feeder.join().expect("joined feeder");
        assert!(matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ));
        assert!(
            elapsed < Duration::from_millis(750),
            "read exceeded deadline: {elapsed:?}"
        );
    }

    #[test]
    fn keep_awake_busy_queries_do_not_starve_expiry_or_retry_failed_clear() {
        let control = Control::default();
        let clock = Arc::new(FakeClock::new());
        let service = Arc::new(control.service(&clock));
        let lease = service
            .run("busy", Duration::from_millis(5), true)
            .expect("lease");
        control.fail_release.store(true, Ordering::SeqCst);
        let querying = Arc::new(AtomicBool::new(true));
        let callers = (0..4)
            .map(|_| {
                let service = Arc::clone(&service);
                let querying = Arc::clone(&querying);
                let token = lease.token.clone();
                thread::spawn(move || {
                    while querying.load(Ordering::SeqCst) {
                        let _ = service.status(&token, "busy");
                    }
                })
            })
            .collect::<Vec<_>>();
        clock.advance(Duration::from_millis(5));
        let results = (0..100)
            .map(|_| service.status(&lease.token, "busy"))
            .collect::<Vec<_>>();
        querying.store(false, Ordering::SeqCst);
        for caller in callers {
            caller.join().expect("joined caller");
        }
        assert!(
            results
                .iter()
                .all(|result| matches!(result, Err(KeepAwakeError::BackendFailed { .. })))
        );
        let releases = || {
            control
                .calls
                .lock()
                .expect("calls")
                .iter()
                .filter(|call| call.0 == "release")
                .count()
        };
        assert_eq!(releases(), 1, "failed expiry must not retry on every query");
        control.fail_release.store(false, Ordering::SeqCst);
        assert_eq!(
            service
                .stop(&lease.token, "busy")
                .expect("explicit retry")
                .state,
            "stopped"
        );
        assert_eq!(releases(), 2);
        service.shutdown().expect("joined owner");
        control.assert_one_worker();
    }

    #[test]
    fn keep_awake_backend_is_dropped_on_the_thread_that_acquired_and_cleared() {
        struct DropBackend(Arc<Mutex<Vec<(&'static str, ThreadId)>>>);
        impl AwakeBackend for DropBackend {
            fn acquire(&mut self, _: &str) -> Result<(), KeepAwakeError> {
                self.0
                    .lock()
                    .expect("calls")
                    .push(("acquire", thread::current().id()));
                Ok(())
            }
            fn release(&mut self, _: &str) -> Result<(), KeepAwakeError> {
                self.0
                    .lock()
                    .expect("calls")
                    .push(("release", thread::current().id()));
                Ok(())
            }
        }
        impl Drop for DropBackend {
            fn drop(&mut self) {
                self.0
                    .lock()
                    .expect("calls")
                    .push(("drop", thread::current().id()));
            }
        }
        let calls = Arc::new(Mutex::new(Vec::new()));
        let service = KeepAwakeService::new(
            Arc::new(FakeClock::new()),
            Box::new(DropBackend(Arc::clone(&calls))),
            true,
        );
        let session = KeepAwakeSession::open(service, "drop", Duration::from_secs(30), true)
            .expect("session");
        drop(session);
        let calls = calls.lock().expect("calls");
        assert_eq!(
            calls.iter().map(|call| call.0).collect::<Vec<_>>(),
            ["acquire", "release", "drop"]
        );
        assert_ne!(calls[0].1, thread::current().id());
        assert!(calls.iter().all(|call| call.1 == calls[0].1));
    }

    #[test]
    fn keep_awake_service_and_session_are_send_sync_without_native_backend_calls() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<KeepAwakeService>();
        send_sync::<KeepAwakeSession>();
        let control = Control::default();
        let clock = Arc::new(FakeClock::new());
        for (duration, confirm, expected) in [
            (
                Duration::from_secs(1),
                false,
                KeepAwakeError::ConfirmRequired,
            ),
            (Duration::ZERO, true, KeepAwakeError::DurationNotFinite),
            (
                Duration::from_millis(MAX_KEEP_AWAKE_MS + 1),
                true,
                KeepAwakeError::DurationNotFinite,
            ),
        ] {
            let result =
                KeepAwakeSession::open(control.service(&clock), "gui-owner", duration, confirm);
            assert!(matches!(result, Err(ref error) if *error == expected));
        }
        let service = control.service(&clock);
        assert_eq!(
            service.run("owner with spaces", Duration::from_secs(1), true),
            Err(KeepAwakeError::InvalidOwner)
        );
        assert!(control.calls.lock().expect("calls").is_empty());
    }

    #[test]
    fn keep_awake_all_backend_calls_use_one_owner_thread_across_callers_and_expiry() {
        let control = Control::default();
        let clock = Arc::new(FakeClock::new());
        let service = Arc::new(control.service(&clock));
        let left = service
            .run("left", Duration::from_secs(1), true)
            .expect("left");
        let right = service
            .run("right", Duration::from_secs(30), true)
            .expect("right");
        let independent_plan = service
            .plan_start("future", Duration::from_secs(1))
            .expect("independent plan");
        let independent_preview = service
            .preview(&independent_plan)
            .expect("independent preview");
        let other = Arc::clone(&service);
        let token = left.token.clone();
        thread::spawn(move || other.stop(&token, "left"))
            .join()
            .expect("caller joined")
            .expect("stopped");
        assert_eq!(
            service
                .preview(&independent_plan)
                .expect("unrelated owner is not stale")
                .digest,
            independent_preview.digest
        );
        assert_eq!(
            service
                .status(&right.token, "right")
                .expect("unaffected")
                .state,
            "active"
        );
        let short = service
            .run("expiry", Duration::from_millis(10), true)
            .expect("short");
        clock.advance(Duration::from_millis(10));
        assert_eq!(
            service
                .status(&short.token, "expiry")
                .expect("expiry")
                .state,
            "expired"
        );
        service.shutdown().expect("joined release");
        control.assert_one_worker();
        let calls = control.calls.lock().expect("calls");
        assert!(
            calls
                .iter()
                .any(|call| call.0 == "release" && call.1 == right.token)
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.0 == "release" && call.1 == left.token)
                .count(),
            1
        );
    }

    #[test]
    fn keep_awake_stopped_gui_endpoint_is_queryable_until_shutdown_and_other_owners_survive() {
        let clock = Arc::new(FakeClock::new());
        let control = Control::default();
        let independent = Control::default();
        let mut left = open(&control, &clock, "gui-left");
        let mut right = open(&independent, &clock, "gui-right");
        let tuple = (left.endpoint().to_string(), left.token().to_string());
        assert_eq!(
            query(&left, KeepAwakeVerb::Status).expect("active").state,
            "active"
        );
        assert_eq!(
            query_keep_awake(left.endpoint(), KeepAwakeVerb::Stop, left.token(), "wrong"),
            Err(KeepAwakeError::OwnerMismatch)
        );
        assert_eq!(
            query(&left, KeepAwakeVerb::Stop).expect("cleared").state,
            "stopped"
        );
        assert_eq!(
            query(&left, KeepAwakeVerb::Status)
                .expect("retained endpoint")
                .state,
            "stopped"
        );
        assert_eq!(
            query(&right, KeepAwakeVerb::Status)
                .expect("other owner")
                .state,
            "active"
        );
        assert_eq!(left.shutdown().expect("shutdown stopped").state, "stopped");
        assert!(matches!(
            query_keep_awake(&tuple.0, KeepAwakeVerb::Status, &tuple.1, "gui-left"),
            Err(KeepAwakeError::NetworkUnavailable { .. })
        ));
        assert_eq!(
            query(&right, KeepAwakeVerb::Status)
                .expect("other owner after exit")
                .state,
            "active"
        );
        right.shutdown().expect("clear right");
        control.assert_one_worker();
        independent.assert_one_worker();
        assert_ne!(
            control.calls.lock().expect("left")[0].2,
            independent.calls.lock().expect("right")[0].2
        );
    }

    #[test]
    fn keep_awake_expiry_and_drop_clear_on_owner_thread_without_status_polling() {
        let clock = Arc::new(FakeClock::new());
        let control = Control::default();
        let session = KeepAwakeSession::open(
            control.service(&clock),
            "gui",
            Duration::from_millis(5),
            true,
        )
        .expect("short");
        clock.advance(Duration::from_millis(5));
        let deadline = Instant::now() + Duration::from_secs(3);
        while !control
            .calls
            .lock()
            .expect("calls")
            .iter()
            .any(|call| call.0 == "release")
        {
            assert!(
                Instant::now() < deadline,
                "worker did not expire independently"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            query(&session, KeepAwakeVerb::Status)
                .expect("expired retained")
                .state,
            "expired"
        );
        drop(session);
        let calls_after_drop = control.calls.lock().expect("calls").len();
        thread::sleep(Duration::from_millis(30));
        assert_eq!(
            control.calls.lock().expect("calls").len(),
            calls_after_drop,
            "worker not joined"
        );
        control.assert_one_worker();

        let active = open(&control, &clock, "active-drop");
        let token = active.token().to_string();
        drop(active);
        assert!(
            control
                .calls
                .lock()
                .expect("calls")
                .iter()
                .any(|call| call.0 == "release" && call.1 == token)
        );
    }

    #[test]
    fn keep_awake_backend_failures_never_manufacture_active_or_off() {
        let clock = Arc::new(FakeClock::new());
        let control = Control::default();
        control.fail_acquire.store(true, Ordering::SeqCst);
        let service = control.service(&clock);
        assert!(matches!(
            service.run("gui", Duration::from_secs(1), true),
            Err(KeepAwakeError::BackendFailed {
                operation: "acquire",
                ..
            })
        ));
        assert_eq!(service.stored_lease_count(), 0);
        service.shutdown().expect("no held request");
        control.fail_acquire.store(false, Ordering::SeqCst);

        let mut session = open(&control, &clock, "gui");
        control.fail_release.store(true, Ordering::SeqCst);
        assert!(matches!(
            session.stop(),
            Err(KeepAwakeError::BackendFailed {
                operation: "release",
                ..
            })
        ));
        assert!(matches!(
            session.status(),
            Err(KeepAwakeError::BackendFailed { .. })
        ));
        assert!(matches!(
            query(&session, KeepAwakeVerb::Stop),
            Err(KeepAwakeError::BackendFailed { .. })
        ));
        clock.advance(Duration::from_secs(31));
        assert!(matches!(
            query(&session, KeepAwakeVerb::Status),
            Err(KeepAwakeError::BackendFailed { .. })
        ));
        control.fail_release.store(false, Ordering::SeqCst);
        assert_eq!(
            session
                .stop()
                .expect("successful explicit clear retry")
                .state,
            "stopped"
        );
        session.shutdown().expect("joined");
    }

    #[test]
    fn keep_awake_shutdown_failure_is_returned_and_endpoint_is_unknown_after_join() {
        let clock = Arc::new(FakeClock::new());
        let control = Control::default();
        let mut session = open(&control, &clock, "gui");
        let tuple = (session.endpoint().to_string(), session.token().to_string());
        control.fail_release.store(true, Ordering::SeqCst);
        assert!(matches!(
            session.shutdown(),
            Err(KeepAwakeError::BackendFailed { .. })
        ));
        let count = control.calls.lock().expect("calls").len();
        drop(session);
        assert_eq!(control.calls.lock().expect("calls").len(), count);
        assert!(matches!(
            query_keep_awake(&tuple.0, KeepAwakeVerb::Status, &tuple.1, "gui"),
            Err(KeepAwakeError::NetworkUnavailable { .. })
        ));
        control.assert_one_worker();
    }

    #[test]
    fn keep_awake_transport_refuses_foreign_endpoint_identity_and_multiline_request() {
        let control = Control::default();
        let clock = Arc::new(FakeClock::new());
        let session = open(&control, &clock, "gui");
        assert_eq!(
            query_keep_awake("8.8.8.8:9", KeepAwakeVerb::Status, "token", "gui"),
            Err(KeepAwakeError::InvalidEndpoint)
        );
        assert_eq!(
            query_keep_awake(session.endpoint(), KeepAwakeVerb::Stop, "token\ngui", "gui"),
            Err(KeepAwakeError::InvalidRequest)
        );
        assert_eq!(
            query_keep_awake(session.endpoint(), KeepAwakeVerb::Stop, "unknown", "gui"),
            Err(KeepAwakeError::UnknownToken)
        );
        let mut stream = TcpStream::connect(session.endpoint()).expect("loopback");
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("timeout");
        stream
            .write_all(format!("STOP {} gui extra\n", session.token()).as_bytes())
            .expect("request");
        let reply: Reply =
            serde_json::from_str(&read_line(&mut stream, Duration::from_secs(1)).expect("refusal"))
                .expect("JSON");
        assert!(!reply.ok);
        assert_eq!(reply.code.as_deref(), Some("invalid_request"));
        assert_eq!(
            query(&session, KeepAwakeVerb::Status)
                .expect("still active")
                .state,
            "active"
        );
    }

    #[test]
    fn keep_awake_live_review_binds_duration_action_service_and_session_generation() {
        let control = Control::default();
        let clock = Arc::new(FakeClock::new());
        let service = control.service(&clock);
        let plan = service
            .plan_start("gui", Duration::from_secs(30))
            .expect("plan");
        let preview = service.preview(&plan).expect("preview");
        assert!(control.calls.lock().expect("calls").is_empty());
        clock.advance(Duration::from_secs(1));
        assert_eq!(
            service
                .preview(&plan)
                .expect("time alone does not stale")
                .digest,
            preview.digest
        );
        assert_eq!(
            service.execute_start(&plan, &preview.digest, false),
            Err(KeepAwakeError::ConfirmRequired)
        );
        let changed = service
            .plan_start("gui", Duration::from_secs(10))
            .expect("changed plan");
        assert_eq!(
            service.execute_start(&changed, &preview.digest, true),
            Err(KeepAwakeError::StaleConfirmation)
        );
        assert!(control.calls.lock().expect("calls").is_empty());
        let other = control.service(&clock);
        assert_eq!(other.preview(&plan), Err(KeepAwakeError::StaleConfirmation));
        let mut session = KeepAwakeSession::start(service, &plan, &preview.digest, true)
            .expect("confirmed session");
        let tuple = (session.endpoint().to_string(), session.token().to_string());
        let stop = session.plan_stop().expect("stop plan");
        let stop_preview = session.preview(&stop).expect("stop preview");
        assert_eq!(
            session.execute_stop(&stop, &stop_preview.digest, false),
            Err(KeepAwakeError::ConfirmRequired)
        );
        assert_eq!(
            query(&session, KeepAwakeVerb::Status)
                .expect("no unconfirmed clear")
                .state,
            "active"
        );
        clock.advance(Duration::from_secs(1));
        assert_eq!(
            session
                .preview(&stop)
                .expect("remaining time not authority")
                .digest,
            stop_preview.digest
        );
        session
            .execute_stop(&stop, &stop_preview.digest, true)
            .expect("confirmed stop");
        assert_eq!(
            session.execute_stop(&stop, &stop_preview.digest, true),
            Err(KeepAwakeError::StaleConfirmation)
        );
        let restart = session
            .plan_start(Duration::from_secs(2))
            .expect("same session restart");
        let restart_preview = session.preview(&restart).expect("restart preview");
        session
            .execute_start(&restart, &restart_preview.digest, true)
            .expect("restart");
        assert_eq!(
            (session.endpoint(), session.token()),
            (tuple.0.as_str(), tuple.1.as_str())
        );
        for _ in 0..3 {
            let stop = session.plan_stop().expect("repeat stop");
            let preview = session.preview(&stop).expect("repeat stop preview");
            session
                .execute_stop(&stop, &preview.digest, true)
                .expect("repeat clear");
            let restart = session
                .plan_start(Duration::from_secs(2))
                .expect("repeat start");
            let preview = session.preview(&restart).expect("repeat start preview");
            session
                .execute_start(&restart, &preview.digest, true)
                .expect("repeat acquire");
            assert_eq!(session.service.stored_lease_count(), 1);
            assert_eq!(
                (session.endpoint(), session.token()),
                (tuple.0.as_str(), tuple.1.as_str())
            );
        }
        let old_stop = session.plan_stop().expect("plan before expiry");
        let old_preview = session.preview(&old_stop).expect("preview before expiry");
        clock.advance(Duration::from_secs(2));
        assert_eq!(
            session.execute_stop(&old_stop, &old_preview.digest, true),
            Err(KeepAwakeError::StaleConfirmation)
        );
        assert_eq!(
            query(&session, KeepAwakeVerb::Status)
                .expect("expiry retained")
                .state,
            "expired"
        );
        session.shutdown().expect("joined");
        control.assert_one_worker();
    }
}
