mod analyze;
mod clean;
mod commands;
mod error;
mod optimize;
mod scan;
#[cfg(test)]
mod service_boundary;
mod software;
mod status;
mod support;
#[cfg(test)]
mod wire_parity;

/// Frozen Tauri command names registered after every mode child passed
/// recursive acceptance. Glue adds no compatibility alias and no domain
/// handler of its own.
#[cfg(test)]
const SHIPPED_INVOKE_COMMANDS: &[&str] = &[
    "scan_start",
    "scan_cancel",
    "analyze_start",
    "analyze_cancel",
    "software_inventory_start",
    "software_preview",
    "software_uninstall",
    "software_audit",
    "software_cancel",
    "optimize_list",
    "optimize_preview",
    "optimize_run",
    "optimize_audit",
    "optimize_cancel",
    "status_snapshot",
    "status_live_start",
    "status_cancel",
    "plan_dry_run",
    "plan_execute",
    "protection_list_get",
    "protection_list_set",
    "protection_list",
    "protection_add",
    "protection_remove",
    "rules_list",
    "rules_show",
    "history_list",
    "history_show",
    "presentation_settings_get",
    "presentation_settings_set",
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(scan::ScanCoordinator::default())
        .manage(analyze::AnalyzeCoordinator::default())
        .manage(software::SoftwareCoordinator::default())
        .manage(optimize::OptimizeCoordinator::default())
        .manage(status::StatusCoordinator::default())
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
            optimize::optimize_list,
            optimize::optimize_preview,
            optimize::optimize_run,
            optimize::optimize_audit,
            optimize::optimize_cancel,
            status::status_snapshot,
            status::status_live_start,
            status::status_cancel,
            clean::plan_dry_run,
            clean::plan_execute,
            commands::protection_list_get,
            commands::protection_list_set,
            support::protection_list,
            support::protection_add,
            support::protection_remove,
            support::rules_list,
            support::rules_show,
            support::history_list,
            support::history_show,
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
    use super::SHIPPED_INVOKE_COMMANDS;

    #[test]
    fn invoke_handler_registers_every_accepted_mode_and_no_alias() {
        let source = include_str!("lib.rs");
        let start = source
            .find("tauri::generate_handler![")
            .expect("invoke handler inventory");
        let block = source[start..]
            .split_once("])")
            .map(|(block, _)| block)
            .expect("closed invoke handler");
        for command in SHIPPED_INVOKE_COMMANDS {
            assert!(
                block.contains(command),
                "shipped command {command} is missing from generate_handler"
            );
        }
        for alias in [
            "commands::tui",
            "commands::inventory",
            "legacy_scan",
            "compatibility_alias",
        ] {
            assert!(
                !block.contains(alias),
                "compatibility alias {alias} must not enter the invoke handler"
            );
        }
        assert!(block.contains("debug_native_fault_mode"));
        assert_eq!(SHIPPED_INVOKE_COMMANDS.len(), 30);
    }

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
