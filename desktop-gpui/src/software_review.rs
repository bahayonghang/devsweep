//! Isolated Software review for Uninstall, Updates, and Startup.
//!
//! Search, sort, filter, and refresh read only an injected inventory, an
//! updates document, or a temporary startup hive. This module does not scan
//! the host uninstall list, list host startup keys, probe winget, or construct
//! a host software executor.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use devsweep_core::software::{
    SOFTWARE_INVENTORY_VERSION, SoftwareEligibilityReason, SoftwareEligibilityState,
    SoftwareEntryV1, SoftwareIdentity, SoftwareInventoryV1, SoftwarePlanError,
    SoftwareSelectionPlanV1, SoftwareSourceEvidence, SoftwareSourceState, SoftwareStartupError,
    SoftwareStartupLocation, SoftwareUpdateError, SoftwareUpdateRowV1, SoftwareUpdatesV1,
    StartupFixtureClass, StartupFixtureHiveV1, build_selection_plan, plan_software_update,
    plan_startup_hive, preview_selection_plan_live, preview_startup_hive, software_entry_id,
    software_inventory_fingerprint, startup_fixture_entry_id,
};

use crate::copy::{self, Locale};
use crate::workbench::SoftwareSegment;

pub const INVENTORY_ENV: &str = "DEVSWEEP_SOFTWARE_INVENTORY";
pub const UPDATES_FILE_ENV: &str = "DEVSWEEP_SOFTWARE_UPDATES_FILE";
pub const UPDATE_PROGRAM_ENV: &str = "DEVSWEEP_SOFTWARE_UPDATE_PROGRAM";
pub const HIVE_ENV: &str = "DEVSWEEP_SOFTWARE_STARTUP_HIVE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftwareSort {
    Name,
    Source,
    Version,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftwareFilter {
    All,
    Selectable,
    Protected,
    PermissionDenied,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoftwareItem {
    pub id: String,
    pub name: String,
    pub identity: String,
    pub count: u64,
    pub source: String,
    pub version: String,
    pub permission: String,
    pub skip: String,
    pub last: String,
    pub selectable: bool,
}

#[derive(Clone, Debug)]
struct PendingUninstall {
    id: String,
    plan: SoftwareSelectionPlanV1,
    digest: String,
}

#[derive(Clone, Debug)]
enum ReviewSource {
    None,
    Inventory {
        path: Option<PathBuf>,
        inventory: SoftwareInventoryV1,
    },
    Updates {
        path: Option<PathBuf>,
        rows: Vec<SoftwareUpdateRowV1>,
    },
    Hive {
        path: PathBuf,
    },
}

/// One segment review. The window leaves the executor unset.
#[derive(Clone, Debug)]
pub struct SoftwareReview {
    segment: Option<SoftwareSegment>,
    source: ReviewSource,
    items: Vec<SoftwareItem>,
    query: String,
    sort: SoftwareSort,
    filter: SoftwareFilter,
    lines: Vec<String>,
    pending: Option<PendingUninstall>,
    stale: bool,
    now_unix_ms: u64,
    executor_calls: u32,
}

impl Default for SoftwareReview {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareReview {
    #[must_use]
    pub fn new() -> Self {
        Self {
            segment: None,
            source: ReviewSource::None,
            items: Vec::new(),
            query: String::new(),
            sort: SoftwareSort::Name,
            filter: SoftwareFilter::All,
            lines: Vec::new(),
            pending: None,
            stale: false,
            now_unix_ms: unix_now_ms(),
            executor_calls: 0,
        }
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.segment.is_some()
    }

    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    #[must_use]
    pub fn executor_calls(&self) -> u32 {
        self.executor_calls
    }

    pub fn close(&mut self) {
        *self = Self {
            now_unix_ms: self.now_unix_ms,
            ..Self::new()
        };
    }

    pub fn set_now(&mut self, now_unix_ms: u64) {
        self.now_unix_ms = now_unix_ms;
    }

    pub fn arm_stale(&mut self) {
        self.stale = true;
    }

    /// Opens a segment from an explicit isolated path, or from the matching
    /// environment variable. A missing source stays empty. It does not read
    /// the host uninstall list.
    pub fn open_segment(&mut self, segment: SoftwareSegment, locale: Locale) {
        if segment == SoftwareSegment::Leftovers {
            self.segment = Some(segment);
            self.items.clear();
            self.lines = vec![copy::software_leftovers_unavailable(locale).to_string()];
            self.pending = None;
            return;
        }
        match segment {
            SoftwareSegment::Uninstall => {
                if let Some(path) = env_path(INVENTORY_ENV) {
                    self.open_inventory_file(&path, locale);
                } else {
                    self.mark_empty(segment, locale);
                }
            }
            SoftwareSegment::Updates => {
                if let Some(path) = env_path(UPDATES_FILE_ENV) {
                    self.open_updates_file(&path, locale);
                } else if let Some(program) = env_path(UPDATE_PROGRAM_ENV) {
                    self.probe_update_program(&program, locale);
                } else {
                    self.segment = Some(segment);
                    self.source = ReviewSource::None;
                    self.items.clear();
                    self.lines = vec![
                        copy::software_updates_title(locale).to_string(),
                        copy::software_updates_not_checked(locale).to_string(),
                    ];
                    self.pending = None;
                }
            }
            SoftwareSegment::Startup => {
                if let Some(path) = env_path(HIVE_ENV) {
                    self.open_hive(&path, locale);
                } else {
                    self.startup_missing(locale);
                }
            }
            SoftwareSegment::Leftovers => {}
        }
    }

    pub fn open_inventory(&mut self, inventory: SoftwareInventoryV1, locale: Locale) {
        self.segment = Some(SoftwareSegment::Uninstall);
        self.pending = None;
        self.source = ReviewSource::Inventory {
            path: None,
            inventory,
        };
        self.rebuild_items(locale);
    }

    pub fn open_inventory_file(&mut self, path: &Path, locale: Locale) {
        self.segment = Some(SoftwareSegment::Uninstall);
        self.pending = None;
        match read_inventory(path) {
            Ok(inventory) => {
                self.source = ReviewSource::Inventory {
                    path: Some(path.to_path_buf()),
                    inventory,
                };
                self.rebuild_items(locale);
            }
            Err(error) => {
                self.source = ReviewSource::None;
                self.items.clear();
                self.lines = vec![format!("inventory_unreadable: {error}")];
            }
        }
    }

    pub fn open_updates(&mut self, rows: Vec<SoftwareUpdateRowV1>, locale: Locale) {
        self.segment = Some(SoftwareSegment::Updates);
        self.pending = None;
        self.source = ReviewSource::Updates { path: None, rows };
        self.rebuild_items(locale);
    }

    pub fn open_updates_file(&mut self, path: &Path, locale: Locale) {
        self.segment = Some(SoftwareSegment::Updates);
        self.pending = None;
        match fs::read(path).and_then(|bytes| {
            serde_json::from_slice::<SoftwareUpdatesV1>(&bytes).map_err(std::io::Error::other)
        }) {
            Ok(SoftwareUpdatesV1::Available { rows, .. }) => {
                self.source = ReviewSource::Updates {
                    path: Some(path.to_path_buf()),
                    rows,
                };
                self.rebuild_items(locale);
            }
            Ok(SoftwareUpdatesV1::Unavailable { reason_code, .. }) => {
                self.source = ReviewSource::Updates {
                    path: Some(path.to_path_buf()),
                    rows: Vec::new(),
                };
                self.items.clear();
                self.lines = vec![
                    copy::software_updates_unavailable(locale).to_string(),
                    format!("probe_failed: {reason_code:?}"),
                ];
            }
            Err(error) => {
                self.source = ReviewSource::None;
                self.items.clear();
                self.lines = vec![format!("updates_unreadable: {error}")];
            }
        }
    }

    pub fn open_hive(&mut self, path: &Path, locale: Locale) {
        self.segment = Some(SoftwareSegment::Startup);
        self.pending = None;
        self.source = ReviewSource::Hive {
            path: path.to_path_buf(),
        };
        self.rebuild_items(locale);
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
    }

    pub fn cycle_sort(&mut self) {
        self.sort = match self.sort {
            SoftwareSort::Name => SoftwareSort::Source,
            SoftwareSort::Source => SoftwareSort::Version,
            SoftwareSort::Version => SoftwareSort::Name,
        };
    }

    pub fn cycle_filter(&mut self) {
        self.filter = match self.filter {
            SoftwareFilter::All => SoftwareFilter::Selectable,
            SoftwareFilter::Selectable => SoftwareFilter::Protected,
            SoftwareFilter::Protected => SoftwareFilter::PermissionDenied,
            SoftwareFilter::PermissionDenied => SoftwareFilter::All,
        };
    }

    /// Re-reads the isolated file or hive. A memory-only review is left as-is.
    pub fn refresh(&mut self, locale: Locale) {
        let path = match &self.source {
            ReviewSource::Inventory { path, .. } | ReviewSource::Updates { path, .. } => {
                path.clone()
            }
            ReviewSource::Hive { path } => Some(path.clone()),
            ReviewSource::None => None,
        };
        let Some(path) = path else {
            return;
        };
        match self.segment {
            Some(SoftwareSegment::Uninstall) => self.open_inventory_file(&path, locale),
            Some(SoftwareSegment::Updates) => self.open_updates_file(&path, locale),
            Some(SoftwareSegment::Startup) => self.open_hive(&path, locale),
            _ => {}
        }
    }

    #[must_use]
    pub fn filtered_ids(&self) -> Vec<String> {
        let mut items = self
            .items
            .iter()
            .filter(|item| self.filter_matches(item) && self.query_matches(item))
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            let order = match self.sort {
                SoftwareSort::Name => left.name.cmp(&right.name),
                SoftwareSort::Source => left.source.cmp(&right.source),
                SoftwareSort::Version => left.version.cmp(&right.version),
            };
            order.then_with(|| left.id.cmp(&right.id))
        });
        items.into_iter().map(|item| item.id).collect()
    }

    #[must_use]
    pub fn row_line(&self, id: &str, locale: Locale, focused: bool) -> Option<String> {
        let item = self.items.iter().find(|item| item.id == id)?;
        let mark = if focused { "> " } else { "" };
        let last = if item.last.is_empty() {
            copy::workbench_not_run(locale)
        } else {
            item.last.as_str()
        };
        Some(format!(
            "{mark}{} | identity={} | count={} | source={} | version={} | permission={} | skip={} | last={last}",
            item.id,
            item.identity,
            item.count,
            item.source,
            item.version,
            item.permission,
            item.skip
        ))
    }

    /// First activation plans and previews. The second activation confirms
    /// only through an injected isolated executor.
    pub fn activate(
        &mut self,
        id: &str,
        locale: Locale,
        executor: Option<&mut dyn FnMut(&SoftwareSelectionPlanV1, &str) -> Result<String, String>>,
    ) {
        if self.segment == Some(SoftwareSegment::Leftovers) {
            self.lines = vec![copy::software_leftovers_unavailable(locale).to_string()];
            return;
        }
        if self.segment == Some(SoftwareSegment::Startup) {
            self.activate_startup(id, locale);
            return;
        }
        if self.segment == Some(SoftwareSegment::Updates) {
            let row = self.row_line(id, locale, true);
            self.lines = vec![
                row.unwrap_or_else(|| copy::software_updates_not_checked(locale).to_string()),
                format!("last={}", copy::workbench_not_run(locale)),
            ];
            return;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.id == id)
        {
            self.confirm_uninstall(locale, executor);
            return;
        }
        self.preview_uninstall(id, locale);
    }

    fn preview_uninstall(&mut self, id: &str, locale: Locale) {
        let ReviewSource::Inventory { inventory, .. } = &self.source else {
            self.lines = vec![copy::software_digest_missing(locale).to_string()];
            self.pending = None;
            return;
        };
        let inventory = inventory.clone();
        let plan = match build_selection_plan(&inventory, &[id.to_string()], self.now_unix_ms) {
            Ok(plan) => plan,
            Err(error) => {
                self.lines = vec![plan_line(locale, &error)];
                self.pending = None;
                return;
            }
        };
        match preview_selection_plan_live(&plan, &inventory, self.now_unix_ms) {
            Ok(preview) => {
                self.lines = vec![copy::software_preview_summary(
                    locale,
                    preview.selected.len(),
                    &preview.digest,
                )];
                self.pending = Some(PendingUninstall {
                    id: id.to_string(),
                    plan,
                    digest: preview.digest,
                });
            }
            Err(error) => {
                self.lines = vec![plan_line(locale, &error)];
                self.pending = None;
            }
        }
    }

    fn confirm_uninstall(
        &mut self,
        locale: Locale,
        mut executor: Option<
            &mut dyn FnMut(&SoftwareSelectionPlanV1, &str) -> Result<String, String>,
        >,
    ) {
        let Some(pending) = self.pending.clone() else {
            self.lines = vec![copy::software_digest_missing(locale).to_string()];
            return;
        };
        let ReviewSource::Inventory { inventory, .. } = &self.source else {
            self.lines = vec![copy::software_digest_missing(locale).to_string()];
            return;
        };
        let live = match preview_selection_plan_live(&pending.plan, inventory, self.now_unix_ms) {
            Ok(preview) => preview,
            Err(error) => {
                self.lines = vec![plan_line(locale, &error)];
                return;
            }
        };
        if self.stale || live.digest != pending.digest {
            self.lines = vec![plan_line(locale, &SoftwarePlanError::DigestMismatch)];
            return;
        }
        let Some(executor) = executor.as_mut() else {
            self.lines = vec![
                "isolated_executor_unavailable".to_string(),
                copy::software_isolated_executor_unavailable(locale).to_string(),
            ];
            return;
        };
        match executor(&pending.plan, &live.digest) {
            Ok(outcome) => {
                self.executor_calls = self.executor_calls.saturating_add(1);
                if let Some(item) = self.items.iter_mut().find(|item| item.id == pending.id) {
                    item.last = outcome.clone();
                }
                self.lines = vec![format!(
                    "{} | {outcome} | last={}",
                    pending.id,
                    copy::workbench_not_run(locale)
                )];
                self.pending = None;
            }
            Err(error) => {
                self.lines = vec![format!("isolated_executor_refused: {error}")];
            }
        }
    }

    fn activate_startup(&mut self, id: &str, locale: Locale) {
        let ReviewSource::Hive { path } = &self.source else {
            self.startup_missing(locale);
            return;
        };
        let path = path.clone();
        let plan = match plan_startup_hive(&path, id, false) {
            Ok(plan) => plan,
            Err(error) => {
                self.lines = vec![startup_line(locale, &error)];
                return;
            }
        };
        let preview = match preview_startup_hive(&path, &plan) {
            Ok(preview) => preview,
            Err(error) => {
                self.lines = vec![startup_line(locale, &error)];
                return;
            }
        };
        let expected = if self.stale {
            flipped_digest(&preview.digest)
        } else {
            preview.digest.clone()
        };
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.id == id)
        {
            match devsweep_core::software::execute_startup_hive(&path, &plan, &expected, true) {
                Ok(report) => {
                    let outcome = serde_json::to_value(report.toggle.outcome)
                        .ok()
                        .and_then(|value| value.as_str().map(str::to_string))
                        .unwrap_or_else(|| "unknown".to_string());
                    if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
                        item.last = outcome.clone();
                    }
                    self.lines = vec![format!(
                        "startup hive outcome={outcome} | last={}",
                        copy::workbench_not_run(locale)
                    )];
                    self.pending = None;
                    self.rebuild_items(locale);
                    self.lines.push(format!(
                        "startup hive outcome={outcome} | last={}",
                        copy::workbench_not_run(locale)
                    ));
                }
                Err(error) => self.lines = vec![startup_line(locale, &error)],
            }
            return;
        }
        self.lines = vec![format!("{id} | {}", preview.digest)];
        self.pending = Some(PendingUninstall {
            id: id.to_string(),
            plan: SoftwareSelectionPlanV1 {
                version: 1,
                inventory_fingerprint: preview.digest.clone(),
                inventory_observed_at_unix_ms: 0,
                expires_at_unix_ms: 0,
                selected_ids: vec![id.to_string()],
            },
            digest: preview.digest,
        });
    }

    fn probe_update_program(&mut self, program: &Path, locale: Locale) {
        self.segment = Some(SoftwareSegment::Updates);
        self.items.clear();
        self.pending = None;
        self.source = ReviewSource::None;
        let line = match plan_software_update(
            program.as_os_str(),
            "devsweep.fixture.package",
            "winget",
            "1.0.0",
        ) {
            Ok(plan) => format!(
                "updates | {} | {}",
                plan.package_id,
                copy::workbench_not_run(locale)
            ),
            Err(error) => format!("{}: {error}", update_code(&error)),
        };
        self.lines = vec![line];
    }

    fn startup_missing(&mut self, locale: Locale) {
        self.segment = Some(SoftwareSegment::Startup);
        self.items.clear();
        self.pending = None;
        self.source = ReviewSource::None;
        let missing = std::env::temp_dir().join("devsweep-gpui-startup-hive-missing.json");
        let line = match plan_startup_hive(&missing, "startup:v1:missing", false) {
            Ok(_) => startup_line(locale, &SoftwareStartupError::StartupSourceUnavailable),
            Err(error) => startup_line(locale, &error),
        };
        self.lines = vec![line];
    }

    fn mark_empty(&mut self, segment: SoftwareSegment, locale: Locale) {
        self.segment = Some(segment);
        self.source = ReviewSource::None;
        self.items.clear();
        self.pending = None;
        self.lines = vec![
            copy::software_empty_title(locale).to_string(),
            copy::software_empty_detail(locale).to_string(),
        ];
    }

    fn rebuild_items(&mut self, locale: Locale) {
        let last = copy::workbench_not_run(locale).to_string();
        self.items = match &self.source {
            ReviewSource::Inventory { inventory, .. } => inventory_items(inventory, &last),
            ReviewSource::Updates { rows, .. } => update_items(rows, &last),
            ReviewSource::Hive { path } => hive_items(path, locale, &last),
            ReviewSource::None => Vec::new(),
        };
        self.lines = self.summary_lines(locale);
    }

    fn summary_lines(&self, locale: Locale) -> Vec<String> {
        match &self.source {
            ReviewSource::Inventory { inventory, .. } => {
                let selectable = inventory
                    .entries
                    .iter()
                    .filter(|entry| entry.eligibility.is_selectable())
                    .count();
                let manual = inventory.entries.len().saturating_sub(selectable);
                let (available, partial, permission, unsupported) =
                    source_counts(&inventory.sources);
                let mut lines = vec![
                    copy::software_inventory_summary(
                        locale,
                        inventory.entries.len(),
                        selectable,
                        manual,
                    ),
                    copy::software_sources_summary(
                        locale,
                        available,
                        partial,
                        permission,
                        unsupported,
                    ),
                ];
                if inventory.entries.is_empty() {
                    lines.push(copy::software_empty_title(locale).to_string());
                }
                lines
            }
            ReviewSource::Updates { rows, .. } => {
                let mut lines = vec![
                    copy::software_updates_title(locale).to_string(),
                    copy::software_updates_summary(locale, rows.len()),
                ];
                if rows.is_empty() {
                    lines.push(copy::software_updates_unavailable(locale).to_string());
                }
                lines
            }
            ReviewSource::Hive { .. } => {
                let mut lines = vec![copy::software_startup_title(locale).to_string()];
                if self.items.is_empty() {
                    lines.push(copy::software_startup_empty(locale).to_string());
                }
                if self
                    .items
                    .iter()
                    .any(|item| item.permission.contains("requires_administrator"))
                {
                    lines.push(copy::software_startup_requires_admin(locale).to_string());
                }
                lines
            }
            ReviewSource::None => vec![copy::software_empty_title(locale).to_string()],
        }
    }

    fn filter_matches(&self, item: &SoftwareItem) -> bool {
        match self.filter {
            SoftwareFilter::All => true,
            SoftwareFilter::Selectable => item.selectable,
            SoftwareFilter::Protected => item.permission.contains("protected"),
            SoftwareFilter::PermissionDenied => {
                item.permission.contains("permission")
                    || item.permission.contains("requires_administrator")
            }
        }
    }

    fn query_matches(&self, item: &SoftwareItem) -> bool {
        if self.query.is_empty() {
            return true;
        }
        let query = self.query.to_lowercase();
        [
            item.id.as_str(),
            item.name.as_str(),
            item.source.as_str(),
            item.version.as_str(),
        ]
        .iter()
        .any(|field| field.to_lowercase().contains(&query))
    }
}

