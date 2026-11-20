//! Clean review-and-clean workflow state for the GPUI shell.
//!
//! The shell owns phases, the accepted scan report projection, the row
//! selection set, and the saved preview plan. Scanning, preview digest
//! computation, and execution are delegated to the shared core services on a
//! worker; nothing here touches the render thread. A stale or missing preview
//! digest never reaches the core executor, and the core re-checks the digest
//! again before any dispatched move. This module never hashes locally.

use std::{collections::BTreeSet, collections::HashMap, path::Path, path::PathBuf, sync::Arc};

use devsweep_core::{
    execution::{
        ActionKind, CapacityEstimate, ConfirmationDigest, ExecutionError, ExecutionReport,
        ExecutionRequest, Executor, OutcomeStatus, TargetOutcome,
    },
    model::{
        CleanupIntent, Ecosystem, ScanPreviewSnapshot, ScanReport, Scope, TargetId, TargetKind,
        UntrustedPlan, UntrustedTarget,
    },
    plan::validate_plan,
    process::{CancelObserver as _, FlagCancelObserver},
    scan::{ScanOptions, ScanReportRunOutcome, Sweeper},
};

use crate::{
    copy::{self, Locale},
    shell::CleanOutcomeNotice,
};

/// Ordered Clean review phases. Rendering never starts a scan or an execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanPhase {
    /// Scope configuration and explicit scan action. No scan has started.
    Landing,
    /// A worker scan is in flight.
    Scanning,
    /// The accepted report has rows to review.
    Found,
    /// The accepted report has no rows.
    AllClean,
    /// A worker execution is in flight.
    Cleaning,
    /// The execution reached a terminal result.
    Completed,
    /// The scan or execution could not complete.
    Failed,
}

impl CleanPhase {
    pub fn id(self) -> &'static str {
        match self {
            Self::Landing => "landing",
            Self::Scanning => "scanning",
            Self::Found => "found",
            Self::AllClean => "all_clean",
            Self::Cleaning => "cleaning",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// Scan scope chosen before any scan starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanScope {
    /// Explicit project roots only.
    Project,
    /// Real global provider caches. Requires an explicit confirmation.
    Global,
    /// Explicit projects plus globals only after a fresh confirmation.
    All,
}

/// Requested scope and one-shot global consent stay separate. Desktop roots
/// must always be explicit; All without consent is only a project scan.
pub fn clean_scan_options(
    scope: CleanScope,
    confirmed_global: bool,
    roots: Vec<PathBuf>,
) -> Option<ScanOptions> {
    let projects = scope != CleanScope::Global;
    if projects && roots.is_empty() {
        return None;
    }
    if scope == CleanScope::Global && !confirmed_global {
        return None;
    }
    Some(ScanOptions {
        include_projects: projects,
        include_global: scope != CleanScope::Project && confirmed_global,
        roots: if projects { roots } else { Vec::new() },
    })
}

impl CleanScope {
    pub fn id(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
            Self::All => "all",
        }
    }
}

/// Landing sub-state. Landing never scans; these prompts only ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanLandingPrompt {
    /// Scope controls and the selected scope configuration are shown.
    None,
    /// Project scope needs a saved root before a scan can start.
    RootRequired,
    /// Global scope asked for an explicit confirmation before its scan.
    ConfirmGlobal,
}

/// Display order for the bounded row window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanSort {
    /// Accepted report order.
    ReportOrder,
    /// Rule id, then ownership path.
    Name,
    /// Largest observed bytes first; report order breaks ties.
    Size,
}

impl CleanSort {
    pub fn cycle(self) -> Self {
        match self {
            Self::ReportOrder => Self::Name,
            Self::Name => Self::Size,
            Self::Size => Self::ReportOrder,
        }
    }
}

/// Boundary within which a scanned row was observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CleanRowScope {
    /// Machine-level provider scope.
    Global,
    /// Project scope rooted at the given path.
    Project { root: String },
}

/// One reviewable row with the metadata the accepted report exposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanRow {
    /// Stable target identifier.
    pub id: String,
    /// Position in the accepted report (canonical digest order).
    pub plan_index: u32,
    /// Built-in rule that produced the target.
    pub rule_id: String,
    /// Rule namespace prefix used as the provider/group tag.
    pub provider: String,
    /// Filesystem or global scope observed by the scanner.
    pub scope: CleanRowScope,
    /// Observed ownership path, when the target is path-backed.
    pub path: Option<String>,
    /// Estimated reclaimable bytes or lower bound.
    pub bytes: u64,
    /// When false, `bytes` is only a lower bound (or zero is untrusted).
    pub size_complete: bool,
    /// True when the target has no cleanup authority.
    pub inspect_only: bool,
    /// Typed scan metadata; neither a rule name nor a path grants trash authority.
    pub ecosystem: Ecosystem,
    pub kind: TargetKind,
    pub recycle_eligible: bool,
    /// Conservative scanner recommendation for initial selection.
    pub selected_by_default: bool,
}

impl CleanRow {
    /// Returns whether the row can enter a cleanup selection.
    pub fn selectable(&self) -> bool {
        self.recycle_eligible && !self.inspect_only
    }

    /// Returns the confidence label for the size estimate.
    pub fn confidence(&self, locale: Locale) -> &'static str {
        if self.size_complete {
            copy::clean_row_confidence_verified(locale)
        } else if self.bytes == 0 {
            copy::clean_row_confidence_unknown(locale)
        } else {
            copy::clean_row_confidence_partial(locale)
        }
    }
}

/// Project an accepted untrusted target into a reviewable row.
pub fn clean_row_from_untrusted(target: &UntrustedTarget, plan_index: usize) -> CleanRow {
    CleanRow {
        id: target.id.as_str().to_string(),
        plan_index: u32::try_from(plan_index).unwrap_or(u32::MAX),
        rule_id: target.rule_id.clone(),
        provider: provider_of(&target.rule_id).to_string(),
        scope: match &target.scope {
            Scope::Global => CleanRowScope::Global,
            Scope::Project { root } => CleanRowScope::Project {
                root: root.display().to_string(),
            },
        },
        path: target.path.as_ref().map(|path| path.display().to_string()),
        bytes: target.estimated_bytes,
        size_complete: target.size_complete,
        inspect_only: matches!(target.intent, CleanupIntent::InspectOnly { .. }),
        ecosystem: target.ecosystem.clone(),
        kind: target.kind.clone(),
        recycle_eligible: matches!(target.intent, CleanupIntent::TrashProjectArtifact { .. }),
        selected_by_default: target.selected_by_default,
    }
}

/// Provider/group tag for a rule id. The namespace prefix is the provider.
pub fn provider_of(rule_id: &str) -> &str {
    rule_id.split('.').next().unwrap_or(rule_id)
}

/// Observed capacity, never a measurement of physical free space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CleanCapacity {
    pub bytes: u64,
    pub incomplete: usize,
}

impl CleanCapacity {
    fn add(&mut self, bytes: u64, complete: bool) {
        self.bytes = self.bytes.saturating_add(bytes);
        self.incomplete += usize::from(!complete);
    }

    pub fn label(self, locale: Locale) -> String {
        copy::clean_capacity(locale, self.bytes, self.incomplete > 0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupSelection {
    None,
    Mixed,
    All,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanGroup {
    pub id: String,
    pub ecosystem: Ecosystem,
    pub kind: TargetKind,
    pub review_only: bool,
    pub ids: Vec<String>,
    pub selectable_ids: Vec<String>,
    pub capacity: CleanCapacity,
    pub recommended: CleanCapacity,
    pub selection: GroupSelection,
    pub selected_count: usize,
}

impl CleanGroup {
    pub fn title(&self, locale: Locale) -> String {
        copy::clean_group_title(locale, &self.ecosystem, &self.kind, self.review_only)
    }
}

fn row_group_id(row: &CleanRow) -> String {
    if !row.selectable() {
        "review-only".to_string()
    } else {
        format!("{:?}-{:?}", row.ecosystem, row.kind).to_ascii_lowercase()
    }
}

/// Pure whole-report projection. Filtering and paging never change group authority.
pub fn project_clean_groups(
    rows: &[CleanRow],
    selected: &BTreeSet<String>,
    blocked: &BTreeSet<String>,
) -> Vec<CleanGroup> {
    let mut groups: Vec<CleanGroup> = Vec::new();
    let mut index = HashMap::new();
    for row in rows {
        let id = row_group_id(row);
        let position = *index.entry(id.clone()).or_insert_with(|| {
            groups.push(CleanGroup {
                id,
                ecosystem: row.ecosystem.clone(),
                kind: row.kind.clone(),
                review_only: !row.selectable(),
                ids: Vec::new(),
                selectable_ids: Vec::new(),
                capacity: CleanCapacity::default(),
                recommended: CleanCapacity::default(),
                selection: GroupSelection::None,
                selected_count: 0,
            });
            groups.len() - 1
        });
        let group = &mut groups[position];
        group.ids.push(row.id.clone());
        group.capacity.add(row.bytes, row.size_complete);
        if row.selectable() && !blocked.contains(&row.id) {
            group.selectable_ids.push(row.id.clone());
            if row.selected_by_default {
                group.recommended.add(row.bytes, row.size_complete);
            }
        }
    }
    for group in &mut groups {
        let count = group
            .selectable_ids
            .iter()
            .filter(|id| selected.contains(*id))
            .count();
        group.selected_count = count;
        group.selection = if count == 0 {
            GroupSelection::None
        } else if count == group.selectable_ids.len() {
            GroupSelection::All
        } else {
            GroupSelection::Mixed
        };
    }
    groups
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanResultGroup {
    pub group: CleanGroup,
    pub outcomes: Vec<TargetOutcome>,
    pub moved: CleanCapacity,
    pub moved_count: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanScanProgress {
    pub found: usize,
    pub capacity: CleanCapacity,
    pub groups: Vec<(Ecosystem, TargetKind, usize, CleanCapacity)>,
}

impl CleanScanProgress {
    fn from_snapshot(snapshot: ScanPreviewSnapshot) -> Self {
        let mut progress = Self {
            found: snapshot.targets.len(),
            ..Self::default()
        };
        for target in snapshot.targets {
            progress
                .capacity
                .add(target.estimated_bytes, target.size_complete);
            let position = progress
                .groups
                .iter()
                .position(|(ecosystem, kind, _, _)| {
                    *ecosystem == target.ecosystem && *kind == target.kind
                })
                .unwrap_or_else(|| {
                    progress.groups.push((
                        target.ecosystem,
                        target.kind,
                        0,
                        CleanCapacity::default(),
                    ));
                    progress.groups.len() - 1
                });
            progress.groups[position].2 += 1;
            progress.groups[position]
                .3
                .add(target.estimated_bytes, target.size_complete);
        }
        // An in-flight observation is always a lower bound on the requested scan.
        progress.capacity.incomplete = progress.capacity.incomplete.max(1);
        progress
    }
}

/// One row line. The same text is the visible row and its accessible name.
/// The trailing rebuild-cost segment explains why the cache will regrow.
pub fn row_text(
    locale: Locale,
    row: &CleanRow,
    selected: bool,
    focused: bool,
    protected: bool,
    ignored: bool,
) -> String {
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
    text.push_str(&row.rule_id);
    text.push_str(" · ");
    match &row.path {
        Some(path) => text.push_str(&crate::ui::display_path_text(path)),
        None => text.push_str(copy::clean_row_global(locale)),
    }
    text.push_str(" · ");
    text.push_str(&copy::clean_bytes_label(row.bytes));
    text.push_str(" · ");
    text.push_str(row.confidence(locale));
    if row.inspect_only {
        text.push_str(" · ");
        text.push_str(copy::clean_row_inspect_only(locale));
    }
    if protected {
        text.push_str(" · ");
        text.push_str(copy::clean_row_protected(locale));
    }
    if ignored {
        text.push_str(" · ");
        text.push_str(copy::clean_row_ignored(locale));
    }
    text.push_str(" · ");
    text.push_str(&copy::clean_row_rebuild_cost(locale, &row.rule_id));
    text
}

/// Terminal result class of one worker scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanScanOutcome {
    /// Every requested phase completed with complete health.
    Completed,
    /// The report is usable but some observation is incomplete.
    Partial,
    /// Cooperative cancellation stopped the scan.
    Canceled,
    /// The scan pipeline itself failed.
    Failed,
}

/// One worker scan result carried through the mailbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanScanPayload {
    /// Reviewable rows in report order.
    pub rows: Vec<CleanRow>,
    /// The accepted untrusted plan, when a report was produced.
    pub plan: Option<UntrustedPlan>,
    /// Terminal outcome class.
    pub outcome: CleanScanOutcome,
    /// Bounded explanation for an incomplete or failed scan.
    pub detail: Option<String>,
}

/// Terminal failure class of one worker execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanExecuteFailure {
    /// The live digest no longer matches the stored preview digest.
    Stale,
    /// The saved plan failed live validation.
    Unavailable,
    /// The executor refused or failed at its boundary.
    Failed,
}

/// One worker execution result carried through the mailbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanExecutePayload {
    /// Aggregate execution report when the executor reached a terminal result.
    pub report: Option<ExecutionReport>,
    /// Boundary failure class when no report was produced.
    pub failure: Option<CleanExecuteFailure>,
    /// Whether cooperative cancellation was requested for this run.
    pub canceled: bool,
}

/// Counts kept for the truthful history entry and result line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanExecutionSummary {
    pub report: ExecutionReport,
    pub groups: Vec<CleanResultGroup>,
    pub moved: CleanCapacity,
    pub moved_count: usize,
    pub history_incomplete: bool,
    outcome_index: HashMap<String, (usize, usize)>,
    /// Number of validated targets selected by the request.
    pub selected: usize,
    /// Number of successful target actions.
    pub succeeded: usize,
    /// Number of failed target actions.
    pub failed: usize,
    /// Number of targets skipped before dispatch.
    pub skipped: usize,
    /// Live digest the executor verified, as `sha256:` plus hex.
    pub digest: Option<String>,
    /// Whether cooperative cancellation was requested for this run.
    pub canceled: bool,
}

impl CleanExecutionSummary {
    pub fn outcome(&self, id: &str) -> Option<&TargetOutcome> {
        let (group, index) = self.outcome_index.get(id)?;
        self.groups.get(*group)?.outcomes.get(*index)
    }
}

/// The saved plan snapshot and live digest shown by the confirm surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanPreview {
    /// Live `sha256:` confirmation digest over the saved plan and selection.
    pub digest: String,
    /// Selected target count the digest covers.
    pub count: usize,
    /// The saved plan restricted to the selection.
    pub saved_plan: UntrustedPlan,
    /// Selected ids in saved-plan order.
    pub selected_ids: Vec<TargetId>,
    /// Selected id snapshot the digest was computed for.
    pub selected_snapshot: BTreeSet<String>,
    /// Accepted-report epoch the digest was computed for.
    pub plan_epoch: u64,
}

