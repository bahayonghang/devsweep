//! Frozen-grammar Analyze handler.

use std::{fs::File, io::Read, path::Path};

use devsweep_core::analysis::{
    ANALYZE_TRASH_PLAN_MAX_BYTES, AnalyzeCompleteness, AnalyzeObserveError, AnalyzeRootsV1,
    AnalyzeRunOutcome, AnalyzeSavedError, AnalyzeSnapshotV1, AnalyzeTrashPlanV1,
    AnalyzeTrashStrictPreviewV1, AnalyzeTrashStrictReportV1, analyze_path,
    decode_analyze_trash_plan, execute_analyze_trash_plan, explain_analyze_node,
    list_analyze_children, list_analyze_roots, plan_analyze_trash, preview_analyze_trash_plan,
    reveal_saved_analyze_node,
};

use super::super::{
    cli::{
        AnalyzeCommand, AnalyzeExplainCommand, AnalyzeListCommand, AnalyzeRevealCommand,
        AnalyzeRootsCommand, AnalyzeTrashPlanCommand, Cli, Command, PlanExecutionCommand,
        PlanPresentationCommand,
    },
    output::{
        ApplicationError, ExitClass, OutputSink, ResultFormat, ensure_output_absent,
        from_domain_code, refuse_closed_stdout, write_json,
    },
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
            AnalyzeCommand::Roots(command) => run_roots(command, locale, cli),
            AnalyzeCommand::List(command) => run_list(command, locale, cli),
            AnalyzeCommand::Explain(command) => run_explain(command, locale, cli),
            AnalyzeCommand::Reveal(command) => run_reveal(command, locale, cli),
            AnalyzeCommand::TrashPlan(command) => run_trash_plan(command),
            AnalyzeCommand::TrashPreview(command) => run_trash_preview(command, locale, cli),
            AnalyzeCommand::TrashExecute(command) => run_trash_execute(command, locale, cli),
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

fn run_roots(
    command: &AnalyzeRootsCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let report = list_analyze_roots(&command.root).map_err(map_observe)?;
    let outcome = roots_outcome(&report);
    let value = serde_json::to_value(&report)
        .map_err(|error| ApplicationError::failed("analyze_serialize_failed", error.to_string()))?;
    emit_data(cli, locale, "analyze.roots", outcome, value, |locale| {
        presentation::analyze::roots(locale, &report)
    })?;
    finish_observed(outcome, "partial_roots", "analyze_roots_failed")
}

fn run_list(
    command: &AnalyzeListCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let snapshot = load_snapshot(&command.snapshot)?;
    let page = list_analyze_children(&snapshot, command.node, command.offset, command.limit)
        .map_err(map_observe)?;
    let value = serde_json::to_value(&page)
        .map_err(|error| ApplicationError::failed("analyze_serialize_failed", error.to_string()))?;
    emit_data(cli, locale, "analyze.list", "succeeded", value, |locale| {
        presentation::analyze::list_children(locale, &page)
    })
}

fn run_explain(
    command: &AnalyzeExplainCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let snapshot = load_snapshot(&command.snapshot)?;
    let explanation = explain_analyze_node(&snapshot, command.node).map_err(map_observe)?;
    let value = serde_json::to_value(&explanation)
        .map_err(|error| ApplicationError::failed("analyze_serialize_failed", error.to_string()))?;
    emit_data(
        cli,
        locale,
        "analyze.explain",
        "succeeded",
        value,
        |locale| presentation::analyze::explain(locale, &explanation),
    )
}

fn emit_data<F>(
    cli: &Cli,
    locale: Locale,
    command: &str,
    outcome: &str,
    value: serde_json::Value,
    human: F,
) -> Result<(), ApplicationError>
where
    F: FnOnce(Locale) -> Result<String, crate::i18n::CatalogueError>,
{
    match cli.result_format() {
        ResultFormat::Human => {
            let text = human(locale).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(cli, command, outcome, value),
    }
}

fn roots_outcome(report: &AnalyzeRootsV1) -> &'static str {
    let failed = report
        .roots
        .iter()
        .filter(|root| root.error.is_some())
        .count();
    if report.roots.is_empty() || failed == 0 {
        "succeeded"
    } else if failed == report.roots.len() {
        "failed"
    } else {
        "partial"
    }
}

fn finish_observed(
    outcome: &str,
    partial_code: &'static str,
    failed_code: &'static str,
) -> Result<(), ApplicationError> {
    match outcome {
        "partial" => Err(ApplicationError::after_output(
            ExitClass::Partial,
            partial_code,
            "one or more analysis roots could not be read",
        )),
        "failed" => Err(ApplicationError::after_output(
            ExitClass::Failed,
            failed_code,
            "the analysis roots could not be read",
        )),
        _ => Ok(()),
    }
}

fn map_observe(error: AnalyzeObserveError) -> ApplicationError {
    match error {
        AnalyzeObserveError::UnknownNode(_) | AnalyzeObserveError::InvalidSnapshot => {
            ApplicationError::usage(error.code(), error.to_string())
        }
        AnalyzeObserveError::RootsUnavailable => {
            ApplicationError::new(ExitClass::Unavailable, error.code(), error.to_string())
        }
    }
}

fn run_reveal(
    command: &AnalyzeRevealCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let snapshot = load_snapshot(&command.snapshot)?;
    ensure_output_absent(cli.output_path())?;
    reveal_saved_analyze_node(&snapshot, command.node).map_err(map_saved)?;
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::analyze::reveal(locale, command.node).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "analyze.reveal",
            "succeeded",
            serde_json::json!({ "node_id": command.node }),
        ),
    }
}

