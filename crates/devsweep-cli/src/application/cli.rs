#![allow(dead_code)]

use std::path::{Path, PathBuf};

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};

use crate::i18n::Locale;

use super::output::ResultFormat;

#[derive(Debug, Parser)]
#[command(name = "devsweep")]
#[command(about = "Inspect, plan, and explicitly execute developer maintenance.")]
#[command(version)]
pub(super) struct Cli {
    /// Language for human-readable output.
    #[arg(long, value_enum, global = true)]
    pub(super) language: Option<Language>,
    #[command(subcommand)]
    pub(super) command: Option<Command>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum Language {
    #[value(name = "en")]
    En,
    #[value(name = "zh-CN")]
    ZhCn,
}

impl From<Language> for Locale {
    fn from(value: Language) -> Self {
        match value {
            Language::En => Self::En,
            Language::ZhCn => Self::ZhCn,
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    /// Inspect and plan filesystem cleanup.
    Clean(CommandGroup<CleanCommand>),
    /// Inspect installed software and plan current-user MSIX removal.
    Software(CommandGroup<SoftwareCommand>),
    /// Inspect and plan one bounded Windows optimization.
    Optimize(CommandGroup<OptimizeCommand>),
    /// Analyze disk usage without creating executable plans.
    Analyze(CommandGroup<AnalyzeCommand>),
    /// Inspect current system status.
    Status(CommandGroup<StatusCommand>),
    /// Read the fixed DevSweep audit stores.
    History(CommandGroup<HistoryCommand>),
}

#[derive(Debug, Args)]
pub(super) struct CommandGroup<T: clap::Subcommand> {
    #[command(subcommand)]
    pub(super) command: T,
}

#[derive(Debug, Subcommand)]
pub(super) enum CleanCommand {
    /// Scan project and global cleanup targets.
    Scan(CleanScanCommand),
    /// Create a saved cleanup plan from an observation.
    Plan(CleanPlanCommand),
    /// Preview a saved cleanup plan and calculate its live digest.
    Preview(PlanPresentationCommand),
    /// Execute a saved cleanup plan after explicit confirmation.
    Execute(PlanExecutionCommand),
    /// Manage the persistent path-protection list.
    Protect(CommandGroup<ProtectCommand>),
    /// Inspect cleanup rules.
    Rules(CommandGroup<RulesCommand>),
}

#[derive(Debug, Args)]
pub(super) struct CleanScanCommand {
    /// Root to scan. Repeat to preserve multiple roots in command-line order.
    #[arg(long = "root", value_name = "PATH", action = clap::ArgAction::Append, default_value = ".")]
    pub(super) roots: Vec<PathBuf>,
    /// Select the project/global discovery scope.
    #[arg(long, value_enum, default_value_t = CleanScope::All)]
    pub(super) scope: CleanScope,
    /// Re-estimate one target using the bounded review budget.
    #[arg(long, value_name = "TARGET_ID")]
    pub(super) rescan_target: Option<String>,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum CleanScope {
    Projects,
    Global,
    All,
}

#[derive(Debug, Args)]
pub(super) struct CleanPlanCommand {
    /// Saved observation JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) observation: PathBuf,
    /// Target identity to select. Supply one or more values.
    #[arg(long, value_name = "TARGET_ID", required = true, num_args = 1..)]
    pub(super) select: Vec<String>,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct PlanPresentationCommand {
    /// Saved versioned plan JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) plan: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct PlanExecutionCommand {
    /// Saved versioned plan JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) plan: PathBuf,
    /// Digest printed by the immediately preceding live preview.
    #[arg(long, value_name = "DIGEST", value_parser = parse_preview_digest)]
    pub(super) preview_digest: String,
    /// Confirm non-interactive execution of exactly the saved plan.
    #[arg(long, required = true, action = clap::ArgAction::SetTrue)]
    pub(super) confirm: bool,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Subcommand)]
pub(super) enum ProtectCommand {
    /// List protected paths.
    List(SnapshotOutputArgs),
    /// Add one protected path.
    Add(ProtectMutationCommand),
    /// Remove one protected path.
    Remove(ProtectMutationCommand),
}

#[derive(Debug, Args)]
pub(super) struct ProtectMutationCommand {
    /// Path whose protection entry is changed.
    #[arg(long, value_name = "PATH")]
    pub(super) path: PathBuf,
    /// Confirm the persistent protection-list mutation.
    #[arg(long, required = true, action = clap::ArgAction::SetTrue)]
    pub(super) confirm: bool,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Subcommand)]
pub(super) enum RulesCommand {
    /// List cleanup rules.
    List(SnapshotOutputArgs),
    /// Show one cleanup rule.
    Show(RulesShowCommand),
}

#[derive(Debug, Args)]
pub(super) struct RulesShowCommand {
    /// Stable cleanup-rule identity.
    #[arg(long, value_name = "RULE_ID")]
    pub(super) id: String,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Subcommand)]
pub(super) enum SoftwareCommand {
    /// Inventory installed software without creating executable authority.
    Inventory(SoftwareInventoryCommand),
    /// Create a saved software plan from an inventory.
    Plan(SoftwarePlanCommand),
    /// Preview a saved software plan and calculate its live digest.
    Preview(PlanPresentationCommand),
    /// Dispatch a confirmed current-user MSIX uninstall plan.
    Uninstall(PlanExecutionCommand),
}

#[derive(Debug, Args)]
pub(super) struct SoftwareInventoryCommand {
    /// Inventory source.
    #[arg(long, value_enum, default_value_t = SoftwareSource::All)]
    pub(super) source: SoftwareSource,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum SoftwareSource {
    All,
    Arp,
    Msi,
    Msix,
}

#[derive(Debug, Args)]
pub(super) struct SoftwarePlanCommand {
    /// Saved software-inventory JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) inventory: PathBuf,
    /// Stable software identity to select. Supply one or more values.
    #[arg(long, value_name = "SOFTWARE_ID", required = true, num_args = 1..)]
    pub(super) select: Vec<String>,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Subcommand)]
pub(super) enum OptimizeCommand {
    /// List the fixed optimization catalogue.
    List(SnapshotOutputArgs),
    /// Create a saved plan for one exact operation.
    Plan(OptimizePlanCommand),
    /// Preview a saved optimization plan and calculate its live digest.
    Preview(PlanPresentationCommand),
    /// Run a confirmed saved optimization plan.
    Run(PlanExecutionCommand),
}

#[derive(Debug, Args)]
pub(super) struct OptimizePlanCommand {
    /// Stable optimization operation identity.
    #[arg(long, value_name = "OPERATION_ID")]
    pub(super) operation: String,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Subcommand)]
pub(super) enum AnalyzeCommand {
    /// Scan one analysis root.
    Scan(AnalyzeScanCommand),
}

#[derive(Debug, Args)]
pub(super) struct AnalyzeScanCommand {
    /// Root to analyze.
    #[arg(long, value_name = "PATH")]
    pub(super) root: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Subcommand)]