/// Inputs handed to the execute worker after all shell-level gates pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanExecutionStart {
    /// Saved plan restricted to the selection.
    pub saved_plan: UntrustedPlan,
    /// Selected ids in saved-plan order.
    pub selected: Vec<TargetId>,
    /// The stored live preview digest.
    pub digest: String,
}

/// Pure Clean workflow model. It performs no I/O beyond calling shared core
/// services from [`scan_clean_rows`] and [`execute_clean_plan`].
///
/// The sorted row projection and the id index are rebuilt once per row-set or
/// sort change, never per render: [`Self::row`] is O(1) and [`Self::row_ids`]
/// returns the cached projection by reference.
#[derive(Clone, Debug)]
pub struct CleanWorkbench {
    phase: CleanPhase,
    prompt: CleanLandingPrompt,
    selected_scope: CleanScope,
    rows: Vec<CleanRow>,
    row_index: HashMap<String, usize>,
    cached_ids: Vec<String>,
    groups: Vec<CleanGroup>,
    group_index: HashMap<String, usize>,
    reviewing: bool,
    native_fixture: Option<String>,
    effective_scope: Option<CleanScope>,
    plan: Option<UntrustedPlan>,
    plan_epoch: u64,
    selected: BTreeSet<String>,
    sort: CleanSort,
    found_so_far: usize,
    scan_progress: CleanScanProgress,
    preview: Option<CleanPreview>,
    notice: Option<CleanOutcomeNotice>,
    failure_detail: Option<String>,
    result_counts: Option<(usize, usize)>,
    execution: Option<CleanExecutionSummary>,
    last_scope: Option<CleanScope>,
    suggestions: Vec<PathBuf>,
    root_session_only: bool,
    protected_paths: Vec<String>,
    ignored: BTreeSet<String>,
}

impl CleanWorkbench {
    /// Creates the landing state. Nothing scans until an explicit action.
    pub fn new() -> Self {
        Self {
            phase: CleanPhase::Landing,
            prompt: CleanLandingPrompt::None,
            selected_scope: CleanScope::Project,
            rows: Vec::new(),
            row_index: HashMap::new(),
            cached_ids: Vec::new(),
            groups: Vec::new(),
            group_index: HashMap::new(),
            reviewing: false,
            native_fixture: None,
            effective_scope: None,
            plan: None,
            plan_epoch: 0,
            selected: BTreeSet::new(),
            sort: CleanSort::ReportOrder,
            found_so_far: 0,
            scan_progress: CleanScanProgress::default(),
            preview: None,
            notice: None,
            failure_detail: None,
            result_counts: None,
            execution: None,
            last_scope: None,
            suggestions: Vec::new(),
            root_session_only: false,
            protected_paths: Vec::new(),
            ignored: BTreeSet::new(),
        }
    }

    pub fn phase(&self) -> CleanPhase {
        self.phase
    }

    pub fn prompt(&self) -> CleanLandingPrompt {
        self.prompt
    }

    pub fn selected_scope(&self) -> CleanScope {
        self.selected_scope
    }

    /// Scope editing never starts work and invalidates any prior authority.
    pub fn select_scope(&mut self, scope: CleanScope) {
        if matches!(self.phase, CleanPhase::Scanning | CleanPhase::Cleaning) {
            return;
        }
        if self.selected_scope != scope {
            self.reset_to_landing();
            self.selected_scope = scope;
        }
    }

    pub fn notice(&self) -> Option<CleanOutcomeNotice> {
        self.notice
    }

    pub fn set_notice(&mut self, notice: Option<CleanOutcomeNotice>) {
        self.notice = notice;
    }

    pub fn rows(&self) -> &[CleanRow] {
        &self.rows
    }

