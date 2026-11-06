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
    SoftwareStartupListV1, SoftwareStartupLocation, SoftwareStartupState, SoftwareUpdateError,
    SoftwareUpdateRowV1, SoftwareUpdatesV1, StartupFixtureClass, StartupFixtureHiveV1,
    build_selection_plan, plan_software_update, plan_startup_hive, preview_selection_plan_live,
    preview_startup_hive, software_entry_id, software_inventory_fingerprint,
    startup_fixture_entry_id,
};

use crate::copy::{self, Locale};
use crate::workbench::SoftwareSegment;

pub const INVENTORY_ENV: &str = "DEVSWEEP_SOFTWARE_INVENTORY";
pub const UPDATES_FILE_ENV: &str = "DEVSWEEP_SOFTWARE_UPDATES_FILE";
pub const UPDATE_PROGRAM_ENV: &str = "DEVSWEEP_SOFTWARE_UPDATE_PROGRAM";
pub const HIVE_ENV: &str = "DEVSWEEP_SOFTWARE_STARTUP_HIVE";

/// Isolated uninstall executor injected by tests; the window passes none.
pub type UninstallExecutor<'a> =
    dyn FnMut(&SoftwareSelectionPlanV1, &str) -> Result<String, String> + 'a;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftwareSort {
    Name,
    Source,
    Version,
    /// Largest qualified size first; rows without a size sort last.
    Size,
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
    /// Qualified install size from inventory evidence. `None` when the
    /// source reports no size; it is never painted as zero.
    pub size: Option<SoftwareSize>,
    /// Updates only: the installed version the available one replaces.
    pub installed_version: Option<String>,
}

/// How much an install size can be trusted. The window always shows the
/// qualifier next to the number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftwareSize {
    /// Reported by the installer (Programs and Features estimate).
    Estimate(u64),
    /// Measured from the installed location.
    Measured(u64),
    /// A partial walk; the true size is at least this.
    AtLeast(u64),
}

impl SoftwareSize {
    fn from_evidence(evidence: &devsweep_core::software::SoftwareSizeEvidence) -> Option<Self> {
        use devsweep_core::software::{SoftwareSizeBasis, SoftwareSizeEvidence};
        match evidence {
            SoftwareSizeEvidence::Available {
                value_bytes, basis, ..
            } => Some(match basis {
                SoftwareSizeBasis::ReportedEstimate => Self::Estimate(*value_bytes),
                SoftwareSizeBasis::MeasuredInstalledLocation
                | SoftwareSizeBasis::MeasuredDirectory => Self::Measured(*value_bytes),
            }),
            SoftwareSizeEvidence::Partial {
                lower_bound_bytes, ..
            } => Some(Self::AtLeast(*lower_bound_bytes)),
            SoftwareSizeEvidence::Unknown { .. } => None,
        }
    }

    #[must_use]
    pub fn bytes(self) -> u64 {
        match self {
            Self::Estimate(value) | Self::Measured(value) | Self::AtLeast(value) => value,
        }
    }
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
    /// Read-only host startup list. Toggles preview only; the window never writes.
    Startup {
        list: SoftwareStartupListV1,
    },
}

/// Result of one read-only host load, produced on a worker thread.
#[derive(Clone, Debug)]
pub enum SoftwareHostLoad {
    Inventory(Result<SoftwareInventoryV1, String>),
    Updates(SoftwareUpdatesV1),
    Startup(SoftwareStartupListV1),
}