pub(super) enum StatusCommand {
    /// Capture one bounded status snapshot.
    Snapshot(StatusSnapshotCommand),
    /// Stream status snapshots until canceled.
    Live(StatusLiveCommand),
}

#[derive(Debug, Args)]
pub(super) struct StatusSnapshotCommand {
    /// Maximum process rows.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100), default_value_t = 15)]
    pub(super) process_limit: u32,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct StatusLiveCommand {
    /// Sampling interval in seconds.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=60), default_value_t = 2)]
    pub(super) interval: u32,
    /// Maximum process rows.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100), default_value_t = 15)]
    pub(super) process_limit: u32,
    /// Result stream format.
    #[arg(long, value_enum, default_value_t = StreamFormat::Human)]
    pub(super) format: StreamFormat,
    /// New output file. Human live output cannot use this option.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(super) enum HistoryCommand {
    /// List audit operations.
    List(HistoryListCommand),
    /// Show one audit operation.
    Show(HistoryShowCommand),
}

#[derive(Debug, Args)]
pub(super) struct HistoryListCommand {
    /// Restrict results to one fixed audit store.
    #[arg(long, value_enum)]
    pub(super) domain: Option<HistoryDomain>,
    /// Maximum operation rows.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=1000), default_value_t = 100)]
    pub(super) limit: u32,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum HistoryDomain {
    Clean,
    Software,
    Optimize,
}

#[derive(Debug, Args)]
pub(super) struct HistoryShowCommand {
    /// Stable operation identity.
    #[arg(long, value_name = "ID")]
    pub(super) operation_id: String,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SnapshotOutputArgs {
    /// Result document format.
    #[arg(long, value_enum, default_value_t = SnapshotFormat::Human)]
    pub(super) format: SnapshotFormat,
    /// New output file. Existing files are never overwritten.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum SnapshotFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum StreamFormat {
    Human,
    Ndjson,
}

impl Cli {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.language.is_some() && !self.accepts_language() {
            return Err("--language is valid only for human-readable output".to_string());
        }

        match self.command.as_ref() {
            Some(Command::Clean(group)) => match &group.command {
                CleanCommand::Scan(command) => {
                    if command.rescan_target.is_some()
                        && (command.roots.len() != 1 || command.scope == CleanScope::Global)
                    {
                        return Err("--rescan-target requires exactly one --root and a scope containing projects".to_string());
                    }
                }
                CleanCommand::Plan(command) => {
                    reject_same_path(&command.observation, &command.output)?
                }
                CleanCommand::Preview(command) => validate_plan_output(command)?,
                CleanCommand::Execute(command) => validate_execution_output(command)?,
                CleanCommand::Protect(_) | CleanCommand::Rules(_) => {}
            },
            Some(Command::Software(group)) => match &group.command {
                SoftwareCommand::Plan(command) => {
                    reject_same_path(&command.inventory, &command.output)?
                }
                SoftwareCommand::Preview(command) => validate_plan_output(command)?,
                SoftwareCommand::Uninstall(command) => validate_execution_output(command)?,
                SoftwareCommand::Inventory(_) => {}
            },
            Some(Command::Optimize(group)) => match &group.command {
                OptimizeCommand::Preview(command) => validate_plan_output(command)?,
                OptimizeCommand::Run(command) => validate_execution_output(command)?,
                OptimizeCommand::List(_) | OptimizeCommand::Plan(_) => {}
            },
            Some(Command::Status(group)) => {
                if let StatusCommand::Live(command) = &group.command
                    && command.format == StreamFormat::Human
                    && command.output.is_some()
                {
                    return Err(
                        "status live --format human conflicts with --output; use --format ndjson"
                            .to_string(),
                    );
                }
            }
            Some(Command::Analyze(_)) | Some(Command::History(_)) | None => {}
        }

        Ok(())
    }

    pub(super) fn explicit_locale(&self) -> Option<Locale> {
        self.language.map(Into::into)
    }

    pub(super) fn is_bare(&self) -> bool {
        self.command.is_none()
    }

    pub(super) fn is_human_status_live(&self) -> bool {
        matches!(
            self.command.as_ref(),
            Some(Command::Status(CommandGroup {
                command: StatusCommand::Live(StatusLiveCommand {
                    format: StreamFormat::Human,
                    ..
                })
            }))
        )
    }

    pub(super) fn command_name(&self) -> Option<&'static str> {
        match self.command.as_ref()? {
            Command::Clean(group) => Some(match &group.command {
                CleanCommand::Scan(_) => "clean.scan",
                CleanCommand::Plan(_) => "clean.plan",
                CleanCommand::Preview(_) => "clean.preview",
                CleanCommand::Execute(_) => "clean.execute",
                CleanCommand::Protect(group) => match &group.command {
                    ProtectCommand::List(_) => "clean.protect.list",
                    ProtectCommand::Add(_) => "clean.protect.add",
                    ProtectCommand::Remove(_) => "clean.protect.remove",
                },
                CleanCommand::Rules(group) => match &group.command {
                    RulesCommand::List(_) => "clean.rules.list",
                    RulesCommand::Show(_) => "clean.rules.show",
                },
            }),
            Command::Software(group) => Some(match &group.command {
                SoftwareCommand::Inventory(_) => "software.inventory",
                SoftwareCommand::Plan(_) => "software.plan",
                SoftwareCommand::Preview(_) => "software.preview",
                SoftwareCommand::Uninstall(_) => "software.uninstall",
            }),
            Command::Optimize(group) => Some(match &group.command {
                OptimizeCommand::List(_) => "optimize.list",
                OptimizeCommand::Plan(_) => "optimize.plan",
                OptimizeCommand::Preview(_) => "optimize.preview",
                OptimizeCommand::Run(_) => "optimize.run",
            }),
            Command::Analyze(group) => Some(match &group.command {
                AnalyzeCommand::Scan(_) => "analyze.scan",
            }),
            Command::Status(group) => Some(match &group.command {
                StatusCommand::Snapshot(_) => "status.snapshot",
                StatusCommand::Live(_) => "status.live",
            }),
            Command::History(group) => Some(match &group.command {
                HistoryCommand::List(_) => "history.list",
                HistoryCommand::Show(_) => "history.show",
            }),
        }
    }

