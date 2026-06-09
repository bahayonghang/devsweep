use std::{
    collections::HashSet,
    io,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, Paragraph, Tabs, Wrap},
};

use crate::{
    executor::{ExecutionReport, ExecutionRequest, Executor},
    model::{
        CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel, Scope, TargetId,
    },
    providers::GlobalProviderScanner,
    scanner::ProjectScanner,
};

type JobId = u64;

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal")?;
    let loop_result = run_event_loop(&mut terminal);
    let restore_result = restore_terminal(&mut terminal);

    restore_result?;
    loop_result
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();
    let (worker_tx, worker_rx) = mpsc::channel();

    while !app.should_quit {
        drain_worker_events(&mut app, &worker_rx, &worker_tx)?;
        terminal.draw(|frame| render_app(frame, &app))?;

        if event::poll(Duration::from_millis(100))?
            && let CrosstermEvent::Key(key) = event::read()?
        {
            let effects = app.update(UiEvent::Key(key));
            dispatch_effects(effects, &worker_tx)?;
        }
    }

    Ok(())
}

fn drain_worker_events(
    app: &mut App,
    worker_rx: &Receiver<WorkerEvent>,
    worker_tx: &Sender<WorkerEvent>,
) -> Result<()> {
    while let Ok(event) = worker_rx.try_recv() {
        let effects = app.update(UiEvent::Worker(event));
        dispatch_effects(effects, worker_tx)?;
    }
    Ok(())
}

fn dispatch_effects(effects: Vec<Effect>, worker_tx: &Sender<WorkerEvent>) -> Result<()> {
    for effect in effects {
        dispatch_effect(effect, worker_tx.clone())?;
    }
    Ok(())
}

fn dispatch_effect(effect: Effect, worker_tx: Sender<WorkerEvent>) -> Result<()> {
    match effect {
        Effect::StartScan { job_id } => {
            thread::spawn(move || run_scan_worker(job_id, worker_tx));
        }
        Effect::StartClean { job_id, plan } => {
            thread::spawn(move || run_clean_worker(job_id, plan, worker_tx));
        }
        Effect::CancelJob { job_id } => {
            let _ = worker_tx.send(WorkerEvent::JobCanceled { job_id });
        }
        Effect::Quit => {}
    }

    Ok(())
}

