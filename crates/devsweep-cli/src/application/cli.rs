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
    /// Inspect and plan opt-in AI-tool cleanup.
    Ai(CommandGroup<CleanAiCommand>),
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
pub(super) enum CleanAiCommand {
    /// Observe one AI home or the production defaults.
    Scan(CleanAiScanCommand),
    /// Create a saved AI plan for one class.
    Plan(CleanAiPlanCommand),
    /// Preview a saved AI plan and calculate its live digest.
    Preview(PlanPresentationCommand),
    /// Move confirmed AI targets to the Recycle Bin after a recovery bundle.
    Execute(PlanExecutionCommand),
    /// List recovery bundles without restoring them.
    RecoveryList(CleanAiRecoveryListCommand),
    /// Create a restore plan for one recovery bundle.
    RestorePlan(CleanAiRestorePlanCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum AiProviderArg {
    #[value(name = "claude-code")]
    ClaudeCode,
    #[value(name = "codex")]
    Codex,
    #[value(name = "cursor")]
    Cursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum AiClassArg {
    #[value(name = "regenerable_cache")]
    RegenerableCache,
    #[value(name = "superseded_version")]
    SupersededVersion,
    #[value(name = "session_archive")]
    SessionArchive,
    #[value(name = "worktree")]
    Worktree,
    #[value(name = "installed_tool")]
    InstalledTool,
}

#[derive(Debug, Args)]
pub(super) struct CleanAiScanCommand {
    /// AI provider. Omit with `--home` to observe every production default.
    #[arg(long, value_enum, value_name = "PROVIDER")]
    pub(super) provider: Option<AiProviderArg>,
    /// Absolute provider home. Omit to use the production default.
    #[arg(long, value_name = "PATH", value_parser = parse_regular_path)]
    pub(super) home: Option<PathBuf>,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct CleanAiPlanCommand {
    /// Saved AI observation JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) scan: PathBuf,
    /// Data class to select.
    #[arg(long, value_enum, value_name = "CLASS")]
    pub(super) class: AiClassArg,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct CleanAiRecoveryListCommand {
    /// Absolute provider home.
    #[arg(long, value_name = "PATH", value_parser = parse_regular_path)]
    pub(super) home: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct CleanAiRestorePlanCommand {
    /// Absolute provider home.
    #[arg(long, value_name = "PATH", value_parser = parse_regular_path)]
    pub(super) home: PathBuf,
    /// Recovery bundle id.
    #[arg(long, value_name = "ID")]
    pub(super) bundle: String,
    /// New JSON restore plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
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
    /// Check package updates and apply one explicit package plan.
    Updates(CommandGroup<SoftwareUpdatesCommand>),
    /// List startup entries and toggle current-user or offline-hive approval.
    Startup(CommandGroup<SoftwareStartupCommand>),
    /// Review leftover folders and one closed registry-residue key.
    Leftovers(CommandGroup<SoftwareLeftoversCommand>),
}

#[derive(Debug, Subcommand)]
pub(super) enum SoftwareUpdatesCommand {
    /// Read available package updates.
    Check(SoftwareUpdatesCheckCommand),
    /// Create a saved plan for one package, source, and version.
    Plan(SoftwareUpdatesPlanCommand),
    /// Preview a saved package update plan.
    Preview(SoftwareUpdatesPreviewCommand),
    /// Apply one confirmed package update plan.
    Execute(SoftwareUpdatesExecuteCommand),
}

#[derive(Debug, Args)]
pub(super) struct SoftwareUpdatesCheckCommand {
    /// Saved software-inventory JSON file. Omit to skip name matching.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) inventory: Option<PathBuf>,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareUpdatesPlanCommand {
    /// Update provider program. Tests pass a command double, not installed winget.
    #[arg(
        long,
        id = "update_provider",
        value_name = "PROGRAM",
        default_value = "winget",
        value_parser = parse_regular_path
    )]
    pub(super) provider: PathBuf,
    /// Package id from the update table.
    #[arg(long = "package-id", id = "package_id", value_name = "PACKAGE_ID")]
    pub(super) package_id: String,
    /// Package source. Only `winget` is supported.
    #[arg(long, id = "update_source", value_name = "SOURCE")]
    pub(super) source: String,
    /// Exact available version to plan.
    #[arg(
        long = "available-version",
        id = "available_version",
        value_name = "VERSION"
    )]
    pub(super) version: String,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareUpdatesPreviewCommand {
    /// Update provider program. Tests pass a command double, not installed winget.
    #[arg(
        long,
        id = "update_provider",
        value_name = "PROGRAM",
        default_value = "winget",
        value_parser = parse_regular_path
    )]
    pub(super) provider: PathBuf,
    /// Saved versioned plan JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) plan: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareUpdatesExecuteCommand {
    /// Update provider program. Tests pass a command double, not installed winget.
    #[arg(
        long,
        id = "update_provider",
        value_name = "PROGRAM",
        default_value = "winget",
        value_parser = parse_regular_path
    )]
    pub(super) provider: PathBuf,
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
pub(super) enum SoftwareStartupCommand {
    /// List startup entries.
    List(SnapshotOutputArgs),
    /// Create a saved startup toggle plan.
    Plan(SoftwareStartupPlanCommand),
    /// Preview a saved startup toggle plan.
    Preview(PlanPresentationCommand),
    /// Apply a confirmed startup toggle plan.
    Execute(PlanExecutionCommand),
    /// Create a startup toggle plan from an offline fixture hive.
    HivePlan(SoftwareStartupHivePlanCommand),
    /// Preview a saved offline-hive startup plan.
    HivePreview(SoftwareStartupHivePreviewCommand),
    /// Apply a confirmed offline-hive startup plan.
    HiveExecute(SoftwareStartupHiveExecuteCommand),
    /// Create a restore plan from a hive execute report.
    RestorePlan(SoftwareStartupRestorePlanCommand),
    /// Preview a saved offline-hive restore plan.
    RestorePreview(SoftwareStartupRestorePreviewCommand),
    /// Restore one offline-hive startup entry when its identity still matches.
    RestoreExecute(SoftwareStartupRestoreExecuteCommand),
}

