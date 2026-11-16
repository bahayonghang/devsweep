//! In-memory keep-awake leases. A lease is owned by a session token and an
//! explicit finite duration. Stop, expiry, and owner exit release only the
//! matching token. This module does not call a persistent power-setting API.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::keep_awake_audit::{
    FileKeepAwakeAudit, KeepAwakeAudit, KeepAwakeAuditAction, RecordingKeepAwakeAudit,
    StatusActionAuditCode, StatusActionAuditRecordV1,
};

/// Longest accepted lease. The duration is still explicit on each request.
pub const MAX_KEEP_AWAKE_MS: u64 = 12 * 60 * 60 * 1_000;

/// One lease view. `power_settings_changed` is always false.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeepAwakeStatusV1 {
    pub token: String,
    pub owner: String,
    pub state: String,
    pub duration_ms: u64,
    pub remaining_ms: u64,
    pub power_settings_changed: bool,
}

/// Keep-awake refusal. A capability gap stores nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeepAwakeError {
    CapabilityGap {
        reason: &'static str,
    },
    ConfirmRequired,
    DurationNotFinite,
    InvalidOwner,
    UnknownToken,
    OwnerMismatch,
    BackendFailed {
        operation: &'static str,
        message: String,
    },
    WorkerUnavailable,
    SessionIo {
        operation: &'static str,
        message: String,
    },
    InvalidEndpoint,
    InvalidRequest,
    NetworkUnavailable {
        message: String,
    },
    InvalidReply,
    StaleConfirmation,
    AuditFailed {
        after_dispatch: bool,
        message: String,
    },
}

impl fmt::Display for KeepAwakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapabilityGap { reason } => {
                write!(formatter, "keep-awake capability gap: {reason}")
            }
            Self::ConfirmRequired => write!(formatter, "keep-awake requires confirm"),
            Self::DurationNotFinite => {
                write!(formatter, "keep-awake duration must be explicit and finite")
            }
            Self::InvalidOwner => write!(formatter, "keep-awake owner is invalid"),
            Self::UnknownToken => write!(formatter, "keep-awake token was not found"),
            Self::OwnerMismatch => {
                write!(formatter, "keep-awake token belongs to another owner")
            }
            Self::BackendFailed { operation, message } => {
                write!(formatter, "keep-awake {operation} failed: {message}")
            }
            Self::WorkerUnavailable => write!(formatter, "keep-awake owner worker is unavailable"),
            Self::SessionIo { operation, message } => {
                write!(
                    formatter,
                    "keep-awake session {operation} failed: {message}"
                )
            }
            Self::InvalidEndpoint => write!(
                formatter,
                "keep-awake endpoint must be a loopback socket address"
            ),
            Self::InvalidRequest => write!(formatter, "keep-awake request is invalid"),
            Self::NetworkUnavailable { message } => {
                write!(formatter, "keep-awake endpoint unavailable: {message}")
            }
            Self::InvalidReply => write!(formatter, "keep-awake reply is invalid"),
            Self::StaleConfirmation => {
                write!(formatter, "keep-awake preview changed; review again")
            }
            Self::AuditFailed { message, .. } => write!(
                formatter,
                "keep-awake action audit failed; result is unconfirmed: {message}"
            ),
        }
    }
}

impl std::error::Error for KeepAwakeError {}

impl KeepAwakeError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::CapabilityGap { .. } => "unsupported_platform",
            Self::ConfirmRequired => "confirmation_required",
            Self::DurationNotFinite
            | Self::InvalidOwner
            | Self::InvalidRequest
            | Self::InvalidReply => "invalid_plan",
            Self::UnknownToken | Self::OwnerMismatch => "invalid_identity",
            Self::StaleConfirmation => "stale_confirmation",
            Self::BackendFailed {
                operation: "acquire",
                ..
            } => "keep_awake_acquire_failed",
            Self::BackendFailed { .. } => "keep_awake_clear_failed",
            Self::WorkerUnavailable => "keep_awake_worker_unavailable",
            Self::SessionIo { .. } => "output_open_failed",
            Self::InvalidEndpoint => "invalid_endpoint",
            Self::NetworkUnavailable { .. } => "network_unavailable",
            Self::AuditFailed {
                after_dispatch: false,
                ..
            } => "audit_unavailable",
            Self::AuditFailed {
                after_dispatch: true,
                ..
            } => "unknown_after_dispatch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeepAwakeAction {
    Start,
    Stop,
}

