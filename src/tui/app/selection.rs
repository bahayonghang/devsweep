use super::*;

impl App {
    pub(super) fn move_selection(&mut self, delta: isize) {
        let count = self.visible_target_rows().len();
        if count == 0 {
            self.selected_index = 0;
            self.list_scroll = 0;
            return;
        }

        let current = self.selected_index.min(count - 1) as isize;
        let next = (current + delta).clamp(0, count as isize - 1);
        self.selected_index = next as usize;
        self.ensure_selection_visible(count.saturating_sub(1).max(1));
    }

    pub(super) fn move_inventory_selection(&mut self, delta: isize) {
        let count = self
            .inventory_report
            .as_ref()
            .map_or(0, |report| report.observations.len());
        if count == 0 {
            self.inventory_selected_index = 0;
            self.inventory_list_scroll = 0;
            return;
        }

        let current = self.inventory_selected_index.min(count - 1) as isize;
        self.inventory_selected_index = (current + delta).clamp(0, count as isize - 1) as usize;
        self.clamp_inventory_selection();
    }

    pub(super) fn clamp_inventory_selection(&mut self) {
        let count = self
            .inventory_report
            .as_ref()
            .map_or(0, |report| report.observations.len());
        if count == 0 {
            self.inventory_selected_index = 0;
            self.inventory_list_scroll = 0;
            return;
        }

        self.inventory_selected_index = self.inventory_selected_index.min(count - 1);
        let page_size = self.viewport_page_size().max(1);
        if self.inventory_selected_index < self.inventory_list_scroll {
            self.inventory_list_scroll = self.inventory_selected_index;
        } else if self.inventory_selected_index >= self.inventory_list_scroll + page_size {
            self.inventory_list_scroll = self.inventory_selected_index + 1 - page_size;
        }
        self.inventory_list_scroll = self
            .inventory_list_scroll
            .min(count.saturating_sub(page_size));
    }

    pub(super) fn page_selection(&mut self, direction: isize) {
        let page = self.viewport_page_size().max(1) as isize;
        self.move_selection(direction * page);
    }

    /// Keep the selected row inside a viewport of `page_size` content rows.
    pub(super) fn ensure_selection_visible(&mut self, page_size: usize) {
        let count = self.visible_target_rows().len();
        if count == 0 || page_size == 0 {
            self.list_scroll = 0;
            self.selected_index = 0;
            return;
        }
        self.selected_index = self.selected_index.min(count - 1);
        if self.selected_index < self.list_scroll {
            self.list_scroll = self.selected_index;
        } else if self.selected_index >= self.list_scroll + page_size {
            self.list_scroll = self.selected_index + 1 - page_size;
        }
        let max_scroll = count.saturating_sub(page_size);
        self.list_scroll = self.list_scroll.min(max_scroll);
    }

    pub(super) fn viewport_page_size(&self) -> usize {
        // Default page used by keyboard navigation before the next render
        // reports an exact panel height.
        10
    }

    pub(in crate::tui) fn is_cleaned(&self, target_id: &TargetId) -> bool {
        self.cleaned_ids.contains(target_id)
    }

    pub(super) fn mark_target_cleaned(&mut self, target_id: TargetId) {
        self.cleaned_ids.insert(target_id.clone());
        self.selected_ids.remove(&target_id);
        self.selection_overrides
            .insert(target_id, SelectionOverride::Deselected);
    }

    pub(super) fn toggle_selected_target(&mut self) {
        match self.selected_target_row() {
            Some(TargetListRow::Target(index)) => {
                let target = &self.targets[index];
                self.toggle_target_selection(target.id.clone(), target.action.is_executable());
            }
            Some(TargetListRow::PycacheGroup(group)) => {
                self.toggle_pycache_group_selection(&group);
            }
            None => {}
        }
    }

    pub(super) fn toggle_target_selection(&mut self, target_id: TargetId, executable: bool) {
        if self.cleaned_ids.contains(&target_id) {
            self.log_entry(
                AppLogLevel::Warning,
                AppLogSource::Clean,
                None,
                Some(target_id),
                "Cleaned targets stay disabled until the next rescan",
            );
            return;
        }

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

        self.set_target_selected(target_id.clone(), !self.selected_ids.contains(&target_id));
    }

    pub(super) fn toggle_pycache_group_selection(&mut self, group: &PycacheGroup) {
        let executable_ids: Vec<TargetId> = group
            .target_indices
            .iter()
            .filter_map(|index| self.targets.get(*index))
            .filter(|target| {
                target.action.is_executable() && !self.cleaned_ids.contains(&target.id)
            })
            .map(|target| target.id.clone())
            .collect();
        if executable_ids.is_empty() {
            return;
        }

        let select = !executable_ids
            .iter()
            .all(|target_id| self.selected_ids.contains(target_id));
        for target_id in executable_ids {
            self.set_target_selected(target_id, select);
        }
    }

