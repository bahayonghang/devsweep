//! Frozen-grammar Analyze handler.

use devsweep_core::analysis::{AnalyzeCompleteness, AnalyzeRunOutcome, analyze_path};

use super::super::{
    cli::{AnalyzeCommand, Cli, Command},
    output::{ApplicationError, OutputSink, ResultFormat, write_json},
    presentation,
};
use crate::i18n::{Locale, catalogue};

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    match cli.command.as_ref() {
        Some(Command::Analyze(group)) => match &group.command {
            AnalyzeCommand::Scan(command) => {
                let outcome = analyze_path(&command.root, None, None)
                    .map_err(|error| map_anyhow("analyze_failed", error))?;
                emit_scan(cli, locale, &outcome)
            }
        },
        _ => Err(mode_unavailable(cli, locale)),
    }
}

fn mode_unavailable(cli: &Cli, locale: Locale) -> ApplicationError {
    let command = cli.command_name().unwrap_or("analyze");
    let message = catalogue(locale)
        .render("error.mode_unavailable", &[("command", command)], None)
        .unwrap_or_else(|_| format!("unavailable: {command}"));
    ApplicationError::unavailable(message)
}

fn emit_scan(
    cli: &Cli,
    locale: Locale,
    outcome: &AnalyzeRunOutcome,
) -> Result<(), ApplicationError> {
    let snapshot = outcome.snapshot();
    let result_outcome = match snapshot.completeness {
        AnalyzeCompleteness::Complete => "succeeded",
        AnalyzeCompleteness::PartialBudget | AnalyzeCompleteness::Canceled => "partial",
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::analyze::scan_snapshot(locale, snapshot).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "analyze.scan",
            result_outcome,
            serde_json::to_value(snapshot).map_err(|error| {
                ApplicationError::failed("analyze_serialize_failed", error.to_string())
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

fn map_anyhow(code: &'static str, error: anyhow::Error) -> ApplicationError {
    ApplicationError::failed(code, format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("cli parses")
    }

    #[test]
    fn scan_json_is_locale_invariant_and_has_no_cleanup_plan() {
        let fixture = TempDir::new().unwrap();
        let root = fixture.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.bin"), [1, 2, 3, 4]).unwrap();
        let en_out = fixture.path().join("en.json");
        let zh_out = fixture.path().join("zh.json");
        let en = parse(&[
            "devsweep",
            "analyze",
            "scan",
            "--root",
            root.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            en_out.to_str().unwrap(),
        ]);
        let zh = parse(&[
            "devsweep",
            "--language",
            "zh-CN",
            "analyze",
            "scan",
            "--root",
            root.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            zh_out.to_str().unwrap(),
        ]);
        // JSON rejects --language; the second parse should fail closed.
        let zh_ok = parse(&[
            "devsweep",
            "analyze",
            "scan",
            "--root",
            root.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            zh_out.to_str().unwrap(),
        ]);
        let _ = zh;
        run(&en, Locale::En).expect("english json");
        run(&zh_ok, Locale::ZhCn).expect("chinese json");
        let en_bytes = std::fs::read_to_string(&en_out).unwrap();
        let zh_bytes = std::fs::read_to_string(&zh_out).unwrap();
        assert_eq!(en_bytes, zh_bytes);
        assert!(!en_bytes.contains("CleanupPlan"));
        assert!(!en_bytes.contains("intent"));
        let document: serde_json::Value = serde_json::from_str(&en_bytes).unwrap();
        assert_eq!(document["command"], "analyze.scan");
        assert_eq!(document["data"]["version"], 1);
    }

    #[test]
    fn scan_human_uses_owned_renderer_in_both_locales() {
        let fixture = TempDir::new().unwrap();
        let root = fixture.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        let en_out = fixture.path().join("en.txt");
        let zh_out = fixture.path().join("zh.txt");
        let en = parse(&[
            "devsweep",
            "analyze",
            "scan",
            "--root",
            root.to_str().unwrap(),
            "--output",
            en_out.to_str().unwrap(),
        ]);
        run(&en, Locale::En).expect("english human");
        let zh = parse(&[
            "devsweep",
            "analyze",
            "scan",
            "--root",
            root.to_str().unwrap(),
            "--output",
            zh_out.to_str().unwrap(),
        ]);
        run(&zh, Locale::ZhCn).expect("chinese human");
        let english = std::fs::read_to_string(&en_out).unwrap();
        let chinese = std::fs::read_to_string(&zh_out).unwrap();
        assert!(english.contains("Analysis complete") || english.contains("Analysis stopped"));
        assert!(chinese.contains("分析完成") || chinese.contains("预算"));
        assert!(!english.contains("CleanupPlan"));
    }
}