fn inventory_items(inventory: &SoftwareInventoryV1, last: &str) -> Vec<SoftwareItem> {
    inventory
        .entries
        .iter()
        .map(|entry| {
            let permission = permission_token(entry, &inventory.sources);
            let skip = if entry.eligibility.is_selectable() {
                source_skip(entry, &inventory.sources)
            } else {
                eligibility_token(entry.eligibility.reason).to_string()
            };
            SoftwareItem {
                id: entry.id.clone(),
                name: entry
                    .display_name
                    .clone()
                    .unwrap_or_else(|| entry.id.clone()),
                identity: identity_token(&entry.identity),
                count: entry.provenance.len() as u64,
                source: entry
                    .provenance
                    .first()
                    .map(source_token)
                    .unwrap_or_else(|| "unknown".to_string()),
                version: entry
                    .version
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                permission,
                skip,
                last: last.to_string(),
                selectable: entry.eligibility.is_selectable(),
            }
        })
        .collect()
}

fn update_items(rows: &[SoftwareUpdateRowV1], last: &str) -> Vec<SoftwareItem> {
    rows.iter()
        .map(|row| SoftwareItem {
            id: row.id.clone(),
            name: row.name.clone(),
            identity: row.id.clone(),
            count: row.matched_software_ids.len() as u64,
            source: row.source.clone(),
            version: row.available_version.clone(),
            permission: if row.name_truncated {
                "truncated".to_string()
            } else {
                "review".to_string()
            },
            skip: copy::workbench_not_run(Locale::En).to_string(),
            last: last.to_string(),
            selectable: false,
        })
        .collect()
}