    pub fn groups(&self) -> &[CleanGroup] {
        &self.groups
    }
    pub fn native_fixture(&self) -> Option<&str> {
        self.native_fixture.as_deref()
    }
    pub fn group_for_row(&self, id: &str) -> Option<&CleanGroup> {
        self.group_index
            .get(id)
            .and_then(|index| self.groups.get(*index))
    }
    pub fn reviewing(&self) -> bool {
        self.reviewing
    }
    pub fn begin_review(&mut self) {
        if self.phase == CleanPhase::Found {
            self.reviewing = true;
        }
    }
    pub fn execution(&self) -> Option<&CleanExecutionSummary> {
        self.execution.as_ref()
    }
    #[cfg(test)]
    pub fn effective_scope(&self) -> Option<CleanScope> {
        self.effective_scope
    }
    pub fn set_effective_scope(&mut self, scope: CleanScope) {
        self.effective_scope = Some(scope);
    }
    pub fn global_omitted(&self) -> bool {
        self.selected_scope == CleanScope::All && self.effective_scope == Some(CleanScope::Project)
    }
    pub fn capacity(&self) -> CleanCapacity {
        self.groups
            .iter()
            .fold(CleanCapacity::default(), |mut total, group| {
                total.bytes = total.bytes.saturating_add(group.capacity.bytes);
                total.incomplete += group.capacity.incomplete;
                total
            })
    }
    pub fn recommended_capacity(&self) -> CleanCapacity {
        self.groups
            .iter()
            .fold(CleanCapacity::default(), |mut total, group| {
                total.bytes = total.bytes.saturating_add(group.recommended.bytes);
                total.incomplete += group.recommended.incomplete;
                total
            })
    }
    pub fn selected_capacity(&self) -> CleanCapacity {
        self.selected.iter().filter_map(|id| self.row(id)).fold(
            CleanCapacity::default(),
            |mut total, row| {
                total.add(row.bytes, row.size_complete);
                total
            },
        )
    }
    fn rebuild_groups(&mut self) {
        let blocked = self
            .rows
            .iter()
            .filter(|row| !self.row_selectable(row))
            .map(|row| row.id.clone())
            .collect();
        self.groups = project_clean_groups(&self.rows, &self.selected, &blocked);
        self.group_index.clear();
        self.cached_ids.clear();
        for (index, group) in self.groups.iter().enumerate() {
            for id in &group.ids {
                self.group_index.insert(id.clone(), index);
                self.cached_ids.push(id.clone());
            }
        }
    }
    pub fn toggle_group(&mut self, id: &str) {
        if self.phase != CleanPhase::Found {
            return;
        }
        let Some(group) = self.groups.iter().find(|group| group.id == id) else {
            return;
        };
        if group.selectable_ids.is_empty() {
            return;
        }
        let remove = group.selection == GroupSelection::All;
        for id in group.selectable_ids.clone() {
            if remove {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id);
            }
        }
        self.selection_changed();
    }
    fn selection_changed(&mut self) {
        self.preview = None;
        self.reviewing = true;
        self.rebuild_groups();
    }

    /// Row lookup through the cached index. O(1), so the bounded window can
    /// paint 48 of 10k rows without scanning the report.
    pub fn row(&self, id: &str) -> Option<&CleanRow> {
        self.row_index
            .get(id)
            .and_then(|index| self.rows.get(*index))
    }

    /// True when one saved protection covers this row's ownership path. A
    /// protected row stays visible and read-only.
    pub fn is_protected(&self, id: &str) -> bool {
        self.row(id)
            .and_then(|row| row.path.as_deref())
            .is_some_and(|path| {
                self.protected_paths
                    .iter()
                    .any(|protected| path_covers(protected, path))
            })
    }

    /// True when the row was ignored for this review. An ignored row keeps its
    /// place in the report but never enters the selection.
    pub fn is_ignored(&self, id: &str) -> bool {
        self.ignored.contains(id)
    }

    /// Returns whether the row can enter the cleanup selection right now:
    /// selectable in the report, not protected, and not ignored.
    pub fn row_selectable(&self, row: &CleanRow) -> bool {
        row.selectable() && !self.is_protected(&row.id) && !self.is_ignored(&row.id)
    }

    /// Replaces the protection projection from the existing protections store.
    /// Newly protected rows leave the selection and any reviewed digest is
    /// invalidated; the rows themselves stay visible.
    ///
    /// An empty or relative entry cannot be a stored protection, so its
    /// presence means the input is not the validated protection list. The
    /// projection then fails closed: the reviewed selection and any digest are
    /// dropped (the rows stay visible), and such entries still cover nothing.
    pub fn set_protected_paths(&mut self, paths: Vec<String>) {
        let trusted = paths
            .iter()
            .all(|path| !path.is_empty() && Path::new(path).is_absolute());
        self.protected_paths = paths;
        if !trusted {
            self.selected.clear();
            self.preview = None;
            self.rebuild_groups();
            return;
        }
        let dropped = self
            .selected
            .iter()
            .filter(|id| self.row(id).is_none_or(|row| !self.row_selectable(row)))
            .cloned()
            .collect::<Vec<_>>();
        for id in dropped {
            self.selected.remove(&id);
        }
        if self.preview.is_some() {
            self.preview = None;
        }
        self.rebuild_groups();
    }

    /// Toggles one row's ignore state. Returns the new state, or `None` when
    /// the row does not exist. Ignoring drops the row from the selection and
    /// invalidates a reviewed digest.
    pub fn toggle_ignored(&mut self, id: &str) -> Option<bool> {
        self.row(id)?;
        if self.ignored.remove(id) {
            self.selection_changed();
            return Some(false);
        }
        self.ignored.insert(id.to_string());
        self.selected.remove(id);
        self.selection_changed();
        Some(true)
    }

    #[cfg(test)]
    pub fn selected_ids(&self) -> &BTreeSet<String> {
        &self.selected
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.contains(id)
    }

    pub fn sort(&self) -> CleanSort {
        self.sort
    }

    pub fn found_so_far(&self) -> usize {
        self.found_so_far
    }
    pub fn scan_progress(&self) -> &CleanScanProgress {
        &self.scan_progress
    }
    pub fn on_scan_observations(&mut self, progress: CleanScanProgress) {
        if self.phase == CleanPhase::Scanning {
            self.on_scan_progress(progress.found);
            self.scan_progress = progress;
        }
    }

    pub fn preview(&self) -> Option<&CleanPreview> {
        self.preview.as_ref()
    }

    /// True when the stored preview still covers the current selection and
    /// accepted report. Used to decide between preview and confirm.
    pub fn preview_matches_current(&self) -> bool {
        self.preview.as_ref().is_some_and(|preview| {
            preview.plan_epoch == self.plan_epoch && preview.selected_snapshot == self.selected
        })
    }

    /// Scope accepted for the last worker scan, if any.
    pub fn last_scope(&self) -> Option<CleanScope> {
        self.last_scope
    }

    /// Records whether the picked root could be remembered.
    pub fn set_root_session_only(&mut self, session_only: bool) {
        self.root_session_only = session_only;
    }

    /// Drops the selection and the preview without touching the accepted rows.
    /// Scope and protection edits invalidate a reviewed digest immediately.
    pub fn invalidate_review(&mut self) {
        self.selected.clear();
        self.preview = None;
        self.rebuild_groups();
    }

    /// Returns to scope configuration and drops the review projection.
    pub fn reset_to_landing(&mut self) {
        self.phase = CleanPhase::Landing;
        self.reviewing = false;
        self.effective_scope = None;
        self.prompt = CleanLandingPrompt::None;
        self.set_rows(Vec::new());
        self.plan = None;
        self.selected.clear();
        self.preview = None;
        self.set_notice(None);
        self.failure_detail = None;
        self.result_counts = None;
        self.execution = None;
        self.found_so_far = 0;
    }

    pub fn failure_detail(&self) -> Option<&str> {
        self.failure_detail.as_deref()
    }

    #[cfg(test)]
    pub fn result_counts(&self) -> Option<(usize, usize)> {
        self.result_counts
    }

    pub fn suggestions(&self) -> &[PathBuf] {
        &self.suggestions
    }

    pub fn root_session_only(&self) -> bool {
        self.root_session_only
    }

    /// Cached row ids in current display order. The projection is rebuilt once
    /// per row-set or sort change and returned by reference.
    pub fn row_ids(&self) -> &[String] {
        &self.cached_ids
    }

    /// Row ids selected for cleanup, in display order. Protected and ignored
    /// rows are never recommended.
    pub fn recommended_ids(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter(|row| self.row_selectable(row) && row.selected_by_default)
            .map(|row| row.id.clone())
            .collect()
    }

    /// Global scope only arms an explicit confirmation; it never scans here.
    pub fn require_global_confirmation(&mut self) {
        if self.phase == CleanPhase::Landing {
            self.prompt = CleanLandingPrompt::ConfirmGlobal;
        }
    }

    /// Consumes the explicit global confirmation. True only once per prompt.
    pub fn take_global_confirmation(&mut self) -> bool {
        if self.prompt == CleanLandingPrompt::ConfirmGlobal {
            self.prompt = CleanLandingPrompt::None;
            true
        } else {
            false
        }
    }

    /// Project scope without a saved root asks for the first-use root setup.
    pub fn require_root(&mut self, suggestions: Vec<PathBuf>) {
        if self.phase == CleanPhase::Landing {
            self.prompt = CleanLandingPrompt::RootRequired;
            self.suggestions = suggestions;
        }
    }

    /// Cancels a confirmation or root prompt and returns to scope configuration.
    pub fn cancel_prompt(&mut self) {
        self.prompt = CleanLandingPrompt::None;
    }

    /// Records that a scope was accepted and a worker scan is starting.
    pub fn on_scan_started(&mut self, scope: CleanScope) {
        self.scan_progress = CleanScanProgress::default();
        self.scan_progress.capacity.incomplete = 1;
        self.selected_scope = scope;
        self.phase = CleanPhase::Scanning;
        self.prompt = CleanLandingPrompt::None;
        self.set_rows(Vec::new());
        self.plan = None;
        self.selected.clear();
        self.preview = None;
        self.set_notice(None);
        self.failure_detail = None;
        self.result_counts = None;
        self.execution = None;
        self.found_so_far = 0;
        self.last_scope = Some(scope);
        self.reviewing = false;
        self.effective_scope = Some(scope);
    }

    /// Latest in-flight discovery count. Older counts never move backwards.
    pub fn on_scan_progress(&mut self, found: usize) {
        if self.phase == CleanPhase::Scanning {
            self.found_so_far = self.found_so_far.max(found);
        }
    }

    /// Applies one accepted or failed scan payload and derives the phase.
    pub fn on_scan_result(&mut self, payload: CleanScanPayload) {
        let CleanScanPayload {
            rows,
            plan,
            outcome,
            detail,
        } = payload;
        self.set_rows(rows);
        self.plan = plan;
        self.plan_epoch += 1;
        self.selected.clear();
        self.preview = None;
        self.result_counts = None;
        self.execution = None;
        self.found_so_far = self.rows.len();
        let has_rows = !self.rows.is_empty();
        match outcome {
            CleanScanOutcome::Completed if has_rows => {
                self.phase = CleanPhase::Found;
                self.set_notice(None);
            }
            CleanScanOutcome::Completed => {
                self.phase = CleanPhase::AllClean;
                self.set_notice(Some(CleanOutcomeNotice::Empty));
            }
            // A partial report without rows cannot claim an empty machine.
            CleanScanOutcome::Partial if has_rows => {
                self.phase = CleanPhase::Found;
                self.set_notice(Some(CleanOutcomeNotice::Partial));
            }
            CleanScanOutcome::Partial => {
                self.phase = CleanPhase::Failed;
                self.set_notice(Some(CleanOutcomeNotice::Partial));
            }
            CleanScanOutcome::Canceled => {
                self.phase = CleanPhase::Failed;
                self.set_notice(Some(CleanOutcomeNotice::Canceled));
            }
            CleanScanOutcome::Failed => {
                self.phase = CleanPhase::Failed;
                self.set_notice(Some(CleanOutcomeNotice::Failed));
            }
        }
        self.failure_detail = detail;
        self.reviewing = false;
        self.rebuild_groups();
    }

    /// True when the row entered the cleanup selection. Inspect-only, protected,
    /// ignored, and non-candidate rows refuse without touching the selection.
    pub fn toggle_selected(&mut self, id: &str) -> bool {
        let Some(row) = self.row(id) else {
            return false;
        };
        if !self.row_selectable(row) {
            return false;
        }
        let selected = if self.selected.remove(id) {
            false
        } else {
            self.selected.insert(id.to_string());
            true
        };
        self.selection_changed();
        selected
    }

    /// Selects every conservatively recommended row. Returns the new count.
    ///
    /// The default selection is the projection of [`Self::recommended_ids`]:
    /// one definition of "recommended" drives both this action and the tests.
    pub fn select_recommended(&mut self) -> usize {
        for id in self.recommended_ids() {
            self.selected.insert(id);
        }
        self.selection_changed();
        self.selected.len()
    }

    pub fn clear_selected(&mut self) {
        self.selected.clear();
        self.selection_changed();
    }

    /// Cycles the row sort and reorders the display projection.
    pub fn cycle_sort(&mut self) -> CleanSort {
        self.sort = self.sort.cycle();
        self.apply_sort();
        self.sort
    }

    /// Reorders rows for the current sort. The plan order is never touched.
    pub fn apply_sort(&mut self) {
        match self.sort {
            CleanSort::ReportOrder => self.rows.sort_by_key(|row| row.plan_index),
            CleanSort::Name => self.rows.sort_by(|left, right| {
                (&left.rule_id, &left.path).cmp(&(&right.rule_id, &right.path))
            }),
            // Size order is a pure projection: report order breaks ties, so
            // the result never depends on the previously applied sort.
            CleanSort::Size => self
                .rows
                .sort_by_key(|row| (std::cmp::Reverse(row.bytes), row.plan_index)),
        }
        self.rebuild_row_projections();
    }

    /// Installs one row set and rebuilds the id index and the id projection
    /// exactly once.
    fn set_rows(&mut self, rows: Vec<CleanRow>) {
        self.rows = rows;
        self.rebuild_row_projections();
    }

    fn rebuild_row_projections(&mut self) {
        self.row_index.clear();
        self.cached_ids.clear();
        self.cached_ids.reserve(self.rows.len());
        for (index, row) in self.rows.iter().enumerate() {
            self.row_index.insert(row.id.clone(), index);
            self.cached_ids.push(row.id.clone());
        }
        self.rebuild_groups();
    }

    fn saved_plan_for_selection(&self) -> Option<UntrustedPlan> {
        if self.native_fixture.is_some() {
            return None;
        }
        let plan = self.plan.as_ref()?;
        // This page only authorizes Recycle Bin moves, even if a caller
        // accidentally installs an inconsistent row/selection projection.
        if self.selected.iter().any(|id| {
            !plan.targets.iter().any(|target| {
                target.id.as_str() == id
                    && matches!(target.intent, CleanupIntent::TrashProjectArtifact { .. })
            })
        }) {
            return None;
        }
        let targets = plan
            .targets
            .iter()
            .filter(|target| self.selected.contains(target.id.as_str()))
            .cloned()
            // Mirror `clean plan --select`: every explicitly selected target
            // carries `selected_by_default = true` in the saved plan, so the
            // GPUI digest matches the CLI digest for the same selection.
            .map(|mut target| {
                target.selected_by_default = true;
                target
            })
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return None;
        }
        Some(UntrustedPlan {
            version: plan.version,
            targets,
        })
    }

    /// Computes the live preview digest over the selected saved plan.
    ///
    /// The digest comes from `validate_plan` plus `confirmation_digest`; this
    /// never hashes locally. A validation or digest failure records
    /// `Unavailable` and refuses.
    pub fn preview_selection(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let Some(saved_plan) = self.saved_plan_for_selection() else {
            self.preview = None;
            self.set_notice(Some(CleanOutcomeNotice::Unavailable));
            return false;
        };
        let plan = match validate_plan(&saved_plan) {
            Ok(plan) => plan,
            Err(_) => {
                self.preview = None;
                self.set_notice(Some(CleanOutcomeNotice::Unavailable));
                return false;
            }
        };
        let selected_ids = saved_plan
            .targets
            .iter()
            .map(|target| target.id.clone())
            .collect::<Vec<_>>();
        let digest = match devsweep_core::execution::confirmation_digest(&plan, &selected_ids) {
            Ok(digest) => format!("sha256:{}", digest.as_str()),
            Err(_) => {
                self.preview = None;
                self.set_notice(Some(CleanOutcomeNotice::Unavailable));
                return false;
            }
        };
        self.preview = Some(CleanPreview {
            digest,
            count: selected_ids.len(),
            saved_plan,
            selected_ids,
            selected_snapshot: self.selected.clone(),
            plan_epoch: self.plan_epoch,
        });
        self.set_notice(None);
        true
    }

    /// Gate for execution. A missing preview records `ConfirmationRequired`;
    /// a preview that no longer matches the selection records `StalePreview`.
    /// Both refuse without touching any file.
    pub fn begin_execute(&mut self) -> Option<CleanExecutionStart> {
        if self.phase != CleanPhase::Found || self.selected.is_empty() {
            return None;
        }
        self.saved_plan_for_selection()?;
        let stale = match self.preview.as_ref() {
            None => {
                self.set_notice(Some(CleanOutcomeNotice::ConfirmationRequired));
                return None;
            }
            Some(preview) => {
                preview.plan_epoch != self.plan_epoch || preview.selected_snapshot != self.selected
            }
        };
        if stale {
            // A stale preview is invalidated: the review surface returns so
            // the user must produce a digest for the current selection again.
            self.preview = None;
            self.set_notice(Some(CleanOutcomeNotice::StalePreview));
            return None;
        }
        let preview = self.preview.as_ref().expect("preview survived the gate");
        let start = CleanExecutionStart {
            saved_plan: preview.saved_plan.clone(),
            selected: preview.selected_ids.clone(),
            digest: preview.digest.clone(),
        };
        self.phase = CleanPhase::Cleaning;
        self.set_notice(None);
        self.failure_detail = None;
        Some(start)
    }

    /// Records that cooperative cancellation was requested for an active worker.
    pub fn on_cancel_requested(&mut self) {
        if matches!(self.phase, CleanPhase::Scanning | CleanPhase::Cleaning) {
            self.set_notice(Some(CleanOutcomeNotice::Canceled));
        }
    }

    /// Applies one execution payload and derives the terminal phase.
    pub fn on_execute_result(&mut self, payload: CleanExecutePayload) {
        self.preview = None;
        self.execution = None;
        self.result_counts = None;
        match payload.report {
            None => {
                let failure = payload.failure.unwrap_or(CleanExecuteFailure::Failed);
                match failure {
                    CleanExecuteFailure::Stale => {
                        // The review surface returns so the user can preview again.
                        self.phase = CleanPhase::Found;
                        self.set_notice(Some(CleanOutcomeNotice::StalePreview));
                    }
                    CleanExecuteFailure::Unavailable => {
                        self.phase = CleanPhase::Failed;
                        self.set_notice(Some(CleanOutcomeNotice::Unavailable));
                    }
                    CleanExecuteFailure::Failed => {
                        self.phase = CleanPhase::Failed;
                        self.set_notice(Some(CleanOutcomeNotice::Failed));
                    }
                }
            }
            Some(report) => {
                self.result_counts = Some((report.succeeded, report.selected));
                let notice = if payload.canceled && report.succeeded < report.selected {
                    Some(CleanOutcomeNotice::Canceled)
                } else {
                    CleanOutcomeNotice::from_execution_report(&report)
                };
                self.set_notice(notice);
                self.phase = if notice == Some(CleanOutcomeNotice::Failed) {
                    CleanPhase::Failed
                } else {
                    CleanPhase::Completed
                };
                let mut groups = Vec::<CleanResultGroup>::new();
                let by_id: HashMap<_, _> = self
                    .groups
                    .iter()
                    .flat_map(|group| group.ids.iter().map(move |id| (id.as_str(), group)))
                    .collect();
                for outcome in &report.outcomes {
                    let original = by_id.get(outcome.target_id.as_str()).copied();
                    let group_id = original
                        .map(|group| group.id.as_str())
                        .unwrap_or("unmatched");
                    let position = groups
                        .iter()
                        .position(|entry| entry.group.id == group_id)
                        .unwrap_or_else(|| {
                            let group = original.cloned().unwrap_or(CleanGroup {
                                id: "unmatched".to_string(),
                                ecosystem: Ecosystem::Generic,
                                kind: TargetKind::ToolCache,
                                review_only: true,
                                ids: Vec::new(),
                                selectable_ids: Vec::new(),
                                capacity: CleanCapacity::default(),
                                recommended: CleanCapacity::default(),
                                selection: GroupSelection::None,
                                selected_count: 0,
                            });
                            groups.push(CleanResultGroup {
                                group,
                                outcomes: Vec::new(),
                                moved: CleanCapacity::default(),
                                moved_count: 0,
                                failed: 0,
                                skipped: 0,
                            });
                            groups.len() - 1
                        });
                    let entry = &mut groups[position];
                    match &outcome.status {
                        OutcomeStatus::Succeeded
                            if outcome.action == ActionKind::MoveToTrash && !report.dry_run =>
                        {
                            entry.moved_count += 1;
                            match outcome.estimated_recoverable {
                                CapacityEstimate::Verified { bytes } => {
                                    entry.moved.add(bytes, true)
                                }
                                CapacityEstimate::Partial { lower_bound_bytes } => {
                                    entry.moved.add(lower_bound_bytes, false)
                                }
                                CapacityEstimate::Unknown => entry.moved.add(0, false),
                            }
                        }
                        OutcomeStatus::Failed { .. } => entry.failed += 1,
                        OutcomeStatus::Skipped { .. } => entry.skipped += 1,
                        _ => {}
                    }
                    entry.outcomes.push(outcome.clone());
                }
                let moved = groups
                    .iter()
                    .fold(CleanCapacity::default(), |mut sum, group| {
                        sum.bytes = sum.bytes.saturating_add(group.moved.bytes);
                        sum.incomplete += group.moved.incomplete;
                        sum
                    });
                let moved_count = groups.iter().map(|group| group.moved_count).sum();
                let history_incomplete = payload.canceled
                    || report.outcomes.iter().any(|outcome| {
                        let message = match &outcome.status {
                            OutcomeStatus::Failed { message } => message,
                            OutcomeStatus::Skipped { reason } => reason,
                            _ => return false,
                        };
                        message.contains("audit")
                            || message.contains("canceled")
                            || message.contains("halted")
                    });
                let outcome_index = groups
                    .iter()
                    .enumerate()
                    .flat_map(|(group_index, group)| {
                        group
                            .outcomes
                            .iter()
                            .enumerate()
                            .map(move |(index, outcome)| {
                                (outcome.target_id.as_str().to_string(), (group_index, index))
                            })
                    })
                    .collect();
                self.execution = Some(CleanExecutionSummary {
                    outcome_index,
                    groups,
                    moved,
                    moved_count,
                    history_incomplete,
                    selected: report.selected,
                    succeeded: report.succeeded,
                    failed: report.failed,
                    skipped: report.skipped,
                    digest: Some(format!("sha256:{}", report.confirmation_digest.as_str())),
                    canceled: payload.canceled,
                    report,
                });
            }
        }
    }

    /// Truthful history entry for the last terminal execution, if any.
    pub fn history_entry(&self, locale: Locale) -> Option<String> {
        let execution = self.execution.as_ref()?;
        let entry = if execution.canceled && execution.succeeded < execution.selected {
            copy::history_clean_canceled(
                locale,
                execution.selected,
                execution.succeeded,
                execution.failed,
                execution.skipped,
            )
        } else {
            copy::history_clean_executed(
                locale,
                execution.selected,
                execution.succeeded,
                execution.failed,
                execution.skipped,
                execution.digest.as_deref().unwrap_or(""),
            )
        };
        Some(format!(
            "{entry} {} {}",
            copy::clean_moved_summary(
                locale,
                execution.moved_count,
                &execution.moved.label(locale)
            ),
            copy::clean_history_evidence(locale, execution.history_incomplete)
        ))
    }

    /// Explicit display-only native scenes. They contain no executable plan;
    /// neither scanning nor execution is allowed while this marker is present.
    pub fn install_native_fixture(&mut self, state: &str) -> bool {
        if !matches!(
            state,
            "landing"
                | "scanning"
                | "scanned"
                | "review"
                | "previewed"
                | "complete"
                | "mixed"
                | "canceled"
                | "audit-failed"
        ) {
            return false;
        }
        *self = Self::new();
        self.native_fixture = Some(state.to_string());
        if state == "landing" {
            return true;
        }
        let rows = (0..4)
            .map(|index| CleanRow {
                id: format!("fixture-{index}"),
                plan_index: index,
                rule_id: if index == 3 {
                    "rust.target"
                } else {
                    "python.pytest_cache"
                }
                .to_string(),
                provider: if index == 3 { "rust" } else { "python" }.to_string(),
                scope: CleanRowScope::Project {
                    root: "C:/review-fixture".to_string(),
                },
                path: Some(format!("C:/review-fixture/project-{index}/.pytest_cache")),
                bytes: 4096 * (u64::from(index) + 1),
                size_complete: index != 2,
                inspect_only: false,
                ecosystem: if index == 3 {
                    Ecosystem::Rust
                } else {
                    Ecosystem::Python
                },
                kind: TargetKind::TestCache,
                recycle_eligible: index != 3,
                selected_by_default: true,
            })
            .collect::<Vec<_>>();
        self.on_scan_result(CleanScanPayload {
            rows,
            plan: None,
            outcome: CleanScanOutcome::Completed,
            detail: None,
        });
        self.last_scope = Some(CleanScope::Project);
        self.effective_scope = Some(CleanScope::Project);
        if state == "scanning" {
            let observations = CleanScanProgress {
                found: 3,
                capacity: CleanCapacity {
                    bytes: 24576,
                    incomplete: 1,
                },
                groups: vec![(
                    Ecosystem::Python,
                    TargetKind::TestCache,
                    3,
                    CleanCapacity {
                        bytes: 24576,
                        incomplete: 1,
                    },
                )],
            };
            self.phase = CleanPhase::Scanning;
            self.on_scan_observations(observations);
        } else if matches!(state, "review" | "previewed") {
            self.toggle_selected("fixture-0");
        } else if matches!(state, "complete" | "mixed" | "canceled" | "audit-failed") {
            let mut report = ExecutionReport {
                dry_run: false,
                selected: 3,
                attempted: 3,
                succeeded: 0,
                failed: 0,
                skipped: 0,
                failures: Vec::new(),
                outcomes: Vec::new(),
                notes: Vec::new(),
                estimated_recoverable: Default::default(),
                confirmation_digest: ConfirmationDigest::new("00".repeat(32)),
                audit_log: None,
            };
            for index in 0..3 {
                let target_id = TargetId::new(format!("fixture-{index}"));
                let status = if state == "audit-failed" && index == 0 {
                    OutcomeStatus::Failed {
                        message:
                            "fixture: audit terminal persistence failed; physical result unknown"
                                .to_string(),
                    }
                } else if state == "canceled" || (state == "audit-failed" && index > 0) {
                    OutcomeStatus::Skipped {
                        reason: "fixture: canceled or halted before action started".to_string(),
                    }
                } else if state == "mixed" && index == 1 {
                    OutcomeStatus::Failed {
                        message: "fixture: simulated access denied".to_string(),
                    }
                } else if state == "mixed" && index == 2 {
                    OutcomeStatus::Skipped {
                        reason: "fixture: simulated safety skip".to_string(),
                    }
                } else {
                    OutcomeStatus::Succeeded
                };
                match &status {
                    OutcomeStatus::Succeeded => report.succeeded += 1,
                    OutcomeStatus::Failed { message } => {
                        report.failed += 1;
                        report
                            .failures
                            .push(devsweep_core::execution::ActionFailure {
                                target_id: target_id.clone(),
                                message: message.clone(),
                            });
                    }
                    OutcomeStatus::Skipped { .. } => report.skipped += 1,
                }
                report.outcomes.push(TargetOutcome {
                    target_id,
                    action: ActionKind::MoveToTrash,
                    status,
                    estimated_recoverable: if index == 2 {
                        CapacityEstimate::Unknown
                    } else {
                        CapacityEstimate::Verified {
                            bytes: 4096 * (index + 1),
                        }
                    },
                });
            }
            self.on_execute_result(CleanExecutePayload {
                report: Some(report),
                failure: None,
                canceled: state == "canceled",
            });
        }
        true
    }

    /// Test-only accepted-report injection for window and selection tests.
    #[cfg(test)]
    pub(crate) fn install_rows_for_test(&mut self, mut rows: Vec<CleanRow>) {
        for (index, row) in rows.iter_mut().enumerate() {
            row.plan_index = u32::try_from(index).unwrap_or(u32::MAX);
        }
        self.set_rows(rows);
        self.plan_epoch += 1;
        self.phase = CleanPhase::Found;
        self.reviewing = true;
        self.prompt = CleanLandingPrompt::None;
        self.selected.clear();
        self.preview = None;
        self.set_notice(None);
    }
}

