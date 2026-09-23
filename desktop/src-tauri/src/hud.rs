//! Tray HUD sampler. It samples Status every 2 s only while the HUD window is
//! visible. Hide and app exit cancel and join the sampler thread, so no sample
//! is emitted after `stop` returns.

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use devsweep_core::{
    process::{CancelObserver, FlagCancelObserver},
    status::StatusSnapshotV1,
};
use serde::Serialize;

/// Label of the lazily created HUD window.
pub(crate) const HUD_WINDOW_LABEL: &str = "hud";
/// Event name for HUD samples. Only the HUD window receives it.
pub(crate) const HUD_STATUS_EVENT: &str = "hud-status";
/// HUD sample cadence.
pub(crate) const HUD_SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

/// Closed HUD event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum HudStatusEvent {
    /// The sampler started; values from an earlier show are stale.
    Sampling,
    /// One Status snapshot.
    Snapshot { snapshot: Box<StatusSnapshotV1> },
}

struct RunningSampler {
    cancel: Arc<FlagCancelObserver>,
    join: JoinHandle<()>,
}

/// Owns at most one HUD sampler thread.
pub(crate) struct HudSampler {
    interval: Duration,
    running: Mutex<Option<RunningSampler>>,
}

impl Default for HudSampler {
    fn default() -> Self {
        Self::with_interval(HUD_SAMPLE_INTERVAL)
    }
}

