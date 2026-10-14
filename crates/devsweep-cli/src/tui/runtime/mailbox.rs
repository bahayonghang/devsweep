//! One replaceable scan observation and a bounded, lossless lifecycle FIFO.

use super::WorkerEvent;
use std::{
    collections::VecDeque,
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{SendError, TryRecvError},
    },
};
const CAPACITY: usize = 32;

struct State {
    lifecycle: VecDeque<WorkerEvent>,
    scan: Option<WorkerEvent>,
    closed: bool,
    senders: usize,
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}
pub(super) struct WorkerSender(Arc<Shared>);
pub(super) struct WorkerReceiver(Arc<Shared>);

pub(super) fn channel() -> (WorkerSender, WorkerReceiver) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            lifecycle: VecDeque::new(),
            scan: None,
            closed: false,
            senders: 1,
        }),
        changed: Condvar::new(),
    });
    (WorkerSender(shared.clone()), WorkerReceiver(shared))
}

impl WorkerSender {
    pub(super) fn send(&self, event: WorkerEvent) -> Result<(), SendError<()>> {
        let mut state = self.0.state.lock().expect("worker mailbox lock");
        if state.closed {
            return Err(SendError(()));
        }
        let replaceable = matches!(event, WorkerEvent::ScanProgress { plan: Some(_), .. });
        if let WorkerEvent::ScanProgress {
            job_id,
            phase,
            plan: Some(_),
            ..
        } = &event
        {
            let same_phase = state.scan.as_ref().is_none_or(|previous| {
                matches!(previous,
                WorkerEvent::ScanProgress { job_id: prior_job, phase: prior_phase, .. }
                    if prior_job == job_id && prior_phase == phase)
            });
            if same_phase {
                state.scan = Some(event);
                self.0.changed.notify_all();
                return Ok(());
            }
        }
        // Wait releases the lock, so the UI can always drain a full FIFO.
        // Flush the last observation before this phase/lifecycle/final event.
        while state.lifecycle.len() + usize::from(state.scan.is_some()) >= CAPACITY && !state.closed
        {
            state = self.0.changed.wait(state).expect("worker mailbox wait");
        }
        if state.closed {
            return Err(SendError(()));
        }
        if let Some(previous) = state.scan.take() {
            state.lifecycle.push_back(previous);
        }
        if replaceable {
            state.scan = Some(event);
        } else {
            state.lifecycle.push_back(event);
        }
        self.0.changed.notify_all();
        Ok(())
    }
}

impl Clone for WorkerSender {
    fn clone(&self) -> Self {
        self.0.state.lock().expect("worker mailbox lock").senders += 1;
        Self(self.0.clone())
    }
}
impl Drop for WorkerSender {
    fn drop(&mut self) {
        self.0.state.lock().expect("worker mailbox lock").senders -= 1;
        self.0.changed.notify_all();
    }
}
impl WorkerReceiver {
    pub(super) fn try_recv(&self) -> Result<WorkerEvent, TryRecvError> {
        let mut state = self.0.state.lock().expect("worker mailbox lock");
        let event = state.lifecycle.pop_front().or_else(|| state.scan.take());
        self.0.changed.notify_all();
        event.ok_or(if state.senders == 0 {
            TryRecvError::Disconnected
        } else {
            TryRecvError::Empty
        })
    }
    #[cfg(test)]
    pub(super) fn try_iter(&self) -> impl Iterator<Item = WorkerEvent> + '_ {
        std::iter::from_fn(|| self.try_recv().ok())
    }
    #[cfg(test)]
    pub(super) fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<WorkerEvent, std::sync::mpsc::RecvTimeoutError> {
        use std::sync::mpsc::RecvTimeoutError;
        let deadline = std::time::Instant::now() + timeout;
        let mut state = self.0.state.lock().expect("worker mailbox lock");
        loop {
            if let Some(event) = state.lifecycle.pop_front().or_else(|| state.scan.take()) {
                self.0.changed.notify_all();
                return Ok(event);
            }
            if state.senders == 0 {
                return Err(RecvTimeoutError::Disconnected);
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(RecvTimeoutError::Timeout);
            }
            state = self
                .0
                .changed
                .wait_timeout(state, remaining)
                .expect("worker mailbox wait")
                .0;
        }
    }
}
impl Drop for WorkerReceiver {
    fn drop(&mut self) {
        self.0.state.lock().expect("worker mailbox lock").closed = true;
        self.0.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::CleanupPlan, scan::ScanPhase};
    use std::sync::mpsc;

    fn progress(index: usize, phase: ScanPhase) -> WorkerEvent {
        WorkerEvent::ScanProgress {
            job_id: 1,
            phase,
            message: index.to_string(),
            plan: Some(CleanupPlan::empty()),
        }
    }

    #[test]
    fn ten_thousand_snapshots_retain_one_and_flush_before_phase_and_terminal() {
        let (tx, rx) = channel();
        tx.send(WorkerEvent::ScanStarted { job_id: 1 }).unwrap();
        for i in 0..10_000 {
            tx.send(progress(i, ScanPhase::Projects)).unwrap();
        }
        assert_eq!(
            rx.0.state.lock().unwrap().scan.as_ref(),
            Some(&progress(9_999, ScanPhase::Projects))
        );
        tx.send(WorkerEvent::ScanProgress {
            job_id: 1,
            phase: ScanPhase::Global,
            message: "start".into(),
            plan: None,
        })
        .unwrap();
        tx.send(progress(10_000, ScanPhase::Global)).unwrap();
        tx.send(WorkerEvent::JobCanceled { job_id: 1 }).unwrap();
        let events: Vec<_> = rx.try_iter().collect();
        assert_eq!(events.len(), 5);
        assert_eq!(events[1], progress(9_999, ScanPhase::Projects));
        assert_eq!(events[3], progress(10_000, ScanPhase::Global));
        assert_eq!(events[4], WorkerEvent::JobCanceled { job_id: 1 });
    }

    #[test]
    fn receiver_close_unblocks_full_lifecycle_fifo() {
        let (tx, rx) = channel();
        for job_id in 0..CAPACITY as u64 {
            tx.send(WorkerEvent::JobCanceled { job_id }).unwrap();
        }
        let (done_tx, done_rx) = mpsc::channel();
        let join = std::thread::spawn(move || {
            done_tx
                .send(tx.send(WorkerEvent::JobCanceled { job_id: 100 }).is_err())
                .unwrap();
        });
        assert!(
            done_rx
                .recv_timeout(std::time::Duration::from_millis(30))
                .is_err()
        );
        drop(rx);
        assert!(
            done_rx
                .recv_timeout(std::time::Duration::from_secs(1))
                .unwrap()
        );
        join.join().unwrap();
    }

    #[test]
    fn full_fifo_keeps_all_lifecycle_records_and_final_in_order() {
        let (tx, rx) = channel();
        for job_id in 0..CAPACITY as u64 {
            tx.send(WorkerEvent::ScanStarted { job_id }).unwrap();
        }
        let join = std::thread::spawn(move || {
            for job_id in CAPACITY as u64..100 {
                tx.send(WorkerEvent::ScanStarted { job_id }).unwrap();
            }
            tx.send(WorkerEvent::JobCanceled { job_id: 100 }).unwrap();
        });
        assert_eq!(rx.0.state.lock().unwrap().lifecycle.len(), CAPACITY);
        for job_id in 0..100 {
            assert_eq!(
                rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap(),
                WorkerEvent::ScanStarted { job_id }
            );
        }
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap(),
            WorkerEvent::JobCanceled { job_id: 100 }
        );
        join.join().unwrap();
    }
}
