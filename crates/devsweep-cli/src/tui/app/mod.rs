use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    execution::{ExecutionReport, ExecutionTargetStatus},
    inventory::InventoryReport,
    model::{
        CleanAction, CleanTarget, CleanupPlan, Evidence, RiskLevel, ScanHealth, ScanTotals, Scope,
        TargetId,
    },
    plan::validate_scanned_plan,
    rules::PYCACHE_RULE_DOC,
    scan::ScanPhase,
};

use super::display::{
    CommandPreview, action_summary, command_previews, compact_target_id, display_path,
    format_cleanup_progress, selected_target_summary, target_title,
};
use super::{
    modes::analyze::{AnalyzeAction, AnalyzeModeState, AnalyzeSort},
    shell::{ModeId, ShellComposition},
};
use crate::i18n::Locale;

mod events;
mod input;
mod jobs;
mod selection;
mod worker;

pub(super) use events::{Effect, JobId, UiEvent, WorkerEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionOverride {
    Selected,
    Deselected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct App {
    /// Presentation-only shell state. It has no cleanup-plan authority.
    pub(super) shell: ShellComposition,
    pub(super) language_settings: LanguageSettingsState,
    pub(super) analyze: AnalyzeModeState,
    pub(super) pending_mode: Option<ModeId>,
    pub(super) targets: Vec<CleanTarget>,
    pub(super) scan_health: ScanHealth,
    /// Read-only capacity observations kept separate from cleanup targets.
    pub(super) inventory_report: Option<InventoryReport>,
    pub(super) inventory_selected_index: usize,
    pub(super) inventory_list_scroll: usize,
    /// Display-only collapse state for high-volume Python bytecode cache rows.
    collapsed_pycache_projects: HashSet<PathBuf>,
    pub(super) selected_ids: HashSet<TargetId>,
    selection_overrides: HashMap<TargetId, SelectionOverride>,
    /// Target ids successfully cleaned in this session until the next rescan.
    pub(super) cleaned_ids: HashSet<TargetId>,
    pub(super) selected_index: usize,
    /// First visible target-list row for the current viewport.
    pub(super) list_scroll: usize,
    pub(super) active_tab: ActiveTab,
    pub(super) filter: String,
    pub(super) filter_active: bool,
    pub(super) risk_filter: Option<RiskLevel>,
    pub(super) overlay: Overlay,
    pub(super) jobs: Vec<JobRecord>,
    pub(super) logs: Vec<LogEntry>,
    pub(super) next_log_seq: u64,
    pub(super) cleanup_progress: Option<CleanupProgress>,
    pub(super) scan_snapshot: Option<ScanSnapshot>,
    pub(super) should_quit: bool,
    /// User asked to quit after active jobs reach a terminal state (D9).
    pub(super) quit_after_jobs: bool,
    /// A replacement scan requested while read-only work is still owned. The
    /// terminal event starts it only after every prior worker has joined.
    pending_scan_restart: bool,
    pub(super) next_job_id: JobId,
    next_language_request_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LanguageSettingsState {
    pub(super) open: bool,
    pub(super) selected: Locale,
    pub(super) pending_request: Option<u64>,
    pub(super) failure: Option<LanguageSettingsFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LanguageSettingsFailure {
    PersistenceUnavailable,
}

impl LanguageSettingsState {
    fn closed(locale: Locale) -> Self {
        Self {
            open: false,
            selected: locale,
            pending_request: None,
            failure: None,
        }
    }
}

impl App {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::with_plan(CleanupPlan::empty())
    }

    pub(super) fn with_shell(shell: ShellComposition) -> Self {
        Self::with_plan_and_shell(CleanupPlan::empty(), shell)
    }

    pub(super) fn startup_effects(&mut self) -> Vec<Effect> {
        let job_id = self.start_scan_job("Startup scan requested");
        vec![Effect::StartScan { job_id }]
    }

    #[cfg(test)]
    pub(super) fn with_plan(plan: CleanupPlan) -> Self {
        let shell = ShellComposition::for_locale(Locale::En)
            .expect("the embedded English shell catalogue must be valid");
        Self::with_plan_and_shell(plan, shell)
    }

    fn with_plan_and_shell(plan: CleanupPlan, shell: ShellComposition) -> Self {
        let selected_ids = default_selected_ids(&plan.targets);
        let collapsed_pycache_projects = pycache_project_roots(&plan.targets);
        let scan_health = complete_health_for_plan(&plan);
        let locale = shell.locale;

        let mut app = Self {
            shell,
            language_settings: LanguageSettingsState::closed(locale),
            analyze: AnalyzeModeState::default(),
            pending_mode: None,
            targets: plan.targets,
            scan_health,
            inventory_report: None,
            inventory_selected_index: 0,
            inventory_list_scroll: 0,
            collapsed_pycache_projects,
            selected_ids,
            selection_overrides: HashMap::new(),
            cleaned_ids: HashSet::new(),
            selected_index: 0,
            list_scroll: 0,
            active_tab: ActiveTab::Dashboard,
            filter: String::new(),
            filter_active: false,
            risk_filter: None,
            overlay: Overlay::None,
            jobs: Vec::new(),
            logs: Vec::new(),
            next_log_seq: 1,
            cleanup_progress: None,
            scan_snapshot: None,
            should_quit: false,
            quit_after_jobs: false,
            pending_scan_restart: false,
            next_job_id: 1,
            next_language_request_id: 1,
        };
        app.log("Ready");
        app
    }

    pub(super) fn update(&mut self, event: UiEvent) -> Vec<Effect> {
        let effects = match event {
            UiEvent::Key(key) => self.handle_key(key),
            UiEvent::Worker(event) => self.handle_worker_event(event),
        };
        self.maybe_finish_pending_quit();
        effects
    }
}

fn sum_unique_target_bytes<'a>(targets: impl IntoIterator<Item = &'a CleanTarget>) -> u64 {
    let mut seen_paths = HashSet::new();
    let mut seen_ids = HashSet::new();
    let mut bytes = 0;

    for target in targets {
        let is_new_footprint = match &target.path {
            Some(path) => seen_paths.insert(path.clone()),
            None => seen_ids.insert(target.id.clone()),
        };
        if is_new_footprint {
            bytes += target.estimated_bytes;
        }
    }

    bytes
}

fn complete_health_for_plan(plan: &CleanupPlan) -> ScanHealth {
    let mut health = ScanHealth::complete();
    health.totals = ScanTotals::from_cleanup_plan(plan);
    health
}

fn pycache_project_root(target: &CleanTarget) -> Option<&PathBuf> {
    let Scope::Project { root } = &target.scope else {
        return None;
    };
    target.evidence.iter().any(|evidence| {
        matches!(evidence, Evidence::RuleMatched { rule_id } if rule_id == PYCACHE_RULE_DOC.id)
    })
    .then_some(root)
}

fn pycache_project_roots(targets: &[CleanTarget]) -> HashSet<PathBuf> {
    let mut counts = HashMap::new();
    for target in targets {
        if let Some(project_root) = pycache_project_root(target) {
            *counts.entry(project_root.clone()).or_insert(0usize) += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(project_root, count)| (count > 1).then_some(project_root))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeKind {
    Global,
    Project,
}

/// A derived target-list row. It never changes the cleanup-plan target list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TargetListRow {
    Target(usize),
    PycacheGroup(PycacheGroup),
}

impl TargetListRow {
    fn target_indices(&self) -> Vec<usize> {
        match self {
            Self::Target(index) => vec![*index],
            Self::PycacheGroup(group) => group.target_indices.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PycacheGroup {
    pub(super) project_root: PathBuf,
    pub(super) target_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScanSnapshot {
    job_id: JobId,
    latest_targets: Vec<CleanTarget>,
}

impl ScanSnapshot {
    fn new(job_id: JobId) -> Self {
        Self {
            job_id,
            latest_targets: Vec::new(),
        }
    }

    fn set_targets(&mut self, targets: Vec<CleanTarget>) {
        self.latest_targets = targets;
    }

    fn targets(&self) -> Vec<CleanTarget> {
        self.latest_targets.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActiveTab {
    Dashboard,
    Global,
    Projects,
    Rules,
    JobsLogs,
    Inventory,
}

impl ActiveTab {
    pub(super) const ALL: [Self; 6] = [
        Self::Dashboard,
        Self::Global,
        Self::Projects,
        Self::Rules,
        Self::JobsLogs,
        Self::Inventory,
    ];

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Global => "Global",
            Self::Projects => "Projects",
            Self::Rules => "Rules",
            Self::JobsLogs => "Jobs/Logs",
            Self::Inventory => "Inventory",
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|tab| *tab == self)
            .expect("active tab exists");
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|tab| *tab == self)
            .expect("active tab exists");
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    fn matches_target(self, target: &CleanTarget) -> bool {
        match self {
            Self::Dashboard => true,
            Self::Global => matches!(target.scope, Scope::Global),
            Self::Projects => matches!(target.scope, Scope::Project { .. }),
            Self::Rules | Self::JobsLogs | Self::Inventory => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Overlay {
    None,
    Help,
    Details,
    DryRun,
    Confirm(ConfirmState),
    /// D9: mutation running; user must wait or cancel-and-wait before quit.
    QuitConfirm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfirmState {
    pub(super) target_count: usize,
    pub(super) estimated_bytes: u64,
    pub(super) plan_digest_prefix: String,
    pub(super) has_irreversible_commands: bool,
    pub(super) required_phrase: String,
    pub(super) input: String,
    pub(super) feedback: Option<String>,
    pub(super) message: String,
    pub(super) selected_targets: Vec<String>,
    pub(super) command_previews: Vec<CommandPreview>,
    manifest: Box<ExecutionManifest>,
    pub(super) invalidated_by_scan: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExecutionManifest {
    plan: CleanupPlan,
    selected: Vec<TargetId>,
    digest: String,
}

impl ExecutionManifest {
    fn from_targets(targets: &[&CleanTarget]) -> Result<Self, String> {
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: targets.iter().map(|target| (*target).clone()).collect(),
        };
        let validated = validate_scanned_plan(&plan).map_err(|error| error.to_string())?;
        let selected = plan
            .targets
            .iter()
            .map(|target| target.id.clone())
            .collect();
        Ok(Self {
            plan,
            selected,
            digest: validated.digest().to_string(),
        })
    }

    fn digest_prefix(&self) -> &str {
        &self.digest[..12]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CleanupProgress {
    pub(super) job_id: JobId,
    pub(super) completed: usize,
    pub(super) total: usize,
    pub(super) summary: String,
    pub(super) finished: bool,
    pub(super) items: Vec<CleanupProgressItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CleanupProgressItem {
    pub(super) target_id: TargetId,
    pub(super) label: String,
    pub(super) status: CleanupItemStatus,
    pub(super) detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CleanupItemStatus {
    Pending,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanupProgressUpdate {
    job_id: JobId,
    completed: usize,
    total: usize,
    summary: String,
    target_id: TargetId,
    status: ExecutionTargetStatus,
    detail: String,
}

impl From<ExecutionTargetStatus> for CleanupItemStatus {
    fn from(status: ExecutionTargetStatus) -> Self {
        match status {
            ExecutionTargetStatus::Succeeded => Self::Succeeded,
            ExecutionTargetStatus::Failed | ExecutionTargetStatus::Unknown => Self::Failed,
            ExecutionTargetStatus::Skipped => Self::Skipped,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobKind {
    Scan,
    Inventory,
    Clean,
    Analyze,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobStatus {
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Canceled,
}

impl JobStatus {
    pub(super) fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::Cancelling)
    }

    fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Running,
                Self::Running | Self::Cancelling | Self::Succeeded | Self::Failed
            ) | (
                Self::Cancelling,
                Self::Canceled | Self::Succeeded | Self::Failed
            )
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct JobRecord {
    pub(super) id: JobId,
    pub(super) kind: JobKind,
    pub(super) label: String,
    pub(super) status: JobStatus,
    pub(super) progress: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LogEntry {
    pub(super) seq: u64,
    pub(super) level: AppLogLevel,
    pub(super) source: AppLogSource,
    pub(super) job_id: Option<JobId>,
    pub(super) target_id: Option<TargetId>,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AppLogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AppLogSource {
    App,
    Scan,
    Inventory,
    Clean,
    Audit,
    Analyze,
}

pub(super) fn cleanup_progress_for_plan(job_id: JobId, plan: &CleanupPlan) -> CleanupProgress {
    CleanupProgress {
        job_id,
        completed: 0,
        total: plan.targets.len(),
        summary: "Executing selected cleanup plan".to_string(),
        finished: false,
        items: plan
            .targets
            .iter()
            .map(|target| CleanupProgressItem {
                target_id: target.id.clone(),
                label: target_title(target),
                status: CleanupItemStatus::Pending,
                detail: None,
            })
            .collect(),
    }
}

fn cleanup_item_detail(status: CleanupItemStatus, detail: String) -> Option<String> {
    match status {
        CleanupItemStatus::Pending | CleanupItemStatus::Succeeded => None,
        CleanupItemStatus::Failed | CleanupItemStatus::Skipped => Some(detail),
    }
}

fn log_level_for_execution_status(status: ExecutionTargetStatus) -> AppLogLevel {
    match status {
        ExecutionTargetStatus::Succeeded | ExecutionTargetStatus::Skipped => AppLogLevel::Info,
        ExecutionTargetStatus::Failed | ExecutionTargetStatus::Unknown => AppLogLevel::Error,
    }
}

fn default_selected_ids(targets: &[CleanTarget]) -> HashSet<TargetId> {
    targets
        .iter()
        .filter(|target| target.selected_by_default && target.action.is_executable())
        .map(|target| target.id.clone())
        .collect()
}

#[cfg(test)]
mod tests;
