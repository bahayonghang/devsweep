use super::*;

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        if key.kind == KeyEventKind::Release {
            return Vec::new();
        }

        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'C'))
        {
            return self.request_quit();
        }

        if key.modifiers.contains(KeyModifiers::ALT)
            && let KeyCode::Char(ch) = key.code
            && let Some(mode) = self
                .shell
                .navigation
                .iter()
                .find(|item| item.accelerator == Some(ch.to_ascii_lowercase()))
                .map(|item| item.id)
        {
            return self.request_mode(mode);
        }

        if self.language_settings.open {
            return self.handle_language_settings_key(key);
        }

        if self.filter_active {
            return self.handle_filter_key(key);
        }

        if self.shell.active == ModeId::Analyze {
            return self.handle_analyze_key(key);
        }
        if self.shell.active == ModeId::Software {
            return self.handle_software_key(key);
        }
        if self.shell.active == ModeId::Optimize {
            return self.handle_optimize_key(key);
        }
        if self.shell.active == ModeId::Status {
            return self.handle_status_key(key);
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
            Overlay::QuitConfirm => self.handle_quit_confirm_key(key),
            Overlay::Help | Overlay::Details | Overlay::DryRun => self.handle_overlay_key(key),
            Overlay::None => self.handle_normal_key(key),
        }
    }

    pub(super) fn handle_normal_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Char('q') => self.request_quit(),
            KeyCode::Esc => {
                if self.has_active_mutation_job() {
                    self.request_quit()
                } else {
                    self.should_quit = true;
                    vec![Effect::Quit]
                }
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
                if self.pending_scan_restart {
                    self.log_entry(
                        AppLogLevel::Info,
                        AppLogSource::Scan,
                        None,
                        None,
                        "Replacement scan is already waiting for prior work to join",
                    );
                    return Vec::new();
                }
                let active = self
                    .jobs
                    .iter()
                    .filter(|job| job.status.is_active())
                    .map(|job| job.id)
                    .collect::<Vec<_>>();
                if !active.is_empty() {
                    self.pending_scan_restart = true;
                    let mut effects = Vec::new();
                    for job_id in active {
                        if self.transition_job(
                            job_id,
                            JobStatus::Cancelling,
                            "Cancellation requested; waiting to join before replacement scan.",
                        ) {
                            effects.push(Effect::CancelJob { job_id });
                        }
                    }
                    self.log_entry(
                        AppLogLevel::Info,
                        AppLogSource::Scan,
                        None,
                        None,
                        "Replacement scan queued after cancel and join",
                    );
                    return effects;
                }
                let job_id = self.start_scan_job("Scan requested");
                vec![Effect::StartScan { job_id }]
            }
            KeyCode::Char('c') => {
                if self.active_tab == ActiveTab::Inventory {
                    return Vec::new();
                }
                if self.has_unpromoted_scan_preview() {
                    self.log_entry(
                        AppLogLevel::Info,
                        AppLogSource::Scan,
                        None,
                        None,
                        "Scan previews are read-only until the scan finishes",
                    );
                } else if self.has_active_clean_job() {
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
                if self.active_tab == ActiveTab::Inventory {
                    return Vec::new();
                }
                if self.has_unpromoted_scan_preview() {
                    self.log_entry(
                        AppLogLevel::Info,
                        AppLogSource::Scan,
                        None,
                        None,
                        "Dry run is unavailable until the scan finishes",
                    );
                    return Vec::new();
                }
                self.overlay = Overlay::DryRun;
                Vec::new()
            }
            KeyCode::Char('?') => {
                self.overlay = Overlay::Help;
                Vec::new()
            }
            KeyCode::Char('p') => {
                self.language_settings.open = true;
                self.language_settings.selected = self.shell.locale;
                self.language_settings.pending_request = None;
                self.language_settings.failure = None;
                Vec::new()
            }
            KeyCode::Char('/') => {
                if self.active_tab == ActiveTab::Inventory {
                    return Vec::new();
                }
                self.filter_active = true;
                Vec::new()
            }
            KeyCode::Char('r') => {
                if self.active_tab == ActiveTab::Inventory {
                    return Vec::new();
                }
                self.cycle_risk_filter();
                self.selected_index = 0;
                Vec::new()
            }
            KeyCode::Char('a') => {
                if self.active_tab == ActiveTab::Inventory {
                    return Vec::new();
                }
                if self.has_unpromoted_scan_preview() {
                    return Vec::new();
                }
                self.toggle_visible_selection();
                Vec::new()
            }
            KeyCode::Char('i') => {
                self.active_tab = ActiveTab::Inventory;
                self.inventory_selected_index = 0;
                self.inventory_list_scroll = 0;
                self.request_inventory("Inventory refresh requested")
            }
            KeyCode::Char('l') => self.set_tab(ActiveTab::JobsLogs),
            KeyCode::Char('x') => self.cancel_active_job(),
            KeyCode::Char('1') => self.set_tab(ActiveTab::Dashboard),
            KeyCode::Char('2') => self.set_tab(ActiveTab::Global),
            KeyCode::Char('3') => self.set_tab(ActiveTab::Projects),
            KeyCode::Char('4') => self.set_tab(ActiveTab::Rules),
            KeyCode::Char('5') => self.set_tab(ActiveTab::JobsLogs),
            KeyCode::Char('6') => self.set_tab(ActiveTab::Inventory),
            KeyCode::Tab | KeyCode::Right => self.set_tab(self.active_tab.next()),
            KeyCode::BackTab | KeyCode::Left => self.set_tab(self.active_tab.previous()),
            KeyCode::Down | KeyCode::Char('j') => {
                if self.active_tab == ActiveTab::Inventory {
                    self.move_inventory_selection(1);
                } else {
                    self.move_selection(1);
                }
                Vec::new()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.active_tab == ActiveTab::Inventory {
                    self.move_inventory_selection(-1);
                } else {
                    self.move_selection(-1);
                }
                Vec::new()
            }
            KeyCode::PageDown => {
                if self.active_tab == ActiveTab::Inventory {
                    self.move_inventory_selection(self.viewport_page_size() as isize);
                } else {
                    self.page_selection(1);
                }
                Vec::new()
            }
            KeyCode::PageUp => {
                if self.active_tab == ActiveTab::Inventory {
                    self.move_inventory_selection(-(self.viewport_page_size() as isize));
                } else {
                    self.page_selection(-1);
                }
                Vec::new()
            }
            KeyCode::Enter => {
                if self.active_tab != ActiveTab::Inventory {
                    match self.selected_target_row() {
                        Some(TargetListRow::Target(_)) => self.overlay = Overlay::Details,
                        Some(TargetListRow::PycacheGroup(group)) => {
                            self.toggle_pycache_project(&group.project_root);
                        }
                        None => {}
                    }
                }
                Vec::new()
            }
            KeyCode::Char('g') => {
                if self.active_tab != ActiveTab::Inventory && !self.has_unpromoted_scan_preview() {
                    self.toggle_selected_pycache_project();
                }
                Vec::new()
            }
            KeyCode::Char(' ') => {
                if self.active_tab != ActiveTab::Inventory && !self.has_unpromoted_scan_preview() {
                    self.toggle_selected_target();
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn request_mode(&mut self, mode: ModeId) -> Vec<Effect> {
        if self.shell.active == mode || self.pending_mode == Some(mode) {
            return Vec::new();
        }
        self.pending_mode = Some(mode);
        if let Some(operation_id) = self.analyze.operation_id
            && self.shell.active == ModeId::Analyze
        {
            self.analyze
                .reduce(AnalyzeAction::CancelRequested { operation_id });
        }
        if let Some(operation_id) = self.software.operation_id
            && self.shell.active == ModeId::Software
        {
            self.software
                .reduce(SoftwareAction::CancelRequested(operation_id));
        }
        if let Some(operation_id) = self.optimize.operation_id
            && self.shell.active == ModeId::Optimize
        {
            self.optimize
                .reduce(OptimizeAction::CancelRequested(operation_id));
        }
        if let Some(operation_id) = self.status.operation_id
            && self.shell.active == ModeId::Status
        {
            self.status
                .reduce(crate::tui::modes::status::StatusAction::CancelRequested(
                    operation_id,
                ));
        }
        let effects = self.cancel_all_active_jobs();
        if effects.is_empty() {
            return self.maybe_finish_pending_mode();
        }
        effects
    }

    pub(super) fn handle_analyze_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.request_quit(),
            KeyCode::Char('s') => self.start_analyze_job(),
            KeyCode::Char('x') => {
                if let Some(operation_id) = self.analyze.operation_id {
                    self.analyze
                        .reduce(AnalyzeAction::CancelRequested { operation_id });
                }
                self.cancel_active_job()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.analyze.reduce(AnalyzeAction::MoveCursor(1));
                Vec::new()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.analyze.reduce(AnalyzeAction::MoveCursor(-1));
                Vec::new()
            }
            KeyCode::PageDown | KeyCode::Char(']') => {
                self.analyze.reduce(AnalyzeAction::PageChanged(1));
                Vec::new()
            }
            KeyCode::PageUp | KeyCode::Char('[') => {
                self.analyze.reduce(AnalyzeAction::PageChanged(-1));
                Vec::new()
            }
            KeyCode::Enter | KeyCode::Right => {
                self.analyze.reduce(AnalyzeAction::OpenFocused);
                Vec::new()
            }
            KeyCode::Backspace | KeyCode::Left => {
                self.analyze.reduce(AnalyzeAction::Up);
                Vec::new()
            }
            KeyCode::Char('/') => {
                self.filter_active = true;
                Vec::new()
            }
            KeyCode::Char('o') => {
                let next = match self.analyze.sort {
                    AnalyzeSort::SizeDescending => AnalyzeSort::SizeAscending,
                    AnalyzeSort::SizeAscending => AnalyzeSort::Name,
                    AnalyzeSort::Name => AnalyzeSort::Kind,
                    AnalyzeSort::Kind => AnalyzeSort::SizeDescending,
                };
                self.analyze.reduce(AnalyzeAction::SortChanged(next));
                Vec::new()
            }
            KeyCode::Char('p') => {
                self.language_settings.open = true;
                self.language_settings.selected = self.shell.locale;
                self.language_settings.failure = None;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn handle_software_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Char('q') => self.request_quit(),
            KeyCode::Esc => {
                if self.software.phase == SoftwarePhase::Confirming {
                    self.software.reduce(SoftwareAction::CloseConfirmation);
                    Vec::new()
                } else {
                    self.request_quit()
                }
            }
            KeyCode::Char('i') => {
                if self.has_active_mutation_job() || self.software.operation_id.is_some() {
                    return Vec::new();
                }
                let job_id = self.start_job(JobKind::Software, "Inventory installed software");
                self.software
                    .reduce(SoftwareAction::InventoryStarted(job_id));
                vec![Effect::StartSoftwareInventory { job_id }]
            }
            KeyCode::Char('v') => {
                let Some(inventory) = self.software.inventory.clone() else {
                    return Vec::new();
                };
                if self.software.selected_ids.is_empty() || self.software.operation_id.is_some() {
                    return Vec::new();
                }
                let selected_ids = self.software.selected_ids.iter().cloned().collect();
                let job_id = self.start_job(JobKind::Software, "Revalidate Software preview");
                self.software.reduce(SoftwareAction::PreviewStarted(job_id));
                vec![Effect::StartSoftwarePreview {
                    job_id,
                    inventory,
                    selected_ids,
                }]
            }
            KeyCode::Char('r') => {
                if self.has_active_mutation_job() || self.software.operation_id.is_some() {
                    return Vec::new();
                }
                let job_id = self.start_job(JobKind::Software, "Recover Software audit state");
                self.software.reduce(SoftwareAction::AuditStarted(job_id));
                vec![Effect::StartSoftwareAudit { job_id }]
            }
            KeyCode::Enter if self.software.phase == SoftwarePhase::PreviewReady => {
                self.software.reduce(SoftwareAction::OpenConfirmation);
                Vec::new()
            }
            KeyCode::Enter if self.software.phase == SoftwarePhase::Confirming => {
                let (Some(plan), Some(preview)) =
                    (self.software.plan.clone(), self.software.preview.clone())
                else {
                    return Vec::new();
                };
                let job_id =
                    self.start_job(JobKind::Software, "Uninstall confirmed Software selection");
                self.software
                    .reduce(SoftwareAction::UninstallStarted(job_id));
                vec![Effect::StartSoftwareUninstall {
                    job_id,
                    plan,
                    preview_digest: preview.digest,
                }]
            }
            KeyCode::Char('x') => {
                if let Some(operation_id) = self.software.operation_id {
                    self.software
                        .reduce(SoftwareAction::CancelRequested(operation_id));
                }
                self.cancel_active_job()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.software.reduce(SoftwareAction::MoveCursor(1));
                Vec::new()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.software.reduce(SoftwareAction::MoveCursor(-1));
                Vec::new()
            }
            KeyCode::Char(' ') => {
                self.software.reduce(SoftwareAction::ToggleFocused);
                Vec::new()
            }
            KeyCode::Char('a') => {
                self.software.reduce(SoftwareAction::SelectAll);
                Vec::new()
            }
            KeyCode::Char('/') => {
                self.filter_active = true;
                Vec::new()
            }
            KeyCode::Char('p') => {
                self.language_settings.open = true;
                self.language_settings.selected = self.shell.locale;
                self.language_settings.failure = None;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn handle_optimize_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Char('q') => self.request_quit(),
            KeyCode::Esc => {
                if self.optimize.phase == OptimizePhase::Confirming {
                    self.optimize.reduce(OptimizeAction::CloseConfirmation);
                    Vec::new()
                } else {
                    self.request_quit()
                }
            }
            KeyCode::Char('i') => {
                if self.has_active_mutation_job() || self.optimize.operation_id.is_some() {
                    return Vec::new();
                }
                let job_id = self.start_job(JobKind::Optimize, "Load Optimize catalogue");
                self.optimize.reduce(OptimizeAction::ListStarted(job_id));
                vec![Effect::StartOptimizeList { job_id }]
            }
            KeyCode::Char('v') => self.start_optimize_preview(),
            KeyCode::Char('r') => {
                if self.has_active_mutation_job() || self.optimize.operation_id.is_some() {
                    return Vec::new();
                }
                let job_id = self.start_job(JobKind::Optimize, "Recover Optimize audit state");
                self.optimize.reduce(OptimizeAction::AuditStarted(job_id));
                vec![Effect::StartOptimizeAudit { job_id }]
            }
            KeyCode::Enter if self.optimize.phase == OptimizePhase::PreviewReady => {
                self.optimize.reduce(OptimizeAction::OpenConfirmation);
                Vec::new()
            }
            KeyCode::Enter if self.optimize.phase == OptimizePhase::Confirming => {
                self.start_optimize_run()
            }
            KeyCode::Enter => {
                self.optimize.reduce(OptimizeAction::SelectFocused);
                Vec::new()
            }
            KeyCode::Char('x') => {
                if let Some(operation_id) = self.optimize.operation_id {
                    self.optimize
                        .reduce(OptimizeAction::CancelRequested(operation_id));
                }
                self.cancel_active_job()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.optimize.reduce(OptimizeAction::MoveCursor(1));
                Vec::new()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.optimize.reduce(OptimizeAction::MoveCursor(-1));
                Vec::new()
            }
            KeyCode::Char(' ') => {
                self.optimize.reduce(OptimizeAction::SelectFocused);
                Vec::new()
            }
            KeyCode::Char('p') => {
                self.language_settings.open = true;
                self.language_settings.selected = self.shell.locale;
                self.language_settings.failure = None;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn start_optimize_preview(&mut self) -> Vec<Effect> {
        let Some(entry) = self.optimize.selected_entry().cloned() else {
            return Vec::new();
        };
        if !crate::tui::modes::optimize::dispatchable(entry.action_class)
            || self.optimize.operation_id.is_some()
        {
            return Vec::new();
        }
        let job_id = self.start_job(JobKind::Optimize, "Revalidate Optimize preview");
        self.optimize.reduce(OptimizeAction::PreviewStarted(job_id));
        vec![Effect::StartOptimizePreview {
            job_id,
            catalogue_id: entry.id.to_string(),
        }]
    }

    fn start_optimize_run(&mut self) -> Vec<Effect> {
        let (Some(plan), Some(preview)) =
            (self.optimize.plan.clone(), self.optimize.preview.clone())
        else {
            return Vec::new();
        };
        let job_id = self.start_job(JobKind::Optimize, "Dispatch confirmed Optimize operation");
        self.optimize.reduce(OptimizeAction::RunStarted(job_id));
        vec![Effect::StartOptimizeRun {
            job_id,
            plan,
            preview_digest: preview.digest,
        }]
    }

    pub(super) fn handle_status_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.request_quit(),
            KeyCode::Char('r') => self.start_status_snapshot(),
            KeyCode::Char('l') => self.start_status_live(),
            KeyCode::Char('x') => {
                if let Some(operation_id) = self.status.operation_id {
                    self.status
                        .reduce(crate::tui::modes::status::StatusAction::CancelRequested(
                            operation_id,
                        ));
                }
                self.cancel_active_job()
            }
            KeyCode::Char(']') => {
                self.status
                    .reduce(crate::tui::modes::status::StatusAction::IntervalStep(1));
                self.maybe_restart_status_live()
            }
            KeyCode::Char('[') => {
                self.status
                    .reduce(crate::tui::modes::status::StatusAction::IntervalStep(-1));
                self.maybe_restart_status_live()
            }
            KeyCode::Char('s') => {
                self.status
                    .reduce(crate::tui::modes::status::StatusAction::CycleSort);
                Vec::new()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.status
                    .reduce(crate::tui::modes::status::StatusAction::MoveCursor(1));
                Vec::new()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.status
                    .reduce(crate::tui::modes::status::StatusAction::MoveCursor(-1));
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn start_status_snapshot(&mut self) -> Vec<Effect> {
        if self.has_active_mutation_job() || self.status.operation_id.is_some() {
            return Vec::new();
        }
        let job_id = self.start_job(JobKind::Status, "Capture Status snapshot");
        self.status
            .reduce(crate::tui::modes::status::StatusAction::SnapshotStarted(
                job_id,
            ));
        vec![Effect::StartStatusSnapshot { job_id }]
    }

    pub(super) fn start_status_live(&mut self) -> Vec<Effect> {
        if self.has_active_mutation_job() || self.status.operation_id.is_some() {
            return Vec::new();
        }
        let job_id = self.start_job(JobKind::Status, "Start Status live");
        let interval_ms = self.status.interval_ms;
        let process_limit = self.status.process_limit;
        self.status
            .reduce(crate::tui::modes::status::StatusAction::LiveStarted(job_id));
        vec![Effect::StartStatusLive {
            job_id,
            interval_ms,
            process_limit,
        }]
    }

    fn maybe_restart_status_live(&mut self) -> Vec<Effect> {
        if !self.status.pending_live_restart
            || self.status.operation != Some(crate::tui::modes::status::StatusOperation::Live)
        {
            return Vec::new();
        }
        if let Some(operation_id) = self.status.operation_id {
            self.status
                .reduce(crate::tui::modes::status::StatusAction::CancelRequested(
                    operation_id,
                ));
            return self.cancel_active_job();
        }
        Vec::new()
    }

    fn handle_language_settings_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        if self.language_settings.pending_request.is_some() {
            return Vec::new();
        }
        match key.code {
            KeyCode::Esc => {
                self.language_settings.open = false;
                self.language_settings.selected = self.shell.locale;
                self.language_settings.failure = None;
                Vec::new()
            }
            KeyCode::Left
            | KeyCode::Up
            | KeyCode::Right
            | KeyCode::Down
            | KeyCode::Tab
            | KeyCode::BackTab => {
                self.language_settings.selected = match self.language_settings.selected {
                    Locale::En => Locale::ZhCn,
                    Locale::ZhCn => Locale::En,
                };
                self.language_settings.failure = None;
                Vec::new()
            }
            KeyCode::Char('e') => {
                self.language_settings.selected = Locale::En;
                self.language_settings.failure = None;
                Vec::new()
            }
            KeyCode::Char('z') => {
                self.language_settings.selected = Locale::ZhCn;
                self.language_settings.failure = None;
                Vec::new()
            }
            KeyCode::Enter => {
                let request_id = self.next_language_request_id;
                self.next_language_request_id = self.next_language_request_id.saturating_add(1);
                let locale = self.language_settings.selected;
                self.language_settings.pending_request = Some(request_id);
                self.language_settings.failure = None;
                vec![Effect::SavePresentationLanguage { request_id, locale }]
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn handle_filter_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        if self.shell.active == ModeId::Analyze {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.filter_active = false,
                KeyCode::Backspace => {
                    let mut query = self.analyze.query.clone();
                    query.pop();
                    self.analyze.reduce(AnalyzeAction::QueryChanged(query));
                }
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let mut query = self.analyze.query.clone();
                    query.push(ch);
                    self.analyze.reduce(AnalyzeAction::QueryChanged(query));
                }
                _ => {}
            }
            return Vec::new();
        }
        if self.shell.active == ModeId::Software {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.filter_active = false,
                KeyCode::Backspace => {
                    let mut query = self.software.query.clone();
                    query.pop();
                    self.software.reduce(SoftwareAction::QueryChanged(query));
                }
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let mut query = self.software.query.clone();
                    query.push(ch);
                    self.software.reduce(SoftwareAction::QueryChanged(query));
                }
                _ => {}
            }
            return Vec::new();
        }
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

    pub(super) fn handle_overlay_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.overlay = Overlay::None;
            }
            _ => {}
        }
        Vec::new()
    }

    pub(super) fn handle_quit_confirm_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Char('w') | KeyCode::Enter => {
                // Keep waiting for the active job; dismiss the overlay.
                self.overlay = Overlay::None;
                self.log_entry(
                    AppLogLevel::Info,
                    AppLogSource::App,
                    None,
                    None,
                    "Continuing to wait for active jobs before quit is allowed",
                );
                Vec::new()
            }
            KeyCode::Char('c') | KeyCode::Char('x') => {
                self.overlay = Overlay::None;
                self.quit_after_jobs = true;
                let mut effects = self.cancel_all_active_jobs();
                self.log_entry(
                    AppLogLevel::Warning,
                    AppLogSource::App,
                    None,
                    None,
                    "Cancel requested; waiting for workers to confirm before quit",
                );
                if !self.has_active_mutation_job() {
                    self.should_quit = true;
                    effects.push(Effect::Quit);
                }
                effects
            }
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                self.quit_after_jobs = false;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn request_quit(&mut self) -> Vec<Effect> {
        if self.has_active_mutation_job() {
            self.overlay = Overlay::QuitConfirm;
            self.log_entry(
                AppLogLevel::Warning,
                AppLogSource::App,
                None,
                None,
                "Active job running: press w to wait, c to cancel-and-wait, Esc to stay",
            );
            return Vec::new();
        }
        self.should_quit = true;
        vec![Effect::Quit]
    }

    pub(super) fn cancel_all_active_jobs(&mut self) -> Vec<Effect> {
        let active: Vec<JobId> = self
            .jobs
            .iter()
            .filter(|job| job.status.is_active())
            .map(|job| job.id)
            .collect();
        let mut effects = Vec::new();
        for job_id in active {
            if self.transition_job(
                job_id,
                JobStatus::Cancelling,
                "Cancellation requested; waiting for worker confirmation.",
            ) {
                effects.push(Effect::CancelJob { job_id });
            }
        }
        effects
    }

    pub(super) fn has_active_mutation_job(&self) -> bool {
        self.jobs.iter().any(|job| job.status.is_active())
    }

    pub(super) fn maybe_finish_pending_quit(&mut self) {
        if self.quit_after_jobs && !self.has_active_mutation_job() {
            self.should_quit = true;
        }
    }

    pub(super) fn handle_confirm_key(&mut self, key: KeyEvent) -> Vec<Effect> {
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
}
