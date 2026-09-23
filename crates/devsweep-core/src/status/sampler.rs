//! Snapshot and live cadence: one sample in flight, skipped ticks, cancel/join.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::process::{CancelObserver, FlagCancelObserver, NoopCancelObserver};

use super::{
    AvailabilityV1, CpuV1, DEFAULT_LIVE_INTERVAL_MS, DEFAULT_PROCESS_LIMIT, LiveRequest,
    PROCESS_DETAIL_BUDGET_MS, PROCESS_REFRESH_MS, SNAPSHOT_WINDOW_MS, StatusEventV1,
    StatusSnapshotV1, StatusStartedV1, StatusTerminalV1, TickSkipReason, TickSkippedV1,
    VOLUME_POWER_REFRESH_MS, clamp_interval_ms, clamp_process_limit, logical_processor_count,
    network::{InterfaceCounters, network_from_pair, sample_interfaces},
    pdh::{PdhGroups, PdhProbe},
    process::{
        ProcessCounters, ProcessGroupMeta, ProcessIdentity, detail_processes, enumerate_identities,
        sample_process_group,
    },
    system::{
        CpuTimes, cpu_availability, sample_cpu_times, sample_memory, sample_power, sample_volumes,
    },
    unix_now_ms, unsupported_capabilities,
};

const _: (u32, u32) = (DEFAULT_LIVE_INTERVAL_MS, DEFAULT_PROCESS_LIMIT);

static LIVE_PERMIT: AtomicBool = AtomicBool::new(false);
static SNAPSHOT_SEQ: AtomicU64 = AtomicU64::new(1);

/// Status collector errors. Coordinator refusal is fail-closed and never
/// starts a second live producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusError {
    Canceled,
    CoordinatorRefused,
    ChannelClosed,
}

impl std::fmt::Display for StatusError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canceled => formatter.write_str("status sampling was canceled"),
            Self::CoordinatorRefused => {
                formatter.write_str("status live is already running in this process")
            }
            Self::ChannelClosed => formatter.write_str("status live channel closed"),
        }
    }
}

impl std::error::Error for StatusError {}

struct LivePermit;

impl Drop for LivePermit {
    fn drop(&mut self) {
        LIVE_PERMIT.store(false, Ordering::SeqCst);
    }
}

fn acquire_live() -> Result<LivePermit, StatusError> {
    LIVE_PERMIT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map(|_| LivePermit)
        .map_err(|_| StatusError::CoordinatorRefused)
}

/// One bounded Status sampler. Live occupies the process coordinator permit.
#[derive(Debug, Default)]
pub struct StatusSampler {
    state: SamplerCache,
}

#[derive(Debug, Default, Clone)]
struct SamplerCache {
    cpu: Option<CpuTimes>,
    network: Option<Vec<InterfaceCounters>>,
    processes: Option<Vec<ProcessCounters>>,
    process_identities: Option<Vec<ProcessIdentity>>,
    process_enumerated: u32,
    process_budget_exhausted: bool,
    process_access_denied: bool,
    processes_at_ms: u64,
    volumes: Option<AvailabilityV1<super::VolumesV1>>,
    volumes_at_ms: u64,
    power: Option<AvailabilityV1<super::PowerV1>>,
    power_at_ms: u64,
    last_processes: Option<AvailabilityV1<super::ProcessesV1>>,
    last_sample_at: Option<Instant>,
    pdh: Option<Arc<PdhProbe>>,
}

impl SamplerCache {
    /// Open and prime the PDH query once per sampler. Later ticks collect once
    /// and read the rate since the previous tick.
    fn pdh_probe(&mut self) -> Arc<PdhProbe> {
        Arc::clone(
            self.pdh
                .get_or_insert_with(|| Arc::new(PdhProbe::open_primed())),
        )
    }
}

impl StatusSampler {
    /// Capture baseline counters, wait 500 ms cooperatively, then compose.
    pub fn snapshot(
        &self,
        process_limit: u32,
        cancel: Option<&dyn CancelObserver>,
    ) -> Result<StatusSnapshotV1, StatusError> {
        let noop = NoopCancelObserver;
        let observer: &dyn CancelObserver = cancel.unwrap_or(&noop);
        let mut cache = self.state.clone();
        collect_snapshot(&mut cache, process_limit, SNAPSHOT_WINDOW_MS, observer)
    }
}