    pub(super) fn result_format(&self) -> ResultFormat {
        match self.command.as_ref() {
            None => ResultFormat::Human,
            Some(Command::Clean(group)) => match &group.command {
                CleanCommand::Scan(command) => command.output.format.into(),
                CleanCommand::Plan(_) => ResultFormat::PlanJson,
                CleanCommand::Preview(command) => command.output.format.into(),
                CleanCommand::Execute(command) => command.output.format.into(),
                CleanCommand::Protect(group) => match &group.command {
                    ProtectCommand::List(output) => output.format.into(),
                    ProtectCommand::Add(command) | ProtectCommand::Remove(command) => {
                        command.output.format.into()
                    }
                },
                CleanCommand::Rules(group) => match &group.command {
                    RulesCommand::List(output) => output.format.into(),
                    RulesCommand::Show(command) => command.output.format.into(),
                },
            },
            Some(Command::Software(group)) => match &group.command {
                SoftwareCommand::Inventory(command) => command.output.format.into(),
                SoftwareCommand::Plan(_) => ResultFormat::PlanJson,
                SoftwareCommand::Preview(command) => command.output.format.into(),
                SoftwareCommand::Uninstall(command) => command.output.format.into(),
            },
            Some(Command::Optimize(group)) => match &group.command {
                OptimizeCommand::List(output) => output.format.into(),
                OptimizeCommand::Plan(_) => ResultFormat::PlanJson,
                OptimizeCommand::Preview(command) => command.output.format.into(),
                OptimizeCommand::Run(command) => command.output.format.into(),
            },
            Some(Command::Analyze(group)) => match &group.command {
                AnalyzeCommand::Scan(command) => command.output.format.into(),
            },
            Some(Command::Status(group)) => match &group.command {
                StatusCommand::Snapshot(command) => command.output.format.into(),
                StatusCommand::Live(command) => match command.format {
                    StreamFormat::Human => ResultFormat::Human,
                    StreamFormat::Ndjson => ResultFormat::Ndjson,
                },
            },
            Some(Command::History(group)) => match &group.command {
                HistoryCommand::List(command) => command.output.format.into(),
                HistoryCommand::Show(command) => command.output.format.into(),
            },
        }
    }

    pub(super) fn output_path(&self) -> Option<&Path> {
        match self.command.as_ref()? {
            Command::Clean(group) => match &group.command {
                CleanCommand::Scan(command) => command.output.output.as_deref(),
                CleanCommand::Plan(command) => Some(command.output.as_path()),
                CleanCommand::Preview(command) => command.output.output.as_deref(),
                CleanCommand::Execute(command) => command.output.output.as_deref(),
                CleanCommand::Protect(group) => match &group.command {
                    ProtectCommand::List(output) => output.output.as_deref(),
                    ProtectCommand::Add(command) | ProtectCommand::Remove(command) => {
                        command.output.output.as_deref()
                    }
                },
                CleanCommand::Rules(group) => match &group.command {
                    RulesCommand::List(output) => output.output.as_deref(),
                    RulesCommand::Show(command) => command.output.output.as_deref(),
                },
            },
            Command::Software(group) => match &group.command {
                SoftwareCommand::Inventory(command) => command.output.output.as_deref(),
                SoftwareCommand::Plan(command) => Some(command.output.as_path()),
                SoftwareCommand::Preview(command) => command.output.output.as_deref(),
                SoftwareCommand::Uninstall(command) => command.output.output.as_deref(),
            },
            Command::Optimize(group) => match &group.command {
                OptimizeCommand::List(output) => output.output.as_deref(),
                OptimizeCommand::Plan(command) => Some(command.output.as_path()),
                OptimizeCommand::Preview(command) => command.output.output.as_deref(),
                OptimizeCommand::Run(command) => command.output.output.as_deref(),
            },
            Command::Analyze(group) => match &group.command {
                AnalyzeCommand::Scan(command) => command.output.output.as_deref(),
            },
            Command::Status(group) => match &group.command {
                StatusCommand::Snapshot(command) => command.output.output.as_deref(),
                StatusCommand::Live(command) => command.output.as_deref(),
            },
            Command::History(group) => match &group.command {
                HistoryCommand::List(command) => command.output.output.as_deref(),
                HistoryCommand::Show(command) => command.output.output.as_deref(),
            },
        }
    }

