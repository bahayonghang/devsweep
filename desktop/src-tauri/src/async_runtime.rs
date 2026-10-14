//! Async command dispatch has a small pool; blocking domain jobs keep Tokio's
//! separate blocking executor. One worker per host CPU inflated native idle.

pub(crate) fn build() -> std::io::Result<tokio::runtime::Runtime> {
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(4);
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn bounded_dispatch_runs_while_a_domain_job_waits_on_the_blocking_executor() {
        let runtime = super::build().unwrap();
        assert!((1..=4).contains(&runtime.metrics().num_workers()));
        let (started, observed) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let job = runtime.spawn_blocking(move || {
            started.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        let dispatch = runtime.spawn(async { "dispatched" });
        assert_eq!(runtime.block_on(dispatch).unwrap(), "dispatched");
        release.send(()).unwrap();
        runtime.block_on(job).unwrap();
    }
}