/// Spawn the live producer. The caller writes events and must join on stop.
pub fn spawn_live(
    request: LiveRequest<'_>,
) -> Result<(Receiver<StatusEventV1>, LiveControl), StatusError> {
    let permit = acquire_live()?;
    let interval_ms = clamp_interval_ms(request.interval_ms);
    let process_limit = clamp_process_limit(request.process_limit);
    let operation_id = if request.operation_id.is_empty() {
        format!("op-{}", unix_now_ms())
    } else {
        request.operation_id.clone()
    };
    let cancel = request
        .cancel
        .cloned()
        .unwrap_or_else(|| Arc::new(FlagCancelObserver::new()));
    let (tx, rx) = mpsc::channel();
    let worker_cancel = Arc::clone(&cancel);
    let worker_id = operation_id.clone();
    let join = thread::Builder::new()
        .name("devsweep-status-live".into())
        .spawn(move || {
            let _permit = permit;
            run_live_loop(tx, worker_id, interval_ms, process_limit, &worker_cancel);
        })
        .expect("status live thread starts");
    Ok((
        rx,
        LiveControl {
            cancel,
            join: Some(join),
        },
    ))
}

/// Cancel and join handle for the live producer.
pub struct LiveControl {
    cancel: Arc<FlagCancelObserver>,
    join: Option<JoinHandle<()>>,
}

impl LiveControl {
    pub fn request_cancel(&self) {
        self.cancel.request_cancel();
    }

    pub fn join(&mut self) {
        self.request_cancel();
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }

    #[must_use]
    pub fn cancel_flag(&self) -> Arc<FlagCancelObserver> {
        Arc::clone(&self.cancel)
    }
}

impl Drop for LiveControl {
    fn drop(&mut self) {
        self.join();
    }
}

fn run_live_loop(
    tx: Sender<StatusEventV1>,
    operation_id: String,
    interval_ms: u32,
    process_limit: u32,
    cancel: &FlagCancelObserver,
) {
    let mut sequence = 0_u64;
    let mut skipped_total = 0_u64;
    if emit(
        &tx,
        StatusEventV1::Started {
            schema_version: 1,
            operation_id: operation_id.clone(),
            sequence,
            emitted_at_unix_ms: unix_now_ms(),
            data: StatusStartedV1 {
                interval_ms,
                process_limit,
            },
        },
    )
    .is_err()
    {
        return;
    }
    sequence += 1;

    let mut cache = SamplerCache::default();
    let mut next = Instant::now();
    loop {
        if cancel.is_cancel_requested() {
            let _ = emit(
                &tx,
                terminal(
                    &operation_id,
                    sequence,
                    super::TerminalReason::Canceled,
                    None,
                ),
            );
            return;
        }
        let now = Instant::now();
        if now < next {
            let remaining = next.saturating_duration_since(now);
            if wait_with_cancel(remaining.min(Duration::from_millis(25)), cancel) {
                let _ = emit(
                    &tx,
                    terminal(
                        &operation_id,
                        sequence,
                        super::TerminalReason::Canceled,
                        None,
                    ),
                );
                return;
            }
            continue;
        }

        match collect_snapshot(&mut cache, process_limit, interval_ms, cancel) {
            Ok(snapshot) => {
                if emit(
                    &tx,
                    StatusEventV1::Snapshot {
                        schema_version: 1,
                        operation_id: operation_id.clone(),
                        sequence,
                        emitted_at_unix_ms: unix_now_ms(),
                        data: Box::new(snapshot),
                    },
                )
                .is_err()
                {
                    return;
                }
                sequence += 1;
            }
            Err(StatusError::Canceled) => {
                let _ = emit(
                    &tx,
                    terminal(
                        &operation_id,
                        sequence,
                        super::TerminalReason::Canceled,
                        None,
                    ),
                );
                return;
            }
            Err(_) => {
                let _ = emit(
                    &tx,
                    terminal(
                        &operation_id,
                        sequence,
                        super::TerminalReason::ProducerError,
                        Some("status_sample_failed"),
                    ),
                );
                return;
            }
        }

        next += Duration::from_millis(u64::from(interval_ms));
        while next <= Instant::now() {
            skipped_total += 1;
            if emit(
                &tx,
                StatusEventV1::TickSkipped {
                    schema_version: 1,
                    operation_id: operation_id.clone(),
                    sequence,
                    emitted_at_unix_ms: unix_now_ms(),
                    data: TickSkippedV1 {
                        reason: TickSkipReason::SampleInFlight,
                        skipped_total,
                    },
                },
            )
            .is_err()
            {
                return;
            }
            sequence += 1;
            next += Duration::from_millis(u64::from(interval_ms));
        }
    }
}