    fn accepts_language(&self) -> bool {
        match self.command.as_ref() {
            None => true,
            Some(Command::Clean(group)) => match &group.command {
                CleanCommand::Scan(command) => command.output.format == SnapshotFormat::Human,
                CleanCommand::Preview(command) => command.output.format == SnapshotFormat::Human,
                CleanCommand::Execute(command) => command.output.format == SnapshotFormat::Human,
                CleanCommand::Protect(group) => match &group.command {
                    ProtectCommand::List(output) => output.format == SnapshotFormat::Human,
                    ProtectCommand::Add(command) | ProtectCommand::Remove(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                },
                CleanCommand::Rules(group) => match &group.command {
                    RulesCommand::List(output) => output.format == SnapshotFormat::Human,
                    RulesCommand::Show(command) => command.output.format == SnapshotFormat::Human,
                },
                CleanCommand::Plan(_) => false,
            },
            Some(Command::Software(group)) => match &group.command {
                SoftwareCommand::Inventory(command) => {
                    command.output.format == SnapshotFormat::Human
                }
                SoftwareCommand::Preview(command) => command.output.format == SnapshotFormat::Human,
                SoftwareCommand::Uninstall(command) => {
                    command.output.format == SnapshotFormat::Human
                }
                SoftwareCommand::Plan(_) => false,
            },
            Some(Command::Optimize(group)) => match &group.command {
                OptimizeCommand::List(output) => output.format == SnapshotFormat::Human,
                OptimizeCommand::Preview(command) => command.output.format == SnapshotFormat::Human,
                OptimizeCommand::Run(command) => command.output.format == SnapshotFormat::Human,
                OptimizeCommand::Plan(_) => false,
            },
            Some(Command::Analyze(group)) => match &group.command {
                AnalyzeCommand::Scan(command) => command.output.format == SnapshotFormat::Human,
            },
            Some(Command::Status(group)) => match &group.command {
                StatusCommand::Snapshot(command) => command.output.format == SnapshotFormat::Human,
                StatusCommand::Live(command) => command.format == StreamFormat::Human,
            },
            Some(Command::History(group)) => match &group.command {
                HistoryCommand::List(command) => command.output.format == SnapshotFormat::Human,
                HistoryCommand::Show(command) => command.output.format == SnapshotFormat::Human,
            },
        }
    }
}

impl From<SnapshotFormat> for ResultFormat {
    fn from(value: SnapshotFormat) -> Self {
        match value {
            SnapshotFormat::Human => Self::Human,
            SnapshotFormat::Json => Self::Json,
        }
    }
}

pub(super) fn chinese_help_for(args: &[std::ffi::OsString]) -> Option<String> {
    let values = args
        .iter()
        .filter_map(|value| value.to_str())
        .collect::<Vec<_>>();
    let requested = values
        .iter()
        .any(|value| *value == "--help" || *value == "-h")
        && values
            .windows(2)
            .any(|pair| pair == ["--language", "zh-CN"])
        && matches!(
            Cli::try_parse_from(args),
            Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp
        );
    if !requested {
        return None;
    }

    let (mut command, path) = command_context(args);
    let command_path = if path.is_empty() {
        "devsweep".to_string()
    } else {
        format!("devsweep {}", path.join(" "))
    };
    command = command.bin_name(&command_path);

    let mut help = format!(
        "DevSweep 中文帮助\n\n上下文：{command_path}\n{}\n\n{}\n",
        chinese_command_description(&path),
        command
            .render_usage()
            .to_string()
            .replacen("Usage:", "用法：", 1)
    );

    let subcommands = frozen_subcommands(&command).collect::<Vec<_>>();
    if !subcommands.is_empty() {
        help.push_str("\n子命令：\n");
        for child in subcommands {
            let mut child_path = path.clone();
            child_path.push(child.get_name().to_string());
            help.push_str(&format!(
                "  {:<12} {}\n",
                child.get_name(),
                chinese_command_description(&child_path)
            ));
        }
    }

    let arguments = command.get_arguments().collect::<Vec<_>>();
    if !arguments.is_empty() {
        help.push_str("\n选项：\n");
        for argument in arguments {
            let mut spelling = String::new();
            if let Some(short) = argument.get_short() {
                spelling.push('-');
                spelling.push(short);
                if argument.get_long().is_some() {
                    spelling.push_str(", ");
                }
            }
            if let Some(long) = argument.get_long() {
                spelling.push_str("--");
                spelling.push_str(long);
            }
            if argument.get_action().takes_values()
                && let Some(names) = argument.get_value_names()
            {
                for name in names {
                    spelling.push_str(" <");
                    spelling.push_str(name.as_ref());
                    spelling.push('>');
                }
            }
            let required = if argument.is_required_set() {
                "；必需"
            } else {
                "；可选"
            };
            help.push_str(&format!(
                "  {:<32} {}{}",
                spelling,
                chinese_argument_description(argument.get_id().as_str()),
                required
            ));
            let defaults = if argument.get_action().takes_values() {
                argument
                    .get_default_values()
                    .iter()
                    .filter_map(|value| value.to_str())
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if !defaults.is_empty() {
                help.push_str(&format!("；默认 {}", defaults.join(",")));
            }
            if argument.get_action().takes_values()
                && let Some(values) = argument.get_value_parser().possible_values()
            {
                let values = values
                    .map(|value| value.get_name().to_string())
                    .collect::<Vec<_>>();
                if !values.is_empty() {
                    help.push_str(&format!("；取值 {}", values.join("|")));
                }
            }
            if let Some(range) = chinese_numeric_range(argument.get_id().as_str()) {
                help.push_str(&format!("；范围 {range}"));
            }
            help.push('\n');
        }
    }

    help.push_str("\n约束：\n");
    help.push_str("  --language 仅适用于 human；JSON/NDJSON 与计划输出不本地化。\n");
    help.push_str("  --output 使用排他 create-new；'-' 不是文件或标准输出哨兵。\n");
    for constraint in chinese_context_constraints(&path) {
        help.push_str("  ");
        help.push_str(constraint);
        help.push('\n');
    }
    Some(help)
}

fn command_context(args: &[std::ffi::OsString]) -> (clap::Command, Vec<String>) {
    let root = built_cli_command();
    let mut current = &root;
    let mut path = Vec::new();
    for value in args.iter().skip(1).filter_map(|value| value.to_str()) {
        if let Some(child) = current.find_subcommand(value) {
            path.push(value.to_string());
            current = child;
        }
    }
    (current.clone(), path)
}

fn built_cli_command() -> clap::Command {
    let mut command = Cli::command();
    command.build();
    command
}

fn frozen_subcommands(command: &clap::Command) -> impl Iterator<Item = &clap::Command> {
    command
        .get_subcommands()
        .filter(|child| child.get_name() != "help")
}

fn chinese_command_description(path: &[String]) -> &'static str {
    match path.join(" ").as_str() {
        "" => "检查、规划并仅在明确授权后执行开发环境维护。",
        "clean" => "检查和规划文件系统清理。",
        "clean scan" => "扫描项目与全局清理目标。",
        "clean plan" => "根据观察文档创建保存的清理计划。",
        "clean preview" => "实时预览保存的清理计划并计算摘要。",
        "clean execute" => "在明确确认后执行保存的清理计划。",
        "clean protect" => "管理持久路径保护列表。",
        "clean protect list" => "列出受保护路径。",
        "clean protect add" => "添加一条路径保护。",
        "clean protect remove" => "移除一条路径保护。",
        "clean rules" => "检查清理规则。",
        "clean rules list" => "列出清理规则。",
        "clean rules show" => "显示一条清理规则。",
        "software" => "盘点软件并规划当前用户 MSIX 移除。",
        "software inventory" => "只读盘点已安装软件。",
        "software plan" => "根据软件盘点创建保存的计划。",
        "software preview" => "实时预览保存的软件计划并计算摘要。",
        "software uninstall" => "派发已确认的当前用户 MSIX 卸载计划。",
        "optimize" => "检查和规划一个受限 Windows 优化。",
        "optimize list" => "列出冻结的优化目录。",
        "optimize plan" => "为一个精确操作创建保存的计划。",
        "optimize preview" => "实时预览保存的优化计划并计算摘要。",
        "optimize run" => "运行已确认的优化计划。",
        "analyze" => "分析磁盘占用，不创建可执行计划。",
        "analyze scan" => "分析一个磁盘根目录。",
        "status" => "检查当前系统状态。",
        "status snapshot" => "采集一次有界状态快照。",
        "status live" => "持续输出状态快照，直到取消。",
        "history" => "读取固定的 DevSweep 审计存储。",
        "history list" => "列出审计操作。",
        "history show" => "显示一条审计操作。",
        _ => "DevSweep 命令。",
    }
}

fn chinese_argument_description(id: &str) -> &'static str {
    match id {
        "language" => "人类可读输出语言",
        "roots" | "root" => "输入根目录",
        "scope" => "扫描范围",
        "rescan_target" => "重新测量的稳定目标 ID",
        "format" => "结果格式",
        "output" => "新建输出文件",
        "observation" => "保存的观察 JSON 文件",
        "select" => "按命令行顺序选择一个或多个稳定 ID",
        "plan" => "保存的版本化计划 JSON 文件",
        "preview_digest" => "实时预览输出的 sha256 摘要",
        "confirm" => "明确确认无提示派发",
        "path" => "保护列表路径",
        "id" => "稳定规则 ID",
        "source" => "软件盘点来源",
        "inventory" => "保存的软件盘点 JSON 文件",
        "operation" => "稳定优化操作 ID",
        "process_limit" => "最大进程行数",
        "interval" => "采样间隔秒数",
        "domain" => "固定审计域",
        "limit" => "最大审计行数",
        "operation_id" => "稳定操作 ID",
        "help" => "显示当前上下文帮助",
        "version" => "显示版本",
        _ => "命令参数",
    }
}

fn chinese_numeric_range(id: &str) -> Option<&'static str> {
    match id {
        "process_limit" => Some("1..100"),
        "interval" => Some("1..60"),
        "limit" => Some("1..1000"),
        _ => None,
    }
}