fn run_scan_worker(job_id: JobId, worker_tx: Sender<WorkerEvent>) {
    let _ = worker_tx.send(WorkerEvent::ScanStarted { job_id });
    let _ = worker_tx.send(WorkerEvent::JobProgress {
        job_id,
        message: "Scanning current directory and global providers".to_string(),
    });

    let result = scan_current_workspace();
    match result {
        Ok(plan) => {
            let _ = worker_tx.send(WorkerEvent::ScanFinished { job_id, plan });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

fn scan_current_workspace() -> Result<CleanupPlan> {
    let current_dir = std::env::current_dir().context("failed to get current directory")?;
    let mut plan = CleanupPlan::empty();
    plan.targets
        .extend(ProjectScanner::new().scan_roots(&[current_dir])?.targets);
    plan.targets
        .extend(GlobalProviderScanner::new().scan().targets);
    Ok(plan)
}

fn run_clean_worker(job_id: JobId, plan: CleanupPlan, worker_tx: Sender<WorkerEvent>) {
    let _ = worker_tx.send(WorkerEvent::CleanProgress {
        job_id,
        message: "Executing selected cleanup plan".to_string(),
    });

    let result = Executor::default().run_plan(
        &plan,
        ExecutionRequest {
            execute: true,
            allow_permanent_delete: false,
            audit_log: None,
        },
    );

    match result {
        Ok(report) => {
            let _ = worker_tx.send(WorkerEvent::CleanFinished { job_id, report });
        }
        Err(error) => {
            let _ = worker_tx.send(WorkerEvent::JobFailed {
                job_id,
                message: error.to_string(),
            });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct App {
    targets: Vec<CleanTarget>,
    selected_ids: HashSet<TargetId>,
    selected_index: usize,
    active_tab: ActiveTab,
    filter: String,
    filter_active: bool,
    risk_filter: Option<RiskLevel>,
    overlay: Overlay,
    jobs: Vec<JobRecord>,
    logs: Vec<LogEntry>,
    should_quit: bool,
    next_job_id: JobId,
}

impl App {
    fn new() -> Self {
        Self::with_plan(CleanupPlan::empty())
    }

    fn with_plan(plan: CleanupPlan) -> Self {
        let selected_ids = plan
            .targets
            .iter()
            .filter(|target| target.selected_by_default)
            .map(|target| target.id.clone())
            .collect();

        let mut app = Self {
            targets: plan.targets,
            selected_ids,
            selected_index: 0,
            active_tab: ActiveTab::Dashboard,
            filter: String::new(),
            filter_active: false,
            risk_filter: None,
            overlay: Overlay::None,
            jobs: Vec::new(),
            logs: Vec::new(),
            should_quit: false,
            next_job_id: 1,
        };
        app.log("Ready");
        app
    }

    fn update(&mut self, event: UiEvent) -> Vec<Effect> {
        match event {
            UiEvent::Key(key) => self.handle_key(key),
            UiEvent::Worker(event) => self.handle_worker_event(event),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        if key.kind == KeyEventKind::Release {
            return Vec::new();
        }

        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'C'))
        {
            self.should_quit = true;
            return vec![Effect::Quit];
        }

        if self.filter_active {
            return self.handle_filter_key(key);
        }

        match self.overlay {
            Overlay::Confirm(_) => self.handle_confirm_key(key),
            Overlay::Help | Overlay::Details | Overlay::DryRun => self.handle_overlay_key(key),
            Overlay::None => self.handle_normal_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                vec![Effect::Quit]
            }
            KeyCode::Char('s') => {
                let job_id = self.start_job(JobKind::Scan, "Scan current directory and globals");
                self.log("Scan requested");
                vec![Effect::StartScan { job_id }]
            }
            KeyCode::Char('c') => {
                if self.selected_ids.is_empty() {
                    self.log("No selected targets to clean");
                } else {
                    self.overlay = Overlay::Confirm(self.confirm_state());
                }
                Vec::new()
            }
            KeyCode::Char('d') => {
                self.overlay = Overlay::DryRun;
                Vec::new()
            }
            KeyCode::Char('?') => {
                self.overlay = Overlay::Help;
                Vec::new()
            }
            KeyCode::Char('/') => {
                self.filter_active = true;
                Vec::new()
            }
            KeyCode::Char('r') => {
                self.cycle_risk_filter();
                self.selected_index = 0;
                Vec::new()
            }
            KeyCode::Char('a') => {
                self.toggle_visible_selection();
                Vec::new()
            }
            KeyCode::Char('l') => {
                self.active_tab = ActiveTab::JobsLogs;
                self.selected_index = 0;
                Vec::new()
            }
            KeyCode::Char('x') => self.cancel_active_job(),
            KeyCode::Char('1') => self.set_tab(ActiveTab::Dashboard),
            KeyCode::Char('2') => self.set_tab(ActiveTab::Global),
            KeyCode::Char('3') => self.set_tab(ActiveTab::Projects),
            KeyCode::Char('4') => self.set_tab(ActiveTab::Rules),
            KeyCode::Char('5') => self.set_tab(ActiveTab::JobsLogs),
            KeyCode::Tab | KeyCode::Right => {
                self.active_tab = self.active_tab.next();
                self.selected_index = 0;
                Vec::new()
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.active_tab = self.active_tab.previous();
                self.selected_index = 0;
                Vec::new()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                Vec::new()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                Vec::new()
            }
            KeyCode::Enter => {
                if self.selected_target().is_some() {
                    self.overlay = Overlay::Details;
                }
                Vec::new()
            }
            KeyCode::Char(' ') => {
                self.toggle_selected_target();
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.filter_active = false;
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.selected_index = 0;
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.filter.push(ch);
                self.selected_index = 0;
            }
            _ => {}
        }
        Vec::new()
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.overlay = Overlay::None;
            }
            _ => {}
        }
        Vec::new()
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                Vec::new()
            }
            KeyCode::Backspace => {
                if let Overlay::Confirm(confirm) = &mut self.overlay {
                    confirm.input.pop();
                }
                Vec::new()
            }
            KeyCode::Char(ch) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && let Overlay::Confirm(confirm) = &mut self.overlay
                {
                    confirm.input.push(ch);
                }
                Vec::new()
            }
            KeyCode::Enter => {
                let accepted = matches!(
                    &self.overlay,
                    Overlay::Confirm(confirm)
                        if confirm.input.trim() == confirm.required_phrase
                );

                if !accepted {
                    self.log("Confirmation phrase did not match");
                    return Vec::new();
                }

                let target_count = self.selected_ids.len();
                let plan = self.selected_cleanup_plan();
                self.overlay = Overlay::None;
                let job_id =
                    self.start_job(JobKind::Clean, format!("Clean {target_count} target(s)"));
                self.log(format!("Clean requested for {target_count} target(s)"));
                vec![Effect::StartClean { job_id, plan }]
            }
            _ => Vec::new(),
        }
    }

    fn handle_worker_event(&mut self, event: WorkerEvent) -> Vec<Effect> {
        match event {
            WorkerEvent::ScanStarted { job_id } => {
                self.ensure_job(job_id, JobKind::Scan, "Scan current directory and globals");
                self.mark_job(job_id, JobStatus::Running, "Started");
            }
            WorkerEvent::JobProgress { job_id, message }
            | WorkerEvent::CleanProgress { job_id, message } => {
                self.mark_job(job_id, JobStatus::Running, message.clone());
                self.log(message);
            }
            WorkerEvent::ScanFinished { job_id, plan } => {
                let count = plan.targets.len();
                self.targets = plan.targets;
                self.selected_ids = self
                    .targets
                    .iter()
                    .filter(|target| target.selected_by_default)
                    .map(|target| target.id.clone())
                    .collect();
                self.selected_index = 0;
                self.mark_job(
                    job_id,
                    JobStatus::Succeeded,
                    format!("Found {count} target(s)"),
                );
                self.log(format!("Scan finished: {count} target(s)"));
            }
            WorkerEvent::CleanFinished { job_id, report } => {
                let status = if report.failed == 0 {
                    JobStatus::Succeeded
                } else {
                    JobStatus::Failed
                };
                self.mark_job(
                    job_id,
                    status,
                    format!(
                        "{} succeeded, {} failed, {} skipped",
                        report.succeeded, report.failed, report.skipped
                    ),
                );
                if let Some(path) = report.audit_log {
                    self.log(format!("Audit log: {}", path.display()));
                }
            }
            WorkerEvent::JobFailed { job_id, message } => {
                self.mark_job(job_id, JobStatus::Failed, message.clone());
                self.log(message);
            }
            WorkerEvent::JobCanceled { job_id } => {
                self.mark_job(job_id, JobStatus::Canceled, "Canceled");
                self.log(format!("Job {job_id} canceled"));
            }
        }

        Vec::new()
    }

    fn set_tab(&mut self, tab: ActiveTab) -> Vec<Effect> {
        self.active_tab = tab;
        self.selected_index = 0;
        Vec::new()
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.visible_target_indices().len();
        if count == 0 {
            self.selected_index = 0;
            return;
        }

        let current = self.selected_index.min(count - 1) as isize;
        let next = (current + delta).clamp(0, count as isize - 1);
        self.selected_index = next as usize;
    }

    fn toggle_selected_target(&mut self) {
        let Some(target_id) = self.selected_target().map(|target| target.id.clone()) else {
            return;
        };

        if !self.selected_ids.remove(&target_id) {
            self.selected_ids.insert(target_id);
        }
    }

    fn toggle_visible_selection(&mut self) {
        let visible_ids: Vec<TargetId> = self
            .visible_target_indices()
            .into_iter()
            .map(|index| self.targets[index].id.clone())
            .collect();

        if visible_ids.is_empty() {
            return;
        }

        let all_selected = visible_ids.iter().all(|id| self.selected_ids.contains(id));
        if all_selected {
            for id in visible_ids {
                self.selected_ids.remove(&id);
            }
        } else {
            self.selected_ids.extend(visible_ids);
        }
    }

    fn cycle_risk_filter(&mut self) {
        self.risk_filter = match self.risk_filter.as_ref() {
            None => Some(RiskLevel::Low),
            Some(RiskLevel::Low) => Some(RiskLevel::Medium),
            Some(RiskLevel::Medium) => Some(RiskLevel::High),
            Some(RiskLevel::High) => Some(RiskLevel::Dangerous),
            Some(RiskLevel::Dangerous) => None,
        };
    }

    fn cancel_active_job(&mut self) -> Vec<Effect> {
        let Some(job_id) = self
            .jobs
            .iter()
            .rev()
            .find(|job| job.status.is_active())
            .map(|job| job.id)
        else {
            self.log("No active job to cancel");
            return Vec::new();
        };

        self.mark_job(job_id, JobStatus::Cancelling, "Cancellation requested");
        vec![Effect::CancelJob { job_id }]
    }

    fn start_job(&mut self, kind: JobKind, label: impl Into<String>) -> JobId {
        let id = self.next_job_id;
        self.next_job_id += 1;
        self.jobs.push(JobRecord {
            id,
            kind,
            label: label.into(),
            status: JobStatus::Running,
            progress: "Queued".to_string(),
        });
        id
    }

    fn ensure_job(&mut self, job_id: JobId, kind: JobKind, label: impl Into<String>) {
        if self.jobs.iter().any(|job| job.id == job_id) {
            return;
        }

        self.jobs.push(JobRecord {
            id: job_id,
            kind,
            label: label.into(),
            status: JobStatus::Running,
            progress: String::new(),
        });
        self.next_job_id = self.next_job_id.max(job_id + 1);
    }

    fn mark_job(&mut self, job_id: JobId, status: JobStatus, progress: impl Into<String>) {
        if let Some(job) = self.jobs.iter_mut().find(|job| job.id == job_id) {
            job.status = status;
            job.progress = progress.into();
        }
    }

    fn confirm_state(&self) -> ConfirmState {
        let selected = self.selected_targets();
        let estimated_bytes = selected.iter().map(|target| target.estimated_bytes).sum();
        let has_irreversible_commands = selected.iter().any(|target| {
            matches!(
                target.action,
                CleanAction::Command {
                    irreversible: true,
                    ..
                } | CleanAction::DeletePermanently { .. }
            )
        });

        let required_phrase = if has_irreversible_commands {
            format!("CLEAN {}", format_bytes(estimated_bytes))
        } else {
            "clean".to_string()
        };
        let message = if has_irreversible_commands {
            "This will run irreversible command-backed cleanup actions.".to_string()
        } else {
            "Move selected reversible targets to Trash.".to_string()
        };

        ConfirmState {
            target_count: selected.len(),
            estimated_bytes,
            has_irreversible_commands,
            required_phrase,
            input: String::new(),
            message,
        }
    }

    fn selected_cleanup_plan(&self) -> CleanupPlan {
        CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: self
                .targets
                .iter()
                .filter(|target| self.selected_ids.contains(&target.id))
                .cloned()
                .map(|mut target| {
                    target.selected_by_default = true;
                    target
                })
                .collect(),
        }
    }

    fn selected_target(&self) -> Option<&CleanTarget> {
        let indices = self.visible_target_indices();
        let index = indices.get(self.selected_index.min(indices.len().saturating_sub(1)))?;
        self.targets.get(*index)
    }

    fn selected_targets(&self) -> Vec<&CleanTarget> {
        self.targets
            .iter()
            .filter(|target| self.selected_ids.contains(&target.id))
            .collect()
    }

    fn visible_target_indices(&self) -> Vec<usize> {
        self.targets
            .iter()
            .enumerate()
            .filter(|(_, target)| self.target_is_visible(target))
            .map(|(index, _)| index)
            .collect()
    }

    fn target_is_visible(&self, target: &CleanTarget) -> bool {
        if !self.active_tab.matches_target(target) {
            return false;
        }

        if let Some(risk_filter) = &self.risk_filter
            && &target.risk != risk_filter
        {
            return false;
        }

        if self.filter.trim().is_empty() {
            return true;
        }

        let needle = self.filter.to_ascii_lowercase();
        target.id.as_str().to_ascii_lowercase().contains(&needle)
            || target
                .path
                .as_ref()
                .map(|path| path.display().to_string().to_ascii_lowercase())
                .is_some_and(|path| path.contains(&needle))
            || action_summary(&target.action)
                .to_ascii_lowercase()
                .contains(&needle)
    }

    fn selected_bytes(&self) -> u64 {
        self.selected_targets()
            .iter()
            .map(|target| target.estimated_bytes)
            .sum()
    }

    fn scope_bytes(&self, scope_kind: ScopeKind) -> u64 {
        self.targets
            .iter()
            .filter(|target| match scope_kind {
                ScopeKind::Global => matches!(target.scope, Scope::Global),
                ScopeKind::Project => matches!(target.scope, Scope::Project { .. }),
            })
            .map(|target| target.estimated_bytes)
            .sum()
    }

    fn log(&mut self, message: impl Into<String>) {
        self.logs.push(LogEntry {
            message: message.into(),
        });
        if self.logs.len() > 200 {
            let overflow = self.logs.len() - 200;
            self.logs.drain(0..overflow);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Global,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum UiEvent {
    Key(KeyEvent),
    Worker(WorkerEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Effect {
    StartScan { job_id: JobId },
    StartClean { job_id: JobId, plan: CleanupPlan },
    CancelJob { job_id: JobId },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkerEvent {
    ScanStarted {
        job_id: JobId,
    },
    JobProgress {
        job_id: JobId,
        message: String,
    },
    ScanFinished {
        job_id: JobId,
        plan: CleanupPlan,
    },
    CleanProgress {
        job_id: JobId,
        message: String,
    },
    CleanFinished {
        job_id: JobId,
        report: ExecutionReport,
    },
    JobFailed {
        job_id: JobId,
        message: String,
    },
    JobCanceled {
        job_id: JobId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveTab {
    Dashboard,
    Global,
    Projects,
    Rules,
    JobsLogs,
}

impl ActiveTab {
    const ALL: [Self; 5] = [
        Self::Dashboard,
        Self::Global,
        Self::Projects,
        Self::Rules,
        Self::JobsLogs,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Global => "Global",
            Self::Projects => "Projects",
            Self::Rules => "Rules",
            Self::JobsLogs => "Jobs/Logs",
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
            Self::Rules | Self::JobsLogs => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Overlay {
    None,
    Help,
    Details,
    DryRun,
    Confirm(ConfirmState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfirmState {
    target_count: usize,
    estimated_bytes: u64,
    has_irreversible_commands: bool,
    required_phrase: String,
    input: String,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobKind {
    Scan,
    Clean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobStatus {
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Canceled,
}

impl JobStatus {
    fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::Cancelling)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JobRecord {
    id: JobId,
    kind: JobKind,
    label: String,
    status: JobStatus,
    progress: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogEntry {
    message: String,
}

fn render_app(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_tabs(frame, chunks[1], app);
    render_body(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
    render_overlay(frame, app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let active_jobs = app.jobs.iter().filter(|job| job.status.is_active()).count();
    let filter = if app.filter.is_empty() {
        "none".to_string()
    } else {
        app.filter.clone()
    };
    let risk = app.risk_filter.as_ref().map(risk_label).unwrap_or("all");
    let text = vec![
        Line::from(vec![
            Span::styled("devsweep", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "  Global: {}  Projects: {}  Selected: {} ({})",
                format_bytes(app.scope_bytes(ScopeKind::Global)),
                format_bytes(app.scope_bytes(ScopeKind::Project)),
                app.selected_ids.len(),
                format_bytes(app.selected_bytes())
            )),
        ]),
        Line::from(format!(
            "Filter: {filter}  Risk: {risk}  Active jobs: {active_jobs}"
        )),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(text))
            .block(Block::bordered().title("Summary"))
            .alignment(Alignment::Left),
        area,
    );
}

fn render_tabs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let titles = ActiveTab::ALL
        .iter()
        .map(|tab| Line::from(tab.title()))
        .collect::<Vec<_>>();
    let selected = ActiveTab::ALL
        .iter()
        .position(|tab| *tab == app.active_tab)
        .unwrap_or_default();
    let tabs = Tabs::new(titles)
        .select(selected)
        .block(Block::bordered().title("Views"))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, app: &App) {
    match app.active_tab {
        ActiveTab::Rules => render_rules(frame, area),
        ActiveTab::JobsLogs => render_jobs_logs(frame, area, app),
        ActiveTab::Dashboard | ActiveTab::Global | ActiveTab::Projects => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(22),
                    Constraint::Percentage(45),
                    Constraint::Percentage(33),
                ])
                .split(area);
            render_categories(frame, chunks[0], app);
            render_targets(frame, chunks[1], app);
            render_details_panel(frame, chunks[2], app);
        }
    }
}

fn render_categories(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = vec![
        Line::from(format!(
            "Global     {}",
            count_scope(app, ScopeKind::Global)
        )),
        Line::from(format!(
            "Projects   {}",
            count_scope(app, ScopeKind::Project)
        )),
        Line::from(""),
        Line::from(format!(
            "Rust       {}",
            count_ecosystem(app, Ecosystem::Rust)
        )),
        Line::from(format!(
            "Node       {}",
            count_ecosystem(app, Ecosystem::Node)
        )),
        Line::from(format!(
            "Python     {}",
            count_ecosystem(app, Ecosystem::Python)
        )),
        Line::from(format!(
            "Generic    {}",
            count_ecosystem(app, Ecosystem::Generic)
        )),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(Block::bordered().title("Categories")),
        area,
    );
}

fn render_targets(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let visible = app.visible_target_indices();
    let selected_row = app.selected_index.min(visible.len().saturating_sub(1));
    let lines = if visible.is_empty() {
        vec![Line::from("No targets in this view.")]
    } else {
        visible
            .iter()
            .enumerate()
            .map(|(row, index)| {
                let target = &app.targets[*index];
                let cursor = if row == selected_row { ">" } else { " " };
                let mark = if app.selected_ids.contains(&target.id) {
                    "[x]"
                } else {
                    "[ ]"
                };
                Line::from(format!(
                    "{cursor} {mark} {:<9} {:>9} {}",
                    risk_label(&target.risk),
                    format_bytes(target.estimated_bytes),
                    target_title(target)
                ))
            })
            .collect()
    };

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title("Targets"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_details_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = app
        .selected_target()
        .map(target_details_lines)
        .unwrap_or_else(|| vec![Line::from("No target selected.")]);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title("Details"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_rules(frame: &mut Frame<'_>, area: Rect) {
    let lines = vec![
        Line::from("Project rules"),
        Line::from("  rust.target -> cargo clean"),
        Line::from("  node.node_modules -> trash"),
        Line::from("  node cache dirs -> trash"),
        Line::from("  python caches and venvs -> trash"),
        Line::from(""),
        Line::from("Global providers"),
        Line::from("  npm, pip, pnpm, yarn -> official commands"),
        Line::from("  cargo home -> inspect only"),
        Line::from("  docker -> deferred"),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title("Rules"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_jobs_logs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let job_lines = if app.jobs.is_empty() {
        vec![Line::from("No jobs yet.")]
    } else {
        app.jobs
            .iter()
            .rev()
            .take(12)
            .map(|job| {
                Line::from(format!(
                    "#{} {:?} {:?}: {} ({})",
                    job.id, job.kind, job.status, job.label, job.progress
                ))
            })
            .collect()
    };
    let log_lines = if app.logs.is_empty() {
        vec![Line::from("No logs yet.")]
    } else {
        app.logs
            .iter()
            .rev()
            .take(16)
            .map(|entry| Line::from(entry.message.clone()))
            .collect()
    };

    frame.render_widget(
        Paragraph::new(Text::from(job_lines))
            .block(Block::bordered().title("Jobs"))
            .wrap(Wrap { trim: false }),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(Text::from(log_lines))
            .block(Block::bordered().title("Logs"))
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mode = if app.filter_active {
        "FILTER"
    } else if matches!(app.overlay, Overlay::Confirm(_)) {
        "CONFIRM"
    } else {
        "NORMAL"
    };
    let text = format!(
        "{mode}  s scan | Space select | a all | d dry-run | c clean | / filter | r risk | x cancel | ? help | q quit"
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::bordered())
            .alignment(Alignment::Center),
        area,
    );
}

fn render_overlay(frame: &mut Frame<'_>, app: &App) {
    match &app.overlay {
        Overlay::None => {}
        Overlay::Help => render_modal(
            frame,
            "Keyboard help",
            vec![
                Line::from("s scan current directory and global providers"),
                Line::from("Space toggles the selected target"),
                Line::from("a toggles all visible targets"),
                Line::from("d opens dry-run preview"),
                Line::from("c opens cleanup confirmation"),
                Line::from("/ filters targets; r cycles risk filter"),
                Line::from("x requests active job cancellation"),
                Line::from("Esc closes overlays; q quits"),
            ],
        ),
        Overlay::Details => {
            let lines = app
                .selected_target()
                .map(target_details_lines)
                .unwrap_or_else(|| vec![Line::from("No target selected.")]);
            render_modal(frame, "Target details", lines);
        }
        Overlay::DryRun => render_modal(frame, "Dry-run preview", dry_run_lines(app)),
        Overlay::Confirm(confirm) => render_confirm(frame, confirm),
    }
}

fn render_confirm(frame: &mut Frame<'_>, confirm: &ConfirmState) {
    let strength = if confirm.has_irreversible_commands {
        "Irreversible command-backed cleanup"
    } else {
        "Trash-backed cleanup"
    };
    render_modal(
        frame,
        "Confirm cleanup",
        vec![
            Line::from(strength),
            Line::from(confirm.message.clone()),
            Line::from(format!(
                "Targets: {}  Estimated: {}",
                confirm.target_count,
                format_bytes(confirm.estimated_bytes)
            )),
            Line::from(format!("Type: {}", confirm.required_phrase)),
            Line::from(format!("Input: {}", confirm.input)),
        ],
    );
}

fn render_modal(frame: &mut Frame<'_>, title: &'static str, lines: Vec<Line<'static>>) {
    let area = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title(title))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

fn dry_run_lines(app: &App) -> Vec<Line<'static>> {
    let selected = app.selected_targets();
    if selected.is_empty() {
        return vec![Line::from("No selected targets.")];
    }

    let mut lines = vec![
        Line::from(format!(
            "{} selected target(s), {} estimated.",
            selected.len(),
            format_bytes(app.selected_bytes())
        )),
        Line::from(""),
    ];
    lines.extend(selected.into_iter().take(12).map(|target| {
        Line::from(format!(
            "{} | {} | {}",
            target_title(target),
            risk_label(&target.risk),
            action_summary(&target.action)
        ))
    }));
    lines
}

fn target_details_lines(target: &CleanTarget) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("ID: {}", target.id.as_str())),
        Line::from(format!("Scope: {}", scope_label(&target.scope))),
        Line::from(format!("Path: {}", path_label(target.path.as_ref()))),
        Line::from(format!("Risk: {}", risk_label(&target.risk))),
        Line::from(format!("Size: {}", format_bytes(target.estimated_bytes))),
        Line::from(format!("Reversible: {}", target.reversible)),
        Line::from(format!("Action: {}", action_summary(&target.action))),
        Line::from("Evidence:"),
    ];
    lines.extend(
        target
            .evidence
            .iter()
            .take(8)
            .map(|evidence| Line::from(format!("  {}", evidence_summary(evidence)))),
    );
    lines
}

fn count_scope(app: &App, scope_kind: ScopeKind) -> usize {
    app.targets
        .iter()
        .filter(|target| match scope_kind {
            ScopeKind::Global => matches!(target.scope, Scope::Global),
            ScopeKind::Project => matches!(target.scope, Scope::Project { .. }),
        })
        .count()
}

fn count_ecosystem(app: &App, ecosystem: Ecosystem) -> usize {
    app.targets
        .iter()
        .filter(|target| target.ecosystem == ecosystem)
        .count()
}

fn target_title(target: &CleanTarget) -> String {
    target
        .path
        .as_ref()
        .map(|path| compact_path(path))
        .unwrap_or_else(|| target.id.as_str().to_string())
}

fn compact_path(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    let char_count = text.chars().count();
    if char_count <= 48 {
        return text;
    }

    let tail = text
        .chars()
        .rev()
        .take(45)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("...{tail}")
}

fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Global => "Global".to_string(),
        Scope::Project { root } => format!("Project ({})", root.display()),
    }
}

fn path_label(path: Option<&PathBuf>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "Low",
        RiskLevel::Medium => "Medium",
        RiskLevel::High => "High",
        RiskLevel::Dangerous => "Dangerous",
    }
}

fn action_summary(action: &CleanAction) -> String {
    match action {
        CleanAction::Command {
            program,
            args,
            irreversible,
            cwd: _,
        } => {
            let mut command = Vec::with_capacity(args.len() + 1);
            command.push(program.clone());
            command.extend(args.clone());
            let suffix = if *irreversible { " irreversible" } else { "" };
            format!("command{}: {}", suffix, command.join(" "))
        }
        CleanAction::MoveToTrash { path } => format!("trash: {}", path.display()),
        CleanAction::DeletePermanently { path, .. } => {
            format!("permanent delete disabled: {}", path.display())
        }
        CleanAction::NoopInspectOnly => "inspect only".to_string(),
    }
}

fn evidence_summary(evidence: &Evidence) -> String {
    match evidence {
        Evidence::MarkerFile { path } => format!("marker {}", path.display()),
        Evidence::KnownCacheDir { source, path } => {
            format!("{source} -> {}", path.display())
        }
        Evidence::OfficialCommand { command } => format!("official command {command}"),
        Evidence::RuleMatched { rule_id } => format!("rule {rule_id}"),
        Evidence::UserConfigured => "user configured".to_string(),
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::model::{CLEANUP_PLAN_VERSION, TargetKind};

    #[test]
    fn representative_state_renders_with_targets_details_and_jobs() {
        let mut app = App::with_plan(representative_plan());
        let job_id = app.start_job(JobKind::Scan, "Scan fixture");
        app.update(UiEvent::Worker(WorkerEvent::JobProgress {
            job_id,
            message: "Scanning fixture".to_string(),
        }));
        app.active_tab = ActiveTab::JobsLogs;

        let backend = TestBackend::new(120, 32);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("representative app renders");
    }

    #[test]
    fn update_handles_scan_selection_details_filter_help_and_quit() {
        let mut app = App::with_plan(representative_plan());

        let effects = app.update(key(KeyCode::Char('s')));
        assert!(matches!(effects.as_slice(), [Effect::StartScan { .. }]));
        assert_eq!(app.jobs.len(), 1);

        let first_id = app.targets[0].id.clone();
        assert!(app.selected_ids.contains(&first_id));
        app.update(key(KeyCode::Char(' ')));
        assert!(!app.selected_ids.contains(&first_id));

        app.update(key(KeyCode::Down));
        app.update(key(KeyCode::Enter));
        assert!(matches!(app.overlay, Overlay::Details));
        app.update(key(KeyCode::Esc));
        assert!(matches!(app.overlay, Overlay::None));

        app.update(key(KeyCode::Char('/')));
        assert!(app.filter_active);
        app.update(key(KeyCode::Char('n')));
        app.update(key(KeyCode::Char('p')));
        app.update(key(KeyCode::Enter));
        assert_eq!(app.filter, "np");
        assert!(!app.filter_active);

        app.update(key(KeyCode::Char('?')));
        assert!(matches!(app.overlay, Overlay::Help));
        app.update(key(KeyCode::Esc));

        app.update(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn clean_confirmation_copy_differs_by_action_strength() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[0].id.clone());

        app.update(key(KeyCode::Char('c')));
        let Overlay::Confirm(confirm) = &app.overlay else {
            panic!("trash selection opens confirm");
        };
        assert_eq!(confirm.required_phrase, "clean");
        assert!(confirm.message.contains("Trash"));
        assert!(!confirm.has_irreversible_commands);

        app.overlay = Overlay::None;
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[1].id.clone());

        app.update(key(KeyCode::Char('c')));
        let Overlay::Confirm(confirm) = &app.overlay else {
            panic!("command selection opens confirm");
        };
        assert!(confirm.required_phrase.starts_with("CLEAN "));
        assert!(confirm.message.contains("irreversible"));
        assert!(confirm.has_irreversible_commands);
    }

    #[test]
    fn accepted_confirmation_emits_selected_clean_plan() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[0].id.clone());

        app.update(key(KeyCode::Char('c')));
        for ch in "clean".chars() {
            app.update(key(KeyCode::Char(ch)));
        }
        let effects = app.update(key(KeyCode::Enter));

        let [Effect::StartClean { job_id, plan }] = effects.as_slice() else {
            panic!("confirmation emits clean effect");
        };
        assert_eq!(*job_id, 1);
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.targets[0].id, app.targets[0].id);
        assert!(plan.targets[0].selected_by_default);
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn worker_events_update_jobs_and_targets() {
        let mut app = App::new();
        let effects = app.update(key(KeyCode::Char('s')));
        let [Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("scan key starts scan");
        };
        let job_id = *job_id;

        app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
            job_id,
            plan: representative_plan(),
        }));

        assert_eq!(app.targets.len(), 3);
        assert_eq!(app.jobs[0].status, JobStatus::Succeeded);
        assert!(app.selected_ids.contains(&app.targets[0].id));

        let report = ExecutionReport {
            dry_run: false,
            selected: 1,
            attempted: 1,
            succeeded: 1,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
            audit_log: Some(PathBuf::from("audit.jsonl")),
        };
        app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
            job_id,
            report,
        }));
        assert!(
            app.logs
                .iter()
                .any(|entry| entry.message.contains("audit.jsonl"))
        );
    }

    #[test]
    fn cancellation_key_requests_active_job_cancellation() {
        let mut app = App::with_plan(representative_plan());
        let effects = app.update(key(KeyCode::Char('s')));
        let [Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("scan key starts scan");
        };
        let job_id = *job_id;

        let effects = app.update(key(KeyCode::Char('x')));

        assert!(matches!(
            effects.as_slice(),
            [Effect::CancelJob { job_id: requested }] if *requested == job_id
        ));
        assert_eq!(app.jobs[0].status, JobStatus::Cancelling);

        app.update(UiEvent::Worker(WorkerEvent::JobCanceled { job_id }));
        assert_eq!(app.jobs[0].status, JobStatus::Canceled);
    }

    #[test]
    fn render_path_does_not_mutate_app_state() {
        let mut app = App::with_plan(representative_plan());
        app.overlay = Overlay::DryRun;
        let before = app.clone();
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("dry-run preview renders");

        assert_eq!(app, before);
    }

    fn key(code: KeyCode) -> UiEvent {
        UiEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn representative_plan() -> CleanupPlan {
        CleanupPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "node.next_cache",
                    Scope::Project {
                        root: PathBuf::from("D:/code/web"),
                    },
                    Ecosystem::Node,
                    TargetKind::BuildArtifacts,
                    Some(PathBuf::from("D:/code/web/.next/cache")),
                    1024,
                    RiskLevel::Low,
                    true,
                    true,
                    CleanAction::MoveToTrash {
                        path: PathBuf::from("D:/code/web/.next/cache"),
                    },
                ),
                target(
                    "npm.cache.clean",
                    Scope::Global,
                    Ecosystem::Node,
                    TargetKind::PackageCache,
                    Some(PathBuf::from("C:/Users/me/AppData/Local/npm-cache")),
                    2048,
                    RiskLevel::Medium,
                    false,
                    false,
                    CleanAction::Command {
                        program: "npm".to_string(),
                        args: vec![
                            "cache".to_string(),
                            "clean".to_string(),
                            "--force".to_string(),
                        ],
                        cwd: None,
                        irreversible: true,
                    },
                ),
                target(
                    "cargo.home.inspect",
                    Scope::Global,
                    Ecosystem::Rust,
                    TargetKind::PackageCache,
                    Some(PathBuf::from("C:/Users/me/.cargo")),
                    0,
                    RiskLevel::High,
                    false,
                    true,
                    CleanAction::NoopInspectOnly,
                ),
            ],
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn target(
        rule_id: &str,
        scope: Scope,
        ecosystem: Ecosystem,
        kind: TargetKind,
        path: Option<PathBuf>,
        estimated_bytes: u64,
        risk: RiskLevel,
        selected_by_default: bool,
        reversible: bool,
        action: CleanAction,
    ) -> CleanTarget {
        CleanTarget {
            id: TargetId::new(format!(
                "{rule_id}:{}",
                path.as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string())
            )),
            scope,
            ecosystem,
            kind,
            path: path.clone(),
            estimated_bytes,
            last_modified: None,
            risk,
            reversible,
            selected_by_default,
            evidence: vec![
                Evidence::RuleMatched {
                    rule_id: rule_id.to_string(),
                },
                Evidence::OfficialCommand {
                    command: action_summary(&action),
                },
            ],
            action,
        }
    }
}
