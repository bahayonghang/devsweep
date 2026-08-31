use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, anyhow};
use devsweep_core::{
    process::{CancelObserver, FlagCancelObserver},
    status::{
        DEFAULT_PROCESS_LIMIT, LiveRequest, StatusError, StatusEventV1, StatusSnapshotV1,
        TerminalReason, capture_snapshot, spawn_live,
    },
};
use serde::Serialize;

use crate::error::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DesktopStatusSnapshotResult {
    Completed {
        operation_id: String,
        snapshot: Box<StatusSnapshotV1>,
    },
    Canceled {
        operation_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DesktopStatusLiveResult {
    Completed { operation_id: String },
    Canceled { operation_id: String },
}

#[derive(Clone)]
struct RunningStatus {
    operation_id: String,
    cancel: Arc<FlagCancelObserver>,
}

#[derive(Clone, Default)]
pub(crate) struct StatusCoordinator {
    running: Arc<Mutex<Option<RunningStatus>>>,
}

impl StatusCoordinator {
    fn begin(&self, operation_id: String) -> Result<Arc<FlagCancelObserver>, CommandError> {
        if operation_id.trim().is_empty() {
            return Err(CommandError::status_failed(anyhow!(
                "operation id must not be empty"
            )));
        }
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running.is_some() {
            return Err(CommandError::StatusAlreadyRunning);
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        *running = Some(RunningStatus {
            operation_id,
            cancel: Arc::clone(&cancel),
        });
        Ok(cancel)
    }

    fn cancel(&self, operation_id: &str) -> Result<(), CommandError> {
        if let Some(running) = self.running.lock().map_err(CommandError::io)?.as_ref()
            && running.operation_id == operation_id
        {
            running.cancel.request_cancel();
        }
        Ok(())
    }

    fn finish(
        &self,
        operation_id: &str,
        completed: &Arc<FlagCancelObserver>,
    ) -> Result<(), CommandError> {
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running.as_ref().is_some_and(|current| {
            current.operation_id == operation_id && Arc::ptr_eq(&current.cancel, completed)
        }) {
            *running = None;
        }
        Ok(())
    }
}

fn run_snapshot(
    operation_id: String,
    process_limit: u32,
    cancel: &Arc<FlagCancelObserver>,
) -> Result<DesktopStatusSnapshotResult, CommandError> {
    match capture_snapshot(process_limit, Some(cancel.as_ref())) {
        Ok(snapshot) => Ok(DesktopStatusSnapshotResult::Completed {
            operation_id,
            snapshot: Box::new(snapshot),
        }),
        Err(StatusError::Canceled) => Ok(DesktopStatusSnapshotResult::Canceled { operation_id }),
        Err(error) => Err(CommandError::status_failed(anyhow!("{error}"))),
    }
}

fn run_live<E>(
    operation_id: String,
    interval_ms: u32,
    process_limit: u32,
    cancel: &Arc<FlagCancelObserver>,
    mut emit: E,
) -> Result<DesktopStatusLiveResult, CommandError>
where
    E: FnMut(StatusEventV1) -> Result<()>,
{
    let (rx, mut control) = match spawn_live(LiveRequest {
        interval_ms,
        process_limit,
        operation_id: operation_id.clone(),
        cancel: Some(cancel),
    }) {
        Ok(pair) => pair,
        Err(StatusError::Canceled) => {
            return Ok(DesktopStatusLiveResult::Canceled { operation_id });
        }
        Err(error) => return Err(CommandError::status_failed(anyhow!("{error}"))),
    };
    let mut canceled = false;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => {
                if matches!(
                    event,
                    StatusEventV1::Terminal {
                        data: devsweep_core::status::StatusTerminalV1 {
                            reason: TerminalReason::Canceled | TerminalReason::BrokenPipe,
                            ..
                        },
                        ..
                    }
                ) {
                    canceled = true;
                }
                let terminal = event.is_terminal();
                if emit(event).is_err() {
                    control.request_cancel();
                    canceled = true;
                    break;
                }
                if terminal {
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if cancel.is_cancel_requested() {
                    control.request_cancel();
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                canceled = true;
                break;
            }
        }
    }
    control.join();
    Ok(if canceled {
        DesktopStatusLiveResult::Canceled { operation_id }
    } else {
        DesktopStatusLiveResult::Completed { operation_id }
    })
}

#[tauri::command]
pub(crate) async fn status_snapshot(
    state: tauri::State<'_, StatusCoordinator>,
    operation_id: String,
    process_limit: Option<u32>,
) -> Result<DesktopStatusSnapshotResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    let limit = process_limit.unwrap_or(DEFAULT_PROCESS_LIMIT);
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_snapshot(operation_id.clone(), limit, &worker_cancel);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn status_live_start(
    state: tauri::State<'_, StatusCoordinator>,
    operation_id: String,
    interval_ms: u32,
    process_limit: Option<u32>,
    on_event: tauri::ipc::Channel<StatusEventV1>,
) -> Result<DesktopStatusLiveResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    let limit = process_limit.unwrap_or(DEFAULT_PROCESS_LIMIT);
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_live(
            operation_id.clone(),
            interval_ms,
            limit,
            &worker_cancel,
            |event| on_event.send(event).map_err(anyhow::Error::from),
        );
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn status_cancel(
    state: tauri::State<'_, StatusCoordinator>,
    operation_id: String,
) -> Result<(), CommandError> {
    state.cancel(&operation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_status_is_rejected() {
        let coordinator = StatusCoordinator::default();
        let running = coordinator
            .begin("first".to_string())
            .expect("first operation reserves slot");
        assert_eq!(
            coordinator
                .begin("second".to_string())
                .expect_err("second operation is rejected"),
            CommandError::StatusAlreadyRunning
        );
        coordinator
            .finish("first", &running)
            .expect("slot releases");
        coordinator
            .begin("second".to_string())
            .expect("next operation can start");
    }
}
