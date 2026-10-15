use crate::error::CommandError;
use devsweep_core::{
    clean_advisories::{
        CleanAdvisoriesV1, CleanAdvisoryPreferencesV1, load_clean_advisory_preferences,
        project_capacity_advisories, save_clean_advisory_preferences,
    },
    model::ScanReport,
};

#[tauri::command]
pub(crate) async fn clean_advisory_preferences_get()
-> Result<CleanAdvisoryPreferencesV1, CommandError> {
    tauri::async_runtime::spawn_blocking(|| {
        load_clean_advisory_preferences().map_err(CommandError::io)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn clean_advisory_preferences_set(
    threshold_gib: u64,
) -> Result<CleanAdvisoryPreferencesV1, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        save_clean_advisory_preferences(threshold_gib).map_err(CommandError::io)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn clean_advisories(
    report: ScanReport,
    threshold_gib: u64,
) -> Result<CleanAdvisoriesV1, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        project_capacity_advisories(&report, threshold_gib).map_err(CommandError::scan_failed)
    })
    .await
    .map_err(CommandError::io)?
}
