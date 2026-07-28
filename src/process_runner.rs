//! Bounded external-process runner with timeout, output caps, and tree kill.

use std::{
    collections::VecDeque,
    env,
    ffi::OsString,
    fs,
    io::Read,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

/// Observes cooperative cancellation without owning a shared token type.
/// Real tokens are provided later by the true-cancellation task.
pub trait CancelObserver: Send + Sync {
    fn is_cancel_requested(&self) -> bool;
}

/// Default observer that never requests cancellation.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCancelObserver;

impl CancelObserver for NoopCancelObserver {
    fn is_cancel_requested(&self) -> bool {
        false
    }
}

/// Working-directory policy for a process request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwdPolicy {
    /// Create and use a unique directory under the system temp dir.
    Neutral,
    /// Caller-selected directory with an explicit reason for auditability.
    Explicit { path: PathBuf, reason: String },
}

/// Per-run capture and termination policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessPolicy {
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
    pub termination_grace: Duration,
    pub poll_interval: Duration,
}

impl Default for ProcessPolicy {
    fn default() -> Self {
        Self {
            max_stdout_bytes: DEFAULT_STREAM_CAP_BYTES,
            max_stderr_bytes: DEFAULT_STREAM_CAP_BYTES,
            termination_grace: Duration::from_secs(2),
            poll_interval: Duration::from_millis(20),
        }
    }
}

/// Default per-stream retained tail (1 MiB).
pub const DEFAULT_STREAM_CAP_BYTES: usize = 1024 * 1024;

/// Default provider probe timeout (within the 3–10s SLO band).
pub const DEFAULT_PROVIDER_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Hard cap for the global provider probe phase.
pub const DEFAULT_PROVIDER_PHASE_DEADLINE: Duration = Duration::from_secs(30);

/// Hard cap for executor-backed cleanup commands.
pub const DEFAULT_EXECUTOR_COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

/// Request describing one external command invocation.
pub struct ProcessRequest<'a> {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub cwd: CwdPolicy,
    pub timeout: Option<Duration>,
    pub job_deadline: Option<Instant>,
    pub cancel: &'a dyn CancelObserver,
}

/// Typed process completion status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessStatus {
    Success,
    NotFound,
    Timeout,
    Exit { code: Option<i32> },
    InvalidOutput,
    Canceled,
}

/// Bounded stdout/stderr capture with truncation metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub total_stdout_bytes: u64,
    pub total_stderr_bytes: u64,
}

/// Full runner result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessResult {
    pub status: ProcessStatus,
    pub output: ProcessOutput,
}

impl ProcessResult {
    fn empty(status: ProcessStatus) -> Self {
        Self {
            status,
            output: ProcessOutput::default(),
        }
    }
}

/// Portable process runner port used by providers and the executor.
#[derive(Debug, Clone)]
pub struct ProcessRunner {
    policy: ProcessPolicy,
}

impl Default for ProcessRunner {
    fn default() -> Self {
        Self::new(ProcessPolicy::default())
    }
}