/// Immutable in-memory request, bound to the live service generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepAwakePlan {
    service_id: String,
    generation: u64,
    action: KeepAwakeAction,
    owner: String,
    duration: Duration,
    token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct KeepAwakePreviewV1 {
    pub action: KeepAwakeAction,
    pub owner: String,
    pub duration_ms: u64,
    pub current_state: String,
    pub digest: String,
}

/// Clock used by lease expiry. Tests supply a fake clock.
pub trait KeepAwakeClock: Send + Sync {
    fn now(&self) -> Instant;
}

/// Optional display request. The default recorder does not change power settings.
pub trait AwakeBackend: Send {
    fn acquire(&mut self, token: &str) -> Result<(), KeepAwakeError>;
    fn release(&mut self, token: &str) -> Result<(), KeepAwakeError>;
}

/// Wall clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl KeepAwakeClock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Test clock. `advance` moves lease expiry without sleeping.
#[derive(Debug)]
pub struct FakeClock {
    origin: Instant,
    offset_ms: AtomicU64,
}

impl FakeClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            offset_ms: AtomicU64::new(0),
        }
    }

    pub fn advance(&self, duration: Duration) {
        self.offset_ms.fetch_add(
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
            Ordering::SeqCst,
        );
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl KeepAwakeClock for FakeClock {
    fn now(&self) -> Instant {
        self.origin + Duration::from_millis(self.offset_ms.load(Ordering::SeqCst))
    }
}

/// Records acquire and release. It does not change power settings.
#[derive(Debug, Clone)]
pub struct RecordingBackend {
    log: Arc<Mutex<Vec<BackendCall>>>,
}

/// One backend call captured by [`RecordingBackend`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendCall {
    Acquire(String),
    Release(String),
}

impl RecordingBackend {
    #[must_use]
    pub fn pair() -> (Self, Arc<Mutex<Vec<BackendCall>>>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                log: Arc::clone(&log),
            },
            log,
        )
    }
}

impl AwakeBackend for RecordingBackend {
    fn acquire(&mut self, token: &str) -> Result<(), KeepAwakeError> {
        self.log
            .lock()
            .expect("keep-awake backend lock")
            .push(BackendCall::Acquire(token.to_string()));
        Ok(())
    }

    fn release(&mut self, token: &str) -> Result<(), KeepAwakeError> {
        self.log
            .lock()
            .expect("keep-awake backend lock")
            .push(BackendCall::Release(token.to_string()));
        Ok(())
    }
}

/// Backend that does not touch the operating system.
#[derive(Debug, Default)]
pub struct NoopBackend;

impl AwakeBackend for NoopBackend {
    fn acquire(&mut self, _token: &str) -> Result<(), KeepAwakeError> {
        Ok(())
    }

    fn release(&mut self, _token: &str) -> Result<(), KeepAwakeError> {
        Ok(())
    }
}

/// Windows display request. This is a thread execution state, not a power scheme.
#[cfg(windows)]
#[derive(Debug, Default)]
struct ExecutionStateBackend {
    held: u32,
}

#[cfg(windows)]
impl AwakeBackend for ExecutionStateBackend {
    fn acquire(&mut self, _token: &str) -> Result<(), KeepAwakeError> {
        if self.held == 0 {
            set_execution_state(true)?;
        }
        self.held += 1;
        Ok(())
    }

    fn release(&mut self, _token: &str) -> Result<(), KeepAwakeError> {
        if self.held == 1 {
            set_execution_state(false)?;
        }
        self.held = self.held.saturating_sub(1);
        Ok(())
    }
}

