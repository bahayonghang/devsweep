use std::path::PathBuf;

use devsweep_core::scan_preferences::{
    load_scan_roots, save_scan_roots, scan_root_suggestions, validate_scan_roots,
};
use serde::Serialize;

use crate::error::CommandError;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct CleanScanPreferencesSnapshot {
    version: u8,
    roots: Vec<PathBuf>,
    suggestions: Vec<PathBuf>,
}

fn snapshot(roots: Vec<PathBuf>) -> CleanScanPreferencesSnapshot {
    CleanScanPreferencesSnapshot {
        version: 1,
        roots,
        suggestions: scan_root_suggestions(),
    }
}

#[tauri::command]
pub(crate) async fn clean_scan_preferences_get()
-> Result<CleanScanPreferencesSnapshot, CommandError> {
    tauri::async_runtime::spawn_blocking(|| {
        load_scan_roots().map(snapshot).map_err(CommandError::io)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn clean_scan_preferences_set(
    roots: Vec<PathBuf>,
) -> Result<CleanScanPreferencesSnapshot, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        save_scan_roots(&roots)
            .map(snapshot)
            .map_err(CommandError::io)
    })
    .await
    .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn clean_scan_roots_validate(
    roots: Vec<PathBuf>,
) -> Result<CleanScanPreferencesSnapshot, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_scan_roots(&roots)
            .map(snapshot)
            .map_err(CommandError::scan_failed)
    })
    .await
    .map_err(CommandError::io)?
}
