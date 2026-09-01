//! Frozen-grammar `clean protect` handlers.

use std::path::Path;

use devsweep_core::execution::{ProtectionError, ProtectionMutationAction, UserProtectionList};

use super::super::{
    cli::{CleanCommand, Cli, Command, ProtectCommand},
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, write_json},
    presentation,
};
use crate::i18n::{Locale, catalogue};

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Clean(group)) => match &group.command {
            CleanCommand::Protect(protect) => match &protect.command {
                ProtectCommand::List(_) => run_list(cli, locale),
                ProtectCommand::Add(command) => {
                    run_mutate(cli, locale, ProtectionMutationAction::Add, &command.path)
                }
                ProtectCommand::Remove(command) => {
                    run_mutate(cli, locale, ProtectionMutationAction::Remove, &command.path)
                }
            },
            _ => Err(mode_unavailable(cli, locale)),
        },
        _ => Err(mode_unavailable(cli, locale)),
    }
}

fn mode_unavailable(cli: &Cli, locale: Locale) -> ApplicationError {
    let command = cli.command_name().unwrap_or("clean.protect");
    let message = catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .unwrap_or_else(|_| format!("unavailable: {command}"));
    ApplicationError::unavailable(message)
}

fn run_list(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    let list = UserProtectionList::load().map_err(|error| map_protection_error(locale, error))?;
    let paths: Vec<String> = list
        .list()
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::protect::list(locale, &paths).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "clean.protect.list",
            "succeeded",
            serde_json::json!({ "paths": paths, "count": paths.len() }),
        ),
    }
}

fn run_mutate(
    cli: &Cli,
    locale: Locale,
    action: ProtectionMutationAction,
    path: &Path,
) -> Result<(), ApplicationError> {
    let mut list =
        UserProtectionList::load().map_err(|error| map_protection_error(locale, error))?;
    let report = match action {
        ProtectionMutationAction::Add => list.add(path),
        ProtectionMutationAction::Remove => list.remove(path),
    }
    .map_err(|error| map_protection_error(locale, error))?;
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::protect::mutation(locale, &report).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => {
            let command = match action {
                ProtectionMutationAction::Add => "clean.protect.add",
                ProtectionMutationAction::Remove => "clean.protect.remove",
            };
            write_envelope(
                cli,
                command,
                "succeeded",
                serde_json::to_value(&report).map_err(|error| {
                    ApplicationError::failed("output_serialize_failed", error.to_string())
                })?,
            )
        }
    }
}

fn map_protection_error(locale: Locale, error: ProtectionError) -> ApplicationError {
    let key = match error {
        ProtectionError::TargetMissing { .. } => "protect.v1.path.missing",
        _ => "protect.v1.store.unavailable",
    };
    let message = catalogue(locale)
        .render(key, &[], None)
        .unwrap_or_else(|_| error.to_string());
    let exit = match error {
        ProtectionError::AuditUnknown | ProtectionError::TargetMissing { .. } => ExitClass::Failed,
        ProtectionError::AuditBlocked
        | ProtectionError::StoreUnavailable { .. }
        | ProtectionError::LockUnavailable(_)
        | ProtectionError::Io { .. } => ExitClass::Unavailable,
    };
    ApplicationError::new(exit, error.code(), message)
}

