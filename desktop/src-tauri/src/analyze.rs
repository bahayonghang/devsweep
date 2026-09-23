use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use devsweep_core::{
    analysis::{AnalyzeProgressV1, AnalyzeRunOutcome, AnalyzeSnapshotV1, analyze_path},
    process::FlagCancelObserver,
};
use serde::Serialize;

use crate::error::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopAnalyzeProgress {
    pub operation_id: String,
    pub sequence: u64,
    pub stored_nodes: u32,
    pub accounted_owned_bytes: u64,
    pub changed_nodes: Vec<devsweep_core::analysis::AnalyzeNodeV1>,
    pub queue_depth: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DesktopAnalyzeResult {
    Completed {
        operation_id: String,
        snapshot: AnalyzeSnapshotV1,
    },
    Canceled {
        operation_id: String,
        snapshot: AnalyzeSnapshotV1,
    },
}

#[derive(Clone)]
struct RunningAnalyze {
    operation_id: String,
    cancel: Arc<FlagCancelObserver>,
}

#[derive(Clone, Default)]
pub(crate) struct AnalyzeCoordinator {
    running: Arc<Mutex<Option<RunningAnalyze>>>,
}

impl AnalyzeCoordinator {
    pub(crate) fn begin(
        &self,
        operation_id: String,
    ) -> Result<Arc<FlagCancelObserver>, CommandError> {
        if operation_id.trim().is_empty() {
            return Err(CommandError::analyze_failed(anyhow!(
                "operation id must not be empty"
            )));
        }
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running.is_some() {
            return Err(CommandError::AnalyzeAlreadyRunning);
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        *running = Some(RunningAnalyze {
            operation_id,
            cancel: Arc::clone(&cancel),
        });
        Ok(cancel)
    }

    pub(crate) fn cancel(&self, operation_id: &str) -> Result<(), CommandError> {
        if let Some(running) = self.running.lock().map_err(CommandError::io)?.as_ref()
            && running.operation_id == operation_id
        {
            running.cancel.request_cancel();
        }
        Ok(())
    }

    pub(crate) fn finish(
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

pub(crate) fn run_analyze_job<E>(
    root: std::path::PathBuf,
    operation_id: String,
    cancel: &Arc<FlagCancelObserver>,
    mut emit: E,
) -> Result<DesktopAnalyzeResult>
where
    E: FnMut(DesktopAnalyzeProgress) -> Result<()>,
{
    let mut last_sequence = 0_u64;
    let mut progress = |batch: AnalyzeProgressV1| {
        if batch.sequence <= last_sequence {
            return;
        }
        last_sequence = batch.sequence;
        let _ = emit(DesktopAnalyzeProgress {
            operation_id: operation_id.clone(),
            sequence: batch.sequence,
            stored_nodes: batch.stored_nodes,
            accounted_owned_bytes: batch.accounted_owned_bytes,
            changed_nodes: batch.changed_nodes,
            queue_depth: batch.queue_depth,
        });
    };
    let outcome = analyze_path(&root, Some(cancel), Some(&mut progress))?;
    Ok(match outcome {
        AnalyzeRunOutcome::Completed { snapshot } => DesktopAnalyzeResult::Completed {
            operation_id,
            snapshot,
        },
        AnalyzeRunOutcome::Canceled { snapshot } => DesktopAnalyzeResult::Canceled {
            operation_id,
            snapshot,
        },
    })
}

#[tauri::command]
pub(crate) async fn analyze_start(
    state: tauri::State<'_, AnalyzeCoordinator>,
    operation_id: String,
    root: std::path::PathBuf,
    on_progress: tauri::ipc::Channel<DesktopAnalyzeProgress>,
) -> Result<DesktopAnalyzeResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_analyze_job(root, operation_id.clone(), &worker_cancel, |progress| {
            on_progress.send(progress).map_err(anyhow::Error::from)
        })
        .map_err(CommandError::analyze_failed);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn analyze_cancel(
    state: tauri::State<'_, AnalyzeCoordinator>,
    operation_id: String,
) -> Result<(), CommandError> {
    state.cancel(&operation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_analyze_is_rejected() {
        let coordinator = AnalyzeCoordinator::default();
        let running = coordinator
            .begin("first".to_string())
            .expect("first operation reserves slot");
        assert_eq!(
            coordinator
                .begin("second".to_string())
                .expect_err("second operation is rejected"),
            CommandError::AnalyzeAlreadyRunning
        );
        coordinator
            .finish("first", &running)
            .expect("slot releases");
        coordinator
            .begin("second".to_string())
            .expect("next operation can start");
    }

    #[test]
    fn stale_progress_sequences_are_discarded() {
        let mut last = 0_u64;
        let mut kept = Vec::new();
        for sequence in [1_u64, 1, 3, 2, 4] {
            if sequence <= last {
                continue;
            }
            last = sequence;
            kept.push(sequence);
        }
        assert_eq!(kept, vec![1, 3, 4]);
    }

    #[test]
    fn cancel_joins_before_terminal_result() {
        let coordinator = Arc::new(AnalyzeCoordinator::default());
        let cancel = coordinator
            .begin("slow".to_string())
            .expect("operation reserves slot");
        cancel.request_cancel();
        let outcome = run_analyze_job(
            std::env::temp_dir(),
            "slow".to_string(),
            &cancel,
            |_| Ok(()),
        )
        .expect("canceled analyze joins");
        match outcome {
            DesktopAnalyzeResult::Canceled { operation_id, .. } => {
                assert_eq!(operation_id, "slow");
            }
            DesktopAnalyzeResult::Completed { snapshot, .. } => {
                assert_eq!(
                    snapshot.completeness,
                    devsweep_core::analysis::AnalyzeCompleteness::Canceled
                );
            }
        }
        coordinator
            .finish("slow", &cancel)
            .expect("join then finish");
    }

    #[test]
    fn analyze_result_serializes_without_cleanup_intent() {
        let result = DesktopAnalyzeResult::Canceled {
            operation_id: "op".to_string(),
            snapshot: AnalyzeSnapshotV1 {
                version: 1,
                root: devsweep_core::analysis::AnalyzeRootIdentity {
                    input: "C:/root".into(),
                    normalized: "C:/root".into(),
                    volume: "vol".into(),
                },
                nodes: Vec::new(),
                warnings: Vec::new(),
                completeness: devsweep_core::analysis::AnalyzeCompleteness::Canceled,
                accounted_owned_bytes: 0,
            },
        };
        let encoded = serde_json::to_string(&result).expect("serialize");
        assert!(!encoded.contains("intent"));
        assert!(!encoded.contains("action"));
    }
}