fn emit(tx: &Sender<StatusEventV1>, event: StatusEventV1) -> Result<(), StatusError> {
    tx.send(event).map_err(|_| StatusError::ChannelClosed)
}

fn terminal(
    operation_id: &str,
    sequence: u64,
    reason: super::TerminalReason,
    error_code: Option<&str>,
) -> StatusEventV1 {
    StatusEventV1::Terminal {
        schema_version: 1,
        operation_id: operation_id.to_string(),
        sequence,
        emitted_at_unix_ms: unix_now_ms(),
        data: StatusTerminalV1 {
            reason,
            error_code: error_code.map(str::to_string),
        },
    }
}

fn wait_with_cancel(duration: Duration, cancel: &dyn CancelObserver) -> bool {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        if cancel.is_cancel_requested() {
            return true;
        }
        thread::sleep(
            Duration::from_millis(10).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    cancel.is_cancel_requested()
}

fn collect_snapshot(
    cache: &mut SamplerCache,
    process_limit: u32,
    expected_window_ms: u32,
    cancel: &dyn CancelObserver,
) -> Result<StatusSnapshotV1, StatusError> {
    let process_limit = clamp_process_limit(process_limit);
    let need_process_refresh = cache.processes.is_none()
        || monotonic_ms().saturating_sub(cache.processes_at_ms) >= u64::from(PROCESS_REFRESH_MS);
    let need_volume_refresh = cache.volumes.is_none()
        || monotonic_ms().saturating_sub(cache.volumes_at_ms) >= u64::from(VOLUME_POWER_REFRESH_MS);
    let need_power_refresh = cache.power.is_none()
        || monotonic_ms().saturating_sub(cache.power_at_ms) >= u64::from(VOLUME_POWER_REFRESH_MS);

    let first_cpu = match sample_cpu_times() {
        Ok(times) => times,
        Err(failure) => {
            if cache.cpu.is_none() {
                // still wait so snapshot window is honest
            }
            return compose_without_delta(
                cache,
                process_limit,
                expected_window_ms,
                failure.cpu(),
                need_volume_refresh,
                need_power_refresh,
                cancel,
            );
        }
    };
    let first_net = sample_interfaces().ok();
    let pdh = cache.pdh_probe();
    let ProcessBaseline {
        counters: first_proc,
        identities,
        enumerated,
        budget_exhausted: budget,
        access_denied: denied,
    } = if need_process_refresh {
        capture_process_baseline(cancel)?
    } else {
        ProcessBaseline {
            counters: cache.processes.clone().unwrap_or_default(),
            identities: cache.process_identities.clone().unwrap_or_default(),
            enumerated: cache.process_enumerated,
            budget_exhausted: cache.process_budget_exhausted,
            access_denied: cache.process_access_denied,
        }
    };

    let started = Instant::now();
    let wait_ms = if cache.last_sample_at.is_some() {
        0
    } else {
        SNAPSHOT_WINDOW_MS
    };
    if wait_ms > 0 && wait_with_cancel(Duration::from_millis(u64::from(wait_ms)), cancel) {
        return Err(StatusError::Canceled);
    }
    let actual_window_ms = match cache.last_sample_at {
        Some(previous) => u32::try_from(previous.elapsed().as_millis().min(u128::from(u32::MAX)))
            .unwrap_or(u32::MAX)
            .max(1),
        None => u32::try_from(started.elapsed().as_millis().min(u128::from(u32::MAX)))
            .unwrap_or(SNAPSHOT_WINDOW_MS)
            .max(1),
    };
    cache.last_sample_at = Some(Instant::now());

    if cancel.is_cancel_requested() {
        return Err(StatusError::Canceled);
    }

    let sampled_at = unix_now_ms();
    let cpu = match (cache.cpu.replace(first_cpu), sample_cpu_times()) {
        (Some(previous), Ok(current)) => {
            cache.cpu = Some(current);
            cpu_availability(
                previous,
                current,
                expected_window_ms,
                actual_window_ms,
                sampled_at,
            )
        }
        (None, Ok(current)) => {
            cache.cpu = Some(current);
            cpu_availability(
                first_cpu,
                current,
                expected_window_ms,
                actual_window_ms,
                sampled_at,
            )
        }
        (_, Err(failure)) => failure.cpu(),
    };

    let network = match (cache.network.clone(), sample_interfaces()) {
        (Some(previous), Ok(current)) => {
            cache.network = Some(current.clone());
            network_from_pair(&previous, &current, expected_window_ms, actual_window_ms)
        }
        (None, Ok(current)) => {
            let first = first_net.unwrap_or_default();
            cache.network = Some(current.clone());
            network_from_pair(&first, &current, expected_window_ms, actual_window_ms)
        }
        (_, Err(failure)) => match failure {
            super::system::AvailabilityFailure::PermissionDenied => {
                AvailabilityV1::permission_denied("access_denied")
            }
            super::system::AvailabilityFailure::Unsupported => {
                AvailabilityV1::unsupported("platform_unsupported")
            }
            super::system::AvailabilityFailure::Unavailable => {
                AvailabilityV1::unavailable("api_unavailable")
            }
        },
    };

    let memory = sample_memory();
    if need_volume_refresh {
        cache.volumes = Some(sample_volumes());
        cache.volumes_at_ms = monotonic_ms();
    }
    if need_power_refresh {
        cache.power = Some(sample_power());
        cache.power_at_ms = monotonic_ms();
    }
    let volumes = age_group(cache.volumes.clone(), cache.volumes_at_ms, sampled_at);
    let power = age_group(cache.power.clone(), cache.power_at_ms, sampled_at);
    let PdhGroups { gpu, thermal } = pdh.sample(sampled_at);

    let processes = if need_process_refresh {
        let second = detail_processes(
            &identities,
            Some(cancel),
            Duration::from_millis(u64::from(PROCESS_DETAIL_BUDGET_MS)),
        );
        if cancel.is_cancel_requested() {
            return Err(StatusError::Canceled);
        }
        cache.processes = Some(second.counters.clone());
        cache.process_identities = Some(identities);
        cache.process_enumerated = enumerated;
        cache.process_budget_exhausted = budget || second.budget_exhausted;
        cache.process_access_denied = denied || second.access_denied;
        cache.processes_at_ms = monotonic_ms();
        let group = sample_process_group(
            process_limit,
            &first_proc,
            &second.counters,
            ProcessGroupMeta {
                enumerated_count: enumerated,
                budget_exhausted: cache.process_budget_exhausted,
                access_denied: cache.process_access_denied,
            },
            expected_window_ms,
            actual_window_ms,
        );
        cache.last_processes = Some(group.clone());
        group
    } else {
        age_group(
            cache.last_processes.clone(),
            cache.processes_at_ms,
            sampled_at,
        )
    };

    Ok(StatusSnapshotV1 {
        snapshot_id: format!(
            "snapshot-{}-{}",
            sampled_at,
            SNAPSHOT_SEQ.fetch_add(1, Ordering::Relaxed)
        ),
        sampled_at_unix_ms: sampled_at,
        sample_window_ms: actual_window_ms.max(1),
        logical_processor_count: logical_processor_count(),
        cpu,
        memory,
        volumes,
        network,
        power,
        unsupported_capabilities: unsupported_capabilities(&gpu, &thermal),
        gpu,
        thermal,
        processes,
    })
}

fn compose_without_delta(
    cache: &mut SamplerCache,
    process_limit: u32,
    expected_window_ms: u32,
    cpu: AvailabilityV1<CpuV1>,
    need_volume_refresh: bool,
    need_power_refresh: bool,
    cancel: &dyn CancelObserver,
) -> Result<StatusSnapshotV1, StatusError> {
    let pdh = cache.pdh_probe();
    if wait_with_cancel(Duration::from_millis(u64::from(SNAPSHOT_WINDOW_MS)), cancel) {
        return Err(StatusError::Canceled);
    }
    let sampled_at = unix_now_ms();
    let PdhGroups { gpu, thermal } = pdh.sample(sampled_at);
    if need_volume_refresh {
        cache.volumes = Some(sample_volumes());
        cache.volumes_at_ms = monotonic_ms();
    }
    if need_power_refresh {
        cache.power = Some(sample_power());
        cache.power_at_ms = monotonic_ms();
    }
    let processes = match enumerate_identities(Some(cancel)) {
        Ok(enumeration) => {
            let detail = detail_processes(
                &enumeration.identities,
                Some(cancel),
                Duration::from_millis(u64::from(PROCESS_DETAIL_BUDGET_MS)),
            );
            sample_process_group(
                process_limit,
                &[],
                &detail.counters,
                ProcessGroupMeta {
                    enumerated_count: enumeration.enumerated_count,
                    budget_exhausted: detail.budget_exhausted,
                    access_denied: detail.access_denied,
                },
                expected_window_ms,
                expected_window_ms.max(1),
            )
        }
        Err(super::system::AvailabilityFailure::PermissionDenied) => {
            AvailabilityV1::permission_denied("access_denied")
        }
        Err(super::system::AvailabilityFailure::Unsupported) => {
            AvailabilityV1::unsupported("platform_unsupported")
        }
        Err(super::system::AvailabilityFailure::Unavailable) => {
            AvailabilityV1::unavailable("api_unavailable")
        }
    };
    let network = match sample_interfaces() {
        Ok(current) => {
            cache.network = Some(current.clone());
            AvailabilityV1::unavailable("zero_elapsed")
        }
        Err(super::system::AvailabilityFailure::PermissionDenied) => {
            AvailabilityV1::permission_denied("access_denied")
        }
        Err(super::system::AvailabilityFailure::Unsupported) => {
            AvailabilityV1::unsupported("platform_unsupported")
        }
        Err(super::system::AvailabilityFailure::Unavailable) => {
            AvailabilityV1::unavailable("api_unavailable")
        }
    };
    Ok(StatusSnapshotV1 {
        snapshot_id: format!(
            "snapshot-{}-{}",
            sampled_at,
            SNAPSHOT_SEQ.fetch_add(1, Ordering::Relaxed)
        ),
        sampled_at_unix_ms: sampled_at,
        sample_window_ms: SNAPSHOT_WINDOW_MS,
        logical_processor_count: logical_processor_count(),
        cpu,
        memory: sample_memory(),
        volumes: age_group(cache.volumes.clone(), cache.volumes_at_ms, sampled_at),
        network,
        power: age_group(cache.power.clone(), cache.power_at_ms, sampled_at),
        unsupported_capabilities: unsupported_capabilities(&gpu, &thermal),
        gpu,
        thermal,
        processes,
    })
}

struct ProcessBaseline {
    counters: Vec<ProcessCounters>,
    identities: Vec<ProcessIdentity>,
    enumerated: u32,
    budget_exhausted: bool,
    access_denied: bool,
}

fn capture_process_baseline(cancel: &dyn CancelObserver) -> Result<ProcessBaseline, StatusError> {
    if cancel.is_cancel_requested() {
        return Err(StatusError::Canceled);
    }
    match enumerate_identities(Some(cancel)) {
        Ok(enumeration) => {
            let detail = detail_processes(
                &enumeration.identities,
                Some(cancel),
                Duration::from_millis(u64::from(PROCESS_DETAIL_BUDGET_MS)),
            );
            Ok(ProcessBaseline {
                counters: detail.counters,
                identities: enumeration.identities,
                enumerated: enumeration.enumerated_count,
                budget_exhausted: detail.budget_exhausted,
                access_denied: detail.access_denied,
            })
        }
        Err(super::system::AvailabilityFailure::Unsupported) => Ok(ProcessBaseline {
            counters: Vec::new(),
            identities: Vec::new(),
            enumerated: 0,
            budget_exhausted: false,
            access_denied: false,
        }),
        Err(_) => Ok(ProcessBaseline {
            counters: Vec::new(),
            identities: Vec::new(),
            enumerated: 0,
            budget_exhausted: false,
            access_denied: true,
        }),
    }
}

fn age_group<T: Clone>(
    group: Option<AvailabilityV1<T>>,
    captured_at_ms: u64,
    sampled_at_unix_ms: u64,
) -> AvailabilityV1<T> {
    let _ = sampled_at_unix_ms;
    let age = monotonic_ms().saturating_sub(captured_at_ms);
    match group {
        Some(value) => value.with_age(age),
        None => AvailabilityV1::unavailable("not_sampled"),
    }
}

fn monotonic_ms() -> u64 {
    static ORIGIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let origin = ORIGIN.get_or_init(Instant::now);
    u64::try_from(origin.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Test-only scheduler model with fake time: one in flight, skip, no backfill.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct FakeScheduler {
    pub now_ms: u64,
    pub interval_ms: u32,
    pub in_flight: bool,
    pub next_deadline_ms: u64,
    pub skipped_total: u64,
    pub snapshots: u64,
    pub running: bool,
}

#[cfg(test)]
impl FakeScheduler {
    pub(crate) fn new(interval_ms: u32) -> Self {
        Self {
            now_ms: 0,
            interval_ms: clamp_interval_ms(interval_ms),
            in_flight: false,
            next_deadline_ms: 0,
            skipped_total: 0,
            snapshots: 0,
            running: true,
        }
    }

    pub(crate) fn on_tick(&mut self, sample_duration_ms: u64) -> TickOutcome {
        if !self.running {
            return TickOutcome::Stopped;
        }
        if self.in_flight {
            self.skipped_total += 1;
            self.next_deadline_ms += u64::from(self.interval_ms);
            return TickOutcome::Skipped {
                skipped_total: self.skipped_total,
            };
        }
        self.in_flight = true;
        self.now_ms += sample_duration_ms;
        self.in_flight = false;
        self.snapshots += 1;
        self.next_deadline_ms += u64::from(self.interval_ms);
        while self.next_deadline_ms <= self.now_ms {
            self.skipped_total += 1;
            self.next_deadline_ms += u64::from(self.interval_ms);
        }
        TickOutcome::Snapshot {
            skipped_total: self.skipped_total,
        }
    }

    pub(crate) fn cancel(&mut self) {
        self.running = false;
        self.in_flight = false;
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickOutcome {
    Snapshot { skipped_total: u64 },
    Skipped { skipped_total: u64 },
    Stopped,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LIVE: Mutex<()> = Mutex::new(());

    #[test]
    fn snapshot_window_and_interval_clamps() {
        assert_eq!(SNAPSHOT_WINDOW_MS, 500);
        assert_eq!(clamp_interval_ms(0), 1_000);
        assert_eq!(clamp_interval_ms(2_000), 2_000);
        assert_eq!(clamp_interval_ms(1_500), 1_000);
        assert_eq!(clamp_interval_ms(61_000), 60_000);
        assert_eq!(DEFAULT_LIVE_INTERVAL_MS, 2_000);
        assert_eq!(PROCESS_REFRESH_MS, 4_000);
        assert_eq!(VOLUME_POWER_REFRESH_MS, 30_000);
        assert_eq!(clamp_process_limit(0), 1);
        assert_eq!(clamp_process_limit(15), 15);
        assert_eq!(clamp_process_limit(101), 100);
    }

    #[test]
    fn one_in_flight_skips_and_does_not_backfill() {
        let mut scheduler = FakeScheduler::new(1_000);
        assert_eq!(
            scheduler.on_tick(2_500),
            TickOutcome::Snapshot { skipped_total: 2 }
        );
        assert_eq!(scheduler.snapshots, 1);
        scheduler.in_flight = true;
        assert_eq!(
            scheduler.on_tick(0),
            TickOutcome::Skipped { skipped_total: 3 }
        );
        assert_eq!(scheduler.snapshots, 1);
    }

    #[test]
    fn cancel_joins_and_stops_without_backfill() {
        let mut scheduler = FakeScheduler::new(2_000);
        scheduler.on_tick(10);
        scheduler.cancel();
        assert_eq!(scheduler.on_tick(10), TickOutcome::Stopped);
        assert!(!scheduler.running);
        assert!(!scheduler.in_flight);
    }

    #[test]
    fn coordinator_refuses_a_second_live_session() {
        let _lock = TEST_LIVE.lock().expect("live test lock");
        LIVE_PERMIT.store(false, Ordering::SeqCst);
        let first = acquire_live().expect("first live permit");
        assert!(matches!(
            acquire_live(),
            Err(StatusError::CoordinatorRefused)
        ));
        drop(first);
        acquire_live().expect("permit is released after join");
        LIVE_PERMIT.store(false, Ordering::SeqCst);
    }

    #[test]
    fn channel_close_stops_producer() {
        let _lock = TEST_LIVE.lock().expect("live test lock");
        LIVE_PERMIT.store(false, Ordering::SeqCst);
        let (rx, mut control) = spawn_live(LiveRequest {
            interval_ms: 1_000,
            process_limit: 1,
            operation_id: "op-channel".into(),
            cancel: None,
        })
        .expect("live starts");
        let started = rx.recv_timeout(Duration::from_secs(3));
        assert!(started.is_ok(), "producer emits status_started");
        drop(rx);
        control.join();
        LIVE_PERMIT.store(false, Ordering::SeqCst);
    }

    #[test]
    fn fake_time_snapshot_cadence_is_five_hundred_ms() {
        assert_eq!(SNAPSHOT_WINDOW_MS, 500);
        let mut scheduler = FakeScheduler::new(1_000);
        scheduler.on_tick(500);
        assert_eq!(scheduler.now_ms, 500);
        assert_eq!(scheduler.next_deadline_ms, 1_000);
    }
}