#[cfg(windows)]
fn set_execution_state(active: bool) -> Result<(), KeepAwakeError> {
    use windows_sys::Win32::System::Power::{
        ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
    };

    let flags = if active {
        ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    // SAFETY: the flag is a thread execution request. It does not write a power scheme.
    if unsafe { SetThreadExecutionState(flags) } == 0 {
        return Err(KeepAwakeError::BackendFailed {
            operation: if active { "acquire" } else { "release" },
            message: std::io::Error::last_os_error().to_string(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeaseState {
    Active,
    Stopped,
    Expired,
    Canceled,
    OwnerExited,
}

impl LeaseState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Stopped => "stopped",
            Self::Expired => "expired",
            Self::Canceled => "canceled",
            Self::OwnerExited => "owner_exited",
        }
    }
}

struct Lease {
    token: String,
    owner: String,
    duration: Duration,
    started: Instant,
    state: LeaseState,
    error: Option<KeepAwakeError>,
    expiry_attempted: bool,
}

struct Inner {
    leases: Vec<Lease>,
    backend: Box<dyn AwakeBackend>,
    clock: Arc<dyn KeepAwakeClock>,
    service_id: String,
    generations: BTreeMap<String, u64>,
    audit: Box<dyn KeepAwakeAudit>,
}

type OwnerTask = Box<dyn FnOnce(&mut Inner) + Send>;

enum OwnerMessage {
    Task(OwnerTask),
    Shutdown,
}

/// Process-local leases whose backend is used exclusively by one joined thread.
/// No caller, including the transport thread, invokes a backend directly.
pub struct KeepAwakeService {
    sender: Sender<OwnerMessage>,
    worker: Mutex<Option<JoinHandle<Result<(), KeepAwakeError>>>>,
    supported: bool,
}

impl KeepAwakeService {
    #[must_use]
    pub fn new(
        clock: Arc<dyn KeepAwakeClock>,
        backend: Box<dyn AwakeBackend>,
        supported: bool,
    ) -> Self {
        Self::new_with_audit(
            clock,
            backend,
            supported,
            Box::new(RecordingKeepAwakeAudit::default()),
        )
    }

    /// Injects a backend and journal; production uses the fixed durable Status store.
    #[must_use]
    pub fn new_with_audit(
        clock: Arc<dyn KeepAwakeClock>,
        backend: Box<dyn AwakeBackend>,
        supported: bool,
        audit: Box<dyn KeepAwakeAudit>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut inner = Inner {
                leases: Vec::new(),
                backend,
                clock,
                service_id: new_token(),
                generations: BTreeMap::new(),
                audit,
            };
            loop {
                match receiver.recv_timeout(Duration::from_millis(20)) {
                    Ok(OwnerMessage::Task(task)) => {
                        task(&mut inner);
                        inner.refresh();
                    }
                    Ok(OwnerMessage::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                        return inner.close();
                    }
                    Err(RecvTimeoutError::Timeout) => inner.refresh(),
                }
            }
        });
        Self {
            sender,
            worker: Mutex::new(Some(worker)),
            supported,
        }
    }

    /// Supported on Windows. Other platforms report a capability gap.
    #[must_use]
    pub fn production() -> Self {
        #[cfg(windows)]
        {
            Self::new_with_audit(
                Arc::new(SystemClock),
                Box::new(ExecutionStateBackend::default()),
                true,
                Box::new(FileKeepAwakeAudit::production()),
            )
        }
        #[cfg(not(windows))]
        {
            Self::new_with_audit(
                Arc::new(SystemClock),
                Box::new(NoopBackend),
                false,
                Box::new(FileKeepAwakeAudit::production()),
            )
        }
    }

    #[must_use]
    pub fn is_supported(&self) -> bool {
        self.supported
    }

    #[must_use]
    pub fn stored_lease_count(&self) -> usize {
        self.request(|inner| Ok(inner.leases.len())).unwrap_or(0)
    }

    pub fn run(
        &self,
        owner: &str,
        duration: Duration,
        confirm: bool,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        if !self.supported {
            return Err(KeepAwakeError::CapabilityGap {
                reason: "platform_unsupported",
            });
        }
        if !confirm {
            return Err(KeepAwakeError::ConfirmRequired);
        }
        let plan = self.plan_start(owner, duration)?;
        let preview = self.preview(&plan)?;
        self.execute_start(&plan, &preview.digest, confirm)
    }

    pub fn plan_start(
        &self,
        owner: &str,
        duration: Duration,
    ) -> Result<KeepAwakePlan, KeepAwakeError> {
        self.validate_request(owner, duration)?;
        let owner = owner.to_string();
        self.request(move |inner| {
            inner.refresh();
            Ok(KeepAwakePlan {
                service_id: inner.service_id.clone(),
                generation: inner.generation(&owner),
                action: KeepAwakeAction::Start,
                owner,
                duration,
                token: None,
            })
        })
    }

    pub(crate) fn plan_existing(
        &self,
        token: &str,
        owner: &str,
        action: KeepAwakeAction,
        duration: Option<Duration>,
    ) -> Result<KeepAwakePlan, KeepAwakeError> {
        if let Some(duration) = duration {
            self.validate_request(owner, duration)?;
        }
        let token = token.to_string();
        let owner = owner.to_string();
        self.request(move |inner| {
            inner.refresh();
            let lease = find_lease(&inner.leases, &token, &owner)?;
            if action == KeepAwakeAction::Start
                && let Some(error) = &lease.error
            {
                return Err(error.clone());
            }
            if (action == KeepAwakeAction::Start
                && !matches!(
                    lease.state,
                    LeaseState::Stopped | LeaseState::Expired | LeaseState::Canceled
                ))
                || (action == KeepAwakeAction::Stop && lease.state != LeaseState::Active)
            {
                return Err(KeepAwakeError::InvalidRequest);
            }
            Ok(KeepAwakePlan {
                service_id: inner.service_id.clone(),
                generation: inner.generation(&owner),
                action,
                owner,
                duration: duration.unwrap_or(lease.duration),
                token: Some(token),
            })
        })
    }

    pub fn preview(&self, plan: &KeepAwakePlan) -> Result<KeepAwakePreviewV1, KeepAwakeError> {
        let plan = plan.clone();
        self.request(move |inner| inner.preview(&plan))
    }

    pub(crate) fn execute_start(
        &self,
        plan: &KeepAwakePlan,
        expected_digest: &str,
        confirmed: bool,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        if !confirmed {
            return Err(KeepAwakeError::ConfirmRequired);
        }
        let plan = plan.clone();
        let expected_digest = expected_digest.to_string();
        self.request(move |inner| {
            let preview = inner.preview(&plan)?;
            if plan.action != KeepAwakeAction::Start || preview.digest != expected_digest {
                return Err(KeepAwakeError::StaleConfirmation);
            }
            inner.start(&plan, &preview.digest)
        })
    }

    pub(crate) fn execute_stop(
        &self,
        plan: &KeepAwakePlan,
        expected_digest: &str,
        confirmed: bool,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        if !confirmed {
            return Err(KeepAwakeError::ConfirmRequired);
        }
        let plan = plan.clone();
        let expected_digest = expected_digest.to_string();
        self.request(move |inner| {
            let preview = inner.preview(&plan)?;
            if plan.action != KeepAwakeAction::Stop || preview.digest != expected_digest {
                return Err(KeepAwakeError::StaleConfirmation);
            }
            inner.finish(
                plan.token
                    .as_deref()
                    .ok_or(KeepAwakeError::InvalidRequest)?,
                &plan.owner,
                LeaseState::Stopped,
                Some(&preview.digest),
            )
        })
    }

    fn validate_request(&self, owner: &str, duration: Duration) -> Result<(), KeepAwakeError> {
        if !self.supported {
            return Err(KeepAwakeError::CapabilityGap {
                reason: "platform_unsupported",
            });
        }
        let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        if duration_ms == 0
            || duration > Duration::from_millis(MAX_KEEP_AWAKE_MS)
            || !duration.as_nanos().is_multiple_of(1_000_000)
        {
            return Err(KeepAwakeError::DurationNotFinite);
        }
        if !valid_owner(owner) {
            return Err(KeepAwakeError::InvalidOwner);
        }
        Ok(())
    }

    pub fn status(&self, token: &str, owner: &str) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        let token = token.to_string();
        let owner = owner.to_string();
        self.request(move |inner| {
            inner.refresh();
            let lease = find_lease(&inner.leases, &token, &owner)?;
            if let Some(error) = &lease.error {
                return Err(error.clone());
            }
            Ok(view(lease, inner.clock.now()))
        })
    }

    pub fn stop(&self, token: &str, owner: &str) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        // Existing explicit tuple protocol is the CLI confirmation surface.
        // Repeated STOP stays idempotent; fresh active STOP uses the shared chain.
        match self.status(token, owner) {
            Ok(status) if status.state != "active" => return Ok(status),
            Ok(_)
            | Err(KeepAwakeError::BackendFailed { .. } | KeepAwakeError::AuditFailed { .. }) => {}
            Err(error) => return Err(error),
        }
        let plan = self.plan_existing(token, owner, KeepAwakeAction::Stop, None)?;
        let preview = self.preview(&plan)?;
        self.execute_stop(&plan, &preview.digest, true)
    }

    pub fn cancel(&self, token: &str, owner: &str) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.finish(token, owner, LeaseState::Canceled)
    }

    /// Releases only this owner's leases. A failed clear remains nonterminal.
    pub fn owner_exit(&self, owner: &str) -> Result<Vec<String>, KeepAwakeError> {
        let owner = owner.to_string();
        self.request(move |inner| {
            let tokens = inner
                .leases
                .iter()
                .filter(|lease| lease.owner == owner && lease.state == LeaseState::Active)
                .map(|lease| lease.token.clone())
                .collect::<Vec<_>>();
            let mut first_error = None;
            for token in &tokens {
                if let Err(error) = inner.finish(token, &owner, LeaseState::OwnerExited, None) {
                    first_error.get_or_insert(error);
                }
            }
            first_error.map_or(Ok(tokens), Err)
        })
    }

    /// Clears remaining requests on their owner thread and joins it, even on failure.
    pub fn shutdown(&self) -> Result<(), KeepAwakeError> {
        let Some(worker) = self.worker.lock().expect("keep-awake worker lock").take() else {
            return Ok(());
        };
        let _ = self.sender.send(OwnerMessage::Shutdown);
        worker
            .join()
            .map_err(|_| KeepAwakeError::WorkerUnavailable)?
    }

    fn finish(
        &self,
        token: &str,
        owner: &str,
        next: LeaseState,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        let token = token.to_string();
        let owner = owner.to_string();
        self.request(move |inner| inner.finish(&token, &owner, next, None))
    }

    fn request<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Inner) -> Result<T, KeepAwakeError> + Send + 'static,
    ) -> Result<T, KeepAwakeError> {
        let (sender, receiver) = mpsc::sync_channel(0);
        self.sender
            .send(OwnerMessage::Task(Box::new(move |inner| {
                let _ = sender.send(action(inner));
            })))
            .map_err(|_| KeepAwakeError::WorkerUnavailable)?;
        receiver
            .recv()
            .map_err(|_| KeepAwakeError::WorkerUnavailable)?
    }
}

