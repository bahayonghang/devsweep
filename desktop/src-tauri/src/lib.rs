mod analyze;
mod clean;
mod commands;
mod desktop_preferences;
mod error;
mod hud;
mod optimize;
mod scan;
#[cfg(test)]
mod service_boundary;
mod software;
mod status;
mod support;
mod tray;
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
    "analyze_default_root",
    "analyze_reveal",
    "analyze_trash_preview",
    "analyze_trash_execute",
    "software_inventory_start",
    "software_preview",
    "software_uninstall",
    "software_audit",
    "software_cancel",
    "software_updates_check",
    "software_startup_list",
    "software_startup_set",
    "software_leftovers_preview",
    "software_leftovers_execute",
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
    "history_clean_totals",
    "presentation_settings_get",
    "presentation_settings_set",
    "desktop_preferences_get",
    "desktop_preferences_update",
];

/// App commands the tray HUD window may invoke. The HUD reads only the
/// persisted language and desktop preferences; samples arrive as events.
const HUD_INVOKE_COMMANDS: &[&str] = &["presentation_settings_get", "desktop_preferences_get"];

/// Tauri allows every local window to invoke every app command when the app
/// has no ACL manifest. This gate keeps cleanup, uninstall, and trash commands
/// on the main window. The HUD gets its read-only allow-list; any other window
/// gets nothing.
fn window_may_invoke(window_label: &str, command: &str) -> bool {
    match window_label {
        tray::MAIN_WINDOW_LABEL => true,
        hud::HUD_WINDOW_LABEL => HUD_INVOKE_COMMANDS.contains(&command),
        _ => false,
    }
}

fn window_gated<R: tauri::Runtime>(
    handler: impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static,
) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    move |invoke| {
        if window_may_invoke(
            invoke.message.webview_ref().label(),
            invoke.message.command(),
        ) {
            return handler(invoke);
        }
        let message = format!(
            "Command {} is not allowed for window {}",
            invoke.message.command(),
            invoke.message.webview_ref().label()
        );
        invoke.resolver.reject(message);
        true
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(scan::ScanCoordinator::default())
        .manage(analyze::AnalyzeCoordinator::default())
        .manage(software::SoftwareCoordinator::default())
        .manage(optimize::OptimizeCoordinator::default())
        .manage(status::StatusCoordinator::default())
        .manage(commands::PresentationSettingsCoordinator::default())
        .manage(desktop_preferences::DesktopPreferencesCoordinator::default())
        .manage(tray::HudState::default())
        .setup(|app| {
            tray::setup(app)?;
            Ok(())
        })
        .on_window_event(tray::on_window_event)
        .invoke_handler(window_gated(tauri::generate_handler![
            commands::scan_start,
            commands::scan_cancel,
            analyze::analyze_start,
            analyze::analyze_cancel,
            analyze::analyze_default_root,
            analyze::analyze_reveal,
            analyze::analyze_trash_preview,
            analyze::analyze_trash_execute,
            software::software_inventory_start,
            software::software_preview,
            software::software_uninstall,
            software::software_audit,
            software::software_cancel,
            software::software_updates_check,
            software::software_startup_list,
            software::software_startup_set,
            software::software_leftovers_preview,
            software::software_leftovers_execute,
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
            support::history_clean_totals,
            commands::presentation_settings_get,
            commands::presentation_settings_set,
            desktop_preferences::desktop_preferences_get,
            desktop_preferences::desktop_preferences_update,
            #[cfg(debug_assertions)]
            commands::debug_native_fault_mode,
        ]))
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| tray::on_run_event(app, &event));
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
        assert_eq!(SHIPPED_INVOKE_COMMANDS.len(), 42);
    }

    #[test]
    fn main_capability_grants_only_event_subscription_and_window_controls() {
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
                "core:window:allow-minimize",
                "core:window:allow-toggle-maximize",
                "core:window:allow-is-maximized",
                "core:window:allow-start-dragging",
                "core:window:allow-close",
            ]
        );
    }

    #[test]
    fn hud_capability_grants_only_event_subscription_to_the_hud_window() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/hud.json"))
                .expect("HUD capability must be valid JSON");
        assert_eq!(capability["windows"], serde_json::json!(["hud"]));
        assert_eq!(
            capability["permissions"],
            serde_json::json!(["core:event:allow-listen", "core:event:allow-unlisten"])
        );
        let default: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json"))
                .expect("desktop capability must be valid JSON");
        assert_eq!(default["windows"], serde_json::json!(["main"]));
    }

    #[test]
    fn only_the_main_window_reaches_mutating_commands() {
        use super::{HUD_INVOKE_COMMANDS, window_may_invoke};

        for command in SHIPPED_INVOKE_COMMANDS {
            assert!(window_may_invoke("main", command), "main lost {command}");
            assert_eq!(
                window_may_invoke("hud", command),
                matches!(
                    *command,
                    "presentation_settings_get" | "desktop_preferences_get"
                ),
                "hud access to {command}"
            );
            assert!(
                !window_may_invoke("other", command),
                "other reached {command}"
            );
        }
        assert!(window_may_invoke("main", "debug_native_fault_mode"));
        assert!(!window_may_invoke("hud", "debug_native_fault_mode"));
        assert_eq!(
            HUD_INVOKE_COMMANDS,
            ["presentation_settings_get", "desktop_preferences_get"]
        );
        let source = include_str!("lib.rs");
        assert!(source.contains(".invoke_handler(window_gated(tauri::generate_handler!["));
    }

    #[test]
    fn tray_icon_feature_is_the_only_added_tauri_feature() {
        let manifest = include_str!("../Cargo.toml");
        assert!(manifest.contains(r#"tauri = { version = "2.11.3", features = ["tray-icon"] }"#));
        assert!(!manifest.contains("autostart"));
    }
}
