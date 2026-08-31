use super::*;

impl App {
    pub(super) fn cancel_active_job(&mut self) -> Vec<Effect> {
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

    pub(in crate::tui) fn start_job(&mut self, kind: JobKind, label: impl Into<String>) -> JobId {
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

    pub(super) fn ensure_job(&mut self, job_id: JobId, kind: JobKind, label: impl Into<String>) {
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

    pub(super) fn transition_job(
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

    pub(super) fn has_active_clean_job(&self) -> bool {
        self.jobs
            .iter()
            .any(|job| job.kind == JobKind::Clean && job.status.is_active())
    }

    pub(super) fn has_unpromoted_scan_preview(&self) -> bool {
        self.scan_snapshot.is_some()
    }

    pub(super) fn has_active_inventory_job(&self) -> bool {
        self.jobs
            .iter()
            .any(|job| job.kind == JobKind::Inventory && job.status.is_active())
    }

    pub(super) fn maybe_start_pending_scan(&mut self) -> Vec<Effect> {
        if !self.pending_scan_restart
            || self.has_active_mutation_job()
            || self.pending_mode.is_some()
        {
            return Vec::new();
        }
        self.pending_scan_restart = false;
        let job_id = self.start_scan_job("Replacement scan started after prior work joined");
        vec![Effect::StartScan { job_id }]
    }

    pub(super) fn maybe_finish_pending_mode(&mut self) -> Vec<Effect> {
        let Some(mode) = self.pending_mode else {
            return Vec::new();
        };
        if self.has_active_mutation_job() {
            return Vec::new();
        }
        self.pending_mode = None;
        if self.shell.active == ModeId::Analyze && mode != ModeId::Analyze {
            self.analyze.reduce(AnalyzeAction::Release);
        }
        if self.shell.active == ModeId::Software && mode != ModeId::Software {
            self.software.reduce(SoftwareAction::Released);
        }
        let _ = self.shell.activate(mode);
        self.overlay = Overlay::None;
        self.filter_active = false;
        Vec::new()
    }

    pub(super) fn log_ignored_worker_event(&mut self, job_id: JobId, event: &str) {
        self.log_job(
            AppLogLevel::Warning,
            self.job_source(job_id),
            job_id,
            format!("Ignored delayed {event} event"),
        );
    }

    pub(super) fn job_source(&self, job_id: JobId) -> AppLogSource {
        self.jobs
            .iter()
            .find(|job| job.id == job_id)
            .map(|job| match job.kind {
                JobKind::Scan => AppLogSource::Scan,
                JobKind::Clean => AppLogSource::Clean,
                JobKind::Inventory => AppLogSource::Inventory,
                JobKind::Analyze => AppLogSource::Analyze,
                JobKind::Software => AppLogSource::Software,
            })
            .unwrap_or(AppLogSource::App)
    }

    pub(super) fn cleanup_progress_matches(&self, job_id: JobId) -> bool {
        self.cleanup_progress
            .as_ref()
            .is_some_and(|progress| progress.job_id == job_id)
    }

    pub(super) fn update_cleanup_progress_item(&mut self, update: CleanupProgressUpdate) {
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

    pub(super) fn finish_cleanup_progress(
        &mut self,
        job_id: JobId,
        report: &ExecutionReport,
        summary: &str,
    ) {
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

    pub(super) fn confirm_state(&self) -> Result<ConfirmState, String> {
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

    pub(super) fn log(&mut self, message: impl Into<String>) {
        self.log_entry(AppLogLevel::Info, AppLogSource::App, None, None, message);
    }

    pub(super) fn log_job(
        &mut self,
        level: AppLogLevel,
        source: AppLogSource,
        job_id: JobId,
        message: impl Into<String>,
    ) {
        self.log_entry(level, source, Some(job_id), None, message);
    }

    pub(super) fn log_target(
        &mut self,
        level: AppLogLevel,
        source: AppLogSource,
        job_id: JobId,
        target_id: TargetId,
        message: impl Into<String>,
    ) {
        self.log_entry(level, source, Some(job_id), Some(target_id), message);
    }

    pub(super) fn log_entry(
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
