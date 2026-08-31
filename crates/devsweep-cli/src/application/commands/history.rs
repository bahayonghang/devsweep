//! Frozen-grammar `history` inspect-only handlers.

use devsweep_core::history::{
    HistoryDomain, HistoryError, HistoryListOptions, list_history, show_history,
};

use super::super::{
    cli::{Cli, Command, HistoryCommand, HistoryDomain as CliHistoryDomain},
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, write_json},
    presentation,
};
use crate::i18n::{Locale, catalogue};

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::History(group)) => match &group.command {
            HistoryCommand::List(command) => {
                run_list(cli, locale, command.domain.map(map_domain), command.limit)
            }
            HistoryCommand::Show(command) => run_show(cli, locale, &command.operation_id),
        },
        _ => Err(mode_unavailable(cli, locale)),
    }
}

fn map_domain(domain: CliHistoryDomain) -> HistoryDomain {
    match domain {
        CliHistoryDomain::Clean => HistoryDomain::Clean,
        CliHistoryDomain::Software => HistoryDomain::Software,
        CliHistoryDomain::Optimize => HistoryDomain::Optimize,
    }
}

fn mode_unavailable(cli: &Cli, locale: Locale) -> ApplicationError {
    let command = cli.command_name().unwrap_or("history");
    let message = catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .unwrap_or_else(|_| format!("unavailable: {command}"));
    ApplicationError::unavailable(message)
}

fn run_list(
    cli: &Cli,
    locale: Locale,
    domain: Option<HistoryDomain>,
    limit: u32,
) -> Result<(), ApplicationError> {
    let listed = list_history(HistoryListOptions { domain, limit })
        .map_err(|error| map_history_error(locale, error))?;
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::history::list(locale, &listed).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "history.list",
            "succeeded",
            serde_json::to_value(&listed).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn run_show(cli: &Cli, locale: Locale, operation_id: &str) -> Result<(), ApplicationError> {
    let detail = show_history(operation_id).map_err(|error| map_history_error(locale, error))?;
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::history::show(locale, &detail).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "history.show",
            "succeeded",
            serde_json::to_value(&detail).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn map_history_error(locale: Locale, error: HistoryError) -> ApplicationError {
    let message = match &error {
        HistoryError::NotFound {
            stores_incomplete: false,
            operation_id,
        } => catalogue(locale)
            .render(
                "history.v1.show.missing",
                &[("id", operation_id.as_str())],
                None,
            )
            .unwrap_or_else(|_| error.to_string()),
        _ => catalogue(locale)
            .render("history.v1.store.unavailable", &[("domain", "clean")], None)
            .unwrap_or_else(|_| error.to_string()),
    };
    let exit = match &error {
        HistoryError::NotFound {
            stores_incomplete: false,
            ..
        } => ExitClass::Failed,
        _ => ExitClass::Unavailable,
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

    struct IsolatedLocalAppData {
        _temp: TempDir,
        _guard: std::sync::MutexGuard<'static, ()>,
        previous_local: Option<std::ffi::OsString>,
        previous_appdata: Option<std::ffi::OsString>,
    }

    impl IsolatedLocalAppData {
        fn new() -> Self {
            let guard = super::super::lock_process_env();
            let temp = TempDir::new().expect("temp");
            let previous_local = std::env::var_os("LOCALAPPDATA");
            let previous_appdata = std::env::var_os("APPDATA");
            unsafe {
                std::env::set_var("LOCALAPPDATA", temp.path());
                std::env::set_var("APPDATA", temp.path());
            }
            Self {
                _temp: temp,
                _guard: guard,
                previous_local,
                previous_appdata,
            }
        }

        fn path(&self) -> &std::path::Path {
            self._temp.path()
        }
    }

    impl Drop for IsolatedLocalAppData {
        fn drop(&mut self) {
            unsafe {
                match &self.previous_local {
                    Some(value) => std::env::set_var("LOCALAPPDATA", value),
                    None => std::env::remove_var("LOCALAPPDATA"),
                }
                match &self.previous_appdata {
                    Some(value) => std::env::set_var("APPDATA", value),
                    None => std::env::remove_var("APPDATA"),
                }
            }
        }
    }

    #[test]
    fn history_list_reads_only_fixed_v1_stores_and_cannot_replay() {
        let isolated = IsolatedLocalAppData::new();
        let clean = isolated.path().join("DevSweep/audit/v1/clean.jsonl");
        fs::create_dir_all(clean.parent().unwrap()).expect("dir");
        fs::write(
            &clean,
            include_str!(
                "../../../../devsweep-core/tests/fixtures/history/clean-v1/protection-mutation.jsonl"
            ),
        )
        .expect("write");
        let legacy = isolated.path().join("devsweep/audit.jsonl");
        fs::create_dir_all(legacy.parent().unwrap()).expect("legacy dir");
        let legacy_bytes = br#"{"command":"rm","argv":["-rf","/secret"]}
"#;
        fs::write(&legacy, legacy_bytes).expect("legacy");
        let cli = parse(&["devsweep", "history", "list", "--format", "json"]);
        run(&cli, Locale::En).expect("list");
        assert_eq!(fs::read(&legacy).expect("legacy"), legacy_bytes);
        let listed = list_history(HistoryListOptions::default()).expect("core list");
        let encoded = serde_json::to_string(&listed).expect("json");
        assert!(encoded.contains("op-protect-v1-fixture"));
        assert!(!encoded.contains("/secret"));
        assert!(!encoded.contains("\"argv\""));
        assert!(!encoded.contains("\"path\""));
    }

    #[test]
    fn history_show_missing_id_fails_without_replay() {
        let _isolated = IsolatedLocalAppData::new();
        let cli = parse(&[
            "devsweep",
            "history",
            "show",
            "--operation-id",
            "missing-op",
            "--format",
            "json",
        ]);
        let error = run(&cli, Locale::En).expect_err("missing");
        assert_eq!(error.code, "history_not_found");
    }
}