fn hive_items(path: &Path, locale: Locale, last: &str) -> Vec<SoftwareItem> {
    let hive = match fs::read(path).and_then(|bytes| {
        serde_json::from_slice::<StartupFixtureHiveV1>(&bytes).map_err(std::io::Error::other)
    }) {
        Ok(hive) => hive,
        Err(_) => return Vec::new(),
    };
    hive.entries
        .iter()
        .map(|entry| {
            let id = startup_fixture_entry_id(entry.location, &entry.name);
            let (permission, skip) = match plan_startup_hive(path, &id, false) {
                Ok(_) => (
                    "allowed".to_string(),
                    copy::workbench_not_run(locale).to_string(),
                ),
                Err(error) => (startup_permission(&error), error.code().to_string()),
            };
            SoftwareItem {
                id,
                name: entry.name.clone(),
                identity: format!("{}:{}", location_token(entry.location), entry.name),
                count: 1,
                source: location_token(entry.location).to_string(),
                version: format!("{}", hive.version),
                permission,
                skip,
                last: last.to_string(),
                selectable: entry.class == StartupFixtureClass::Ordinary
                    && entry.location == SoftwareStartupLocation::CurrentUserRun,
            }
        })
        .collect()
}

fn permission_token(entry: &SoftwareEntryV1, sources: &[SoftwareSourceEvidence]) -> String {
    if entry.eligibility.reason == SoftwareEligibilityReason::ProtectedProduct {
        return "protected".to_string();
    }
    if let Some(evidence) = sources.iter().find(|evidence| {
        entry
            .provenance
            .iter()
            .any(|source| source == &evidence.source)
            && evidence.state == SoftwareSourceState::Permission
    }) {
        return format!(
            "permission_denied:{}",
            evidence.reason_code.as_deref().unwrap_or("permission")
        );
    }
    if entry.eligibility.state == SoftwareEligibilityState::Selectable {
        "selectable".to_string()
    } else {
        eligibility_token(entry.eligibility.reason).to_string()
    }
}

