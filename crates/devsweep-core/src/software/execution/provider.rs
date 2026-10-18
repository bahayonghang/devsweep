//! Fixture process start for MSI, vendor, and Scoop.
//!
//! The child is one direct executable plus a separate argument list. This
//! module does not call the Windows action helper, does not invoke a shell,
//! and does not delete an installation directory.

use std::{
    io,
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use super::{AdapterEvidence, RemovalTiming, SoftwareAuditErrorCode, ValidatedSoftwareAction};
#[cfg(test)]
use super::{SoftwareExecutionOutcome, SoftwareInstalledState, classify_terminal};
use crate::process::FlagCancelObserver;
use crate::software::FixtureLaunch;
use crate::software::launch;
#[cfg(test)]
use crate::software::launch::LaunchRefusal;

pub(super) struct RealFixtureAdapter;

impl super::FixtureRemovalAdapter for RealFixtureAdapter {
    fn start(
        &self,
        action: &ValidatedSoftwareAction,
        _cancel: Option<&Arc<FlagCancelObserver>>,
        timing: RemovalTiming,
    ) -> AdapterEvidence {
        let Some(launch) = action.launch() else {
            return AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterDispatchFailed);
        };
        run_fixture_launch(launch, timing.monitor_timeout)
    }
}

/// Runs one fixture tool. Exit status alone is not a removal result.
pub(super) fn run_fixture_launch(launch: &FixtureLaunch, timeout: Duration) -> AdapterEvidence {
    let executable = match launch::prepare_fixture_start(launch) {
        Ok(path) => path,
        Err(_) => return AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterDispatchFailed),
    };
    match spawn_once(&executable, &launch.arguments, &launch.root, timeout) {
        Ok(Some(0)) => AdapterEvidence::success(),
        Ok(Some(3010 | 3011)) => AdapterEvidence::reboot_required(),
        Ok(Some(_)) => AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
        Ok(None) => AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterTimedOut),
        Err(_) => AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterDispatchFailed),
    }
}

/// Exit code 0 and an opened window never authorize removal by themselves.
#[cfg(test)]
pub(super) fn outcome_from_process_signal(
    exit_code: Option<i32>,
    window_opened: bool,
    installed: SoftwareInstalledState,
) -> SoftwareExecutionOutcome {
    let adapter = match exit_code {
        Some(3010 | 3011) => AdapterEvidence::reboot_required(),
        Some(0) => AdapterEvidence::success(),
        Some(_) => AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed),
        None if window_opened => AdapterEvidence::success(),
        None => AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterStatusUnavailable),
    };
    classify_terminal(adapter, installed)
}

#[cfg(test)]
pub(super) fn refusal_starts_process(_: LaunchRefusal) -> bool {
    false
}

fn spawn_once(
    executable: &Path,
    arguments: &[String],
    root: &Path,
    timeout: Duration,
) -> io::Result<Option<i32>> {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn()?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.code());
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(10));
    }
}
