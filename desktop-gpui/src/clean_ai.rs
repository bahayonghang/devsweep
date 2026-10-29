//! AI-tool review subview inside Clean (C11-R5).
//!
//! The subview is a pure projection over one accepted C15 observation plus the
//! user's per-category selection. It never scans, digests, or moves a file by
//! itself: scanning, live digest computation, and execution are delegated to
//! the shared core services through the wrappers in [`crate::clean`]. It also
//! never resolves a provider home from the environment; the shell passes the
//! explicit home that the shared core resolver produced, so an isolated test
//! environment and the production environment take the same path.
//!
//! Sensitive paths (credentials, memory, settings, skills, mcp, models,
//! projects) stay visible in their own section, are marked, and can never enter
//! a selection. The core refuses them again on plan, preview, and execute.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
};

use devsweep_core::ai::{
    AiClass, AiDisposition, AiError, AiExecuteReportV1, AiObservationV1, AiPlanV1, AiProvider,
    AiRecoveryBundle, AiRecoveryListV1,
};

use crate::{
    clean_workbench::bounded_detail,
    copy::{self, Locale},
    shell::CleanOutcomeNotice,
};

/// Sensitive names that never enter a cleanup selection. Display mirror of the
/// C15 deny list.
const SENSITIVE_NAMES: [&str; 7] = [
    "settings",
    "credentials",
    "memory",
    "skills",
    "mcp",
    "models",
    "projects",
];

/// Ordered AI review sections. Cache, versions, sessions, and worktrees are
/// separate plannable categories; the sensitive section collects every
/// protected denied-name row and is never plannable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AiCategory {
    /// `regenerable_cache` rows.
    Cache,
    /// `superseded_version` rows.
    Versions,
    /// `session_archive` rows.
    Sessions,
    /// `worktree` rows.
    Worktrees,
    /// Protected denied-name rows from any class. Never selectable.
    Sensitive,
}

impl AiCategory {
    /// Every explicit section, in render order.
    pub const ALL: [Self; 5] = [
        Self::Cache,
        Self::Versions,
        Self::Sessions,
        Self::Worktrees,
        Self::Sensitive,
    ];

    /// Stable English id used by element ids and category counts.
    pub fn id(self) -> &'static str {
        match self {
            Self::Cache => "cache",
            Self::Versions => "versions",
            Self::Sessions => "sessions",
            Self::Worktrees => "worktrees",
            Self::Sensitive => "sensitive",
        }
    }

    /// The C15 class this section reviews. The sensitive section has none.
    pub fn class(self) -> Option<AiClass> {
        match self {
            Self::Cache => Some(AiClass::RegenerableCache),
            Self::Versions => Some(AiClass::SupersededVersion),
            Self::Sessions => Some(AiClass::SessionArchive),
            Self::Worktrees => Some(AiClass::Worktree),
            Self::Sensitive => None,
        }
    }
}

/// Ordered AI subview phases. Rendering never starts a scan or an execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSubviewPhase {
    /// Nothing scanned for this subview yet. One explicit action starts it.
    Landing,
    /// A worker scan is in flight.
    Scanning,
    /// An accepted observation is under review.
    Review,
    /// A worker execution is in flight.
    Cleaning,
    /// The execution reached a terminal result.
    Completed,
    /// The scan or the execution could not complete.
    Failed,
}

impl AiSubviewPhase {
    pub fn id(self) -> &'static str {
        match self {
            Self::Landing => "ai-landing",
            Self::Scanning => "ai-scanning",
            Self::Review => "ai-review",
            Self::Cleaning => "ai-cleaning",
            Self::Completed => "ai-completed",
            Self::Failed => "ai-failed",
        }
    }
}

/// One reviewable AI row derived from the accepted observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiRowView {
    /// Stable core id `ai:<provider>:<class>:<relative>`.
    pub id: String,
    /// C15 class of the row.
    pub class: AiClass,
    /// `/`-separated path relative to the provider home.
    pub relative_path: String,
    /// Core disposition. Only `Candidate` rows can be selected.
    pub disposition: AiDisposition,
    /// Locale-neutral core reason code.
    pub reason: String,
    /// True for protected denied-name rows. Never selectable.
    pub sensitive: bool,
    /// Display section, or `None` for tool rows that are not cleanup targets.
    pub category: Option<AiCategory>,
}

impl AiRowView {
    /// Returns whether the row can enter a cleanup selection. A sensitive row
    /// can never be selectable, whatever its core disposition says.
    pub fn selectable(&self) -> bool {
        !self.sensitive && self.disposition == AiDisposition::Candidate
    }
}

/// True when any path component (or its dot stem) is a denied sensitive name.
fn sensitive_path(relative: &str) -> bool {
    relative.split('/').any(|segment| {
        let stem = segment
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(segment);
        SENSITIVE_NAMES.contains(&segment) || SENSITIVE_NAMES.contains(&stem)
    })
}

/// Display section of one observed row. `None` rows are not cleanup targets
/// and stay out of the review surface.
fn category_of(class: AiClass, sensitive: bool) -> Option<AiCategory> {
    if sensitive {
        return Some(AiCategory::Sensitive);
    }
    match class {
        AiClass::RegenerableCache => Some(AiCategory::Cache),
        AiClass::SupersededVersion => Some(AiCategory::Versions),
        AiClass::SessionArchive => Some(AiCategory::Sessions),
        AiClass::Worktree => Some(AiCategory::Worktrees),
        AiClass::InstalledTool => None,
    }
}

/// One converged worker scan result carried through the mailbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanAiScanPayload {
    /// Accepted observation, when the scan produced one.
    pub observation: Option<AiObservationV1>,
    /// Stable failure when no observation was produced.
    pub error: Option<AiError>,
}

/// One converged worker execution result carried through the mailbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanAiExecutePayload {
    /// Execute report when the core reached a terminal outcome.
    pub report: Option<AiExecuteReportV1>,
    /// Stable failure when no report was produced.
    pub error: Option<AiError>,
    /// Whether cooperative cancellation was requested for this run.
    pub canceled: bool,
}