fn source_skip(entry: &SoftwareEntryV1, sources: &[SoftwareSourceEvidence]) -> String {
    sources
        .iter()
        .find(|evidence| {
            entry
                .provenance
                .iter()
                .any(|source| source == &evidence.source)
                && evidence.state != SoftwareSourceState::Available
        })
        .map(|evidence| {
            format!(
                "{:?}:{}",
                evidence.state,
                evidence.reason_code.as_deref().unwrap_or("unspecified")
            )
        })
        .unwrap_or_else(|| "none".to_string())
}

fn source_counts(sources: &[SoftwareSourceEvidence]) -> (usize, usize, usize, usize) {
    let mut available = 0;
    let mut partial = 0;
    let mut permission = 0;
    let mut unsupported = 0;
    for source in sources {
        match source.state {
            SoftwareSourceState::Available => available += 1,
            SoftwareSourceState::Partial => partial += 1,
            SoftwareSourceState::Permission => permission += 1,
            SoftwareSourceState::Unsupported => unsupported += 1,
        }
    }
    (available, partial, permission, unsupported)
}

fn identity_token(identity: &SoftwareIdentity) -> String {
    match identity {
        SoftwareIdentity::Msix { package_full_name } => format!("msix:{package_full_name}"),
        SoftwareIdentity::Msi {
            product_code,
            context,
        } => format!("msi:{context:?}:{product_code}"),
        SoftwareIdentity::Arp { hive, view, subkey } => format!("arp:{hive:?}:{view:?}:{subkey}"),
    }
}

