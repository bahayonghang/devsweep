use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
    sync::OnceLock,
};

use windows::Win32::{
    Graphics::Dxgi::{DXGI_FRAME_STATISTICS, IDXGISwapChain1},
    System::Performance::QueryPerformanceFrequency,
};

const FRAME_STATS_ENV: &str = "DEVSWEEP_FRAME_STATS";
const FLUSH_EVERY: u8 = 8;

/// Opt-in DXGI `GetFrameStatistics` clock. This is not the `Present()` return
/// timestamp written by [`crate::frame_trace::FrameTrace`].
pub(crate) struct FrameStats {
    file: Option<BufWriter<std::fs::File>>,
    origin_qpc: Option<i64>,
    last_written_qpc: Option<i64>,
    pending_lines: u8,
}

impl FrameStats {
    pub(crate) fn from_env() -> Self {
        let path = std::env::var_os(FRAME_STATS_ENV).filter(|path| !path.is_empty());
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
            origin_qpc: None,
            last_written_qpc: None,
            pending_lines: 0,
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.file.is_some()
    }

    /// `None` means `GetFrameStatistics` failed. `0` and a repeat of the last
    /// written `SyncQPCTime` write nothing.
    pub(crate) fn record_qpc(&mut self, qpc: Option<i64>) {
        let Some(qpc) = qpc else {
            return;
        };
        self.record_qpc_at(qpc, qpc_frequency());
    }

    fn record_qpc_at(&mut self, qpc: i64, frequency: u64) {
        if self.file.is_none() || qpc == 0 || self.last_written_qpc == Some(qpc) || frequency == 0
        {
            return;
        }
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let origin = *self.origin_qpc.get_or_insert(qpc);
        let micros = qpc_delta_micros(origin, qpc, frequency);
        let _ = writeln!(file, "{micros}");
        self.last_written_qpc = Some(qpc);
        self.pending_lines = self.pending_lines.saturating_add(1);
        if self.pending_lines >= FLUSH_EVERY {
            let _ = file.flush();
            self.pending_lines = 0;
        }
    }
}

impl Drop for FrameStats {
    fn drop(&mut self) {
        if let Some(file) = self.file.as_mut() {
            let _ = file.flush();
        }
    }
}

pub(crate) fn sync_qpc_time(swap_chain: &IDXGISwapChain1) -> Option<i64> {
    let mut stats = DXGI_FRAME_STATISTICS::default();
    unsafe { swap_chain.GetFrameStatistics(&mut stats) }.ok()?;
    Some(stats.SyncQPCTime)
}

fn qpc_frequency() -> u64 {
    static FREQUENCY: OnceLock<u64> = OnceLock::new();
    *FREQUENCY.get_or_init(|| {
        let mut frequency = 0;
        // On systems that run Windows XP or later, the function always succeeds.
        unsafe { QueryPerformanceFrequency(&mut frequency).unwrap() };
        frequency as u64
    })
}

fn qpc_delta_micros(origin: i64, qpc: i64, frequency: u64) -> u64 {
    let delta = qpc.saturating_sub(origin).max(0) as u64;
    let micros = (delta as u128).saturating_mul(1_000_000) / u128::from(frequency);
    micros as u64
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static STATS_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_stats_env<T>(value: Option<&std::ffi::OsStr>, body: impl FnOnce() -> T) -> T {
        let _guard = STATS_ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let previous = std::env::var_os(FRAME_STATS_ENV);
        unsafe {
            match value {
                Some(value) => std::env::set_var(FRAME_STATS_ENV, value),
                None => std::env::remove_var(FRAME_STATS_ENV),
            }
        }
        let result = body();
        unsafe {
            match previous {
                Some(value) => std::env::set_var(FRAME_STATS_ENV, value),
                None => std::env::remove_var(FRAME_STATS_ENV),
            }
        }
        result
    }

    #[test]
    fn frame_stats_writes_micros_since_first_sync_qpc() {
        let path = std::env::temp_dir().join(format!(
            "devsweep-frame-stats-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        with_stats_env(Some(path.as_os_str()), || {
            let mut stats = FrameStats::from_env();
            let frequency = 10_000_000;
            stats.record_qpc_at(0, frequency);
            stats.record_qpc_at(1_000_000, frequency);
            stats.record_qpc_at(1_000_000, frequency);
            stats.record_qpc_at(0, frequency);
            stats.record_qpc_at(1_050_000, frequency);
            drop(stats);
            let text = std::fs::read_to_string(&path).expect("stats file");
            assert_eq!(text, "0\n5000\n");
        });
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn frame_stats_unset_does_not_query_or_write() {
        let sentinel = std::env::temp_dir().join(format!(
            "devsweep-frame-stats-unset-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&sentinel);
        with_stats_env(None, || {
            let mut stats = FrameStats::from_env();
            assert!(!stats.is_enabled());
            let mut queries = 0;
            let qpc = if stats.is_enabled() {
                queries += 1;
                Some(1_000_000)
            } else {
                None
            };
            stats.record_qpc(qpc);
            drop(stats);
            assert_eq!(queries, 0, "unset DEVSWEEP_FRAME_STATS must not query");
        });
        assert!(
            !sentinel.exists(),
            "unset DEVSWEEP_FRAME_STATS must not create a stats file"
        );
    }
}