impl Drop for KeepAwakeService {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            tracing::error!(%error, "keep-awake owner shutdown failed; release is unconfirmed");
        }
    }
}

impl Inner {
    fn generation(&self, owner: &str) -> u64 {
        self.generations.get(owner).copied().unwrap_or(0)
    }

    fn advance(&mut self, owner: &str) {
        *self.generations.entry(owner.to_string()).or_default() += 1;
    }

    fn preview(&mut self, plan: &KeepAwakePlan) -> Result<KeepAwakePreviewV1, KeepAwakeError> {
        self.refresh();
        if plan.service_id != self.service_id || plan.generation != self.generation(&plan.owner) {
            return Err(KeepAwakeError::StaleConfirmation);
        }
        let current_state = if let Some(token) = &plan.token {
            let lease = find_lease(&self.leases, token, &plan.owner)?;
            if plan.action == KeepAwakeAction::Start
                && !matches!(
                    lease.state,
                    LeaseState::Stopped | LeaseState::Expired | LeaseState::Canceled
                )
            {
                return Err(KeepAwakeError::StaleConfirmation);
            }
            if plan.action == KeepAwakeAction::Stop && lease.state != LeaseState::Active {
                return Err(KeepAwakeError::StaleConfirmation);
            }
            lease.state.as_str()
        } else {
            "idle"
        };
        let duration_ms = u64::try_from(plan.duration.as_millis()).unwrap_or(u64::MAX);
        let preimage = serde_json::to_vec(&(
            "devsweep.keep-awake.v1",
            &plan.service_id,
            plan.generation,
            plan.action,
            &plan.owner,
            duration_ms,
            &plan.token,
            current_state,
        ))
        .map_err(|_| KeepAwakeError::InvalidRequest)?;
        let digest = format!(
            "sha256:{}",
            crate::digest_encoding::hex_digest(Sha256::digest(preimage))
        );
        Ok(KeepAwakePreviewV1 {
            action: plan.action,
            owner: plan.owner.clone(),
            duration_ms,
            current_state: current_state.into(),
            digest,
        })
    }

