use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    executor::{ExecutionReport, ExecutionTargetStatus},
    model::{CleanAction, CleanTarget, CleanupPlan, RiskLevel, Scope, TargetId},
    plan_validation::validate_scanned_plan,
    sweep::ScanPhase,
};

use super::render::{
    action_summary, command_previews, compact_target_id, display_path, format_cleanup_progress,
    selected_target_summary, target_title,
};

pub(super) type JobId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionOverride {
    Selected,
    Deselected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct App {
    pub(super) targets: Vec<CleanTarget>,
    pub(super) selected_ids: HashSet<TargetId>,
    selection_overrides: HashMap<TargetId, SelectionOverride>,
    pub(super) selected_index: usize,
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
    pub(super) next_job_id: JobId,
}

impl App {
    pub(super) fn new() -> Self {
        Self::with_plan(CleanupPlan::empty())
    }

    pub(super) fn startup_effects(&mut self) -> Vec<Effect> {
        let job_id = self.start_scan_job("Startup scan requested");
        vec![Effect::StartScan { job_id }]
    }

    pub(super) fn with_plan(plan: CleanupPlan) -> Self {
        let selected_ids = default_selected_ids(&plan.targets);

        let mut app = Self {
            targets: plan.targets,
            selected_ids,
            selection_overrides: HashMap::new(),
            selected_index: 0,
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
            next_job_id: 1,
        };
        app.log("Ready");
        app
    }

    pub(super) fn update(&mut self, event: UiEvent) -> Vec<Effect> {
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

        if matches!(self.overlay, Overlay::None)
            && self
                .cleanup_progress
                .as_ref()
                .is_some_and(|progress| progress.finished)
            && matches!(key.code, KeyCode::Esc | KeyCode::Enter)
        {
            self.cleanup_progress = None;
            return Vec::new();
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
                if self.has_active_clean_job() {
                    self.log_entry(
                        AppLogLevel::Warning,
                        AppLogSource::Scan,
                        None,
                        None,
                        "Cleanup is active; scan requests are disabled until it finishes",
                    );
                    return Vec::new();
                }
                let job_id = self.start_scan_job("Scan requested");
                vec![Effect::StartScan { job_id }]
            }
            KeyCode::Char('c') => {
                if self.has_active_clean_job() {
                    self.log_entry(
                        AppLogLevel::Warning,
                        AppLogSource::Clean,
                        None,
                        None,
                        "Cleanup is already active; a second cleanup cannot start",
                    );
                } else if self.selected_targets().is_empty() {
                    self.log_entry(
                        AppLogLevel::Warning,
                        AppLogSource::Clean,
                        None,
                        None,
                        "No selected targets to clean",
                    );
                } else {
                    match self.confirm_state() {
                        Ok(confirm) => self.overlay = Overlay::Confirm(confirm),
                        Err(error) => self.log_entry(
                            AppLogLevel::Error,
                            AppLogSource::Clean,
                            None,
                            None,
                            format!("Selected cleanup plan failed validation: {error}"),
                        ),
                    }
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
                    confirm.feedback = None;
                }
                Vec::new()
            }
            KeyCode::Char(ch) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && let Overlay::Confirm(confirm) = &mut self.overlay
                {
                    confirm.input.push(ch);
                    confirm.feedback = None;
                }
                Vec::new()
            }
            KeyCode::Enter => {
                let (invalidated_by_scan, accepted) = match &self.overlay {
                    Overlay::Confirm(confirm) => (
                        confirm.invalidated_by_scan,
                        confirm.input.trim() == confirm.required_phrase,
                    ),
                    _ => return Vec::new(),
                };

                if invalidated_by_scan {
                    if let Overlay::Confirm(confirm) = &mut self.overlay {
                        confirm.feedback = Some(
                            "Scan results changed. Close this dialog and confirm the updated selection."
                                .to_string(),
                        );
                    }
                    self.log_entry(
                        AppLogLevel::Warning,
                        AppLogSource::Clean,
                        None,
                        None,
                        "Confirmation rejected because scan results changed",
                    );
                    return Vec::new();
                }

                if !accepted {
                    if let Overlay::Confirm(confirm) = &mut self.overlay {
                        confirm.feedback = Some(format!(
                            "Type {} before pressing Enter.",
                            confirm.required_phrase
                        ));
                    }
                    self.log_entry(
                        AppLogLevel::Warning,
                        AppLogSource::Clean,
                        None,
                        None,
                        "Confirmation phrase did not match",
                    );
                    return Vec::new();
                }

                let manifest = match &self.overlay {
                    Overlay::Confirm(confirm) => confirm.manifest.clone(),
                    _ => return Vec::new(),
                };
                let ExecutionManifest {
                    plan,
                    selected,
                    digest,
                } = *manifest;
                let target_count = plan.targets.len();
                self.overlay = Overlay::None;
                let job_id =
                    self.start_job(JobKind::Clean, format!("Clean {target_count} target(s)"));
                self.cleanup_progress = Some(cleanup_progress_for_plan(job_id, &plan));
                self.log_job(
                    AppLogLevel::Info,
                    AppLogSource::Clean,
                    job_id,
                    format!("Clean requested for {target_count} target(s)"),
                );
                vec![Effect::StartClean {
                    job_id,
                    plan,
                    selected,
                    plan_digest: digest,
                }]
            }
            _ => Vec::new(),
        }
    }

    fn handle_worker_event(&mut self, event: WorkerEvent) -> Vec<Effect> {
        match event {
            WorkerEvent::ScanStarted { job_id } => {
                self.ensure_job(job_id, JobKind::Scan, "Scan current directory and globals");
                if !self.transition_job(job_id, JobStatus::Running, "Started") {
                    self.log_ignored_worker_event(job_id, "scan started");
                }
            }
            WorkerEvent::JobProgress { job_id, message } => {
                if self.transition_job(job_id, JobStatus::Running, message.clone()) {
                    self.log_job(AppLogLevel::Info, self.job_source(job_id), job_id, message);
                } else {
                    self.log_ignored_worker_event(job_id, "job progress");
                }
            }
            WorkerEvent::ScanProgress {
                job_id,
                phase,
                message,
                plan,
            } => {
                self.handle_scan_progress(job_id, phase, message, plan);
            }
            WorkerEvent::CleanProgress {
                job_id,
                target_id,
                status,
                message,
                detail,
                completed,
                total,
            } => {
                let progress = format_cleanup_progress(completed, total, &message);
                if !self.transition_job(job_id, JobStatus::Running, progress) {
                    self.log_ignored_worker_event(job_id, "cleanup progress");
                    return Vec::new();
                }
                self.update_cleanup_progress_item(CleanupProgressUpdate {
                    job_id,
                    completed,
                    total,
                    summary: message.clone(),
                    target_id: target_id.clone(),
                    status,
                    detail,
                });
                self.log_target(
                    log_level_for_execution_status(status),
                    AppLogSource::Clean,
                    job_id,
                    target_id,
                    format_cleanup_progress(completed, total, &message),
                );
            }
            WorkerEvent::ScanFinished { job_id, plan } => {
                let count = plan.targets.len();
                let should_apply_scan_update = self.should_apply_scan_update(job_id);
                if !self.transition_job(
                    job_id,
                    JobStatus::Succeeded,
                    format!("Found {count} target(s)"),
                ) {
                    self.log_ignored_worker_event(job_id, "scan finished");
                    return Vec::new();
                }
                if should_apply_scan_update {
                    self.invalidate_confirmation_due_to_scan();
                    self.replace_targets_preserving_selection(plan.targets);
                    self.scan_snapshot = None;
                }
                self.log_job(
                    AppLogLevel::Info,
                    AppLogSource::Scan,
                    job_id,
                    format!("Scan finished: {count} target(s)"),
                );
            }
            WorkerEvent::CleanFinished { job_id, report } => {
                let status = if report.failed == 0 {
                    JobStatus::Succeeded
                } else {
                    JobStatus::Failed
                };
                let summary = format!(
                    "{} succeeded, {} failed, {} skipped",
                    report.succeeded, report.failed, report.skipped
                );
                if !self.transition_job(job_id, status, summary.clone()) {
                    self.log_ignored_worker_event(job_id, "cleanup finished");
                    return Vec::new();
                }
                self.finish_cleanup_progress(job_id, &report, &summary);
                self.log_job(
                    if report.failed == 0 {
                        AppLogLevel::Info
                    } else {
                        AppLogLevel::Error
                    },
                    AppLogSource::Clean,
                    job_id,
                    format!("Clean finished: {summary}"),
                );
                if let Some(path) = report.audit_log {
                    self.log_job(
                        AppLogLevel::Info,
                        AppLogSource::Audit,
                        job_id,
                        format!("Audit log: {}", display_path(&path)),
                    );
                }
            }
            WorkerEvent::JobFailed { job_id, message } => {
                if !self.transition_job(job_id, JobStatus::Failed, message.clone()) {
                    self.log_ignored_worker_event(job_id, "job failure");
                    return Vec::new();
                }
                self.clear_scan_snapshot(job_id);
                if self.cleanup_progress_matches(job_id)
                    && let Some(progress) = &mut self.cleanup_progress
                {
                    progress.summary = format!("failed: {message}");
                    progress.finished = true;
                }
                self.log_job(AppLogLevel::Error, self.job_source(job_id), job_id, message);
            }
            WorkerEvent::JobCanceled { job_id } => {
                if !self.transition_job(job_id, JobStatus::Canceled, "Canceled") {
                    self.log_ignored_worker_event(job_id, "job cancellation");
                    return Vec::new();
                }
                self.clear_scan_snapshot(job_id);
                if self.cleanup_progress_matches(job_id) {
                    self.cleanup_progress = None;
                }
                self.log_job(
                    AppLogLevel::Warning,
                    self.job_source(job_id),
                    job_id,
                    format!("Job {job_id} canceled"),
                );
            }
        }

        Vec::new()
    }

    fn set_tab(&mut self, tab: ActiveTab) -> Vec<Effect> {
        self.active_tab = tab;
        self.selected_index = 0;
        Vec::new()
    }

    fn start_scan_job(&mut self, log_message: impl Into<String>) -> JobId {
        let job_id = self.start_job(JobKind::Scan, "Scan current directory and globals");
        self.scan_snapshot = Some(ScanSnapshot::new(job_id));
        self.log_job(AppLogLevel::Info, AppLogSource::Scan, job_id, log_message);
        job_id
    }

    fn handle_scan_progress(
        &mut self,
        job_id: JobId,
        _phase: ScanPhase,
        message: String,
        plan: Option<CleanupPlan>,
    ) {
        if !self.transition_job(job_id, JobStatus::Running, message.clone()) {
            self.log_ignored_worker_event(job_id, "scan progress");
            return;
        }
        self.log_job(AppLogLevel::Info, AppLogSource::Scan, job_id, message);

        let should_apply_scan_update = self.should_apply_scan_update(job_id);
        if should_apply_scan_update {
            self.invalidate_confirmation_due_to_scan();
        }

        let Some(plan) = plan else {
            return;
        };

        if !should_apply_scan_update {
            return;
        }

        if self.scan_snapshot.is_none() {
            self.scan_snapshot = Some(ScanSnapshot::new(job_id));
        }

        let mut updated = false;
        if let Some(snapshot) = &mut self.scan_snapshot
            && snapshot.job_id == job_id
        {
            snapshot.set_targets(plan.targets);
            updated = true;
        }

        if updated {
            self.rebuild_targets_from_scan_snapshot();
        }
    }

    fn should_apply_scan_update(&self, job_id: JobId) -> bool {
        let Some(job) = self.jobs.iter().find(|job| job.id == job_id) else {
            return false;
        };
        if job.kind != JobKind::Scan || !job.status.is_active() {
            return false;
        }
        self.jobs
            .iter()
            .filter(|job| job.kind == JobKind::Scan)
            .map(|job| job.id)
            .max()
            == Some(job_id)
    }

    fn rebuild_targets_from_scan_snapshot(&mut self) {
        if let Some(snapshot) = &self.scan_snapshot {
            self.replace_targets_preserving_selection(snapshot.targets());
        }
    }

    fn replace_targets_preserving_selection(&mut self, targets: Vec<CleanTarget>) {
        let available_ids: HashSet<TargetId> =
            targets.iter().map(|target| target.id.clone()).collect();
        self.selection_overrides
            .retain(|target_id, _| available_ids.contains(target_id));
        self.selected_ids = targets
            .iter()
            .filter_map(|target| {
                if !target.action.is_executable() {
                    return None;
                }
                match self.selection_overrides.get(&target.id) {
                    Some(SelectionOverride::Selected) => Some(target.id.clone()),
                    Some(SelectionOverride::Deselected) => None,
                    None if target.selected_by_default => Some(target.id.clone()),
                    None => None,
                }
            })
            .collect();
        self.targets = targets;
        self.selected_index = 0;
    }

    fn invalidate_confirmation_due_to_scan(&mut self) {
        if let Overlay::Confirm(confirm) = &mut self.overlay {
            confirm.invalidated_by_scan = true;
            confirm.feedback = Some(
                "Scan results changed. Close this dialog and confirm the updated selection."
                    .to_string(),
            );
        }
    }

    fn clear_scan_snapshot(&mut self, job_id: JobId) {
        if self
            .scan_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.job_id == job_id)
        {
            self.scan_snapshot = None;
        }
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
        let Some((target_id, executable)) = self
            .selected_target()
            .map(|target| (target.id.clone(), target.action.is_executable()))
        else {
            return;
        };

        if !executable {
            self.log_entry(
                AppLogLevel::Warning,
                AppLogSource::Clean,
                None,
                Some(target_id),
                "Inspect-only targets cannot be selected for cleanup",
            );
            return;
        }

        let selected = !self.selected_ids.contains(&target_id);
        self.set_target_selected(target_id, selected);
    }

    fn toggle_visible_selection(&mut self) {
        let visible_ids: Vec<TargetId> = self
            .visible_target_indices()
            .into_iter()
            .filter(|index| self.targets[*index].action.is_executable())
            .map(|index| self.targets[index].id.clone())
            .collect();

        if visible_ids.is_empty() {
            return;
        }

        let all_selected = visible_ids.iter().all(|id| self.selected_ids.contains(id));
        for id in visible_ids {
            self.set_target_selected(id, !all_selected);
        }
    }

    fn set_target_selected(&mut self, target_id: TargetId, selected: bool) {
        if selected
            && self
                .targets
                .iter()
                .any(|target| target.id == target_id && !target.action.is_executable())
        {
            return;
        }

        if selected {
            self.selected_ids.insert(target_id.clone());
            self.selection_overrides
                .insert(target_id, SelectionOverride::Selected);
        } else {
            self.selected_ids.remove(&target_id);
            self.selection_overrides
                .insert(target_id, SelectionOverride::Deselected);
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

        if !self.transition_job(
            job_id,
            JobStatus::Cancelling,
            "Cancellation requested; current action cannot be interrupted.",
        ) {
            self.log_job(
                AppLogLevel::Warning,
                self.job_source(job_id),
                job_id,
                "Cancellation was already requested",
            );
            return Vec::new();
        }
        self.log_job(
            AppLogLevel::Warning,
            self.job_source(job_id),
            job_id,
            "Cancellation requested; current action cannot be interrupted.",
        );
        vec![Effect::CancelJob { job_id }]
    }

    pub(super) fn start_job(&mut self, kind: JobKind, label: impl Into<String>) -> JobId {
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

    fn transition_job(
        &mut self,
        job_id: JobId,
        status: JobStatus,
        progress: impl Into<String>,
    ) -> bool {
        if let Some(job) = self.jobs.iter_mut().find(|job| job.id == job_id) {
            if !job.status.can_transition_to(status) {
                return false;
            }
            job.status = status;
            job.progress = progress.into();
            return true;
        }
        false
    }

    fn has_active_clean_job(&self) -> bool {
        self.jobs
            .iter()
            .any(|job| job.kind == JobKind::Clean && job.status.is_active())
    }

    fn log_ignored_worker_event(&mut self, job_id: JobId, event: &str) {
        self.log_job(
            AppLogLevel::Warning,
            self.job_source(job_id),
            job_id,
            format!("Ignored delayed {event} event"),
        );
    }

    fn job_source(&self, job_id: JobId) -> AppLogSource {
        self.jobs
            .iter()
            .find(|job| job.id == job_id)
            .map(|job| match job.kind {
                JobKind::Scan => AppLogSource::Scan,
                JobKind::Clean => AppLogSource::Clean,
            })
            .unwrap_or(AppLogSource::App)
    }

    fn cleanup_progress_matches(&self, job_id: JobId) -> bool {
        self.cleanup_progress
            .as_ref()
            .is_some_and(|progress| progress.job_id == job_id)
    }

    fn update_cleanup_progress_item(&mut self, update: CleanupProgressUpdate) {
        if !self.cleanup_progress_matches(update.job_id) {
            self.cleanup_progress = Some(CleanupProgress {
                job_id: update.job_id,
                completed: 0,
                total: update.total,
                summary: "Executing selected cleanup plan".to_string(),
                finished: false,
                items: Vec::new(),
            });
        }

        let Some(progress) = &mut self.cleanup_progress else {
            return;
        };

        progress.completed = update.completed;
        progress.total = update.total;
        progress.summary = update.summary;
        progress.finished = false;

        let item_status = CleanupItemStatus::from(update.status);
        let item_detail = cleanup_item_detail(item_status, update.detail);
        if let Some(item) = progress
            .items
            .iter_mut()
            .find(|item| item.target_id == update.target_id)
        {
            item.status = item_status;
            item.detail = item_detail;
            return;
        }

        progress.items.push(CleanupProgressItem {
            label: compact_target_id(&update.target_id),
            target_id: update.target_id,
            status: item_status,
            detail: item_detail,
        });
    }

    fn finish_cleanup_progress(&mut self, job_id: JobId, report: &ExecutionReport, summary: &str) {
        if !self.cleanup_progress_matches(job_id) {
            self.cleanup_progress = Some(CleanupProgress {
                job_id,
                completed: 0,
                total: report.selected,
                summary: String::new(),
                finished: false,
                items: Vec::new(),
            });
        }

        let Some(progress) = &mut self.cleanup_progress else {
            return;
        };

        progress.completed = report.succeeded + report.failed + report.skipped;
        progress.total = report.selected;
        progress.summary = format!("finished: {summary}");
        progress.finished = true;

        for failure in &report.failures {
            if let Some(item) = progress
                .items
                .iter_mut()
                .find(|item| item.target_id == failure.target_id)
            {
                item.status = CleanupItemStatus::Failed;
                item.detail = Some(failure.message.clone());
            } else {
                progress.items.push(CleanupProgressItem {
                    target_id: failure.target_id.clone(),
                    label: compact_target_id(&failure.target_id),
                    status: CleanupItemStatus::Failed,
                    detail: Some(failure.message.clone()),
                });
            }
        }
    }

    fn confirm_state(&self) -> Result<ConfirmState, String> {
        let selected = self.selected_targets();
        let manifest = ExecutionManifest::from_targets(&selected)?;
        let selected_targets = &manifest.plan.targets;
        let estimated_bytes = sum_unique_target_bytes(selected_targets.iter());
        let has_irreversible_commands = selected_targets.iter().any(|target| {
            matches!(
                target.action,
                CleanAction::Command {
                    irreversible: true,
                    ..
                } | CleanAction::DeletePermanently { .. }
            )
        });

        let required_phrase = "confirm".to_string();
        let message = if has_irreversible_commands {
            "Type confirm to run these irreversible command-backed cleanups. They cannot be reversed by devsweep."
                .to_string()
        } else {
            "Type confirm to move selected targets to Trash.".to_string()
        };

        Ok(ConfirmState {
            target_count: selected_targets.len(),
            estimated_bytes,
            has_irreversible_commands,
            required_phrase,
            input: String::new(),
            feedback: None,
            message,
            plan_digest_prefix: manifest.digest_prefix().to_string(),
            selected_targets: selected_targets
                .iter()
                .map(selected_target_summary)
                .collect(),
            command_previews: command_previews(selected_targets.iter()),
            manifest: Box::new(manifest),
            invalidated_by_scan: false,
        })
    }

    pub(super) fn selected_target(&self) -> Option<&CleanTarget> {
        let indices = self.visible_target_indices();
        let index = indices.get(self.selected_index.min(indices.len().saturating_sub(1)))?;
        self.targets.get(*index)
    }

    pub(super) fn selected_targets(&self) -> Vec<&CleanTarget> {
        self.targets
            .iter()
            .filter(|target| {
                self.selected_ids.contains(&target.id) && target.action.is_executable()
            })
            .collect()
    }

    pub(super) fn visible_target_indices(&self) -> Vec<usize> {
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
                .map(|path| display_path(path).to_ascii_lowercase())
                .is_some_and(|path| path.contains(&needle))
            || action_summary(&target.action)
                .to_ascii_lowercase()
                .contains(&needle)
    }

    pub(super) fn selected_bytes(&self) -> u64 {
        sum_unique_target_bytes(self.selected_targets())
    }

    pub(super) fn scope_bytes(&self, scope_kind: ScopeKind) -> u64 {
        sum_unique_target_bytes(self.targets.iter().filter(|target| match scope_kind {
            ScopeKind::Global => matches!(target.scope, Scope::Global),
            ScopeKind::Project => matches!(target.scope, Scope::Project { .. }),
        }))
    }

    fn log(&mut self, message: impl Into<String>) {
        self.log_entry(AppLogLevel::Info, AppLogSource::App, None, None, message);
    }

    fn log_job(
        &mut self,
        level: AppLogLevel,
        source: AppLogSource,
        job_id: JobId,
        message: impl Into<String>,
    ) {
        self.log_entry(level, source, Some(job_id), None, message);
    }

    fn log_target(
        &mut self,
        level: AppLogLevel,
        source: AppLogSource,
        job_id: JobId,
        target_id: TargetId,
        message: impl Into<String>,
    ) {
        self.log_entry(level, source, Some(job_id), Some(target_id), message);
    }

    fn log_entry(
        &mut self,
        level: AppLogLevel,
        source: AppLogSource,
        job_id: Option<JobId>,
        target_id: Option<TargetId>,
        message: impl Into<String>,
    ) {
        let seq = self.next_log_seq;
        self.next_log_seq += 1;
        self.logs.push(LogEntry {
            seq,
            level,
            source,
            job_id,
            target_id,
            message: message.into(),
        });
        if self.logs.len() > 200 {
            let overflow = self.logs.len() - 200;
            self.logs.drain(0..overflow);
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeKind {
    Global,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UiEvent {
    Key(KeyEvent),
    Worker(WorkerEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Effect {
    StartScan {
        job_id: JobId,
    },
    StartClean {
        job_id: JobId,
        plan: CleanupPlan,
        selected: Vec<TargetId>,
        plan_digest: String,
    },
    CancelJob {
        job_id: JobId,
    },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WorkerEvent {
    ScanStarted {
        job_id: JobId,
    },
    JobProgress {
        job_id: JobId,
        message: String,
    },
    ScanProgress {
        job_id: JobId,
        phase: ScanPhase,
        message: String,
        plan: Option<CleanupPlan>,
    },
    ScanFinished {
        job_id: JobId,
        plan: CleanupPlan,
    },
    CleanProgress {
        job_id: JobId,
        target_id: TargetId,
        status: ExecutionTargetStatus,
        message: String,
        detail: String,
        completed: usize,
        total: usize,
    },
    CleanFinished {
        job_id: JobId,
        report: ExecutionReport,
    },
    JobFailed {
        job_id: JobId,
        message: String,
    },
    #[allow(dead_code)] // Retained for the true-cancellation worker handoff.
    JobCanceled {
        job_id: JobId,
    },
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
}

impl ActiveTab {
    pub(super) const ALL: [Self; 5] = [
        Self::Dashboard,
        Self::Global,
        Self::Projects,
        Self::Rules,
        Self::JobsLogs,
    ];

    pub(super) fn title(self) -> &'static str {
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
pub(super) enum Overlay {
    None,
    Help,
    Details,
    DryRun,
    Confirm(ConfirmState),
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
pub(super) struct CommandPreview {
    pub(super) target: String,
    pub(super) command: String,
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
            ExecutionTargetStatus::Failed => Self::Failed,
            ExecutionTargetStatus::Skipped => Self::Skipped,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobKind {
    Scan,
    Clean,
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
    Clean,
    Audit,
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
        ExecutionTargetStatus::Failed => AppLogLevel::Error,
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
mod tests {
    use std::{path::PathBuf, time::SystemTime};

    use crossterm::event::KeyCode;

    use super::*;
    use crate::model::{CLEANUP_PLAN_VERSION, Ecosystem, Evidence, TargetKind};
    use crate::tui::test_support::{
        key, plan_with_targets, render_text, representative_plan, target,
    };

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
    fn startup_effects_request_an_initial_scan() {
        let mut app = App::new();

        let effects = app.startup_effects();

        let [Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("startup requests one scan");
        };
        assert_eq!(*job_id, 1);
        assert_eq!(app.jobs.len(), 1);
        assert_eq!(app.jobs[0].label, "Scan current directory and globals");
        assert_eq!(
            app.logs.last().map(|entry| entry.message.as_str()),
            Some("Startup scan requested")
        );
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
        assert_eq!(confirm.required_phrase, "confirm");
        assert!(confirm.message.contains("Trash"));
        assert!(!confirm.has_irreversible_commands);

        app.overlay = Overlay::None;
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[1].id.clone());

        app.update(key(KeyCode::Char('c')));
        let Overlay::Confirm(confirm) = &app.overlay else {
            panic!("command selection opens confirm");
        };
        assert_eq!(confirm.required_phrase, "confirm");
        assert!(confirm.message.contains("irreversible"));
        assert!(confirm.has_irreversible_commands);
        assert_eq!(confirm.command_previews.len(), 1);
        assert_eq!(
            confirm.command_previews[0].command,
            "argv: npm cache clean --force"
        );
    }

    #[test]
    fn inspect_only_targets_are_not_selectable_for_cleanup() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        app.selected_index = 2;

        app.update(key(KeyCode::Char(' ')));

        let inspect_id = app.targets[2].id.clone();
        assert!(!app.selected_ids.contains(&inspect_id));
        assert!(app.logs.iter().any(|entry| {
            entry.message == "Inspect-only targets cannot be selected for cleanup"
        }));

        app.selected_ids.insert(inspect_id);
        app.update(key(KeyCode::Char('c')));
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn startup_default_selection_follows_selected_by_default_even_for_self_targets() {
        let current_exe = std::env::current_exe().expect("current executable path");
        let current_exe_dir = current_exe.parent().expect("current executable has parent");
        let safe_path = PathBuf::from("D:/code/web/.next/cache");
        let plan = CleanupPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "rust.target",
                    Scope::Project {
                        root: current_exe_dir.to_path_buf(),
                    },
                    Ecosystem::Rust,
                    TargetKind::BuildArtifacts,
                    Some(current_exe_dir.to_path_buf()),
                    4096,
                    RiskLevel::Low,
                    true,
                    false,
                    CleanAction::Command {
                        program: "cargo".to_string(),
                        args: vec!["clean".to_string()],
                        cwd: None,
                        irreversible: true,
                    },
                ),
                target(
                    "node.next_cache",
                    Scope::Project {
                        root: PathBuf::from("D:/code/web"),
                    },
                    Ecosystem::Node,
                    TargetKind::BuildArtifacts,
                    Some(safe_path.clone()),
                    1024,
                    RiskLevel::Low,
                    true,
                    true,
                    CleanAction::MoveToTrash { path: safe_path },
                ),
            ],
        };

        let app = App::with_plan(plan);

        // The self-clean guard is enforced by the executor at run time; the UI
        // default selection is a pure projection of `selected_by_default`.
        assert!(
            app.targets
                .iter()
                .find(|target| target.id.as_str().starts_with("rust.target"))
                .is_some_and(|target| app.selected_ids.contains(&target.id))
        );
        assert!(
            app.targets
                .iter()
                .find(|target| target.id.as_str().starts_with("node.next_cache"))
                .is_some_and(|target| app.selected_ids.contains(&target.id))
        );
    }

    #[test]
    fn accepted_confirmation_emits_selected_clean_plan() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[0].id.clone());

        app.update(key(KeyCode::Char('c')));
        let confirmation_digest_prefix = match &app.overlay {
            Overlay::Confirm(confirm) => confirm.plan_digest_prefix.clone(),
            _ => panic!("confirmation opens with a digest"),
        };
        for ch in "confirm".chars() {
            app.update(key(KeyCode::Char(ch)));
        }
        let effects = app.update(key(KeyCode::Enter));

        let [
            Effect::StartClean {
                job_id,
                plan,
                selected,
                plan_digest,
            },
        ] = effects.as_slice()
        else {
            panic!("confirmation emits clean effect");
        };
        assert_eq!(*job_id, 1);
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.targets[0].id, app.targets[0].id);
        assert_eq!(selected, &vec![app.targets[0].id.clone()]);
        assert_eq!(
            plan_digest,
            crate::plan_validation::validate_scanned_plan(plan)
                .expect("frozen plan validates")
                .digest()
        );
        assert_eq!(
            confirmation_digest_prefix,
            plan_digest[..12],
            "the confirmation prefix belongs to the exact plan sent to the worker"
        );
        assert!(matches!(app.overlay, Overlay::None));
        let progress = app
            .cleanup_progress
            .as_ref()
            .expect("accepted confirmation initializes progress");
        assert_eq!(progress.job_id, *job_id);
        assert_eq!(progress.completed, 0);
        assert_eq!(progress.total, 1);
        assert_eq!(progress.summary, "Executing selected cleanup plan");
        assert!(!progress.finished);
        assert_eq!(progress.items.len(), 1);
        assert_eq!(progress.items[0].target_id, app.targets[0].id);
        assert_eq!(progress.items[0].status, CleanupItemStatus::Pending);
    }

    #[test]
    fn accepted_confirmation_keeps_explicit_fresh_target_selection() {
        let mut target = representative_plan().targets[0].clone();
        target.last_modified = Some(SystemTime::now());
        target.selected_by_default = false;
        target.evidence.push(Evidence::RuleMatched {
            rule_id: crate::ranking::FRESHNESS_GUARD_RULE_ID.to_string(),
        });
        let mut app = App::with_plan(plan_with_targets(vec![target.clone()]));
        app.selected_ids.insert(target.id.clone());

        app.update(key(KeyCode::Char('c')));
        for ch in "confirm".chars() {
            app.update(key(KeyCode::Char(ch)));
        }
        let effects = app.update(key(KeyCode::Enter));

        let [Effect::StartClean { plan, selected, .. }] = effects.as_slice() else {
            panic!("confirmation emits clean effect");
        };
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.targets[0].id, target.id);
        assert_eq!(selected, &vec![target.id.clone()]);
        assert!(
            !plan.targets[0].selected_by_default,
            "explicit selection must not rewrite the ranking hint"
        );
    }

    #[test]
    fn rejected_confirmation_shows_inline_feedback() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[1].id.clone());

        app.update(key(KeyCode::Char('c')));
        let effects = app.update(key(KeyCode::Enter));

        assert!(effects.is_empty());
        let Overlay::Confirm(confirm) = &app.overlay else {
            panic!("invalid confirmation keeps confirm overlay open");
        };
        assert_eq!(
            confirm.feedback.as_deref(),
            Some("Type confirm before pressing Enter.")
        );
        let rendered = render_text(&app);
        assert!(rendered.contains("Type confirm before pressing Enter."));
        assert!(rendered.contains("Input:"));
    }

    #[test]
    fn finished_cleanup_progress_stays_visible_until_dismissed() {
        let mut app = App::with_plan(representative_plan());
        let job_id = app.start_job(JobKind::Clean, "Clean fixture");
        let target_id = app.targets[1].id.clone();
        app.cleanup_progress = Some(CleanupProgress {
            job_id,
            completed: 0,
            total: 1,
            summary: "Executing selected cleanup plan".to_string(),
            finished: false,
            items: vec![CleanupProgressItem {
                target_id: target_id.clone(),
                label: "npm cache clean".to_string(),
                status: CleanupItemStatus::Pending,
                detail: None,
            }],
        });

        app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
            job_id,
            target_id,
            status: ExecutionTargetStatus::Succeeded,
            message: "npm cache clean".to_string(),
            detail: "completed".to_string(),
            completed: 0,
            total: 1,
        }));
        app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
            job_id,
            report: ExecutionReport {
                dry_run: false,
                selected: 1,
                attempted: 1,
                succeeded: 1,
                failed: 0,
                skipped: 0,
                failures: Vec::new(),
                audit_log: Some(PathBuf::from("audit.jsonl")),
            },
        }));

        let progress = app
            .cleanup_progress
            .as_ref()
            .expect("finished cleanup result remains visible");
        assert!(progress.finished);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.total, 1);
        let rendered = render_text(&app);
        assert!(rendered.contains("Cleanup progress"));
        assert!(rendered.contains("1 / 1 finished: 1 succeeded, 0 failed, 0 skipped"));
        assert!(rendered.contains("Enter/Esc close"));

        app.update(key(KeyCode::Enter));

        assert!(app.cleanup_progress.is_none());
        assert!(!app.should_quit);
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
        assert!(
            app.targets
                .iter()
                .find(|target| target.id.as_str().starts_with("node.next_cache"))
                .is_some_and(|target| app.selected_ids.contains(&target.id))
        );

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
        let clean_job_id = app.start_job(JobKind::Clean, "Clean fixture");
        app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
            job_id: clean_job_id,
            report,
        }));
        assert!(
            app.logs
                .iter()
                .any(|entry| entry.message.contains("audit.jsonl"))
        );
    }

    #[test]
    fn scan_progress_shows_project_targets_before_global_scan_finishes() {
        let mut app = App::new();
        let effects = app.startup_effects();
        let [Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("startup requests scan");
        };
        let job_id = *job_id;
        let project_plan = plan_with_targets(vec![representative_plan().targets[0].clone()]);

        app.update(UiEvent::Worker(WorkerEvent::ScanStarted { job_id }));
        app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
            job_id,
            phase: ScanPhase::Projects,
            message: "Project scan finished: 1 target(s)".to_string(),
            plan: Some(project_plan),
        }));

        assert_eq!(app.targets.len(), 1);
        assert!(matches!(app.targets[0].scope, Scope::Project { .. }));
        assert!(app.selected_ids.contains(&app.targets[0].id));
        assert_eq!(app.jobs[0].status, JobStatus::Running);
        assert_eq!(app.jobs[0].progress, "Project scan finished: 1 target(s)");

        app.active_tab = ActiveTab::JobsLogs;
        let rendered = render_text(&app);
        assert!(rendered.contains("Project scan finished: 1 target(s)"));
    }

    #[test]
    fn scan_progress_applies_latest_cumulative_partial_plan() {
        let mut app = App::new();
        let effects = app.startup_effects();
        let [Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("startup requests scan");
        };
        let job_id = *job_id;
        let representative = representative_plan();
        let project_target = representative.targets[0].clone();
        let global_target = representative.targets[1].clone();

        app.update(UiEvent::Worker(WorkerEvent::ScanStarted { job_id }));
        app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
            job_id,
            phase: ScanPhase::Projects,
            message: "Project scan finished: 1 target(s)".to_string(),
            plan: Some(plan_with_targets(vec![project_target.clone()])),
        }));
        app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
            job_id,
            phase: ScanPhase::Global,
            message: "Global scan finished: 1 target(s)".to_string(),
            plan: Some(plan_with_targets(vec![
                global_target.clone(),
                project_target.clone(),
            ])),
        }));
        app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
            job_id,
            phase: ScanPhase::Global,
            message: "Global scan finished: 1 target(s)".to_string(),
            plan: Some(plan_with_targets(vec![
                global_target.clone(),
                project_target.clone(),
            ])),
        }));

        assert_eq!(
            app.targets
                .iter()
                .map(|target| &target.id)
                .collect::<Vec<_>>(),
            vec![&global_target.id, &project_target.id]
        );
        assert_eq!(app.targets.len(), 2);
        assert!(app.selected_ids.contains(&project_target.id));
        assert!(!app.selected_ids.contains(&global_target.id));
    }

    #[test]
    fn stale_scan_updates_do_not_overwrite_newer_scan_results() {
        let mut app = App::new();
        let startup_effects = app.startup_effects();
        let [Effect::StartScan { job_id: first_job }] = startup_effects.as_slice() else {
            panic!("startup requests scan");
        };
        let first_job = *first_job;
        app.update(UiEvent::Worker(WorkerEvent::ScanStarted {
            job_id: first_job,
        }));

        let manual_effects = app.update(key(KeyCode::Char('s')));
        let [Effect::StartScan { job_id: second_job }] = manual_effects.as_slice() else {
            panic!("manual scan starts second job");
        };
        let second_job = *second_job;
        let representative = representative_plan();
        let second_target = representative.targets[0].clone();
        let stale_target = representative.targets[1].clone();

        app.update(UiEvent::Worker(WorkerEvent::ScanStarted {
            job_id: second_job,
        }));
        app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
            job_id: second_job,
            phase: ScanPhase::Projects,
            message: "Project scan finished: 1 target(s)".to_string(),
            plan: Some(plan_with_targets(vec![second_target.clone()])),
        }));
        app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
            job_id: first_job,
            plan: plan_with_targets(vec![stale_target]),
        }));

        assert_eq!(app.targets.len(), 1);
        assert_eq!(app.targets[0].id, second_target.id);
        assert_eq!(app.jobs[0].status, JobStatus::Succeeded);
        assert_eq!(app.jobs[1].status, JobStatus::Running);
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
        assert_eq!(
            app.jobs[0].progress,
            "Cancellation requested; current action cannot be interrupted."
        );

        app.update(UiEvent::Worker(WorkerEvent::JobCanceled { job_id }));
        assert_eq!(app.jobs[0].status, JobStatus::Canceled);
    }

    #[test]
    fn scan_updates_invalidate_open_confirmation_and_block_enter() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        let selected_target = app.targets[0].clone();
        app.selected_ids.insert(selected_target.id.clone());
        app.update(key(KeyCode::Char('c')));

        let effects = app.startup_effects();
        let [Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("startup requests a scan");
        };
        let job_id = *job_id;
        let updated_plan = plan_with_targets(vec![app.targets[1].clone()]);

        app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
            job_id,
            phase: ScanPhase::Projects,
            message: "Scan found an updated target".to_string(),
            plan: Some(updated_plan.clone()),
        }));
        app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
            job_id,
            plan: updated_plan,
        }));
        for ch in "confirm".chars() {
            app.update(key(KeyCode::Char(ch)));
        }

        let effects = app.update(key(KeyCode::Enter));

        assert!(effects.is_empty());
        let Overlay::Confirm(confirm) = &app.overlay else {
            panic!("invalidated confirmation remains visible");
        };
        assert!(confirm.invalidated_by_scan);
        assert_eq!(
            confirm.feedback.as_deref(),
            Some("Scan results changed. Close this dialog and confirm the updated selection.")
        );
        assert!(render_text(&app).contains("This confirmation is disabled."));
        assert!(app.logs.iter().any(|entry| {
            entry
                .message
                .contains("Confirmation rejected because scan results changed")
        }));
    }

    #[test]
    fn accepted_confirmation_executes_its_frozen_manifest() {
        let mut app = App::with_plan(representative_plan());
        app.selected_ids.clear();
        let frozen_target = app.targets[0].clone();
        app.selected_ids.insert(frozen_target.id.clone());
        app.update(key(KeyCode::Char('c')));

        app.selected_ids.clear();
        app.selected_ids.insert(app.targets[1].id.clone());
        for ch in "confirm".chars() {
            app.update(key(KeyCode::Char(ch)));
        }

        let effects = app.update(key(KeyCode::Enter));

        let [Effect::StartClean { plan, selected, .. }] = effects.as_slice() else {
            panic!("confirmation emits one clean effect");
        };
        assert_eq!(plan.targets, vec![frozen_target.clone()]);
        assert_eq!(selected, &vec![frozen_target.id]);
    }

    #[test]
    fn staged_scan_preserves_explicit_selection_and_defaults_new_targets() {
        let original_target = representative_plan().targets[0].clone();
        let mut new_target = representative_plan().targets[1].clone();
        new_target.selected_by_default = true;
        let mut app = App::with_plan(plan_with_targets(vec![original_target.clone()]));

        app.update(key(KeyCode::Char(' ')));
        assert!(!app.selected_ids.contains(&original_target.id));

        let effects = app.startup_effects();
        let [Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("startup requests a scan");
        };
        let job_id = *job_id;
        let updated_plan = plan_with_targets(vec![original_target.clone(), new_target.clone()]);

        app.update(UiEvent::Worker(WorkerEvent::ScanProgress {
            job_id,
            phase: ScanPhase::Projects,
            message: "Project scan updated".to_string(),
            plan: Some(updated_plan.clone()),
        }));
        app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
            job_id,
            plan: updated_plan,
        }));

        assert!(!app.selected_ids.contains(&original_target.id));
        assert!(app.selected_ids.contains(&new_target.id));

        let effects = app.startup_effects();
        let [Effect::StartScan { job_id }] = effects.as_slice() else {
            panic!("follow-up scan starts");
        };
        app.update(UiEvent::Worker(WorkerEvent::ScanFinished {
            job_id: *job_id,
            plan: plan_with_targets(vec![new_target]),
        }));
        assert!(!app.selection_overrides.contains_key(&original_target.id));
    }

    #[test]
    fn active_cleanup_rejects_new_scan_and_cleanup_requests() {
        let mut app = App::with_plan(representative_plan());
        let job_id = app.start_job(JobKind::Clean, "Clean fixture");

        assert!(app.update(key(KeyCode::Char('s'))).is_empty());
        assert!(app.update(key(KeyCode::Char('c'))).is_empty());

        assert_eq!(app.jobs.len(), 1);
        assert_eq!(app.jobs[0].id, job_id);
        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.logs.iter().any(|entry| {
            entry
                .message
                .contains("scan requests are disabled until it finishes")
        }));
        assert!(
            app.logs
                .iter()
                .any(|entry| { entry.message.contains("a second cleanup cannot start") })
        );
    }

    #[test]
    fn cancelling_job_ignores_late_progress_events() {
        let mut app = App::with_plan(representative_plan());
        let job_id = app.start_job(JobKind::Clean, "Clean fixture");
        let target_id = app.targets[0].id.clone();
        let plan = plan_with_targets(vec![app.targets[0].clone()]);
        app.cleanup_progress = Some(cleanup_progress_for_plan(job_id, &plan));

        app.update(key(KeyCode::Char('x')));
        let cancelling_progress = app.jobs[0].progress.clone();
        app.update(UiEvent::Worker(WorkerEvent::JobProgress {
            job_id,
            message: "Late worker progress".to_string(),
        }));
        app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
            job_id,
            target_id,
            status: ExecutionTargetStatus::Succeeded,
            message: "Late cleanup progress".to_string(),
            detail: "completed".to_string(),
            completed: 1,
            total: 1,
        }));

        assert_eq!(app.jobs[0].status, JobStatus::Cancelling);
        assert_eq!(app.jobs[0].progress, cancelling_progress);
        assert_eq!(
            app.cleanup_progress
                .as_ref()
                .map(|progress| progress.completed),
            Some(0)
        );
    }

    #[test]
    fn terminal_jobs_ignore_late_worker_events() {
        let mut succeeded = App::with_plan(representative_plan());
        let succeeded_job = succeeded.start_job(JobKind::Clean, "Clean fixture");
        succeeded.update(UiEvent::Worker(WorkerEvent::CleanFinished {
            job_id: succeeded_job,
            report: ExecutionReport {
                dry_run: false,
                selected: 1,
                attempted: 1,
                succeeded: 1,
                failed: 0,
                skipped: 0,
                failures: Vec::new(),
                audit_log: None,
            },
        }));
        assert_terminal_job_ignores_late_events(
            &mut succeeded,
            succeeded_job,
            JobStatus::Succeeded,
        );

        let mut failed = App::with_plan(representative_plan());
        let failed_job = failed.start_job(JobKind::Clean, "Clean fixture");
        failed.update(UiEvent::Worker(WorkerEvent::JobFailed {
            job_id: failed_job,
            message: "Cleanup failed".to_string(),
        }));
        assert_terminal_job_ignores_late_events(&mut failed, failed_job, JobStatus::Failed);

        let mut canceled = App::with_plan(representative_plan());
        let canceled_job = canceled.start_job(JobKind::Clean, "Clean fixture");
        canceled.update(key(KeyCode::Char('x')));
        canceled.update(UiEvent::Worker(WorkerEvent::JobCanceled {
            job_id: canceled_job,
        }));
        assert_terminal_job_ignores_late_events(&mut canceled, canceled_job, JobStatus::Canceled);
    }

    #[test]
    fn byte_summaries_count_duplicate_paths_once() {
        let cache_path = PathBuf::from("C:/Users/me/AppData/Local/npm-cache");
        let plan = CleanupPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![
                target(
                    "npm.cache.verify",
                    Scope::Global,
                    Ecosystem::Node,
                    TargetKind::PackageCache,
                    Some(cache_path.clone()),
                    2048,
                    RiskLevel::Low,
                    true,
                    false,
                    CleanAction::Command {
                        program: "npm".to_string(),
                        args: vec!["cache".to_string(), "verify".to_string()],
                        cwd: None,
                        irreversible: true,
                    },
                ),
                target(
                    "npm.cache.clean",
                    Scope::Global,
                    Ecosystem::Node,
                    TargetKind::PackageCache,
                    Some(cache_path),
                    2048,
                    RiskLevel::Medium,
                    true,
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
            ],
        };
        let app = App::with_plan(plan);

        assert_eq!(app.scope_bytes(ScopeKind::Global), 2048);
        assert_eq!(app.selected_bytes(), 2048);
        let mut app = app;
        app.selected_ids.remove(&app.targets[0].id);
        assert_eq!(
            app.confirm_state()
                .expect("selected cleanup target validates")
                .estimated_bytes,
            2048
        );
    }

    fn assert_terminal_job_ignores_late_events(
        app: &mut App,
        job_id: JobId,
        expected_status: JobStatus,
    ) {
        let target_id = app.targets[0].id.clone();
        let targets = app.targets.clone();
        let progress = app.jobs[0].progress.clone();
        let cleanup_progress = app.cleanup_progress.clone();

        app.update(UiEvent::Worker(WorkerEvent::JobProgress {
            job_id,
            message: "Late worker progress".to_string(),
        }));
        app.update(UiEvent::Worker(WorkerEvent::CleanProgress {
            job_id,
            target_id,
            status: ExecutionTargetStatus::Succeeded,
            message: "Late cleanup progress".to_string(),
            detail: "completed".to_string(),
            completed: 1,
            total: 1,
        }));
        app.update(UiEvent::Worker(WorkerEvent::CleanFinished {
            job_id,
            report: ExecutionReport {
                dry_run: false,
                selected: 1,
                attempted: 1,
                succeeded: 1,
                failed: 0,
                skipped: 0,
                failures: Vec::new(),
                audit_log: None,
            },
        }));
        app.update(UiEvent::Worker(WorkerEvent::JobCanceled { job_id }));

        assert_eq!(app.jobs[0].status, expected_status);
        assert_eq!(app.jobs[0].progress, progress);
        assert_eq!(app.cleanup_progress, cleanup_progress);
        assert_eq!(app.targets, targets);
    }
}