impl ProcessRunner {
    pub fn new(policy: ProcessPolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &ProcessPolicy {
        &self.policy
    }

    pub fn run(&self, request: &ProcessRequest<'_>) -> ProcessResult {
        if request.cancel.is_cancel_requested() {
            return ProcessResult::empty(ProcessStatus::Canceled);
        }

        let effective_deadline = effective_deadline(request.timeout, request.job_deadline);
        if effective_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return ProcessResult::empty(ProcessStatus::Timeout);
        }

        let (cwd_path, neutral_dir) = match resolve_cwd(&request.cwd) {
            Ok(paths) => paths,
            Err(_) => return ProcessResult::empty(ProcessStatus::InvalidOutput),
        };

        let mut command = Command::new(&request.program);
        command.args(&request.args);
        command.current_dir(&cwd_path);
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut tree = ProcessTreeBackend::default();
        tree.configure_command(&mut command);

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                cleanup_neutral_dir(neutral_dir.as_ref());
                return ProcessResult::empty(ProcessStatus::NotFound);
            }
            Err(_) => {
                cleanup_neutral_dir(neutral_dir.as_ref());
                return ProcessResult::empty(ProcessStatus::InvalidOutput);
            }
        };

        if tree.attach(&child).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            cleanup_neutral_dir(neutral_dir.as_ref());
            return ProcessResult::empty(ProcessStatus::InvalidOutput);
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdout_cap = self.policy.max_stdout_bytes;
        let stderr_cap = self.policy.max_stderr_bytes;
        let (stdout_tx, stdout_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();

        let stdout_thread = stdout.map(|pipe| {
            thread::spawn(move || {
                let capture = read_bounded(pipe, stdout_cap);
                let _ = stdout_tx.send(capture);
            })
        });
        let stderr_thread = stderr.map(|pipe| {
            thread::spawn(move || {
                let capture = read_bounded(pipe, stderr_cap);
                let _ = stderr_tx.send(capture);
            })
        });

        let status = loop {
            if request.cancel.is_cancel_requested() {
                break ProcessStatus::Canceled;
            }
            if effective_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                break ProcessStatus::Timeout;
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    break if status.success() {
                        ProcessStatus::Success
                    } else {
                        ProcessStatus::Exit {
                            code: status.code(),
                        }
                    };
                }
                Ok(None) => thread::sleep(self.policy.poll_interval),
                Err(_) => break ProcessStatus::InvalidOutput,
            }
        };
        let needs_terminate = matches!(
            status,
            ProcessStatus::Timeout | ProcessStatus::Canceled | ProcessStatus::InvalidOutput
        );
        if needs_terminate {
            tree.terminate(&mut child);
            wait_for_exit(&mut child, self.policy.termination_grace);
        } else {
            // Process already exited; still wait to reap.
            let _ = child.wait();
        }
        // Dropping the backend closes platform handles (Job Object kill-on-close
        // safety net for any stragglers).
        drop(tree);

        let stdout_capture =
            collect_capture(stdout_rx, stdout_thread, self.policy.termination_grace);
        let stderr_capture =
            collect_capture(stderr_rx, stderr_thread, self.policy.termination_grace);

        if request.cancel.is_cancel_requested() && matches!(status, ProcessStatus::Success) {
            cleanup_neutral_dir(neutral_dir.as_ref());
            return ProcessResult {
                status: ProcessStatus::Canceled,
                output: ProcessOutput {
                    stdout: stdout_capture.bytes,
                    stderr: stderr_capture.bytes,
                    stdout_truncated: stdout_capture.truncated,
                    stderr_truncated: stderr_capture.truncated,
                    total_stdout_bytes: stdout_capture.total,
                    total_stderr_bytes: stderr_capture.total,
                },
            };
        }

        cleanup_neutral_dir(neutral_dir.as_ref());
        ProcessResult {
            status,
            output: ProcessOutput {
                stdout: stdout_capture.bytes,
                stderr: stderr_capture.bytes,
                stdout_truncated: stdout_capture.truncated,
                stderr_truncated: stderr_capture.truncated,
                total_stdout_bytes: stdout_capture.total,
                total_stderr_bytes: stderr_capture.total,
            },
        }
    }
}

/// Convert process bytes into a display/audit-safe string.
///
/// Strips executable terminal control sequences, keeps a bounded tail, and
/// marks truncation. Non-UTF-8 bytes are lossy-decoded only here.
pub fn sanitize_process_output(bytes: &[u8], cap: usize) -> String {
    let lossy = String::from_utf8_lossy(bytes);
    let mut sanitized = String::with_capacity(lossy.len().min(cap.saturating_add(32)));
    let mut truncated = false;

    for ch in lossy.chars() {
        let piece = match ch {
            '\n' | '\r' | '\t' => ch.to_string(),
            c if c.is_control() || c == '\u{7f}' => format!("\\u{{{:x}}}", u32::from(c)),
            c => c.to_string(),
        };
        if sanitized.len() + piece.len() > cap {
            truncated = true;
            break;
        }
        sanitized.push_str(&piece);
    }

    if truncated || (cap > 0 && lossy.len() > sanitized.len()) {
        if sanitized.len() > cap {
            sanitized.truncate(cap);
        }
        sanitized.push_str("...[truncated]");
    } else if bytes.len() > lossy.len() {
        // Byte length can exceed char display length for multi-byte sequences;
        // still mark when the raw buffer was larger than the rendered form and
        // the caller asked for a tight cap relative to raw input.
    }

    if cap == 0 {
        return "...[truncated]".to_string();
    }

    sanitized
}