/// True when one absolute protection entry covers a row ownership path.
fn path_covers(protected: &str, path: &str) -> bool {
    let protected = Path::new(protected);
    protected.is_absolute() && Path::new(path).starts_with(protected)
}

impl Default for CleanWorkbench {
    fn default() -> Self {
        Self::new()
    }
}

/// Bound a diagnostic for the panel. Control characters never reach the UI.
pub fn bounded_detail(text: &str) -> String {
    const CAP: usize = 200;
    let cleaned: String = text
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.chars().count() <= CAP {
        return trimmed.to_string();
    }
    let mut bounded: String = trimmed.chars().take(CAP).collect();
    bounded.push('…');
    bounded
}

/// Runs one worker scan through the shared core scan service.
///
/// The caller decides the scope; global scope requires an explicit user
/// confirmation before this is invoked. Progress reports the observed count.
pub fn scan_clean_rows(
    options: &ScanOptions,
    cancel: &Arc<FlagCancelObserver>,
    on_progress: &mut dyn FnMut(CleanScanProgress),
) -> CleanScanPayload {
    let run = Sweeper::default().full_scan_report_run_with_cancel(
        options,
        &mut |progress| {
            if let Some(partial) = progress.partial.as_ref()
                && let Ok(snapshot) = devsweep_core::scan::scan_preview_from_plan(partial)
            {
                on_progress(CleanScanProgress::from_snapshot(snapshot));
            }
        },
        Some(cancel),
    );
    match run {
        Ok(ScanReportRunOutcome::Completed(report)) => scan_payload_from_report(report),
        Ok(ScanReportRunOutcome::Canceled) => CleanScanPayload {
            rows: Vec::new(),
            plan: None,
            outcome: CleanScanOutcome::Canceled,
            detail: None,
        },
        Err(error) => CleanScanPayload {
            rows: Vec::new(),
            plan: None,
            outcome: CleanScanOutcome::Failed,
            detail: Some(bounded_detail(&error.to_string())),
        },
    }
}

fn scan_payload_from_report(report: ScanReport) -> CleanScanPayload {
    let rows = report
        .plan
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| clean_row_from_untrusted(target, index))
        .collect::<Vec<_>>();
    let detail = report
        .health
        .diagnostics
        .first()
        .map(|diagnostic| bounded_detail(&diagnostic.detail));
    let outcome = if report.health.is_complete() {
        CleanScanOutcome::Completed
    } else {
        CleanScanOutcome::Partial
    };
    CleanScanPayload {
        rows,
        plan: Some(report.plan),
        outcome,
        detail,
    }
}

/// Bare hex from a displayed `sha256:` digest. Anything else is not an
/// executor identity and must not be sent through as if it were one.
fn canonical_execution_digest(displayed: &str) -> Option<&str> {
    let hex = displayed.strip_prefix("sha256:")?;
    let well_formed = hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    well_formed.then_some(hex)
}

