//! Lossless live records with bounded backpressure and a cancellation reserve.

use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, mpsc::RecvTimeoutError},
    time::{Duration, Instant},
};

use crate::process::{CancelObserver, FlagCancelObserver};

use super::{StatusError, StatusEventV1};

const CAPACITY: usize = 16;

#[derive(Default)]
struct State {
    records: VecDeque<StatusEventV1>,
    receiver_closed: bool,
    producer_closed: bool,
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

/// Bounded, lossless live event receiver. Dropping it wakes a blocked producer.
pub struct StatusEventReceiver(Arc<Shared>);

pub(super) struct StatusEventSender {
    shared: Arc<Shared>,
    cancel: Arc<FlagCancelObserver>,
}

pub(super) fn channel(cancel: Arc<FlagCancelObserver>) -> (StatusEventSender, StatusEventReceiver) {
    let shared = Arc::new(Shared::default());
    (
        StatusEventSender {
            shared: shared.clone(),
            cancel,
        },
        StatusEventReceiver(shared),
    )
}

impl StatusEventSender {
    pub(super) fn send(&self, event: StatusEventV1) -> Result<(), StatusError> {
        let mut state = self.shared.state.lock().expect("status channel lock");
        while state.records.len() >= CAPACITY
            && !state.receiver_closed
            && !event.is_terminal()
            && !self.cancel.is_cancel_requested()
        {
            state = self
                .shared
                .changed
                .wait_timeout(state, Duration::from_millis(25))
                .expect("status channel wait")
                .0;
        }
        if state.receiver_closed {
            return Err(StatusError::ChannelClosed);
        }
        // The producer checks cancellation after every send. At most one
        // in-flight record and its terminal use the two reserved positions.
        assert!(state.records.len() < CAPACITY + 2);
        state.records.push_back(event);
        self.shared.changed.notify_all();
        Ok(())
    }
}

impl Drop for StatusEventSender {
    fn drop(&mut self) {
        self.shared
            .state
            .lock()
            .expect("status channel lock")
            .producer_closed = true;
        self.shared.changed.notify_all();
    }
}

impl StatusEventReceiver {
    /// Waits for one ordered record, preserving standard channel timeout errors.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<StatusEventV1, RecvTimeoutError> {
        let deadline = Instant::now() + timeout;
        let mut state = self.0.state.lock().expect("status channel lock");
        loop {
            if let Some(event) = state.records.pop_front() {
                self.0.changed.notify_all();
                return Ok(event);
            }
            if state.producer_closed {
                return Err(RecvTimeoutError::Disconnected);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(RecvTimeoutError::Timeout);
            }
            state = self
                .0
                .changed
                .wait_timeout(state, remaining)
                .expect("status channel wait")
                .0;
        }
    }
}

impl Drop for StatusEventReceiver {
    fn drop(&mut self) {
        self.0
            .state
            .lock()
            .expect("status channel lock")
            .receiver_closed = true;
        self.0.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{StatusStartedV1, StatusTerminalV1, TerminalReason};

    fn record(sequence: u64) -> StatusEventV1 {
        StatusEventV1::Started {
            schema_version: 1,
            operation_id: "bounded-test".into(),
            sequence,
            emitted_at_unix_ms: 1,
            data: StatusStartedV1 {
                interval_ms: 1000,
                process_limit: 1,
            },
        }
    }

    #[test]
    fn stalled_receiver_keeps_every_sequence_and_joins_after_cancel_before_draining() {
        let cancel = Arc::new(FlagCancelObserver::new());
        let (tx, rx) = channel(cancel.clone());
        for sequence in 0..CAPACITY as u64 {
            tx.send(record(sequence)).unwrap();
        }
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let join = std::thread::spawn(move || {
            tx.send(record(CAPACITY as u64)).unwrap();
            tx.send(StatusEventV1::Terminal {
                schema_version: 1,
                operation_id: "bounded-test".into(),
                sequence: CAPACITY as u64 + 1,
                emitted_at_unix_ms: 1,
                data: StatusTerminalV1 {
                    reason: TerminalReason::Canceled,
                    error_code: None,
                },
            })
            .unwrap();
            done_tx.send(()).unwrap();
        });
        assert_eq!(rx.0.state.lock().unwrap().records.len(), CAPACITY);
        assert!(done_rx.recv_timeout(Duration::from_millis(30)).is_err());
        cancel.request_cancel();
        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        join.join().unwrap();
        assert_eq!(rx.0.state.lock().unwrap().records.len(), CAPACITY + 2);
        for sequence in 0..(CAPACITY + 2) as u64 {
            let event = rx.recv_timeout(Duration::ZERO).unwrap();
            assert_eq!(event.sequence(), sequence);
            assert_eq!(event.is_terminal(), sequence == CAPACITY as u64 + 1);
        }
        assert_eq!(
            rx.recv_timeout(Duration::ZERO),
            Err(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn slow_ndjson_reader_disconnect_cancellation_and_full_queue_join_keep_sequence_and_terminal() {
        let cancel = Arc::new(FlagCancelObserver::new());
        let (tx, rx) = channel(cancel.clone());
        for sequence in 0..CAPACITY as u64 {
            tx.send(record(sequence)).unwrap();
        }
        assert_eq!(rx.0.state.lock().unwrap().records.len(), CAPACITY);
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let join = std::thread::spawn(move || {
            tx.send(record(CAPACITY as u64)).unwrap();
            tx.send(StatusEventV1::Terminal {
                schema_version: 1,
                operation_id: "bounded-test".into(),
                sequence: CAPACITY as u64 + 1,
                emitted_at_unix_ms: 1,
                data: StatusTerminalV1 {
                    reason: TerminalReason::Canceled,
                    error_code: None,
                },
            })
            .unwrap();
            done_tx.send(()).unwrap();
        });
        assert!(done_rx.recv_timeout(Duration::from_millis(30)).is_err());
        cancel.request_cancel();
        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        join.join().unwrap();
        for sequence in 0..(CAPACITY + 2) as u64 {
            let event = rx.recv_timeout(Duration::ZERO).unwrap();
            assert_eq!(event.sequence(), sequence);
            assert_eq!(event.is_terminal(), sequence == CAPACITY as u64 + 1);
        }
        drop(rx);
        let (tx, rx) = channel(Arc::new(FlagCancelObserver::new()));
        for sequence in 0..CAPACITY as u64 {
            tx.send(record(sequence)).unwrap();
        }
        let (closed_tx, closed_rx) = std::sync::mpsc::channel();
        let blocked = std::thread::spawn(move || {
            closed_tx.send(tx.send(record(CAPACITY as u64))).unwrap();
        });
        drop(rx);
        assert_eq!(
            closed_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
            Err(StatusError::ChannelClosed)
        );
        blocked.join().unwrap();
    }

    #[test]
    fn closing_stalled_receiver_unblocks_pending_send() {
        let (tx, rx) = channel(Arc::new(FlagCancelObserver::new()));
        for sequence in 0..CAPACITY as u64 {
            tx.send(record(sequence)).unwrap();
        }
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let join = std::thread::spawn(move || {
            done_tx.send(tx.send(record(16))).unwrap();
        });
        drop(rx);
        assert_eq!(
            done_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
            Err(StatusError::ChannelClosed)
        );
        join.join().unwrap();
    }
}