fn effective_deadline(timeout: Option<Duration>, job_deadline: Option<Instant>) -> Option<Instant> {
    let timeout_deadline = timeout.map(|duration| Instant::now() + duration);
    match (timeout_deadline, job_deadline) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn resolve_cwd(policy: &CwdPolicy) -> std::io::Result<(PathBuf, Option<PathBuf>)> {
    match policy {
        CwdPolicy::Neutral => {
            let mut dir = env::temp_dir();
            let unique = format!("devsweep-proc-{}-{}", std::process::id(), neutral_nonce());
            dir.push(unique);
            fs::create_dir_all(&dir)?;
            Ok((dir.clone(), Some(dir)))
        }
        CwdPolicy::Explicit { path, reason: _ } => Ok((path.clone(), None)),
    }
}

fn neutral_nonce() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NONCE: AtomicU64 = AtomicU64::new(1);
    Instant::now()
        .elapsed()
        .as_nanos()
        .wrapping_add(NONCE.fetch_add(1, Ordering::Relaxed) as u128) as u64
}

fn cleanup_neutral_dir(dir: Option<&PathBuf>) {
    if let Some(path) = dir {
        let _ = fs::remove_dir_all(path);
    }
}

#[derive(Debug, Default)]
struct StreamCapture {
    bytes: Vec<u8>,
    truncated: bool,
    total: u64,
}

fn read_bounded<R: Read>(mut reader: R, cap: usize) -> StreamCapture {
    let mut ring = ByteRing::new(cap);
    let mut buf = [0_u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => ring.push(&buf[..n]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    ring.into_capture()
}

#[derive(Debug)]
struct ByteRing {
    buf: VecDeque<u8>,
    cap: usize,
    total: u64,
    truncated: bool,
}

impl ByteRing {
    fn new(cap: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(cap.min(64 * 1024)),
            cap,
            total: 0,
            truncated: false,
        }
    }

    fn push(&mut self, data: &[u8]) {
        self.total = self.total.saturating_add(data.len() as u64);
        if self.cap == 0 {
            self.truncated = self.truncated || !data.is_empty();
            return;
        }
        if data.len() >= self.cap {
            self.truncated = true;
            self.buf.clear();
            self.buf
                .extend(data[data.len() - self.cap..].iter().copied());
            return;
        }
        let next_len = self.buf.len() + data.len();
        if next_len > self.cap {
            let drop_count = next_len - self.cap;
            self.buf.drain(..drop_count);
            self.truncated = true;
        }
        self.buf.extend(data.iter().copied());
    }

    fn into_capture(self) -> StreamCapture {
        StreamCapture {
            bytes: self.buf.into_iter().collect(),
            truncated: self.truncated,
            total: self.total,
        }
    }
}

fn collect_capture(
    rx: mpsc::Receiver<StreamCapture>,
    worker: Option<thread::JoinHandle<()>>,
    grace: Duration,
) -> StreamCapture {
    let deadline = Instant::now() + grace;
    let capture = loop {
        match rx.try_recv() {
            Ok(capture) => break capture,
            Err(mpsc::TryRecvError::Empty) => {
                if Instant::now() >= deadline {
                    break StreamCapture::default();
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(mpsc::TryRecvError::Disconnected) => break StreamCapture::default(),
        }
    };
    if let Some(handle) = worker {
        let _ = handle.join();
    }
    capture
}

fn wait_for_exit(child: &mut Child, grace: Duration) {
    let deadline = Instant::now() + grace;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
}

#[derive(Debug, Default)]
struct ProcessTreeBackend {
    #[cfg(windows)]
    job: Option<windows_sys::Win32::Foundation::HANDLE>,
    #[cfg(unix)]
    child_pid: Option<i32>,
}

impl ProcessTreeBackend {
    fn configure_command(&mut self, command: &mut Command) {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(|| {
                    if libc::setpgid(0, 0) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        #[cfg(windows)]
        {
            let _ = command;
            // Job assignment happens in attach() after spawn. CREATE_SUSPENDED
            // is avoided because std::process does not expose the primary
            // thread handle needed to resume cleanly.
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = command;
        }
    }

    fn attach(&mut self, child: &Child) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            windows_backend::attach_job(self, child)
        }
        #[cfg(unix)]
        {
            self.child_pid = Some(child.id() as i32);
            Ok(())
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = child;
            Ok(())
        }
    }

    fn terminate(&mut self, child: &mut Child) {
        #[cfg(windows)]
        {
            windows_backend::terminate_job(self);
            let _ = child.kill();
        }
        #[cfg(unix)]
        {
            if let Some(pid) = self.child_pid.take() {
                unsafe {
                    // Negative pid targets the process group created in pre_exec.
                    let _ = libc::kill(-pid, libc::SIGKILL);
                }
            }
            let _ = child.kill();
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = child.kill();
        }
    }
}

impl Drop for ProcessTreeBackend {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            windows_backend::close_job(self);
        }
    }
}

#[cfg(windows)]
mod windows_backend {
    use super::ProcessTreeBackend;
    use std::{io, mem, process::Child};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        },
    };

    pub(super) fn attach_job(backend: &mut ProcessTreeBackend, child: &Child) -> io::Result<()> {
        use std::os::windows::io::AsRawHandle;

        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() || job == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                let err = io::Error::last_os_error();
                CloseHandle(job);
                return Err(err);
            }

            let process_handle = child.as_raw_handle() as HANDLE;
            let ok = AssignProcessToJobObject(job, process_handle);
            if ok == 0 {
                let err = io::Error::last_os_error();
                CloseHandle(job);
                return Err(err);
            }

            backend.job = Some(job);
            Ok(())
        }
    }

    pub(super) fn terminate_job(backend: &mut ProcessTreeBackend) {
        if let Some(job) = backend.job.take() {
            unsafe {
                let _ = TerminateJobObject(job, 1);
                let _ = CloseHandle(job);
            }
        }
    }

    pub(super) fn close_job(backend: &mut ProcessTreeBackend) {
        if let Some(job) = backend.job.take() {
            unsafe {
                let _ = CloseHandle(job);
            }
        }
    }
}