/// One recovery listing result. Listing never restores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanAiRecoveryPayload {
    /// Accepted listing, when the read succeeded.
    pub list: Option<AiRecoveryListV1>,
    /// Stable failure when no listing was produced.
    pub error: Option<AiError>,
}

/// One restore-plan result. Planning never copies payload bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanAiRestorePayload {
    /// Recovery bundle id the plan was built for.
    pub bundle_id: String,
    /// Restore plan, when the bundle validated.
    pub plan: Option<AiPlanV1>,
    /// Stable failure when the plan was refused.
    pub error: Option<AiError>,
}

/// The live digest and selected subset plan shown by the confirm surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiPreviewState {
    /// Category (and C15 class) the digest was computed for.
    pub class: AiCategory,
    /// Live `sha256:` digest over the selected subset plan.
    pub digest: String,
    /// Selected target count the digest covers.
    pub count: usize,
    /// The selected subset of the class plan.
    pub plan: AiPlanV1,
    /// Selected id snapshot the digest was computed for.
    pub selected_snapshot: BTreeSet<String>,
    /// Accepted-observation epoch the digest was computed for.
    pub observation_epoch: u64,
}

/// Inputs handed to the execute worker after all shell-level gates pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiExecutionStart {
    /// Category the confirmed plan belongs to.
    pub class: AiCategory,
    /// Selected subset plan.
    pub plan: AiPlanV1,
    /// The stored live preview digest.
    pub digest: String,
}

/// Pure AI review projection. It performs no I/O at all: scan, digest, and
/// execute calls happen in the shell's worker helpers.
#[derive(Clone, Debug)]
pub struct CleanAiView {
    open: bool,
    provider: AiProvider,
    phase: AiSubviewPhase,
    category: AiCategory,
    home: Option<PathBuf>,
    observation: Option<AiObservationV1>,
    rows: Vec<AiRowView>,
    row_index: HashMap<String, usize>,
    category_ids: BTreeMap<AiCategory, Vec<String>>,
    sensitive_count: usize,
    hidden_tool_rows: usize,
    selected: BTreeMap<AiCategory, BTreeSet<String>>,
    observation_epoch: u64,
    preview: Option<AiPreviewState>,
    notice: Option<CleanOutcomeNotice>,
    failure_detail: Option<String>,
    execution: Option<(usize, usize)>,
    recovery: Vec<AiRecoveryBundle>,
    recovery_loaded: bool,
    restore: Option<AiPlanV1>,
    restore_refused_bundle: Option<String>,
}

impl CleanAiView {
    /// Creates the closed subview. Nothing scans until an explicit action.
    pub fn new() -> Self {
        Self {
            open: false,
            provider: AiProvider::Codex,
            phase: AiSubviewPhase::Landing,
            category: AiCategory::Cache,
            home: None,
            observation: None,
            rows: Vec::new(),
            row_index: HashMap::new(),
            category_ids: BTreeMap::new(),
            sensitive_count: 0,
            hidden_tool_rows: 0,
            selected: BTreeMap::new(),
            observation_epoch: 0,
            preview: None,
            notice: None,
            failure_detail: None,
            execution: None,
            recovery: Vec::new(),
            recovery_loaded: false,
            restore: None,
            restore_refused_bundle: None,
        }
    }

    pub fn open(&self) -> bool {
        self.open
    }

    /// Opens the subview. Opening never scans.
    pub fn open_subview(&mut self) -> bool {
        if self.open {
            return false;
        }
        self.open = true;
        true
    }

    /// Closes the subview without dropping the last accepted observation.
    pub fn close_subview(&mut self) -> bool {
        if !self.open {
            return false;
        }
        self.open = false;
        true
    }

    pub fn phase(&self) -> AiSubviewPhase {
        self.phase
    }

    pub fn provider(&self) -> AiProvider {
        self.provider
    }

    /// Cycles the closed provider set. The next scan uses the new provider.
    pub fn cycle_provider(&mut self) -> AiProvider {
        self.provider = match self.provider {
            AiProvider::Codex => AiProvider::ClaudeCode,
            AiProvider::ClaudeCode => AiProvider::Cursor,
            AiProvider::Cursor => AiProvider::Codex,
        };
        self.provider
    }

    /// The explicit home of the last scan, when one started.
    pub fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    pub fn category(&self) -> AiCategory {
        self.category
    }

    /// Switches the active section. The digest belongs to the previous section,
    /// so it is invalidated here.
    pub fn set_category(&mut self, category: AiCategory) -> bool {
        if self.category == category {
            return false;
        }
        self.category = category;
        self.preview = None;
        true
    }

    pub fn rows(&self) -> &[AiRowView] {
        &self.rows
    }

    /// O(1) row lookup through the cached index.
    pub fn row(&self, id: &str) -> Option<&AiRowView> {
        self.row_index
            .get(id)
            .and_then(|index| self.rows.get(*index))
    }

    /// Number of review rows in one section.
    pub fn category_count(&self, category: AiCategory) -> usize {
        self.category_ids.get(&category).map_or(0, Vec::len)
    }

    /// Cached id projection for one section, in observation order.
    pub fn category_ids(&self, category: AiCategory) -> &[String] {
        self.category_ids
            .get(&category)
            .map_or(&[][..], Vec::as_slice)
    }

    /// Cached id projection for the active section.
    pub fn active_row_ids(&self) -> &[String] {
        self.category_ids(self.category)
    }

    /// Selectable rows in the active section. Sensitive rows never count.
    pub fn selectable_count(&self) -> usize {
        self.active_row_ids()
            .iter()
            .filter(|id| self.row(id).is_some_and(AiRowView::selectable))
            .count()
    }

    pub fn sensitive_count(&self) -> usize {
        self.sensitive_count
    }

    /// Observed tool rows that are not cleanup targets and stay out of review.
    pub fn hidden_tool_rows(&self) -> usize {
        self.hidden_tool_rows
    }

