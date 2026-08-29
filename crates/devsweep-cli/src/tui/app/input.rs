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

    pub(super) fn handle_filter_key(&mut self, key: KeyEvent) -> Vec<Effect> {
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