#[derive(Debug, Args)]
pub(super) struct SoftwareStartupPlanCommand {
    /// Stable startup entry identity.
    #[arg(long, value_name = "ENTRY_ID")]
    pub(super) entry: String,
    /// Requested enabled state, `true` or `false`.
    #[arg(
        long,
        value_name = "ENABLED",
        action = clap::ArgAction::Set,
        value_parser = parse_enabled
    )]
    pub(super) enabled: bool,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareStartupHivePlanCommand {
    /// Offline startup fixture hive. This is not host HKCU or HKLM.
    #[arg(long, id = "hive", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) hive: PathBuf,
    /// Stable startup entry identity.
    #[arg(long, value_name = "ENTRY_ID")]
    pub(super) entry: String,
    /// Requested enabled state, `true` or `false`.
    #[arg(
        long,
        value_name = "ENABLED",
        action = clap::ArgAction::Set,
        value_parser = parse_enabled
    )]
    pub(super) enabled: bool,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareStartupHivePreviewCommand {
    /// Offline startup fixture hive. This is not host HKCU or HKLM.
    #[arg(long, id = "hive", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) hive: PathBuf,
    /// Saved versioned plan JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) plan: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareStartupHiveExecuteCommand {
    /// Offline startup fixture hive. This is not host HKCU or HKLM.
    #[arg(long, id = "hive", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) hive: PathBuf,
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

#[derive(Debug, Args)]
pub(super) struct SoftwareStartupRestorePlanCommand {
    /// Hive execute report that contains a backup.
    #[arg(long, id = "startup_report", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) report: PathBuf,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareStartupRestorePreviewCommand {
    /// Offline startup fixture hive. This is not host HKCU or HKLM.
    #[arg(long, id = "hive", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) hive: PathBuf,
    /// Saved versioned plan JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) plan: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareStartupRestoreExecuteCommand {
    /// Offline startup fixture hive. This is not host HKCU or HKLM.
    #[arg(long, id = "hive", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) hive: PathBuf,
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
pub(super) enum SoftwareLeftoversCommand {
    /// Scan leftover candidates from an inventory.
    Scan(SoftwareLeftoversScanCommand),
    /// Create a saved leftover removal plan.
    Plan(SoftwareLeftoversPlanCommand),
    /// Preview a saved leftover plan.
    Preview(SoftwareLeftoversPreviewCommand),
    /// Move confirmed leftover directories to the Recycle Bin.
    Execute(SoftwareLeftoversExecuteCommand),
    /// Plan removal of one closed registry-residue fixture key.
    RegistryPlan(SoftwareLeftoverRegistryPlanCommand),
    /// Preview one closed registry-residue plan.
    RegistryPreview(SoftwareLeftoverRegistryPreviewCommand),
    /// Back up and change one closed registry-residue fixture key.
    RegistryExecute(SoftwareLeftoverRegistryExecuteCommand),
    /// Create a restore plan from a residue execute report.
    RegistryRestorePlan(SoftwareLeftoverRegistryRestorePlanCommand),
    /// Preview a residue restore against the live fixture bytes.
    RegistryRestorePreview(SoftwareLeftoverRegistryRestorePreviewCommand),
    /// Restore one residue key when its bytes still match the backup.
    RegistryRestoreExecute(SoftwareLeftoverRegistryRestoreExecuteCommand),
}

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoversScanCommand {
    /// Saved software-inventory JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) inventory: PathBuf,
    /// Stable software identity to scan. Supply one or more values.
    #[arg(long, value_name = "SOFTWARE_ID", required = true, num_args = 1..)]
    pub(super) select: Vec<String>,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoversPlanCommand {
    /// Saved software-inventory JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) inventory: PathBuf,
    /// Stable software identity.
    #[arg(long, value_name = "SOFTWARE_ID")]
    pub(super) software_id: String,
    /// Succeeded uninstall operation identity.
    #[arg(long, value_name = "OPERATION_ID")]
    pub(super) uninstall_operation: String,
    /// Leftover candidate identity. Supply one or more values.
    #[arg(long, value_name = "CANDIDATE_ID", required = true, num_args = 1..)]
    pub(super) select: Vec<String>,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoversPreviewCommand {
    /// Saved software-inventory JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) inventory: PathBuf,
    /// Saved versioned plan JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) plan: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoversExecuteCommand {
    /// Saved software-inventory JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) inventory: PathBuf,
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

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoverRegistryPlanCommand {
    /// Closed registry-residue fixture key. The file name must be DevSweepResidue.
    #[arg(long, id = "residue_key", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) key: PathBuf,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoverRegistryPreviewCommand {
    /// Closed registry-residue fixture key. The file name must be DevSweepResidue.
    #[arg(long, id = "residue_key", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) key: PathBuf,
    /// Saved versioned plan JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) plan: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoverRegistryExecuteCommand {
    /// Closed registry-residue fixture key. The file name must be DevSweepResidue.
    #[arg(long, id = "residue_key", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) key: PathBuf,
    /// Exact-byte backup written before the key changes.
    #[arg(long, id = "backup", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) backup: PathBuf,
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

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoverRegistryRestorePlanCommand {
    /// Residue execute report that contains the backup digest.
    #[arg(long, id = "residue", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) report: PathBuf,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoverRegistryRestorePreviewCommand {
    /// Closed registry-residue fixture key. The file name must be DevSweepResidue.
    #[arg(long, id = "residue_key", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) key: PathBuf,
    /// Exact-byte backup written before the key changes.
    #[arg(long, id = "backup", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) backup: PathBuf,
    /// Saved versioned plan JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) plan: PathBuf,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct SoftwareLeftoverRegistryRestoreExecuteCommand {
    /// Closed registry-residue fixture key. The file name must be DevSweepResidue.
    #[arg(long, id = "residue_key", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) key: PathBuf,
    /// Exact-byte backup written before the key changes.
    #[arg(long, id = "backup", value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) backup: PathBuf,
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
    /// Print read-only disk health. This command does not repair.
    Health(SnapshotOutputArgs),
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
    /// List volumes the snapshot source can see.
    Roots(AnalyzeRootsCommand),
    /// List bounded children of one snapshot node.
    List(AnalyzeListCommand),
    /// Explain one snapshot node. This command does not create a cleanup plan.
    Explain(AnalyzeExplainCommand),
    /// Show one snapshot node in Explorer.
    Reveal(AnalyzeRevealCommand),
    /// Create a saved Recycle Bin plan from a snapshot.
    TrashPlan(AnalyzeTrashPlanCommand),
    /// Preview a saved Analyze Recycle Bin plan.
    TrashPreview(PlanPresentationCommand),
    /// Move confirmed snapshot nodes to the Recycle Bin.
    TrashExecute(PlanExecutionCommand),
}

#[derive(Debug, Args)]
pub(super) struct AnalyzeRootsCommand {
    /// Root to observe. Repeat to observe more than one. A missing root is an error for that root.
    #[arg(long, value_name = "PATH")]
    pub(super) root: Vec<PathBuf>,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AnalyzeListCommand {
    /// Saved analysis snapshot JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) snapshot: PathBuf,
    /// Snapshot node identity.
    #[arg(long, value_name = "NODE_ID")]
    pub(super) node: u32,
    /// Child rows to skip.
    #[arg(
        id = "child_offset",
        long = "offset",
        value_name = "OFFSET",
        default_value_t = 0
    )]
    pub(super) offset: u32,
    /// Maximum child rows.
    #[arg(
        id = "child_limit",
        long = "limit",
        value_name = "LIMIT",
        value_parser = clap::value_parser!(u32).range(1..=1000),
        default_value_t = 100
    )]
    pub(super) limit: u32,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AnalyzeExplainCommand {
    /// Saved analysis snapshot JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) snapshot: PathBuf,
    /// Snapshot node identity.
    #[arg(long, value_name = "NODE_ID")]
    pub(super) node: u32,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AnalyzeRevealCommand {
    /// Saved analysis snapshot JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) snapshot: PathBuf,
    /// Snapshot node identity.
    #[arg(long, value_name = "NODE_ID")]
    pub(super) node: u32,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct AnalyzeTrashPlanCommand {
    /// Saved analysis snapshot JSON file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) snapshot: PathBuf,
    /// Snapshot node identity to select. Supply one or more values.
    #[arg(long, value_name = "NODE_ID", required = true, num_args = 1..)]
    pub(super) select: Vec<u32>,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
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
    /// Explain one snapshot as named evidence. This command does not score the host.
    Health(StatusHealthCommand),
    /// Review one process termination.
    Process(CommandGroup<StatusProcessCommand>),
    /// Hold one finite in-memory keep-awake lease.
    KeepAwake(CommandGroup<StatusKeepAwakeCommand>),
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

#[derive(Debug, Args)]
pub(super) struct StatusHealthCommand {
    /// Maximum process rows.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100), default_value_t = 15)]
    pub(super) process_limit: u32,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Subcommand)]
pub(super) enum StatusProcessCommand {
    /// Bind a PID and creation time into a saved plan.
    Plan(StatusProcessPlanCommand),
    /// Preview a process plan, including data-loss text.
    Preview(PlanPresentationCommand),
    /// Terminate one reviewed process after confirmation.
    Terminate(PlanExecutionCommand),
}

#[derive(Debug, Args)]
pub(super) struct StatusProcessPlanCommand {
    /// Process id bound by the plan.
    #[arg(long, value_name = "PID")]
    pub(super) pid: u32,
    /// Windows process creation FILETIME bound by the plan.
    #[arg(long, value_name = "FILETIME")]
    pub(super) creation_time: u64,
    /// Process file name bound by the plan.
    #[arg(long, value_name = "NAME")]
    pub(super) name: String,
    /// New JSON plan file.
    #[arg(long, value_name = "FILE", value_parser = parse_regular_path)]
    pub(super) output: PathBuf,
}

#[derive(Debug, Subcommand)]
pub(super) enum StatusKeepAwakeCommand {
    /// Start one finite in-memory lease.
    Run(StatusKeepAwakeRunCommand),
    /// Read one lease by owner and token.
    Status(StatusKeepAwakeTokenCommand),
    /// Release one lease by owner and token.
    Stop(StatusKeepAwakeTokenCommand),
}

#[derive(Debug, Args)]
pub(super) struct StatusKeepAwakeRunCommand {
    /// Lease length in milliseconds. The value is required, finite, and at most 12 hours.
    #[arg(long, value_name = "DURATION_MS", value_parser = clap::value_parser!(u64).range(1..=43_200_000))]
    pub(super) duration_ms: u64,
    /// Lease owner. Stop and exit release only this owner.
    #[arg(long, value_name = "OWNER")]
    pub(super) owner: String,
    /// Confirm the finite keep-awake request.
    #[arg(long, required = true, action = clap::ArgAction::SetTrue)]
    pub(super) confirm: bool,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
}

#[derive(Debug, Args)]
pub(super) struct StatusKeepAwakeTokenCommand {
    /// Session token printed by keep-awake run.
    #[arg(long, value_name = "TOKEN")]
    pub(super) token: String,
    /// Owner that received the token.
    #[arg(long, value_name = "OWNER")]
    pub(super) owner: String,
    /// Loopback endpoint printed by the owning run process.
    #[arg(long, value_name = "ENDPOINT")]
    pub(super) endpoint: String,
    #[command(flatten)]
    pub(super) output: SnapshotOutputArgs,
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
                CleanCommand::Ai(group) => match &group.command {
                    CleanAiCommand::Scan(command) => {
                        if command.home.is_some() && command.provider.is_none() {
                            return Err("--home requires --provider".to_string());
                        }
                        if let Some(home) = &command.home
                            && !home.is_absolute()
                        {
                            return Err("--home must be absolute".to_string());
                        }
                    }
                    CleanAiCommand::Plan(command) => {
                        reject_same_path(&command.scan, &command.output)?;
                    }
                    CleanAiCommand::Preview(command) => validate_plan_output(command)?,
                    CleanAiCommand::Execute(command) => validate_execution_output(command)?,
                    CleanAiCommand::RecoveryList(command) => {
                        if !command.home.is_absolute() {
                            return Err("--home must be absolute".to_string());
                        }
                    }
                    CleanAiCommand::RestorePlan(command) => {
                        if !command.home.is_absolute() {
                            return Err("--home must be absolute".to_string());
                        }
                        reject_same_path(&command.home, &command.output)?;
                    }
                },
            },
            Some(Command::Software(group)) => match &group.command {
                SoftwareCommand::Plan(command) => {
                    reject_same_path(&command.inventory, &command.output)?
                }
                SoftwareCommand::Preview(command) => validate_plan_output(command)?,
                SoftwareCommand::Uninstall(command) => validate_execution_output(command)?,
                SoftwareCommand::Inventory(_) => {}
                SoftwareCommand::Updates(group) => match &group.command {
                    SoftwareUpdatesCommand::Check(_) => {}
                    SoftwareUpdatesCommand::Plan(command) => {
                        reject_same_path(&command.provider, &command.output)?
                    }
                    SoftwareUpdatesCommand::Preview(command) => {
                        reject_same_path(&command.provider, &command.plan)?;
                        if let Some(output) = command.output.output.as_deref() {
                            reject_same_path(&command.plan, output)?;
                            reject_same_path(&command.provider, output)?;
                        }
                    }
                    SoftwareUpdatesCommand::Execute(command) => {
                        reject_same_path(&command.provider, &command.plan)?;
                        if let Some(output) = command.output.output.as_deref() {
                            reject_same_path(&command.plan, output)?;
                            reject_same_path(&command.provider, output)?;
                        }
                    }
                },
                SoftwareCommand::Startup(group) => match &group.command {
                    SoftwareStartupCommand::Plan(_) | SoftwareStartupCommand::List(_) => {}
                    SoftwareStartupCommand::Preview(command) => validate_plan_output(command)?,
                    SoftwareStartupCommand::Execute(command) => validate_execution_output(command)?,
                    SoftwareStartupCommand::HivePlan(command) => {
                        reject_same_path(&command.hive, &command.output)?
                    }
                    SoftwareStartupCommand::HivePreview(command) => reject_distinct_output(
                        &command.hive,
                        &command.plan,
                        command.output.output.as_deref(),
                    )?,
                    SoftwareStartupCommand::HiveExecute(command) => reject_distinct_output(
                        &command.hive,
                        &command.plan,
                        command.output.output.as_deref(),
                    )?,
                    SoftwareStartupCommand::RestorePlan(command) => {
                        reject_same_path(&command.report, &command.output)?
                    }
                    SoftwareStartupCommand::RestorePreview(command) => reject_distinct_output(
                        &command.hive,
                        &command.plan,
                        command.output.output.as_deref(),
                    )?,
                    SoftwareStartupCommand::RestoreExecute(command) => reject_distinct_output(
                        &command.hive,
                        &command.plan,
                        command.output.output.as_deref(),
                    )?,
                },
                SoftwareCommand::Leftovers(group) => match &group.command {
                    SoftwareLeftoversCommand::Scan(_) => {}
                    SoftwareLeftoversCommand::Plan(command) => {
                        reject_same_path(&command.inventory, &command.output)?
                    }
                    SoftwareLeftoversCommand::Preview(command) => {
                        reject_same_path(&command.inventory, &command.plan)?;
                        if let Some(output) = command.output.output.as_deref() {
                            reject_same_path(&command.plan, output)?;
                            reject_same_path(&command.inventory, output)?;
                        }
                    }
                    SoftwareLeftoversCommand::Execute(command) => {
                        reject_same_path(&command.inventory, &command.plan)?;
                        if let Some(output) = command.output.output.as_deref() {
                            reject_same_path(&command.plan, output)?;
                            reject_same_path(&command.inventory, output)?;
                        }
                    }
                    SoftwareLeftoversCommand::RegistryPlan(command) => {
                        reject_same_path(&command.key, &command.output)?
                    }
                    SoftwareLeftoversCommand::RegistryPreview(command) => reject_distinct_output(
                        &command.key,
                        &command.plan,
                        command.output.output.as_deref(),
                    )?,
                    SoftwareLeftoversCommand::RegistryExecute(command) => {
                        reject_same_path(&command.key, &command.backup)?;
                        reject_same_path(&command.key, &command.plan)?;
                        reject_same_path(&command.backup, &command.plan)?;
                        if let Some(output) = command.output.output.as_deref() {
                            reject_same_path(&command.key, output)?;
                            reject_same_path(&command.backup, output)?;
                            reject_same_path(&command.plan, output)?;
                        }
                    }
                    SoftwareLeftoversCommand::RegistryRestorePlan(command) => {
                        reject_same_path(&command.report, &command.output)?
                    }
                    SoftwareLeftoversCommand::RegistryRestorePreview(command) => {
                        reject_same_path(&command.key, &command.backup)?;
                        reject_same_path(&command.key, &command.plan)?;
                        reject_same_path(&command.backup, &command.plan)?;
                        if let Some(output) = command.output.output.as_deref() {
                            reject_same_path(&command.key, output)?;
                            reject_same_path(&command.backup, output)?;
                            reject_same_path(&command.plan, output)?;
                        }
                    }
                    SoftwareLeftoversCommand::RegistryRestoreExecute(command) => {
                        reject_same_path(&command.key, &command.backup)?;
                        reject_same_path(&command.key, &command.plan)?;
                        reject_same_path(&command.backup, &command.plan)?;
                        if let Some(output) = command.output.output.as_deref() {
                            reject_same_path(&command.key, output)?;
                            reject_same_path(&command.backup, output)?;
                            reject_same_path(&command.plan, output)?;
                        }
                    }
                },
            },
            Some(Command::Optimize(group)) => match &group.command {
                OptimizeCommand::Preview(command) => validate_plan_output(command)?,
                OptimizeCommand::Run(command) => validate_execution_output(command)?,
                OptimizeCommand::List(_)
                | OptimizeCommand::Plan(_)
                | OptimizeCommand::Health(_) => {}
            },
            Some(Command::Status(group)) => match &group.command {
                StatusCommand::Live(command)
                    if command.format == StreamFormat::Human && command.output.is_some() =>
                {
                    return Err(
                        "status live --format human conflicts with --output; use --format ndjson"
                            .to_string(),
                    );
                }
                StatusCommand::Process(group) => match &group.command {
                    StatusProcessCommand::Plan(_) => {}
                    StatusProcessCommand::Preview(command) => validate_plan_output(command)?,
                    StatusProcessCommand::Terminate(command) => validate_execution_output(command)?,
                },
                StatusCommand::Snapshot(_)
                | StatusCommand::Live(_)
                | StatusCommand::Health(_)
                | StatusCommand::KeepAwake(_) => {}
            },
            Some(Command::Analyze(group)) => match &group.command {
                AnalyzeCommand::Scan(_)
                | AnalyzeCommand::Roots(_)
                | AnalyzeCommand::List(_)
                | AnalyzeCommand::Explain(_)
                | AnalyzeCommand::Reveal(_) => {}
                AnalyzeCommand::TrashPlan(command) => {
                    reject_same_path(&command.snapshot, &command.output)?
                }
                AnalyzeCommand::TrashPreview(command) => validate_plan_output(command)?,
                AnalyzeCommand::TrashExecute(command) => validate_execution_output(command)?,
            },
            Some(Command::History(_)) | None => {}
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
                CleanCommand::Ai(group) => match &group.command {
                    CleanAiCommand::Scan(_) => "clean.ai.scan",
                    CleanAiCommand::Plan(_) => "clean.ai.plan",
                    CleanAiCommand::Preview(_) => "clean.ai.preview",
                    CleanAiCommand::Execute(_) => "clean.ai.execute",
                    CleanAiCommand::RecoveryList(_) => "clean.ai.recovery-list",
                    CleanAiCommand::RestorePlan(_) => "clean.ai.restore-plan",
                },
            }),
            Command::Software(group) => Some(match &group.command {
                SoftwareCommand::Inventory(_) => "software.inventory",
                SoftwareCommand::Plan(_) => "software.plan",
                SoftwareCommand::Preview(_) => "software.preview",
                SoftwareCommand::Uninstall(_) => "software.uninstall",
                SoftwareCommand::Updates(group) => match &group.command {
                    SoftwareUpdatesCommand::Check(_) => "software.updates.check",
                    SoftwareUpdatesCommand::Plan(_) => "software.updates.plan",
                    SoftwareUpdatesCommand::Preview(_) => "software.updates.preview",
                    SoftwareUpdatesCommand::Execute(_) => "software.updates.execute",
                },
                SoftwareCommand::Startup(group) => match &group.command {
                    SoftwareStartupCommand::List(_) => "software.startup.list",
                    SoftwareStartupCommand::Plan(_) => "software.startup.plan",
                    SoftwareStartupCommand::Preview(_) => "software.startup.preview",
                    SoftwareStartupCommand::Execute(_) => "software.startup.execute",
                    SoftwareStartupCommand::HivePlan(_) => "software.startup.hive-plan",
                    SoftwareStartupCommand::HivePreview(_) => "software.startup.hive-preview",
                    SoftwareStartupCommand::HiveExecute(_) => "software.startup.hive-execute",
                    SoftwareStartupCommand::RestorePlan(_) => "software.startup.restore-plan",
                    SoftwareStartupCommand::RestorePreview(_) => "software.startup.restore-preview",
                    SoftwareStartupCommand::RestoreExecute(_) => "software.startup.restore-execute",
                },
                SoftwareCommand::Leftovers(group) => match &group.command {
                    SoftwareLeftoversCommand::Scan(_) => "software.leftovers.scan",
                    SoftwareLeftoversCommand::Plan(_) => "software.leftovers.plan",
                    SoftwareLeftoversCommand::Preview(_) => "software.leftovers.preview",
                    SoftwareLeftoversCommand::Execute(_) => "software.leftovers.execute",
                    SoftwareLeftoversCommand::RegistryPlan(_) => "software.leftovers.registry-plan",
                    SoftwareLeftoversCommand::RegistryPreview(_) => {
                        "software.leftovers.registry-preview"
                    }
                    SoftwareLeftoversCommand::RegistryExecute(_) => {
                        "software.leftovers.registry-execute"
                    }
                    SoftwareLeftoversCommand::RegistryRestorePlan(_) => {
                        "software.leftovers.registry-restore-plan"
                    }
                    SoftwareLeftoversCommand::RegistryRestorePreview(_) => {
                        "software.leftovers.registry-restore-preview"
                    }
                    SoftwareLeftoversCommand::RegistryRestoreExecute(_) => {
                        "software.leftovers.registry-restore-execute"
                    }
                },
            }),
            Command::Optimize(group) => Some(match &group.command {
                OptimizeCommand::List(_) => "optimize.list",
                OptimizeCommand::Plan(_) => "optimize.plan",
                OptimizeCommand::Preview(_) => "optimize.preview",
                OptimizeCommand::Run(_) => "optimize.run",
                OptimizeCommand::Health(_) => "optimize.health",
            }),
            Command::Analyze(group) => Some(match &group.command {
                AnalyzeCommand::Scan(_) => "analyze.scan",
                AnalyzeCommand::Roots(_) => "analyze.roots",
                AnalyzeCommand::List(_) => "analyze.list",
                AnalyzeCommand::Explain(_) => "analyze.explain",
                AnalyzeCommand::Reveal(_) => "analyze.reveal",
                AnalyzeCommand::TrashPlan(_) => "analyze.trash-plan",
                AnalyzeCommand::TrashPreview(_) => "analyze.trash-preview",
                AnalyzeCommand::TrashExecute(_) => "analyze.trash-execute",
            }),
            Command::Status(group) => Some(match &group.command {
                StatusCommand::Snapshot(_) => "status.snapshot",
                StatusCommand::Live(_) => "status.live",
                StatusCommand::Health(_) => "status.health",
                StatusCommand::Process(group) => match &group.command {
                    StatusProcessCommand::Plan(_) => "status.process.plan",
                    StatusProcessCommand::Preview(_) => "status.process.preview",
                    StatusProcessCommand::Terminate(_) => "status.process.terminate",
                },
                StatusCommand::KeepAwake(group) => match &group.command {
                    StatusKeepAwakeCommand::Run(_) => "status.keep-awake.run",
                    StatusKeepAwakeCommand::Status(_) => "status.keep-awake.status",
                    StatusKeepAwakeCommand::Stop(_) => "status.keep-awake.stop",
                },
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
                CleanCommand::Ai(group) => match &group.command {
                    CleanAiCommand::Scan(command) => command.output.format.into(),
                    CleanAiCommand::Plan(_) | CleanAiCommand::RestorePlan(_) => {
                        ResultFormat::PlanJson
                    }
                    CleanAiCommand::Preview(command) => command.output.format.into(),
                    CleanAiCommand::Execute(command) => command.output.format.into(),
                    CleanAiCommand::RecoveryList(command) => command.output.format.into(),
                },
            },
            Some(Command::Software(group)) => match &group.command {
                SoftwareCommand::Inventory(command) => command.output.format.into(),
                SoftwareCommand::Plan(_) => ResultFormat::PlanJson,
                SoftwareCommand::Preview(command) => command.output.format.into(),
                SoftwareCommand::Uninstall(command) => command.output.format.into(),
                SoftwareCommand::Updates(group) => match &group.command {
                    SoftwareUpdatesCommand::Check(command) => command.output.format.into(),
                    SoftwareUpdatesCommand::Plan(_) => ResultFormat::PlanJson,
                    SoftwareUpdatesCommand::Preview(command) => command.output.format.into(),
                    SoftwareUpdatesCommand::Execute(command) => command.output.format.into(),
                },
                SoftwareCommand::Startup(group) => match &group.command {
                    SoftwareStartupCommand::List(output) => output.format.into(),
                    SoftwareStartupCommand::Plan(_)
                    | SoftwareStartupCommand::HivePlan(_)
                    | SoftwareStartupCommand::RestorePlan(_) => ResultFormat::PlanJson,
                    SoftwareStartupCommand::Preview(command) => command.output.format.into(),
                    SoftwareStartupCommand::Execute(command) => command.output.format.into(),
                    SoftwareStartupCommand::HivePreview(command) => command.output.format.into(),
                    SoftwareStartupCommand::HiveExecute(command) => command.output.format.into(),
                    SoftwareStartupCommand::RestorePreview(command) => command.output.format.into(),
                    SoftwareStartupCommand::RestoreExecute(command) => command.output.format.into(),
                },
                SoftwareCommand::Leftovers(group) => match &group.command {
                    SoftwareLeftoversCommand::Scan(command) => command.output.format.into(),
                    SoftwareLeftoversCommand::Plan(_)
                    | SoftwareLeftoversCommand::RegistryPlan(_)
                    | SoftwareLeftoversCommand::RegistryRestorePlan(_) => ResultFormat::PlanJson,
                    SoftwareLeftoversCommand::Preview(command) => command.output.format.into(),
                    SoftwareLeftoversCommand::Execute(command) => command.output.format.into(),
                    SoftwareLeftoversCommand::RegistryPreview(command) => {
                        command.output.format.into()
                    }
                    SoftwareLeftoversCommand::RegistryExecute(command) => {
                        command.output.format.into()
                    }
                    SoftwareLeftoversCommand::RegistryRestorePreview(command) => {
                        command.output.format.into()
                    }
                    SoftwareLeftoversCommand::RegistryRestoreExecute(command) => {
                        command.output.format.into()
                    }
                },
            },
            Some(Command::Optimize(group)) => match &group.command {
                OptimizeCommand::List(output) => output.format.into(),
                OptimizeCommand::Plan(_) => ResultFormat::PlanJson,
                OptimizeCommand::Preview(command) => command.output.format.into(),
                OptimizeCommand::Run(command) => command.output.format.into(),
                OptimizeCommand::Health(output) => output.format.into(),
            },
            Some(Command::Analyze(group)) => match &group.command {
                AnalyzeCommand::Scan(command) => command.output.format.into(),
                AnalyzeCommand::Roots(command) => command.output.format.into(),
                AnalyzeCommand::List(command) => command.output.format.into(),
                AnalyzeCommand::Explain(command) => command.output.format.into(),
                AnalyzeCommand::Reveal(command) => command.output.format.into(),
                AnalyzeCommand::TrashPlan(_) => ResultFormat::PlanJson,
                AnalyzeCommand::TrashPreview(command) => command.output.format.into(),
                AnalyzeCommand::TrashExecute(command) => command.output.format.into(),
            },
            Some(Command::Status(group)) => match &group.command {
                StatusCommand::Snapshot(command) => command.output.format.into(),
                StatusCommand::Live(command) => match command.format {
                    StreamFormat::Human => ResultFormat::Human,
                    StreamFormat::Ndjson => ResultFormat::Ndjson,
                },
                StatusCommand::Health(command) => command.output.format.into(),
                StatusCommand::Process(group) => match &group.command {
                    StatusProcessCommand::Plan(_) => ResultFormat::PlanJson,
                    StatusProcessCommand::Preview(command) => command.output.format.into(),
                    StatusProcessCommand::Terminate(command) => command.output.format.into(),
                },
                StatusCommand::KeepAwake(group) => match &group.command {
                    StatusKeepAwakeCommand::Run(command) => command.output.format.into(),
                    StatusKeepAwakeCommand::Status(command)
                    | StatusKeepAwakeCommand::Stop(command) => command.output.format.into(),
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
                CleanCommand::Ai(group) => match &group.command {
                    CleanAiCommand::Scan(command) => command.output.output.as_deref(),
                    CleanAiCommand::Plan(command) => Some(command.output.as_path()),
                    CleanAiCommand::Preview(command) => command.output.output.as_deref(),
                    CleanAiCommand::Execute(command) => command.output.output.as_deref(),
                    CleanAiCommand::RecoveryList(command) => command.output.output.as_deref(),
                    CleanAiCommand::RestorePlan(command) => Some(command.output.as_path()),
                },
            },
            Command::Software(group) => match &group.command {
                SoftwareCommand::Inventory(command) => command.output.output.as_deref(),
                SoftwareCommand::Plan(command) => Some(command.output.as_path()),
                SoftwareCommand::Preview(command) => command.output.output.as_deref(),
                SoftwareCommand::Uninstall(command) => command.output.output.as_deref(),
                SoftwareCommand::Updates(group) => match &group.command {
                    SoftwareUpdatesCommand::Check(command) => command.output.output.as_deref(),
                    SoftwareUpdatesCommand::Plan(command) => Some(command.output.as_path()),
                    SoftwareUpdatesCommand::Preview(command) => command.output.output.as_deref(),
                    SoftwareUpdatesCommand::Execute(command) => command.output.output.as_deref(),
                },
                SoftwareCommand::Startup(group) => match &group.command {
                    SoftwareStartupCommand::List(output) => output.output.as_deref(),
                    SoftwareStartupCommand::Plan(command) => Some(command.output.as_path()),
                    SoftwareStartupCommand::Preview(command) => command.output.output.as_deref(),
                    SoftwareStartupCommand::Execute(command) => command.output.output.as_deref(),
                    SoftwareStartupCommand::HivePlan(command) => Some(command.output.as_path()),
                    SoftwareStartupCommand::HivePreview(command) => {
                        command.output.output.as_deref()
                    }
                    SoftwareStartupCommand::HiveExecute(command) => {
                        command.output.output.as_deref()
                    }
                    SoftwareStartupCommand::RestorePlan(command) => Some(command.output.as_path()),
                    SoftwareStartupCommand::RestorePreview(command) => {
                        command.output.output.as_deref()
                    }
                    SoftwareStartupCommand::RestoreExecute(command) => {
                        command.output.output.as_deref()
                    }
                },
                SoftwareCommand::Leftovers(group) => match &group.command {
                    SoftwareLeftoversCommand::Scan(command) => command.output.output.as_deref(),
                    SoftwareLeftoversCommand::Plan(command) => Some(command.output.as_path()),
                    SoftwareLeftoversCommand::Preview(command) => command.output.output.as_deref(),
                    SoftwareLeftoversCommand::Execute(command) => command.output.output.as_deref(),
                    SoftwareLeftoversCommand::RegistryPlan(command) => {
                        Some(command.output.as_path())
                    }
                    SoftwareLeftoversCommand::RegistryPreview(command) => {
                        command.output.output.as_deref()
                    }
                    SoftwareLeftoversCommand::RegistryExecute(command) => {
                        command.output.output.as_deref()
                    }
                    SoftwareLeftoversCommand::RegistryRestorePlan(command) => {
                        Some(command.output.as_path())
                    }
                    SoftwareLeftoversCommand::RegistryRestorePreview(command) => {
                        command.output.output.as_deref()
                    }
                    SoftwareLeftoversCommand::RegistryRestoreExecute(command) => {
                        command.output.output.as_deref()
                    }
                },
            },
            Command::Optimize(group) => match &group.command {
                OptimizeCommand::List(output) => output.output.as_deref(),
                OptimizeCommand::Plan(command) => Some(command.output.as_path()),
                OptimizeCommand::Preview(command) => command.output.output.as_deref(),
                OptimizeCommand::Run(command) => command.output.output.as_deref(),
                OptimizeCommand::Health(output) => output.output.as_deref(),
            },
            Command::Analyze(group) => match &group.command {
                AnalyzeCommand::Scan(command) => command.output.output.as_deref(),
                AnalyzeCommand::Roots(command) => command.output.output.as_deref(),
                AnalyzeCommand::List(command) => command.output.output.as_deref(),
                AnalyzeCommand::Explain(command) => command.output.output.as_deref(),
                AnalyzeCommand::Reveal(command) => command.output.output.as_deref(),
                AnalyzeCommand::TrashPlan(command) => Some(command.output.as_path()),
                AnalyzeCommand::TrashPreview(command) => command.output.output.as_deref(),
                AnalyzeCommand::TrashExecute(command) => command.output.output.as_deref(),
            },
            Command::Status(group) => match &group.command {
                StatusCommand::Snapshot(command) => command.output.output.as_deref(),
                StatusCommand::Live(command) => command.output.as_deref(),
                StatusCommand::Health(command) => command.output.output.as_deref(),
                StatusCommand::Process(group) => match &group.command {
                    StatusProcessCommand::Plan(command) => Some(command.output.as_path()),
                    StatusProcessCommand::Preview(command) => command.output.output.as_deref(),
                    StatusProcessCommand::Terminate(command) => command.output.output.as_deref(),
                },
                StatusCommand::KeepAwake(group) => match &group.command {
                    StatusKeepAwakeCommand::Run(command) => command.output.output.as_deref(),
                    StatusKeepAwakeCommand::Status(command)
                    | StatusKeepAwakeCommand::Stop(command) => command.output.output.as_deref(),
                },
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
                CleanCommand::Ai(group) => match &group.command {
                    CleanAiCommand::Plan(_) | CleanAiCommand::RestorePlan(_) => false,
                    CleanAiCommand::Scan(command) => command.output.format == SnapshotFormat::Human,
                    CleanAiCommand::Preview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    CleanAiCommand::Execute(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    CleanAiCommand::RecoveryList(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                },
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
                SoftwareCommand::Updates(group) => match &group.command {
                    SoftwareUpdatesCommand::Check(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareUpdatesCommand::Plan(_) => false,
                    SoftwareUpdatesCommand::Preview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareUpdatesCommand::Execute(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                },
                SoftwareCommand::Startup(group) => match &group.command {
                    SoftwareStartupCommand::List(output) => output.format == SnapshotFormat::Human,
                    SoftwareStartupCommand::Preview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareStartupCommand::Execute(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareStartupCommand::Plan(_)
                    | SoftwareStartupCommand::HivePlan(_)
                    | SoftwareStartupCommand::RestorePlan(_) => false,
                    SoftwareStartupCommand::HivePreview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareStartupCommand::RestorePreview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareStartupCommand::HiveExecute(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareStartupCommand::RestoreExecute(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                },
                SoftwareCommand::Leftovers(group) => match &group.command {
                    SoftwareLeftoversCommand::Scan(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareLeftoversCommand::Preview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareLeftoversCommand::Execute(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareLeftoversCommand::Plan(_)
                    | SoftwareLeftoversCommand::RegistryPlan(_)
                    | SoftwareLeftoversCommand::RegistryRestorePlan(_) => false,
                    SoftwareLeftoversCommand::RegistryPreview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareLeftoversCommand::RegistryRestorePreview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareLeftoversCommand::RegistryExecute(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    SoftwareLeftoversCommand::RegistryRestoreExecute(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                },
            },
            Some(Command::Optimize(group)) => match &group.command {
                OptimizeCommand::List(output) => output.format == SnapshotFormat::Human,
                OptimizeCommand::Preview(command) => command.output.format == SnapshotFormat::Human,
                OptimizeCommand::Run(command) => command.output.format == SnapshotFormat::Human,
                OptimizeCommand::Health(output) => output.format == SnapshotFormat::Human,
                OptimizeCommand::Plan(_) => false,
            },
            Some(Command::Analyze(group)) => match &group.command {
                AnalyzeCommand::Scan(command) => command.output.format == SnapshotFormat::Human,
                AnalyzeCommand::Roots(command) => command.output.format == SnapshotFormat::Human,
                AnalyzeCommand::List(command) => command.output.format == SnapshotFormat::Human,
                AnalyzeCommand::Explain(command) => command.output.format == SnapshotFormat::Human,
                AnalyzeCommand::Reveal(command) => command.output.format == SnapshotFormat::Human,
                AnalyzeCommand::TrashPreview(command) => {
                    command.output.format == SnapshotFormat::Human
                }
                AnalyzeCommand::TrashExecute(command) => {
                    command.output.format == SnapshotFormat::Human
                }
                AnalyzeCommand::TrashPlan(_) => false,
            },
            Some(Command::Status(group)) => match &group.command {
                StatusCommand::Snapshot(command) => command.output.format == SnapshotFormat::Human,
                StatusCommand::Live(command) => command.format == StreamFormat::Human,
                StatusCommand::Health(command) => command.output.format == SnapshotFormat::Human,
                StatusCommand::Process(group) => match &group.command {
                    StatusProcessCommand::Plan(_) => false,
                    StatusProcessCommand::Preview(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    StatusProcessCommand::Terminate(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                },
                StatusCommand::KeepAwake(group) => match &group.command {
                    StatusKeepAwakeCommand::Run(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                    StatusKeepAwakeCommand::Status(command)
                    | StatusKeepAwakeCommand::Stop(command) => {
                        command.output.format == SnapshotFormat::Human
                    }
                },
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
        "clean ai" => "检查并规划可选的 AI 工具数据清理。",
        "clean ai scan" => "观察一个 AI 主目录，或在省略 --home 时观察生产默认目录。",
        "clean ai plan" => "按数据类从观察文档创建保存的 AI 计划。",
        "clean ai preview" => "实时预览保存的 AI 计划并计算摘要。",
        "clean ai execute" => "写入恢复包后，把已确认的目标移入回收站。",
        "clean ai recovery-list" => "列出恢复包，不执行恢复。",
        "clean ai restore-plan" => "为一个恢复包创建恢复计划。",
        "software" => "盘点软件、检查更新，并规划当前用户维护。",
        "software inventory" => "只读盘点已安装软件。",
        "software plan" => "根据软件盘点创建保存的计划。",
        "software preview" => "实时预览保存的软件计划并计算摘要。",
        "software uninstall" => "派发已确认的当前用户 MSIX 卸载计划。",
        "software updates" => "检查可用更新，并在明确确认后更新一个软件包。",
        "software updates check" => "只读检查可用更新。",
        "software updates plan" => "为一个软件包、来源和版本创建保存的更新计划。",
        "software updates preview" => "实时预览保存的软件包更新计划。",
        "software updates execute" => {
            "在明确确认后应用一个软件包更新计划。需要重启时不记为已更新。"
        }
        "software startup" => "列出启动项，并切换当前用户或离线夹具的批准状态。",
        "software startup list" => "列出启动项。",
        "software startup plan" => "创建保存的启动项切换计划。",
        "software startup preview" => "实时预览保存的启动项切换计划。",
        "software startup execute" => "在明确确认后应用启动项切换计划。",
        "software startup hive-plan" => "从离线夹具配置单元创建启动项切换计划。",
        "software startup hive-preview" => "实时预览离线夹具中的启动项切换计划。",
        "software startup hive-execute" => "在明确确认后切换离线夹具中的一个启动项。",
        "software startup restore-plan" => "根据配置单元执行报告创建恢复计划。",
        "software startup restore-preview" => "实时预览离线夹具的启动项恢复计划。",
        "software startup restore-execute" => "仅当现场身份仍匹配时恢复一个离线启动项。",
        "software leftovers" => "在卸载结果为已移除后审查残留，并处理一个关闭的注册表残留键。",
        "software leftovers scan" => "从软件盘点扫描残留候选。",
        "software leftovers plan" => "创建保存的残留目录移除计划。",
        "software leftovers preview" => "实时预览保存的残留计划。",
        "software leftovers execute" => "在明确确认后把残留目录移入回收站。",
        "software leftovers registry-plan" => "为一个关闭的注册表残留夹具键创建计划。",
        "software leftovers registry-preview" => "实时预览一个关闭的注册表残留计划。",
        "software leftovers registry-execute" => "先做精确字节备份，再修改一个关闭的注册表残留键。",
        "software leftovers registry-restore-plan" => "根据残留执行报告创建恢复计划。",
        "software leftovers registry-restore-preview" => "对照现场字节预览残留恢复。",
        "software leftovers registry-restore-execute" => "仅当现场字节仍匹配时恢复一个残留键。",
        "optimize" => "检查和规划一个受限 Windows 优化。",
        "optimize list" => "列出冻结的优化目录。",
        "optimize plan" => "为一个精确操作创建保存的计划。",
        "optimize preview" => "实时预览保存的优化计划并计算摘要。",
        "optimize run" => "运行已确认的优化计划。",
        "optimize health" => "打印只读磁盘健康。此命令不修复。",
        "analyze" => "分析磁盘占用，并在明确确认后把选定节点移入回收站。",
        "analyze scan" => "分析一个磁盘根目录。",
        "analyze roots" => "列出现有快照来源能看到的卷。缺失的根只记为该根的错误。",
        "analyze list" => "列出一个快照节点的有界子项。",
        "analyze explain" => "解释一个快照节点。此命令不创建清理计划。",
        "analyze reveal" => "在资源管理器中显示一个快照节点。",
        "analyze trash-plan" => "根据快照创建保存的回收站计划。",
        "analyze trash-preview" => "实时预览保存的分析回收站计划。",
        "analyze trash-execute" => "在明确确认后把快照节点移入回收站。",
        "status" => "检查当前系统状态。",
        "status snapshot" => "采集一次有界状态快照。",
        "status live" => "持续输出状态快照，直到取消。",
        "status health" => "用证据说明一次状态快照。此命令不打分。",
        "status process" => "审查一次进程终止。",
        "status process plan" => "把 PID 和创建时间绑定到保存的计划。",
        "status process preview" => "预览进程计划，并包含数据丢失说明。",
        "status process terminate" => "在明确确认后终止一个已审查的进程。",
        "status keep-awake" => "持有一个有限的内存租约，不修改持久电源计划。",
        "status keep-awake run" => "开始一个有限的内存保活租约。",
        "status keep-awake status" => "按所有者和令牌读取一个租约。",
        "status keep-awake stop" => "按所有者和令牌释放一个租约。",
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
        "entry" => "稳定启动项 ID",
        "enabled" => "请求的启用状态，true 或 false",
        "software_id" => "稳定软件 ID",
        "uninstall_operation" => "已成功卸载的操作 ID",
        "snapshot" => "保存的分析快照 JSON 文件",
        "node" => "快照节点 ID",
        "child_offset" => "跳过的子节点行数",
        "child_limit" => "最大子节点行数",
        "operation" => "稳定优化操作 ID",
        "process_limit" => "最大进程行数",
        "interval" => "采样间隔秒数",
        "pid" => "计划绑定的进程 ID",
        "creation_time" => "计划绑定的进程创建 FILETIME",
        "name" => "计划绑定的进程文件名",
        "duration_ms" => "租约毫秒数，必须显式且有限",
        "owner" => "租约所有者",
        "token" => "保活会话令牌",
        "endpoint" => "运行进程打印的环回地址",
        "domain" => "固定审计域",
        "limit" => "最大审计行数",
        "operation_id" => "稳定操作 ID",
        "provider" => "AI 提供方",
        "home" => "AI 主目录的绝对路径",
        "class" => "AI 数据类",
        "scan" => "保存的 AI 观察 JSON 文件",
        "bundle" => "恢复包 ID",
        "help" => "显示当前上下文帮助",
        "version" => "显示版本",
        "update_provider" => "更新提供程序。测试传入命令替身，不使用已安装的 winget。",
        "package_id" => "更新表中的软件包 ID",
        "update_source" => "软件包来源。仅支持 winget。",
        "available_version" => "要规划的确切可用版本",
        "hive" => "离线启动项夹具配置单元。不是本机 HKCU 或 HKLM。",
        "startup_report" => "包含备份的配置单元执行报告",
        "residue_key" => "关闭的注册表残留夹具键",
        "backup" => "残留键的精确字节备份",
        "residue" => "残留执行报告",
        _ => "命令参数",
    }
}

fn chinese_numeric_range(id: &str) -> Option<&'static str> {
    match id {
        "process_limit" => Some("1..100"),
        "interval" => Some("1..60"),
        "duration_ms" => Some("1..43200000"),
        "limit" | "child_limit" => Some("1..1000"),
        _ => None,
    }
}

fn chinese_context_constraints(path: &[String]) -> &'static [&'static str] {
    match path.join(" ").as_str() {
        "" => &["直接运行 devsweep 仅在标准输入与标准输出都是 TTY 时打开 TUI。"],
        "clean scan" => &["--rescan-target 要求恰好一个 --root，且 scope 必须包含 projects。"],
        "clean plan"
        | "software plan"
        | "optimize plan"
        | "software startup plan"
        | "software leftovers plan"
        | "software updates plan"
        | "software startup hive-plan"
        | "software startup restore-plan"
        | "software leftovers registry-plan"
        | "software leftovers registry-restore-plan"
        | "analyze trash-plan"
        | "clean ai plan"
        | "clean ai restore-plan"
        | "status process plan" => {
            &["计划命令不接受 --format 或 --language，只写版本化 JSON 计划。"]
        }
        "clean preview"
        | "software preview"
        | "optimize preview"
        | "software startup preview"
        | "software leftovers preview"
        | "software updates preview"
        | "software startup hive-preview"
        | "software startup restore-preview"
        | "software leftovers registry-preview"
        | "software leftovers registry-restore-preview"
        | "analyze trash-preview"
        | "clean ai preview"
        | "status process preview" => &["计划输入与可选输出路径必须不同。"],
        "clean execute"
        | "software uninstall"
        | "optimize run"
        | "software startup execute"
        | "software leftovers execute"
        | "software updates execute"
        | "software startup hive-execute"
        | "software startup restore-execute"
        | "software leftovers registry-execute"
        | "software leftovers registry-restore-execute"
        | "analyze trash-execute"
        | "clean ai execute"
        | "status process terminate" => &[
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
        "status health" => &["只解释一次快照。CPU 负载不是系统故障，也不生成单一分数。"],
        "status keep-awake run" => {
            &["必须提供有限的 --duration-ms、所有者和 --confirm。此命令不修改持久电源计划。"]
        }
        "history list" => &["limit 范围 1..1000，默认 100。"],
        "clean ai scan" => &[
            "--home 必须是绝对路径；只有省略 --home 时才使用提供方默认目录。",
            "显式 --home 必须同时给出 --provider。",
        ],
        "clean ai recovery-list" => &["--home 必须是绝对路径。此命令不恢复文件。"],
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

fn reject_distinct_output(left: &Path, right: &Path, output: Option<&Path>) -> Result<(), String> {
    reject_same_path(left, right)?;
    if let Some(output) = output {
        reject_same_path(left, output)?;
        reject_same_path(right, output)?;
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

fn parse_enabled(value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("enabled must be true or false".to_string()),
    }
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
            vec!["devsweep", "software", "updates", "check"],
            vec!["devsweep", "software", "startup", "list"],
            vec![
                "devsweep",
                "software",
                "startup",
                "plan",
                "--entry",
                "startup:v1:example",
                "--enabled",
                "false",
                "--output",
                "p.json",
            ],
            vec![
                "devsweep", "software", "startup", "preview", "--plan", "p.json",
            ],
            vec![
                "devsweep",
                "software",
                "startup",
                "execute",
                "--plan",
                "p.json",
                "--preview-digest",
                &digest,
                "--confirm",
            ],
            vec![
                "devsweep",
                "software",
                "leftovers",
                "scan",
                "--inventory",
                "i.json",
                "--select",
                "software:v1:example",
            ],
            vec![
                "devsweep",
                "software",
                "leftovers",
                "plan",
                "--inventory",
                "i.json",
                "--software-id",
                "software:v1:example",
                "--uninstall-operation",
                "op-1",
                "--select",
                "candidate-1",
                "--output",
                "p.json",
            ],
            vec![
                "devsweep",
                "software",
                "leftovers",
                "preview",
                "--inventory",
                "i.json",
                "--plan",
                "p.json",
            ],
            vec![
                "devsweep",
                "software",
                "leftovers",
                "execute",
                "--inventory",
                "i.json",
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
            vec![
                "devsweep",
                "analyze",
                "reveal",
                "--snapshot",
                "s.json",
                "--node",
                "1",
            ],
            vec![
                "devsweep",
                "analyze",
                "trash-plan",
                "--snapshot",
                "s.json",
                "--select",
                "1",
                "--output",
                "p.json",
            ],
            vec!["devsweep", "analyze", "trash-preview", "--plan", "p.json"],
            vec![
                "devsweep",
                "analyze",
                "trash-execute",
                "--plan",
                "p.json",
                "--preview-digest",
                &digest,
                "--confirm",
            ],
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
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["software", "startup", "execute"],
                vec![
                    "--plan".to_string(),
                    "startup-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["software", "leftovers", "execute"],
                vec![
                    "--inventory".to_string(),
                    "inventory.json".to_string(),
                    "--plan".to_string(),
                    "leftover-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["software", "updates", "execute"],
                vec![
                    "--plan".to_string(),
                    "update-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["software", "startup", "hive-execute"],
                vec![
                    "--hive".to_string(),
                    "hive.json".to_string(),
                    "--plan".to_string(),
                    "hive-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["software", "startup", "restore-execute"],
                vec![
                    "--hive".to_string(),
                    "hive.json".to_string(),
                    "--plan".to_string(),
                    "restore-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["software", "leftovers", "registry-execute"],
                vec![
                    "--key".to_string(),
                    "DevSweepResidue".to_string(),
                    "--backup".to_string(),
                    "backup.bin".to_string(),
                    "--plan".to_string(),
                    "residue-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["software", "leftovers", "registry-restore-execute"],
                vec![
                    "--key".to_string(),
                    "DevSweepResidue".to_string(),
                    "--backup".to_string(),
                    "backup.bin".to_string(),
                    "--plan".to_string(),
                    "residue-restore-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["clean", "ai", "execute"],
                vec![
                    "--plan".to_string(),
                    "ai-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["status", "process", "terminate"],
                vec![
                    "--plan".to_string(),
                    "process-plan.json".to_string(),
                    "--preview-digest".to_string(),
                    digest.clone(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["status", "keep-awake", "run"],
                vec![
                    "--duration-ms".to_string(),
                    "1000".to_string(),
                    "--owner".to_string(),
                    "owner-a".to_string(),
                    "--confirm".to_string(),
                ],
            ),
            (
                vec!["analyze", "trash-execute"],
                vec![
                    "--plan".to_string(),
                    "analyze-plan.json".to_string(),
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
                "clean plan"
                | "software plan"
                | "optimize plan"
                | "software startup plan"
                | "software leftovers plan"
                | "software updates plan"
                | "software startup hive-plan"
                | "software startup restore-plan"
                | "software leftovers registry-plan"
                | "software leftovers registry-restore-plan"
                | "analyze trash-plan"
                | "clean ai plan"
                | "clean ai restore-plan" => {
                    "plan_only,no_language,no_format,create_new,input_output_distinct"
                }
                "status process plan" => "plan_only,no_language,no_format,create_new",
                "clean preview"
                | "software preview"
                | "optimize preview"
                | "software startup preview"
                | "software leftovers preview"
                | "software updates preview"
                | "software startup hive-preview"
                | "software startup restore-preview"
                | "software leftovers registry-preview"
                | "software leftovers registry-restore-preview"
                | "analyze trash-preview"
                | "clean ai preview"
                | "status process preview" => "language_human,create_new,input_output_distinct",
                "clean execute"
                | "software uninstall"
                | "optimize run"
                | "software startup execute"
                | "software leftovers execute"
                | "software updates execute"
                | "software startup hive-execute"
                | "software startup restore-execute"
                | "software leftovers registry-execute"
                | "software leftovers registry-restore-execute"
                | "analyze trash-execute"
                | "clean ai execute"
                | "status process terminate" => {
                    "language_human,create_new,input_output_distinct,digest_confirm,no_prompt"
                }
                "clean protect add" | "clean protect remove" | "status keep-awake run" => {
                    "language_human,create_new,confirm"
                }
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