/// Returns true when a process id appears still alive.
/// Used by dynamic tree-cleanup tests; failure to prove death is a test No-Go.
pub fn process_is_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        windows_process_is_alive(pid)
    }
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
        false
    }
}

#[cfg(windows)]
fn windows_process_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, STILL_ACTIVE, WAIT_TIMEOUT},
        System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            PROCESS_SYNCHRONIZE, WaitForSingleObject,
        },
    };

    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        );
        if handle.is_null() {
            return false;
        }
        let mut exit_code = 0_u32;
        let ok = GetExitCodeProcess(handle, &mut exit_code);
        let wait = WaitForSingleObject(handle, 0);
        CloseHandle(handle);
        ok != 0 && exit_code == STILL_ACTIVE as u32 && wait == WAIT_TIMEOUT
    }
}

/// Flag-based cancel observer for tests and future token adapters.
#[derive(Debug, Default)]
pub struct FlagCancelObserver {
    flag: AtomicBool,
}

impl FlagCancelObserver {
    pub fn new() -> Self {
        Self {
            flag: AtomicBool::new(false),
        }
    }

    pub fn request_cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn shared(self: &Arc<Self>) -> ArcCancelObserver {
        ArcCancelObserver {
            inner: Arc::clone(self),
        }
    }
}

