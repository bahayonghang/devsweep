mod commands;
mod error;
mod scan;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(scan::ScanCoordinator::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan_start,
            commands::scan_cancel,
            commands::plan_dry_run,
            commands::plan_execute,
            commands::protection_list_get,
            commands::protection_list_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