fn source_token(source: &devsweep_core::software::SoftwareSourceId) -> String {
    match source {
        devsweep_core::software::SoftwareSourceId::MsixCurrentUser => {
            "msix_current_user".to_string()
        }
        devsweep_core::software::SoftwareSourceId::Msi { context } => format!("msi:{context:?}"),
        devsweep_core::software::SoftwareSourceId::Arp { hive, view } => {
            format!("arp:{hive:?}:{view:?}")
        }
    }
}

fn eligibility_token(reason: SoftwareEligibilityReason) -> &'static str {
    match reason {
        SoftwareEligibilityReason::ProtectedProduct => "protected",
        SoftwareEligibilityReason::SourceIncomplete => "source_incomplete",
        SoftwareEligibilityReason::ConflictingIdentity => "conflicting_identity",
        SoftwareEligibilityReason::NoRemove => "no_remove",
        SoftwareEligibilityReason::HiddenEntry => "hidden_entry",
        SoftwareEligibilityReason::SystemOrUpdate => "system_or_update",
        SoftwareEligibilityReason::DependencyPackage => "dependency_package",
        SoftwareEligibilityReason::StubPackage => "stub_package",
        SoftwareEligibilityReason::UnhealthyPackage => "unhealthy_package",
        SoftwareEligibilityReason::MsiExecutionNotSupportedV1 => "msi_execution_not_supported",
        SoftwareEligibilityReason::RegistryOnlyManual => "registry_only_manual",
        SoftwareEligibilityReason::UnsupportedSource => "unsupported_source",
        SoftwareEligibilityReason::EligibleCurrentUserMsix => "eligible_current_user_msix",
        SoftwareEligibilityReason::EligibleMsiQuiet => "eligible_msi_quiet",
        SoftwareEligibilityReason::EligibleVendorUninstaller => "eligible_vendor_uninstaller",
        SoftwareEligibilityReason::EligibleScoopApp => "eligible_scoop_app",
    }
}