    /// Selected rows in the active section.
    pub fn selected_count(&self) -> usize {
        self.selected.get(&self.category).map_or(0, BTreeSet::len)
    }

    /// Selected rows across every section.
    pub fn total_selected_count(&self) -> usize {
        self.selected.values().map(BTreeSet::len).sum()
    }

    pub fn selection_for(&self, category: AiCategory) -> &BTreeSet<String> {
        match self.selected.get(&category) {
            Some(set) => set,
            None => &EMPTY_SELECTION,
        }
    }

    pub fn is_selected(&self, id: &str) -> bool {
        self.selected
            .get(&self.category)
            .is_some_and(|set| set.contains(id))
    }

    pub fn notice(&self) -> Option<CleanOutcomeNotice> {
        self.notice
    }

    pub fn set_notice(&mut self, notice: Option<CleanOutcomeNotice>) {
        self.notice = notice;
    }

    pub fn failure_detail(&self) -> Option<&str> {
        self.failure_detail.as_deref()
    }

    pub fn preview(&self) -> Option<&AiPreviewState> {
        self.preview.as_ref()
    }

    /// True when the stored digest still covers the active section and its
    /// current selection.
    pub fn preview_matches_current(&self) -> bool {
        self.preview.as_ref().is_some_and(|preview| {
            preview.class == self.category
                && preview.observation_epoch == self.observation_epoch
                && preview.selected_snapshot == self.current_selected()
        })
    }

    pub fn recovery(&self) -> &[AiRecoveryBundle] {
        &self.recovery
    }

    pub fn recovery_loaded(&self) -> bool {
        self.recovery_loaded
    }

    /// The restore plan built for the last accepted bundle, if any. Building a
    /// plan never copies payload bytes.
    pub fn planned_restore(&self) -> Option<&AiPlanV1> {
        self.restore.as_ref()
    }

    /// Bundle id whose restore plan was refused, when one was.
    pub fn restore_refused_bundle(&self) -> Option<&str> {
        self.restore_refused_bundle.as_deref()
    }

    fn current_selected(&self) -> BTreeSet<String> {
        self.selected
            .get(&self.category)
            .cloned()
            .unwrap_or_default()
    }

    /// Records one explicit scan start for one explicit home.
    pub fn on_scan_started(&mut self, home: PathBuf) {
        self.home = Some(home);
        self.phase = AiSubviewPhase::Scanning;
        self.rows.clear();
        self.row_index.clear();
        self.category_ids.clear();
        self.observation = None;
        self.selected.clear();
        self.preview = None;
        self.sensitive_count = 0;
        self.hidden_tool_rows = 0;
        self.execution = None;
        self.recovery.clear();
        self.recovery_loaded = false;
        self.restore = None;
        self.restore_refused_bundle = None;
        self.failure_detail = None;
        self.set_notice(None);
    }

    /// Records that no provider home could be resolved. Nothing was scanned.
    pub fn on_scan_unavailable(&mut self, detail: &str) {
        self.phase = AiSubviewPhase::Failed;
        self.failure_detail = Some(bounded_detail(detail));
        self.set_notice(Some(CleanOutcomeNotice::Unavailable));
    }

    /// Applies one accepted or failed scan payload and derives the phase.
    pub fn on_scan_result(&mut self, payload: CleanAiScanPayload) {
        let CleanAiScanPayload { observation, error } = payload;
        match observation {
            Some(observation) => self.install_observation(observation),
            None => {
                let error = error.unwrap_or(AiError {
                    exit: devsweep_core::ai::AiExit::Failed,
                    code: "execute_failed",
                    message: "the AI scan did not produce an observation".to_string(),
                });
                self.phase = AiSubviewPhase::Failed;
                self.failure_detail = Some(bounded_detail(&error.message));
                self.set_notice(Some(
                    CleanOutcomeNotice::from_ai_error(&error).unwrap_or(CleanOutcomeNotice::Failed),
                ));
            }
        }
    }

    fn install_observation(&mut self, observation: AiObservationV1) {
        self.rows.clear();
        self.row_index.clear();
        self.category_ids.clear();
        self.selected.clear();
        self.preview = None;
        self.execution = None;
        self.sensitive_count = 0;
        self.hidden_tool_rows = 0;
        let mut bounded = false;
        for provider in &observation.providers {
            bounded |= provider.session_bounded;
            for row in &provider.rows {
                let sensitive = row.disposition == AiDisposition::Protected
                    && sensitive_path(&row.relative_path);
                if sensitive {
                    self.sensitive_count += 1;
                }
                let Some(category) = category_of(row.class, sensitive) else {
                    self.hidden_tool_rows += 1;
                    continue;
                };
                let index = self.rows.len();
                self.rows.push(AiRowView {
                    id: row.id.clone(),
                    class: row.class,
                    relative_path: row.relative_path.clone(),
                    disposition: row.disposition,
                    reason: row.reason.clone(),
                    sensitive,
                    category: Some(category),
                });
                self.row_index.insert(row.id.clone(), index);
                self.category_ids
                    .entry(category)
                    .or_default()
                    .push(row.id.clone());
            }
        }
        self.observation = Some(observation);
        self.observation_epoch += 1;
        self.failure_detail = None;
        // A bounded session enumeration is a truthful partial result; the
        // digest still only ever covers explicit candidates.
        self.set_notice(if bounded {
            Some(CleanOutcomeNotice::Partial)
        } else {
            None
        });
        self.phase = AiSubviewPhase::Review;
    }

    /// Toggles one row of the active section. Sensitive, inspect-only, and
    /// non-candidate rows refuse without touching the selection.
    pub fn toggle_selected(&mut self, id: &str) -> bool {
        let Some((category, selectable)) = self.row(id).map(|row| (row.category, row.selectable()))
        else {
            return false;
        };
        if category != Some(self.category) || !selectable {
            return false;
        }
        let bucket = self.selected.entry(self.category).or_default();
        if bucket.remove(id) {
            false
        } else {
            bucket.insert(id.to_string());
            true
        }
    }

    /// Clears the active section's selection. Other sections keep theirs.
    pub fn clear_selected(&mut self) {
        self.selected.remove(&self.category);
    }

