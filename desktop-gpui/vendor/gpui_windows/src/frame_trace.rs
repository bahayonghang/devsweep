use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

const FRAME_TRACE_ENV: &str = "DEVSWEEP_FRAME_TRACE";

/// Opt-in present timestamps. The env path is read once, in [`FrameTrace::from_env`].
/// Each line is one integer microsecond from the first `Present` return, not `SyncQPCTime`.
pub(crate) struct FrameTrace {
    file: Option<BufWriter<std::fs::File>>,
    origin: Option<Instant>,
}

impl FrameTrace {
    pub(crate) fn from_env() -> Self {
        let path = std::env::var_os(FRAME_TRACE_ENV).filter(|path| !path.is_empty());
        let file = path.and_then(|path| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(Path::new(&path))
                .ok()
                .map(BufWriter::new)
        });
        Self {
            file,
            origin: None,
        }
    }

    pub(crate) fn record_at(&mut self, now: Instant) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let origin = *self.origin.get_or_insert(now);
        let micros = now.saturating_duration_since(origin).as_micros();
        if writeln!(file, "{micros}").is_ok() {
            let _ = file.flush();
        }
    }
}

impl Drop for FrameTrace {
    fn drop(&mut self) {
        if let Some(file) = self.file.as_mut() {
            let _ = file.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;

    static TRACE_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_trace_env<T>(value: Option<&std::ffi::OsStr>, body: impl FnOnce() -> T) -> T {
        let _guard = TRACE_ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let previous = std::env::var_os(FRAME_TRACE_ENV);
        unsafe {
            match value {
                Some(value) => std::env::set_var(FRAME_TRACE_ENV, value),
                None => std::env::remove_var(FRAME_TRACE_ENV),
            }
        }
        let result = body();
        unsafe {
            match previous {
                Some(value) => std::env::set_var(FRAME_TRACE_ENV, value),
                None => std::env::remove_var(FRAME_TRACE_ENV),
            }
        }
        result
    }

    #[test]
    fn frame_trace_writes_one_timestamp_per_record() {
        let path = std::env::temp_dir().join(format!(
            "devsweep-frame-trace-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        with_trace_env(Some(path.as_os_str()), || {
            let mut trace = FrameTrace::from_env();
            let start = Instant::now();
            trace.record_at(start);
            trace.record_at(start + Duration::from_micros(1500));
            drop(trace);
            let text = std::fs::read_to_string(&path).expect("trace file");
            assert_eq!(text, "0\n1500\n");
        });
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn frame_trace_writes_nothing_when_env_unset() {
        let sentinel = std::env::temp_dir().join(format!(
            "devsweep-frame-trace-unset-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&sentinel);
        with_trace_env(None, || {
            let mut trace = FrameTrace::from_env();
            assert!(trace.file.is_none());
            let start = Instant::now();
            trace.record_at(start);
            trace.record_at(start + Duration::from_micros(10));
            drop(trace);
        });
        assert!(
            !sentinel.exists(),
            "unset DEVSWEEP_FRAME_TRACE must not create a trace file"
        );
    }
}
