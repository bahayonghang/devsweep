use super::*;

impl App {
    pub(super) fn handle_worker_event(&mut self, event: WorkerEvent) -> Vec<Effect> {
        match event {
            WorkerEvent::ScanStarted { job_id } => {
                self.ensure_job(job_id, JobKind::Scan, "Scan current directory and globals");
                if !self.transition_job(job_id, JobStatus::Running, "Started") {
                    self.log_ignored_worker_event(job_id, "scan started");
                }
            }
            WorkerEvent::InventoryStarted { job_id } => {
                self.ensure_job(job_id, JobKind::Inventory, "Inventory current directory");
                if !self.transition_job(job_id, JobStatus::Running, "Started") {
                    self.log_ignored_worker_event(job_id, "inventory started");
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
                if status == ExecutionTargetStatus::Succeeded {
                    self.mark_target_cleaned(target_id.clone());
                }
                self.log_target(
                    log_level_for_execution_status(status),
                    AppLogSource::Clean,
                    job_id,
                    target_id,
                    format_cleanup_progress(completed, total, &message),
                );
            }
            WorkerEvent::ScanFinished {
                job_id,
                plan,
                health,
            } => {
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
                    self.cleaned_ids.clear();
                    self.replace_targets_preserving_selection(plan.targets);
                    self.scan_health = health.clone();
                    self.scan_snapshot = None;
                }
                self.log_job(
                    if health.is_complete() {
                        AppLogLevel::Info
                    } else {
                        AppLogLevel::Warning
                    },
                    AppLogSource::Scan,
                    job_id,
                    format!(
                        "Scan finished: {count} target(s), {} health, {} diagnostic(s)",
                        health.completeness.label(),
                        health.diagnostics.len()
                    ),
                );
            }
            WorkerEvent::InventoryFinished { job_id, report } => {
                let report = *report;
                let count = report.observations.len();
                let should_apply_inventory_update = self.should_apply_inventory_update(job_id);
                if !self.transition_job(
                    job_id,
                    JobStatus::Succeeded,
                    format!("Observed {count} path(s)"),
                ) {
                    self.log_ignored_worker_event(job_id, "inventory finished");
                    return Vec::new();
                }
                if should_apply_inventory_update {
                    self.inventory_report = Some(report.clone());
                    self.clamp_inventory_selection();
                }
                self.log_job(
                    if report.health.is_complete() {
                        AppLogLevel::Info
                    } else {
                        AppLogLevel::Warning
                    },
                    AppLogSource::Inventory,
                    job_id,
                    format!(
                        "Inventory finished: {count} observation(s), {} health, {} diagnostic(s)",
                        report.health.completeness.label(),
                        report.health.diagnostics.len()
                    ),
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

    pub(super) fn set_tab(&mut self, tab: ActiveTab) -> Vec<Effect> {
        self.active_tab = tab;
        if tab == ActiveTab::Inventory {
            self.inventory_selected_index = 0;
            self.inventory_list_scroll = 0;
            if self.inventory_report.is_none() {
                return self.request_inventory("Inventory requested");
            }
        } else {
            self.selected_index = 0;
        }
        Vec::new()
    }

    pub(super) fn start_scan_job(&mut self, log_message: impl Into<String>) -> JobId {
        let job_id = self.start_job(JobKind::Scan, "Scan current directory and globals");
        self.scan_snapshot = Some(ScanSnapshot::new(job_id));
        self.selected_ids.clear();
        self.selection_overrides.clear();
        self.log_job(AppLogLevel::Info, AppLogSource::Scan, job_id, log_message);
        job_id
    }

    pub(super) fn request_inventory(&mut self, log_message: impl Into<String>) -> Vec<Effect> {
        if self.has_active_clean_job() {
            self.log_entry(
                AppLogLevel::Warning,
                AppLogSource::Inventory,
                None,
                None,
                "Cleanup is active; inventory requests are disabled until it finishes",
            );
            return Vec::new();
        }
        if self.has_active_inventory_job() {
            self.log_entry(
                AppLogLevel::Info,
                AppLogSource::Inventory,
                None,
                None,
                "Inventory is already running",
            );
            return Vec::new();
        }

        let job_id = self.start_inventory_job(log_message);
        vec![Effect::StartInventory { job_id }]
    }

    pub(super) fn start_inventory_job(&mut self, log_message: impl Into<String>) -> JobId {
        let job_id = self.start_job(JobKind::Inventory, "Inventory current directory");
        self.log_job(
            AppLogLevel::Info,
            AppLogSource::Inventory,
            job_id,
            log_message,
        );
        job_id
    }

    pub(super) fn handle_scan_progress(
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

        let mut staged_health = complete_health_for_plan(&plan);
        staged_health.mark_partial();
        let mut updated = false;
        if let Some(snapshot) = &mut self.scan_snapshot
            && snapshot.job_id == job_id
        {
            snapshot.set_targets(plan.targets);
            updated = true;
        }

        if updated {
            self.scan_health = staged_health;
            self.rebuild_targets_from_scan_snapshot();
        }
    }

    pub(super) fn should_apply_scan_update(&self, job_id: JobId) -> bool {
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

    pub(super) fn should_apply_inventory_update(&self, job_id: JobId) -> bool {
        let Some(job) = self.jobs.iter().find(|job| job.id == job_id) else {
            return false;
        };
        if job.kind != JobKind::Inventory || !job.status.is_active() {
            return false;
        }
        self.jobs
            .iter()
            .filter(|job| job.kind == JobKind::Inventory)
            .map(|job| job.id)
            .max()
            == Some(job_id)
    }

    pub(super) fn rebuild_targets_from_scan_snapshot(&mut self) {
        if let Some(snapshot) = &self.scan_snapshot {
            self.targets = snapshot.targets();
            self.selected_ids.clear();
            self.collapsed_pycache_projects = pycache_project_roots(&self.targets);
            self.selected_index = 0;
            self.list_scroll = 0;
        }
    }

    pub(super) fn replace_targets_preserving_selection(&mut self, targets: Vec<CleanTarget>) {
        let available_ids: HashSet<TargetId> =
            targets.iter().map(|target| target.id.clone()).collect();
        self.selection_overrides
            .retain(|target_id, _| available_ids.contains(target_id));
        self.selected_ids = targets
            .iter()
            .filter_map(|target| {
                if !target.action.is_executable() || self.cleaned_ids.contains(&target.id) {
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
        self.collapsed_pycache_projects = pycache_project_roots(&self.targets);
        self.selected_index = 0;
        self.list_scroll = 0;
    }

    pub(super) fn invalidate_confirmation_due_to_scan(&mut self) {
        if let Overlay::Confirm(confirm) = &mut self.overlay {
            confirm.invalidated_by_scan = true;
            confirm.feedback = Some(
                "Scan results changed. Close this dialog and confirm the updated selection."
                    .to_string(),
            );
        }
    }
}
