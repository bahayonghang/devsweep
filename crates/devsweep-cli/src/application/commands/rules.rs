//! Frozen-grammar `clean rules` inspect-only handlers.

use devsweep_core::rules::{rule_projection_by_id, rule_projections};

use super::super::{
    cli::{CleanCommand, Cli, Command, RulesCommand},
    output::{ApplicationError, OutputSink, ResultFormat, write_json},
    presentation,
};
use crate::i18n::{Locale, catalogue};

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Clean(group)) => match &group.command {
            CleanCommand::Rules(rules) => match &rules.command {
                RulesCommand::List(_) => run_list(cli, locale),
                RulesCommand::Show(command) => run_show(cli, locale, &command.id),
            },
            _ => Err(mode_unavailable(cli, locale)),
        },
        _ => Err(mode_unavailable(cli, locale)),
    }
}

fn mode_unavailable(cli: &Cli, locale: Locale) -> ApplicationError {
    let command = cli.command_name().unwrap_or("clean.rules");
    let message = catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .unwrap_or_else(|_| format!("unavailable: {command}"));
    ApplicationError::unavailable(message)
}

fn run_list(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    let rules = rule_projections();
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::rules::list(locale, &rules).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "clean.rules.list",
            "succeeded",
            serde_json::json!({
                "rules": rules,
                "count": rules.len(),
                "inspect_only": true
            }),
        ),
    }
}

fn run_show(cli: &Cli, locale: Locale, id: &str) -> Result<(), ApplicationError> {
    let Some(rule) = rule_projection_by_id(id) else {
        let message = catalogue(locale)
            .render("rules.v1.show.missing", &[("id", id)], None)
            .unwrap_or_else(|_| format!("unknown rule: {id}"));
        return Err(ApplicationError::failed("rule_not_found", message));
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::rules::show(locale, &rule).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "clean.rules.show",
            "succeeded",
            serde_json::to_value(&rule).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
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

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("cli parses")
    }

    #[test]
    fn rules_list_is_inspect_only_and_matches_the_shipped_registry() {
        let cli = parse(&["devsweep", "clean", "rules", "list", "--format", "json"]);
        run(&cli, Locale::En).expect("list succeeds");
        let rules = rule_projections();
        assert!(!rules.is_empty());
        let encoded = serde_json::to_string(&rules).expect("json");
        assert!(!encoded.contains("argv"));
        assert!(!encoded.contains("program"));
        assert!(encoded.contains("inspect_only"));
    }

    #[test]
    fn rules_show_unknown_id_fails_closed() {
        let cli = parse(&[
            "devsweep",
            "clean",
            "rules",
            "show",
            "--id",
            "missing.rule",
            "--format",
            "json",
        ]);
        let error = run(&cli, Locale::En).expect_err("missing");
        assert_eq!(error.code, "rule_not_found");
    }

    #[test]
    fn rules_show_known_id_is_inspect_only() {
        let first = rule_projections().into_iter().next().expect("rule");
        let cli = parse(&[
            "devsweep", "clean", "rules", "show", "--id", &first.id, "--format", "json",
        ]);
        run(&cli, Locale::En).expect("show succeeds");
    }
}