/// Runs one worker execution through the shared core executor.
///
/// The plan is revalidated live, and the core executor re-checks the stored
/// digest before any dispatched action. A stale digest returns without moving
/// anything. Production always uses the shared `Executor` default, which owns
/// the Recycle Bin move.
pub fn execute_clean_plan(
    saved_plan: &UntrustedPlan,
    selected: Vec<TargetId>,
    expected_digest: &str,
    cancel: &Arc<FlagCancelObserver>,
) -> CleanExecutePayload {
    if saved_plan
        .targets
        .iter()
        .any(|target| !matches!(target.intent, CleanupIntent::TrashProjectArtifact { .. }))
    {
        return CleanExecutePayload {
            report: None,
            failure: Some(CleanExecuteFailure::Unavailable),
            canceled: cancel.is_cancel_requested(),
        };
    }
    let plan = match validate_plan(saved_plan) {
        Ok(plan) => plan,
        Err(_) => {
            return CleanExecutePayload {
                report: None,
                failure: Some(CleanExecuteFailure::Unavailable),
                canceled: cancel.is_cancel_requested(),
            };
        }
    };
    // The panel and the CLI JSON show `sha256:` plus 64 hex digits. The
    // executor compares the bare hex, and the CLI strips the same prefix
    // before that comparison. Passing the displayed string makes every live
    // digest look stale.
    let Some(hex) = canonical_execution_digest(expected_digest) else {
        return CleanExecutePayload {
            report: None,
            failure: Some(CleanExecuteFailure::Unavailable),
            canceled: cancel.is_cancel_requested(),
        };
    };
    let request = ExecutionRequest {
        execute: true,
        audit_log: None,
        selected,
        expected_digest: Some(ConfirmationDigest::new(hex)),
        cancel: Some(Arc::clone(cancel)),
    };
    match Executor::default().run_plan_with_progress(&plan, request, |_| {}) {
        Ok(report) => CleanExecutePayload {
            report: Some(report),
            failure: None,
            canceled: cancel.is_cancel_requested(),
        },
        Err(error) => {
            let failure = match error.downcast_ref::<ExecutionError>() {
                Some(ExecutionError::StaleConfirmation { .. }) => CleanExecuteFailure::Stale,
                _ => CleanExecuteFailure::Failed,
            };
            CleanExecutePayload {
                report: None,
                failure: Some(failure),
                canceled: cancel.is_cancel_requested(),
            }
        }
    }
}

