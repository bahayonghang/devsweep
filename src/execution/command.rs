use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};

use crate::process::{
    CancelObserver, CwdPolicy, DEFAULT_EXECUTOR_COMMAND_TIMEOUT, ProcessRequest, ProcessRunner,
    ProcessStatus, sanitize_process_output,
};

use super::EXECUTOR_DIAGNOSTIC_CAP;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    /// Run a command while observing cooperative cancellation.
    fn run_with_cancel(
        &self,
        request: &CommandRequest,
        cancel: &dyn CancelObserver,
    ) -> Result<CommandOutcome>;
}

pub trait TrashRunner {
    fn move_to_trash(&self, path: &Path) -> Result<()>;
}

#[derive(Debug)]
pub struct ProcessCommandRunner {
    runner: ProcessRunner,
}

impl Default for ProcessCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessCommandRunner {
    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
        }
    }
}

impl CommandRunner for ProcessCommandRunner {
    fn run_with_cancel(
        &self,
        request: &CommandRequest,
        cancel: &dyn CancelObserver,
    ) -> Result<CommandOutcome> {
        let cwd = match &request.cwd {
            Some(path) => CwdPolicy::Explicit {
                path: path.clone(),
                reason: "executor command cwd from validated plan".to_string(),
            },
            None => CwdPolicy::Neutral,
        };

        let process_request = ProcessRequest {
            program: OsString::from(&request.program),
            args: request.args.iter().map(OsString::from).collect(),
            cwd,
            timeout: Some(DEFAULT_EXECUTOR_COMMAND_TIMEOUT),
            job_deadline: None,
            cancel,
        };

        let result = self.runner.run(&process_request);
        let stdout = String::from_utf8_lossy(&result.output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&result.output.stderr).into_owned();
        let diagnostic = sanitize_process_output(&result.output.stderr, EXECUTOR_DIAGNOSTIC_CAP);
        let display = command_display(request);

        match result.status {
            ProcessStatus::Success => Ok(CommandOutcome {
                code: Some(0),
                stdout,
                stderr,
            }),
            ProcessStatus::NotFound => Err(anyhow!("failed to run {display}: program not found")),
            ProcessStatus::Timeout => Err(anyhow!(
                "{display} timed out after {:?}: {}",
                DEFAULT_EXECUTOR_COMMAND_TIMEOUT,
                diagnostic.trim()
            )),
            ProcessStatus::Canceled => Err(anyhow!("{display} canceled")),
            ProcessStatus::InvalidOutput => Err(anyhow!(
                "failed to run {display}: invalid process output or spawn failure"
            )),
            ProcessStatus::Exit { code } => Err(anyhow!(
                "{display} exited with status {code:?}: {}",
                diagnostic.trim()
            )),
        }
    }
}

#[derive(Debug, Default)]
pub struct SystemTrashRunner;

impl TrashRunner for SystemTrashRunner {
    fn move_to_trash(&self, path: &Path) -> Result<()> {
        trash::delete(path).with_context(|| format!("failed to move {} to trash", path.display()))
    }
}

pub(super) fn command_argv(request: &CommandRequest) -> Vec<String> {
    let mut argv = Vec::with_capacity(request.args.len() + 1);
    argv.push(request.program.clone());
    argv.extend(request.args.clone());
    argv
}

fn command_display(request: &CommandRequest) -> String {
    command_argv(request).join(" ")
}