impl CancelObserver for FlagCancelObserver {
    fn is_cancel_requested(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Arc-backed cancel observer for multi-thread tests.
#[derive(Debug, Clone)]
pub struct ArcCancelObserver {
    inner: Arc<FlagCancelObserver>,
}

impl CancelObserver for ArcCancelObserver {
    fn is_cancel_requested(&self) -> bool {
        self.inner.is_cancel_requested()
    }
}

/// Shared test helpers for locating the process_fixture binary.
#[cfg(test)]
pub mod test_support {
    use std::{env, ffi::OsString, path::PathBuf, process::Command, sync::Once};

    static ENSURE_FIXTURE: Once = Once::new();

    pub fn process_fixture_exe() -> PathBuf {
        ENSURE_FIXTURE.call_once(|| {
            let path = fixture_path();
            if path.is_file() {
                return;
            }
            let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
            let status = Command::new(cargo)
                .arg("build")
                .arg("--bin")
                .arg("process_fixture")
                .arg("--quiet")
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .status()
                .expect("spawn cargo build --bin process_fixture");
            assert!(
                status.success(),
                "cargo build --bin process_fixture failed with {status}"
            );
        });
        let path = fixture_path();
        assert!(
            path.is_file(),
            "process_fixture binary missing at {}",
            path.display()
        );
        path
    }

    fn fixture_path() -> PathBuf {
        let mut path = env::current_exe().expect("current test executable");
        // target/debug/deps/<test> -> target/debug/process_fixture.exe
        path.pop();
        if path.file_name().and_then(|name| name.to_str()) == Some("deps") {
            path.pop();
        }
        path.push(format!("process_fixture{}", env::consts::EXE_SUFFIX));
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, time::Duration};

    fn fixture_exe() -> PathBuf {
        crate::process_runner::test_support::process_fixture_exe()
    }

    fn runner_with(policy: ProcessPolicy) -> ProcessRunner {
        ProcessRunner::new(policy)
    }

    fn base_request<'a>(
        args: &[&str],
        cancel: &'a dyn CancelObserver,
        timeout: Duration,
    ) -> ProcessRequest<'a> {
        ProcessRequest {
            program: fixture_exe().into_os_string(),
            args: args.iter().map(OsString::from).collect(),
            cwd: CwdPolicy::Neutral,
            timeout: Some(timeout),
            job_deadline: None,
            cancel,
        }
    }

    #[test]
    fn hung_child_times_out_within_policy() {
        let cancel = NoopCancelObserver;
        let runner = runner_with(ProcessPolicy {
            termination_grace: Duration::from_secs(2),
            ..ProcessPolicy::default()
        });
        let started = Instant::now();
        let result = runner.run(&base_request(
            &["hang"],
            &cancel,
            Duration::from_millis(300),
        ));
        let elapsed = started.elapsed();

        assert_eq!(result.status, ProcessStatus::Timeout);
        assert!(
            elapsed < Duration::from_secs(3),
            "timeout should return promptly, took {elapsed:?}"
        );
    }