/// Reads one segment from the host. Read-only: inventory enumerates installed
/// software, updates runs the bounded `winget upgrade` probe, and startup lists
/// the four fixed sources. Blocking; never call it on the UI thread.
#[must_use]
pub fn load_host(segment: SoftwareSegment) -> Option<SoftwareHostLoad> {
    match segment {
        SoftwareSegment::Uninstall => Some(SoftwareHostLoad::Inventory(
            devsweep_core::software::inventory_software(
                devsweep_core::software::SoftwareInventorySource::All,
                None,
            )
            .map_err(|error| error.to_string()),
        )),
        SoftwareSegment::Updates => {
            let inventory = devsweep_core::software::inventory_software(
                devsweep_core::software::SoftwareInventorySource::All,
                None,
            )
            .ok();
            Some(SoftwareHostLoad::Updates(
                devsweep_core::software::check_software_updates(inventory.as_ref(), None),
            ))
        }
        SoftwareSegment::Startup => Some(SoftwareHostLoad::Startup(
            devsweep_core::software::list_startup_entries(),
        )),
        SoftwareSegment::Leftovers => None,
    }
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
    loading: bool,
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
            loading: false,
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

    /// True while a host load for the open segment is in flight.
    #[must_use]
    pub fn loading(&self) -> bool {
        self.loading
    }

    /// True when an isolated fixture env var names this segment's source, so
    /// opening it stays synchronous and never reads the host.
    #[must_use]
    pub fn has_fixture(segment: SoftwareSegment) -> bool {
        match segment {
            SoftwareSegment::Uninstall => env_path(INVENTORY_ENV).is_some(),
            SoftwareSegment::Updates => {
                env_path(UPDATES_FILE_ENV).is_some() || env_path(UPDATE_PROGRAM_ENV).is_some()
            }
            SoftwareSegment::Startup => env_path(HIVE_ENV).is_some(),
            SoftwareSegment::Leftovers => true,
        }
    }

    /// Opens `segment` in its loading state before a worker reads the host.
    pub fn begin_host_load(&mut self, segment: SoftwareSegment, locale: Locale) {
        self.segment = Some(segment);
        self.source = ReviewSource::None;
        self.items.clear();
        self.pending = None;
        self.loading = true;
        self.lines = vec![copy::software_loading(locale, segment_key(segment)).to_string()];
    }

    /// Installs a finished host load. The caller drops loads for a segment
    /// that is no longer open before they reach this method.
    pub fn apply_host_load(&mut self, load: SoftwareHostLoad, locale: Locale) {
        self.loading = false;
        self.pending = None;
        match load {
            SoftwareHostLoad::Inventory(Ok(inventory)) => {
                self.segment = Some(SoftwareSegment::Uninstall);
                self.source = ReviewSource::Inventory {
                    path: None,
                    inventory,
                };
                self.rebuild_items(locale);
            }
            SoftwareHostLoad::Inventory(Err(error)) => {
                self.segment = Some(SoftwareSegment::Uninstall);
                self.source = ReviewSource::None;
                self.items.clear();
                self.lines = vec![
                    copy::software_empty_title(locale).to_string(),
                    format!("inventory_unavailable: {error}"),
                ];
            }
            SoftwareHostLoad::Updates(SoftwareUpdatesV1::Available { rows, .. }) => {
                self.segment = Some(SoftwareSegment::Updates);
                self.source = ReviewSource::Updates { path: None, rows };
                self.rebuild_items(locale);
            }
            SoftwareHostLoad::Updates(SoftwareUpdatesV1::Unavailable { reason_code, .. }) => {
                self.segment = Some(SoftwareSegment::Updates);
                self.source = ReviewSource::None;
                self.items.clear();
                self.lines = vec![
                    copy::software_updates_unavailable(locale).to_string(),
                    format!("probe_failed: {reason_code:?}"),
                ];
            }
            SoftwareHostLoad::Startup(list) => {
                self.segment = Some(SoftwareSegment::Startup);
                self.source = ReviewSource::Startup { list };
                self.rebuild_items(locale);
            }
        }
    }

    /// True when the open review came from the host rather than a fixture.
    #[must_use]
    pub fn host_backed(&self) -> bool {
        matches!(
            self.source,
            ReviewSource::Inventory { path: None, .. }
                | ReviewSource::Updates { path: None, .. }
                | ReviewSource::Startup { .. }
        )
    }

    /// Open segment, when any.
    #[must_use]
    pub fn segment(&self) -> Option<SoftwareSegment> {
        self.segment
    }

    #[must_use]
    pub fn sort(&self) -> SoftwareSort {
        self.sort
    }

    #[must_use]
    pub fn filter(&self) -> SoftwareFilter {
        self.filter
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Escape dismisses pending authority without returning to an introduction.
    pub fn cancel_review(&mut self, locale: Locale) -> bool {
        if self.pending.take().is_none() {
            return false;
        }
        self.lines = self.summary_lines(locale);
        true
    }

    /// One reviewed item by id.
    #[must_use]
    pub fn item(&self, id: &str) -> Option<&SoftwareItem> {
        self.items.iter().find(|item| item.id == id)
    }

    #[must_use]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Id of the uninstall plan awaiting a second confirmation, if any.
    #[must_use]
    pub fn pending_id(&self) -> Option<&str> {
        self.pending.as_ref().map(|pending| pending.id.as_str())
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
            SoftwareSort::Version => SoftwareSort::Size,
            SoftwareSort::Size => SoftwareSort::Name,
        };
    }

    pub fn cycle_filter(&mut self) {
        if self.segment != Some(SoftwareSegment::Uninstall) {
            self.filter = SoftwareFilter::All;
            return;
        }
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
            ReviewSource::Startup { .. } | ReviewSource::None => None,
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

    /// Rows that pass the current search and filter.
    #[must_use]
    pub fn visible_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| self.filter_matches(item) && self.query_matches(item))
            .count()
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
                SoftwareSort::Size => right
                    .size
                    .map(SoftwareSize::bytes)
                    .cmp(&left.size.map(SoftwareSize::bytes)),
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
            "{mark}{} | {} | identity={} | count={} | source={} | version={} | permission={} | skip={} | last={last}",
            item.name,
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
        executor: Option<&mut UninstallExecutor<'_>>,
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
        mut executor: Option<&mut UninstallExecutor<'_>>,
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
        if let ReviewSource::Startup { list } = &self.source {
            let enabled = list
                .entries
                .iter()
                .find(|entry| entry.id == id)
                .map(|entry| entry.state == SoftwareStartupState::Enabled);
            self.preview_host_startup(id, enabled, locale);
            return;
        }
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

    /// Plans and previews one host startup toggle. A second activation is
    /// refused: this window never writes the host approval value.
    fn preview_host_startup(&mut self, id: &str, enabled: Option<bool>, locale: Locale) {
        let Some(enabled) = enabled else {
            self.lines = vec![startup_line(
                locale,
                &SoftwareStartupError::StartupSourceUnavailable,
            )];
            self.pending = None;
            return;
        };
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.id == id)
        {
            self.lines = vec![
                "isolated_executor_unavailable".to_string(),
                copy::software_isolated_executor_unavailable(locale).to_string(),
            ];
            return;
        }
        let plan = match devsweep_core::software::plan_startup_toggle(id, !enabled) {
            Ok(plan) => plan,
            Err(error) => {
                self.lines = vec![startup_line(locale, &error)];
                self.pending = None;
                return;
            }
        };
        match devsweep_core::software::preview_startup_toggle(&plan) {
            Ok(preview) => {
                self.lines = vec![
                    format!(
                        "{id} | {:?} -> {:?} | {}",
                        preview.before_state, preview.after_state, preview.digest
                    )
                    .to_ascii_lowercase(),
                ];
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
            Err(error) => {
                self.lines = vec![startup_line(locale, &error)];
                self.pending = None;
            }
        }
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
            ReviewSource::Startup { list } => startup_items(list, &last),
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
                    lines.push(
                        copy::pick(locale, "No updates were reported.", "未发现可用更新。")
                            .to_string(),
                    );
                }
                lines
            }
            ReviewSource::Hive { .. } | ReviewSource::Startup { .. } => {
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
                size: SoftwareSize::from_evidence(&entry.size),
                installed_version: None,
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
            size: None,
            installed_version: Some(row.installed_version.clone()),
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
            // Plan and preview only read the fixture. The preview's
            // before_state is the observed state; a refused entry stays unknown.
            let (permission, skip, state) = match plan_startup_hive(path, &id, false) {
                Ok(plan) => (
                    "allowed".to_string(),
                    copy::workbench_not_run(locale).to_string(),
                    preview_startup_hive(path, &plan)
                        .map(|preview| preview.before_state)
                        .unwrap_or(SoftwareStartupState::Unknown),
                ),
                Err(error) => (
                    startup_permission(&error),
                    error.code().to_string(),
                    SoftwareStartupState::Unknown,
                ),
            };
            SoftwareItem {
                id,
                name: entry.name.clone(),
                identity: format!("{}:{}", location_token(entry.location), entry.name),
                count: 1,
                source: location_token(entry.location).to_string(),
                version: startup_state_token(state).to_string(),
                permission,
                skip,
                last: last.to_string(),
                selectable: entry.class == StartupFixtureClass::Ordinary
                    && entry.location == SoftwareStartupLocation::CurrentUserRun,
                size: None,
                installed_version: None,
            }
        })
        .collect()
}