fn write_human(cli: &Cli, text: &str) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(cli.output_path())?;
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn write_envelope(
    cli: &Cli,
    command: &str,
    outcome: &str,
    data: serde_json::Value,
) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(cli.output_path())?;
    write_json(
        &mut sink,
        &serde_json::json!({
            "schema_version": 1,
            "command": command,
            "outcome": outcome,
            "data": data,
            "warnings": [],
            "error": null
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::fs;
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("cli parses")
    }

    struct IsolatedAppData {
        _temp: TempDir,
        _guard: std::sync::MutexGuard<'static, ()>,
        previous_appdata: Option<std::ffi::OsString>,
        previous_local: Option<std::ffi::OsString>,
        previous_xdg: Option<std::ffi::OsString>,
        previous_home: Option<std::ffi::OsString>,
    }

    impl IsolatedAppData {
        fn new() -> Self {
            let guard = super::super::lock_process_env();
            let temp = TempDir::new().expect("temp");
            let previous_appdata = std::env::var_os("APPDATA");
            let previous_local = std::env::var_os("LOCALAPPDATA");
            let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
            let previous_home = std::env::var_os("HOME");
            // SAFETY: tests hold ENV_LOCK and restore the previous values on drop.
            unsafe {
                std::env::set_var("APPDATA", temp.path());
                std::env::set_var("LOCALAPPDATA", temp.path());
                std::env::set_var("XDG_CONFIG_HOME", temp.path());
                std::env::set_var("HOME", temp.path());
            }
            Self {
                _temp: temp,
                _guard: guard,
                previous_appdata,
                previous_local,
                previous_xdg,
                previous_home,
            }
        }

        fn path(&self) -> &std::path::Path {
            self._temp.path()
        }

        fn protection_store(&self) -> std::path::PathBuf {
            #[cfg(windows)]
            {
                self.path().join("devsweep/protected-paths.json")
            }
            #[cfg(target_os = "macos")]
            {
                self.path()
                    .join("Library/Application Support/devsweep/protected-paths.json")
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                self.path().join("devsweep/protected-paths.json")
            }
            #[cfg(not(any(windows, unix)))]
            {
                unreachable!("protect isolation tests require windows or unix")
            }
        }
    }

    impl Drop for IsolatedAppData {
        fn drop(&mut self) {
            unsafe {
                match &self.previous_appdata {
                    Some(value) => std::env::set_var("APPDATA", value),
                    None => std::env::remove_var("APPDATA"),
                }
                match &self.previous_local {
                    Some(value) => std::env::set_var("LOCALAPPDATA", value),
                    None => std::env::remove_var("LOCALAPPDATA"),
                }
                match &self.previous_xdg {
                    Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                    None => std::env::remove_var("XDG_CONFIG_HOME"),
                }
                match &self.previous_home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
    }

    #[test]
    fn protect_list_empty_json_has_no_raw_audit_path() {
        let _isolated = IsolatedAppData::new();
        let cli = parse(&["devsweep", "clean", "protect", "list", "--format", "json"]);
        run(&cli, Locale::En).expect("list succeeds");
    }

    #[test]
    fn protect_add_remove_round_trip_and_redacts_audit() {
        let isolated = IsolatedAppData::new();
        let keep = isolated.path().join("keep-cli");
        fs::create_dir_all(&keep).expect("keep");
        let add = parse(&[
            "devsweep",
            "clean",
            "protect",
            "add",
            "--path",
            keep.to_str().unwrap(),
            "--confirm",
            "--format",
            "json",
        ]);
        run(&add, Locale::En).expect("add");
        let listed = UserProtectionList::load().expect("load");
        assert_eq!(listed.list().len(), 1);
        let audit = isolated.path().join("DevSweep/audit/v1/clean.jsonl");
        let audit_text = fs::read_to_string(&audit).expect("audit");
        assert!(audit_text.contains("protection_mutation"));
        assert!(!audit_text.contains("keep-cli"));
        assert!(!audit_text.contains("\"path\""));
        let remove = parse(&[
            "devsweep",
            "clean",
            "protect",
            "remove",
            "--path",
            keep.to_str().unwrap(),
            "--confirm",
            "--format",
            "json",
        ]);
        run(&remove, Locale::En).expect("remove");
        assert!(
            UserProtectionList::load()
                .expect("reload")
                .list()
                .is_empty()
        );
    }

    #[test]
    fn protect_corrupt_store_is_unavailable_and_preserves_bytes() {
        let isolated = IsolatedAppData::new();
        let store = isolated.protection_store();
        fs::create_dir_all(store.parent().unwrap()).expect("dir");
        fs::write(&store, "{not-json").expect("corrupt");
        let cli = parse(&["devsweep", "clean", "protect", "list", "--format", "json"]);
        let error = run(&cli, Locale::En).expect_err("unavailable");
        assert_eq!(error.code, "protection_store_unavailable");
        assert_eq!(error.exit, ExitClass::Unavailable);
        assert_eq!(fs::read(&store).expect("preserved"), b"{not-json");
    }
}
