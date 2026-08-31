use std::io::{IsTerminal, Write};

use anyhow::Result;
use clap::Parser;

use self::{
    cli::{Cli, chinese_help_for},
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, error_envelope, write_json},
};
use crate::i18n::{catalogue, resolve_locale, validate_embedded_catalogues, windows_user_locale};

mod cli;
mod commands;
mod output;
mod presentation;

pub(crate) fn run() -> Result<()> {
    init_tracing();
    if let Err(error) = validate_embedded_catalogues() {
        eprintln!("catalogue_invalid: {error}");
        std::process::exit(ExitClass::Failed.code());
    }

    let args = std::env::args_os().collect::<Vec<_>>();
    if let Some(help) = chinese_help_for(&args) {
        std::io::stdout().write_all(help.as_bytes())?;
        return Ok(());
    }

    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };
    if let Err(detail) = cli.validate() {
        terminate(&cli, ApplicationError::usage("invalid_cli", detail));
    }

    let terminals = TerminalState::current();
    if let Err(error) = validate_terminal_contract(&cli, terminals) {
        terminate(&cli, error);
    }

    let explicit_locale = cli.explicit_locale();
    let result = if cli.is_bare() {
        // The shell owns persisted interactive preference precedence. The CLI
        // passes only the original parsed option and never reparses argv.
        crate::tui::run(explicit_locale).map_err(|error| {
            ApplicationError::failed("tui_failed", format!("terminal UI failed: {error:#}"))
        })
    } else {
        // Non-interactive commands never read persisted presentation settings.
        let locale =
            resolve_non_interactive_locale(explicit_locale, windows_user_locale().as_deref());
        commands::dispatch(&cli, locale)
    };

    if let Err(error) = result {
        terminate(&cli, error);
    }
    Ok(())
}

fn resolve_non_interactive_locale(
    explicit: Option<crate::i18n::Locale>,
    windows_locale: Option<&str>,
) -> crate::i18n::Locale {
    resolve_locale(explicit, windows_locale)
}

#[derive(Debug, Clone, Copy)]
struct TerminalState {
    stdin: bool,
    stdout: bool,
}

impl TerminalState {
    fn current() -> Self {
        Self {
            stdin: std::io::stdin().is_terminal(),
            stdout: std::io::stdout().is_terminal(),
        }
    }
}

fn validate_terminal_contract(cli: &Cli, terminals: TerminalState) -> Result<(), ApplicationError> {
    if cli.is_bare() && !(terminals.stdin && terminals.stdout) {
        let locale = resolve_locale(cli.explicit_locale(), windows_user_locale().as_deref());
        let message = catalogue(locale)
            .render("error.bare_requires_tty", &[], None)
            .map_err(|error| {
                ApplicationError::failed(
                    "catalogue_render_failed",
                    format!("failed to render TTY error: {error}"),
                )
            })?;
        return Err(ApplicationError::usage("tty_required", message));
    }
    if cli.is_human_status_live() && !terminals.stdout {
        return Err(ApplicationError::usage(
            "tty_required",
            "status live --format human requires interactive stdout; use --format ndjson",
        ));
    }
    Ok(())
}

fn terminate(cli: &Cli, mut error: ApplicationError) -> ! {
    if error.output_already_emitted {
        std::process::exit(error.exit.code());
    }
    let exit = emit_error(cli, &error).unwrap_or_else(|write_error| {
        eprintln!("{}: {}", write_error.code, write_error.message);
        error = write_error;
        error.exit
    });
    std::process::exit(exit.code())
}

fn emit_error(cli: &Cli, error: &ApplicationError) -> Result<ExitClass, ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human | ResultFormat::PlanJson => {
            eprintln!("{}: {}", error.code, error.message);
            Ok(error.exit)
        }
        ResultFormat::Json => {
            let mut sink = OutputSink::open(cli.output_path())?;
            write_json(
                &mut sink,
                &error_envelope(cli.command_name().unwrap_or("devsweep"), error),
            )?;
            Ok(error.exit)
        }
        ResultFormat::Ndjson => {
            let mut sink = OutputSink::open(cli.output_path())?;
            let event = serde_json::json!({
                "schema_version": 1,
                "event": "status_terminal",
                "operation_id": "not_dispatched",
                "sequence": 0,
                "emitted_at_unix_ms": 0,
                "data": {
                    "reason": "producer_error",
                    "error_code": error.code
                }
            });
            write_json(&mut sink, &event)?;
            Ok(error.exit)
        }
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_invocation_requires_both_terminal_streams() {
        let cli = Cli::try_parse_from(["devsweep"]).unwrap();
        assert!(
            validate_terminal_contract(
                &cli,
                TerminalState {
                    stdin: true,
                    stdout: true
                }
            )
            .is_ok()
        );
        for state in [
            TerminalState {
                stdin: false,
                stdout: true,
            },
            TerminalState {
                stdin: true,
                stdout: false,
            },
            TerminalState {
                stdin: false,
                stdout: false,
            },
        ] {
            let error = validate_terminal_contract(&cli, state).unwrap_err();
            assert_eq!(error.exit, ExitClass::Usage);
            assert_eq!(error.code, "tty_required");
        }
    }

    #[test]
    fn human_live_requires_stdout_tty_but_ndjson_does_not() {
        let human = Cli::try_parse_from(["devsweep", "status", "live"]).unwrap();
        assert!(
            validate_terminal_contract(
                &human,
                TerminalState {
                    stdin: false,
                    stdout: false
                }
            )
            .is_err()
        );
        let machine =
            Cli::try_parse_from(["devsweep", "status", "live", "--format", "ndjson"]).unwrap();
        assert!(
            validate_terminal_contract(
                &machine,
                TerminalState {
                    stdin: false,
                    stdout: false
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn non_interactive_resolution_remains_explicit_then_os_then_english() {
        use crate::i18n::Locale;

        assert_eq!(
            resolve_non_interactive_locale(Some(Locale::En), Some("zh-CN")),
            Locale::En
        );
        assert_eq!(
            resolve_non_interactive_locale(None, Some("zh-SG")),
            Locale::ZhCn
        );
        assert_eq!(
            resolve_non_interactive_locale(None, Some("zh-TW")),
            Locale::En
        );
        assert_eq!(resolve_non_interactive_locale(None, None), Locale::En);
    }

    #[test]
    fn chinese_help_detection_requires_an_explicit_supported_flag() {
        let args = ["devsweep", "--language", "zh-CN", "status", "--help"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();
        let help = chinese_help_for(&args).expect("Chinese contextual help");
        assert!(help.contains("上下文：devsweep status"));
        let args = ["devsweep", "status", "--help"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();
        assert!(chinese_help_for(&args).is_none());
        let args = ["devsweep", "--language", "zh-CN", "tui", "--help"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();
        assert!(chinese_help_for(&args).is_none());
    }
}
