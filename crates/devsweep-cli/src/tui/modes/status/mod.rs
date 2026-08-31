//! Status V1 presentation state. Collectors remain in `devsweep_core::status`.

use devsweep_core::status::{
    AvailabilityV1, DEFAULT_LIVE_INTERVAL_MS, DEFAULT_PROCESS_LIMIT, MAX_LIVE_INTERVAL_MS,
    MIN_LIVE_INTERVAL_MS, ProcessV1, StatusEventV1, StatusSnapshotV1, TerminalReason,
};

use crate::tui::app::JobId;

pub(in crate::tui) const MAX_CHART_POINTS: usize = 60;

const INTERVAL_STEPS_MS: [u32; 6] = [1_000, 2_000, 5_000, 10_000, 30_000, 60_000];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum StatusPhase {
    Idle,
    Snapshot,
    Ready,
    Live,
    Canceling,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum StatusOperation {
    Snapshot,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum ProcessSort {
    Cpu,
    Memory,
    Name,
    Pid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) struct ChartPoint {
    pub(in crate::tui) sequence: u64,
    pub(in crate::tui) sampled_at_unix_ms: Option<u64>,
    pub(in crate::tui) cpu_bp: Option<u32>,
    pub(in crate::tui) memory_used_bytes: Option<u64>,
    pub(in crate::tui) rx_bytes_per_second: Option<u64>,
    pub(in crate::tui) tx_bytes_per_second: Option<u64>,
}

impl ChartPoint {
    fn gap(sequence: u64) -> Self {
        Self {
            sequence,
            sampled_at_unix_ms: None,
            cpu_bp: None,
            memory_used_bytes: None,
            rx_bytes_per_second: None,
            tx_bytes_per_second: None,
        }
    }

    fn from_snapshot(sequence: u64, snapshot: &StatusSnapshotV1) -> Self {
        Self {
            sequence,
            sampled_at_unix_ms: Some(snapshot.sampled_at_unix_ms),
            cpu_bp: available_value(&snapshot.cpu).map(|cpu| cpu.system_utilization_basis_points),
            memory_used_bytes: available_value(&snapshot.memory).map(|memory| memory.used_bytes),
            rx_bytes_per_second: available_value(&snapshot.network).map(|network| {
                network
                    .interfaces
                    .iter()
                    .map(|item| item.rx_bytes_per_second)
                    .sum()
            }),
            tx_bytes_per_second: available_value(&snapshot.network).map(|network| {
                network
                    .interfaces
                    .iter()
                    .map(|item| item.tx_bytes_per_second)
                    .sum()
            }),
        }
    }

    pub(in crate::tui) fn is_gap(&self) -> bool {
        self.sampled_at_unix_ms.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) struct StatusModeState {
    pub(in crate::tui) phase: StatusPhase,
    pub(in crate::tui) operation_id: Option<JobId>,
    pub(in crate::tui) operation: Option<StatusOperation>,
    pub(in crate::tui) stream_id: Option<String>,
    pub(in crate::tui) snapshot: Option<StatusSnapshotV1>,
    pub(in crate::tui) chart: Vec<ChartPoint>,
    pub(in crate::tui) process_sort: ProcessSort,
    pub(in crate::tui) interval_ms: u32,
    pub(in crate::tui) process_limit: u32,
    pub(in crate::tui) last_sequence: Option<u64>,
    pub(in crate::tui) skipped_total: u64,
    pub(in crate::tui) cursor: usize,
    pub(in crate::tui) error: Option<String>,
    pub(in crate::tui) pending_live_restart: bool,
}

impl Default for StatusModeState {
    fn default() -> Self {
        Self {
            phase: StatusPhase::Idle,
            operation_id: None,
            operation: None,
            stream_id: None,
            snapshot: None,
            chart: Vec::new(),
            process_sort: ProcessSort::Cpu,
            interval_ms: DEFAULT_LIVE_INTERVAL_MS,
            process_limit: DEFAULT_PROCESS_LIMIT,
            last_sequence: None,
            skipped_total: 0,
            cursor: 0,
            error: None,
            pending_live_restart: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum StatusAction {
    SnapshotStarted(JobId),
    SnapshotFinished {
        job_id: JobId,
        snapshot: Box<StatusSnapshotV1>,
    },
    LiveStarted(JobId),
    LiveEvent {
        job_id: JobId,
        event: StatusEventV1,
    },
    IntervalStep(isize),
    SetIntervalMs(u32),
    CycleSort,
    MoveCursor(isize),
    CancelRequested(JobId),
    Canceled(JobId),
    Failed {
        job_id: JobId,
        message: String,
    },
    Released,
}

impl StatusModeState {
    pub(in crate::tui) fn reduce(&mut self, action: StatusAction) {
        match action {
            StatusAction::SnapshotStarted(job_id) if self.operation_id.is_none() => {
                self.phase = StatusPhase::Snapshot;
                self.operation_id = Some(job_id);
                self.operation = Some(StatusOperation::Snapshot);
                self.error = None;
            }
            StatusAction::SnapshotFinished { job_id, snapshot }
                if self.matches(job_id, StatusOperation::Snapshot) =>
            {
                self.phase = StatusPhase::Ready;
                self.operation_id = None;
                self.operation = None;
                self.cursor = 0;
                self.snapshot = Some(*snapshot);
                self.error = None;
            }
            StatusAction::LiveStarted(job_id) if self.operation_id.is_none() => {
                self.phase = StatusPhase::Live;
                self.operation_id = Some(job_id);
                self.operation = Some(StatusOperation::Live);
                self.stream_id = None;
                self.last_sequence = None;
                self.skipped_total = 0;
                self.chart.clear();
                self.error = None;
                self.pending_live_restart = false;
            }
            StatusAction::LiveEvent { job_id, event }
                if self.matches(job_id, StatusOperation::Live) =>
            {
                self.ingest_live_event(event);
            }
            StatusAction::IntervalStep(delta) => {
                self.interval_ms = step_interval(self.interval_ms, delta);
                if self.operation == Some(StatusOperation::Live) {
                    self.pending_live_restart = true;
                }
            }
            StatusAction::SetIntervalMs(interval_ms) => {
                self.interval_ms = clamp_interval_ms(interval_ms);
                if self.operation == Some(StatusOperation::Live) {
                    self.pending_live_restart = true;
                }
            }
            StatusAction::CycleSort => {
                self.process_sort = match self.process_sort {
                    ProcessSort::Cpu => ProcessSort::Memory,
                    ProcessSort::Memory => ProcessSort::Name,
                    ProcessSort::Name => ProcessSort::Pid,
                    ProcessSort::Pid => ProcessSort::Cpu,
                };
                self.cursor = 0;
            }
            StatusAction::MoveCursor(delta) => {
                let count = self.sorted_processes().len();
                if count > 0 {
                    self.cursor = self.cursor.saturating_add_signed(delta).min(count - 1);
                }
            }
            StatusAction::CancelRequested(job_id) if self.operation_id == Some(job_id) => {
                self.phase = StatusPhase::Canceling;
            }
            StatusAction::Canceled(job_id) if self.operation_id == Some(job_id) => {
                self.finish_operation(if self.snapshot.is_some() {
                    StatusPhase::Ready
                } else {
                    StatusPhase::Idle
                });
            }
            StatusAction::Failed { job_id, message } if self.operation_id == Some(job_id) => {
                self.finish_operation(StatusPhase::Failed);
                self.error = Some(message);
            }
            StatusAction::Released => *self = Self::default(),
            _ => {}
        }
    }

    fn ingest_live_event(&mut self, event: StatusEventV1) {
        let operation_id = event.operation_id().to_string();
        let sequence = event.sequence();
        if self
            .stream_id
            .as_deref()
            .is_some_and(|expected| expected != operation_id)
        {
            return;
        }
        match event {
            StatusEventV1::Started { data, .. } => {
                if self.stream_id.is_some() || self.last_sequence.is_some() || sequence != 0 {
                    self.fail_decode("status_started sequence is not exact");
                    return;
                }
                self.stream_id = Some(operation_id);
                self.last_sequence = Some(0);
                self.interval_ms = data.interval_ms;
                self.process_limit = data.process_limit;
                self.phase = StatusPhase::Live;
            }
            StatusEventV1::Snapshot { data, .. } => {
                if !self.accept_next_sequence(&operation_id, sequence) {
                    return;
                }
                let point = ChartPoint::from_snapshot(sequence, data.as_ref());
                self.snapshot = Some(*data);
                push_chart(&mut self.chart, point);
                self.clamp_cursor();
            }
            StatusEventV1::TickSkipped { data, .. } => {
                if !self.accept_next_sequence(&operation_id, sequence) {
                    return;
                }
                self.skipped_total = data.skipped_total;
                push_chart(&mut self.chart, ChartPoint::gap(sequence));
            }
            StatusEventV1::Terminal { data, .. } => {
                if !self.accept_next_sequence(&operation_id, sequence) {
                    return;
                }
                let phase = match data.reason {
                    TerminalReason::ProducerError => StatusPhase::Failed,
                    _ if self.snapshot.is_some() => StatusPhase::Ready,
                    _ => StatusPhase::Idle,
                };
                if data.reason == TerminalReason::ProducerError {
                    self.error = data.error_code.clone();
                }
                self.finish_operation(phase);
            }
        }
    }

    fn accept_next_sequence(&mut self, operation_id: &str, sequence: u64) -> bool {
        if self.stream_id.as_deref() != Some(operation_id) {
            self.fail_decode("live event is missing status_started");
            return false;
        }
        let expected = self.last_sequence.map(|value| value.saturating_add(1));
        if expected != Some(sequence) {
            self.fail_decode("skipped, duplicate, or nonmonotonic sequence");
            return false;
        }
        self.last_sequence = Some(sequence);
        true
    }

    fn fail_decode(&mut self, reason: &str) {
        self.phase = StatusPhase::Failed;
        self.error = Some(reason.to_string());
    }

    fn finish_operation(&mut self, phase: StatusPhase) {
        self.phase = phase;
        self.operation_id = None;
        self.operation = None;
        self.stream_id = None;
        self.last_sequence = None;
    }

    fn matches(&self, job_id: JobId, operation: StatusOperation) -> bool {
        self.operation_id == Some(job_id) && self.operation == Some(operation)
    }

    fn clamp_cursor(&mut self) {
        let count = self.sorted_processes().len();
        if count == 0 {
            self.cursor = 0;
        } else {
            self.cursor = self.cursor.min(count - 1);
        }
    }

    pub(in crate::tui) fn sorted_processes(&self) -> Vec<&ProcessV1> {
        let Some(processes) = self.snapshot.as_ref().and_then(|snapshot| {
            available_value(&snapshot.processes).map(|group| group.items.as_slice())
        }) else {
            return Vec::new();
        };
        let mut rows: Vec<&ProcessV1> = processes.iter().collect();
        rows.sort_by(|left, right| match self.process_sort {
            ProcessSort::Cpu => right
                .cpu_basis_points_of_one_logical_core
                .cmp(&left.cpu_basis_points_of_one_logical_core)
                .then_with(|| left.pid.cmp(&right.pid)),
            ProcessSort::Memory => right
                .private_bytes
                .cmp(&left.private_bytes)
                .then_with(|| left.pid.cmp(&right.pid)),
            ProcessSort::Name => left
                .name
                .cmp(&right.name)
                .then_with(|| left.pid.cmp(&right.pid)),
            ProcessSort::Pid => left.pid.cmp(&right.pid),
        });
        rows
    }

    pub(in crate::tui) fn needs_cancel_after_decode_failure(&self) -> bool {
        self.phase == StatusPhase::Failed && self.operation_id.is_some()
    }
}

pub(in crate::tui) fn available_value<T>(availability: &AvailabilityV1<T>) -> Option<&T> {
    match availability {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
            Some(value)
        }
        _ => None,
    }
}

pub(in crate::tui) fn basis_points(points: u32) -> String {
    format!("{}.{:02}", points / 100, points % 100)
}

fn push_chart(points: &mut Vec<ChartPoint>, point: ChartPoint) {
    points.push(point);
    if points.len() > MAX_CHART_POINTS {
        let overflow = points.len() - MAX_CHART_POINTS;
        points.drain(0..overflow);
    }
}

fn clamp_interval_ms(interval_ms: u32) -> u32 {
    let rounded = interval_ms / 1_000 * 1_000;
    rounded.clamp(MIN_LIVE_INTERVAL_MS, MAX_LIVE_INTERVAL_MS)
}

fn step_interval(current: u32, delta: isize) -> u32 {
    let index = INTERVAL_STEPS_MS
        .iter()
        .position(|value| *value == current)
        .unwrap_or(1);
    let next = index
        .saturating_add_signed(delta)
        .min(INTERVAL_STEPS_MS.len() - 1);
    INTERVAL_STEPS_MS[next]
}

#[cfg(test)]
mod tests;
