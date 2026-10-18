use std::{fs, io, path::Path};

use devsweep_core::{
    ai::{
        AiClass, AiError, AiExecuteReportV1, AiExit, AiObservationV1, AiPlanV1, AiProvider,
        AiScanRequest, execute_ai, plan_ai, preview_ai, recovery_list, restore_plan, scan_ai,
    },
    execution::SystemTrashRunner,
    process::FlagCancelObserver,
};
use serde_json::Value;

use crate::{
    application::{
        cli::{
            AiClassArg, AiProviderArg, CleanAiCommand, CleanAiPlanCommand,
            CleanAiRecoveryListCommand, CleanAiRestorePlanCommand, CleanAiScanCommand,
            CleanCommand, Cli, Command,
        },
        output::{ApplicationError, ExitClass, OutputSink, ResultFormat, write_json},
    },
    i18n::{Locale, catalogue},
};

pub(super) fn run(cli: &Cli, locale: Locale) -> Result<(), ApplicationError> {
    let Some(Command::Clean(group)) = cli.command.as_ref() else {
        unreachable!("AI commands are dispatched only from clean");
    };
    let CleanCommand::Ai(group) = &group.command else {
        unreachable!("AI commands are dispatched only from clean ai");
    };
    match &group.command {
        CleanAiCommand::Scan(command) => run_scan(cli, locale, command),
        CleanAiCommand::Plan(command) => run_plan(cli, locale, command),
        CleanAiCommand::Preview(command) => run_preview(cli, locale, command),
        CleanAiCommand::Execute(command) => run_execute(cli, locale, command),
        CleanAiCommand::RecoveryList(command) => run_recovery_list(cli, locale, command),
        CleanAiCommand::RestorePlan(command) => run_restore_plan(cli, locale, command),
    }
}

fn run_scan(
    cli: &Cli,
    locale: Locale,
    command: &CleanAiScanCommand,
) -> Result<(), ApplicationError> {
    let observation = scan_ai(
        &AiScanRequest {
            provider: command.provider.map(provider_of),
            home: command.home.clone(),
        },
        &FlagCancelObserver::new(),
    )
    .map_err(|error| map_ai(locale, error))?;
    let bounded = observation
        .providers
        .iter()
        .any(|provider| provider.session_bounded);
    let count = observation.providers.len().to_string();
    let mut human = render(
        locale,
        "clean.ai.scan.summary",
        &[("count", count.as_str())],
    )?;
    if bounded {
        human.push('\n');
        human.push_str(&render(locale, "clean.ai.scan.partial", &[])?);
    }
    let outcome = if bounded { "partial" } else { "succeeded" };
    emit(
        cli,
        "clean.ai.scan",
        outcome,
        serde_json::to_value(&observation).map_err(serialize_error)?,
        &human,
    )?;
    if bounded {
        return Err(ApplicationError::after_output(
            ExitClass::Partial,
            "partial_scan",
            render(locale, "clean.ai.scan.partial", &[])?,
        ));
    }
    Ok(())
}

fn run_plan(
    cli: &Cli,
    locale: Locale,
    command: &CleanAiPlanCommand,
) -> Result<(), ApplicationError> {
    let observation = load_observation(locale, &command.scan)?;
    let plan =
        plan_ai(&observation, class_of(command.class)).map_err(|error| map_ai(locale, error))?;
    write_plan(cli, &plan)
}

fn run_preview(
    cli: &Cli,
    locale: Locale,
    command: &crate::application::cli::PlanPresentationCommand,
) -> Result<(), ApplicationError> {
    let plan = load_plan(locale, &command.plan)?;
    let preview = preview_ai(&plan).map_err(|error| map_ai(locale, error))?;
    let count = preview.targets.len().to_string();
    let human = render(
        locale,
        "clean.ai.preview.digest",
        &[
            ("count", count.as_str()),
            ("digest", preview.digest.as_str()),
        ],
    )?;
    emit(
        cli,
        "clean.ai.preview",
        "succeeded",
        serde_json::to_value(&preview).map_err(serialize_error)?,
        &human,
    )
}

fn run_execute(
    cli: &Cli,
    locale: Locale,
    command: &crate::application::cli::PlanExecutionCommand,
) -> Result<(), ApplicationError> {
    let plan = load_plan(locale, &command.plan)?;
    let report = execute_ai(
        &plan,
        &command.preview_digest,
        command.confirm,
        &FlagCancelObserver::new(),
        &SystemTrashRunner,
    )
    .map_err(|error| map_ai(locale, error))?;
    let count = report.moved.len().to_string();
    let human_key = match report.outcome.as_str() {
        "partial" => "clean.ai.execute.partial",
        "failed" | "unknown" => "clean.ai.execute.failed",
        _ => "clean.ai.execute.completed",
    };
    let human = if human_key == "clean.ai.execute.completed" {
        render(locale, human_key, &[("count", count.as_str())])?
    } else {
        render(locale, human_key, &[])?
    };
    emit(
        cli,
        "clean.ai.execute",
        &report.outcome,
        serde_json::to_value(&report).map_err(serialize_error)?,
        &human,
    )?;
    finish_execute(locale, &report)
}

fn run_recovery_list(
    cli: &Cli,
    locale: Locale,
    command: &CleanAiRecoveryListCommand,
) -> Result<(), ApplicationError> {
    let list = recovery_list(&command.home).map_err(|error| map_ai(locale, error))?;
    let count = list.bundles.len().to_string();
    let human = render(
        locale,
        "clean.ai.recovery.summary",
        &[("count", count.as_str())],
    )?;
    emit(
        cli,
        "clean.ai.recovery-list",
        "succeeded",
        serde_json::to_value(&list).map_err(serialize_error)?,
        &human,
    )
}