    fn record(
        &mut self,
        operation_id: &str,
        action: KeepAwakeAuditAction,
        status_code: StatusActionAuditCode,
        duration: Duration,
        digest: Option<&str>,
    ) -> Result<(), KeepAwakeError> {
        self.audit.append(StatusActionAuditRecordV1 {
            schema_version: 1,
            domain: "status".into(),
            operation_id: operation_id.into(),
            timestamp_unix_ms: unix_now_ms(),
            action,
            status_code,
            duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
            preview_digest: digest.map(str::to_string),
            process: None,
        })
    }

    fn start(
        &mut self,
        plan: &KeepAwakePlan,
        digest: &str,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        let operation_id = new_operation_id();
        self.record(
            &operation_id,
            KeepAwakeAuditAction::Start,
            StatusActionAuditCode::DispatchStarted,
            plan.duration,
            Some(digest),
        )?;
        let token = plan.token.clone().unwrap_or_else(new_token);
        let acquired = self.backend.acquire(&token);
        self.advance(&plan.owner);
        if let Err(error) = acquired {
            self.record(
                &operation_id,
                KeepAwakeAuditAction::Start,
                StatusActionAuditCode::BackendFailed,
                plan.duration,
                Some(digest),
            )
            .map_err(|audit| after_dispatch_with_backend(audit, Some(&error)))?;
            return Err(error);
        }
        let lease = Lease {
            token: token.clone(),
            owner: plan.owner.clone(),
            duration: plan.duration,
            started: self.clock.now(),
            state: LeaseState::Active,
            error: None,
            expiry_attempted: false,
        };
        if let Some(current) = self
            .leases
            .iter_mut()
            .find(|current| current.token == token)
        {
            *current = lease;
        } else {
            self.leases.push(lease);
        }
        let terminal = self
            .record(
                &operation_id,
                KeepAwakeAuditAction::Start,
                StatusActionAuditCode::Succeeded,
                plan.duration,
                Some(digest),
            )
            .map_err(after_dispatch);
        if let Err(error) = terminal {
            find_lease_mut(&mut self.leases, &token, &plan.owner)?.error = Some(error.clone());
            return Err(error);
        }
        Ok(view(
            find_lease(&self.leases, &token, &plan.owner)?,
            self.clock.now(),
        ))
    }

