//! One-shot spawn for the Windows action helper.
//!
//! The child receives a sealed frame on stdin. It has no caller argv and no
//! shell. A timeout kills that process once; this function does not spawn again.

use std::{
    io::{self, Write},
    path::Path,
    process::{ChildStdin, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// Process facts observed for one helper attempt.
#[derive(Debug)]
pub(crate) struct HelperRun {
    pub timed_out: bool,
    pub exit_code: Option<i32>,
    pub stdin_broken: bool,
}

/// Spawns `program` once, writes `frame`, and waits until exit or `timeout`.
///
/// `extra_stdin` is appended only so a child that never reads can fill the
/// pipe and surface a broken-pipe write. It is not an argument vector.
pub(crate) fn run_helper(
    program: &Path,
    frame: &[u8],
    extra_stdin: &[u8],
    timeout: Duration,
    fault: Option<&str>,
    gate_dir: Option<&Path>,
) -> io::Result<HelperRun> {
    let mut command = Command::new(program);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(std::env::temp_dir())
        .env_remove(FAULT_ENV)
        .env_remove(GATE_ENV);
    if let Some(value) = fault {
        command.env(FAULT_ENV, value);
    }
    if let Some(path) = gate_dir {
        command.env(GATE_ENV, path);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take();
    let started = Instant::now();
    let write_result = write_frame(stdin.as_mut(), frame, extra_stdin);
    drop(stdin);
    let stdin_broken = match &write_result {
        Ok(()) => false,
        Err(error) if is_broken_pipe(error) => true,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(error.kind(), "helper stdin write failed"));
        }
    };

    let remaining = timeout.saturating_sub(started.elapsed());
    let (timed_out, exit_code) = wait_child(&mut child, remaining);
    Ok(HelperRun {
        timed_out,
        exit_code,
        stdin_broken,
    })
}

pub(crate) const FAULT_ENV: &str = "DEVSWEEP_WA_FAULT";
pub(crate) const GATE_ENV: &str = "DEVSWEEP_WA_GATE";

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn write_frame(stdin: Option<&mut ChildStdin>, frame: &[u8], extra: &[u8]) -> io::Result<()> {
    let Some(stdin) = stdin else {
        return Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "helper stdin is closed",
        ));
    };
    stdin.write_all(frame)?;
    if !extra.is_empty() {
        stdin.write_all(extra)?;
    }
    stdin.flush()
}

fn wait_child(child: &mut std::process::Child, timeout: Duration) -> (bool, Option<i32>) {
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let exit_code = child.wait().ok().and_then(|status| status.code());
            return (true, exit_code);
        }
        match child.try_wait() {
            Ok(Some(status)) => return (false, status.code()),
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => {
                let _ = child.kill();
                let exit_code = child.wait().ok().and_then(|status| status.code());
                return (false, exit_code);
            }
        }
    }
}

fn is_broken_pipe(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::BrokenPipe
        || matches!(error.raw_os_error(), Some(109 | 232 | 233))
}