fn run_trash_plan(command: &AnalyzeTrashPlanCommand) -> Result<(), ApplicationError> {
    let snapshot = load_snapshot(&command.snapshot)?;
    let plan = plan_analyze_trash(&snapshot, &command.select).map_err(map_saved)?;
    let mut sink = OutputSink::open(Some(&command.output))?;
    let mut bytes = serde_json::to_vec_pretty(&plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn run_trash_preview(
    command: &PlanPresentationCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let plan = load_plan(&command.plan)?;
    let preview = preview_analyze_trash_plan(&plan).map_err(map_saved)?;
    emit_preview(cli, locale, &preview)
}

fn run_trash_execute(
    command: &PlanExecutionCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    if !command.confirm {
        return Err(ApplicationError::usage(
            "confirmation_required",
            "the Recycle Bin move requires explicit confirmation",
        ));
    }
    let plan = load_plan(&command.plan)?;
    ensure_output_absent(cli.output_path())?;
    refuse_closed_stdout(cli.output_path())?;
    let report = execute_analyze_trash_plan(&plan, &command.preview_digest, command.confirm, None)
        .map_err(map_saved)?;
    emit_execute(cli, locale, &report)?;
    let failed = report.report.as_ref().is_some_and(|item| item.failed > 0);
    if failed {
        Err(ApplicationError::after_output(
            ExitClass::Failed,
            "analyze_trash_failed",
            "the Recycle Bin move did not complete",
        ))
    } else {
        Ok(())
    }
}

fn load_snapshot(path: &Path) -> Result<AnalyzeSnapshotV1, ApplicationError> {
    let bytes = read_bounded(path, ANALYZE_TRASH_PLAN_MAX_BYTES)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| ApplicationError::usage("invalid_snapshot", "analysis snapshot is invalid"))?;
    let data = if value.get("schema_version") == Some(&serde_json::json!(1))
        && value.get("command") == Some(&serde_json::json!("analyze.scan"))
    {
        value.get("data").cloned().ok_or_else(|| {
            ApplicationError::usage("invalid_snapshot", "analysis snapshot is invalid")
        })?
    } else {
        value
    };
    serde_json::from_value(data)
        .map_err(|_| ApplicationError::usage("invalid_snapshot", "analysis snapshot is invalid"))
}

fn load_plan(path: &Path) -> Result<AnalyzeTrashPlanV1, ApplicationError> {
    let bytes = read_bounded(path, ANALYZE_TRASH_PLAN_MAX_BYTES)?;
    decode_analyze_trash_plan(&bytes).map_err(map_saved)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, ApplicationError> {
    let file = File::open(path).map_err(|error| {
        ApplicationError::failed(
            "plan_read_failed",
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| {
            ApplicationError::failed(
                "plan_read_failed",
                format!("failed to read {}: {error}", path.display()),
            )
        })?;
    if bytes.len() as u64 > limit {
        return Err(ApplicationError::usage(
            "snapshot_limit_exceeded",
            "analysis snapshot exceeds the byte limit",
        ));
    }
    Ok(bytes)
}

fn map_saved(error: AnalyzeSavedError) -> ApplicationError {
    from_domain_code(error.code(), error.to_string())
}

fn emit_preview(
    cli: &Cli,
    locale: Locale,
    preview: &AnalyzeTrashStrictPreviewV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::analyze::trash_preview(locale, preview).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "analyze.trash-preview",
            "succeeded",
            serde_json::to_value(preview).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
}

fn emit_execute(
    cli: &Cli,
    locale: Locale,
    report: &AnalyzeTrashStrictReportV1,
) -> Result<(), ApplicationError> {
    let outcome = if report.report.as_ref().is_some_and(|item| item.failed > 0) {
        "failed"
    } else {
        "succeeded"
    };
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::analyze::trash_execute(locale, report).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            write_human(cli, &text)
        }
        _ => write_envelope(
            cli,
            "analyze.trash-execute",
            outcome,
            serde_json::to_value(report).map_err(|error| {
                ApplicationError::failed("output_serialize_failed", error.to_string())
            })?,
        ),
    }
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

    #[test]
    fn malformed_trash_plan_is_usage_and_keeps_the_existing_output() {
        let fixture = TempDir::new().unwrap();
        let plan = fixture.path().join("plan.json");
        let output = fixture.path().join("out.json");
        std::fs::write(&plan, b"{").unwrap();
        std::fs::write(&output, b"keep").unwrap();
        let digest = format!("sha256:{}", "e".repeat(64));
        let cli = parse(&[
            "devsweep",
            "analyze",
            "trash-execute",
            "--plan",
            plan.to_str().unwrap(),
            "--preview-digest",
            &digest,
            "--confirm",
            "--output",
            output.to_str().unwrap(),
        ]);
        let error = run(&cli, Locale::En).expect_err("malformed plan");
        assert_eq!(error.code, "invalid_plan");
        assert_eq!(error.exit, crate::application::output::ExitClass::Usage);
        assert_eq!(std::fs::read(&output).unwrap(), b"keep");
    }

    #[test]
    fn existing_output_is_rejected_before_reveal() {
        let fixture = TempDir::new().unwrap();
        let snapshot = fixture.path().join("snapshot.json");
        let output = fixture.path().join("out.json");
        std::fs::write(
            &snapshot,
            br#"{"version":1,"root":{"input":"C:/root","normalized":"C:/root","volume":"vol"},"nodes":[],"warnings":[],"completeness":"complete","accounted_owned_bytes":0}"#,
        )
        .unwrap();
        std::fs::write(&output, b"keep").unwrap();
        let cli = parse(&[
            "devsweep",
            "analyze",
            "reveal",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--node",
            "0",
            "--output",
            output.to_str().unwrap(),
        ]);
        let error = run(&cli, Locale::En).expect_err("collision");
        assert_eq!(error.code, "output_exists");
        assert_eq!(std::fs::read(&output).unwrap(), b"keep");
    }
}
