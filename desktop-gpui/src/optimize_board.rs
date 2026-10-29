//! Grouped Optimize browsing.
//!
//! Rows keep the published class, privilege, skip, and last-result facts.
//! Guidance and observe rows have no schedule control. Host defrag, DISM, and
//! clock-sync rows stay not-run. This module does not dispatch those tools.

use devsweep_core::optimize::{MaintenanceActionClass, catalogue_entry, published_entries};

use crate::copy::{self, Locale};
use crate::workbench::optimize_row_line;

pub const GROUP_STARTUP: &str = "group:startup";
pub const GROUP_NETWORK: &str = "group:network_search";
pub const GROUP_MAINTENANCE: &str = "group:maintenance";

const HOST_NOT_DISPOSABLE: &[&str] = &[
    "guidance.drive_optimize",
    "maintenance.retrim",
    "maintenance.image_scan",
    "maintenance.image_repair",
    "maintenance.clock_sync",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptimizeGroup {
    Startup,
    NetworkAndSearch,
    Maintenance,
}

impl OptimizeGroup {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Startup => GROUP_STARTUP,
            Self::NetworkAndSearch => GROUP_NETWORK,
            Self::Maintenance => GROUP_MAINTENANCE,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::NetworkAndSearch => "network_search",
            Self::Maintenance => "maintenance",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptimizePhase {
    Landing,
    Running,
    Completed,
    Failed,
    AdminWait,
}

impl OptimizePhase {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Landing => "landing",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::AdminWait => "admin_wait",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptimizeBoard {
    group: Option<OptimizeGroup>,
    phase: OptimizePhase,
    query: String,
}

impl Default for OptimizeBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimizeBoard {
    #[must_use]
    pub fn new() -> Self {
        Self {
            group: None,
            phase: OptimizePhase::Landing,
            query: String::new(),
        }
    }

    #[must_use]
    pub fn group(&self) -> Option<OptimizeGroup> {
        self.group
    }

    #[must_use]
    pub fn phase(&self) -> OptimizePhase {
        self.phase
    }

    pub fn open_group(&mut self, group: OptimizeGroup) {
        self.group = Some(group);
        if self.phase == OptimizePhase::Completed {
            self.phase = OptimizePhase::Landing;
        }
    }

    pub fn close_group(&mut self) {
        self.group = None;
        self.phase = OptimizePhase::Landing;
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
    }

    /// Host maintenance cannot become Completed. Guidance and observe cannot run.
    pub fn note_completed(&mut self, id: &str) -> bool {
        if !schedule_allowed(id) || HOST_NOT_DISPOSABLE.contains(&id) {
            self.phase = OptimizePhase::Landing;
            return false;
        }
        self.phase = OptimizePhase::Completed;
        true
    }

    pub fn note_running(&mut self, id: &str) -> bool {
        if !schedule_allowed(id) || HOST_NOT_DISPOSABLE.contains(&id) {
            return false;
        }
        self.phase = OptimizePhase::Running;
        true
    }

    pub fn note_failed(&mut self, id: &str) {
        if HOST_NOT_DISPOSABLE.contains(&id) {
            self.phase = OptimizePhase::Landing;
            return;
        }
        self.phase = OptimizePhase::Failed;
    }

    pub fn note_admin_wait(&mut self, id: &str) {
        let _ = id;
        self.phase = OptimizePhase::AdminWait;
    }

    #[must_use]
    pub fn phase_line(&self, locale: Locale) -> String {
        let name = copy::optimize_phase_name(locale, self.phase.key());
        match self.phase {
            OptimizePhase::AdminWait => {
                format!("phase={name} | {}", copy::optimize_admin_wait(locale))
            }
            OptimizePhase::Completed | OptimizePhase::Running | OptimizePhase::Failed => {
                format!("phase={name}")
            }
            OptimizePhase::Landing => format!("phase={name}"),
        }
    }

    #[must_use]
    pub fn group_lines(&self, locale: Locale) -> Vec<String> {
        OptimizeGroup::ALL
            .into_iter()
            .map(|group| {
                let mark = if self.group == Some(group) { "> " } else { "" };
                format!(
                    "{mark}{} | {}",
                    group.id(),
                    copy::optimize_group_name(locale, group.key())
                )
            })
            .collect()
    }

    #[must_use]
    pub fn visible_ids(&self) -> Vec<String> {
        let mut ids = if let Some(group) = self.group {
            ids_in(group)
        } else {
            published_entries()
                .iter()
                .map(|entry| entry.id.to_string())
                .collect::<Vec<_>>()
        };
        if !self.query.is_empty() {
            let query = self.query.to_lowercase();
            ids.retain(|id| id.to_lowercase().contains(&query));
        }
        ids
    }

    #[must_use]
    pub fn row_line(&self, id: &str, locale: Locale, focused: bool) -> Option<String> {
        let entry = catalogue_entry(id)?;
        let mut line = optimize_row_line(locale, entry.id, entry.action_class);
        if !schedule_allowed(id) {
            line.push_str(" | ");
            line.push_str(copy::optimize_schedule_absent(locale));
        }
        if focused {
            Some(format!("> {line}"))
        } else {
            Some(line)
        }
    }
}

impl OptimizeGroup {
    const ALL: [Self; 3] = [Self::Startup, Self::NetworkAndSearch, Self::Maintenance];
}

#[must_use]
pub fn group_from_id(id: &str) -> Option<OptimizeGroup> {
    match id {
        GROUP_STARTUP => Some(OptimizeGroup::Startup),
        GROUP_NETWORK => Some(OptimizeGroup::NetworkAndSearch),
        GROUP_MAINTENANCE => Some(OptimizeGroup::Maintenance),
        _ => None,
    }
}

#[must_use]
pub fn group_of(id: &str) -> Option<OptimizeGroup> {
    match id {
        "maintenance.dead_startup" => Some(OptimizeGroup::Startup),
        "dns.flush" | "settings.search" | "guidance.network_reset" | "maintenance.search" => {
            Some(OptimizeGroup::NetworkAndSearch)
        }
        "settings.storage_recommendations"
        | "settings.energy_recommendations"
        | "guidance.drive_optimize"
        | "guidance.system_integrity"
        | "guidance.filesystem_check"
        | "maintenance.retrim"
        | "maintenance.clock_probe"
        | "maintenance.clock_sync"
        | "maintenance.image_scan"
        | "maintenance.image_repair"
        | "maintenance.disk_health"
        | "maintenance.update_cache"
        | "maintenance.icon_cache"
        | "maintenance.winget_repair" => Some(OptimizeGroup::Maintenance),
        _ => None,
    }
}

#[must_use]
pub fn ids_in(group: OptimizeGroup) -> Vec<String> {
    published_entries()
        .iter()
        .filter(|entry| group_of(entry.id) == Some(group))
        .map(|entry| entry.id.to_string())
        .collect()
}

#[must_use]
pub fn schedule_allowed(id: &str) -> bool {
    if HOST_NOT_DISPOSABLE.contains(&id) {
        return false;
    }
    match catalogue_entry(id).map(|entry| entry.action_class) {
        Some(MaintenanceActionClass::Guidance | MaintenanceActionClass::Observe) => false,
        Some(_) => true,
        None => false,
    }
}

#[must_use]
pub fn browse_ids() -> Vec<String> {
    let mut ids = vec![
        GROUP_STARTUP.to_string(),
        GROUP_NETWORK.to_string(),
        GROUP_MAINTENANCE.to_string(),
    ];
    ids.extend(crate::workbench_review::optimize_row_ids());
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_cover_every_row_and_keep_image_scan_apart_from_repair() {
        let mut seen = Vec::new();
        for entry in published_entries() {
            let group = group_of(entry.id).unwrap_or_else(|| panic!("ungrouped {}", entry.id));
            seen.push((entry.id, group));
        }
        assert_eq!(seen.len(), published_entries().len());
        let maintenance = ids_in(OptimizeGroup::Maintenance);
        assert!(maintenance.contains(&"maintenance.image_scan".to_string()));
        assert!(maintenance.contains(&"maintenance.image_repair".to_string()));
        assert_ne!(
            maintenance
                .iter()
                .position(|id| id == "maintenance.image_scan"),
            maintenance
                .iter()
                .position(|id| id == "maintenance.image_repair")
        );
        assert!(ids_in(OptimizeGroup::Startup).contains(&"maintenance.dead_startup".to_string()));
        assert!(ids_in(OptimizeGroup::NetworkAndSearch).contains(&"dns.flush".to_string()));
    }

    #[test]
    fn guidance_observe_and_host_rows_have_no_success_phase() {
        let mut board = OptimizeBoard::new();
        assert_eq!(board.phase(), OptimizePhase::Landing);
        assert!(board.phase_line(Locale::En).contains("Landing"));
        assert!(!schedule_allowed("guidance.drive_optimize"));
        assert!(!schedule_allowed("maintenance.disk_health"));
        assert!(!board.note_running("guidance.system_integrity"));
        assert!(!board.note_completed("maintenance.image_repair"));
        assert!(!board.note_completed("maintenance.clock_sync"));
        assert!(!board.note_completed("maintenance.retrim"));
        assert_eq!(board.phase(), OptimizePhase::Landing);
        let repair = board
            .row_line("maintenance.image_repair", Locale::ZhCn, false)
            .expect("repair");
        assert!(repair.contains("未运行"));
        assert!(repair.contains("无调度按钮") || repair.contains("class=execute"));
        assert!(!repair.contains("成功"));
        let guidance = board
            .row_line("guidance.drive_optimize", Locale::En, true)
            .expect("guidance");
        assert!(guidance.contains("no schedule button"));
        assert!(guidance.contains("class=guidance"));
        assert!(!guidance.to_ascii_lowercase().contains("succeed"));

        assert!(board.note_running("dns.flush"));
        assert_eq!(board.phase(), OptimizePhase::Running);
        board.note_failed("dns.flush");
        assert_eq!(board.phase(), OptimizePhase::Failed);
        assert!(board.note_completed("dns.flush"));
        assert_eq!(board.phase(), OptimizePhase::Completed);
        board.note_admin_wait("maintenance.retrim");
        assert_eq!(board.phase(), OptimizePhase::AdminWait);
        let wait = board.phase_line(Locale::En);
        assert!(wait.contains("Admin wait"));
        assert!(!wait.to_ascii_lowercase().contains("succeed"));
        board.note_failed("maintenance.image_scan");
        assert_eq!(board.phase(), OptimizePhase::Landing);

        board.open_group(OptimizeGroup::Maintenance);
        board.set_query("image_");
        let visible = board.visible_ids();
        assert!(visible.contains(&"maintenance.image_scan".to_string()));
        assert!(visible.contains(&"maintenance.image_repair".to_string()));
        assert!(!visible.contains(&"dns.flush".to_string()));
    }
}
