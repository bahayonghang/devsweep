use std::{
    cell::RefCell,
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use tracing::warn;

use crate::{
    filesystem::{SizeEstimate, estimate_tree_with_budget_and_cancel},
    model::ScanCoverageStatus,
    process::{
        CancelObserver, CwdPolicy, DEFAULT_PROVIDER_PHASE_DEADLINE, DEFAULT_PROVIDER_PROBE_TIMEOUT,
        FlagCancelObserver, NoopCancelObserver, ProcessRequest, ProcessRunner, ProcessStatus,
    },
};

pub(super) trait ProviderProbe {
    fn begin_provider(&self) {}
    fn failure(&self) -> Option<ScanCoverageStatus> {
        None
    }
    fn failure_detail(&self) -> Option<String> {
        None
    }
    fn set_failure(&self, _status: ScanCoverageStatus) {}
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
    failure: RefCell<Option<ScanCoverageStatus>>,
    failure_detail: RefCell<Option<String>>,
}

impl SystemProviderProbe {
    pub(super) fn with_cancel(cancel: Option<Arc<FlagCancelObserver>>) -> Self {
        Self {
            runner: ProcessRunner::default(),
            phase_deadline: Instant::now() + DEFAULT_PROVIDER_PHASE_DEADLINE,
            cancel,
            failure: RefCell::new(None),
            failure_detail: RefCell::new(None),
        }
    }
}

impl ProviderProbe for SystemProviderProbe {
    fn begin_provider(&self) {
        *self.failure.borrow_mut() = None;
        *self.failure_detail.borrow_mut() = None;
    }
    fn failure(&self) -> Option<ScanCoverageStatus> {
        *self.failure.borrow()
    }
    fn failure_detail(&self) -> Option<String> {
        self.failure_detail.borrow().clone()
    }
    fn set_failure(&self, status: ScanCoverageStatus) {
        *self.failure.borrow_mut() = Some(status);
    }
    fn resolve_executable(&self, program: &str) -> Option<PathBuf> {
        let resolved = resolve_executable(program);
        if resolved.is_none() && self.failure.borrow().is_none() {
            *self.failure.borrow_mut() = Some(ScanCoverageStatus::Absent);
        }
        resolved
    }

    fn command_output(&self, program: &Path, args: &[&str]) -> Option<String> {
        let now = Instant::now();
        if now >= self.phase_deadline {
            *self.failure.borrow_mut() = Some(ScanCoverageStatus::BudgetLimited);
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
            *self.failure.borrow_mut() = Some(ScanCoverageStatus::Canceled);
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
        let name = program
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let rule = match name.as_str() {
            "npm" => "npm.cache.clean",
            "pnpm" => "pnpm.store.prune",
            "yarn" => "yarn.cache.clean.classic",
            _ => "native",
        };
        let (program, prefix_args, _) =
            match crate::process::resolve_provider_dispatch(program, rule) {
                Ok(dispatch) => dispatch,
                Err(error) => {
                    *self.failure.borrow_mut() = Some(ScanCoverageStatus::Unsupported);
                    *self.failure_detail.borrow_mut() = Some(error.to_string());
                    return None;
                }
            };
        let request = ProcessRequest {
            program: OsString::from(program.as_os_str()),
            args: prefix_args
                .iter()
                .map(OsString::from)
                .chain(args.iter().map(|arg| OsString::from(*arg)))
                .collect(),
            cwd: CwdPolicy::Neutral,
            timeout: Some(timeout),
            job_deadline: Some(self.phase_deadline),
            cancel,
        };

        let result = self.runner.run(&request);
        if result.output.stdout_truncated || result.output.stderr_truncated {
            *self.failure.borrow_mut() = Some(ScanCoverageStatus::OutputTruncated);
            return None;
        }
        match result.status {
            ProcessStatus::Success => match String::from_utf8(result.output.stdout) {
                Ok(output) => {
                    *self.failure.borrow_mut() = None;
                    Some(output)
                }
                Err(_) => {
                    *self.failure.borrow_mut() = Some(ScanCoverageStatus::Failed);
                    None
                }
            },
            status => {
                *self.failure.borrow_mut() = Some(match status {
                    ProcessStatus::Timeout => ScanCoverageStatus::Timeout,
                    ProcessStatus::Canceled => ScanCoverageStatus::Canceled,
                    ProcessStatus::NotFound => ScanCoverageStatus::Absent,
                    _ => ScanCoverageStatus::Failed,
                });
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

pub(crate) use crate::process::resolve_executable;