fn run_restore_plan(
    cli: &Cli,
    locale: Locale,
    command: &CleanAiRestorePlanCommand,
) -> Result<(), ApplicationError> {
    let plan =
        restore_plan(&command.home, &command.bundle).map_err(|error| map_ai(locale, error))?;
    write_plan(cli, &plan)
}

fn finish_execute(locale: Locale, report: &AiExecuteReportV1) -> Result<(), ApplicationError> {
    match report.outcome.as_str() {
        "succeeded" => Ok(()),
        "partial" => Err(ApplicationError::after_output(
            ExitClass::Partial,
            "partial_execution",
            render(locale, "clean.ai.execute.partial", &[])?,
        )),
        "unknown" => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "unknown_after_dispatch",
            render(locale, "clean.ai.execute.failed", &[])?,
        )),
        _ => Err(ApplicationError::after_output(
            ExitClass::Failed,
            "move_failed",
            render(locale, "clean.ai.execute.failed", &[])?,
        )),
    }
}

fn emit(
    cli: &Cli,
    command: &str,
    outcome: &str,
    data: Value,
    human: &str,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Json => {
            let mut sink = OutputSink::open(cli.output_path())?;
            write_json(
                &mut sink,
                &serde_json::json!({
                    "schema_version": 1,
                    "command": command,
                    "outcome": outcome,
                    "data": data,
                    "warnings": if outcome == "partial" { vec![outcome] } else { Vec::<&str>::new() },
                    "error": null
                }),
            )
        }
        ResultFormat::Human | ResultFormat::PlanJson | ResultFormat::Ndjson => {
            super::write_human(cli, human)
        }
    }
}

fn write_plan(cli: &Cli, plan: &AiPlanV1) -> Result<(), ApplicationError> {
    let mut sink = OutputSink::open(cli.output_path())?;
    let mut bytes = serde_json::to_vec_pretty(plan).map_err(serialize_error)?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn load_observation(locale: Locale, path: &Path) -> Result<AiObservationV1, ApplicationError> {
    let value = read_json(locale, path)?;
    let data = if value.get("schema_version") == Some(&serde_json::json!(1)) {
        value.get("data").cloned().unwrap_or(value)
    } else {
        value
    };
    serde_json::from_value(data)
        .map_err(|_| ApplicationError::usage("invalid_plan", "failed to decode the AI observation"))
}

fn load_plan(locale: Locale, path: &Path) -> Result<AiPlanV1, ApplicationError> {
    let value = read_json(locale, path)?;
    serde_json::from_value(value)
        .map_err(|_| ApplicationError::usage("invalid_plan", "failed to decode the AI plan"))
}

fn read_json(locale: Locale, path: &Path) -> Result<Value, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| map_io(locale, path, error))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ApplicationError::usage("invalid_plan", "failed to decode AI JSON"))
}

fn map_io(locale: Locale, path: &Path, error: io::Error) -> ApplicationError {
    if matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
    ) {
        return ApplicationError::new(
            ExitClass::Unavailable,
            "source_unavailable",
            render(locale, "clean.ai.error.unavailable", &[])
                .unwrap_or_else(|_| format!("AI source is unavailable: {}", path.display())),
        );
    }
    ApplicationError::failed(
        "observation_read_failed",
        format!("failed to read {}: {error}", path.display()),
    )
}

fn map_ai(locale: Locale, error: AiError) -> ApplicationError {
    let exit = match error.exit {
        AiExit::Usage => ExitClass::Usage,
        AiExit::Stale => ExitClass::InvalidAuthority,
        AiExit::Unavailable => ExitClass::Unavailable,
        AiExit::Partial => ExitClass::Partial,
        AiExit::Failed => ExitClass::Failed,
        AiExit::Canceled => ExitClass::CanceledBeforeDispatch,
    };
    let message = match error.code {
        "stale_confirmation" => render_or_code(locale, "clean.ai.error.stale"),
        "deletion_policy" => render_or_code(locale, "clean.ai.error.deletion_policy"),
        "restore_refused" => render_or_code(locale, "clean.ai.error.restore_refused"),
        "source_unavailable" => render_or_code(locale, "clean.ai.error.unavailable"),
        "canceled" => render_or_code(locale, "clean.ai.error.canceled"),
        _ => error.message,
    };
    ApplicationError::new(exit, error.code, message)
}

fn render(locale: Locale, key: &str, values: &[(&str, &str)]) -> Result<String, ApplicationError> {
    catalogue(locale)
        .render(key, values, None)
        .map_err(|error| ApplicationError::failed("catalogue_render_failed", error.to_string()))
}

fn render_or_code(locale: Locale, key: &str) -> String {
    catalogue(locale)
        .render(key, &[], None)
        .unwrap_or_else(|_| key.to_string())
}

fn serialize_error(error: serde_json::Error) -> ApplicationError {
    ApplicationError::failed("output_serialize_failed", error.to_string())
}

fn provider_of(value: AiProviderArg) -> AiProvider {
    match value {
        AiProviderArg::ClaudeCode => AiProvider::ClaudeCode,
        AiProviderArg::Codex => AiProvider::Codex,
        AiProviderArg::Cursor => AiProvider::Cursor,
    }
}

fn class_of(value: AiClassArg) -> AiClass {
    match value {
        AiClassArg::RegenerableCache => AiClass::RegenerableCache,
        AiClassArg::SupersededVersion => AiClass::SupersededVersion,
        AiClassArg::SessionArchive => AiClass::SessionArchive,
        AiClassArg::Worktree => AiClass::Worktree,
        AiClassArg::InstalledTool => AiClass::InstalledTool,
    }
}