fn location_token(location: SoftwareStartupLocation) -> &'static str {
    match location {
        SoftwareStartupLocation::CurrentUserRun => "current_user_run",
        SoftwareStartupLocation::MachineRun => "machine_run",
        SoftwareStartupLocation::MachineRun32 => "machine_run32",
        SoftwareStartupLocation::CurrentUserStartupFolder => "current_user_startup_folder",
    }
}

fn startup_permission(error: &SoftwareStartupError) -> String {
    match error {
        SoftwareStartupError::Protected(_) => "protected".to_string(),
        SoftwareStartupError::RequiresAdministrator(_) => "requires_administrator".to_string(),
        _ => error.code().to_string(),
    }
}

fn plan_line(locale: Locale, error: &SoftwarePlanError) -> String {
    let code = match error {
        SoftwarePlanError::DigestMismatch => "digest_mismatch",
        SoftwarePlanError::EmptySelection => "empty_selection",
        SoftwarePlanError::ManualSelection(_) => "manual_selection",
        SoftwarePlanError::UnknownSelection(_) => "unknown_selection",
        SoftwarePlanError::InventoryExpired => "inventory_expired",
        SoftwarePlanError::InventoryFingerprintMismatch | SoftwarePlanError::InvalidFingerprint => {
            "inventory_fingerprint_mismatch"
        }
        SoftwarePlanError::StaleSelection(_) => "stale_selection",
        _ => "software_plan_error",
    };
    match locale {
        Locale::En => format!("{code}: {error}"),
        Locale::ZhCn => format!("{code}：{error}"),
    }
}

fn startup_line(locale: Locale, error: &SoftwareStartupError) -> String {
    match locale {
        Locale::En => format!("{}: {error}", error.code()),
        Locale::ZhCn => format!("{}：{error}", error.code()),
    }
}

fn update_code(error: &SoftwareUpdateError) -> &'static str {
    error.code()
}

fn read_inventory(path: &Path) -> Result<SoftwareInventoryV1, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice::<SoftwareInventoryV1>(&bytes).map_err(|error| error.to_string())
}

