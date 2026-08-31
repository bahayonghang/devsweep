mod analyze;
mod clean;
mod commands;
mod error;
mod scan;
mod software;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(scan::ScanCoordinator::default())
        .manage(analyze::AnalyzeCoordinator::default())
        .manage(software::SoftwareCoordinator::default())
        .manage(commands::PresentationSettingsCoordinator::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan_start,
            commands::scan_cancel,
            analyze::analyze_start,
            analyze::analyze_cancel,
            software::software_inventory_start,
            software::software_preview,
            software::software_uninstall,
            software::software_audit,
            software::software_cancel,
            clean::plan_dry_run,
            clean::plan_execute,
            commands::protection_list_get,
            commands::protection_list_set,
            commands::presentation_settings_get,
            commands::presentation_settings_set,
            #[cfg(debug_assertions)]
            commands::debug_native_fault_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn capability_grants_only_event_subscription_and_last_window_destroy() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json"))
                .expect("desktop capability must be valid JSON");
        let permissions = capability["permissions"]
            .as_array()
            .expect("desktop capability permissions must be an array");
        let actual = permissions
            .iter()
            .map(|permission| permission.as_str().expect("permission must be a string"))
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            [
                "core:event:allow-listen",
                "core:event:allow-unlisten",
                "core:window:allow-destroy",
            ]
        );
    }
}