impl HudSampler {
    pub(crate) fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            running: Mutex::new(None),
        }
    }

    /// Start the sampler thread unless one is already running. `sample`
    /// returns `None` when the sample was canceled or failed; nothing is
    /// emitted for it. Returns `true` when a new thread started.
    pub(crate) fn start<S, E>(&self, mut sample: S, mut emit: E) -> bool
    where
        S: FnMut(&FlagCancelObserver) -> Option<StatusSnapshotV1> + Send + 'static,
        E: FnMut(HudStatusEvent) + Send + 'static,
    {
        let Ok(mut running) = self.running.lock() else {
            return false;
        };
        if running.is_some() {
            return false;
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        let worker = Arc::clone(&cancel);
        let interval = self.interval;
        let spawned = thread::Builder::new()
            .name("devsweep-hud-sampler".into())
            .spawn(move || {
                emit(HudStatusEvent::Sampling);
                loop {
                    if worker.is_cancel_requested() {
                        return;
                    }
                    let started = Instant::now();
                    if let Some(snapshot) = sample(&worker) {
                        if worker.is_cancel_requested() {
                            return;
                        }
                        emit(HudStatusEvent::Snapshot {
                            snapshot: Box::new(snapshot),
                        });
                    }
                    if wait_until(started + interval, &worker) {
                        return;
                    }
                }
            });
        match spawned {
            Ok(join) => {
                *running = Some(RunningSampler { cancel, join });
                true
            }
            Err(_) => false,
        }
    }

    /// Cancel and join the sampler thread. Safe to call when stopped.
    pub(crate) fn stop(&self) {
        let running = self
            .running
            .lock()
            .map(|mut running| running.take())
            .unwrap_or(None);
        if let Some(running) = running {
            running.cancel.request_cancel();
            let _ = running.join.join();
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        self.running
            .lock()
            .map(|running| running.is_some())
            .unwrap_or(false)
    }
}

impl Drop for HudSampler {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Wait until `deadline` or cancellation. Returns `true` when canceled.
fn wait_until(deadline: Instant, cancel: &FlagCancelObserver) -> bool {
    loop {
        if cancel.is_cancel_requested() {
            return true;
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        thread::sleep((deadline - now).min(Duration::from_millis(10)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fixture_snapshot() -> StatusSnapshotV1 {
        serde_json::from_str(include_str!("../../src/api/fixtures/status/snapshot.json"))
            .expect("status fixture decodes")
    }

    fn counting_sampler(
        sampler: &HudSampler,
    ) -> (Arc<AtomicUsize>, Arc<Mutex<Vec<HudStatusEvent>>>) {
        let samples = Arc::new(AtomicUsize::new(0));
        let events = Arc::new(Mutex::new(Vec::new()));
        let sample_count = Arc::clone(&samples);
        let emitted = Arc::clone(&events);
        let snapshot = fixture_snapshot();
        assert!(sampler.start(
            move |_| {
                sample_count.fetch_add(1, Ordering::SeqCst);
                Some(snapshot.clone())
            },
            move |event| emitted.lock().unwrap().push(event),
        ));
        (samples, events)
    }

    fn wait_for(condition: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !condition() {
            assert!(Instant::now() < deadline, "condition not reached in 5 s");
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn show_starts_sampling_and_emits_sampling_then_snapshots() {
        let sampler = HudSampler::with_interval(Duration::from_millis(20));
        let (samples, events) = counting_sampler(&sampler);
        assert!(sampler.is_running());
        wait_for(|| samples.load(Ordering::SeqCst) >= 3);
        sampler.stop();
        let events = events.lock().unwrap();
        assert_eq!(events[0], HudStatusEvent::Sampling);
        assert!(
            events[1..]
                .iter()
                .all(|event| matches!(event, HudStatusEvent::Snapshot { .. }))
        );
    }

    #[test]
    fn hide_cancels_and_joins_and_no_sample_follows() {
        let sampler = HudSampler::with_interval(Duration::from_millis(10));
        let (samples, events) = counting_sampler(&sampler);
        wait_for(|| samples.load(Ordering::SeqCst) >= 2);
        sampler.stop();
        assert!(!sampler.is_running());
        let sampled = samples.load(Ordering::SeqCst);
        let emitted = events.lock().unwrap().len();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(samples.load(Ordering::SeqCst), sampled);
        assert_eq!(events.lock().unwrap().len(), emitted);
    }

    #[test]
    fn the_default_cadence_is_two_seconds_and_waits_between_samples() {
        assert_eq!(HudSampler::default().interval, Duration::from_secs(2));
        let sampler = HudSampler::default();
        let (samples, _events) = counting_sampler(&sampler);
        wait_for(|| samples.load(Ordering::SeqCst) >= 1);
        thread::sleep(Duration::from_millis(200));
        assert_eq!(samples.load(Ordering::SeqCst), 1);
        let started = Instant::now();
        sampler.stop();
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "stop cancels the 2 s wait instead of sleeping through it"
        );
    }

    #[test]
    fn a_second_show_does_not_start_a_second_thread() {
        let sampler = HudSampler::with_interval(Duration::from_millis(10));
        let (_samples, _events) = counting_sampler(&sampler);
        assert!(!sampler.start(|_| None, |_| {}));
        sampler.stop();
        sampler.stop();
        assert!(sampler.start(|_| None, |_| {}));
        sampler.stop();
    }

    #[test]
    fn app_exit_joins_the_sampler_through_drop() {
        let sampler = HudSampler::with_interval(Duration::from_millis(10));
        let (samples, _events) = counting_sampler(&sampler);
        wait_for(|| samples.load(Ordering::SeqCst) >= 1);
        drop(sampler);
        let sampled = samples.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(60));
        assert_eq!(samples.load(Ordering::SeqCst), sampled);
    }

    #[test]
    fn a_canceled_sample_emits_nothing() {
        let sampler = HudSampler::with_interval(Duration::from_millis(10));
        let events = Arc::new(Mutex::new(Vec::new()));
        let emitted = Arc::clone(&events);
        let entered = Arc::new(AtomicUsize::new(0));
        let entered_sample = Arc::clone(&entered);
        let snapshot = fixture_snapshot();
        assert!(sampler.start(
            move |cancel| {
                entered_sample.fetch_add(1, Ordering::SeqCst);
                while !cancel.is_cancel_requested() {
                    thread::sleep(Duration::from_millis(2));
                }
                Some(snapshot.clone())
            },
            move |event| emitted.lock().unwrap().push(event),
        ));
        wait_for(|| entered.load(Ordering::SeqCst) == 1);
        sampler.stop();
        assert_eq!(*events.lock().unwrap(), vec![HudStatusEvent::Sampling]);
    }
}