    #[test]
    fn hung_child_and_grandchild_are_terminated() {
        let cancel = NoopCancelObserver;
        let runner = runner_with(ProcessPolicy {
            termination_grace: Duration::from_secs(3),
            ..ProcessPolicy::default()
        });

        let pid_dir = env::temp_dir().join(format!("devsweep-tree-{}", std::process::id()));
        let _ = fs::create_dir_all(&pid_dir);
        let pid_file = pid_dir.join("grandchild.pid");
        let _ = fs::remove_file(&pid_file);

        let request = ProcessRequest {
            program: fixture_exe().into_os_string(),
            args: vec![
                OsString::from("hang-tree"),
                pid_file.as_os_str().to_os_string(),
            ],
            cwd: CwdPolicy::Neutral,
            timeout: Some(Duration::from_millis(500)),
            job_deadline: None,
            cancel: &cancel,
        };

        let result = runner.run(&request);
        assert_eq!(
            result.status,
            ProcessStatus::Timeout,
            "hung tree must time out"
        );

        // Wait until the fixture reports the grandchild pid.
        let mut grandchild_pid = None;
        let dig_deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < dig_deadline {
            if let Ok(text) = fs::read_to_string(&pid_file)
                && let Ok(pid) = text.trim().parse::<u32>()
            {
                grandchild_pid = Some(pid);
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }

        let grandchild_pid = grandchild_pid.expect(
            "No-Go: grandchild pid file was not written; cannot prove process-tree termination",
        );

        let dead_deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < dead_deadline {
            if !process_is_alive(grandchild_pid) {
                let _ = fs::remove_dir_all(&pid_dir);
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }

        let _ = fs::remove_dir_all(&pid_dir);
        panic!(
            "No-Go: grandchild pid {grandchild_pid} still alive after timeout grace; \
             process-tree backend failed to prove termination on this platform"
        );
    }

    #[test]
    fn endless_stdout_is_capped_with_truncation_mark() {
        let cancel = NoopCancelObserver;
        let runner = runner_with(ProcessPolicy {
            max_stdout_bytes: 64 * 1024,
            max_stderr_bytes: 64 * 1024,
            termination_grace: Duration::from_secs(2),
            ..ProcessPolicy::default()
        });
        let result = runner.run(&base_request(
            &["endless-stdout"],
            &cancel,
            Duration::from_millis(400),
        ));

        assert_eq!(result.status, ProcessStatus::Timeout);
        assert!(result.output.stdout_truncated);
        assert!(result.output.stdout.len() <= 64 * 1024);
        assert!(result.output.total_stdout_bytes as usize > result.output.stdout.len());
        assert!(result.output.stdout.iter().all(|b| *b == b'x'));
    }

    #[test]
    fn endless_stderr_is_capped_with_truncation_mark() {
        let cancel = NoopCancelObserver;
        let runner = runner_with(ProcessPolicy {
            max_stdout_bytes: 32 * 1024,
            max_stderr_bytes: 32 * 1024,
            termination_grace: Duration::from_secs(2),
            ..ProcessPolicy::default()
        });
        let result = runner.run(&base_request(
            &["endless-stderr"],
            &cancel,
            Duration::from_millis(400),
        ));

        assert_eq!(result.status, ProcessStatus::Timeout);
        assert!(result.output.stderr_truncated);
        assert!(result.output.stderr.len() <= 32 * 1024);
        assert!(result.output.total_stderr_bytes as usize > result.output.stderr.len());
    }

    #[test]
    fn non_utf8_and_nonzero_exit_are_typed_without_panic() {
        let cancel = NoopCancelObserver;
        let runner = ProcessRunner::default();

        let non_utf8 = runner.run(&base_request(
            &["non-utf8"],
            &cancel,
            Duration::from_secs(2),
        ));
        assert_eq!(non_utf8.status, ProcessStatus::Success);
        assert!(!non_utf8.output.stdout.is_empty());
        let sanitized = sanitize_process_output(&non_utf8.output.stderr, 1024);
        assert!(!sanitized.contains('\u{1b}'));

        let failed = runner.run(&base_request(
            &["exit", "7"],
            &cancel,
            Duration::from_secs(2),
        ));
        assert_eq!(failed.status, ProcessStatus::Exit { code: Some(7) });
    }

    #[test]
    fn not_found_program_returns_typed_status() {
        let cancel = NoopCancelObserver;
        let runner = ProcessRunner::default();
        let result = runner.run(&ProcessRequest {
            program: OsString::from("devsweep-definitely-missing-binary-xyz"),
            args: Vec::new(),
            cwd: CwdPolicy::Neutral,
            timeout: Some(Duration::from_secs(1)),
            job_deadline: None,
            cancel: &cancel,
        });
        assert_eq!(result.status, ProcessStatus::NotFound);
    }

    #[test]
    fn neutral_cwd_is_not_caller_project_dir() {
        let cancel = NoopCancelObserver;
        let runner = ProcessRunner::default();
        let project = env::temp_dir().join(format!("devsweep-npmrc-{}", std::process::id()));
        fs::create_dir_all(&project).expect("project dir");
        fs::write(project.join(".npmrc"), "cache=/should-not-apply\n").expect("npmrc");

        let previous = env::current_dir().expect("cwd");
        env::set_current_dir(&project).expect("enter project");
        let result = runner.run(&base_request(
            &["print-cwd"],
            &cancel,
            Duration::from_secs(2),
        ));
        env::set_current_dir(&previous).expect("restore cwd");
        let _ = fs::remove_dir_all(&project);

        assert_eq!(result.status, ProcessStatus::Success);
        let printed = String::from_utf8_lossy(&result.output.stdout);
        let printed_path = PathBuf::from(printed.trim());
        assert_ne!(
            printed_path, project,
            "neutral cwd must not inherit the project directory that contains .npmrc"
        );
        assert!(
            !printed_path.starts_with(&project),
            "neutral cwd must be outside the project tree"
        );
    }

    #[test]
    fn explicit_cwd_is_honored() {
        let cancel = NoopCancelObserver;
        let runner = ProcessRunner::default();
        let dir = env::temp_dir().join(format!("devsweep-explicit-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("explicit dir");

        let result = runner.run(&ProcessRequest {
            program: fixture_exe().into_os_string(),
            args: vec![OsString::from("print-cwd")],
            cwd: CwdPolicy::Explicit {
                path: dir.clone(),
                reason: "unit test explicit cwd".to_string(),
            },
            timeout: Some(Duration::from_secs(2)),
            job_deadline: None,
            cancel: &cancel,
        });
        let printed = String::from_utf8_lossy(&result.output.stdout);
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(result.status, ProcessStatus::Success);
        let printed_path = PathBuf::from(printed.trim());
        // Windows may return different prefix spellings; compare canonically when possible.
        let expected = dir.canonicalize().unwrap_or(dir);
        let actual = printed_path.canonicalize().unwrap_or(printed_path);
        assert_eq!(actual, expected);
    }

    #[test]
    fn sanitize_strips_control_sequences_and_marks_truncation() {
        let raw = b"\x1b[31m\x07red\x1b[0m and plain";
        let cleaned = sanitize_process_output(raw, 64);
        assert!(!cleaned.as_bytes().contains(&0x1b));
        assert!(!cleaned.as_bytes().contains(&0x07));
        assert!(cleaned.contains("red"));
        assert!(cleaned.contains("plain"));

        let long = vec![b'A'; 100];
        let truncated = sanitize_process_output(&long, 16);
        assert!(truncated.contains("...[truncated]"));
        assert!(truncated.len() <= 16 + "...[truncated]".len());
    }

    #[test]
    fn noop_cancel_preserves_success_path() {
        let cancel = NoopCancelObserver;
        let runner = ProcessRunner::default();
        let result = runner.run(&base_request(
            &["sleep-exit", "30"],
            &cancel,
            Duration::from_secs(2),
        ));
        assert_eq!(result.status, ProcessStatus::Success);
    }

    #[test]
    fn injected_cancel_observer_yields_canceled() {
        let flag = Arc::new(FlagCancelObserver::new());
        flag.request_cancel();
        let observer = flag.shared();
        let runner = ProcessRunner::default();
        let result = runner.run(&base_request(&["hang"], &observer, Duration::from_secs(5)));
        assert_eq!(result.status, ProcessStatus::Canceled);
    }

    #[test]
    fn cancel_during_run_terminates_and_returns_canceled() {
        let flag = Arc::new(FlagCancelObserver::new());
        let observer = flag.shared();
        let runner = runner_with(ProcessPolicy {
            poll_interval: Duration::from_millis(10),
            termination_grace: Duration::from_secs(2),
            ..ProcessPolicy::default()
        });

        let flag_for_thread = Arc::clone(&flag);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            flag_for_thread.request_cancel();
        });

        let result = runner.run(&base_request(&["hang"], &observer, Duration::from_secs(5)));
        assert_eq!(result.status, ProcessStatus::Canceled);
    }

    #[test]
    fn job_deadline_can_beat_per_command_timeout() {
        let cancel = NoopCancelObserver;
        let runner = ProcessRunner::default();
        let result = runner.run(&ProcessRequest {
            program: fixture_exe().into_os_string(),
            args: vec![OsString::from("hang")],
            cwd: CwdPolicy::Neutral,
            timeout: Some(Duration::from_secs(10)),
            job_deadline: Some(Instant::now() + Duration::from_millis(200)),
            cancel: &cancel,
        });
        assert_eq!(result.status, ProcessStatus::Timeout);
    }

    /// macOS dynamic verification row (process-group kill).
    ///
    /// On macOS/Linux this exercises the same Unix process-group backend.
    /// Compile-only evidence is insufficient; this test is the dynamic proof
    /// when run on a Unix host. Windows CI covers Job Objects separately via
    /// `hung_child_and_grandchild_are_terminated`.
    #[test]
    #[cfg(unix)]
    fn macos_linux_process_group_tree_termination_dynamic() {
        hung_child_and_grandchild_are_terminated();
    }
}
