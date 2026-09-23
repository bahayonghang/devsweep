use std::path::PathBuf;

use devsweep_core::{
    execution::{ProtectionMutationAction, ProtectionMutationReport, UserProtectionList},
    history::{
        CleanMovedTotalsV1, HistoryDetailV1, HistoryDomain, HistoryListOptions, HistoryListV1,
        clean_moved_totals, list_history, show_history,
    },
    rules::{RuleProjectionV1, rule_projection_by_id, rule_projections},
};

use crate::error::CommandError;

#[tauri::command]
pub(crate) async fn protection_list() -> Result<Vec<PathBuf>, CommandError> {
    tauri::async_runtime::spawn_blocking(|| {
        UserProtectionList::load()
            .map(|list| list.list().to_vec())
            .map_err(CommandError::protection)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn protection_add(
    path: PathBuf,
    confirm: bool,
) -> Result<ProtectionMutationReport, CommandError> {
    mutate_protection(ProtectionMutationAction::Add, path, confirm).await
}

#[tauri::command]
pub(crate) async fn protection_remove(
    path: PathBuf,
    confirm: bool,
) -> Result<ProtectionMutationReport, CommandError> {
    mutate_protection(ProtectionMutationAction::Remove, path, confirm).await
}

async fn mutate_protection(
    action: ProtectionMutationAction,
    path: PathBuf,
    confirm: bool,
) -> Result<ProtectionMutationReport, CommandError> {
    if !confirm {
        return Err(CommandError::ProtectionConfirmationRequired {
            message: "protection mutations require explicit confirmation".to_string(),
        });
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut list = UserProtectionList::load().map_err(CommandError::protection)?;
        match action {
            ProtectionMutationAction::Add => list.add(&path),
            ProtectionMutationAction::Remove => list.remove(&path),
        }
        .map_err(CommandError::protection)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) fn rules_list() -> Vec<RuleProjectionV1> {
    rule_projections()
}

#[tauri::command]
pub(crate) fn rules_show(id: String) -> Result<RuleProjectionV1, CommandError> {
    rule_projection_by_id(&id).ok_or_else(|| CommandError::RuleNotFound {
        message: format!("rule {id} is not in the shipped registry"),
    })
}

#[tauri::command]
pub(crate) async fn history_list(
    domain: Option<HistoryDomain>,
    limit: Option<u32>,
) -> Result<HistoryListV1, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        list_history(HistoryListOptions {
            domain,
            limit: limit.unwrap_or(100),
        })
        .map_err(CommandError::history)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn history_show(operation_id: String) -> Result<HistoryDetailV1, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        show_history(&operation_id).map_err(CommandError::history)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn history_clean_totals() -> Result<CleanMovedTotalsV1, CommandError> {
    tauri::async_runtime::spawn_blocking(|| clean_moved_totals().map_err(CommandError::history))
        .await
        .map_err(CommandError::io)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::history::HistoryRecordV1;

    #[test]
    fn rules_list_is_inspect_only_and_cannot_execute() {
        let rules = rules_list();
        assert!(!rules.is_empty());
        let encoded = serde_json::to_string(&rules).expect("json");
        assert!(!encoded.contains("argv"));
        assert!(!encoded.contains("program"));
        assert!(rules.iter().all(|rule| rule.source == "shipped_registry"));
        assert!(rules_show("missing.rule".to_string()).is_err());
    }

    #[test]
    fn history_show_payload_has_no_replay_fields() {
        let listed = HistoryListV1 {
            operations: Vec::new(),
            stores: Vec::new(),
        };
        let encoded = serde_json::to_string(&listed).expect("json");
        assert!(!encoded.contains("argv"));
        assert!(!encoded.contains("\"path\""));
        let _ = HistoryRecordV1::Unsupported {
            domain: HistoryDomain::Clean,
            operation_id: None,
            schema_version: Some(1),
            reason_code: "unknown_schema",
        };
    }

    #[test]
    fn protection_mutation_requires_confirm() {
        let payload = serde_json::to_value(CommandError::ProtectionConfirmationRequired {
            message: "protection mutations require explicit confirmation".to_string(),
        })
        .expect("json");
        assert_eq!(payload["code"], "protection_confirmation_required");
    }
}