    fn finish(
        &mut self,
        token: &str,
        owner: &str,
        next: LeaseState,
        digest: Option<&str>,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        let lease = find_lease(&self.leases, token, owner)?;
        if lease.state != LeaseState::Active {
            if let Some(error) = &lease.error {
                return Err(error.clone());
            }
            return Ok(view(lease, self.clock.now()));
        }
        let duration = lease.duration;
        let action = match next {
            LeaseState::Stopped => KeepAwakeAuditAction::Stop,
            LeaseState::Expired => KeepAwakeAuditAction::Expiry,
            LeaseState::Canceled => KeepAwakeAuditAction::Cancel,
            _ => KeepAwakeAuditAction::OwnerExit,
        };
        let operation_id = new_operation_id();
        let started = self.record(
            &operation_id,
            action,
            StatusActionAuditCode::DispatchStarted,
            duration,
            digest,
        );
        // Explicit actions require a durable started record. Safety cleanup on
        // expiry/exit/cancellation still clears even if the audit became unavailable.
        if next == LeaseState::Stopped {
            started.clone()?;
        }
        let released = self.backend.release(token);
        self.advance(owner);
        let outcome = if released.is_ok() {
            StatusActionAuditCode::Succeeded
        } else {
            StatusActionAuditCode::BackendFailed
        };
        let audited = if started.is_ok() {
            self.record(&operation_id, action, outcome, duration, digest)
        } else {
            started
        };
        let backend_error = released.err();
        let error = audited
            .err()
            .map(|audit| after_dispatch_with_backend(audit, backend_error.as_ref()))
            .or(backend_error);
        let lease = find_lease_mut(&mut self.leases, token, owner)?;
        if outcome == StatusActionAuditCode::Succeeded {
            lease.state = next;
        }
        lease.error = error.clone();
        if let Some(error) = error {
            return Err(error);
        }
        Ok(view(lease, self.clock.now()))
    }

    fn refresh(&mut self) {
        let now = self.clock.now();
        let expired = self
            .leases
            .iter()
            .filter(|lease| {
                lease.state == LeaseState::Active
                    && !lease.expiry_attempted
                    && now >= lease.started + lease.duration
            })
            .map(|lease| (lease.token.clone(), lease.owner.clone()))
            .collect::<Vec<_>>();
        for (token, owner) in expired {
            // A failed clear is retained for explicit retry/shutdown; no false off
            // result and no unbounded background retry/audit loop.
            if let Ok(lease) = find_lease_mut(&mut self.leases, &token, &owner) {
                lease.expiry_attempted = true;
            }
            let _ = self.finish(&token, &owner, LeaseState::Expired, None);
        }
    }