fn chinese_context_constraints(path: &[String]) -> &'static [&'static str] {
    match path.join(" ").as_str() {
        "" => &["直接运行 devsweep 仅在标准输入与标准输出都是 TTY 时打开 TUI。"],
        "clean scan" => &["--rescan-target 要求恰好一个 --root，且 scope 必须包含 projects。"],
        "clean plan" | "software plan" | "optimize plan" => {
            &["计划命令不接受 --format 或 --language，只写版本化 JSON 计划。"]
        }
        "clean preview" | "software preview" | "optimize preview" => {
            &["计划输入与可选输出路径必须不同。"]
        }
        "clean execute" | "software uninstall" | "optimize run" => &[
            "必须同时提供保存计划、sha256: 加 64 位小写十六进制摘要和 --confirm。",
            "计划输入与可选输出路径必须不同；命令不会从 stdin 提示。",
        ],
        "clean protect add" | "clean protect remove" => {
            &["持久保护变更必须提供 --path 和 --confirm。"]
        }
        "status live" => &[
            "human 要求交互式 stdout 且与 --output 冲突；非交互流必须显式选择 ndjson。",
            "NDJSON interval 范围 1..60，process-limit 范围 1..100。",
        ],
        "status snapshot" => &["process-limit 范围 1..100，默认 15。"],
        "history list" => &["limit 范围 1..1000，默认 100。"],
        _ => &[],
    }
}

fn validate_plan_output(command: &PlanPresentationCommand) -> Result<(), String> {
    if let Some(output) = command.output.output.as_deref() {
        reject_same_path(&command.plan, output)?;
    }
    Ok(())
}

fn validate_execution_output(command: &PlanExecutionCommand) -> Result<(), String> {
    if let Some(output) = command.output.output.as_deref() {
        reject_same_path(&command.plan, output)?;
    }
    Ok(())
}

fn reject_same_path(input: &Path, output: &Path) -> Result<(), String> {
    let input = std::path::absolute(input).unwrap_or_else(|_| input.to_path_buf());
    let output = std::path::absolute(output).unwrap_or_else(|_| output.to_path_buf());
    if input == output {
        return Err("input and output paths must differ".to_string());
    }
    Ok(())
}

fn parse_regular_path(value: &str) -> Result<PathBuf, String> {
    if value == "-" {
        return Err("'-' is not a file sentinel; omit --output to use stdout".to_string());
    }
    Ok(PathBuf::from(value))
}