    /// Builds the class plan restricted to the active selection.
    fn build_selected_plan(&self) -> Result<Option<AiPlanV1>, AiError> {
        let Some(class) = self.category.class() else {
            return Ok(None);
        };
        let selected = self.selected.get(&self.category);
        let Some(selected) = selected.filter(|set| !set.is_empty()) else {
            return Ok(None);
        };
        let Some(observation) = self.observation.as_ref() else {
            return Ok(None);
        };
        let mut plan = crate::clean::ai_plan(observation, class)?;
        plan.targets.retain(|target| selected.contains(&target.id));
        if plan.targets.is_empty() {
            return Ok(None);
        }
        Ok(Some(plan))
    }

    /// Computes the live preview digest over the selected subset plan. The
    /// digest comes from the shared core AI preview service; this never hashes.
    pub fn preview_selection(&mut self) -> bool {
        if self.phase != AiSubviewPhase::Review {
            return false;
        }
        let plan = match self.build_selected_plan() {
            Ok(Some(plan)) => plan,
            Ok(None) => {
                self.preview = None;
                return false;
            }
            Err(error) => {
                self.preview = None;
                self.failure_detail = Some(bounded_detail(&error.message));
                self.set_notice(Some(
                    CleanOutcomeNotice::from_ai_error(&error)
                        .unwrap_or(CleanOutcomeNotice::Unavailable),
                ));
                return false;
            }
        };
        match crate::clean::ai_preview_digest(&plan) {
            Ok(preview) => {
                let snapshot = self.current_selected();
                self.preview = Some(AiPreviewState {
                    class: self.category,
                    digest: preview.digest,
                    count: plan.targets.len(),
                    plan,
                    selected_snapshot: snapshot,
                    observation_epoch: self.observation_epoch,
                });
                self.set_notice(None);
                true
            }
            Err(error) => {
                self.preview = None;
                self.failure_detail = Some(bounded_detail(&error.to_string()));
                self.set_notice(Some(CleanOutcomeNotice::Unavailable));
                false
            }
        }
    }

    /// Gate for execution. A missing digest records `ConfirmationRequired`; a
    /// digest that no longer matches the active selection records
    /// `StalePreview`. Both refuse without touching any file.
    pub fn begin_execute(&mut self) -> Option<AiExecutionStart> {
        if self.phase != AiSubviewPhase::Review || self.selected_count() == 0 {
            return None;
        }
        self.category.class()?;
        if self.preview.is_none() {
            self.set_notice(Some(CleanOutcomeNotice::ConfirmationRequired));
            return None;
        }
        if !self.preview_matches_current() {
            // A stale digest is invalidated: the review surface returns so the
            // user must produce a digest for the current selection again.
            self.preview = None;
            self.set_notice(Some(CleanOutcomeNotice::StalePreview));
            return None;
        }
        let preview = self.preview.as_ref().expect("preview survived the gate");
        let start = AiExecutionStart {
            class: preview.class,
            plan: preview.plan.clone(),
            digest: preview.digest.clone(),
        };
        self.phase = AiSubviewPhase::Cleaning;
        self.failure_detail = None;
        self.set_notice(None);
        Some(start)
    }

    /// Applies one execution payload and derives the terminal phase. The
    /// notice mapping is the shared `CleanOutcomeNotice` contract, so a
    /// partial report never becomes success copy.
    pub fn on_execute_result(&mut self, payload: CleanAiExecutePayload) {
        self.preview = None;
        self.execution = None;
        let mut notice = payload
            .report
            .as_ref()
            .and_then(CleanOutcomeNotice::from_ai_execute_report)
            .or_else(|| {
                payload
                    .error
                    .as_ref()
                    .and_then(CleanOutcomeNotice::from_ai_error)
            });
        if payload.canceled && notice != Some(CleanOutcomeNotice::TrashReported) {
            notice = Some(CleanOutcomeNotice::Canceled);
        }
        self.failure_detail = payload
            .error
            .as_ref()
            .map(|error| bounded_detail(&error.message));
        if let Some(report) = payload.report.as_ref() {
            self.execution = Some((report.moved.len(), report.bundle_ids.len()));
        }
        match notice {
            Some(notice) => {
                self.phase = if notice == CleanOutcomeNotice::Failed {
                    AiSubviewPhase::Failed
                } else {
                    AiSubviewPhase::Completed
                };
                self.set_notice(Some(notice));
            }
            None => {
                self.phase = if payload.error.is_some() {
                    AiSubviewPhase::Failed
                } else {
                    AiSubviewPhase::Completed
                };
                self.set_notice(None);
            }
        }
    }

    /// Applies one recovery listing. Listing never restores.
    pub fn on_recovery_result(&mut self, payload: CleanAiRecoveryPayload) {
        self.restore = None;
        self.restore_refused_bundle = None;
        match payload.list {
            Some(list) => {
                self.recovery = list.bundles;
                self.recovery_loaded = true;
                self.failure_detail = None;
                self.set_notice(None);
            }
            None => {
                let error = payload.error;
                self.failure_detail = error.as_ref().map(|error| bounded_detail(&error.message));
                self.set_notice(Some(
                    error
                        .as_ref()
                        .and_then(CleanOutcomeNotice::from_ai_error)
                        .unwrap_or(CleanOutcomeNotice::Unavailable),
                ));
            }
        }
    }

    /// Applies one restore-plan result. The plan never copies payload bytes.
    pub fn on_restore_result(&mut self, payload: CleanAiRestorePayload) {
        match payload.plan {
            Some(plan) => {
                self.restore = Some(plan);
                self.restore_refused_bundle = None;
                self.failure_detail = None;
                self.set_notice(None);
            }
            None => {
                self.restore = None;
                self.restore_refused_bundle = Some(payload.bundle_id);
                let error = payload.error;
                self.failure_detail = error.as_ref().map(|error| bounded_detail(&error.message));
                self.set_notice(Some(
                    error
                        .as_ref()
                        .and_then(CleanOutcomeNotice::from_ai_error)
                        .unwrap_or(CleanOutcomeNotice::Failed),
                ));
            }
        }
    }