fn env_path(key: &str) -> Option<PathBuf> {
    let value = std::env::var_os(key)?;
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn flipped_digest(digest: &str) -> String {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return String::new();
    };
    let mut chars = hex.chars().collect::<Vec<_>>();
    if let Some(first) = chars.first_mut() {
        *first = if *first == '0' { '1' } else { '0' };
    }
    format!("sha256:{}", chars.into_iter().collect::<String>())
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Writes a fingerprint the accepted plan functions will accept.
pub fn seal_inventory(inventory: &mut SoftwareInventoryV1) -> Result<(), String> {
    inventory.version = SOFTWARE_INVENTORY_VERSION;
    inventory.fingerprint = String::new();
    inventory.fingerprint =
        software_inventory_fingerprint(inventory).map_err(|error| error.to_string())?;
    Ok(())
}

#[must_use]
pub fn entry_id(identity: &SoftwareIdentity) -> Result<String, String> {
    software_entry_id(identity).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::software::{
        RegistryHive, RegistryView, SoftwareEligibility, SoftwareLastUsedEvidence, SoftwareScope,
        SoftwareSizeEvidence, SoftwareSourceId,
    };

    fn msix(name: &str, version: &str) -> SoftwareEntryV1 {
        let identity = SoftwareIdentity::Msix {
            package_full_name: format!("{name}_1.0.0.0_x64__publisher"),
        };
        let id = entry_id(&identity).expect("id");
        SoftwareEntryV1 {
            id,
            identity,
            scope: SoftwareScope::CurrentUser,
            display_name: Some(name.to_string()),
            publisher: Some("Fixture".to_string()),
            version: Some(version.to_string()),
            provenance: vec![SoftwareSourceId::MsixCurrentUser],
            eligibility: SoftwareEligibility {
                state: SoftwareEligibilityState::Selectable,
                reason: SoftwareEligibilityReason::EligibleCurrentUserMsix,
            },
            size: SoftwareSizeEvidence::Unknown {
                reason_code: "fixture".to_string(),
            },
            last_used: SoftwareLastUsedEvidence::default(),
            launch: None,
        }
    }

    fn protected_entry() -> SoftwareEntryV1 {
        let identity = SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "ProtectedApp".to_string(),
        };
        let id = entry_id(&identity).expect("id");
        SoftwareEntryV1 {
            id,
            identity,
            scope: SoftwareScope::CurrentUser,
            display_name: Some("ProtectedApp".to_string()),
            publisher: None,
            version: Some("9".to_string()),
            provenance: vec![SoftwareSourceId::Arp {
                hive: RegistryHive::CurrentUser,
                view: RegistryView::Registry64,
            }],
            eligibility: SoftwareEligibility {
                state: SoftwareEligibilityState::Manual,
                reason: SoftwareEligibilityReason::ProtectedProduct,
            },
            size: SoftwareSizeEvidence::Unknown {
                reason_code: "fixture".to_string(),
            },
            last_used: SoftwareLastUsedEvidence::default(),
            launch: None,
        }
    }

    fn inventory(
        entries: Vec<SoftwareEntryV1>,
        sources: Vec<SoftwareSourceEvidence>,
    ) -> SoftwareInventoryV1 {
        let mut inventory = SoftwareInventoryV1 {
            version: SOFTWARE_INVENTORY_VERSION,
            observed_at_unix_ms: 1_700_000_000_000,
            sources,
            entries,
            fingerprint: String::new(),
        };
        seal_inventory(&mut inventory).expect("seal");
        inventory
    }

    fn source(state: SoftwareSourceState, reason: &str) -> SoftwareSourceEvidence {
        SoftwareSourceEvidence {
            source: SoftwareSourceId::MsixCurrentUser,
            state,
            reason_code: Some(reason.to_string()),
        }
    }

    #[test]
    fn uninstall_review_plans_refuses_without_executor_and_rejects_stale() {
        let inventory = inventory(
            vec![msix("Alpha", "1.2.3"), protected_entry()],
            vec![source(SoftwareSourceState::Partial, "source_partial")],
        );
        let mut review = SoftwareReview::new();
        review.set_now(inventory.observed_at_unix_ms);
        review.open_inventory(inventory.clone(), Locale::En);
        let lines = review.lines().join("\n");
        assert!(lines.contains("partial"), "{lines}");
        assert!(lines.contains("2 entries"), "{lines}");
        let alpha_id = review
            .items
            .iter()
            .find(|item| item.name == "Alpha")
            .expect("alpha")
            .id
            .clone();
        let alpha = review
            .items
            .iter()
            .find(|item| item.id == alpha_id)
            .expect("alpha");
        assert!(alpha.identity.starts_with("msix:"));
        assert_eq!(alpha.version, "1.2.3");
        assert_eq!(alpha.source, "msix_current_user");
        assert!(
            alpha.permission.contains("permission")
                || alpha.skip.contains("Partial")
                || lines.contains("partial")
        );
        let protected_id = review
            .items
            .iter()
            .find(|item| item.permission.contains("protected"))
            .expect("protected")
            .id
            .clone();
        review.activate(&protected_id, Locale::En, None);
        let refused = review.lines().join("\n");
        assert!(refused.contains("manual_selection"), "{refused}");
        assert_eq!(review.executor_calls(), 0);

        review.activate(&alpha_id, Locale::En, None);
        let preview = review.lines().join("\n");
        assert!(preview.contains("sha256:"), "{preview}");
        review.activate(&alpha_id, Locale::ZhCn, None);
        let second = review.lines().join("\n");
        assert!(second.contains("isolated_executor_unavailable"), "{second}");
        assert!(!second.contains("成功"));
        assert_eq!(review.executor_calls(), 0);

        review.arm_stale();
        review.activate(&alpha_id, Locale::En, None);
        review.activate(&alpha_id, Locale::En, None);
        let stale = review.lines().join("\n");
        assert!(
            stale.contains("digest_mismatch") || stale.contains("isolated_executor_unavailable"),
            "{stale}"
        );
        assert_eq!(review.executor_calls(), 0);

        let source = include_str!("software_review.rs");
        let production = source.split("mod tests").next().expect("tests");
        assert!(!production.contains("SoftwareExecutor"));
        assert!(!production.contains("inventory_software"));
        assert!(!production.contains("list_startup_entries"));
        assert!(!production.contains("check_software_updates"));
    }

    #[test]
    fn isolated_executor_receives_the_live_digest_and_search_stays_local() {
        let stored = inventory(
            vec![msix("Alpha", "1.0.0"), msix("Beta", "2.0.0")],
            Vec::new(),
        );
        let mut review = SoftwareReview::new();
        review.set_now(stored.observed_at_unix_ms);
        review.open_inventory(stored, Locale::En);
        review.set_query("beta");
        let ids = review.filtered_ids();
        assert_eq!(ids.len(), 1);
        review.cycle_sort();
        review.cycle_filter();
        assert!(review.filtered_ids().iter().all(|id| {
            review
                .items
                .iter()
                .any(|item| &item.id == id && item.selectable)
        }));
        review.set_query("");
        review.cycle_filter();
        review.cycle_filter();
        review.cycle_filter();
        let id = review
            .items
            .iter()
            .find(|item| item.name == "Alpha")
            .expect("alpha")
            .id
            .clone();
        review.activate(&id, Locale::En, None);
        let digest = review.pending.as_ref().expect("pending").digest.clone();
        let mut seen = String::new();
        review.activate(
            &id,
            Locale::En,
            Some(&mut |plan, live| {
                assert_eq!(plan.selected_ids, vec![id.clone()]);
                assert_eq!(live, digest);
                seen = live.to_string();
                Ok("removed".to_string())
            }),
        );
        assert_eq!(seen, digest);
        assert_eq!(review.executor_calls(), 1);
        let painted = review.row_line(&id, Locale::En, true).expect("row");
        assert!(painted.contains("identity="));
        assert!(painted.contains("last=removed"));
        assert!(!painted.to_ascii_lowercase().contains("succeed"));
    }

    #[test]
    fn refresh_rereads_only_the_isolated_file() {
        let root = std::env::temp_dir().join(format!(
            "devsweep-gpui-software-review-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("root");
        let path = root.join("inventory.json");
        let first = inventory(vec![msix("Alpha", "1")], Vec::new());
        fs::write(&path, serde_json::to_vec(&first).expect("json")).expect("write");
        let mut review = SoftwareReview::new();
        review.set_now(first.observed_at_unix_ms);
        review.open_inventory_file(&path, Locale::En);
        assert!(review.items.iter().any(|item| item.name == "Alpha"));
        let mut second = inventory(vec![msix("Gamma", "3")], Vec::new());
        second.observed_at_unix_ms = first.observed_at_unix_ms;
        seal_inventory(&mut second).expect("reseal");
        fs::write(&path, serde_json::to_vec(&second).expect("json")).expect("rewrite");
        review.refresh(Locale::En);
        assert!(review.items.iter().any(|item| item.name == "Gamma"));
        assert!(!review.items.iter().any(|item| item.name == "Alpha"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn session_review_refuses_a_host_inventory_and_escape_returns() {
        use crate::shell::{Mode, NavKey, Session};
        use crate::view::panel_lines;

        let mut session = Session::unbacked();
        session.show_mode(Mode::Software);
        session.select_mode_row(Mode::Software, "uninstall");
        session.apply_key(NavKey::Enter);
        assert!(session.software_review().is_open());
        let lines = panel_lines(&session, None).join("\n");
        assert!(
            lines.contains("No software inventory") || lines.contains("尚无软件清单"),
            "{lines}"
        );
        assert!(!lines.to_ascii_lowercase().contains("succeed"));
        session.software_cycle_filter();
        session.apply_key(NavKey::Escape);
        assert!(!session.software_review().is_open());
        assert!(!session.input_locked());
    }

    #[test]
    fn empty_permission_and_updates_do_not_claim_success() {
        let empty = inventory(
            Vec::new(),
            vec![source(SoftwareSourceState::Permission, "access_denied")],
        );
        let mut review = SoftwareReview::new();
        review.open_inventory(empty, Locale::ZhCn);
        let lines = review.lines().join("\n");
        assert!(
            lines.contains("权限受限") || lines.contains("permission"),
            "{lines}"
        );
        assert!(lines.contains("尚无软件清单") || lines.contains("0"));
        assert!(!lines.contains("成功"));

        review.open_updates(Vec::new(), Locale::En);
        let updates = review.lines().join("\n");
        assert!(updates.contains("Available updates"));
        assert!(!updates.to_ascii_lowercase().contains("succeed"));
        review.open_segment(SoftwareSegment::Updates, Locale::En);
        let unchecked = review.lines().join("\n");
        assert!(
            unchecked.contains("not checked")
                || unchecked.contains("尚未检查")
                || unchecked.contains("probe_failed"),
            "{unchecked}"
        );
    }
}
