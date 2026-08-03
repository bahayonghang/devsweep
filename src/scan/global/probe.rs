use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use tracing::warn;

use crate::{
    filesystem::{SizeEstimate, estimate_tree_with_budget_and_cancel},
    process::{
        CancelObserver, CwdPolicy, DEFAULT_PROVIDER_PHASE_DEADLINE, DEFAULT_PROVIDER_PROBE_TIMEOUT,
        FlagCancelObserver, NoopCancelObserver, ProcessRequest, ProcessRunner, ProcessStatus,
    },
};

pub(super) trait ProviderProbe {
    fn resolve_executable(&self, program: &str) -> Option<PathBuf>;
    fn command_output(&self, program: &Path, args: &[&str]) -> Option<String>;
    fn env_path(&self, key: &str) -> Option<PathBuf>;
    fn home_dir(&self) -> Option<PathBuf>;
    fn is_dir(&self, path: &Path) -> bool;
    fn estimate_path_size(&self, path: &Path) -> SizeEstimate;
}

pub(super) struct SystemProviderProbe {
    runner: ProcessRunner,
    phase_deadline: Instant,
    cancel: Option<Arc<FlagCancelObserver>>,
}

impl SystemProviderProbe {
    pub(super) fn with_cancel(cancel: Option<Arc<FlagCancelObserver>>) -> Self {
        Self {
            runner: ProcessRunner::default(),
            phase_deadline: Instant::now() + DEFAULT_PROVIDER_PHASE_DEADLINE,
            cancel,
        }
    }
}

impl ProviderProbe for SystemProviderProbe {
    fn resolve_executable(&self, program: &str) -> Option<PathBuf> {
        resolve_executable(program)
    }

    fn command_output(&self, program: &Path, args: &[&str]) -> Option<String> {
        let now = Instant::now();
        if now >= self.phase_deadline {
            warn!(
                program = %program.display(),
                args = ?args,
                "provider probe skipped: global phase deadline reached"
            );
            return None;
        }

        if self
            .cancel
            .as_ref()
            .is_some_and(|flag| flag.is_cancel_requested())
        {
            warn!(
                program = %program.display(),
                "provider probe skipped: cancel requested"
            );
            return None;
        }

        let remaining = self.phase_deadline.saturating_duration_since(now);
        let timeout = DEFAULT_PROVIDER_PROBE_TIMEOUT.min(remaining);
        let noop = NoopCancelObserver;
        let cancel: &dyn CancelObserver = match self.cancel.as_ref() {
            Some(flag) => flag.as_ref(),
            None => &noop,
        };
        let request = ProcessRequest {
            program: OsString::from(program.as_os_str()),
            args: args.iter().map(|arg| OsString::from(*arg)).collect(),
            cwd: CwdPolicy::Neutral,
            timeout: Some(timeout),
            job_deadline: Some(self.phase_deadline),
            cancel,
        };

        let result = self.runner.run(&request);
        match result.status {
            ProcessStatus::Success => {
                Some(String::from_utf8_lossy(&result.output.stdout).into_owned())
            }
            status => {
                warn!(
                    program = %program.display(),
                    args = ?args,
                    ?status,
                    stdout_truncated = result.output.stdout_truncated,
                    stderr_truncated = result.output.stderr_truncated,
                    "provider probe failed with typed process status"
                );
                None
            }
        }
    }

    fn env_path(&self, key: &str) -> Option<PathBuf> {
        env::var_os(key).map(PathBuf::from)
    }

    fn home_dir(&self) -> Option<PathBuf> {
        env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|| {
                let drive = env::var_os("HOMEDRIVE")?;
                let path = env::var_os("HOMEPATH")?;
                Some(PathBuf::from(format!(
                    "{}{}",
                    drive.to_string_lossy(),
                    path.to_string_lossy()
                )))
            })
            .or_else(|| env::var_os("HOME").map(PathBuf::from))
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn estimate_path_size(&self, path: &Path) -> SizeEstimate {
        estimate_tree_with_budget_and_cancel(
            path,
            crate::filesystem::DEFAULT_SIZE_ENTRY_BUDGET,
            self.cancel.as_ref(),
        )
    }
}

pub(crate) fn resolve_executable(program: &str) -> Option<PathBuf> {
    let program_path = Path::new(program);
    if program_path.components().count() > 1 && program_path.is_file() {
        return Some(program_path.to_path_buf());
    }

    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        #[cfg(windows)]
        {
            if program_path.extension().is_none() {
                for extension in windows_path_extensions() {
                    let candidate = dir.join(format!("{program}{extension}"));
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }

        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(windows)]
fn windows_path_extensions() -> Vec<String> {
    env::var_os("PATHEXT")
        .map(|value| {
            value
                .to_string_lossy()
                .split(';')
                .filter(|extension| !extension.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                ".COM".to_string(),
                ".EXE".to_string(),
                ".BAT".to_string(),
                ".CMD".to_string(),
            ]
        })
}