    /// Truthful history entry for the last terminal execution, if any.
    pub fn history_entry(&self, locale: Locale) -> Option<String> {
        let (moved, bundles) = self.execution?;
        Some(copy::history_clean_ai_executed(locale, moved, bundles))
    }

    /// Whether one row id matches the user-visible search text.
    ///
    /// The search key is the relative path shown on the row line: the raw id
    /// embeds the C15 class name (`regenerable_cache` contains `regen`), so a
    /// raw id filter would match every row of a class. An empty query matches
    /// every row.
    pub fn matches_query(&self, id: &str, query: &str) -> bool {
        self.row(id)
            .is_some_and(|row| query.is_empty() || row.relative_path.contains(query))
    }

    /// One row line. The same text is the visible row and its accessible name.
    pub fn row_text(&self, locale: Locale, id: &str, focused: bool) -> Option<String> {
        let row = self.row(id)?;
        Some(ai_row_text(locale, row, self.is_selected(id), focused))
    }
}

static EMPTY_SELECTION: BTreeSet<String> = BTreeSet::new();

impl Default for CleanAiView {
    fn default() -> Self {
        Self::new()
    }
}

/// One AI row line. Every sensitive row stays visibly marked.
pub fn ai_row_text(locale: Locale, row: &AiRowView, selected: bool, focused: bool) -> String {
    let marker = if selected {
        copy::clean_row_selected_marker(locale)
    } else {
        copy::clean_row_unselected_marker(locale)
    };
    let mut text = String::new();
    if focused {
        text.push_str("> ");
    }
    text.push_str(marker);
    text.push(' ');
    text.push_str(&row.relative_path);
    text.push_str(" · ");
    text.push_str(copy::clean_ai_row_class(locale, row.class.as_str()));
    text.push_str(" · ");
    if row.sensitive {
        text.push_str(copy::clean_ai_row_sensitive(locale));
    } else {
        match row.disposition {
            AiDisposition::Candidate => text.push_str(copy::clean_ai_row_selectable(locale)),
            AiDisposition::InspectOnly => text.push_str(copy::clean_ai_row_inspect_only(locale)),
            AiDisposition::Protected => text.push_str(copy::clean_ai_row_protected(locale)),
            AiDisposition::UnsupportedVersion => {
                text.push_str(copy::clean_ai_row_unsupported(locale))
            }
        }
        if !matches!(row.reason.as_str(), "" | "selectable" | "protected")
            && row.reason != "unsupported_version"
        {
            text.push_str(" (");
            text.push_str(&copy::clean_ai_reason(locale, &row.reason));
            text.push(')');
        }
    }
    text
}