#[cfg(test)]
pub(crate) use tests::{cleanup, isolated_multi_cache_project};

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    pub(crate) fn isolated_multi_cache_project() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-{}-review",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing a project fixture outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".pytest_cache")).unwrap();
        fs::create_dir_all(root.join(".mypy_cache")).unwrap();
        fs::create_dir_all(root.join(".venv")).unwrap();
        fs::write(root.join("pyproject.toml"), "[project]\nname='c11'\n").unwrap();
        fs::write(root.join(".pytest_cache").join("payload"), "pytest bytes").unwrap();
        fs::write(root.join(".mypy_cache").join("payload"), "mypy bytes").unwrap();
        fs::write(root.join(".venv").join("payload"), "venv bytes").unwrap();
        root
    }

    pub(crate) fn cleanup(root: &std::path::Path) {
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing to remove a directory outside the temp prefix"
        );
        let _ = fs::remove_dir_all(root);
    }

    fn project_scan(root: &std::path::Path) -> CleanScanPayload {
        let options = ScanOptions {
            include_projects: true,
            include_global: false,
            roots: vec![root.to_path_buf()],
        };
        let cancel = Arc::new(FlagCancelObserver::new());
        scan_clean_rows(&options, &cancel, &mut |_| {})
    }

    fn row_for<'a>(payload: &'a CleanScanPayload, rule_id: &str) -> &'a CleanRow {
        payload
            .rows
            .iter()
            .find(|row| row.rule_id == rule_id)
            .unwrap_or_else(|| panic!("missing {rule_id} row in {:?}", payload.rows))
    }

    #[test]
    fn landing_requires_an_explicit_scope_before_any_scan() {
        let mut workbench = CleanWorkbench::new();
        assert_eq!(workbench.phase(), CleanPhase::Landing);
        assert_eq!(workbench.prompt(), CleanLandingPrompt::None);
        assert!(workbench.rows().is_empty());
        assert!(workbench.row_ids().is_empty());

        assert_eq!(workbench.selected_scope(), CleanScope::Project);
        workbench.select_scope(CleanScope::Global);
        assert_eq!(workbench.selected_scope(), CleanScope::Global);
        assert_eq!(workbench.prompt(), CleanLandingPrompt::None);
        assert_eq!(workbench.phase(), CleanPhase::Landing);

        // Global scope arms a confirmation only; it never starts a scan here.
        workbench.require_global_confirmation();
        assert_eq!(workbench.prompt(), CleanLandingPrompt::ConfirmGlobal);
        assert_eq!(workbench.phase(), CleanPhase::Landing);
        assert!(workbench.rows().is_empty());

        // The confirmation is consumed exactly once.
        assert!(workbench.take_global_confirmation());
        assert_eq!(workbench.prompt(), CleanLandingPrompt::None);
        assert!(!workbench.take_global_confirmation());

        // Project scope without a saved root asks for the first-use root setup.
        let suggestion = PathBuf::from("C:/example/projects");
        workbench.require_root(vec![suggestion.clone()]);
        assert_eq!(workbench.prompt(), CleanLandingPrompt::RootRequired);
        assert_eq!(workbench.suggestions(), [suggestion]);
        assert_eq!(workbench.phase(), CleanPhase::Landing);
        workbench.cancel_prompt();
        assert_eq!(workbench.prompt(), CleanLandingPrompt::None);
    }

    #[test]
    fn scan_progress_only_moves_forward_while_scanning() {
        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_progress(5);
        assert_eq!(workbench.found_so_far(), 0);
        workbench.on_scan_started(CleanScope::Project);
        assert_eq!(workbench.scan_progress().capacity.bytes, 0);
        assert_eq!(workbench.scan_progress().capacity.incomplete, 1);
        assert_eq!(workbench.phase(), CleanPhase::Scanning);
        workbench.on_scan_progress(5);
        workbench.on_scan_progress(2);
        assert_eq!(workbench.found_so_far(), 5);
        workbench.on_scan_result(CleanScanPayload {
            rows: Vec::new(),
            plan: None,
            outcome: CleanScanOutcome::Completed,
            detail: None,
        });
        workbench.on_scan_progress(99);
        assert_eq!(workbench.phase(), CleanPhase::AllClean);
        assert_eq!(workbench.found_so_far(), 0);
    }

    #[test]
    fn scan_outcomes_map_to_phases_and_notices() {
        let row = test_row("row-1", 10);
        let plan = UntrustedPlan::empty();
        let payload = |outcome, rows: Vec<CleanRow>| CleanScanPayload {
            rows,
            plan: Some(plan.clone()),
            outcome,
            detail: None,
        };

        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_result(payload(CleanScanOutcome::Completed, vec![row.clone()]));
        assert_eq!(workbench.phase(), CleanPhase::Found);
        assert_eq!(workbench.notice(), None);

        workbench.on_scan_result(payload(CleanScanOutcome::Completed, Vec::new()));
        assert_eq!(workbench.phase(), CleanPhase::AllClean);
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Empty));

        workbench.on_scan_result(payload(CleanScanOutcome::Partial, vec![row.clone()]));
        assert_eq!(workbench.phase(), CleanPhase::Found);
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Partial));

        // A partial report without rows never claims an empty machine.
        let mut partial = payload(CleanScanOutcome::Partial, Vec::new());
        partial.detail = Some("root is unavailable".to_string());
        workbench.on_scan_result(partial);
        assert_eq!(workbench.phase(), CleanPhase::Failed);
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Partial));
        assert_eq!(workbench.failure_detail(), Some("root is unavailable"));

        workbench.on_scan_result(payload(CleanScanOutcome::Canceled, Vec::new()));
        assert_eq!(workbench.phase(), CleanPhase::Failed);
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Canceled));

        workbench.on_scan_result(payload(CleanScanOutcome::Failed, Vec::new()));
        assert_eq!(workbench.phase(), CleanPhase::Failed);
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Failed));
    }

    #[test]
    fn a_new_scan_result_clears_selection_and_preview() {
        let root = isolated_multi_cache_project();
        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_started(CleanScope::Project);
        workbench.on_scan_result(project_scan(&root));
        let pytest = workbench_row_id(&workbench, "python.pytest_cache");
        assert!(workbench.toggle_selected(&pytest));
        assert!(workbench.preview_selection());
        assert!(workbench.preview().is_some());

        workbench.on_scan_started(CleanScope::Project);
        assert!(workbench.rows().is_empty());
        assert_eq!(workbench.selected_count(), 0);
        assert!(workbench.preview().is_none());
        assert_eq!(workbench.notice(), None);

        workbench.on_scan_result(project_scan(&root));
        assert!(workbench.preview().is_none());
        assert!(
            workbench
                .rows()
                .iter()
                .all(|row| !workbench.is_selected(&row.id))
        );
        cleanup(&root);
    }

    fn workbench_row_id(workbench: &CleanWorkbench, rule_id: &str) -> String {
        workbench
            .rows()
            .iter()
            .find(|row| row.rule_id == rule_id)
            .map(|row| row.id.clone())
            .unwrap_or_else(|| panic!("missing {rule_id} row"))
    }

    #[test]
    fn project_scan_populates_rows_and_metadata() {
        let root = isolated_multi_cache_project();
        let payload = project_scan(&root);
        assert_eq!(payload.outcome, CleanScanOutcome::Completed);
        assert!(payload.plan.is_some());
        assert!(payload.rows.len() >= 3, "rows: {:?}", payload.rows);

        let pytest = row_for(&payload, "python.pytest_cache");
        assert_eq!(pytest.provider, "python");
        assert!(pytest.selectable());
        // The shared ranking policy never preselects project-scope targets
        // (`ranking.project_opt_in`), so a project scan recommends nothing
        // and every target needs an explicit opt-in.
        assert!(
            !pytest.selected_by_default,
            "project targets require explicit opt-in: {pytest:?}"
        );
        assert!(pytest.size_complete);
        assert!(pytest.bytes > 0);
        // The scanner normalizes both the ownership path and the scope root
        // to canonical (verbatim) paths on Windows.
        let expected_path = fs::canonicalize(root.join(".pytest_cache")).expect("canonical path");
        let expected_root = fs::canonicalize(&root).expect("canonical root");
        let path = pytest.path.as_deref().expect("pytest path");
        assert_eq!(path, expected_path.display().to_string());
        assert_eq!(
            pytest.scope,
            CleanRowScope::Project {
                root: expected_root.display().to_string()
            }
        );

        let venv = row_for(&payload, "python.venv_dot");
        assert!(venv.selectable());
        assert!(!venv.selected_by_default);
        assert!(
            payload.rows.iter().all(|row| !row.selected_by_default),
            "project scan must not recommend rows: {:?}",
            payload.rows
        );

        for (index, row) in payload.rows.iter().enumerate() {
            assert_eq!(row.plan_index as usize, index);
            assert!(row.id.starts_with(&row.rule_id));
        }
        cleanup(&root);
    }

    #[test]
    fn empty_project_scan_reads_all_clean() {
        static EMPTY: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-{}-empty",
            std::process::id(),
            EMPTY.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let payload = project_scan(&root);
        assert_eq!(payload.outcome, CleanScanOutcome::Completed);
        assert!(payload.rows.is_empty());
        cleanup(&root);
    }

    #[test]
    fn missing_project_root_scan_is_partial_without_rows() {
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-missing-root",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let payload = project_scan(&root);
        assert_eq!(payload.outcome, CleanScanOutcome::Partial);
        assert!(payload.rows.is_empty());
        let detail = payload.detail.expect("root failure detail");
        assert!(!detail.is_empty());
        assert!(detail.chars().count() <= 201, "detail: {detail}");
        assert!(!detail.contains('\n'), "detail must be one line: {detail}");
    }

    #[test]
    fn canceled_scan_reports_canceled_outcome() {
        let root = isolated_multi_cache_project();
        let options = ScanOptions {
            include_projects: true,
            include_global: false,
            roots: vec![root.clone()],
        };
        let cancel = Arc::new(FlagCancelObserver::new());
        cancel.request_cancel();
        let payload = scan_clean_rows(&options, &cancel, &mut |_| {});
        assert_eq!(payload.outcome, CleanScanOutcome::Canceled);
        assert!(payload.rows.is_empty());
        assert!(payload.plan.is_none());
        cleanup(&root);
    }

    #[test]
    fn preview_digest_matches_cli_parity_helper() {
        let root = isolated_multi_cache_project();
        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_started(CleanScope::Project);
        workbench.on_scan_result(project_scan(&root));
        let pytest = workbench_row_id(&workbench, "python.pytest_cache");
        workbench.clear_selected();
        assert!(workbench.toggle_selected(&pytest));
        assert!(workbench.preview_selection());
        let preview = workbench.preview().expect("preview").clone();
        assert!(preview.digest.starts_with("sha256:"));
        assert_eq!(preview.count, 1);
        assert_eq!(preview.selected_ids.len(), 1);
        assert_eq!(preview.saved_plan.targets.len(), 1);
        assert_eq!(preview.saved_plan.targets[0].id.as_str(), pytest);
        // Parity with `clean plan --select`: the saved plan marks the
        // explicit selection, so the digest covers the same plan shape.
        assert!(
            preview.saved_plan.targets[0].selected_by_default,
            "saved plan must mark explicit selections like the CLI"
        );

        // The CLI-parity helper for the same fixture and the same single
        // selection must produce the identical digest.
        let parity =
            crate::clean::pytest_cache_confirmation_digest(&root).expect("pytest parity preview");
        assert_eq!(preview.digest, parity.digest);
        cleanup(&root);
    }

    #[test]
    fn stale_preview_refuses_and_missing_preview_needs_confirmation() {
        let root = isolated_multi_cache_project();
        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_started(CleanScope::Project);
        workbench.on_scan_result(project_scan(&root));
        let pytest = workbench_row_id(&workbench, "python.pytest_cache");
        let mypy = workbench_row_id(&workbench, "python.mypy_cache");

        // No preview yet: the confirmation is required.
        assert!(workbench.toggle_selected(&pytest));
        assert!(workbench.begin_execute().is_none());
        assert_eq!(
            workbench.notice(),
            Some(CleanOutcomeNotice::ConfirmationRequired)
        );
        assert_eq!(workbench.phase(), CleanPhase::Found);

        // A preview over one selection is stale as soon as the selection changes.
        assert!(workbench.preview_selection());
        assert!(workbench.toggle_selected(&mypy));
        workbench.set_notice(None);
        assert!(workbench.begin_execute().is_none());
        assert_eq!(
            workbench.notice(),
            Some(CleanOutcomeNotice::ConfirmationRequired)
        );
        assert!(workbench.preview().is_none());
        assert_eq!(workbench.phase(), CleanPhase::Found);
        assert!(
            workbench.preview().is_none(),
            "the stale preview is invalidated by the refusal"
        );

        // An empty selection refuses without a notice.
        workbench.clear_selected();
        workbench.set_notice(None);
        assert!(workbench.begin_execute().is_none());
        assert_eq!(workbench.notice(), None);
        assert_eq!(workbench.phase(), CleanPhase::Found);

        // A matching preview starts the execution handoff. Both selected ids must
        // be covered by the saved plan and by the stored digest.
        assert!(workbench.toggle_selected(&pytest));
        assert!(workbench.toggle_selected(&mypy));
        assert!(workbench.preview_selection());
        let start = workbench.begin_execute().expect("execution gate passes");
        assert_eq!(workbench.phase(), CleanPhase::Cleaning);
        assert!(start.digest.starts_with("sha256:"));
        assert!(
            workbench.preview().is_some(),
            "the in-flight digest stays visible while cleaning"
        );
        assert_eq!(start.saved_plan.targets.len(), 2);
        assert_eq!(start.selected.len(), 2);
        assert_eq!(pytest, start.selected[0].as_str());
        assert_eq!(mypy, start.selected[1].as_str());
        cleanup(&root);
    }

    #[test]
    #[cfg_attr(
        not(windows),
        ignore = "Windows-only behavior (Recycle Bin, Win32 paths, cmd.exe or file locking)"
    )]
    fn execute_worker_accepts_the_live_prefixed_digest_and_refuses_a_stale_one() {
        let root = isolated_multi_cache_project();
        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_started(CleanScope::Project);
        workbench.on_scan_result(project_scan(&root));
        let pytest = workbench_row_id(&workbench, "python.pytest_cache");
        assert!(workbench.toggle_selected(&pytest));
        assert!(workbench.preview_selection());
        let preview = workbench.preview().expect("preview").clone();

        let cancel = Arc::new(FlagCancelObserver::new());
        let mut wrong_digest = format!("sha256:{}", "0".repeat(64));
        if wrong_digest == preview.digest {
            wrong_digest = format!("sha256:{}", "1".repeat(64));
        }
        let stale = execute_clean_plan(
            &preview.saved_plan,
            preview.selected_ids.clone(),
            &wrong_digest,
            &cancel,
        );
        assert_eq!(stale.failure, Some(CleanExecuteFailure::Stale));
        assert!(stale.report.is_none());
        assert!(
            root.join(".pytest_cache").join("payload").is_file(),
            "a stale digest must not move the fixture"
        );
        assert!(root.join(".mypy_cache").join("payload").is_file());

        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_started(CleanScope::Project);
        workbench.on_scan_result(project_scan(&root));
        workbench.on_execute_result(stale.clone());
        assert_eq!(workbench.phase(), CleanPhase::Found);
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::StalePreview));
        assert!(workbench.preview().is_none());
        assert!(workbench.history_entry(Locale::En).is_none());

        let missing_prefix = execute_clean_plan(
            &preview.saved_plan,
            preview.selected_ids.clone(),
            preview.digest.strip_prefix("sha256:").expect("prefix"),
            &cancel,
        );
        assert_eq!(
            missing_prefix.failure,
            Some(CleanExecuteFailure::Unavailable)
        );
        assert!(root.join(".pytest_cache").join("payload").is_file());

        let moved = execute_clean_plan(
            &preview.saved_plan,
            preview.selected_ids.clone(),
            &preview.digest,
            &cancel,
        );
        assert_eq!(moved.failure, None, "live digest must not be stale");
        let report = moved.report.expect("execution report");
        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failed, 0);
        assert!(
            !root.join(".pytest_cache").join("payload").is_file(),
            "the live digest must move the isolated pytest payload"
        );
        assert!(
            root.join(".mypy_cache").join("payload").is_file(),
            "an unselected cache must stay"
        );
        cleanup(&root);
    }

    #[test]
    fn execute_worker_reports_unavailable_for_an_invalid_saved_plan() {
        let broken = UntrustedPlan {
            version: 2,
            targets: vec![UntrustedTarget {
                id: TargetId::new("broken:target"),
                rule_id: "no.such.rule".to_string(),
                scope: Scope::Global,
                ecosystem: devsweep_core::model::Ecosystem::Generic,
                kind: devsweep_core::model::TargetKind::PackageCache,
                path: None,
                estimated_bytes: 0,
                size_complete: true,
                sizing_warnings: Vec::new(),
                last_modified: None,
                risk: devsweep_core::model::RiskLevel::Low,
                reversible: true,
                selected_by_default: false,
                evidence: Vec::new(),
                intent: CleanupIntent::InspectOnly {
                    rule_id: "no.such.rule".to_string(),
                },
            }],
        };
        let cancel = Arc::new(FlagCancelObserver::new());
        let payload = execute_clean_plan(
            &broken,
            vec![TargetId::new("broken:target")],
            "sha256:00",
            &cancel,
        );
        assert_eq!(payload.failure, Some(CleanExecuteFailure::Unavailable));
        assert!(payload.report.is_none());

        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_started(CleanScope::Project);
        workbench.on_scan_result(CleanScanPayload {
            rows: vec![test_row("row-1", 4)],
            plan: Some(UntrustedPlan::empty()),
            outcome: CleanScanOutcome::Completed,
            detail: None,
        });
        workbench.on_execute_result(payload);
        assert_eq!(workbench.phase(), CleanPhase::Failed);
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Unavailable));
    }

    #[test]
    fn execution_reports_map_every_terminal_notice() {
        let payload = |succeeded, failed, skipped| CleanExecutePayload {
            report: Some(test_execution_report(succeeded, failed, skipped)),
            failure: None,
            canceled: false,
        };

        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_started(CleanScope::Project);
        workbench.on_scan_result(CleanScanPayload {
            rows: vec![test_row("row-1", 4)],
            plan: Some(UntrustedPlan::empty()),
            outcome: CleanScanOutcome::Completed,
            detail: None,
        });

        workbench.on_execute_result(payload(1, 0, 0));
        assert_eq!(workbench.phase(), CleanPhase::Completed);
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::TrashReported));
        assert_eq!(workbench.result_counts(), Some((1, 1)));
        let entry = workbench.history_entry(Locale::En).expect("history entry");
        assert!(entry.contains("1 succeeded"));
        assert!(entry.contains("sha256:"));
        assert!(entry.contains("Moved to Recycle Bin"));
        assert!(entry.contains("not freed space"));

        workbench.on_execute_result(payload(1, 1, 0));
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Mixed));
        assert_eq!(workbench.phase(), CleanPhase::Completed);

        workbench.on_execute_result(payload(0, 1, 0));
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Failed));
        assert_eq!(workbench.phase(), CleanPhase::Failed);

        workbench.on_execute_result(payload(0, 0, 1));
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Partial));
        assert_eq!(workbench.phase(), CleanPhase::Completed);

        workbench.on_execute_result(payload(1, 0, 1));
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Mixed));

        // A canceled run that did not move everything reports canceled.
        workbench.on_execute_result(CleanExecutePayload {
            report: Some(test_execution_report(0, 0, 1)),
            failure: None,
            canceled: true,
        });
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::Canceled));
        assert_eq!(workbench.phase(), CleanPhase::Completed);
        let entry = workbench
            .history_entry(Locale::ZhCn)
            .expect("canceled history entry");
        assert!(entry.contains("清理已取消"));

        // A canceled run where everything still moved reports the truth.
        workbench.on_execute_result(CleanExecutePayload {
            report: Some(test_execution_report(1, 0, 0)),
            failure: None,
            canceled: true,
        });
        assert_eq!(workbench.notice(), Some(CleanOutcomeNotice::TrashReported));
        assert_eq!(workbench.phase(), CleanPhase::Completed);
    }

    fn test_execution_report(succeeded: usize, failed: usize, skipped: usize) -> ExecutionReport {
        use devsweep_core::execution::{
            ActionFailure, ActionKind, CapacityEstimate, OutcomeStatus, TargetOutcome,
        };
        use devsweep_core::model::ScanTotals;

        let total = succeeded + failed + skipped;
        let mut outcomes = Vec::new();
        let mut failures = Vec::new();
        for index in 0..total {
            let target_id = TargetId::new(format!("row-{index}"));
            let status = if index < succeeded {
                OutcomeStatus::Succeeded
            } else if index < succeeded + failed {
                let message = "fixture failure".to_string();
                failures.push(ActionFailure {
                    target_id: target_id.clone(),
                    message: message.clone(),
                });
                OutcomeStatus::Failed { message }
            } else {
                OutcomeStatus::Skipped {
                    reason: "fixture skip".to_string(),
                }
            };
            outcomes.push(TargetOutcome {
                target_id,
                action: ActionKind::MoveToTrash,
                status,
                estimated_recoverable: CapacityEstimate::Verified { bytes: 1 },
            });
        }
        ExecutionReport {
            dry_run: false,
            selected: total,
            attempted: total,
            succeeded,
            failed,
            skipped,
            failures,
            outcomes,
            notes: Vec::new(),
            estimated_recoverable: ScanTotals::default(),
            confirmation_digest: ConfirmationDigest::new("ab".repeat(32)),
            audit_log: None,
        }
    }

    #[test]
    fn selection_survives_navigation_sort_and_window_moves() {
        let mut workbench = CleanWorkbench::new();
        let mut rows = (0..10_000)
            .map(|index| test_row(&format!("clean-{index}"), index as u64))
            .collect::<Vec<_>>();
        rows.push(test_inspect_only_row("inspect-only-1"));
        workbench.install_rows_for_test(rows);
        assert_eq!(workbench.phase(), CleanPhase::Found);

        let far = "clean-9000".to_string();
        assert!(workbench.toggle_selected(&far));
        assert!(workbench.is_selected(&far));

        // Sorting is a projection: it never drops the selection.
        assert_eq!(workbench.cycle_sort(), CleanSort::Name);
        assert!(workbench.is_selected(&far));
        assert_eq!(workbench.cycle_sort(), CleanSort::Size);
        assert!(workbench.is_selected(&far));
        assert_eq!(workbench.cycle_sort(), CleanSort::ReportOrder);
        assert!(workbench.is_selected(&far));
        assert_eq!(workbench.row_ids()[0], "clean-0");
        assert_eq!(workbench.row_ids()[9_000], far);

        // Clearing the selection only happens on an explicit request.
        workbench.clear_selected();
        assert!(!workbench.is_selected(&far));
        assert_eq!(workbench.selected_count(), 0);

        // Non-selectable rows never enter the selection.
        assert!(!workbench.toggle_selected("inspect-only-1"));
        assert!(!workbench.is_selected("inspect-only-1"));
        assert!(!test_inspect_only_row("inspect-only-1").selectable());
    }

    #[test]
    fn name_and_size_sorts_are_stable_projections() {
        let mut workbench = CleanWorkbench::new();
        workbench.install_rows_for_test(vec![
            test_row_with("row-b", "node.dist", 10),
            test_row_with("row-a", "python.pytest_cache", 40),
            test_row_with("row-c", "python.mypy_cache", 40),
        ]);
        assert_eq!(workbench.row_ids().to_vec(), ["row-b", "row-a", "row-c"]);

        assert_eq!(workbench.cycle_sort(), CleanSort::Name);
        assert_eq!(workbench.row_ids().to_vec(), ["row-b", "row-c", "row-a"]);

        assert_eq!(workbench.cycle_sort(), CleanSort::Size);
        assert_eq!(workbench.row_ids().to_vec(), ["row-a", "row-c", "row-b"]);

        assert_eq!(workbench.cycle_sort(), CleanSort::ReportOrder);
        assert_eq!(workbench.row_ids().to_vec(), ["row-b", "row-a", "row-c"]);

        // The size projection never depends on the previously applied sort:
        // the same path from report order reproduces the same order.
        assert_eq!(workbench.cycle_sort(), CleanSort::Name);
        assert_eq!(workbench.cycle_sort(), CleanSort::Size);
        assert_eq!(workbench.row_ids().to_vec(), ["row-a", "row-c", "row-b"]);

        assert_eq!(workbench.cycle_sort(), CleanSort::ReportOrder);
        assert_eq!(workbench.row_ids().to_vec(), ["row-b", "row-a", "row-c"]);
    }

    #[test]
    fn select_recommended_covers_recommended_selectable_rows() {
        let mut workbench = CleanWorkbench::new();
        let mut manual = test_row("manual", 5);
        manual.selected_by_default = false;
        let mut recommended = test_row("recommended", 6);
        recommended.selected_by_default = true;
        workbench.install_rows_for_test(vec![
            manual,
            recommended,
            test_inspect_only_row("inspect"),
        ]);
        assert_eq!(workbench.recommended_ids(), ["recommended"]);
        assert_eq!(workbench.select_recommended(), 1);
        assert!(workbench.is_selected("recommended"));
        assert!(!workbench.is_selected("manual"));
        assert!(!workbench.is_selected("inspect"));
        workbench.clear_selected();
        assert_eq!(workbench.selected_count(), 0);
    }

    #[test]
    fn chromium_preview_channel_scan_rows_reach_grouped_global_review() {
        use devsweep_core::model::{RiskLevel, ScanHealth};
        let mut plan = UntrustedPlan::empty();
        for (id, channel) in [
            ("windows.chrome_beta.cache", "Chrome Beta"),
            ("windows.chrome_canary.cache", "Chrome SxS"),
            ("windows.edge_beta.cache", "Edge Beta"),
            ("windows.edge_dev.cache", "Edge Dev"),
        ] {
            let path = PathBuf::from(format!("C:/Local/{channel}/User Data/Default/Code Cache"));
            plan.targets.push(UntrustedTarget {
                id: TargetId::new(format!("{id}:{}", path.display())),
                rule_id: id.into(),
                scope: Scope::Global,
                ecosystem: Ecosystem::Generic,
                kind: TargetKind::ToolCache,
                path: Some(path),
                estimated_bytes: 1024,
                size_complete: true,
                sizing_warnings: vec![],
                last_modified: None,
                risk: RiskLevel::Medium,
                reversible: true,
                selected_by_default: false,
                evidence: vec![],
                intent: CleanupIntent::TrashProjectArtifact { rule_id: id.into() },
            });
        }
        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_started(CleanScope::Global);
        workbench.on_scan_result(scan_payload_from_report(ScanReport::new(
            plan,
            ScanHealth::complete(),
        )));
        workbench.begin_review();
        assert!(workbench.reviewing());
        assert_eq!(workbench.selected_count(), 0);
        assert_eq!(workbench.rows().len(), 4);
        for row in workbench.rows() {
            assert_eq!(row.scope, CleanRowScope::Global);
            assert!(workbench.group_for_row(&row.id).is_some());
            assert!(workbench.row_selectable(row));
            for locale in [Locale::En, Locale::ZhCn] {
                let label = row_text(locale, row, false, false, false, false);
                assert!(
                    label.contains(&row.rule_id) && label.contains("Code Cache"),
                    "{label}"
                );
            }
        }
    }

    #[test]
    fn row_text_uses_locale_copy_and_row_metadata() {
        let mut row = test_row("python.pytest_cache:root", 1536);
        row.rule_id = "python.pytest_cache".to_string();
        row.provider = "python".to_string();
        row.path = Some("C:/code/app/.pytest_cache".to_string());
        row.scope = CleanRowScope::Project {
            root: "C:/code/app".to_string(),
        };
        let en = row_text(Locale::En, &row, true, true, false, false);
        assert!(en.starts_with("> [x] python.pytest_cache · C:/code/app/.pytest_cache"));
        assert!(en.contains("1.5 KiB (1536 B)"));
        assert!(en.contains("verified size"));
        assert!(!en.contains("inspect-only"));
        assert!(en.contains("rebuild: pytest recreates it on the next test run."));
        let zh = row_text(Locale::ZhCn, &row, false, false, false, false);
        assert!(zh.starts_with("[ ] python.pytest_cache"));
        assert!(zh.contains("已验证大小"));
        assert!(zh.contains("重建："));
        assert_ne!(en, zh);

        let inspect = test_inspect_only_row("python.venv");
        let line = row_text(Locale::En, &inspect, false, false, false, false);
        assert!(line.contains("inspect-only"));
        let protected = row_text(Locale::En, &row, false, false, true, false);
        assert!(protected.contains("protected"));
        let ignored = row_text(Locale::En, &row, false, false, false, true);
        assert!(ignored.contains("ignored"));

        let global = CleanRow {
            path: None,
            scope: CleanRowScope::Global,
            size_complete: false,
            ..test_row("chrome.cache", 0)
        };
        let line = row_text(Locale::En, &global, false, false, false, false);
        assert!(line.contains("· global · 0 B · unknown size"));
        assert!(line.contains("unknown"));
        // Unknown rule ids keep an honest generic rebuild line.
        assert!(line.contains("rebuild: the owning tool recreates it"));
    }

    #[test]
    fn bounded_detail_stays_one_bounded_line() {
        assert_eq!(bounded_detail("root is unavailable"), "root is unavailable");
        let long = "x".repeat(400);
        let bounded = bounded_detail(&long);
        assert_eq!(bounded.chars().count(), 201);
        assert!(bounded.ends_with('…'));
        let multiline = bounded_detail("first\r\nsecond\tthird");
        assert!(!multiline.contains('\n'));
        assert!(!multiline.contains('\r'));
        assert!(!multiline.contains('\t'));
    }

    #[test]
    fn protected_rows_stay_visible_and_refuse_selection() {
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-protect-model",
            std::process::id()
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing a protection fixture outside the temp prefix"
        );
        let mut protected_row = test_row_with("protected-row", "python.pytest_cache", 40);
        protected_row.selected_by_default = true;
        protected_row.path = Some(root.join("protected-row").display().to_string());
        let mut free_row = test_row_with("free-row", "python.mypy_cache", 20);
        free_row.selected_by_default = true;
        free_row.path = Some(root.join("free-row").display().to_string());
        let mut workbench = CleanWorkbench::new();
        workbench.install_rows_for_test(vec![
            protected_row,
            free_row,
            test_inspect_only_row("inspect-row"),
        ]);

        // Without a saved protection both candidates are selectable.
        assert!(workbench.toggle_selected("protected-row"));
        assert!(workbench.toggle_selected("free-row"));
        assert_eq!(workbench.recommended_ids(), ["protected-row", "free-row"]);

        // The saved protection covers the row path: the row stays visible in
        // the report but leaves the selection and the recommendation.
        workbench.set_protected_paths(vec![root.join("protected-row").display().to_string()]);
        assert!(
            workbench.row("protected-row").is_some(),
            "a protected row must stay visible"
        );
        assert!(
            workbench.row("inspect-row").is_some(),
            "an inspect-only row must stay visible"
        );
        assert!(workbench.is_protected("protected-row"));
        assert!(!workbench.is_protected("free-row"));
        assert!(
            !workbench.is_selected("protected-row"),
            "new protection drops the reviewed selection"
        );
        assert!(workbench.is_selected("free-row"));
        assert!(!workbench.toggle_selected("protected-row"));
        assert_eq!(workbench.recommended_ids(), ["free-row"]);
        assert!(!workbench.toggle_selected("inspect-row"));
        let line = row_text(
            Locale::En,
            workbench.row("protected-row").unwrap(),
            false,
            false,
            true,
            false,
        );
        assert!(line.contains("protected"), "{line}");

        // Removing the saved protection restores selectability.
        workbench.set_protected_paths(Vec::new());
        assert!(workbench.toggle_selected("protected-row"));
        assert_eq!(workbench.recommended_ids(), ["protected-row", "free-row"]);

        // Relative or empty entries never cover anything.
        workbench.set_protected_paths(vec!["relative/path".to_string(), String::new()]);
        assert!(!workbench.is_protected("protected-row"));
        assert!(workbench.toggle_selected("free-row"));
    }

    #[test]
    fn ignored_rows_stay_visible_and_leave_recommended() {
        let mut recommended = test_row_with("ignored-row", "python.pytest_cache", 40);
        recommended.selected_by_default = true;
        let mut workbench = CleanWorkbench::new();
        workbench.install_rows_for_test(vec![
            recommended,
            test_row_with("kept-row", "python.mypy_cache", 20),
            test_inspect_only_row("inspect-row"),
        ]);
        assert!(workbench.toggle_selected("ignored-row"));
        assert_eq!(workbench.recommended_ids(), ["ignored-row"]);

        // Ignoring keeps the row in the report but out of every selection
        // path, and marks the row line.
        assert_eq!(workbench.toggle_ignored("ignored-row"), Some(true));
        assert!(workbench.is_ignored("ignored-row"));
        assert!(
            workbench.row("ignored-row").is_some(),
            "an ignored row must stay visible"
        );
        assert!(
            workbench.row("inspect-row").is_some(),
            "an inspect-only row must stay visible"
        );
        assert!(!workbench.is_selected("ignored-row"));
        assert!(!workbench.toggle_selected("ignored-row"));
        assert!(workbench.recommended_ids().is_empty());
        let line = row_text(
            Locale::En,
            workbench.row("ignored-row").unwrap(),
            false,
            false,
            false,
            true,
        );
        assert!(line.contains("ignored"), "{line}");
        let zh = row_text(
            Locale::ZhCn,
            workbench.row("ignored-row").unwrap(),
            false,
            false,
            false,
            true,
        );
        assert!(zh.contains("已忽略"), "{zh}");

        // Un-ignoring restores the row; a missing row is a no-op.
        assert_eq!(workbench.toggle_ignored("ignored-row"), Some(false));
        assert!(!workbench.is_ignored("ignored-row"));
        assert!(workbench.toggle_selected("ignored-row"));
        assert_eq!(workbench.recommended_ids(), ["ignored-row"]);
        assert_eq!(workbench.toggle_ignored("missing-row"), None);
    }

    #[test]
    fn ten_thousand_rows_keep_cached_bounded_projections() {
        use std::time::{Duration, Instant};

        let started = Instant::now();
        let mut workbench = CleanWorkbench::new();
        let rows = (0..10_000)
            .map(|index| {
                test_row_with(
                    &format!("clean-{index}"),
                    "python.fixture_rule",
                    (index % 97) as u64,
                )
            })
            .collect::<Vec<_>>();
        workbench.install_rows_for_test(rows);

        // The id projection is cached and returned by reference: repeated
        // reads and O(1) lookups never rebuild it.
        let cached_ptr = workbench.row_ids().as_ptr();
        for _ in 0..100 {
            assert_eq!(workbench.row_ids().len(), 10_000);
            assert_eq!(
                workbench.row_ids().as_ptr(),
                cached_ptr,
                "row_ids must return the cached projection, not rebuild per read"
            );
            assert_eq!(
                workbench.row("clean-9999").map(|row| row.id.as_str()),
                Some("clean-9999")
            );
            assert!(workbench.row("clean-10000").is_none());
        }

        // A sort change rebuilds the projection once; every id still resolves
        // and report order returns exactly.
        assert_eq!(workbench.cycle_sort(), CleanSort::Name);
        assert_eq!(workbench.row_ids().len(), 10_000);
        assert_eq!(workbench.row("clean-9999").unwrap().plan_index, 9_999);
        assert_eq!(workbench.cycle_sort(), CleanSort::Size);
        let sized = workbench.row_ids().to_vec();
        let sized_ptr = workbench.row_ids().as_ptr();
        workbench.apply_sort();
        assert_eq!(
            workbench.row_ids(),
            sized.as_slice(),
            "re-applying the same size sort is stable"
        );
        assert_eq!(
            workbench.row_ids().as_ptr(),
            sized_ptr,
            "re-applying the same sort keeps the cached projection in place"
        );
        assert_eq!(workbench.cycle_sort(), CleanSort::ReportOrder);
        assert_eq!(workbench.row_ids()[0], "clean-0");
        assert_eq!(workbench.row_ids()[9_999], "clean-9999");

        // Selection survival across sorts and clears stays exact.
        assert!(workbench.toggle_selected("clean-9000"));
        assert_eq!(workbench.cycle_sort(), CleanSort::Name);
        assert!(workbench.is_selected("clean-9000"));
        assert_eq!(workbench.selected_count(), 1);
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "10k synthetic projections must stay under one second: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn clean_groups_cache_subtotals_tri_state_and_review_only_authority() {
        let mut workbench = CleanWorkbench::new();
        let mut a = test_row("a", 10);
        a.selected_by_default = true;
        let mut b = test_row("b", 20);
        b.size_complete = false;
        let mut command = test_row("command", 100);
        command.recycle_eligible = false;
        command.selected_by_default = true;
        workbench.install_rows_for_test(vec![a, b, command, test_inspect_only_row("inspect")]);
        assert_eq!(workbench.groups().len(), 2);
        assert_eq!(workbench.capacity().bytes, 130);
        assert_eq!(workbench.recommended_capacity().bytes, 10);
        let group_id = workbench.groups()[0].id.clone();
        assert_eq!(
            workbench.groups()[0].capacity,
            CleanCapacity {
                bytes: 30,
                incomplete: 1
            }
        );
        assert_eq!(workbench.groups()[0].selection, GroupSelection::None);
        workbench.toggle_selected("a");
        assert_eq!(workbench.groups()[0].selection, GroupSelection::Mixed);
        assert_eq!(workbench.groups()[0].selected_count, 1);
        workbench.toggle_group(&group_id);
        assert_eq!(workbench.groups()[0].selection, GroupSelection::All);
        assert_eq!(workbench.groups()[0].selected_count, 2);
        assert_eq!(workbench.selected_count(), 2);
        workbench.toggle_group(&group_id);
        assert!(workbench.selected_ids().is_empty());
        assert!(!workbench.toggle_selected("command"));
        assert!(!workbench.toggle_selected("inspect"));
        workbench.toggle_group("review-only");
        assert!(workbench.selected_ids().is_empty());
        assert!(project_clean_groups(&[], &BTreeSet::new(), &BTreeSet::new()).is_empty());
        workbench.toggle_ignored("a");
        assert_eq!(workbench.recommended_capacity().bytes, 0);
        assert_eq!(workbench.groups()[0].selectable_ids, ["b"]);
    }

    #[test]
    fn clean_all_scope_requires_explicit_roots_and_fresh_global_consent() {
        let roots = vec![PathBuf::from("C:/task-owned-project")];
        let unconfirmed = clean_scan_options(CleanScope::All, false, roots.clone()).unwrap();
        assert!(unconfirmed.include_projects);
        assert!(!unconfirmed.include_global);
        assert_eq!(unconfirmed.roots, roots);
        let confirmed = clean_scan_options(CleanScope::All, true, roots).unwrap();
        assert!(confirmed.include_projects && confirmed.include_global);
        assert!(clean_scan_options(CleanScope::All, true, Vec::new()).is_none());
        assert!(clean_scan_options(CleanScope::Global, false, Vec::new()).is_none());
        let global = clean_scan_options(CleanScope::Global, true, Vec::new()).unwrap();
        assert!(!global.include_projects && global.include_global);
    }

    #[test]
    fn clean_preview_is_cleared_by_every_selection_edit_and_command_plan_is_refused() {
        let root = isolated_multi_cache_project();
        let mut workbench = CleanWorkbench::new();
        workbench.on_scan_result(project_scan(&root));
        // Fresh temporary files are not scanner recommendations. Set the hint
        // explicitly to exercise the recommendation selection path deterministically.
        for row in &mut workbench.rows {
            row.selected_by_default = row.selectable();
        }
        workbench.rebuild_groups();
        workbench.select_recommended();
        assert!(workbench.preview_selection());
        workbench.clear_selected();
        assert!(workbench.preview().is_none());
        workbench.select_recommended();
        assert!(workbench.preview_selection());
        let id = workbench.selected_ids().first().unwrap().clone();
        workbench.toggle_selected(&id);
        assert!(workbench.preview().is_none());
        workbench.select_recommended();
        assert!(workbench.preview_selection());
        let group = workbench.group_for_row(&id).unwrap().id.clone();
        workbench.toggle_group(&group);
        assert!(workbench.preview().is_none());
        // A corrupt UI projection cannot turn a command intent into trash authority.
        let plan = workbench.plan.as_mut().unwrap();
        let target = &mut plan.targets[0];
        target.intent = CleanupIntent::RunBuiltInAction {
            provider_id: "cargo".to_string(),
            action_id: "clean".to_string(),
        };
        let id = target.id.as_str().to_string();
        workbench.selected.insert(id);
        assert!(!workbench.preview_selection());
        assert!(workbench.begin_execute().is_none());
        let cancel = Arc::new(FlagCancelObserver::new());
        let result = execute_clean_plan(
            workbench.plan.as_ref().unwrap(),
            Vec::new(),
            "sha256:invalid",
            &cancel,
        );
        assert_eq!(result.failure, Some(CleanExecuteFailure::Unavailable));
        cleanup(&root);
    }

    #[test]
    fn clean_results_keep_reasons_and_exclude_commands_unknown_and_failed_capacity() {
        let mut workbench = CleanWorkbench::new();
        workbench.install_rows_for_test(
            (0..5)
                .map(|index| test_row(&format!("row-{index}"), 99))
                .collect(),
        );
        let mut report = test_execution_report(3, 1, 1);
        report.outcomes[0].estimated_recoverable = CapacityEstimate::Partial {
            lower_bound_bytes: 10,
        };
        report.outcomes[1].estimated_recoverable = CapacityEstimate::Unknown;
        report.outcomes[2].action = ActionKind::Command { irreversible: true };
        workbench.on_execute_result(CleanExecutePayload {
            report: Some(report.clone()),
            failure: None,
            canceled: false,
        });
        let execution = workbench.execution().unwrap();
        assert_eq!(execution.report, report);
        assert_eq!(execution.moved_count, 2);
        assert_eq!(
            execution.moved,
            CleanCapacity {
                bytes: 10,
                incomplete: 2
            }
        );
        assert_eq!((execution.failed, execution.skipped), (1, 1));
        assert!(
            matches!(&execution.groups[0].outcomes[3].status, OutcomeStatus::Failed { message } if message == "fixture failure")
        );
        assert!(
            matches!(&execution.groups[0].outcomes[4].status, OutcomeStatus::Skipped { reason } if reason == "fixture skip")
        );
        for locale in [Locale::En, Locale::ZhCn] {
            let entry = workbench.history_entry(locale).unwrap();
            assert!(entry.contains(&copy::clean_moved_summary(
                locale,
                execution.moved_count,
                &execution.moved.label(locale)
            )));
            assert!(entry.contains(copy::pick(locale, "At least", "至少")));
        }
    }

    #[test]
    fn clean_cancellation_and_audit_failure_do_not_claim_complete_history() {
        for state in ["canceled", "audit-failed"] {
            let mut workbench = CleanWorkbench::new();
            assert!(workbench.install_native_fixture(state));
            let execution = workbench.execution().unwrap();
            assert!(execution.history_incomplete);
            assert_eq!(execution.moved_count, 0);
            assert_eq!(execution.report.outcomes.len(), 3);
            assert!(
                workbench
                    .history_entry(Locale::En)
                    .unwrap()
                    .contains("may be incomplete")
            );
        }
    }

    #[test]
    fn clean_normal_report_matches_corresponding_isolated_history_evidence() {
        let root = isolated_multi_cache_project();
        let store = root.join("local/DevSweep/audit/v1");
        fs::create_dir_all(&store).unwrap();
        let mut lines = Vec::new();
        for (index, outcome) in ["succeeded", "failed", "skipped"].iter().enumerate() {
            let record = serde_json::json!({
                "record_kind": "execution_transition", "schema_version": 1, "domain": "clean",
                "operation_id": "fixture-run", "timestamp_epoch_ms": index + 1, "transition": index + 1,
                "outcome_code": outcome, "error_code": if index == 1 { Some("trash_failed") } else if index == 2 { Some("safety_skip") } else { None },
                "evidence": { "target_identity_sha256": devsweep_core::execution::identity_sha256(&format!("row-{index}")), "action_kind": "move_to_trash", "irreversible": false,
                    "capacity_class": "verified", "duration_ms": 1, }
            });
            let mut record = record;
            if index < 2 {
                let mut started = record.clone();
                started["outcome_code"] = "started".into();
                started["error_code"] = serde_json::Value::Null;
                started["transition"] = (lines.len() + 1).into();
                lines.push(started.to_string());
            }
            record["transition"] = (lines.len() + 1).into();
            if index == 0 {
                record["evidence"]["estimated_bytes"] = 1.into();
            }
            lines.push(record.to_string());
        }
        fs::write(store.join("clean.jsonl"), lines.join("\n") + "\n").unwrap();
        // The subprocess gets its own environment. Parallel tests in this process
        // never see a redirected audit store or a temporary LOCALAPPDATA value.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "clean_workbench::tests::clean_isolated_history_child",
                "--nocapture",
            ])
            .env("LOCALAPPDATA", root.join("local"))
            .env("DEVSWEEP_CLEAN_HISTORY_TEST_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        cleanup(&root);
    }

    #[test]
    fn clean_isolated_history_child() {
        if std::env::var_os("DEVSWEEP_CLEAN_HISTORY_TEST_CHILD").is_none() {
            return;
        }
        let detail = devsweep_core::history::show_history("fixture-run").unwrap();
        let mut counts = (0, 0, 0);
        let mut moved_bytes = 0;
        for record in serde_json::to_value(detail.records)
            .unwrap()
            .as_array()
            .unwrap()
        {
            if record["history_kind"] == "clean" {
                let record = &record["record"];
                match record["outcome_code"].as_str().unwrap() {
                    "succeeded" => {
                        counts.0 += 1;
                        moved_bytes += record["evidence"]["estimated_bytes"].as_u64().unwrap();
                    }
                    "failed" => counts.1 += 1,
                    "skipped" => counts.2 += 1,
                    _ => {}
                }
            }
        }
        let mut workbench = CleanWorkbench::new();
        workbench.on_execute_result(CleanExecutePayload {
            report: Some(test_execution_report(1, 1, 1)),
            failure: None,
            canceled: false,
        });
        let execution = workbench.execution().unwrap();
        assert_eq!(
            counts,
            (execution.moved_count, execution.failed, execution.skipped)
        );
        assert_eq!(moved_bytes, execution.moved.bytes);
        assert!(!execution.history_incomplete);
    }

    #[test]
    fn clean_native_fixtures_never_grant_plan_or_execution_authority() {
        for state in [
            "landing",
            "scanning",
            "scanned",
            "review",
            "previewed",
            "complete",
            "mixed",
            "canceled",
            "audit-failed",
        ] {
            let mut workbench = CleanWorkbench::new();
            assert!(workbench.install_native_fixture(state));
            assert!(workbench.plan.is_none());
            assert!(!workbench.preview_selection());
            assert!(workbench.begin_execute().is_none());
            assert!(!workbench.preview_matches_current());
        }
    }

    fn test_row(id: &str, bytes: u64) -> CleanRow {
        test_row_with(id, "python.fixture_rule", bytes)
    }

    fn test_row_with(id: &str, rule_id: &str, bytes: u64) -> CleanRow {
        CleanRow {
            id: id.to_string(),
            plan_index: 0,
            rule_id: rule_id.to_string(),
            provider: provider_of(rule_id).to_string(),
            scope: CleanRowScope::Global,
            path: Some(format!("C:/fixture/{id}")),
            bytes,
            size_complete: true,
            inspect_only: false,
            ecosystem: devsweep_core::model::Ecosystem::Python,
            kind: devsweep_core::model::TargetKind::ToolCache,
            recycle_eligible: true,
            selected_by_default: false,
        }
    }

    fn test_inspect_only_row(id: &str) -> CleanRow {
        CleanRow {
            inspect_only: true,
            selected_by_default: false,
            ..test_row(id, 0)
        }
    }
}