    pub(super) fn toggle_visible_selection(&mut self) {
        let visible_ids: Vec<TargetId> = self
            .visible_target_rows()
            .into_iter()
            .flat_map(|row| row.target_indices())
            .filter_map(|index| self.targets.get(index))
            .filter(|target| target.action.is_executable())
            .map(|target| target.id.clone())
            .collect();

        if visible_ids.is_empty() {
            return;
        }

        let all_selected = visible_ids.iter().all(|id| self.selected_ids.contains(id));
        for id in visible_ids {
            self.set_target_selected(id, !all_selected);
        }
    }

    pub(super) fn set_target_selected(&mut self, target_id: TargetId, selected: bool) {
        if selected && self.cleaned_ids.contains(&target_id) {
            return;
        }
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

    pub(super) fn cycle_risk_filter(&mut self) {
        self.risk_filter = match self.risk_filter.as_ref() {
            None => Some(RiskLevel::Low),
            Some(RiskLevel::Low) => Some(RiskLevel::Medium),
            Some(RiskLevel::Medium) => Some(RiskLevel::High),
            Some(RiskLevel::High) => Some(RiskLevel::Dangerous),
            Some(RiskLevel::Dangerous) => None,
        };
    }

    pub(in crate::tui) fn selected_target(&self) -> Option<&CleanTarget> {
        let TargetListRow::Target(index) = self.selected_target_row()? else {
            return None;
        };
        self.targets.get(index)
    }

    pub(in crate::tui) fn selected_pycache_group(&self) -> Option<PycacheGroup> {
        let TargetListRow::PycacheGroup(group) = self.selected_target_row()? else {
            return None;
        };
        Some(group)
    }

    pub(in crate::tui) fn selected_targets(&self) -> Vec<&CleanTarget> {
        self.targets
            .iter()
            .filter(|target| {
                self.selected_ids.contains(&target.id)
                    && target.action.is_executable()
                    && !self.cleaned_ids.contains(&target.id)
            })
            .collect()
    }

    pub(in crate::tui) fn visible_target_rows(&self) -> Vec<TargetListRow> {
        let visible_indices: Vec<usize> = self
            .targets
            .iter()
            .enumerate()
            .filter(|(_, target)| self.target_is_visible(target))
            .map(|(index, _)| index)
            .collect();
        let mut pycache_groups: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        for index in &visible_indices {
            if let Some(project_root) = pycache_project_root(&self.targets[*index]) {
                pycache_groups
                    .entry(project_root.clone())
                    .or_default()
                    .push(*index);
            }
        }

        let mut grouped_projects = HashSet::new();
        let mut rows = Vec::with_capacity(visible_indices.len());
        for index in visible_indices {
            let Some(project_root) = pycache_project_root(&self.targets[index]) else {
                rows.push(TargetListRow::Target(index));
                continue;
            };
            let Some(group) = pycache_groups.get(project_root) else {
                rows.push(TargetListRow::Target(index));
                continue;
            };
            if group.len() < 2 {
                rows.push(TargetListRow::Target(index));
                continue;
            }
            if !grouped_projects.insert(project_root.clone()) {
                continue;
            }
            if self.collapsed_pycache_projects.contains(project_root) {
                rows.push(TargetListRow::PycacheGroup(PycacheGroup {
                    project_root: project_root.clone(),
                    target_indices: group.clone(),
                }));
            } else {
                rows.extend(group.iter().copied().map(TargetListRow::Target));
            }
        }
        rows
    }

    pub(super) fn selected_target_row(&self) -> Option<TargetListRow> {
        let rows = self.visible_target_rows();
        rows.get(self.selected_index.min(rows.len().saturating_sub(1)))
            .cloned()
    }

    pub(super) fn target_is_visible(&self, target: &CleanTarget) -> bool {
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

    pub(super) fn toggle_selected_pycache_project(&mut self) {
        let project_root = match self.selected_target_row() {
            Some(TargetListRow::PycacheGroup(group)) => Some(group.project_root),
            Some(TargetListRow::Target(index)) => self
                .targets
                .get(index)
                .and_then(pycache_project_root)
                .cloned(),
            None => None,
        };
        if let Some(project_root) = project_root {
            self.toggle_pycache_project(&project_root);
        }
    }

    pub(super) fn toggle_pycache_project(&mut self, project_root: &PathBuf) {
        if !pycache_project_roots(&self.targets).contains(project_root) {
            return;
        }
        if !self.collapsed_pycache_projects.remove(project_root) {
            self.collapsed_pycache_projects.insert(project_root.clone());
        }
        self.selected_index = 0;
        self.list_scroll = 0;
    }

    pub(in crate::tui) fn selected_bytes(&self) -> u64 {
        sum_unique_target_bytes(self.selected_targets())
    }

    #[cfg(test)]
    pub(super) fn scope_bytes(&self, scope_kind: ScopeKind) -> u64 {
        sum_unique_target_bytes(self.targets.iter().filter(|target| match scope_kind {
            ScopeKind::Global => matches!(target.scope, Scope::Global),
            ScopeKind::Project => matches!(target.scope, Scope::Project { .. }),
        }))
    }
}