fn parse_preview_digest(value: &str) -> Result<String, String> {
    let Some(digest) = value.strip_prefix("sha256:") else {
        return Err(
            "digest must be sha256: followed by 64 lowercase hexadecimal characters".into(),
        );
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(
            "digest must be sha256: followed by 64 lowercase hexadecimal characters".into(),
        );
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::*;

    fn parse(args: &[&str]) -> Cli {
        let cli = Cli::try_parse_from(args).expect("command parses");
        cli.validate().expect("semantic constraints pass");
        cli
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_frozen_root_and_nested_shape_parses() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let cases = vec![
            vec!["devsweep", "clean", "scan"],
            vec![
                "devsweep",
                "clean",
                "plan",
                "--observation",
                "o.json",
                "--select",
                "x",
                "--output",
                "p.json",
            ],
            vec!["devsweep", "clean", "preview", "--plan", "p.json"],
            vec![
                "devsweep",
                "clean",
                "execute",
                "--plan",
                "p.json",
                "--preview-digest",
                &digest,
                "--confirm",
            ],
            vec!["devsweep", "clean", "protect", "list"],
            vec![
                "devsweep",
                "clean",
                "protect",
                "add",
                "--path",
                "C:/code",
                "--confirm",
            ],
            vec![
                "devsweep",
                "clean",
                "protect",
                "remove",
                "--path",
                "C:/code",
                "--confirm",
            ],
            vec!["devsweep", "clean", "rules", "list"],
            vec!["devsweep", "clean", "rules", "show", "--id", "rust.target"],
            vec!["devsweep", "software", "inventory"],
            vec![
                "devsweep",
                "software",
                "plan",
                "--inventory",
                "i.json",
                "--select",
                "x",
                "--output",
                "p.json",
            ],
            vec!["devsweep", "software", "preview", "--plan", "p.json"],
            vec![
                "devsweep",
                "software",
                "uninstall",
                "--plan",
                "p.json",
                "--preview-digest",
                &digest,
                "--confirm",
            ],
            vec!["devsweep", "optimize", "list"],
            vec![
                "devsweep",
                "optimize",
                "plan",
                "--operation",
                "dns.flush",
                "--output",
                "p.json",
            ],
            vec!["devsweep", "optimize", "preview", "--plan", "p.json"],
            vec![
                "devsweep",
                "optimize",
                "run",
                "--plan",
                "p.json",
                "--preview-digest",
                &digest,
                "--confirm",
            ],
            vec!["devsweep", "analyze", "scan", "--root", "C:/code"],
            vec!["devsweep", "status", "snapshot"],
            vec!["devsweep", "status", "live", "--format", "ndjson"],
            vec!["devsweep", "history", "list"],
            vec!["devsweep", "history", "show", "--operation-id", "op-1"],
        ];
        for case in cases {
            parse(&case);
        }
    }

    #[test]
    fn removed_roots_are_unknown_commands() {
        for root in ["tui", "scan", "inventory", "protect", "rules"] {
            assert!(Cli::try_parse_from(["devsweep", root]).is_err(), "{root}");
        }
    }

    #[test]
    fn defaults_ranges_conflicts_and_authority_syntax_are_frozen() {
        let cli = parse(&["devsweep", "clean", "scan"]);
        let Some(Command::Clean(group)) = cli.command else {
            panic!("clean command");
        };
        let CleanCommand::Scan(command) = group.command else {
            panic!("scan command");
        };
        assert_eq!(command.roots, vec![PathBuf::from(".")]);
        assert_eq!(command.scope, CleanScope::All);
        assert_eq!(command.output.format, SnapshotFormat::Human);

        assert!(
            Cli::try_parse_from(["devsweep", "status", "snapshot", "--process-limit", "0"])
                .is_err()
        );

        let cli = parse(&["devsweep", "software", "inventory"]);
        let Some(Command::Software(group)) = cli.command else {
            panic!("software command");
        };
        let SoftwareCommand::Inventory(command) = group.command else {
            panic!("inventory command");
        };
        assert_eq!(command.source, SoftwareSource::All);

        let cli = parse(&["devsweep", "status", "live", "--format", "ndjson"]);
        let Some(Command::Status(group)) = cli.command else {
            panic!("status command");
        };
        let StatusCommand::Live(command) = group.command else {
            panic!("live command");
        };
        assert_eq!(command.interval, 2);
        assert_eq!(command.process_limit, 15);

        let cli = parse(&["devsweep", "history", "list"]);
        let Some(Command::History(group)) = cli.command else {
            panic!("history command");
        };
        let HistoryCommand::List(command) = group.command else {
            panic!("history list command");
        };
        assert_eq!(command.limit, 100);
        assert!(Cli::try_parse_from(["devsweep", "status", "live", "--interval", "61"]).is_err());
        assert!(Cli::try_parse_from(["devsweep", "history", "list", "--limit", "1001"]).is_err());
        assert!(
            Cli::try_parse_from([
                "devsweep",
                "clean",
                "execute",
                "--plan",
                "p.json",
                "--preview-digest",
                "sha256:ABC",
                "--confirm"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from(["devsweep", "software", "uninstall", "--plan", "p.json"]).is_err()
        );
        let digest = format!("sha256:{}", "a".repeat(64));
        assert!(
            Cli::try_parse_from([
                "devsweep",
                "optimize",
                "run",
                "--plan",
                "p.json",
                "--preview-digest",
                &digest
            ])
            .is_err()
        );
    }

    #[test]
    fn semantic_validation_rejects_machine_language_and_path_collisions() {
        let cli = Cli::try_parse_from([
            "devsweep",
            "--language",
            "zh-CN",
            "status",
            "snapshot",
            "--format",
            "json",
        ])
        .expect("grammar parses before semantic validation");
        assert!(cli.validate().is_err());
        let cli = Cli::try_parse_from([
            "devsweep",
            "clean",
            "preview",
            "--plan",
            "same.json",
            "--output",
            "same.json",
        ])
        .expect("grammar parses before path validation");
        assert!(cli.validate().is_err());
        let cli = Cli::try_parse_from(["devsweep", "status", "live", "--output", "status.txt"])
            .expect("grammar parses before live validation");
        assert!(cli.validate().is_err());
    }

    #[test]
    fn rescan_requires_one_project_containing_root() {
        let invalid = Cli::try_parse_from([
            "devsweep",
            "clean",
            "scan",
            "--scope",
            "global",
            "--rescan-target",
            "id",
        ])
        .expect("grammar parses");
        assert!(invalid.validate().is_err());
        let invalid = Cli::try_parse_from([
            "devsweep",
            "clean",
            "scan",
            "--root",
            "C:/one",
            "--root",
            "C:/two",
            "--scope",
            "projects",
            "--rescan-target",
            "id",
        ])
        .expect("grammar parses");
        assert!(invalid.validate().is_err());
        let valid = Cli::try_parse_from([
            "devsweep",
            "clean",
            "scan",
            "--root",
            "C:/code",
            "--scope",
            "projects",
            "--rescan-target",
            "id",
        ])
        .expect("grammar parses");
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn repeated_roots_and_selections_preserve_command_line_order() {
        let cli = parse(&[
            "devsweep",
            "clean",
            "scan",
            "--root",
            "C:/first",
            "--root",
            "C:/second",
        ]);
        let Some(Command::Clean(group)) = cli.command else {
            panic!("clean command");
        };
        let CleanCommand::Scan(command) = group.command else {
            panic!("scan command");
        };
        assert_eq!(
            command.roots,
            [PathBuf::from("C:/first"), PathBuf::from("C:/second")]
        );

        let cli = parse(&[
            "devsweep",
            "software",
            "plan",
            "--inventory",
            "inventory.json",
            "--select",
            "second",
            "first",
            "--output",
            "plan.json",
        ]);
        let Some(Command::Software(group)) = cli.command else {
            panic!("software command");
        };
        let SoftwareCommand::Plan(command) = group.command else {
            panic!("plan command");
        };
        assert_eq!(command.select, ["second", "first"]);
    }

    #[test]
    fn set_true_flags_are_value_less_in_real_parser_and_chinese_help() {
        fn collect_boolean_flags(command: &clap::Command, path: String, flags: &mut Vec<String>) {
            for argument in command.get_arguments() {
                if matches!(
                    argument.get_action(),
                    clap::ArgAction::SetTrue | clap::ArgAction::SetFalse
                ) && let Some(long) = argument.get_long()
                {
                    flags.push(format!("{path} --{long}"));
                }
            }
            for child in frozen_subcommands(command) {
                collect_boolean_flags(child, format!("{path} {}", child.get_name()), flags);
            }
        }

        let digest = format!("sha256:{}", "a".repeat(64));
        let cases = [
            (
                vec!["clean", "execute"],
                vec![
                    "--plan".to_string(),
                    "clean-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["software", "uninstall"],
                vec![
                    "--plan".to_string(),
                    "software-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["optimize", "run"],
                vec![
                    "--plan".to_string(),
                    "optimize-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest,
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["clean", "protect", "add"],
                vec![
                    "--path".to_string(),
                    "C:/protected".to_string(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["clean", "protect", "remove"],
                vec![
                    "--path".to_string(),
                    "C:/protected".to_string(),
                    "--confirm".to_string(),
                ],
            ),
        ];

        let mut discovered = Vec::new();
        collect_boolean_flags(
            &built_cli_command(),
            "devsweep".to_string(),
            &mut discovered,
        );
        discovered.sort();
        let mut expected = cases
            .iter()
            .map(|(path, _)| format!("devsweep {} --confirm", path.join(" ")))
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(
            discovered, expected,
            "every real boolean flag has a runtime case"
        );

        for (path, tail) in cases {
            let mut valid = vec!["devsweep".to_string()];
            valid.extend(path.iter().map(ToString::to_string));
            valid.extend(tail);
            Cli::try_parse_from(&valid).unwrap_or_else(|error| {
                panic!("value-less flag should parse for {path:?}: {error}")
            });

            let confirm = valid
                .iter()
                .position(|value| value == "--confirm")
                .expect("case contains --confirm");
            let mut invalid = valid.clone();
            invalid.insert(confirm + 1, "true".to_string());
            let error = Cli::try_parse_from(&invalid)
                .expect_err("--confirm true must not be accepted as a boolean value");
            assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);

            let mut help_args = vec![
                std::ffi::OsString::from("devsweep"),
                std::ffi::OsString::from("--language"),
                std::ffi::OsString::from("zh-CN"),
            ];
            help_args.extend(path.iter().map(std::ffi::OsString::from));
            help_args.push(std::ffi::OsString::from("--help"));
            let help = chinese_help_for(&help_args).expect("contextual Chinese help");
            let confirm_line = help
                .lines()
                .find(|line| line.starts_with("  ") && line.contains("--confirm"))
                .expect("Chinese help contains --confirm");
            assert!(confirm_line.contains("；必需"));
            assert!(!confirm_line.contains("<CONFIRM>"));
            assert!(!confirm_line.contains("；默认 false"));
            assert!(!confirm_line.contains("；取值 true|false"));
        }
    }

    #[test]
    fn built_tree_and_real_parser_preserve_inherited_language_semantics() {
        fn assert_language(command: &clap::Command, path: String) {
            let language = command
                .get_arguments()
                .find(|argument| argument.get_id() == "language")
                .unwrap_or_else(|| panic!("{path} does not inherit --language"));
            assert!(language.is_global_set(), "{path} language is not global");
            assert!(language.get_action().takes_values());
            assert_eq!(
                language
                    .get_value_parser()
                    .possible_values()
                    .expect("language values")
                    .map(|value| value.get_name().to_string())
                    .collect::<Vec<_>>(),
                ["en", "zh-CN"]
            );
            for child in frozen_subcommands(command) {
                assert_language(child, format!("{path} {}", child.get_name()));
            }
        }

        assert_language(&built_cli_command(), "devsweep".to_string());
        for args in [
            ["devsweep", "--language", "zh-CN", "status", "snapshot"],
            ["devsweep", "status", "snapshot", "--language", "zh-CN"],
        ] {
            let parsed = Cli::try_parse_from(args).expect("global language parses at either level");
            assert_eq!(parsed.language, Some(Language::ZhCn));
        }
    }

    #[test]
    fn every_clap_context_has_complete_contextual_chinese_help() {
        type ExpectedOption = (
            String,
            bool,
            bool,
            Vec<String>,
            Vec<String>,
            Option<&'static str>,
        );

        fn visit(
            command: &clap::Command,
            path: Vec<String>,
            contexts: &mut Vec<(Vec<String>, Vec<ExpectedOption>)>,
        ) {
            let options = command
                .get_arguments()
                .filter_map(|argument| {
                    let long = argument.get_long()?;
                    let takes_values = argument.get_action().takes_values();
                    let defaults = if takes_values {
                        argument
                            .get_default_values()
                            .iter()
                            .map(|value| value.to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };
                    let possible = if takes_values {
                        argument
                            .get_value_parser()
                            .possible_values()
                            .map(|values| {
                                values
                                    .map(|value| value.get_name().to_string())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    Some((
                        format!("--{long}"),
                        argument.is_required_set(),
                        takes_values,
                        defaults,
                        possible,
                        chinese_numeric_range(argument.get_id().as_str()),
                    ))
                })
                .collect::<Vec<_>>();
            contexts.push((path.clone(), options));
            for child in frozen_subcommands(command) {
                let mut child_path = path.clone();
                child_path.push(child.get_name().to_string());
                visit(child, child_path, contexts);
            }
        }

        let mut contexts = Vec::new();
        visit(&built_cli_command(), Vec::new(), &mut contexts);
        for (path, options) in contexts {
            let mut args = vec![
                std::ffi::OsString::from("devsweep"),
                std::ffi::OsString::from("--language"),
                std::ffi::OsString::from("zh-CN"),
            ];
            args.extend(path.iter().map(std::ffi::OsString::from));
            args.push(std::ffi::OsString::from("--help"));
            let help = chinese_help_for(&args).unwrap_or_else(|| {
                panic!("missing contextual Chinese help for {}", path.join(" "))
            });
            let expected_context = if path.is_empty() {
                "上下文：devsweep".to_string()
            } else {
                format!("上下文：devsweep {}", path.join(" "))
            };
            assert!(help.contains(&expected_context), "{expected_context}");
            assert!(
                help.contains(chinese_command_description(&path)),
                "{} omits its localized command description",
                path.join(" ")
            );
            assert!(
                !help.contains("冻结命令："),
                "generic root fallback: {path:?}"
            );
            for (option, required, takes_values, defaults, possible, range) in options {
                let line = help
                    .lines()
                    .find(|line| line.starts_with("  ") && line.contains(&option))
                    .unwrap_or_else(|| panic!("{} omits {option}", path.join(" ")));
                assert!(
                    line.contains(if required { "；必需" } else { "；可选" }),
                    "{} has incomplete required metadata for {option}: {line}",
                    path.join(" ")
                );
                if !takes_values {
                    assert!(
                        !line.contains('<') && !line.contains("；默认") && !line.contains("；取值"),
                        "{} renders value metadata for flag {option}: {line}",
                        path.join(" ")
                    );
                }
                if !defaults.is_empty() {
                    assert!(
                        line.contains(&format!("；默认 {}", defaults.join(","))),
                        "{} has incomplete default metadata for {option}: {line}",
                        path.join(" ")
                    );
                }
                if !possible.is_empty() {
                    assert!(
                        line.contains(&format!("；取值 {}", possible.join("|"))),
                        "{} has incomplete possible-value metadata for {option}: {line}",
                        path.join(" ")
                    );
                }
                if let Some(range) = range {
                    assert!(
                        line.contains(&format!("；范围 {range}")),
                        "{} has incomplete range metadata for {option}: {line}",
                        path.join(" ")
                    );
                }
            }
            for constraint in chinese_context_constraints(&path) {
                assert!(
                    help.contains(constraint),
                    "{} omits contextual constraint {constraint}",
                    path.join(" ")
                );
            }
        }
    }

    fn generated_reference_manifest() -> String {
        fn action_tag(action: &clap::ArgAction) -> &'static str {
            match action {
                clap::ArgAction::Set => "set",
                clap::ArgAction::Append => "append",
                clap::ArgAction::SetTrue => "set_true",
                clap::ArgAction::SetFalse => "set_false",
                clap::ArgAction::Count => "count",
                clap::ArgAction::Help => "help",
                clap::ArgAction::HelpShort => "help_short",
                clap::ArgAction::HelpLong => "help_long",
                clap::ArgAction::Version => "version",
                _ => "unknown",
            }
        }

        fn visit(command: &clap::Command, path: String, lines: &mut Vec<String>) {
            let mut subcommands = frozen_subcommands(command)
                .map(|child| child.get_name().to_string())
                .collect::<Vec<_>>();
            subcommands.sort();
            let mut arguments = command
                .get_arguments()
                .filter_map(|argument| {
                    let long = argument.get_long()?;
                    let takes_values = argument.get_action().takes_values();
                    let names = if takes_values {
                        argument
                            .get_value_names()
                            .map(|names| {
                                names
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(",")
                            })
                            .unwrap_or_else(|| "-".to_string())
                    } else {
                        "-".to_string()
                    };
                    let defaults = if takes_values {
                        argument
                            .get_default_values()
                            .iter()
                            .map(|value| value.to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };
                    let possible = if takes_values {
                        argument
                            .get_value_parser()
                            .possible_values()
                            .map(|values| {
                                values
                                    .map(|value| value.get_name().to_string())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    Some(format!(
                        "--{long}[action={};names={};required={};default={};values={};range={}]",
                        action_tag(argument.get_action()),
                        names,
                        argument.is_required_set(),
                        if defaults.is_empty() {
                            "-".to_string()
                        } else {
                            defaults.join(",")
                        },
                        if possible.is_empty() {
                            "-".to_string()
                        } else {
                            possible.join(",")
                        },
                        chinese_numeric_range(argument.get_id().as_str()).unwrap_or("-")
                    ))
                })
                .collect::<Vec<_>>();
            arguments.sort();
            lines.push(format!(
                "{path}|subcommands={}|options={}|constraints={}",
                if subcommands.is_empty() {
                    "-".to_string()
                } else {
                    subcommands.join(",")
                },
                if arguments.is_empty() {
                    "-".to_string()
                } else {
                    arguments.join(";")
                },
                context_constraint_codes(&path)
            ));
            for child in frozen_subcommands(command) {
                visit(child, format!("{path} {}", child.get_name()), lines);
            }
        }

        fn context_constraint_codes(path: &str) -> &'static str {
            match path.strip_prefix("devsweep ").unwrap_or("") {
                "clean scan" => "language_human,create_new,rescan_single_project_root",
                "clean plan" | "software plan" | "optimize plan" => {
                    "plan_only,no_language,no_format,create_new,input_output_distinct"
                }
                "clean preview" | "software preview" | "optimize preview" => {
                    "language_human,create_new,input_output_distinct"
                }
                "clean execute" | "software uninstall" | "optimize run" => {
                    "language_human,create_new,input_output_distinct,digest_confirm,no_prompt"
                }
                "clean protect add" | "clean protect remove" => "language_human,create_new,confirm",
                "status live" => "language_human,create_new,human_tty_output_conflict",
                "" => "bare_stdin_stdout_tty,language_session_only",
                _ => "language_human,create_new",
            }
        }

        let mut lines = Vec::new();
        visit(&built_cli_command(), "devsweep".to_string(), &mut lines);
        lines.join("\n")
    }

    fn documented_manifest(document: &str) -> String {
        const START: &str = "<!-- cli-contract-manifest:start -->\n```text\n";
        const END: &str = "\n```\n<!-- cli-contract-manifest:end -->";
        let document = document.replace("\r\n", "\n");
        let after_start = document
            .split_once(START)
            .expect("reference contains generated manifest start")
            .1;
        after_start
            .split_once(END)
            .expect("reference contains generated manifest end")
            .0
            .to_string()
    }

    #[test]
    fn english_and_chinese_references_exactly_match_the_clap_manifest() {
        let generated = generated_reference_manifest();
        for line in generated.lines() {
            assert!(
                line.contains(
                    "--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]"
                ),
                "manifest context omits inherited language semantics: {line}"
            );
        }
        assert!(!generated.contains("action=unknown"));
        assert!(generated.contains(
            "--confirm[action=set_true;names=-;required=true;default=-;values=-;range=-]"
        ));
        assert!(!generated.contains("--confirm[action=set_true;names=CONFIRM"));
        let english = include_str!("../../../../docs/reference/cli.md");
        let chinese = include_str!("../../../../docs/zh/reference/cli.md");
        assert_eq!(documented_manifest(english), generated);
        assert_eq!(documented_manifest(chinese), generated);
    }
}