    fn close(&mut self) -> Result<(), KeepAwakeError> {
        let active = self
            .leases
            .iter()
            .filter(|lease| lease.state == LeaseState::Active)
            .map(|lease| (lease.token.clone(), lease.owner.clone()))
            .collect::<Vec<_>>();
        let mut first_error = None;
        for (token, owner) in active {
            if let Err(error) = self.finish(&token, &owner, LeaseState::OwnerExited, None) {
                first_error.get_or_insert(error);
            }
        }
        if first_error.is_none() {
            first_error = self.leases.iter().find_map(|lease| lease.error.clone());
        }
        first_error.map_or(Ok(()), Err)
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn new_operation_id() -> String {
    format!("status-{}-{}", std::process::id(), new_token())
}

fn after_dispatch(error: KeepAwakeError) -> KeepAwakeError {
    match error {
        KeepAwakeError::AuditFailed { message, .. } => KeepAwakeError::AuditFailed {
            after_dispatch: true,
            message,
        },
        other => other,
    }
}

fn after_dispatch_with_backend(
    audit: KeepAwakeError,
    backend: Option<&KeepAwakeError>,
) -> KeepAwakeError {
    let mut error = after_dispatch(audit);
    if let (KeepAwakeError::AuditFailed { message, .. }, Some(backend)) = (&mut error, backend) {
        message.push_str("; ");
        message.push_str(&backend.to_string());
    }
    error
}

/// Drops by releasing only this owner's active tokens.
pub struct KeepAwakeOwner {
    service: Arc<KeepAwakeService>,
    owner: String,
}

impl KeepAwakeOwner {
    #[must_use]
    pub fn new(service: Arc<KeepAwakeService>, owner: impl Into<String>) -> Self {
        Self {
            service,
            owner: owner.into(),
        }
    }
}

impl Drop for KeepAwakeOwner {
    fn drop(&mut self) {
        if let Err(error) = self.service.owner_exit(&self.owner) {
            tracing::error!(%error, "keep-awake owner release failed; release is unconfirmed");
        }
    }
}

fn find_lease<'a>(
    leases: &'a [Lease],
    token: &str,
    owner: &str,
) -> Result<&'a Lease, KeepAwakeError> {
    let Some(lease) = leases.iter().find(|lease| lease.token == token) else {
        return Err(KeepAwakeError::UnknownToken);
    };
    if lease.owner != owner {
        return Err(KeepAwakeError::OwnerMismatch);
    }
    Ok(lease)
}

fn find_lease_mut<'a>(
    leases: &'a mut [Lease],
    token: &str,
    owner: &str,
) -> Result<&'a mut Lease, KeepAwakeError> {
    let Some(position) = leases.iter().position(|lease| lease.token == token) else {
        return Err(KeepAwakeError::UnknownToken);
    };
    if leases[position].owner != owner {
        return Err(KeepAwakeError::OwnerMismatch);
    }
    Ok(&mut leases[position])
}