#[cfg(test)]
pub(crate) use tests::isolated_ai_review_home;

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use devsweep_core::ai::AiPlanKind;
    use devsweep_core::process::FlagCancelObserver;
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::clean::{self, IsolatedRenameTrash, cleanup};

    const RECOVERY_BUNDLE_ID: &str = "0123456789abcdef0123456789abcdef";
    const RECOVERY_RELATIVE: &str = "cache/regen-a";

    /// One isolated AI home under the shared temp prefix. Every sensitive name
    /// is planted so refusal tests have a real target.
    pub(crate) fn isolated_ai_review_home() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-{}-ai-review",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing an AI home outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);

        fs::create_dir_all(root.join("cache/regen-a")).unwrap();
        fs::write(root.join("cache/regen-a/data.bin"), b"cache-bytes-a").unwrap();
        fs::create_dir_all(root.join("cache/regen-b")).unwrap();
        fs::write(root.join("cache/regen-b/data.bin"), b"cache-bytes-b").unwrap();
        fs::write(root.join("cache/other.bin"), b"keep").unwrap();

        fs::create_dir_all(root.join("versions/1.0")).unwrap();
        fs::write(root.join("versions/1.0/payload.bin"), b"old").unwrap();
        fs::create_dir_all(root.join("versions/2.0")).unwrap();
        fs::write(root.join("versions/2.0/payload.bin"), b"current").unwrap();
        fs::write(root.join("versions/current.txt"), b"2.0\n").unwrap();

        fs::create_dir_all(root.join("sessions")).unwrap();
        fs::write(
            root.join("sessions/s2.json"),
            br#"{"version":1,"id":"s2","active":false,"body":"archive-body"}"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("worktrees/plain")).unwrap();
        fs::write(root.join("worktrees/plain/file.txt"), b"not-a-repo").unwrap();

        for (relative, bytes) in [
            ("settings.json", b"{}\n".as_slice()),
            ("credentials", b"secret".as_slice()),
            ("memory.md", b"memory".as_slice()),
            ("skills/skill.md", b"skill".as_slice()),
            ("mcp/server.json", b"{}".as_slice()),
            ("models/weights.bin", b"model".as_slice()),
            ("projects/code.txt", b"project".as_slice()),
        ] {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, bytes).unwrap();
        }
        fs::write(root.join("install.json"), br#"{"version":"1.2.3"}"#).unwrap();
        root
    }

    fn sensitive_planted() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("settings.json", b"{}\n".as_slice()),
            ("credentials", b"secret".as_slice()),
            ("memory.md", b"memory".as_slice()),
            ("skills/skill.md", b"skill".as_slice()),
            ("mcp/server.json", b"{}".as_slice()),
            ("models/weights.bin", b"model".as_slice()),
            ("projects/code.txt", b"project".as_slice()),
        ]
    }

    fn reviewed_view(home: &Path) -> CleanAiView {
        let cancel = FlagCancelObserver::new();
        let observation = clean::ai_scan_home(AiProvider::Codex, home, &cancel).expect("scan");
        let mut view = CleanAiView::new();
        assert!(view.open_subview(), "first open must change the state");
        view.on_scan_started(home.to_path_buf());
        assert_eq!(view.phase(), AiSubviewPhase::Scanning);
        view.on_scan_result(CleanAiScanPayload {
            observation: Some(observation),
            error: None,
        });
        assert_eq!(view.phase(), AiSubviewPhase::Review);
        view
    }

    fn row_id_ending(view: &CleanAiView, suffix: &str) -> String {
        view.rows()
            .iter()
            .find(|row| row.id.ends_with(suffix))
            .map(|row| row.id.clone())
            .unwrap_or_else(|| panic!("missing row ending in {suffix}"))
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn ai_error(error: anyhow::Error) -> AiError {
        error
            .downcast_ref::<AiError>()
            .cloned()
            .expect("the AI error survives the helper boundary")
    }

    #[test]
    fn ai_categories_render_as_separate_sections() {
        let home = isolated_ai_review_home();
        let view = reviewed_view(&home);
        assert_eq!(view.provider(), AiProvider::Codex);
        assert_eq!(view.home(), Some(home.as_path()));

        // Cache: two candidates and one inspect-only row, all visible.
        let cache = view.category_ids(AiCategory::Cache);
        assert_eq!(cache.len(), 3, "{cache:?}");
        for relative in ["cache/regen-a", "cache/regen-b"] {
            let row = view
                .rows()
                .iter()
                .find(|row| row.relative_path == relative)
                .expect("candidate row");
            assert!(row.selectable(), "{relative} must be selectable");
        }
        let other = view
            .rows()
            .iter()
            .find(|row| row.relative_path == "cache/other.bin")
            .expect("inspect-only row");
        assert!(!other.selectable(), "not_regen stays inspect-only");

        // Versions, sessions, and worktrees are their own explicit sections.
        assert_eq!(view.category_ids(AiCategory::Versions).len(), 2);
        assert_eq!(view.category_ids(AiCategory::Sessions).len(), 1);
        assert_eq!(view.category_ids(AiCategory::Worktrees).len(), 1);
        let worktree = view
            .row(&view.category_ids(AiCategory::Worktrees)[0])
            .expect("worktree row");
        assert!(
            !worktree.selectable(),
            "an unregistered worktree is never a candidate: {worktree:?}"
        );

        // The sensitive section collects every denied name and nothing else.
        assert_eq!(view.sensitive_count(), 7);
        assert_eq!(view.category_ids(AiCategory::Sensitive).len(), 7);
        for id in view.category_ids(AiCategory::Sensitive) {
            let row = view.row(id).expect("sensitive row");
            assert!(row.sensitive, "{} must stay marked", row.relative_path);
        }
        // Only the non-sensitive install.json row leaves the review surface.
        assert_eq!(view.hidden_tool_rows(), 1);

        // Sections switch without touching the other sections' id caches.
        let mut view = view;
        assert!(view.set_category(AiCategory::Sessions));
        assert_eq!(view.active_row_ids().len(), 1);
        assert_eq!(view.category_ids(AiCategory::Cache).len(), 3);
        cleanup(&home);
    }

    #[test]
    fn ai_sensitive_rows_stay_visible_and_refuse_selection() {
        let home = isolated_ai_review_home();
        let mut view = reviewed_view(&home);
        assert!(view.set_category(AiCategory::Sensitive));
        assert_eq!(view.selectable_count(), 0);
        let sensitive_ids = view.active_row_ids().to_vec();
        assert_eq!(sensitive_ids.len(), 7);
        for id in &sensitive_ids {
            let row = view.row(id).expect("sensitive row");
            assert!(row.sensitive);
            assert!(!row.selectable());
            assert!(
                !view.toggle_selected(id),
                "a sensitive row must refuse selection: {id}"
            );
            assert!(!view.is_selected(id));
            let line = view.row_text(Locale::En, id, false).expect("row text");
            assert!(line.starts_with("[ ] "), "{line}");
            assert!(line.contains("sensitive · never cleaned"), "{line}");
        }
        let credentials = sensitive_ids
            .iter()
            .find(|id| id.ends_with("credentials"))
            .expect("credentials row");
        let line = view
            .row_text(Locale::En, credentials, true)
            .expect("credentials line");
        assert!(line.starts_with("> [ ] "), "{line}");
        let zh = view
            .row_text(Locale::ZhCn, credentials, false)
            .expect("credentials zh line");
        assert!(zh.contains("敏感"), "{zh}");

        // A candidate selection elsewhere can never include a sensitive id.
        assert!(view.set_category(AiCategory::Cache));
        let regen = row_id_ending(&view, "cache/regen-a");
        assert!(view.toggle_selected(&regen));
        assert_eq!(view.selected_count(), 1);
        assert!(sensitive_ids.iter().all(|id| !view.is_selected(id)));
        assert!(view.selection_for(AiCategory::Sensitive).is_empty());
        cleanup(&home);
    }

    #[test]
    fn ai_search_projection_matches_paths_not_class_names() {
        let home = isolated_ai_review_home();
        let view = reviewed_view(&home);

        // Every cache id embeds the C15 class name `regenerable_cache`, so a
        // raw id filter would match the whole section. The search key is the
        // user-visible relative path instead.
        let other = row_id_ending(&view, "cache/other.bin");
        let regen_a = row_id_ending(&view, "cache/regen-a");
        assert!(other.contains("regenerable_cache"), "{other}");
        assert!(view.matches_query(&other, ""));
        assert!(!view.matches_query(&other, "regen"));
        assert!(view.matches_query(&regen_a, "regen"));
        assert!(view.matches_query(&regen_a, "regen-a"));
        assert!(!view.matches_query(&regen_a, "credentials"));
        assert!(!view.matches_query("missing-row", "regen"));

        let credentials = row_id_ending(&view, "credentials");
        assert!(view.matches_query(&credentials, "credentials"));
        cleanup(&home);
    }

    #[test]
    fn ai_selection_preview_confirm_execute_stays_inside_the_isolated_home() {
        let home = isolated_ai_review_home();
        let mut view = reviewed_view(&home);
        let regen_a = row_id_ending(&view, "cache/regen-a");
        let regen_b = row_id_ending(&view, "cache/regen-b");
        assert!(view.toggle_selected(&regen_a));

        // No digest yet: the execute gate refuses with ConfirmationRequired.
        assert!(view.begin_execute().is_none());
        assert_eq!(
            view.notice(),
            Some(CleanOutcomeNotice::ConfirmationRequired)
        );
        assert_eq!(view.phase(), AiSubviewPhase::Review);

        // The preview digest comes from the shared core preview over the
        // selected subset plan.
        assert!(view.preview_selection());
        let preview = view.preview().expect("preview").clone();
        assert!(preview.digest.starts_with("sha256:"));
        assert_eq!(preview.count, 1);
        assert_eq!(preview.plan.targets.len(), 1);
        assert_eq!(preview.plan.targets[0].relative_path, "cache/regen-a");
        assert!(view.preview_matches_current());

        // A changed selection invalidates the digest; the gate refuses stale.
        assert!(view.toggle_selected(&regen_b));
        view.set_notice(None);
        assert!(view.begin_execute().is_none());
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::StalePreview));
        assert_eq!(view.phase(), AiSubviewPhase::Review);
        assert!(view.preview().is_none());

        // Preview again over both targets and pass the gate.
        assert!(view.preview_selection());
        let preview = view.preview().expect("second preview").clone();
        assert_eq!(preview.count, 2);
        let start = view.begin_execute().expect("gate passes");
        assert_eq!(view.phase(), AiSubviewPhase::Cleaning);
        assert_eq!(start.class, AiCategory::Cache);
        assert_eq!(start.plan.targets.len(), 2);

        // Wrong digest and missing confirmation never reach the trash runner.
        let cancel = FlagCancelObserver::new();
        let trash = IsolatedRenameTrash::new(home.clone());
        let mut wrong = format!("sha256:{}", "0".repeat(64));
        if wrong == start.digest {
            wrong = format!("sha256:{}", "1".repeat(64));
        }
        clean::ai_execute(&start.plan, &wrong, true, &cancel, &trash).expect_err("stale digest");
        clean::ai_execute(&start.plan, &start.digest, false, &cancel, &trash)
            .expect_err("unconfirmed execute");
        assert_eq!(trash.call_count(), 0);
        for (relative, bytes) in sensitive_planted() {
            assert_eq!(
                fs::read(home.join(relative)).expect("sensitive file must survive refusals"),
                bytes,
                "{relative} changed after refused executes"
            );
        }
        assert!(home.join("cache/regen-a/data.bin").is_file());

        // The confirmed matching digest executes with the injected rename
        // runner inside the isolated home.
        let report = clean::ai_execute(&start.plan, &start.digest, true, &cancel, &trash)
            .expect("confirmed execute");
        assert_eq!(trash.call_count(), 2);
        assert_eq!(report.outcome, "succeeded");
        assert_eq!(report.moved.len(), 2);
        assert_eq!(report.bundle_ids.len(), 2);
        assert!(!home.join("cache/regen-a").exists());
        assert!(!home.join("cache/regen-b").exists());
        for (relative, bytes) in sensitive_planted() {
            assert_eq!(
                fs::read(home.join(relative)).expect("sensitive file must survive execute"),
                bytes,
                "{relative} changed during execute"
            );
        }

        // The outcome is the truthful report mapping, never success copy.
        view.on_execute_result(CleanAiExecutePayload {
            report: Some(report),
            error: None,
            canceled: false,
        });
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::TrashReported));
        assert_eq!(view.phase(), AiSubviewPhase::Completed);
        let entry = view.history_entry(Locale::En).expect("history entry");
        assert!(entry.contains("AI review executed"), "{entry}");
        assert!(entry.contains("2 targets moved"), "{entry}");
        assert!(!entry.to_ascii_lowercase().contains("succeeded"), "{entry}");
        cleanup(&home);
    }

    #[test]
    fn ai_partial_and_failed_execute_reports_map_truthfully() {
        let mut view = CleanAiView::new();
        view.open_subview();
        view.on_scan_started(env::temp_dir().join("devsweep-gpui-c11-fixture-home"));
        view.on_scan_result(CleanAiScanPayload {
            observation: Some(AiObservationV1 {
                schema: "devsweep.ai.observation.v1".to_string(),
                version: 1,
                providers: Vec::new(),
            }),
            error: None,
        });
        assert_eq!(view.phase(), AiSubviewPhase::Review);
        assert!(view.rows().is_empty());

        let report = |outcome: &str| AiExecuteReportV1 {
            schema: "devsweep.ai.execute.v1".to_string(),
            version: 1,
            outcome: outcome.to_string(),
            bundle_ids: vec![RECOVERY_BUNDLE_ID.to_string()],
            moved: vec!["target-1".to_string()],
            code: None,
        };
        view.on_execute_result(CleanAiExecutePayload {
            report: Some(report("partial")),
            error: None,
            canceled: false,
        });
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::Mixed));
        assert_eq!(view.phase(), AiSubviewPhase::Completed);

        view.on_execute_result(CleanAiExecutePayload {
            report: Some(report("failed")),
            error: None,
            canceled: false,
        });
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::Failed));
        assert_eq!(view.phase(), AiSubviewPhase::Failed);

        // A canceled run that did not move everything reports canceled.
        view.on_execute_result(CleanAiExecutePayload {
            report: Some(AiExecuteReportV1 {
                schema: "devsweep.ai.execute.v1".to_string(),
                version: 1,
                outcome: "succeeded".to_string(),
                bundle_ids: Vec::new(),
                moved: Vec::new(),
                code: None,
            }),
            error: None,
            canceled: true,
        });
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::Canceled));
        assert_eq!(view.phase(), AiSubviewPhase::Completed);

        // Stale refusals from the worker map to the stale-preview notice.
        view.on_execute_result(CleanAiExecutePayload {
            report: None,
            error: Some(AiError {
                exit: devsweep_core::ai::AiExit::Stale,
                code: "stale_confirmation",
                message: "the live AI identity does not match the preview digest".to_string(),
            }),
            canceled: false,
        });
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::StalePreview));
        assert_eq!(view.phase(), AiSubviewPhase::Completed);

        // An unavailable source maps to the unavailable notice.
        view.on_execute_result(CleanAiExecutePayload {
            report: None,
            error: Some(AiError {
                exit: devsweep_core::ai::AiExit::Unavailable,
                code: "source_unavailable",
                message: "AI target is unavailable".to_string(),
            }),
            canceled: false,
        });
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::Unavailable));
    }

    #[test]
    fn ai_recovery_rows_show_bundle_identity_and_restore_plan_copies_nothing() {
        let home = isolated_ai_review_home();
        // Plant one recovery bundle with the closed manifest shape.
        let payload_bytes = b"c11-regen-cache-bytes".to_vec();
        let content_sha256 = sha256_hex(&payload_bytes);
        let bundle = home.join(".devsweep-recovery").join(RECOVERY_BUNDLE_ID);
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("payload"), &payload_bytes).unwrap();
        let manifest = serde_json::json!({
            "schema": "devsweep.ai.recovery.v1",
            "version": 1,
            "id": RECOVERY_BUNDLE_ID,
            "provider": "codex",
            "class": "regenerable_cache",
            "relative_path": RECOVERY_RELATIVE,
            "content_sha256": content_sha256,
            "kind": "file",
        });
        fs::write(
            bundle.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let mut view = reviewed_view(&home);
        let list = clean::ai_recovery_list(&home).expect("recovery list");
        view.on_recovery_result(CleanAiRecoveryPayload {
            list: Some(list),
            error: None,
        });
        assert!(view.recovery_loaded());
        assert_eq!(view.recovery().len(), 1);
        let entry = &view.recovery()[0];
        assert_eq!(entry.id, RECOVERY_BUNDLE_ID);
        assert_eq!(entry.content_sha256, content_sha256);
        assert_eq!(entry.relative_path, RECOVERY_RELATIVE);
        assert_eq!(view.notice(), None);

        // The restore-plan action builds a plan and copies no bytes.
        let plan = clean::ai_restore_plan(&home, RECOVERY_BUNDLE_ID).expect("restore plan");
        assert_eq!(plan.kind, AiPlanKind::Restore);
        assert_eq!(plan.bundle_id.as_deref(), Some(RECOVERY_BUNDLE_ID));
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.targets[0].action, "restore_bytes");
        view.on_restore_result(CleanAiRestorePayload {
            bundle_id: RECOVERY_BUNDLE_ID.to_string(),
            plan: Some(plan),
            error: None,
        });
        assert!(view.planned_restore().is_some());
        assert_eq!(view.restore_refused_bundle(), None);

        // An unknown bundle is refused truthfully without touching the home.
        let missing = "ffffffffffffffffffffffffffffffff";
        let error = ai_error(clean::ai_restore_plan(&home, missing).expect_err("unknown bundle"));
        assert_eq!(error.code, "source_unavailable");
        view.on_restore_result(CleanAiRestorePayload {
            bundle_id: missing.to_string(),
            plan: None,
            error: Some(error),
        });
        assert_eq!(view.planned_restore(), None);
        assert_eq!(view.restore_refused_bundle(), Some(missing));
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::Unavailable));

        // Listing and planning never copied or moved a byte.
        assert_eq!(
            fs::read(bundle.join("payload")).expect("payload intact"),
            payload_bytes,
            "restore planning must not copy or move payload bytes"
        );
        assert!(
            home.join("cache/regen-a/data.bin").is_file(),
            "restore planning must leave the target untouched"
        );
        assert_eq!(
            fs::read(home.join("cache/regen-a/data.bin")).unwrap(),
            b"cache-bytes-a"
        );
        cleanup(&home);
    }

    #[test]
    fn ai_scan_failure_and_cancel_report_truthfully() {
        let mut view = CleanAiView::new();
        view.open_subview();
        let missing = env::temp_dir().join("devsweep-gpui-c11-missing-ai-home");
        let _ = fs::remove_dir_all(&missing);
        view.on_scan_started(missing.clone());
        let cancel = FlagCancelObserver::new();
        let error = clean::ai_scan_home(AiProvider::Codex, &missing, &cancel)
            .expect_err("missing explicit home");
        assert_eq!(error.code, "source_unavailable");
        view.on_scan_result(CleanAiScanPayload {
            observation: None,
            error: Some(error),
        });
        assert_eq!(view.phase(), AiSubviewPhase::Failed);
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::Unavailable));
        assert!(view.failure_detail().is_some());
        assert!(view.rows().is_empty());
        assert!(view.category_ids(AiCategory::Cache).is_empty());

        // A canceled scan is the canceled notice, never a partial claim.
        let mut view = CleanAiView::new();
        view.open_subview();
        view.on_scan_started(missing.clone());
        view.on_scan_result(CleanAiScanPayload {
            observation: None,
            error: Some(AiError {
                exit: devsweep_core::ai::AiExit::Canceled,
                code: "canceled",
                message: "canceled before dispatch".to_string(),
            }),
        });
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::Canceled));

        // No resolvable home is an explicit unavailable refusal.
        let mut view = CleanAiView::new();
        view.open_subview();
        view.on_scan_unavailable("no AI provider home is available");
        assert_eq!(view.phase(), AiSubviewPhase::Failed);
        assert_eq!(view.notice(), Some(CleanOutcomeNotice::Unavailable));
    }

    #[test]
    fn ai_subview_model_never_reads_the_environment_or_scans_by_itself() {
        let source = include_str!("clean_ai.rs");
        let model = source.split("#[cfg(test)]").next().expect("model section");
        assert!(!model.contains("std::env"));
        assert!(!model.contains("USERPROFILE"));
        assert!(!model.contains("scan_ai"));
        assert!(!model.contains("execute_ai"));
        assert!(!model.contains("preview_ai"));
        assert!(!model.contains("recovery_list"));
        assert!(!model.contains("restore_plan"));
        assert!(!model.contains("remove_file") && !model.contains("remove_dir"));
        assert!(!model.contains("fs::"));
    }
}
