use std::{
    fs,
    io::ErrorKind,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow};
use devsweep_core::{
    process::{CancelObserver, FlagCancelObserver},
    software::{
        SOFTWARE_AUDIT_VERSION, SoftwareActionOutcomeV1, SoftwareAuditRecordV1,
        SoftwareExecutionReportV1, SoftwareExecutionRequest, SoftwareExecutor,
        SoftwareInventorySource, SoftwareInventoryV1, SoftwarePreviewV1, SoftwareSelectionPlanV1,
        build_selection_plan, inventory_software, preview_selection_plan_live,
        software_audit_v1_path,
    },
};
use serde::Serialize;

use crate::error::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DesktopSoftwareInventoryResult {
    Completed {
        operation_id: String,
        inventory: SoftwareInventoryV1,
    },
    Canceled {
        operation_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopSoftwarePreviewResult {
    pub operation_id: String,
    pub plan: SoftwareSelectionPlanV1,
    pub preview: SoftwarePreviewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopSoftwareUninstallResult {
    pub operation_id: String,
    pub report: SoftwareExecutionReportV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopSoftwareAuditResult {
    pub operation_id: String,
    pub recovered: Vec<SoftwareActionOutcomeV1>,
    pub records: Vec<SoftwareAuditRecordV1>,
}

#[derive(Clone)]
struct RunningSoftware {
    operation_id: String,
    cancel: Arc<FlagCancelObserver>,
}

#[derive(Clone, Default)]
pub(crate) struct SoftwareCoordinator {
    running: Arc<Mutex<Option<RunningSoftware>>>,
}

impl SoftwareCoordinator {
    fn begin(&self, operation_id: String) -> Result<Arc<FlagCancelObserver>, CommandError> {
        if operation_id.trim().is_empty() {
            return Err(CommandError::software_failed(anyhow!(
                "operation id must not be empty"
            )));
        }
        let mut running = self.running.lock().map_err(CommandError::io)?;
        if running.is_some() {
            return Err(CommandError::SoftwareAlreadyRunning);
        }
        let cancel = Arc::new(FlagCancelObserver::new());
        *running = Some(RunningSoftware {
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

fn run_inventory(
    operation_id: String,
    cancel: &Arc<FlagCancelObserver>,
) -> Result<DesktopSoftwareInventoryResult> {
    let inventory = inventory_software(SoftwareInventorySource::All, Some(cancel))?;
    if cancel.is_cancel_requested() {
        Ok(DesktopSoftwareInventoryResult::Canceled { operation_id })
    } else {
        Ok(DesktopSoftwareInventoryResult::Completed {
            operation_id,
            inventory,
        })
    }
}

fn run_preview(
    operation_id: String,
    inventory: SoftwareInventoryV1,
    selected_ids: Vec<String>,
    cancel: &Arc<FlagCancelObserver>,
) -> Result<DesktopSoftwarePreviewResult> {
    let now = unix_ms(std::time::SystemTime::now());
    let plan = build_selection_plan(&inventory, &selected_ids, now)?;
    let live = inventory_software(SoftwareInventorySource::Msix, Some(cancel))?;
    if cancel.is_cancel_requested() {
        anyhow::bail!("Software preview canceled before authority construction");
    }
    let preview = preview_selection_plan_live(&plan, &live, now)?;
    Ok(DesktopSoftwarePreviewResult {
        operation_id,
        plan,
        preview,
    })
}

fn run_uninstall(
    operation_id: String,
    plan: SoftwareSelectionPlanV1,
    preview_digest: String,
    confirmed: bool,
    cancel: Arc<FlagCancelObserver>,
) -> Result<DesktopSoftwareUninstallResult> {
    let report = SoftwareExecutor::default().execute(SoftwareExecutionRequest {
        plan: &plan,
        expected_preview_digest: &preview_digest,
        confirmed,
        cancel: Some(cancel),
    })?;
    Ok(DesktopSoftwareUninstallResult {
        operation_id,
        report,
    })
}

fn read_audit_records() -> Result<Vec<SoftwareAuditRecordV1>> {
    let path = software_audit_v1_path()?;
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
        let record: SoftwareAuditRecordV1 = serde_json::from_str(line)
            .with_context(|| format!("invalid Software audit record on line {}", index + 1))?;
        if record.schema_version != SOFTWARE_AUDIT_VERSION || record.domain != "software" {
            anyhow::bail!("unsupported Software audit record on line {}", index + 1);
        }
        records.push(record);
    }
    Ok(records)
}

#[tauri::command]
pub(crate) async fn software_inventory_start(
    state: tauri::State<'_, SoftwareCoordinator>,
    operation_id: String,
) -> Result<DesktopSoftwareInventoryResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_inventory(operation_id.clone(), &worker_cancel)
            .map_err(CommandError::software_failed);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn software_preview(
    state: tauri::State<'_, SoftwareCoordinator>,
    operation_id: String,
    inventory: SoftwareInventoryV1,
    selected_ids: Vec<String>,
) -> Result<DesktopSoftwarePreviewResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_preview(
            operation_id.clone(),
            inventory,
            selected_ids,
            &worker_cancel,
        )
        .map_err(CommandError::software);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn software_uninstall(
    state: tauri::State<'_, SoftwareCoordinator>,
    operation_id: String,
    plan: SoftwareSelectionPlanV1,
    preview_digest: String,
    confirmed: bool,
) -> Result<DesktopSoftwareUninstallResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_uninstall(
            operation_id.clone(),
            plan,
            preview_digest,
            confirmed,
            Arc::clone(&worker_cancel),
        )
        .map_err(CommandError::software);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn software_audit(
    state: tauri::State<'_, SoftwareCoordinator>,
    operation_id: String,
) -> Result<DesktopSoftwareAuditResult, CommandError> {
    let cancel = state.begin(operation_id.clone())?;
    let worker_cancel = Arc::clone(&cancel);
    let worker_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            let recovered = SoftwareExecutor::default().recover_startup()?;
            let records = read_audit_records()?;
            Ok(DesktopSoftwareAuditResult {
                operation_id: operation_id.clone(),
                recovered,
                records,
            })
        })()
        .map_err(CommandError::software);
        worker_state.finish(&operation_id, &worker_cancel)?;
        result
    })
    .await
    .map_err(CommandError::io)
    .and_then(|result| result)
}

#[tauri::command]
pub(crate) async fn software_cancel(
    state: tauri::State<'_, SoftwareCoordinator>,
    operation_id: String,
) -> Result<(), CommandError> {
    state.cancel(&operation_id)
}

fn unix_ms(time: std::time::SystemTime) -> u64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::software::{
        SoftwareExecutionOutcome, SoftwareInstalledState, SoftwareRebootEvidence,
    };

    #[test]
    fn concurrent_software_operations_and_stale_cancel_are_bounded() {
        let coordinator = SoftwareCoordinator::default();
        let running = coordinator.begin("first".into()).expect("first starts");
        assert_eq!(
            coordinator
                .begin("second".into())
                .expect_err("second rejected"),
            CommandError::SoftwareAlreadyRunning
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
    fn software_execution_wire_rejects_partial_and_keeps_five_terminals() {
        assert!(serde_json::from_str::<SoftwareExecutionOutcome>("\"partial\"").is_err());
        for outcome in [
            SoftwareExecutionOutcome::Removed,
            SoftwareExecutionOutcome::RebootRequired,
            SoftwareExecutionOutcome::StillPresent,
            SoftwareExecutionOutcome::Failed,
            SoftwareExecutionOutcome::UnknownAfterDispatch,
        ] {
            let report = DesktopSoftwareUninstallResult {
                operation_id: "op".into(),
                report: SoftwareExecutionReportV1 {
                    version: 1,
                    irreversible: true,
                    outcomes: vec![SoftwareActionOutcomeV1 {
                        operation_id: "core-op".into(),
                        software_id: "software:v1:msix:fixture".into(),
                        outcome,
                        installed_state: Some(SoftwareInstalledState::Present),
                        reboot_evidence: SoftwareRebootEvidence::None,
                        error_code: None,
                        irreversible: true,
                    }],
                },
            };
            let encoded = serde_json::to_string(&report).expect("serialize report");
            assert!(!encoded.contains("partial"));
            assert!(!encoded.contains("argv"));
            assert!(!encoded.contains("path"));
        }
    }
}