fn view(lease: &Lease, now: Instant) -> KeepAwakeStatusV1 {
    let duration_ms = u64::try_from(lease.duration.as_millis()).unwrap_or(u64::MAX);
    let remaining_ms = if lease.state == LeaseState::Active {
        let elapsed = now.saturating_duration_since(lease.started);
        duration_ms.saturating_sub(u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
    } else {
        0
    };
    KeepAwakeStatusV1 {
        token: lease.token.clone(),
        owner: lease.owner.clone(),
        state: lease.state.as_str().to_string(),
        duration_ms,
        remaining_ms,
        power_settings_changed: false,
    }
}

fn valid_owner(owner: &str) -> bool {
    !owner.is_empty()
        && owner.len() <= 64
        && owner
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn new_token() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("ka-{n:x}-{millis:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_service(clock: &Arc<FakeClock>) -> (KeepAwakeService, Arc<Mutex<Vec<BackendCall>>>) {
        let (backend, log) = RecordingBackend::pair();
        (
            KeepAwakeService::new(
                Arc::clone(clock) as Arc<dyn KeepAwakeClock>,
                Box::new(backend),
                true,
            ),
            log,
        )
    }

    #[test]
    fn keep_awake_lease_expiry_token_ownership_cancellation_and_owner_exit() {
        let clock = Arc::new(FakeClock::new());
        let (service, _log) = open_service(&clock);
        let first = service
            .run("owner-a", Duration::from_millis(50), true)
            .expect("first lease");
        let second = service
            .run("owner-b", Duration::from_secs(30), true)
            .expect("second lease");
        assert_eq!(first.state, "active");
        assert!(!first.power_settings_changed);
        assert_ne!(first.token, second.token);

        let wrong_owner = service.status(&first.token, "owner-b");
        assert_eq!(wrong_owner, Err(KeepAwakeError::OwnerMismatch));
        assert_eq!(
            service
                .status(&first.token, "owner-a")
                .expect("still active")
                .state,
            "active"
        );

        let wrong_stop = service.stop(&first.token, "owner-b");
        assert_eq!(wrong_stop, Err(KeepAwakeError::OwnerMismatch));
        assert_eq!(
            service.status(&second.token, "owner-b").expect("b").state,
            "active"
        );

        let stopped = service.stop(&first.token, "owner-a").expect("stop a");
        assert_eq!(stopped.state, "stopped");
        assert_eq!(stopped.remaining_ms, 0);
        assert_eq!(
            service
                .status(&second.token, "owner-b")
                .expect("b stays")
                .state,
            "active"
        );

        let (service, _log) = open_service(&clock);
        let canceled_lease = service
            .run("owner-a", Duration::from_secs(30), true)
            .expect("cancel target");
        let kept = service
            .run("owner-b", Duration::from_secs(30), true)
            .expect("kept");
        let canceled = service
            .cancel(&canceled_lease.token, "owner-a")
            .expect("cancel");
        assert_eq!(canceled.state, "canceled");
        assert_eq!(
            service.status(&kept.token, "owner-b").expect("kept").state,
            "active"
        );

        let (service, log) = open_service(&clock);
        let short = service
            .run("owner-a", Duration::from_millis(40), true)
            .expect("short");
        let long = service
            .run("owner-b", Duration::from_secs(30), true)
            .expect("long");
        clock.advance(Duration::from_millis(40));
        let expired = service.status(&short.token, "owner-a").expect("expired");
        assert_eq!(expired.state, "expired");
        assert_eq!(
            service.status(&long.token, "owner-b").expect("long").state,
            "active"
        );

        let (service, _) = open_service(&clock);
        let owned = Arc::new(service);
        let left = owned
            .run("owner-a", Duration::from_secs(30), true)
            .expect("left");
        let right = owned
            .run("owner-b", Duration::from_secs(30), true)
            .expect("right");
        {
            let _guard = KeepAwakeOwner::new(Arc::clone(&owned), "owner-a");
        }
        assert_eq!(
            owned.status(&left.token, "owner-a").expect("exited").state,
            "owner_exited"
        );
        assert_eq!(
            owned
                .status(&right.token, "owner-b")
                .expect("other owner")
                .state,
            "active"
        );

        let calls = log.lock().expect("log").clone();
        assert!(
            calls
                .iter()
                .any(|call| matches!(call, BackendCall::Release(token) if token == &short.token))
        );
        assert!(
            !calls
                .iter()
                .any(|call| matches!(call, BackendCall::Release(token) if token == &long.token))
        );
        let source = include_str!("keep_awake.rs");
        for needle in [
            ["power", "cfg"].concat(),
            ["Power", "Write"].concat(),
            ["Power", "SetActiveScheme"].concat(),
            ["System", "ParametersInfo"].concat(),
        ] {
            assert!(!source.contains(&needle), "{needle}");
        }
    }

    #[test]
    fn keep_awake_unsupported_platform_returns_a_capability_gap_and_changes_nothing() {
        let clock = Arc::new(FakeClock::new());
        let (backend, log) = RecordingBackend::pair();
        let service = KeepAwakeService::new(
            Arc::clone(&clock) as Arc<dyn KeepAwakeClock>,
            Box::new(backend),
            false,
        );
        let error = service
            .run("owner-a", Duration::from_millis(20), true)
            .expect_err("gap");
        assert_eq!(
            error,
            KeepAwakeError::CapabilityGap {
                reason: "platform_unsupported"
            }
        );
        assert_eq!(service.stored_lease_count(), 0);
        assert!(log.lock().expect("log").is_empty());
        assert_eq!(
            service.run("owner-a", Duration::ZERO, true),
            Err(KeepAwakeError::CapabilityGap {
                reason: "platform_unsupported"
            })
        );

        let (supported, log) = service_supported(&clock);
        assert_eq!(
            supported.run("owner-a", Duration::from_millis(20), false),
            Err(KeepAwakeError::ConfirmRequired)
        );
        assert_eq!(
            supported.run("owner-a", Duration::ZERO, true),
            Err(KeepAwakeError::DurationNotFinite)
        );
        assert_eq!(
            supported.run(
                "owner-a",
                Duration::from_millis(MAX_KEEP_AWAKE_MS + 1),
                true
            ),
            Err(KeepAwakeError::DurationNotFinite)
        );
        assert_eq!(supported.stored_lease_count(), 0);
        assert!(log.lock().expect("log").is_empty());
    }

    fn service_supported(
        clock: &Arc<FakeClock>,
    ) -> (KeepAwakeService, Arc<Mutex<Vec<BackendCall>>>) {
        open_service(clock)
    }
}
