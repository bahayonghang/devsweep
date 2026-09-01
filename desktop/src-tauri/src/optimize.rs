use std::{
    fs,
    io::ErrorKind,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow};
use devsweep_core::{
    optimize::{
        MaintenanceActionOutcomeV1, MaintenanceCatalogueEntryV1, MaintenanceExecutionRequest,
        MaintenanceExecutor, MaintenancePlanV1, MaintenancePreviewV1, OPTIMIZE_AUDIT_VERSION,
        OPTIMIZE_CATALOGUE_VERSION, OptimizeAuditRecordV1, catalogue_entries,
        optimize_audit_v1_path, plan_operation, preview_maintenance_plan_live,
    },
    process::{CancelObserver, FlagCancelObserver},
};
use serde::Serialize;

use crate::error::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DesktopOptimizeListResult {
    Completed {
        operation_id: String,
        catalogue_version: u32,
        entries: Vec<MaintenanceCatalogueEntryV1>,
    },
    Canceled {
        operation_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopOptimizePreviewResult {
    pub operation_id: String,
    pub plan: MaintenancePlanV1,
    pub preview: MaintenancePreviewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopOptimizeRunResult {
    pub operation_id: String,
    pub report: devsweep_core::optimize::MaintenanceExecutionReportV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopOptimizeAuditResult {
    pub operation_id: String,
    pub recovered: Vec<MaintenanceActionOutcomeV1>,
    pub records: Vec<OptimizeAuditRecordV1>,
}

#[derive(Clone)]
struct RunningOptimize {
    operation_id: String,
    cancel: Arc<FlagCancelObserver>,
}

#[derive(Clone, Default)]
pub(crate) struct OptimizeCoordinator {
    running: Arc<Mutex<Option<RunningOptimize>>>,
}

impl OptimizeCoordinator {
    fn begin(&self, operation_id: String) -> Result<Arc<FlagCancelObserver>, CommandError> {
        if operation_id.trim().is_empty() {
            return Err(CommandError::optimize_failed(anyhow!(
                "operation id must not be empty"
            )));
        }
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running.is_some() {
            return Err(CommandError::OptimizeAlreadyRunning);
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        *running = Some(RunningOptimize {
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

fn run_list(
    operation_id: String,
    cancel: &Arc<FlagCancelObserver>,
) -> Result<DesktopOptimizeListResult> {
    if cancel.is_cancel_requested() {
        return Ok(DesktopOptimizeListResult::Canceled { operation_id });
    }
    Ok(DesktopOptimizeListResult::Completed {
        operation_id,
        catalogue_version: OPTIMIZE_CATALOGUE_VERSION,
        entries: catalogue_entries().to_vec(),
    })
}

fn run_preview(
    operation_id: String,
    catalogue_id: String,
    cancel: &Arc<FlagCancelObserver>,
) -> Result<DesktopOptimizePreviewResult> {
    let plan = plan_operation(&catalogue_id)?;
    if cancel.is_cancel_requested() {
        anyhow::bail!("Optimize preview canceled before authority construction");
    }
    let preview = preview_maintenance_plan_live(&plan)?;
    Ok(DesktopOptimizePreviewResult {
        operation_id,
        plan,
        preview,
    })
}

fn run_dispatch(
    operation_id: String,
    plan: MaintenancePlanV1,
    preview_digest: String,
    confirmed: bool,
    cancel: Arc<FlagCancelObserver>,
) -> Result<DesktopOptimizeRunResult> {
    let report = MaintenanceExecutor::default().execute(MaintenanceExecutionRequest {
        plan: &plan,
        expected_preview_digest: &preview_digest,
        confirmed,
        cancel: Some(cancel),
    })?;
    Ok(DesktopOptimizeRunResult {
        operation_id,
        report,
    })
}

fn read_audit_records() -> Result<Vec<OptimizeAuditRecordV1>> {
    let path = optimize_audit_v1_path()?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: OptimizeAuditRecordV1 = serde_json::from_str(line)
            .with_context(|| format!("invalid Optimize audit record on line {}", index + 1))?;
        if record.schema_version != OPTIMIZE_AUDIT_VERSION || record.domain != "optimize" {
            anyhow::bail!("unsupported Optimize audit record on line {}", index + 1);
        }
        records.push(record);
    }
    Ok(records)
}

#[tauri::command]
pub(crate) async fn optimize_list(
    state: tauri::State<'_, OptimizeCoordinator>,
    operation_id: String,
) -> Result<DesktopOptimizeListResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_list(operation_id.clone(), &worker_cancel).map_err(CommandError::optimize);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn optimize_preview(
    state: tauri::State<'_, OptimizeCoordinator>,
    operation_id: String,
    catalogue_id: String,
) -> Result<DesktopOptimizePreviewResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_preview(operation_id.clone(), catalogue_id, &worker_cancel)
            .map_err(CommandError::optimize);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn optimize_run(
    state: tauri::State<'_, OptimizeCoordinator>,
    operation_id: String,
    plan: MaintenancePlanV1,
    preview_digest: String,
    confirmed: bool,
) -> Result<DesktopOptimizeRunResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_dispatch(
            operation_id.clone(),
            plan,
            preview_digest,
            confirmed,
            Arc::clone(&worker_cancel),
        )
        .map_err(CommandError::optimize);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn optimize_audit(
    state: tauri::State<'_, OptimizeCoordinator>,
    operation_id: String,
) -> Result<DesktopOptimizeAuditResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            let recovered = MaintenanceExecutor::default().recover_startup()?;
            let records = read_audit_records()?;
            Ok(DesktopOptimizeAuditResult {
                operation_id: operation_id.clone(),
                recovered,
                records,
            })
        })()
        .map_err(CommandError::optimize);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn optimize_cancel(
    state: tauri::State<'_, OptimizeCoordinator>,
    operation_id: String,
) -> Result<(), CommandError> {
    state.cancel(&operation_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::optimize::{MaintenanceActionClass, MaintenanceExecutionOutcome};

    #[test]
    fn concurrent_optimize_operations_and_stale_cancel_are_bounded() {
        let coordinator = OptimizeCoordinator::default();
        let running = coordinator.begin("first".into()).expect("first starts");
        assert_eq!(
            coordinator
                .begin("second".into())
                .expect_err("second rejected"),
            CommandError::OptimizeAlreadyRunning
        );
        coordinator
            .cancel("stale")
            .expect("stale cancel is harmless");
        assert!(!running.is_cancel_requested());
        coordinator
            .cancel("first")
            .expect("matching cancel requests stop");
        assert!(running.is_cancel_requested());
        coordinator
            .finish("first", &running)
            .expect("joined operation releases slot");
        coordinator
            .begin("second".into())
            .expect("next operation starts");
    }

    #[test]
    fn list_wire_is_the_closed_eight_and_omits_executable_facts() {
        let result = run_list("op".into(), &Arc::new(FlagCancelObserver::new())).unwrap();
        let DesktopOptimizeListResult::Completed {
            entries,
            catalogue_version,
            ..
        } = result
        else {
            panic!("list completed");
        };
        assert_eq!(catalogue_version, 1);
        assert_eq!(entries.len(), 8);
        assert_eq!(entries[0].id, "dns.flush");
        assert_eq!(entries[0].action_class, MaintenanceActionClass::Execute);
        assert_eq!(entries[4].action_class, MaintenanceActionClass::Guidance);
        let encoded = serde_json::to_string(&entries).unwrap();
        assert!(!encoded.contains("program"));
        assert!(!encoded.contains("argv"));
        assert!(!encoded.contains("ms-settings"));
        assert!(!encoded.contains("ipconfig"));
    }

    #[test]
    fn run_wire_keeps_launched_distinct_from_completion() {
        let report = DesktopOptimizeRunResult {
            operation_id: "op".into(),
            report: devsweep_core::optimize::MaintenanceExecutionReportV1 {
                version: 1,
                catalogue_version: 1,
                outcomes: vec![devsweep_core::optimize::MaintenanceActionOutcomeV1 {
                    operation_id: "core-op".into(),
                    catalogue_id: "settings.search".into(),
                    action_class: MaintenanceActionClass::SettingsHandoff,
                    outcome: MaintenanceExecutionOutcome::Launched,
                    error_code: None,
                }],
            },
        };
        let encoded = serde_json::to_string(&report).expect("serialize report");
        assert!(encoded.contains("launched"));
        assert!(!encoded.contains("completed"));
        assert!(!encoded.contains("program"));
        assert!(!encoded.contains("ms-settings"));
    }
}