fn startup_items(list: &SoftwareStartupListV1, last: &str) -> Vec<SoftwareItem> {
    list.entries
        .iter()
        .map(|entry| {
            let state = startup_state_token(entry.state);
            let permission = match entry.toggle {
                devsweep_core::software::SoftwareStartupToggle::Allowed => "allowed",
                devsweep_core::software::SoftwareStartupToggle::RequiresAdministrator => {
                    "requires_administrator"
                }
            };
            SoftwareItem {
                id: entry.id.clone(),
                name: entry.name.clone(),
                identity: format!("{}:{}", location_token(entry.location), entry.name),
                count: 1,
                source: location_token(entry.location).to_string(),
                version: state.to_string(),
                permission: permission.to_string(),
                skip: last.to_string(),
                last: last.to_string(),
                selectable: false,
                size: None,
                installed_version: None,
            }
        })
        .collect()
}

fn startup_state_token(state: SoftwareStartupState) -> &'static str {
    match state {
        SoftwareStartupState::Enabled => "enabled",
        SoftwareStartupState::Disabled => "disabled",
        SoftwareStartupState::Unknown => "unknown",
    }
}

fn segment_key(segment: SoftwareSegment) -> &'static str {
    match segment {
        SoftwareSegment::Uninstall => "uninstall",
        SoftwareSegment::Updates => "updates",
        SoftwareSegment::Startup => "startup",
        SoftwareSegment::Leftovers => "leftovers",
    }
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
    fn install_sizes_keep_their_qualifier_and_sort_largest_first() {
        use devsweep_core::software::{SoftwareSizeBasis, SoftwareSizeSourceCode};
        let mut reported = msix("Reported", "1");
        reported.size = SoftwareSizeEvidence::Available {
            value_bytes: 300,
            basis: SoftwareSizeBasis::ReportedEstimate,
            source_code: SoftwareSizeSourceCode::ArpEstimatedSizeKib,
            observed_at_unix_ms: 1,
        };
        let mut partial = msix("Partial", "1");
        partial.size = SoftwareSizeEvidence::Partial {
            lower_bound_bytes: 900,
            basis: SoftwareSizeBasis::MeasuredInstalledLocation,
            source_code: SoftwareSizeSourceCode::ArpEstimatedSizeKib,
            reason_code: "fixture".to_string(),
            observed_at_unix_ms: 1,
        };
        let unknown = msix("Unknown", "1");
        let inventory = inventory(
            vec![reported, partial, unknown],
            vec![source(SoftwareSourceState::Available, "ok")],
        );
        let mut review = SoftwareReview::new();
        review.set_now(inventory.observed_at_unix_ms);
        review.open_inventory(inventory, Locale::En);
        let size_of = |name: &str| {
            review
                .items
                .iter()
                .find(|item| item.name == name)
                .expect("item")
                .size
        };
        assert_eq!(size_of("Reported"), Some(SoftwareSize::Estimate(300)));
        assert_eq!(size_of("Partial"), Some(SoftwareSize::AtLeast(900)));
        assert_eq!(size_of("Unknown"), None, "unknown is never painted as zero");

        while review.sort() != SoftwareSort::Size {
            review.cycle_sort();
        }
        let names = review
            .filtered_ids()
            .iter()
            .map(|id| review.item(id).expect("item").name.clone())
            .collect::<Vec<_>>();
        assert_eq!(names, ["Partial", "Reported", "Unknown"]);
        assert_eq!(review.visible_count(), 3);
        review.cycle_sort();
        assert_eq!(review.sort(), SoftwareSort::Name);
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

        let source = crate::source_lf(include_str!("software_review.rs"));
        let production = source.split("mod tests").next().expect("tests");
        assert!(!production.contains("SoftwareExecutor"));
        // The window may read the host, but only through `load_host`, which a
        // worker runs after an explicit segment open. No host mutation API is
        // referenced: uninstall, startup writes, and update installs stay in
        // the CLI's plan/preview/confirm path.
        let load_host = production
            .split("pub fn load_host(")
            .nth(1)
            .and_then(|rest| rest.split("\n}\n").next())
            .expect("load_host body");
        let outside = production.replace(load_host, "");
        for read in [
            "inventory_software(",
            "list_startup_entries(",
            "check_software_updates(",
        ] {
            assert!(load_host.contains(read), "{read} belongs in load_host");
            assert!(!outside.contains(read), "{read} outside load_host");
        }
        for write in [
            "execute_startup_toggle",
            "set_startup_enabled",
            "execute_software_update",
            "SoftwareExecutionRequest",
            "ValidatedSoftwareAction",
            "execute_startup_restore",
        ] {
            assert!(!production.contains(write), "window must not call {write}");
        }
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
        // Screen readers hear the software name first; the focus mark and the
        // id, identity and last fields stay for the UIA harnesses.
        let name = review.item(&id).expect("item").name.clone();
        assert!(
            painted.starts_with(&format!("> {name} | {id} | ")),
            "{painted}"
        );
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
    fn session_empty_inventory_and_escape_retain_the_workspace() {
        use crate::shell::{Mode, NavKey, Session};
        use crate::view::panel_lines;

        let mut session = Session::unbacked();
        session.show_mode(Mode::Software);
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
        assert_eq!(
            session.software_review().segment(),
            Some(SoftwareSegment::Uninstall)
        );
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

    #[test]
    fn update_rows_keep_the_installed_version_they_replace() {
        let row = SoftwareUpdateRowV1 {
            id: "Git.Git".to_string(),
            name: "Git".to_string(),
            name_truncated: false,
            installed_version: "2.46.0".to_string(),
            available_version: "2.47.1".to_string(),
            source: "winget".to_string(),
            matched_software_ids: Vec::new(),
        };
        let mut review = SoftwareReview::new();
        review.open_updates(vec![row], Locale::En);
        assert_eq!(review.segment(), Some(SoftwareSegment::Updates));
        let item = review.item("Git.Git").expect("update row");
        assert_eq!(item.version, "2.47.1");
        assert_eq!(item.installed_version.as_deref(), Some("2.46.0"));
        assert!(!item.selectable, "an update row is review-only");
    }

    #[test]
    fn fixture_hive_rows_carry_the_observed_startup_state() {
        let dir =
            std::env::temp_dir().join(format!("devsweep-gpui-hive-state-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("hive.json");
        let hive = serde_json::json!({
            "version": 1,
            "entries": [
                {"name": "On", "location": "current_user_run", "class": "ordinary",
                 "run_value": [65], "approval": [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]},
                {"name": "Off", "location": "current_user_run", "class": "ordinary",
                 "run_value": [66], "approval": [3, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]},
                {"name": "Guard", "location": "machine_run", "class": "security",
                 "run_value": [67]}
            ]
        });
        fs::write(&path, serde_json::to_vec(&hive).expect("json")).expect("write hive");
        let before = fs::read(&path).expect("read hive");

        let mut review = SoftwareReview::new();
        review.open_hive(&path, Locale::En);
        let state = |name: &str| {
            let id = startup_fixture_entry_id(
                if name == "Guard" {
                    SoftwareStartupLocation::MachineRun
                } else {
                    SoftwareStartupLocation::CurrentUserRun
                },
                name,
            );
            review.item(&id).expect("hive row").version.clone()
        };
        assert_eq!(state("On"), "enabled");
        assert_eq!(state("Off"), "disabled");
        assert_eq!(state("Guard"), "unknown", "a refused entry is not guessed");
        assert_eq!(
            fs::read(&path).expect("reread"),
            before,
            "reading never writes"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
