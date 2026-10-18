//! In-memory keep-awake leases. A lease is owned by a session token and an
//! explicit finite duration. Stop, expiry, and owner exit release only the
//! matching token. This module does not call a persistent power-setting API.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
    CapabilityGap { reason: &'static str },
    ConfirmRequired,
    DurationNotFinite,
    InvalidOwner,
    UnknownToken,
    OwnerMismatch,
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
        }
    }
}

impl std::error::Error for KeepAwakeError {}

/// Clock used by lease expiry. Tests supply a fake clock.
pub trait KeepAwakeClock: Send + Sync {
    fn now(&self) -> Instant;
}

/// Optional display request. The default recorder does not change power settings.
pub trait AwakeBackend: Send {
    fn acquire(&mut self, token: &str);
    fn release(&mut self, token: &str);
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
    fn acquire(&mut self, token: &str) {
        self.log
            .lock()
            .expect("keep-awake backend lock")
            .push(BackendCall::Acquire(token.to_string()));
    }

    fn release(&mut self, token: &str) {
        self.log
            .lock()
            .expect("keep-awake backend lock")
            .push(BackendCall::Release(token.to_string()));
    }
}

/// Backend that does not touch the operating system.
#[derive(Debug, Default)]
pub struct NoopBackend;

impl AwakeBackend for NoopBackend {
    fn acquire(&mut self, _token: &str) {}

    fn release(&mut self, _token: &str) {}
}

/// Windows display request. This is a thread execution state, not a power scheme.
#[cfg(windows)]
#[derive(Debug, Default)]
struct ExecutionStateBackend {
    held: u32,
}

#[cfg(windows)]
impl AwakeBackend for ExecutionStateBackend {
    fn acquire(&mut self, _token: &str) {
        self.held = self.held.saturating_add(1);
        if self.held == 1 {
            set_execution_state(true);
        }
    }

    fn release(&mut self, _token: &str) {
        self.held = self.held.saturating_sub(1);
        if self.held == 0 {
            set_execution_state(false);
        }
    }
}

#[cfg(windows)]
impl Drop for ExecutionStateBackend {
    fn drop(&mut self) {
        if self.held > 0 {
            self.held = 0;
            set_execution_state(false);
        }
    }
}

#[cfg(windows)]
fn set_execution_state(active: bool) {
    use windows_sys::Win32::System::Power::{
        ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
    };

    let flags = if active {
        ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    // SAFETY: the flag is a thread execution request. It does not write a power scheme.
    unsafe {
        let _ = SetThreadExecutionState(flags);
    }
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
}

struct Inner {
    leases: Vec<Lease>,
    backend: Box<dyn AwakeBackend>,
}

/// Process-local lease table.
pub struct KeepAwakeService {
    inner: Mutex<Inner>,
    clock: Arc<dyn KeepAwakeClock>,
    supported: bool,
}

impl KeepAwakeService {
    #[must_use]
    pub fn new(
        clock: Arc<dyn KeepAwakeClock>,
        backend: Box<dyn AwakeBackend>,
        supported: bool,
    ) -> Self {
        Self {
            inner: Mutex::new(Inner {
                leases: Vec::new(),
                backend,
            }),
            clock,
            supported,
        }
    }

    /// Supported on Windows. Other platforms report a capability gap.
    #[must_use]
    pub fn production() -> Self {
        #[cfg(windows)]
        {
            Self::new(
                Arc::new(SystemClock),
                Box::new(ExecutionStateBackend::default()),
                true,
            )
        }
        #[cfg(not(windows))]
        {
            Self::new(Arc::new(SystemClock), Box::new(NoopBackend), false)
        }
    }

    #[must_use]
    pub fn is_supported(&self) -> bool {
        self.supported
    }

    #[must_use]
    pub fn stored_lease_count(&self) -> usize {
        self.inner.lock().expect("keep-awake lock").leases.len()
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
        let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        if duration_ms == 0 || duration_ms > MAX_KEEP_AWAKE_MS {
            return Err(KeepAwakeError::DurationNotFinite);
        }
        if !valid_owner(owner) {
            return Err(KeepAwakeError::InvalidOwner);
        }
        let token = new_token();
        let started = self.clock.now();
        let mut inner = self.inner.lock().expect("keep-awake lock");
        inner.backend.acquire(&token);
        inner.leases.push(Lease {
            token: token.clone(),
            owner: owner.to_string(),
            duration,
            started,
            state: LeaseState::Active,
        });
        drop(inner);
        self.status(&token, owner)
    }

    pub fn status(&self, token: &str, owner: &str) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.refresh();
        let inner = self.inner.lock().expect("keep-awake lock");
        let lease = find_lease(&inner.leases, token, owner)?;
        Ok(view(lease, self.clock.now()))
    }

    pub fn stop(&self, token: &str, owner: &str) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.finish(token, owner, LeaseState::Stopped)
    }

    pub fn cancel(&self, token: &str, owner: &str) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.finish(token, owner, LeaseState::Canceled)
    }

    /// Release every active lease for `owner` and leave every other owner in place.
    pub fn owner_exit(&self, owner: &str) -> Vec<String> {
        self.refresh();
        let mut inner = self.inner.lock().expect("keep-awake lock");
        let mut released = Vec::new();
        for lease in inner.leases.iter_mut().filter(|lease| lease.owner == owner) {
            if lease.state == LeaseState::Active {
                lease.state = LeaseState::OwnerExited;
                released.push(lease.token.clone());
            }
        }
        for token in &released {
            inner.backend.release(token);
        }
        released
    }

    fn finish(
        &self,
        token: &str,
        owner: &str,
        next: LeaseState,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        self.refresh();
        let now = self.clock.now();
        let mut inner = self.inner.lock().expect("keep-awake lock");
        let current = find_lease(&inner.leases, token, owner)?.state;
        if current != LeaseState::Active {
            let lease = find_lease(&inner.leases, token, owner)?;
            return Ok(view(lease, now));
        }
        let status = {
            let lease = find_lease_mut(&mut inner.leases, token, owner)?;
            lease.state = next;
            view(lease, now)
        };
        inner.backend.release(token);
        Ok(status)
    }

    fn refresh(&self) {
        let now = self.clock.now();
        let mut inner = self.inner.lock().expect("keep-awake lock");
        let expired = inner
            .leases
            .iter()
            .filter(|lease| {
                lease.state == LeaseState::Active && now >= lease.started + lease.duration
            })
            .map(|lease| lease.token.clone())
            .collect::<Vec<_>>();
        for token in &expired {
            if let Some(lease) = inner.leases.iter_mut().find(|lease| lease.token == *token) {
                lease.state = LeaseState::Expired;
            }
            inner.backend.release(token);
        }
    }
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
        let _ = self.service.owner_exit(&self.owner);
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
