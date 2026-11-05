//! Five-mode shell state, explicit preference edits, and operation fences.
//!
//! Navigation tests call this model directly. Rendering does not own cleanup
//! authority. A completion applies only when its operation id is still current
//! and has not been canceled.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use crate::{
    analyze::AnalyzeBoard,
    analyze_window::{AnalyzeRootChoice, AnalyzeWindow},
    clean_ai::{
        AiCategory, AiSubviewPhase, CleanAiExecutePayload, CleanAiRecoveryPayload,
        CleanAiRestorePayload, CleanAiScanPayload, CleanAiView,
    },
    clean_workbench::{
        CleanExecutePayload, CleanExecutionStart, CleanPhase, CleanScanPayload, CleanScope,
        CleanWorkbench, execute_clean_plan, scan_clean_rows,
    },
    copy::{Locale, StoreName},
    optimize_board::OptimizeBoard,
    preferences::{
        self, DesktopMotion, DesktopPatch, DesktopPreferences, DesktopTheme, LanguageTag,
        PreferenceFiles, StoreError,
    },
    software_review::SoftwareReview,
    status_panel::{ProcessSort, StatusPanel, StatusPhase},
    workbench_review::WorkbenchReview,
};
use devsweep_core::{
    ai::{AiError, AiExecuteReportV1, AiExit},
    execution::ExecutionReport,
    optimize::PrecheckReason,
    process::{CancelObserver as _, FlagCancelObserver},
    scan::ScanOptions,
    software::{AdapterOutcome, SoftwareExecutionOutcome, SoftwarePlanError},
    status::{LiveControl, StatusEventV1, StatusSnapshotV1},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Clean,
    Software,
    Optimize,
    Analyze,
    Status,
}

impl Mode {
    pub const ORDER: [Self; 5] = [
        Self::Clean,
        Self::Software,
        Self::Optimize,
        Self::Analyze,
        Self::Status,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Software => "software",
            Self::Optimize => "optimize",
            Self::Analyze => "analyze",
            Self::Status => "status",
        }
    }

    pub fn element_id(self) -> &'static str {
        match self {
            Self::Clean => "mode-clean",
            Self::Software => "mode-software",
            Self::Optimize => "mode-optimize",
            Self::Analyze => "mode-analyze",
            Self::Status => "mode-status",
        }
    }

    pub fn index(self) -> usize {
        Self::ORDER
            .iter()
            .position(|mode| *mode == self)
            .unwrap_or(0)
    }

    pub fn move_by(self, delta: isize) -> Self {
        let next = self.index() as isize + delta;
        let next = next.clamp(0, Self::ORDER.len() as isize - 1) as usize;
        Self::ORDER[next]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Mode,
    Settings,
    Protection,
    Rules,
    History,
}

pub const MENU: [Surface; 4] = [
    Surface::Settings,
    Surface::Protection,
    Surface::Rules,
    Surface::History,
];

/// Whether the History page has read the fixed audit stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryStoreView {
    NotLoaded,
    Empty,
    Unavailable,
    Ready,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavKey {
    Left,
    Right,
    Up,
    Down,
    RowDown,
    RowUp,
    Escape,
    Enter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Review {
    Valid,
    StaleScanRoots,
    StaleProtection,
}

/// Closed Software/Optimize panel notices. Mapped from domain facts only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkbenchNotice {
    ProviderUiPending,
    PrivilegeDecline,
    RebootRequired,
    FailedUninstall,
    ChangedSelection { selection_id: String },
}

impl WorkbenchNotice {
    /// Provider UI pending is `AdapterOutcome::Unfinished` only. Success is never mapped.
    #[must_use]
    pub fn from_software_adapter(outcome: AdapterOutcome) -> Option<Self> {
        match outcome {
            AdapterOutcome::Unfinished => Some(Self::ProviderUiPending),
            AdapterOutcome::Success | AdapterOutcome::Failure | AdapterOutcome::RebootRequired => {
                None
            }
        }
    }

    /// Privilege decline is `PrecheckReason::UacDeclined` (`uac_declined`) only.
    #[must_use]
    pub fn from_precheck_reason(reason: PrecheckReason) -> Option<Self> {
        match reason {
            PrecheckReason::UacDeclined => Some(Self::PrivilegeDecline),
            PrecheckReason::NotSsd
            | PrecheckReason::LowBattery
            | PrecheckReason::DiskProblem
            | PrecheckReason::RebootPending
            | PrecheckReason::Cooldown
            | PrecheckReason::ProgramMissing
            | PrecheckReason::ImageNotRepairable
            | PrecheckReason::BackupWriteFailed => None,
        }
    }

    /// Reboot and failed uninstall are the matching `SoftwareExecutionOutcome` terminals.
    #[must_use]
    pub fn from_software_execution(outcome: SoftwareExecutionOutcome) -> Option<Self> {
        match outcome {
            SoftwareExecutionOutcome::RebootRequired => Some(Self::RebootRequired),
            SoftwareExecutionOutcome::Failed => Some(Self::FailedUninstall),
            SoftwareExecutionOutcome::CanceledBeforeStart
            | SoftwareExecutionOutcome::Removed
            | SoftwareExecutionOutcome::StillPresent
            | SoftwareExecutionOutcome::UnknownAfterDispatch => None,
        }
    }

    /// Changed selection is `SoftwarePlanError::StaleSelection` and keeps the selection id.
    #[must_use]
    pub fn from_software_plan_error(error: &SoftwarePlanError) -> Option<Self> {
        match error {
            SoftwarePlanError::StaleSelection(selection_id) => Some(Self::ChangedSelection {
                selection_id: selection_id.clone(),
            }),
            SoftwarePlanError::UnsupportedInventoryVersion(_)
            | SoftwarePlanError::UnsupportedPlanVersion(_)
            | SoftwarePlanError::InvalidFingerprint
            | SoftwarePlanError::InventoryFingerprintMismatch
            | SoftwarePlanError::InventoryExpired
            | SoftwarePlanError::EmptySelection
            | SoftwarePlanError::DuplicateSelection(_)
            | SoftwarePlanError::UnknownSelection(_)
            | SoftwarePlanError::ManualSelection(_)
            | SoftwarePlanError::InvalidSelectedIdentity(_)
            | SoftwarePlanError::DigestMismatch
            | SoftwarePlanError::SerializationFailed => None,
        }
    }
}

/// Closed Clean panel outcome. Mapped from domain facts only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanOutcomeNotice {
    Unavailable,
    Empty,
    Partial,
    Failed,
    Canceled,
    StalePreview,
    ConfirmationRequired,
    Mixed,
    TrashReported,
}

impl CleanOutcomeNotice {
    /// `source_unavailable`, `stale_confirmation`, `confirmation_required`,
    /// `canceled`, partial, and failed map from `execute_ai` errors. Success is
    /// never mapped.
    #[must_use]
    pub fn from_ai_error(error: &AiError) -> Option<Self> {
        match error.code {
            "source_unavailable" => Some(Self::Unavailable),
            "stale_confirmation" => Some(Self::StalePreview),
            "confirmation_required" => Some(Self::ConfirmationRequired),
            "canceled" => Some(Self::Canceled),
            _ => match error.exit {
                AiExit::Partial => Some(Self::Partial),
                AiExit::Failed => Some(Self::Failed),
                AiExit::Unavailable => Some(Self::Unavailable),
                AiExit::Canceled => Some(Self::Canceled),
                AiExit::Stale => Some(Self::StalePreview),
                AiExit::Usage => None,
            },
        }
    }

    /// Succeeded with a non-empty isolated `moved` list is trash reported.
    /// Partial is mixed even when `moved` is non-empty. Failed and unknown are
    /// failed. This is not a Recycle Bin claim and is never success copy.
    #[must_use]
    pub fn from_ai_execute_report(report: &AiExecuteReportV1) -> Option<Self> {
        match report.outcome.as_str() {
            "succeeded" if !report.moved.is_empty() => Some(Self::TrashReported),
            "partial" => Some(Self::Mixed),
            "failed" | "unknown" => Some(Self::Failed),
            _ => None,
        }
    }

    /// Aggregates one core execution report into a closed Clean notice.
    ///
    /// Mixed means some selected targets moved and some did not. Failed means
    /// nothing moved and at least one target failed. Partial means nothing
    /// failed and at least one target was skipped before dispatch. Trash
    /// reported means every selected target moved. Dry runs and empty
    /// selections never map. This is not a Recycle Bin claim.
    #[must_use]
    pub fn from_execution_report(report: &ExecutionReport) -> Option<Self> {
        if report.dry_run || report.selected == 0 {
            return None;
        }
        let moved = report.succeeded;
        if report.failed > 0 && moved > 0 {
            Some(Self::Mixed)
        } else if report.failed > 0 {
            Some(Self::Failed)
        } else if report.skipped > 0 && moved > 0 {
            Some(Self::Mixed)
        } else if report.skipped > 0 {
            Some(Self::Partial)
        } else if moved > 0 {
            Some(Self::TrashReported)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    Visible,
    Minimized,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Idle,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationClass {
    ReadOnly,
    Mutating,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseAction {
    Quit,
    CancelAndJoin { id: u64 },
    WaitOnly { id: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailEvent {
    Completion {
        id: u64,
        report: String,
    },
    CleanScan {
        id: u64,
        payload: CleanScanPayload,
    },
    CleanExecute {
        id: u64,
        payload: CleanExecutePayload,
    },
    CleanAiScan {
        id: u64,
        payload: CleanAiScanPayload,
    },
    CleanAiExecute {
        id: u64,
        payload: CleanAiExecutePayload,
    },
    CleanAiRecovery {
        id: u64,
        payload: CleanAiRecoveryPayload,
    },
    CleanAiRestore {
        id: u64,
        payload: CleanAiRestorePayload,
    },
    Lifecycle {
        seq: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MailboxFull;

#[derive(Debug)]
pub struct SharedMailbox {
    events: Mutex<VecDeque<MailEvent>>,
}

impl SharedMailbox {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(VecDeque::new()),
        }
    }

    pub fn try_post(&self, event: MailEvent) -> Result<(), MailboxFull> {
        let mut events = self.events.lock().expect("mailbox lock");
        if events.len() >= 32 {
            return Err(MailboxFull);
        }
        events.push_back(event);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.events.lock().expect("mailbox lock").len()
    }

    /// Removes and returns at most `limit` queued events in arrival order.
    pub fn take_batch(&self, limit: usize) -> Vec<MailEvent> {
        let mut events = self.events.lock().expect("mailbox lock");
        let count = events.len().min(limit);
        events.drain(..count).collect()
    }
}

impl Default for SharedMailbox {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Poll {
    pub applied: usize,
    pub decorative_frame: bool,
}

/// Run `work` off the caller thread and post one event. The caller does not join.
pub fn dispatch_blocking<F>(mailbox: Arc<SharedMailbox>, work: F)
where
    F: FnOnce() -> MailEvent + Send + 'static,
{
    thread::spawn(move || {
        let event = work();
        let _ = mailbox.try_post(event);
    });
}

/// Visible Clean/Software/Optimize rows. The shell may store every id; rendering
/// may append at most this many from the current window start.
pub const VISIBLE_ROW_CAPACITY: usize = 48;

#[derive(Clone, Debug)]
struct ModeMemory {
    report: Option<String>,
    selection: Option<String>,
    review: Review,
    row_ids: Vec<String>,
    row_window_start: usize,
    row_query: String,
    filtered_row_ids: Vec<String>,
    /// Number of times the filtered projection was rebuilt. Read-only
    /// observers can prove that rendering does not rebuild it.
    filter_rebuilds: u64,
    /// Number of times the full id set was replaced.
    row_set_installs: u64,
}

impl Default for ModeMemory {
    fn default() -> Self {
        Self {
            report: None,
            selection: None,
            review: Review::Valid,
            row_ids: Vec::new(),
            row_window_start: 0,
            row_query: String::new(),
            filtered_row_ids: Vec::new(),
            filter_rebuilds: 0,
            row_set_installs: 0,
        }
    }
}

impl ModeMemory {
    fn window_ids(&self) -> &[String] {
        if self.row_query.is_empty() {
            &self.row_ids
        } else {
            &self.filtered_row_ids
        }
    }

    fn rebuild_filtered_rows(&mut self) {
        self.filter_rebuilds += 1;
        if self.row_query.is_empty() {
            self.filtered_row_ids.clear();
            return;
        }
        self.filtered_row_ids = self
            .row_ids
            .iter()
            .filter(|id| id.contains(&self.row_query))
            .cloned()
            .collect();
    }

    fn clamp_window_start(&mut self) {
        let len = self.window_ids().len();
        if self.row_window_start > len {
            self.row_window_start = len;
        }
    }
}

struct LiveOperation {
    id: u64,
    class: OperationClass,
    mode: Mode,
    canceled: bool,
}

#[derive(Clone, Debug)]
struct Draft {
    committed: DesktopPreferences,
    theme: DesktopTheme,
    text_scale_percent: u8,
    motion: DesktopMotion,
    language: Option<LanguageTag>,
    committed_language: Option<LanguageTag>,
}

pub struct ShellState {
    mode: Mode,
    surface: Surface,
    menu_open: bool,
    menu_index: usize,
    memories: [ModeMemory; 5],
    history: Vec<String>,
    history_store: HistoryStoreView,
    history_selection: Option<String>,
    rules: Vec<String>,
    scan_roots: Vec<String>,
    protections: Vec<String>,
    operation: Option<LiveOperation>,
    next_operation: u64,
    rejected_results: u32,
    lifecycle_applied: u32,
    presence: Presence,
    activity: Activity,
    motion: DesktopMotion,
    decorative_rejections: u32,
    locale: Locale,
    workbench_notice: Option<WorkbenchNotice>,
    cover: Option<CleanScreen>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CleanScreen {
    mode: Mode,
    surface: Surface,
}

impl ShellState {
    pub fn new() -> Self {
        Self::from_preferences(DesktopPreferences::default(), Locale::En)
    }

    fn from_preferences(preferences: DesktopPreferences, locale: Locale) -> Self {
        Self {
            mode: Mode::Clean,
            surface: Surface::Mode,
            menu_open: false,
            menu_index: 0,
            memories: Default::default(),
            history: Vec::new(),
            history_store: HistoryStoreView::NotLoaded,
            history_selection: None,
            rules: Vec::new(),
            scan_roots: Vec::new(),
            protections: Vec::new(),
            operation: None,
            next_operation: 0,
            rejected_results: 0,
            lifecycle_applied: 0,
            presence: Presence::Visible,
            activity: Activity::Idle,
            motion: preferences.motion,
            decorative_rejections: 0,
            locale,
            workbench_notice: None,
            cover: None,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn surface(&self) -> Surface {
        self.surface
    }

    pub fn menu_open(&self) -> bool {
        self.menu_open
    }

    pub fn menu_index(&self) -> usize {
        self.menu_index
    }

    pub fn report(&self, mode: Mode) -> Option<&str> {
        self.memories[mode.index()].report.as_deref()
    }

    pub fn selection(&self, mode: Mode) -> Option<&str> {
        self.memories[mode.index()].selection.as_deref()
    }

    pub fn review(&self, mode: Mode) -> Review {
        self.memories[mode.index()].review
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub(crate) fn history_store(&self) -> HistoryStoreView {
        self.history_store
    }

    pub(crate) fn set_history_store(&mut self, view: HistoryStoreView, lines: Vec<String>) {
        self.history = lines;
        self.history_store = view;
    }

    pub fn rules(&self) -> &[String] {
        &self.rules
    }

    pub fn history_selection(&self) -> Option<&str> {
        self.history_selection.as_deref()
    }

    pub fn scan_roots(&self) -> &[String] {
        &self.scan_roots
    }

    pub fn protections(&self) -> &[String] {
        &self.protections
    }

    pub fn workbench_notice(&self) -> Option<&WorkbenchNotice> {
        self.workbench_notice.as_ref()
    }

    pub fn set_workbench_notice(&mut self, notice: Option<WorkbenchNotice>) {
        self.workbench_notice = notice;
    }

    pub fn activity(&self) -> Activity {
        self.activity
    }

    pub fn rejected_results(&self) -> u32 {
        self.rejected_results
    }

    pub fn lifecycle_applied(&self) -> u32 {
        self.lifecycle_applied
    }

    pub fn select(&mut self, mode: Mode, selection: impl Into<String>) {
        self.memories[mode.index()].selection = Some(selection.into());
    }

    pub fn set_row_ids(&mut self, mode: Mode, ids: Vec<String>) {
        let memory = &mut self.memories[mode.index()];
        memory.row_set_installs += 1;
        memory.row_ids = ids;
        memory.rebuild_filtered_rows();
        memory.clamp_window_start();
    }

    /// Number of filtered-projection rebuilds. Repeated reads through
    /// [`Self::visible_row_ids`] must not increase this.
    pub fn filter_rebuild_count(&self, mode: Mode) -> u64 {
        self.memories[mode.index()].filter_rebuilds
    }

    /// Number of full id-set installs for this mode.
    pub fn row_set_install_count(&self, mode: Mode) -> u64 {
        self.memories[mode.index()].row_set_installs
    }

    pub fn set_row_window_start(&mut self, mode: Mode, start: usize) {
        let memory = &mut self.memories[mode.index()];
        memory.row_window_start = start.min(memory.window_ids().len());
    }

    /// Current window start for the filtered row projection.
    pub fn row_window_start(&self, mode: Mode) -> usize {
        self.memories[mode.index()].row_window_start
    }

    /// Number of ids in the current filtered row projection.
    pub fn window_row_count(&self, mode: Mode) -> usize {
        self.memories[mode.index()].window_ids().len()
    }

    /// Current row filter query.
    pub fn row_query(&self, mode: Mode) -> &str {
        &self.memories[mode.index()].row_query
    }

    /// Clears the row cursor without touching row ids or the window.
    pub fn clear_row_selection(&mut self, mode: Mode) {
        self.memories[mode.index()].selection = None;
    }

    /// Filter visible ids by substring. An empty query uses the unfiltered window.
    /// This does not clear [`Self::selection`].
    pub fn set_row_query(&mut self, mode: Mode, query: impl Into<String>) {
        let memory = &mut self.memories[mode.index()];
        memory.row_query = query.into();
        memory.rebuild_filtered_rows();
        memory.clamp_window_start();
    }

    /// At most [`VISIBLE_ROW_CAPACITY`] ids, beginning at the mode window start.
    pub fn visible_row_ids(&self, mode: Mode) -> &[String] {
        let memory = &self.memories[mode.index()];
        visible_slice(memory.window_ids(), memory.row_window_start)
    }

    /// Every id in the current search projection, not only the painted window.
    pub fn filtered_row_ids(&self, mode: Mode) -> &[String] {
        self.memories[mode.index()].window_ids()
    }

    pub fn select_history(&mut self, selection: impl Into<String>) {
        self.history_selection = Some(selection.into());
    }

    pub fn show_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.surface = Surface::Mode;
        self.menu_open = false;
        // Classify the landing identity only. Do not scan, mutate, or record success.
        match mode {
            Mode::Software => {
                let _ = crate::workbench::software_landing();
            }
            Mode::Optimize => {
                let _ = crate::workbench::resolve(crate::workbench::WorkbenchAction::Catalogue(
                    "dns.flush",
                ));
            }
            Mode::Clean | Mode::Analyze | Mode::Status => {}
        }
    }

    pub fn open_surface(&mut self, surface: Surface) {
        self.surface = surface;
        self.menu_open = false;
    }

    pub fn toggle_menu(&mut self) {
        self.menu_open = !self.menu_open;
        if self.menu_open {
            self.menu_index = 0;
        }
    }

    pub fn open_menu_item(&mut self, index: usize) {
        let Some(surface) = MENU.get(index).copied() else {
            return;
        };
        self.menu_index = index;
        self.open_surface(surface);
    }

    pub fn begin_clean_screen(&mut self) {
        if self.cover.is_none() {
            self.cover = Some(CleanScreen {
                mode: self.mode,
                surface: self.surface,
            });
        }
    }

    pub fn screen_covered(&self) -> bool {
        self.cover.is_some()
    }

    /// Clean Screen covers the view. It is not an input lock.
    pub fn input_locked(&self) -> bool {
        false
    }

    pub fn apply_key(&mut self, key: NavKey) {
        if let Some(cover) = self.cover {
            if key == NavKey::Escape {
                self.mode = cover.mode;
                self.surface = cover.surface;
                self.menu_open = false;
                self.cover = None;
            }
            return;
        }
        match key {
            NavKey::Escape => {
                if self.menu_open {
                    self.menu_open = false;
                } else {
                    self.surface = Surface::Mode;
                }
            }
            NavKey::Enter => {
                if self.menu_open {
                    self.open_menu_item(self.menu_index);
                }
            }
            NavKey::Up | NavKey::Down if self.menu_open => {
                let delta = if key == NavKey::Down { 1 } else { -1 };
                self.menu_index = move_index(self.menu_index, delta, MENU.len());
            }
            NavKey::Left | NavKey::Up => {
                if !self.menu_open && self.surface == Surface::Mode {
                    self.mode = self.mode.move_by(-1);
                }
            }
            NavKey::Right | NavKey::Down => {
                if !self.menu_open && self.surface == Surface::Mode {
                    self.mode = self.mode.move_by(1);
                }
            }
            NavKey::RowDown => {
                if self.surface == Surface::Mode {
                    self.select_next_visible_row();
                }
            }
            NavKey::RowUp => {
                if self.surface == Surface::Mode {
                    self.select_previous_visible_row();
                }
            }
        }
    }

    fn select_next_visible_row(&mut self) {
        let next = {
            let visible = self.visible_row_ids(self.mode);
            if visible.is_empty() {
                return;
            }
            let index = match self.selection(self.mode) {
                Some(current) => visible
                    .iter()
                    .position(|id| id == current)
                    .map(|i| (i + 1).min(visible.len() - 1))
                    .unwrap_or(0),
                None => 0,
            };
            visible[index].clone()
        };
        self.select(self.mode, next);
    }

    fn select_previous_visible_row(&mut self) {
        let previous = {
            let visible = self.visible_row_ids(self.mode);
            if visible.is_empty() {
                return;
            }
            let index = match self.selection(self.mode) {
                Some(current) => visible
                    .iter()
                    .position(|id| id == current)
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0),
                None => visible.len() - 1,
            };
            visible[index].clone()
        };
        self.select(self.mode, previous);
    }

    pub fn set_scan_roots(&mut self, roots: Vec<String>) {
        self.scan_roots = roots;
        let memory = &mut self.memories[Mode::Clean.index()];
        memory.selection = None;
        memory.review = Review::StaleScanRoots;
    }

    pub fn set_protections(&mut self, paths: Vec<String>) {
        self.protections = paths;
        let memory = &mut self.memories[Mode::Clean.index()];
        memory.selection = None;
        memory.review = Review::StaleProtection;
    }

    /// Replaces the protection list without marking the review stale.
    ///
    /// The in-app protection toggle re-derives the read-only rows from this
    /// list directly, so no rescan is needed for that edit.
    pub fn replace_protections(&mut self, paths: Vec<String>) {
        self.protections = paths;
    }

    pub fn begin_operation(&mut self, class: OperationClass) -> u64 {
        // A live mutating operation is single-flight. Do not replace or cancel it.
        if let Some(operation) = &self.operation
            && operation.class == OperationClass::Mutating
        {
            return operation.id;
        }
        self.next_operation += 1;
        let id = self.next_operation;
        self.operation = Some(LiveOperation {
            id,
            class,
            mode: self.mode,
            canceled: false,
        });
        self.activity = Activity::Busy;
        id
    }

    pub fn cancel(&mut self, id: u64) -> bool {
        let Some(operation) = &mut self.operation else {
            return false;
        };
        if operation.id != id || operation.class != OperationClass::ReadOnly {
            return false;
        }
        operation.canceled = true;
        true
    }

    pub fn request_close(&mut self) -> CloseAction {
        let Some(operation) = &mut self.operation else {
            return CloseAction::Quit;
        };
        if operation.class == OperationClass::ReadOnly {
            operation.canceled = true;
            CloseAction::CancelAndJoin { id: operation.id }
        } else {
            CloseAction::WaitOnly { id: operation.id }
        }
    }

    pub fn settle(&mut self, id: u64) -> bool {
        match &self.operation {
            Some(operation) if operation.id == id && operation.canceled => {
                self.operation = None;
                self.activity = Activity::Idle;
                true
            }
            _ => false,
        }
    }

    /// Returns the live operation id, if any.
    pub fn operation_id(&self) -> Option<u64> {
        self.operation.as_ref().map(|operation| operation.id)
    }

    /// Clears one terminal operation regardless of its cancel flag. A worker
    /// result that already reached the shell owns this transition.
    pub fn finish_operation(&mut self, id: u64) -> bool {
        match &self.operation {
            Some(operation) if operation.id == id => {
                self.operation = None;
                self.activity = Activity::Idle;
                true
            }
            _ => false,
        }
    }

    /// Appends one truthful history entry, keeping only the newest entries.
    pub fn push_history(&mut self, entry: impl Into<String>) {
        const HISTORY_CAP: usize = 20;
        self.history.push(entry.into());
        let overflow = self.history.len().saturating_sub(HISTORY_CAP);
        if overflow > 0 {
            self.history.drain(..overflow);
        }
    }

    pub(crate) fn reject_result(&mut self) {
        self.rejected_results += 1;
    }

    pub fn apply_completion(&mut self, id: u64, report: impl Into<String>) -> bool {
        let Some(operation) = &self.operation else {
            self.rejected_results += 1;
            return false;
        };
        if operation.id != id || operation.canceled {
            self.rejected_results += 1;
            return false;
        }
        let mode = operation.mode;
        let memory = &mut self.memories[mode.index()];
        memory.report = Some(report.into());
        memory.review = Review::Valid;
        self.operation = None;
        self.activity = Activity::Idle;
        true
    }

    pub fn drain_mailbox(&mut self, mailbox: &SharedMailbox) -> Poll {
        let batch = mailbox.take_batch(32);
        for event in &batch {
            self.apply_mail(event);
        }
        Poll {
            applied: batch.len(),
            decorative_frame: self.decorative_deadline().is_some(),
        }
    }

    /// Applies one mailbox event. Clean worker events are owned by the session,
    /// which pairs the workbench projection with these shell transitions.
    pub(crate) fn apply_mail(&mut self, event: &MailEvent) {
        match event {
            MailEvent::Lifecycle { .. } => self.lifecycle_applied += 1,
            MailEvent::Completion { id, report } => {
                let _ = self.apply_completion(*id, report.clone());
            }
            MailEvent::CleanScan { .. }
            | MailEvent::CleanExecute { .. }
            | MailEvent::CleanAiScan { .. }
            | MailEvent::CleanAiExecute { .. }
            | MailEvent::CleanAiRecovery { .. }
            | MailEvent::CleanAiRestore { .. } => {}
        }
    }

    pub fn set_presence(&mut self, presence: Presence) {
        self.presence = presence;
    }

    pub fn set_motion(&mut self, motion: DesktopMotion) {
        self.motion = motion;
    }

    /// C10 paints stills only. No presence or activity schedules a repeating frame.
    pub fn decorative_deadline(&self) -> Option<std::time::Duration> {
        match (self.presence, self.activity, self.motion) {
            (Presence::Minimized | Presence::Hidden, _, _) => None,
            (_, Activity::Idle, _) => None,
            (_, _, DesktopMotion::Reduced) => None,
            (Presence::Visible, Activity::Busy, DesktopMotion::System) => None,
        }
    }

    pub fn request_decorative_frame(&mut self) -> bool {
        self.decorative_rejections += 1;
        false
    }

    pub fn decorative_frames_scheduled(&self) -> u32 {
        0
    }

    pub fn decorative_rejections(&self) -> u32 {
        self.decorative_rejections
    }
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

/// Classifies one worker error into the shared AI failure shape.
///
/// A stable `AiError` survives the helper boundary unchanged; anything else
/// becomes an honest failed diagnostic instead of being dropped.
fn ai_failure(error: anyhow::Error) -> AiError {
    let classified = error.downcast_ref::<AiError>().cloned();
    classified.unwrap_or_else(|| AiError {
        exit: AiExit::Failed,
        code: "ai_worker_failed",
        message: error.to_string(),
    })
}

fn move_index(index: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let next = index as isize + delta;
    next.clamp(0, len as isize - 1) as usize
}

fn visible_slice(ids: &[String], start: usize) -> &[String] {
    if ids.is_empty() {
        return ids;
    }
    let cap = VISIBLE_ROW_CAPACITY;
    if start == 0 {
        return &ids[..cap.min(ids.len())];
    }
    // A caller using `index - capacity` (9000 - 48 = 8952) still sees that
    // index: inclusive last is start+capacity, clamped to 48 rows.
    let last = start.saturating_add(cap).min(ids.len() - 1);
    let first = last.saturating_sub(cap - 1);
    &ids[first..=last]
}

pub struct Session {
    shell: ShellState,
    mailbox: Arc<SharedMailbox>,
    files: Option<PreferenceFiles>,
    draft: Draft,
    desktop_blocked: bool,
    language_blocked: bool,
    scan_blocked: bool,
    protection_blocked: bool,
    notices: Vec<StoreName>,
    analyze_board: Option<AnalyzeBoard>,
    analyze_window: Option<AnalyzeWindow>,
    status_panel: Option<StatusPanel>,
    status_lines: Vec<String>,
    workbench_review: WorkbenchReview,
    software_review: SoftwareReview,
    optimize_board: OptimizeBoard,
    analyze_roots: Vec<AnalyzeRootChoice>,
    analyze_notice: Option<String>,
    analyze_landing_prepared: bool,
    status_phase: StatusPhase,
    status_snapshot: Option<StatusSnapshotV1>,
    status_query: String,
    status_sort: ProcessSort,
    status_pick: Option<crate::status_panel::ProcessPick>,
    status_live: Option<LiveControl>,
    status_receiver: Option<devsweep_core::status::StatusEventReceiver>,
    status_subscription_starts: u32,
    status_subscription_stops: u32,
    clean: CleanWorkbench,
    clean_ai: CleanAiView,
    clean_cancel: Option<Arc<FlagCancelObserver>>,
    /// Out-of-band scan progress. Progress never occupies the mailbox, so the
    /// only event a woken drain can observe is a terminal payload.
    clean_progress: Option<Arc<AtomicUsize>>,
    clean_progress_seen: usize,
    protection_session_only: bool,
    /// When true (the running window), slow read-only work is queued as
    /// [`BackgroundJob`]s instead of running inline on the UI thread. Tests and
    /// `Session::unbacked` keep the synchronous behavior.
    background_jobs: bool,
    jobs: Vec<BackgroundJob>,
    job_generation: u64,
    software_job: Option<u64>,
    status_job: Option<u64>,
    analyze_job: Option<(u64, PathBuf, Arc<FlagCancelObserver>)>,
}

/// One slow, read-only operation the window runs on a worker thread.
pub enum BackgroundJob {
    SoftwareHost {
        generation: u64,
        segment: crate::workbench::SoftwareSegment,
    },
    StatusSnapshot {
        generation: u64,
    },
    AnalyzeScan {
        generation: u64,
        path: PathBuf,
        cancel: Arc<FlagCancelObserver>,
    },
}

/// The finished result of one [`BackgroundJob`].
pub enum JobResult {
    SoftwareHost {
        generation: u64,
        load: Option<crate::software_review::SoftwareHostLoad>,
    },
    StatusSnapshot {
        generation: u64,
        result: Result<StatusSnapshotV1, String>,
    },
    AnalyzeScan {
        generation: u64,
        result: Result<AnalyzeWindow, String>,
    },
}

impl BackgroundJob {
    /// Runs the job. Blocking; call it from a worker thread only.
    #[must_use]
    pub fn run(self) -> JobResult {
        match self {
            Self::SoftwareHost {
                generation,
                segment,
            } => JobResult::SoftwareHost {
                generation,
                load: crate::software_review::load_host(segment),
            },
            Self::StatusSnapshot { generation } => JobResult::StatusSnapshot {
                generation,
                result: devsweep_core::status::capture_snapshot(8, None)
                    .map_err(|error| error.to_string()),
            },
            Self::AnalyzeScan {
                generation,
                path,
                cancel,
            } => JobResult::AnalyzeScan {
                generation,
                result: AnalyzeWindow::scan_with_cancel(&path, Some(cancel.as_ref())),
            },
        }
    }
}

impl Session {
    pub fn launch(files: PreferenceFiles) -> Self {
        let loaded = preferences::load_stores(&files);
        Self::from_loaded(Some(files), loaded)
    }

    pub fn unbacked() -> Self {
        Self::from_loaded(
            None,
            preferences::LoadedStores {
                desktop: DesktopPreferences::default(),
                desktop_blocked: false,
                language: None,
                language_blocked: false,
                scan_roots: Vec::new(),
                scan_blocked: false,
                protections: Vec::new(),
                protection_blocked: false,
            },
        )
    }

    fn from_loaded(files: Option<PreferenceFiles>, loaded: preferences::LoadedStores) -> Self {
        let locale = match loaded.language {
            Some(LanguageTag::ZhCn) => Locale::ZhCn,
            Some(LanguageTag::En) | None => Locale::En,
        };
        let mut shell = ShellState::from_preferences(loaded.desktop.clone(), locale);
        shell.scan_roots = loaded.scan_roots;
        shell.protections = loaded.protections;
        let mut clean = CleanWorkbench::new();
        clean.set_protected_paths(shell.protections.clone());
        let mut notices = Vec::new();
        if loaded.desktop_blocked {
            notices.push(StoreName::Desktop);
        }
        if loaded.language_blocked {
            notices.push(StoreName::Language);
        }
        if loaded.scan_blocked {
            notices.push(StoreName::ScanRoots);
        }
        if loaded.protection_blocked {
            notices.push(StoreName::Protections);
        }
        Self {
            shell,
            mailbox: Arc::new(SharedMailbox::new()),
            files,
            draft: Draft {
                theme: loaded.desktop.theme,
                text_scale_percent: loaded.desktop.text_scale_percent,
                motion: loaded.desktop.motion,
                language: loaded.language,
                committed_language: loaded.language,
                committed: loaded.desktop,
            },
            desktop_blocked: loaded.desktop_blocked,
            language_blocked: loaded.language_blocked,
            scan_blocked: loaded.scan_blocked,
            protection_blocked: loaded.protection_blocked,
            notices,
            analyze_board: None,
            analyze_window: None,
            status_panel: None,
            status_lines: Vec::new(),
            workbench_review: WorkbenchReview::new(),
            software_review: SoftwareReview::new(),
            optimize_board: OptimizeBoard::new(),
            analyze_roots: Vec::new(),
            analyze_notice: None,
            analyze_landing_prepared: false,
            status_phase: StatusPhase::Landing,
            status_snapshot: None,
            status_query: String::new(),
            status_sort: ProcessSort::Name,
            status_pick: None,
            status_live: None,
            status_receiver: None,
            status_subscription_starts: 0,
            status_subscription_stops: 0,
            clean,
            clean_ai: CleanAiView::new(),
            clean_cancel: None,
            clean_progress: None,
            clean_progress_seen: 0,
            protection_session_only: false,
            background_jobs: false,
            jobs: Vec::new(),
            job_generation: 0,
            software_job: None,
            status_job: None,
            analyze_job: None,
        }
    }

    /// The window opts in: slow read-only work becomes background jobs.
    pub fn enable_background_jobs(&mut self) {
        self.background_jobs = true;
    }

    /// Drains queued jobs for the window to run off the UI thread.
    pub fn take_jobs(&mut self) -> Vec<BackgroundJob> {
        std::mem::take(&mut self.jobs)
    }

    fn next_generation(&mut self) -> u64 {
        self.job_generation = self.job_generation.wrapping_add(1);
        self.job_generation
    }

    /// True while any background job is outstanding.
    pub fn background_busy(&self) -> bool {
        self.software_job.is_some() || self.status_job.is_some() || self.analyze_job.is_some()
    }

    /// True while a Status snapshot is being sampled off the UI thread.
    pub fn status_sampling(&self) -> bool {
        self.status_job.is_some()
    }

    /// The folder an Analyze walk is reading, while it runs.
    pub fn analyze_scanning(&self) -> Option<&Path> {
        self.analyze_job.as_ref().map(|(_, path, _)| path.as_path())
    }

    /// Requests cooperative cancellation of the running Analyze walk.
    pub fn analyze_cancel_scan(&mut self) -> bool {
        let Some((_, _, cancel)) = self.analyze_job.take() else {
            return false;
        };
        cancel.request_cancel();
        self.analyze_notice = Some(
            crate::copy::pick(self.locale(), "Analysis canceled.", "已取消分析。").to_string(),
        );
        true
    }

    /// Applies one finished job. Results whose generation no longer matches
    /// the outstanding request (superseded or canceled) are dropped.
    pub fn apply_job(&mut self, result: JobResult) -> bool {
        match result {
            JobResult::SoftwareHost { generation, load } => {
                if self.software_job != Some(generation) {
                    return false;
                }
                self.software_job = None;
                if let Some(load) = load {
                    let locale = self.locale();
                    self.software_review.apply_host_load(load, locale);
                    self.sync_software_rows();
                }
                true
            }
            JobResult::StatusSnapshot { generation, result } => {
                if self.status_job != Some(generation) {
                    return false;
                }
                self.status_job = None;
                self.apply_status_capture(result);
                true
            }
            JobResult::AnalyzeScan { generation, result } => {
                if self.analyze_job.as_ref().map(|job| job.0) != Some(generation) {
                    return false;
                }
                self.analyze_job = None;
                self.install_analyze_result(result);
                true
            }
        }
    }

    pub fn shell(&self) -> &ShellState {
        &self.shell
    }

    pub fn locale(&self) -> Locale {
        self.shell.locale
    }

    pub fn theme(&self) -> DesktopTheme {
        self.draft.theme
    }

    pub fn text_scale(&self) -> u8 {
        self.draft.text_scale_percent
    }

    pub fn motion(&self) -> DesktopMotion {
        self.draft.motion
    }

    pub fn set_motion(&mut self, motion: DesktopMotion) {
        self.draft.motion = motion;
        self.shell.set_motion(motion);
    }

    pub fn notices(&self) -> &[StoreName] {
        &self.notices
    }

    pub fn workbench_notice(&self) -> Option<&WorkbenchNotice> {
        self.shell.workbench_notice()
    }

    pub fn set_workbench_notice(&mut self, notice: Option<WorkbenchNotice>) {
        self.shell.set_workbench_notice(notice);
    }

    /// The Clean outcome notice is owned by the workbench state machine. It is
    /// the single source for the rendered outcome line.
    pub fn clean_outcome(&self) -> Option<CleanOutcomeNotice> {
        self.clean.notice()
    }

    /// Replaces the workbench outcome notice without touching the rows or the
    /// selection. The workbench still derives its own notices from scan and
    /// execution results.
    pub fn set_clean_outcome(&mut self, notice: Option<CleanOutcomeNotice>) {
        self.clean.set_notice(notice);
    }

    pub fn set_analyze_board(&mut self, board: Option<AnalyzeBoard>) {
        self.analyze_board = board;
    }

    pub fn analyze_board(&self) -> Option<&AnalyzeBoard> {
        self.analyze_board.as_ref()
    }

    pub fn analyze_face_line(&self) -> Option<String> {
        self.analyze_board
            .as_ref()
            .and_then(AnalyzeBoard::selected_face_line)
    }

    pub fn set_status_panel(&mut self, panel: Option<StatusPanel>) {
        self.status_panel = panel;
    }

    pub fn status_sensor_line(&self) -> Option<String> {
        self.status_panel
            .as_ref()
            .and_then(|panel| panel.sensor_line().map(str::to_string))
    }

    pub fn status_pid_line(&self) -> Option<String> {
        self.status_panel
            .as_ref()
            .and_then(StatusPanel::displayed_pid_line)
    }

    pub fn apply_key(&mut self, key: NavKey) {
        if self.shell.screen_covered() {
            self.shell.apply_key(key);
            return;
        }
        // Escape from Settings returns to the workbench that opened it.
        // Only Escape on the Clean workbench can cancel its worker or close AI.
        if key == NavKey::Escape
            && self.shell.surface() == Surface::Mode
            && self.shell.mode() == Mode::Clean
            && !self.shell.menu_open()
            && (self.clean_request_cancel() || self.clean_back())
        {
            return;
        }
        if key == NavKey::Escape
            && self.shell.surface() == Surface::Mode
            && self.close_nested_review()
        {
            return;
        }
        let before = self.shell.surface();
        let mode_before = self.shell.mode();
        if !self.shell.menu_open() && self.shell.surface() == Surface::Mode {
            if key == NavKey::Enter && self.activate_mode_enter() {
                return;
            }
            if matches!(key, NavKey::RowDown | NavKey::RowUp)
                && matches!(
                    self.shell.mode(),
                    Mode::Software | Mode::Optimize | Mode::Analyze
                )
                && self.slide_workbench_row(key)
            {
                if self.shell.mode() == Mode::Analyze {
                    self.sync_analyze_board_selection();
                }
                return;
            }
        }
        self.shell.apply_key(key);
        if self.shell.mode() != mode_before {
            self.ensure_workbench_rows();
        }
        self.note_history_opened(before);
    }

    fn activate_mode_enter(&mut self) -> bool {
        match self.shell.mode() {
            Mode::Clean => {
                if self.clean_ai.open() {
                    self.clean_ai_enter()
                } else {
                    self.clean_enter()
                }
            }
            Mode::Software | Mode::Optimize => {
                self.workbench_enter();
                true
            }
            Mode::Analyze => {
                self.analyze_enter();
                true
            }
            Mode::Status => {
                self.status_enter();
                true
            }
        }
    }

    fn workbench_enter(&mut self) {
        self.ensure_workbench_rows();
        let mode = self.shell.mode();
        let id = self
            .shell
            .selection(mode)
            .map(str::to_string)
            .or_else(|| self.shell.visible_row_ids(mode).first().cloned());
        let Some(id) = id else {
            return;
        };
        if self.shell.selection(mode).is_none() {
            self.shell.select(mode, id.clone());
        }
        let locale = self.locale();
        if mode == Mode::Software {
            self.software_enter(&id, locale);
            return;
        }
        if mode == Mode::Optimize {
            if let Some(group) = crate::optimize_board::group_from_id(&id) {
                self.optimize_board.open_group(group);
                self.sync_optimize_rows();
                return;
            }
            if !crate::optimize_board::schedule_allowed(&id) {
                self.optimize_board.note_failed(&id);
            }
        }
        self.workbench_review.activate(&id, locale);
        if let Some(reason) = self.workbench_review.precheck() {
            self.optimize_board.note_admin_wait(&id);
            if let Some(notice) = WorkbenchNotice::from_precheck_reason(reason) {
                self.shell.set_workbench_notice(Some(notice));
            }
        } else if !crate::optimize_board::schedule_allowed(&id) {
            self.optimize_board.note_failed(&id);
        }
    }

    fn software_enter(&mut self, id: &str, locale: Locale) {
        if self.software_review.is_open() {
            self.software_review.activate(id, locale, None);
            self.sync_software_rows();
            return;
        }
        if id == "leftovers" {
            self.workbench_review.activate(id, locale);
            return;
        }
        let segment = match id {
            "uninstall" => Some(crate::workbench::SoftwareSegment::Uninstall),
            "updates" => Some(crate::workbench::SoftwareSegment::Updates),
            "startup" => Some(crate::workbench::SoftwareSegment::Startup),
            _ => None,
        };
        if let Some(segment) = segment {
            if self.background_jobs && !SoftwareReview::has_fixture(segment) {
                self.queue_software_load(segment, locale);
                return;
            }
            self.software_review.open_segment(segment, locale);
            self.sync_software_rows();
            return;
        }
        self.workbench_review.activate(id, locale);
    }

    fn queue_software_load(&mut self, segment: crate::workbench::SoftwareSegment, locale: Locale) {
        self.software_review.begin_host_load(segment, locale);
        self.sync_software_rows();
        let generation = self.next_generation();
        self.software_job = Some(generation);
        self.jobs.push(BackgroundJob::SoftwareHost {
            generation,
            segment,
        });
    }

    fn close_nested_review(&mut self) -> bool {
        if self.analyze_cancel_scan() {
            return true;
        }
        if self.software_review.is_open() {
            self.software_job = None;
            self.software_review.close();
            self.shell
                .set_row_ids(Mode::Software, crate::workbench_review::software_row_ids());
            return true;
        }
        if self.optimize_board.group().is_some() {
            self.optimize_board.close_group();
            self.sync_optimize_rows();
            return true;
        }
        if self.shell.mode() == Mode::Analyze && self.analyze_window.is_some() {
            self.analyze_window = None;
            self.analyze_board = None;
            self.shell
                .set_row_ids(Mode::Analyze, self.analyze_root_ids());
            return true;
        }
        false
    }

    fn sync_software_rows(&mut self) {
        if self.software_review.is_open() {
            self.shell
                .set_row_ids(Mode::Software, self.software_review.filtered_ids());
        }
    }

    fn sync_optimize_rows(&mut self) {
        let mut ids = Vec::new();
        if self.optimize_board.group().is_none() {
            ids.extend(crate::optimize_board::browse_ids());
        } else {
            ids.extend([
                crate::optimize_board::GROUP_STARTUP.to_string(),
                crate::optimize_board::GROUP_NETWORK.to_string(),
                crate::optimize_board::GROUP_MAINTENANCE.to_string(),
            ]);
            ids.extend(self.optimize_board.visible_ids());
        }
        self.shell.set_row_ids(Mode::Optimize, ids);
    }

    fn analyze_enter(&mut self) {
        // Landing never reads DEVSWEEP_ANALYZE_ROOT. A listed root or an
        // explicit folder pick is the only scan.
        if self.analyze_window.is_none() {
            let _ = self.open_selected_analyze_root();
            return;
        }
        let locale = self.locale();
        let selected = self
            .shell
            .selection(Mode::Analyze)
            .and_then(|id| id.parse::<u32>().ok());
        let selected_kind = selected.and_then(|id| {
            self.analyze_window.as_ref().and_then(|window| {
                window
                    .snapshot()
                    .nodes
                    .iter()
                    .find(|node| node.id == id)
                    .map(|node| node.kind)
            })
        });
        let Some(window) = self.analyze_window.as_mut() else {
            return;
        };
        let fallback = window.board().list_ids().first().copied();
        if let Some(selected) = selected {
            if selected_kind == Some(devsweep_core::analysis::AnalyzeNodeKind::Directory) {
                window.drill(selected);
                let ids = window.child_ids().iter().map(|id| id.to_string()).collect();
                self.shell.set_row_ids(Mode::Analyze, ids);
                self.analyze_board = Some(window.board().clone());
                return;
            }
            window.select(selected);
        } else if let Some(id) = fallback {
            window.select(id);
        }
        let already = window.recycle_line().is_some();
        if already {
            let _ = window.refuse_unconfirmed(locale);
        } else {
            let _ = window.preview_selected(locale);
        }
        let board = window.board().clone();
        let chosen = window.board().list_selection();
        if selected.is_none()
            && let Some(id) = chosen
        {
            self.shell.select(Mode::Analyze, id.to_string());
        }
        self.analyze_board = Some(board);
    }

    fn status_enter(&mut self) {
        if self.background_jobs {
            if self.status_job.is_none() {
                let generation = self.next_generation();
                self.status_job = Some(generation);
                self.jobs.push(BackgroundJob::StatusSnapshot { generation });
            }
            return;
        }
        let result =
            devsweep_core::status::capture_snapshot(8, None).map_err(|error| error.to_string());
        self.apply_status_capture(result);
    }

    fn apply_status_capture(&mut self, result: Result<StatusSnapshotV1, String>) {
        // Linux preview builds have no collectors. A snapshot JSON file stands
        // in so the Status layout can be reviewed; Windows builds never read it.
        #[cfg(feature = "linux-preview")]
        let result = std::env::var_os("DEVSWEEP_PREVIEW_STATUS_FILE")
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<StatusSnapshotV1>(&bytes).ok())
            .map_or(result, Ok);
        let mut panel = self.status_panel.take().unwrap_or_default();
        panel.set_main_visible(true);
        match result {
            Ok(snapshot) => {
                self.status_lines = crate::status_panel::snapshot_lines(&snapshot);
                self.apply_status_snapshot(snapshot.clone());
                if self
                    .status_lines
                    .iter()
                    .any(|line| line.contains("unavailable") || line.contains("permission_denied"))
                {
                    panel.set_sensor(&devsweep_core::status::AvailabilityV1::<()>::unavailable(
                        "sensor_unavailable",
                    ));
                }
                if let Some(pid) = crate::status_panel::displayed_pid_from_snapshot(&snapshot) {
                    panel.set_displayed_pid(Some(pid));
                    let error = panel
                        .request_kill_displayed_pid()
                        .expect_err("displayed pid is not a kill");
                    self.status_lines.push(error.to_string());
                }
            }
            Err(error) => {
                panel.set_sensor(&devsweep_core::status::AvailabilityV1::<()>::unavailable(
                    "snapshot_unavailable",
                ));
                self.status_lines = vec![format!("unavailable snapshot_unavailable: {error}")];
            }
        }
        panel.reconcile(|| {}, || {});
        self.status_panel = Some(panel);
    }

    fn slide_workbench_row(&mut self, key: NavKey) -> bool {
        let mode = self.shell.mode();
        let ids = self.shell.filtered_row_ids(mode).to_vec();
        if ids.is_empty() {
            return false;
        }
        let current = self.shell.selection(mode);
        let index = current.and_then(|id| ids.iter().position(|item| item == id));
        let next = match (current, key) {
            (None, _) => 0,
            (Some(_), NavKey::RowDown) => (index.unwrap_or(0) + 1).min(ids.len() - 1),
            (Some(_), NavKey::RowUp) => index.unwrap_or(0).saturating_sub(1),
            (Some(_), _) => index.unwrap_or(0),
        };
        let id = ids[next].clone();
        self.shell.select(mode, id);
        let start = next.saturating_sub(crate::shell::VISIBLE_ROW_CAPACITY / 2);
        self.shell.set_row_window_start(mode, start);
        true
    }

    fn ensure_workbench_rows(&mut self) {
        match self.shell.mode() {
            Mode::Software if self.shell.filtered_row_ids(Mode::Software).is_empty() => {
                self.shell
                    .set_row_ids(Mode::Software, crate::workbench_review::software_row_ids());
            }
            Mode::Optimize if self.shell.filtered_row_ids(Mode::Optimize).is_empty() => {
                self.sync_optimize_rows();
            }
            _ => {}
        }
    }

    /// Click path for one Software, Optimize, or Analyze row: select it and run
    /// the same Enter activation the keyboard uses. Activation plans and
    /// previews; it never confirms on the first press.
    pub fn activate_mode_row(&mut self, mode: Mode, id: impl Into<String>) {
        if self.shell.mode() != mode || self.shell.surface() != Surface::Mode {
            return;
        }
        self.shell.select(mode, id);
        if mode == Mode::Analyze {
            self.sync_analyze_board_selection();
        }
        self.activate_mode_enter();
    }

    /// Opens one Software segment directly (the segmented control). Leaves any
    /// open segment first so the three segments never stack.
    pub fn software_open_segment(&mut self, segment: crate::workbench::SoftwareSegment) {
        if self.shell.mode() != Mode::Software {
            return;
        }
        if self.software_review.is_open() {
            self.software_review.close();
            self.shell
                .set_row_ids(Mode::Software, crate::workbench_review::software_row_ids());
        }
        let id = match segment {
            crate::workbench::SoftwareSegment::Uninstall => "uninstall",
            crate::workbench::SoftwareSegment::Updates => "updates",
            crate::workbench::SoftwareSegment::Startup => "startup",
            crate::workbench::SoftwareSegment::Leftovers => "leftovers",
        };
        self.shell.select(Mode::Software, id);
        let locale = self.locale();
        self.software_enter(id, locale);
    }

    /// Returns to the Software landing.
    pub fn software_close_segment(&mut self) {
        if self.software_review.is_open() {
            self.software_review.close();
            self.shell
                .set_row_ids(Mode::Software, crate::workbench_review::software_row_ids());
        }
    }

    /// Leaves an open Optimize group, or the Analyze detail, like Escape.
    pub fn close_mode_detail(&mut self) -> bool {
        self.close_nested_review()
    }

    pub fn select_mode_row(&mut self, mode: Mode, id: impl Into<String>) {
        self.shell.select(mode, id);
    }

    pub fn set_mode_row_window(&mut self, mode: Mode, start: usize) {
        self.shell.set_row_window_start(mode, start);
    }

    /// Id of the Software/Optimize review awaiting its second activation.
    pub fn workbench_pending_id(&self) -> Option<&str> {
        self.workbench_review.pending_id()
    }

    pub fn workbench_review_lines(&self) -> &[String] {
        self.workbench_review.lines()
    }

    pub fn offer_analyze_roots(&mut self, paths: &[std::path::PathBuf]) {
        self.analyze_landing_prepared = true;
        self.analyze_roots = crate::analyze_window::landing_roots(paths);
        self.shell
            .set_row_ids(Mode::Analyze, self.analyze_root_ids());
    }

    pub fn prepare_analyze_landing(&mut self) {
        if self.analyze_landing_prepared || self.analyze_window.is_some() {
            return;
        }
        self.analyze_landing_prepared = true;
        let drives = crate::analyze_window::landing_roots(&[]);
        let extras = self
            .shell
            .scan_roots()
            .iter()
            .filter(|root| !root.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let extra_choices = if extras.is_empty() {
            Vec::new()
        } else {
            crate::analyze_window::landing_roots(&extras)
        };
        self.analyze_roots = merge_analyze_roots(drives, extra_choices);
        if self.shell.mode() == Mode::Analyze {
            self.shell
                .set_row_ids(Mode::Analyze, self.analyze_root_ids());
        }
    }

    fn analyze_root_ids(&self) -> Vec<String> {
        self.analyze_roots
            .iter()
            .enumerate()
            .map(|(index, _)| format!("root:{index}"))
            .collect()
    }

    fn open_selected_analyze_root(&mut self) -> bool {
        let Some(selected) = self.shell.selection(Mode::Analyze) else {
            return false;
        };
        let Some(index) = selected
            .strip_prefix("root:")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            return false;
        };
        self.open_analyze_choice(index)
    }

    /// Opens one drive or folder the window already listed. Does not read
    /// `DEVSWEEP_ANALYZE_ROOT`.
    pub fn analyze_open_listed(&mut self, index: usize) -> bool {
        if index >= self.analyze_roots.len() {
            return false;
        }
        self.shell.select(Mode::Analyze, format!("root:{index}"));
        self.open_analyze_choice(index)
    }

    /// Scans one folder the user picked. The path is the scan root; the
    /// analyze environment variable is not consulted.
    pub fn analyze_open_isolated_root(&mut self, root: PathBuf) -> bool {
        self.install_analyze_scan(&root)
    }

    /// Selects one snapshot node in both the list and the treemap.
    pub fn analyze_select(&mut self, id: u32) {
        self.shell.select(Mode::Analyze, id.to_string());
        self.sync_analyze_board_selection();
    }

    pub fn analyze_notice(&self) -> Option<&str> {
        self.analyze_notice.as_deref()
    }

    /// True when the list shows a folder below the scan root.
    pub fn analyze_can_go_up(&self) -> bool {
        self.analyze_window.as_ref().is_some_and(|window| {
            window
                .snapshot()
                .nodes
                .iter()
                .find(|node| node.id == window.parent_id())
                .is_some_and(|node| node.parent_id.is_some())
        })
    }

    /// Moves the list to the parent folder. Does not rescan.
    pub fn analyze_go_up(&mut self) -> bool {
        let Some(window) = self.analyze_window.as_mut() else {
            return false;
        };
        let current = window.parent_id();
        let Some(up) = window
            .snapshot()
            .nodes
            .iter()
            .find(|node| node.id == current)
            .and_then(|node| node.parent_id)
        else {
            return false;
        };
        window.drill(up);
        let ids = window.child_ids().iter().map(|id| id.to_string()).collect();
        self.analyze_board = Some(window.board().clone());
        self.shell.set_row_ids(Mode::Analyze, ids);
        self.shell.select(Mode::Analyze, current.to_string());
        self.sync_analyze_board_selection();
        true
    }

    /// Builds the Recycle Bin plan for the selected node and paints its live
    /// `sha256:` digest. Nothing moves.
    pub fn analyze_preview_recycle(&mut self) -> bool {
        let locale = self.locale();
        let Some(window) = self.analyze_window.as_mut() else {
            return false;
        };
        let ok = window.preview_selected(locale).is_ok();
        self.analyze_board = Some(window.board().clone());
        ok
    }

    /// Drops the preview. Nothing moves.
    pub fn analyze_cancel_recycle(&mut self) {
        if let Some(window) = self.analyze_window.as_mut() {
            window.cancel_recycle();
        }
    }

    /// Explicit confirm from the reviewed preview. The digest the user saw
    /// goes to the same core executor the CLI's `--confirm` path calls; a
    /// stale snapshot or a changed file refuses before any move.
    pub fn analyze_confirm_recycle(&mut self) -> Result<String, String> {
        let locale = self.locale();
        let Some(window) = self.analyze_window.as_mut() else {
            return Err("analyze snapshot is not open".to_string());
        };
        window.execute_confirmed(locale)
    }

    /// Shows the selected node in Explorer. Program and argv stay separate;
    /// off Windows the core refuses before starting a process.
    pub fn analyze_reveal(&mut self) -> Result<(), String> {
        let Some(window) = self.analyze_window.as_ref() else {
            return Err("analyze snapshot is not open".to_string());
        };
        match window.reveal_selected(true) {
            Ok(_) => {
                self.analyze_notice = None;
                Ok(())
            }
            Err(error) => {
                self.analyze_notice = Some(error.clone());
                Err(error)
            }
        }
    }

    fn open_analyze_choice(&mut self, index: usize) -> bool {
        let Some(choice) = self.analyze_roots.get(index) else {
            return false;
        };
        if choice.error.is_some() {
            self.analyze_notice = choice.error.clone();
            return true;
        }
        let path = choice
            .normalized
            .clone()
            .unwrap_or_else(|| choice.input.clone());
        let opened = self.install_analyze_scan(Path::new(&path));
        opened || self.analyze_notice.is_some()
    }

    fn install_analyze_scan(&mut self, path: &Path) -> bool {
        if self.background_jobs {
            if !path.exists() {
                self.analyze_notice = Some("analyze root is missing".to_string());
                return false;
            }
            if let Some((_, _, cancel)) = self.analyze_job.take() {
                cancel.request_cancel();
            }
            let generation = self.next_generation();
            let cancel = Arc::new(FlagCancelObserver::new());
            self.analyze_notice = None;
            self.analyze_job = Some((generation, path.to_path_buf(), cancel.clone()));
            self.jobs.push(BackgroundJob::AnalyzeScan {
                generation,
                path: path.to_path_buf(),
                cancel,
            });
            return true;
        }
        self.install_analyze_result(AnalyzeWindow::scan(path))
    }

    fn install_analyze_result(&mut self, result: Result<AnalyzeWindow, String>) -> bool {
        match result {
            Ok(window) => {
                let ids = window
                    .board()
                    .list_ids()
                    .iter()
                    .map(|id| id.to_string())
                    .collect();
                self.analyze_board = Some(window.board().clone());
                self.analyze_window = Some(window);
                self.analyze_notice = None;
                self.shell.set_row_ids(Mode::Analyze, ids);
                true
            }
            Err(error) => {
                self.analyze_notice = Some(error);
                false
            }
        }
    }

    fn sync_analyze_board_selection(&mut self) {
        let Some(selected) = self
            .shell
            .selection(Mode::Analyze)
            .and_then(|id| id.parse::<u32>().ok())
        else {
            return;
        };
        let current = self
            .analyze_window
            .as_ref()
            .and_then(|window| window.board().list_selection());
        if current == Some(selected) {
            return;
        }
        if let Some(window) = self.analyze_window.as_mut() {
            window.select(selected);
            self.analyze_board = Some(window.board().clone());
        }
    }

    pub fn analyze_window(&self) -> Option<&AnalyzeWindow> {
        self.analyze_window.as_ref()
    }

    pub fn analyze_roots(&self) -> &[AnalyzeRootChoice] {
        &self.analyze_roots
    }

    #[must_use]
    pub fn analyze_landing_prepared(&self) -> bool {
        self.analyze_landing_prepared
    }

    pub fn software_review(&self) -> &SoftwareReview {
        &self.software_review
    }

    pub fn software_set_query(&mut self, query: impl Into<String>) {
        self.software_review.set_query(query);
        self.sync_software_rows();
    }

    pub fn software_cycle_sort(&mut self) {
        self.software_review.cycle_sort();
        self.sync_software_rows();
    }

    pub fn software_cycle_filter(&mut self) {
        self.software_review.cycle_filter();
        self.sync_software_rows();
    }

    pub fn software_refresh(&mut self) {
        let locale = self.locale();
        if self.background_jobs
            && self.software_review.host_backed()
            && let Some(segment) = self.software_review.segment()
        {
            self.queue_software_load(segment, locale);
            return;
        }
        self.software_review.refresh(locale);
        self.sync_software_rows();
    }

    pub fn optimize_board(&self) -> &OptimizeBoard {
        &self.optimize_board
    }

    pub fn optimize_set_query(&mut self, query: impl Into<String>) {
        self.optimize_board.set_query(query);
        self.sync_optimize_rows();
    }

    pub fn status_phase(&self) -> StatusPhase {
        self.status_phase
    }

    pub fn status_snapshot(&self) -> Option<&StatusSnapshotV1> {
        self.status_snapshot.as_ref()
    }

    pub fn status_set_query(&mut self, query: impl Into<String>) {
        self.status_query = query.into();
    }

    pub fn status_cycle_sort(&mut self) {
        self.status_sort = match self.status_sort {
            ProcessSort::Name => ProcessSort::Cpu,
            ProcessSort::Cpu => ProcessSort::Memory,
            ProcessSort::Memory => ProcessSort::Pid,
            ProcessSort::Pid => ProcessSort::Name,
        };
    }

    pub fn status_sort(&self) -> ProcessSort {
        self.status_sort
    }

    pub fn status_query(&self) -> &str {
        &self.status_query
    }

    /// Pick a process row for its local explanation. Picking the same row
    /// again closes it. A pick is presentation only; no process action reads it.
    pub fn status_toggle_process(&mut self, row: &devsweep_core::status::ProcessV1) {
        let pick = crate::status_panel::ProcessPick::of(row);
        if self.status_pick.as_ref() == Some(&pick) {
            self.status_pick = None;
        } else {
            self.status_pick = Some(pick);
        }
    }

    pub fn status_clear_process(&mut self) {
        self.status_pick = None;
    }

    pub fn status_picked_process(&self) -> Option<&crate::status_panel::ProcessPick> {
        self.status_pick.as_ref()
    }

    pub fn apply_status_snapshot(&mut self, snapshot: StatusSnapshotV1) {
        self.status_snapshot = Some(snapshot);
    }

    /// Main and tray share one live control. A second start does not spawn again.
    pub fn status_set_main_visible(&mut self, visible: bool) {
        self.status_set_visible(Some(visible), None);
    }

    pub fn status_set_tray_visible(&mut self, visible: bool) {
        self.status_set_visible(None, Some(visible));
    }

    fn status_set_visible(&mut self, main: Option<bool>, tray: Option<bool>) {
        let panel = self.status_panel.get_or_insert_with(StatusPanel::new);
        if let Some(visible) = main {
            panel.set_main_visible(visible);
        }
        if let Some(visible) = tray {
            panel.set_tray_visible(visible);
        }
        let mut start = false;
        let mut stop = false;
        panel.reconcile(|| start = true, || stop = true);
        if start && self.status_live.is_none() {
            self.status_subscription_starts = self.status_subscription_starts.saturating_add(1);
        }
        if stop {
            self.status_subscription_stops = self.status_subscription_stops.saturating_add(1);
            if let Some(mut live) = self.status_live.take() {
                live.join();
            }
            self.status_receiver = None;
            self.status_phase = StatusPhase::Landing;
        }
    }

    pub fn status_subscription_starts(&self) -> u32 {
        self.status_subscription_starts
    }

    pub fn status_subscription_stops(&self) -> u32 {
        self.status_subscription_stops
    }

    /// Starts the accepted live sampler once. Hidden panels do not request a decorative frame.
    pub fn status_start_live(&mut self) -> Result<(), String> {
        if self.status_live.is_some() {
            self.status_phase = StatusPhase::Live;
            return Ok(());
        }
        let request = devsweep_core::status::LiveRequest {
            interval_ms: 1_000,
            process_limit: 8,
            operation_id: "gpui-status".to_string(),
            cancel: None,
        };
        match devsweep_core::status::spawn_live(request) {
            Ok((receiver, control)) => {
                self.status_receiver = Some(receiver);
                self.status_live = Some(control);
                self.status_phase = StatusPhase::Live;
                self.status_subscription_starts = self.status_subscription_starts.saturating_add(1);
                self.status_lines
                    .push(crate::copy::status_live_started(self.locale(), 1, 8));
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn status_stop_live(&mut self) {
        if let Some(mut live) = self.status_live.take() {
            live.join();
            self.status_subscription_stops = self.status_subscription_stops.saturating_add(1);
        }
        self.status_receiver = None;
        self.status_phase = StatusPhase::Landing;
        self.status_lines
            .push(crate::copy::status_live_stopped(self.locale(), "stopped"));
    }

    pub fn poll_status_live(&mut self) -> bool {
        let Some(receiver) = self.status_receiver.as_ref() else {
            return false;
        };
        let mut events = Vec::new();
        while let Ok(event) = receiver.recv_timeout(std::time::Duration::ZERO) {
            events.push(event);
        }
        let mut applied = false;
        for event in events {
            applied = true;
            match event {
                StatusEventV1::Snapshot { data, .. } => self.apply_status_snapshot(*data),
                StatusEventV1::Terminal { .. } => self.status_phase = StatusPhase::Landing,
                _ => {}
            }
        }
        applied
    }

    pub fn analyze_window_lines(&self) -> Vec<String> {
        let Some(window) = &self.analyze_window else {
            return Vec::new();
        };
        crate::analyze_window::window_lines(
            window,
            self.locale(),
            self.shell.visible_row_ids(Mode::Analyze),
        )
    }

    pub fn status_lines(&self) -> &[String] {
        &self.status_lines
    }

    fn note_history_opened(&mut self, before: Surface) {
        if self.shell.surface() == Surface::History && before != Surface::History {
            self.refresh_history_from_store();
        }
    }

    /// Replaces the History page with the fixed V1 audit list. Read-only.
    pub fn refresh_history_from_store(&mut self) {
        let listed =
            devsweep_core::history::list_history(devsweep_core::history::HistoryListOptions {
                domain: None,
                limit: 20,
            });
        self.apply_history_list(listed.ok());
    }

    /// `None` means the store could not be read. An empty list is an empty store.
    pub(crate) fn apply_history_list(
        &mut self,
        listed: Option<devsweep_core::history::HistoryListV1>,
    ) {
        let Some(listed) = listed else {
            self.shell
                .set_history_store(HistoryStoreView::Unavailable, Vec::new());
            return;
        };
        let store_failed = listed.stores.iter().any(|store| {
            matches!(
                store.state,
                devsweep_core::history::HistoryStoreState::Unavailable
                    | devsweep_core::history::HistoryStoreState::Unsupported
            )
        });
        if listed.operations.is_empty() {
            let view = if store_failed {
                HistoryStoreView::Unavailable
            } else {
                HistoryStoreView::Empty
            };
            self.shell.set_history_store(view, Vec::new());
            return;
        }
        let lines = listed
            .operations
            .iter()
            .map(|operation| {
                crate::copy::history_operation_line(
                    operation.domain.as_str(),
                    &operation.operation_id,
                    &operation.outcome_code,
                )
            })
            .collect();
        self.shell.set_history_store(HistoryStoreView::Ready, lines);
    }

    pub fn set_row_ids(&mut self, mode: Mode, ids: Vec<String>) {
        self.shell.set_row_ids(mode, ids);
    }

    pub fn begin_clean_screen(&mut self) {
        self.shell.begin_clean_screen();
    }

    pub fn screen_covered(&self) -> bool {
        self.shell.screen_covered()
    }

    pub fn input_locked(&self) -> bool {
        self.shell.input_locked()
    }

    pub fn show_mode(&mut self, mode: Mode) {
        self.shell.show_mode(mode);
        self.ensure_workbench_rows();
    }

    pub fn toggle_menu(&mut self) {
        self.shell.toggle_menu();
    }

    pub fn open_menu_item(&mut self, index: usize) {
        let before = self.shell.surface();
        self.shell.open_menu_item(index);
        self.note_history_opened(before);
    }

    pub fn open_settings(&mut self) {
        self.shell.open_surface(Surface::Settings);
    }

    pub fn cycle_language(&mut self) {
        self.draft.language = Some(match self.draft.language {
            Some(LanguageTag::En) | None => LanguageTag::ZhCn,
            Some(LanguageTag::ZhCn) => LanguageTag::En,
        });
        self.shell.locale = match self.draft.language {
            Some(LanguageTag::ZhCn) => Locale::ZhCn,
            Some(LanguageTag::En) | None => Locale::En,
        };
    }

    /// Draft a theme choice. Saving stays an explicit action.
    pub fn set_theme(&mut self, theme: DesktopTheme) {
        self.draft.theme = theme;
    }

    /// Draft a language choice. Saving stays an explicit action.
    pub fn set_locale(&mut self, locale: Locale) {
        if self.locale() != locale {
            self.cycle_language();
        }
    }

    /// Draft one of the stored text scales (100, 110, 125). Other values are
    /// ignored so the draft never holds a value the store rejects.
    pub fn set_text_scale(&mut self, percent: u8) {
        if matches!(percent, 100 | 110 | 125) {
            self.draft.text_scale_percent = percent;
        }
    }

    /// True when the draft differs from the committed preferences.
    pub fn settings_dirty(&self) -> bool {
        self.draft_desktop_dirty() || self.draft.language != self.draft.committed_language
    }

    pub fn set_scan_roots(&mut self, roots: Vec<String>) {
        self.shell.set_scan_roots(roots);
        self.clean.invalidate_review();
    }

    /// The accepted Clean scan projection owned by this session.
    pub fn clean_workbench(&self) -> &CleanWorkbench {
        &self.clean
    }

    /// Installs an accepted Found report for tests and mirrors its ids.
    #[cfg(test)]
    pub(crate) fn install_clean_rows_for_test(
        &mut self,
        rows: Vec<crate::clean_workbench::CleanRow>,
    ) {
        self.show_mode(Mode::Clean);
        self.clean.install_rows_for_test(rows);
        let ids = self.clean.row_ids().to_vec();
        self.shell.set_row_ids(Mode::Clean, ids);
    }

    /// The landing primary button asks for a scope instead of scanning.
    pub fn clean_open_scope_choice(&mut self) {
        self.clean.open_scope_choice();
    }

    /// Asks for the scope before any scan. Global only arms a confirmation.
    pub fn clean_choose_scope(&mut self, scope: CleanScope) -> bool {
        if self.clean.phase() != CleanPhase::Landing {
            return false;
        }
        match scope {
            CleanScope::Global => {
                self.clean.require_global_confirmation();
                false
            }
            CleanScope::Project => self.request_project_scan(),
        }
    }

    /// Starts the global scan only after the explicit confirmation prompt.
    pub fn clean_confirm_global_scan(&mut self) -> bool {
        if !self.clean.take_global_confirmation() {
            return false;
        }
        self.start_clean_scan(CleanScope::Global, Vec::new())
    }

    /// Stores one explicitly picked project folder as the scan root.
    ///
    /// The folder is remembered through the existing scan-root store when one
    /// is available; otherwise it stays explicit for this session only.
    pub fn clean_pick_scan_root(&mut self, root: PathBuf) -> bool {
        let Ok(validated) =
            devsweep_core::scan_preferences::validate_scan_roots(std::slice::from_ref(&root))
        else {
            return false;
        };
        let Some(picked) = validated.into_iter().next() else {
            return false;
        };
        self.shell
            .set_scan_roots(vec![picked.display().to_string()]);
        let session_only = match self.save_scan_roots() {
            Ok(()) => false,
            Err(StoreError::Unavailable) => true,
            Err(_) => true,
        };
        self.clean.set_root_session_only(session_only);
        self.request_project_scan()
    }

    /// Cancels a landing prompt without starting anything.
    pub fn clean_cancel_prompt(&mut self) {
        self.clean.cancel_prompt();
    }

    /// True only when a scan or an execution accepted cooperative cancellation.
    pub fn clean_request_cancel(&mut self) -> bool {
        let active = matches!(
            self.clean.phase(),
            CleanPhase::Scanning | CleanPhase::Cleaning
        ) || (self.clean_ai.open()
            && matches!(
                self.clean_ai.phase(),
                AiSubviewPhase::Scanning | AiSubviewPhase::Cleaning
            ));
        if !active {
            return false;
        }
        let Some(cancel) = self.clean_cancel.as_ref() else {
            return false;
        };
        cancel.request_cancel();
        self.clean.on_cancel_requested();
        true
    }

    /// Space toggles the focused row. Without a cursor the first visible row
    /// becomes the cursor. Inspect-only, protected, ignored, and sensitive
    /// rows never enter the selection.
    pub fn clean_toggle_focused_row(&mut self) -> bool {
        if self.clean_ai.open() {
            if self.clean_ai.phase() != AiSubviewPhase::Review {
                return false;
            }
            let focused = match self.shell.selection(Mode::Clean) {
                Some(id) => id.to_string(),
                None => match self.shell.visible_row_ids(Mode::Clean).first() {
                    Some(id) => id.clone(),
                    None => return false,
                },
            };
            if self.shell.selection(Mode::Clean).is_none() {
                self.shell.select(Mode::Clean, focused.clone());
            }
            return self.clean_ai.toggle_selected(&focused);
        }
        if self.clean.phase() != CleanPhase::Found {
            return false;
        }
        let focused = match self.shell.selection(Mode::Clean) {
            Some(id) => id.to_string(),
            None => match self.shell.visible_row_ids(Mode::Clean).first() {
                Some(id) => id.clone(),
                None => return false,
            },
        };
        if self.shell.selection(Mode::Clean).is_none() {
            self.shell.select(Mode::Clean, focused.clone());
        }
        self.clean.toggle_selected(&focused)
    }

    /// A row click moves the cursor and toggles the row selection.
    pub fn clean_click_row(&mut self, id: &str) -> bool {
        if self.clean_ai.open() {
            if self.clean_ai.phase() != AiSubviewPhase::Review {
                return false;
            }
            if self.clean_ai.row(id).is_none() {
                return false;
            }
            self.shell.select(Mode::Clean, id.to_string());
            return self.clean_ai.toggle_selected(id);
        }
        if self.clean.phase() != CleanPhase::Found {
            return false;
        }
        if self.clean.row(id).is_none() {
            return false;
        }
        self.shell.select(Mode::Clean, id.to_string());
        self.clean.toggle_selected(id)
    }

    /// Selects every conservatively recommended row in the whole report.
    pub fn clean_select_recommended(&mut self) -> usize {
        if self.clean.phase() != CleanPhase::Found {
            return self.clean.selected_count();
        }
        self.clean.select_recommended()
    }

    pub fn clean_clear_selected(&mut self) {
        self.clean.clear_selected();
    }

    /// The AI review subview owned by this session.
    pub fn clean_ai(&self) -> &CleanAiView {
        &self.clean_ai
    }

    /// Opens the AI review subview from the Clean surface. Opening never scans.
    pub fn clean_ai_open(&mut self) -> bool {
        if self.shell.operation_id().is_some() {
            return false;
        }
        if !self.clean_ai.open_subview() {
            return false;
        }
        self.reload_clean_ai_rows();
        true
    }

    /// Closes the AI review subview and restores the Clean row projection.
    pub fn clean_ai_close(&mut self) -> bool {
        if self.shell.operation_id().is_some() {
            return false;
        }
        if !self.clean_ai.close_subview() {
            return false;
        }
        self.reset_clean_row_view();
        true
    }

    /// Escape on the Clean surface: leave the AI subview first.
    pub fn clean_back(&mut self) -> bool {
        self.clean_ai_close()
    }

    /// Cycles the closed provider set the next explicit scan will use.
    pub fn clean_ai_cycle_provider(&mut self) -> bool {
        if !self.clean_ai.open() {
            return false;
        }
        self.clean_ai.cycle_provider();
        true
    }

    /// Switches the active AI section and reloads the bounded window.
    pub fn clean_ai_set_category(&mut self, category: AiCategory) -> bool {
        if !self.clean_ai.open() {
            return false;
        }
        if !self.clean_ai.set_category(category) {
            return false;
        }
        self.reload_clean_ai_rows();
        true
    }

    /// Scans only an explicit temporary home. Unset and refused overrides do
    /// not fall back to `USERPROFILE`, `.codex`, `.claude`, or Cursor.
    pub fn clean_ai_scan(&mut self) -> bool {
        let home = match crate::clean::explicit_ai_review_home() {
            crate::clean::ExplicitAiHome::Isolated(home) => home,
            crate::clean::ExplicitAiHome::Refused(reason) => {
                self.clean_ai
                    .on_scan_unavailable(&format!("explicit AI review home refused: {reason:?}"));
                return false;
            }
            crate::clean::ExplicitAiHome::Unset => {
                self.clean_ai
                    .on_scan_unavailable(crate::copy::clean_ai_isolated_home_required(
                        self.locale(),
                    ));
                return false;
            }
        };
        self.clean_ai_scan_home(home)
    }

    /// Scans one explicit temporary AI home on a worker. A real profile is
    /// refused before any provider directory is opened.
    pub fn clean_ai_scan_home(&mut self, home: PathBuf) -> bool {
        if !matches!(
            crate::clean::classify_explicit_ai_review_home(
                Some(home.as_os_str()),
                &std::env::temp_dir(),
            ),
            crate::clean::ExplicitAiHome::Isolated(_)
        ) {
            self.clean_ai
                .on_scan_unavailable(crate::copy::clean_ai_isolated_home_required(self.locale()));
            return false;
        }
        if self.shell.operation_id().is_some() || !self.clean_ai.open() {
            return false;
        }
        let provider = self.clean_ai.provider();
        let id = self.shell.begin_operation(OperationClass::ReadOnly);
        self.clean_ai.on_scan_started(home.clone());
        self.reload_clean_ai_rows();
        let cancel = Arc::new(FlagCancelObserver::new());
        self.clean_cancel = Some(Arc::clone(&cancel));
        let mailbox = Arc::clone(&self.mailbox);
        dispatch_blocking(mailbox, move || {
            let payload = match crate::clean::ai_scan_home(provider, &home, cancel.as_ref()) {
                Ok(observation) => CleanAiScanPayload {
                    observation: Some(observation),
                    error: None,
                },
                Err(error) => CleanAiScanPayload {
                    observation: None,
                    error: Some(error),
                },
            };
            MailEvent::CleanAiScan { id, payload }
        });
        true
    }

    /// Computes the live AI preview digest over the active selection.
    pub fn clean_ai_preview(&mut self) -> bool {
        if !self.clean_ai.open() {
            return false;
        }
        self.clean_ai.preview_selection()
    }

    /// Clears the active AI category's selection. Other categories keep theirs.
    pub fn clean_ai_clear_selected(&mut self) -> bool {
        if !self.clean_ai.open() {
            return false;
        }
        self.clean_ai.clear_selected();
        true
    }

    /// Executes the confirmed AI plan. The gate refuses a missing or stale
    /// digest before any worker starts.
    pub fn clean_ai_confirm_execute(&mut self) -> bool {
        if self.shell.operation_id().is_some() {
            return false;
        }
        let Some(start) = self.clean_ai.begin_execute() else {
            return false;
        };
        let id = self.shell.begin_operation(OperationClass::Mutating);
        let cancel = Arc::new(FlagCancelObserver::new());
        self.clean_cancel = Some(Arc::clone(&cancel));
        let mailbox = Arc::clone(&self.mailbox);
        dispatch_blocking(mailbox, move || {
            let payload = match crate::clean::ai_execute_production(
                &start.plan,
                &start.digest,
                true,
                cancel.as_ref(),
            ) {
                Ok(report) => CleanAiExecutePayload {
                    report: Some(report),
                    error: None,
                    canceled: cancel.is_cancel_requested(),
                },
                Err(error) => CleanAiExecutePayload {
                    report: None,
                    error: Some(ai_failure(error)),
                    canceled: cancel.is_cancel_requested(),
                },
            };
            MailEvent::CleanAiExecute { id, payload }
        });
        true
    }

    /// Lists recovery bundles under the scanned home. Listing never restores.
    pub fn clean_ai_refresh_recovery(&mut self) -> bool {
        if self.shell.operation_id().is_some() || !self.clean_ai.open() {
            return false;
        }
        let Some(home) = self.clean_ai.home().map(PathBuf::from) else {
            return false;
        };
        let id = self.shell.begin_operation(OperationClass::ReadOnly);
        let mailbox = Arc::clone(&self.mailbox);
        dispatch_blocking(mailbox, move || {
            let payload = match crate::clean::ai_recovery_list(&home) {
                Ok(list) => CleanAiRecoveryPayload {
                    list: Some(list),
                    error: None,
                },
                Err(error) => CleanAiRecoveryPayload {
                    list: None,
                    error: Some(ai_failure(error)),
                },
            };
            MailEvent::CleanAiRecovery { id, payload }
        });
        true
    }

    /// Builds the restore plan for one recovery bundle. The plan copies no
    /// bytes and is never executed from here.
    pub fn clean_ai_restore_plan(&mut self, bundle_id: impl Into<String>) -> bool {
        if self.shell.operation_id().is_some() || !self.clean_ai.open() {
            return false;
        }
        let Some(home) = self.clean_ai.home().map(PathBuf::from) else {
            return false;
        };
        let bundle_id = bundle_id.into();
        let id = self.shell.begin_operation(OperationClass::ReadOnly);
        let request_id = bundle_id.clone();
        let mailbox = Arc::clone(&self.mailbox);
        dispatch_blocking(mailbox, move || {
            let payload = match crate::clean::ai_restore_plan(&home, &request_id) {
                Ok(plan) => CleanAiRestorePayload {
                    bundle_id: request_id.clone(),
                    plan: Some(plan),
                    error: None,
                },
                Err(error) => CleanAiRestorePayload {
                    bundle_id: request_id.clone(),
                    plan: None,
                    error: Some(ai_failure(error)),
                },
            };
            MailEvent::CleanAiRestore { id, payload }
        });
        true
    }

    /// Installs the AI subview's bounded row projection for the current row
    /// query. The core row id embeds the C15 class name (`regenerable_cache`
    /// contains `regen`), so filtering raw ids would match every row of a class;
    /// the projection filters the user-visible relative path instead and hands
    /// the surviving ids to the shell table.
    fn reload_clean_ai_rows(&mut self) {
        let query = self.shell.row_query(Mode::Clean).to_string();
        let ids = self
            .clean_ai
            .active_row_ids()
            .iter()
            .filter(|id| self.clean_ai.matches_query(id, &query))
            .cloned()
            .collect::<Vec<_>>();
        self.shell.set_row_ids(Mode::Clean, ids);
        self.shell.set_row_window_start(Mode::Clean, 0);
        self.shell.clear_row_selection(Mode::Clean);
    }

    fn clean_ai_enter(&mut self) -> bool {
        match self.clean_ai.phase() {
            AiSubviewPhase::Landing | AiSubviewPhase::Failed => self.clean_ai_scan(),
            AiSubviewPhase::Review => {
                if self.clean_ai.preview_matches_current() {
                    self.clean_ai_confirm_execute()
                } else {
                    self.clean_ai_preview()
                }
            }
            AiSubviewPhase::Scanning | AiSubviewPhase::Cleaning | AiSubviewPhase::Completed => {
                false
            }
        }
    }

    /// Wires the Found search input to the existing row query projection. The AI
    /// subview re-projects its rows through the model's path search key.
    pub fn clean_set_search(&mut self, query: impl Into<String>) -> bool {
        let query = query.into();
        if self.shell.row_query(Mode::Clean) == query {
            return false;
        }
        self.shell.set_row_query(Mode::Clean, query);
        if self.clean_ai.open() {
            self.reload_clean_ai_rows();
        }
        true
    }

    /// Cycles report/name/size order and refreshes the bounded window.
    pub fn clean_cycle_sort(&mut self) -> bool {
        if self.clean.phase() != CleanPhase::Found {
            return false;
        }
        self.clean.cycle_sort();
        self.refresh_clean_row_ids();
        true
    }

    /// Scrolls the bounded row window by whole rows over the full id set.
    pub fn clean_scroll_rows(&mut self, delta: isize) -> bool {
        let reviewable = self.clean.phase() == CleanPhase::Found
            || (self.clean_ai.open() && self.clean_ai.phase() == AiSubviewPhase::Review);
        if !reviewable {
            return false;
        }
        let start = self.shell.row_window_start(Mode::Clean);
        let len = self.shell.window_row_count(Mode::Clean);
        let next = usize::try_from((start as isize + delta).max(0)).unwrap_or(0);
        let next = next.min(len);
        if next == start {
            return false;
        }
        self.shell.set_row_window_start(Mode::Clean, next);
        true
    }

    /// Computes the live preview digest over the selected saved plan.
    pub fn clean_preview_selection(&mut self) -> bool {
        if self.clean.phase() != CleanPhase::Found {
            return false;
        }
        self.clean.preview_selection()
    }

    /// Executes only when the stored preview still matches the selection.
    pub fn clean_confirm_execute(&mut self) -> bool {
        if self.shell.operation_id().is_some() {
            return false;
        }
        let Some(start) = self.clean.begin_execute() else {
            return false;
        };
        self.spawn_clean_execute(start)
    }

    /// Retries the accepted scope. A failed global scan asks again first.
    pub fn clean_rescan(&mut self) -> bool {
        match self.clean.last_scope() {
            Some(CleanScope::Project) => self.request_project_scan(),
            Some(CleanScope::Global) => {
                self.clean.reset_to_landing();
                self.clean.require_global_confirmation();
                false
            }
            None => false,
        }
    }

    /// Loads Clean rows from the saved project scan roots with the shared
    /// report pipeline. Empty roots ask for the first-use root setup instead.
    pub fn request_project_rows(&mut self) -> bool {
        self.request_project_scan()
    }

    fn request_project_scan(&mut self) -> bool {
        if self.shell.operation_id().is_some() {
            return false;
        }
        let roots: Vec<PathBuf> = self
            .shell
            .scan_roots()
            .iter()
            .filter(|root| !root.is_empty())
            .map(PathBuf::from)
            .collect();
        if roots.is_empty() {
            if self.clean.phase() == CleanPhase::Landing {
                self.clean
                    .require_root(devsweep_core::scan_preferences::scan_root_suggestions());
            }
            return false;
        }
        self.start_clean_scan(CleanScope::Project, roots)
    }

    fn start_clean_scan(&mut self, scope: CleanScope, roots: Vec<PathBuf>) -> bool {
        if self.shell.operation_id().is_some() {
            return false;
        }
        let options = ScanOptions {
            include_projects: matches!(scope, CleanScope::Project),
            include_global: matches!(scope, CleanScope::Global),
            roots,
        };
        let id = self.shell.begin_operation(OperationClass::ReadOnly);
        self.clean.on_scan_started(scope);
        self.reset_clean_row_view();
        let cancel = Arc::new(FlagCancelObserver::new());
        self.clean_cancel = Some(Arc::clone(&cancel));
        let progress = Arc::new(AtomicUsize::new(0));
        self.clean_progress = Some(Arc::clone(&progress));
        self.clean_progress_seen = 0;
        let mailbox = Arc::clone(&self.mailbox);
        dispatch_blocking(mailbox, move || {
            let payload = scan_clean_rows(&options, &cancel, &mut |found| {
                progress.store(found, Ordering::SeqCst);
            });
            MailEvent::CleanScan { id, payload }
        });
        true
    }

    fn spawn_clean_execute(&mut self, start: CleanExecutionStart) -> bool {
        let id = self.shell.begin_operation(OperationClass::Mutating);
        let cancel = Arc::new(FlagCancelObserver::new());
        self.clean_cancel = Some(Arc::clone(&cancel));
        let mailbox = Arc::clone(&self.mailbox);
        dispatch_blocking(mailbox, move || {
            let payload =
                execute_clean_plan(&start.saved_plan, start.selected, &start.digest, &cancel);
            MailEvent::CleanExecute { id, payload }
        });
        true
    }

    fn clean_enter(&mut self) -> bool {
        match self.clean.phase() {
            CleanPhase::Landing => match self.clean.prompt() {
                crate::clean_workbench::CleanLandingPrompt::None => {
                    self.clean.open_scope_choice();
                    true
                }
                // The keyboard default is the project scope; the global scan
                // remains an explicit, separately confirmed action.
                crate::clean_workbench::CleanLandingPrompt::ScopeChoice => {
                    let _ = self.clean_choose_scope(CleanScope::Project);
                    true
                }
                crate::clean_workbench::CleanLandingPrompt::RootRequired => false,
                crate::clean_workbench::CleanLandingPrompt::ConfirmGlobal => {
                    self.clean_confirm_global_scan()
                }
            },
            CleanPhase::Found => {
                if self.clean.preview_matches_current() {
                    self.clean_confirm_execute()
                } else {
                    self.clean_preview_selection()
                }
            }
            CleanPhase::Failed => self.clean_rescan(),
            CleanPhase::Scanning
            | CleanPhase::AllClean
            | CleanPhase::Cleaning
            | CleanPhase::Completed => false,
        }
    }

    fn reset_clean_row_view(&mut self) {
        let ids = self.clean.row_ids().to_vec();
        self.shell.set_row_query(Mode::Clean, "");
        self.shell.set_row_ids(Mode::Clean, ids);
        self.shell.set_row_window_start(Mode::Clean, 0);
        self.shell.clear_row_selection(Mode::Clean);
    }

    fn refresh_clean_row_ids(&mut self) {
        let ids = self.clean.row_ids().to_vec();
        self.shell.set_row_ids(Mode::Clean, ids);
        self.shell.set_row_window_start(Mode::Clean, 0);
    }

    pub fn drain_mail(&mut self) -> Poll {
        let progress_applied = self.sync_clean_progress();
        let batch = self.mailbox.take_batch(32);
        for event in &batch {
            if !self.apply_clean_mail(event) {
                self.shell.apply_mail(event);
            }
        }
        Poll {
            applied: batch.len() + usize::from(progress_applied),
            decorative_frame: self.shell.decorative_deadline().is_some(),
        }
    }

    /// Samples the out-of-band scan progress into the workbench. Reports
    /// whether the observed discovery count changed.
    fn sync_clean_progress(&mut self) -> bool {
        let Some(progress) = self.clean_progress.as_ref() else {
            return false;
        };
        let found = progress.load(Ordering::SeqCst);
        if found == self.clean_progress_seen {
            return false;
        }
        self.clean_progress_seen = found;
        self.clean.on_scan_progress(found);
        true
    }

    fn apply_clean_mail(&mut self, event: &MailEvent) -> bool {
        match event {
            MailEvent::CleanScan { id, payload } => {
                if self.shell.operation_id() == Some(*id) {
                    self.clean.on_scan_result(payload.clone());
                    self.clean_cancel = None;
                    self.clean_progress = None;
                    self.clean_progress_seen = 0;
                    self.shell.finish_operation(*id);
                    self.reset_clean_row_view();
                } else {
                    self.shell.reject_result();
                }
                true
            }
            MailEvent::CleanExecute { id, payload } => {
                if self.shell.operation_id() == Some(*id) {
                    self.clean.on_execute_result(payload.clone());
                    if let Some(entry) = self.clean.history_entry(self.locale()) {
                        self.shell.push_history(entry);
                    }
                    self.clean_cancel = None;
                    self.shell.finish_operation(*id);
                } else {
                    self.shell.reject_result();
                }
                true
            }
            MailEvent::CleanAiScan { id, payload } => {
                if self.shell.operation_id() == Some(*id) {
                    self.clean_ai.on_scan_result(payload.clone());
                    self.clean_cancel = None;
                    self.shell.finish_operation(*id);
                    self.reload_clean_ai_rows();
                } else {
                    self.shell.reject_result();
                }
                true
            }
            MailEvent::CleanAiExecute { id, payload } => {
                if self.shell.operation_id() == Some(*id) {
                    self.clean_ai.on_execute_result(payload.clone());
                    if let Some(entry) = self.clean_ai.history_entry(self.locale()) {
                        self.shell.push_history(entry);
                    }
                    self.clean_cancel = None;
                    self.shell.finish_operation(*id);
                } else {
                    self.shell.reject_result();
                }
                true
            }
            MailEvent::CleanAiRecovery { id, payload } => {
                if self.shell.operation_id() == Some(*id) {
                    self.clean_ai.on_recovery_result(payload.clone());
                    self.shell.finish_operation(*id);
                } else {
                    self.shell.reject_result();
                }
                true
            }
            MailEvent::CleanAiRestore { id, payload } => {
                if self.shell.operation_id() == Some(*id) {
                    self.clean_ai.on_restore_result(payload.clone());
                    self.shell.finish_operation(*id);
                } else {
                    self.shell.reject_result();
                }
                true
            }
            MailEvent::Completion { .. } | MailEvent::Lifecycle { .. } => false,
        }
    }

    /// Applies an external protection-list replacement and re-derives the
    /// read-only row projection from it.
    pub fn set_protections(&mut self, paths: Vec<String>) {
        self.shell.set_protections(paths.clone());
        self.clean.set_protected_paths(paths);
        self.clean.invalidate_review();
    }

    /// True when the latest protection edit could not be written to the store.
    pub fn protection_session_only(&self) -> bool {
        self.protection_session_only
    }

    /// Toggles one row's cleanup protection through the existing protection
    /// list and its store. The row stays visible and becomes read-only, and
    /// the reviewed digest is invalidated. Returns the new protected state.
    pub fn clean_toggle_row_protection(&mut self, id: &str) -> bool {
        if self.shell.operation_id().is_some() {
            return false;
        }
        let Some(row) = self.clean.row(id) else {
            return false;
        };
        let Some(path) = row.path.clone() else {
            return false;
        };
        let mut paths = self.shell.protections().to_vec();
        let now_protected = match paths.iter().position(|entry| entry == &path) {
            Some(index) => {
                paths.remove(index);
                false
            }
            None => {
                paths.push(path);
                true
            }
        };
        self.shell.replace_protections(paths.clone());
        self.clean.set_protected_paths(paths);
        // Persist through the existing protections store when one is
        // available; otherwise the edit stays active for this session only.
        self.protection_session_only = self.save_protections().is_err();
        now_protected
    }

    /// The focused Clean row id, if any cursor exists.
    pub fn clean_focused_row_id(&self) -> Option<String> {
        self.shell.selection(Mode::Clean).map(str::to_string)
    }

    /// Toggles one row's ignore state. The row stays visible as inspect-only
    /// and leaves the selection and the recommendation. Returns the new state.
    pub fn clean_toggle_row_ignored(&mut self, id: &str) -> bool {
        self.clean.toggle_ignored(id).unwrap_or_default()
    }

    /// Write dirty appearance and language fields. An unchanged draft does not migrate.
    pub fn save_settings(&mut self) -> Result<bool, StoreError> {
        let Some(files) = self.files.clone() else {
            return Err(StoreError::Unavailable);
        };
        if self.desktop_blocked && self.draft_desktop_dirty() {
            return Err(StoreError::Blocked);
        }
        if self.language_blocked && self.draft.language != self.draft.committed_language {
            return Err(StoreError::Blocked);
        }
        let mut wrote = false;
        if self.draft.theme != self.draft.committed.theme {
            let saved = preferences::save_desktop_patch(
                &files,
                DesktopPatch::Theme {
                    value: self.draft.theme,
                },
            )?;
            self.draft.committed = saved;
            wrote = true;
        }
        if self.draft.text_scale_percent != self.draft.committed.text_scale_percent {
            let saved = preferences::save_desktop_patch(
                &files,
                DesktopPatch::TextScalePercent {
                    value: self.draft.text_scale_percent,
                },
            )?;
            self.draft.committed = saved;
            wrote = true;
        }
        if self.draft.motion != self.draft.committed.motion {
            let saved = preferences::save_desktop_patch(
                &files,
                DesktopPatch::Motion {
                    value: self.draft.motion,
                },
            )?;
            self.draft.committed = saved;
            self.shell.motion = self.draft.committed.motion;
            wrote = true;
        }
        if self.draft.language != self.draft.committed_language {
            let Some(language) = self.draft.language else {
                return Ok(wrote);
            };
            preferences::save_language(&files, language)?;
            self.draft.committed_language = Some(language);
            wrote = true;
        }
        Ok(wrote)
    }

    pub fn save_scan_roots(&mut self) -> Result<(), StoreError> {
        let Some(files) = self.files.clone() else {
            return Err(StoreError::Unavailable);
        };
        if self.scan_blocked {
            return Err(StoreError::Blocked);
        }
        let roots = preferences::save_scan_roots(&files, self.shell.scan_roots())?;
        self.shell.scan_roots = roots;
        Ok(())
    }

    pub fn save_protections(&mut self) -> Result<(), StoreError> {
        let Some(files) = self.files.clone() else {
            return Err(StoreError::Unavailable);
        };
        if self.protection_blocked {
            return Err(StoreError::Blocked);
        }
        let paths = preferences::save_protections(&files, self.shell.protections())?;
        self.shell.protections = paths;
        Ok(())
    }

    fn draft_desktop_dirty(&self) -> bool {
        self.draft.theme != self.draft.committed.theme
            || self.draft.text_scale_percent != self.draft.committed.text_scale_percent
            || self.draft.motion != self.draft.committed.motion
    }
}

/// Keeps every listed drive and appends an explicit folder that is not already there.
fn merge_analyze_roots(
    mut drives: Vec<AnalyzeRootChoice>,
    extras: Vec<AnalyzeRootChoice>,
) -> Vec<AnalyzeRootChoice> {
    for choice in extras {
        let key = analyze_choice_key(&choice);
        if key.is_empty() {
            continue;
        }
        let present = drives
            .iter()
            .any(|existing| analyze_choice_key(existing) == key);
        if !present {
            drives.push(choice);
        }
    }
    drives
}

fn analyze_choice_key(choice: &AnalyzeRootChoice) -> String {
    choice
        .normalized
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| choice.input.clone())
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        sync::{Arc, mpsc},
        thread,
        time::{Duration, Instant},
    };

    use super::*;

    #[test]
    fn analyze_up_returns_to_the_parent_folder_without_rescanning() {
        let root =
            std::env::temp_dir().join(format!("devsweep-gpui-analyze-up-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("inner")).expect("inner");
        std::fs::write(root.join("inner").join("keep.bin"), b"keep").expect("file");

        let mut session = Session::unbacked();
        session.show_mode(Mode::Analyze);
        assert!(session.analyze_open_isolated_root(root.clone()));
        assert!(!session.analyze_can_go_up(), "the scan root has no parent");
        let inner = session
            .analyze_window()
            .expect("window")
            .snapshot()
            .nodes
            .iter()
            .find(|node| node.name == "inner")
            .map(|node| node.id)
            .expect("inner node");
        session.activate_mode_row(Mode::Analyze, inner.to_string());
        assert_eq!(session.analyze_window().expect("window").parent_id(), inner);
        assert!(session.analyze_can_go_up());

        let snapshot_before = session.analyze_window().expect("window").snapshot().clone();
        assert!(session.analyze_go_up());
        let window = session.analyze_window().expect("window");
        assert_ne!(window.parent_id(), inner);
        assert_eq!(window.snapshot(), &snapshot_before, "Up must not rescan");
        assert_eq!(window.board().list_selection(), Some(inner));
        assert!(!session.analyze_can_go_up());
        assert!(!session.analyze_go_up());
        assert!(root.join("inner").join("keep.bin").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn window_jobs_queue_slow_reads_and_drop_superseded_results() {
        let mut session = Session::unbacked();
        session.enable_background_jobs();

        // Status: Enter queues one snapshot job instead of sampling inline.
        session.show_mode(Mode::Status);
        session.apply_key(NavKey::Enter);
        assert!(session.status_sampling());
        assert!(session.status_snapshot().is_none());
        let jobs = session.take_jobs();
        assert_eq!(jobs.len(), 1);
        let BackgroundJob::StatusSnapshot { generation } = jobs[0] else {
            panic!("expected a status job");
        };
        // A second Enter while sampling does not queue a duplicate collector.
        session.apply_key(NavKey::Enter);
        assert!(session.take_jobs().is_empty());
        // A stale generation is dropped; the live one applies.
        assert!(!session.apply_job(JobResult::StatusSnapshot {
            generation: generation + 99,
            result: Err("stale".to_string()),
        }));
        assert!(session.status_sampling());
        assert!(session.apply_job(JobResult::StatusSnapshot {
            generation,
            result: Err("snapshot_unavailable".to_string()),
        }));
        assert!(!session.status_sampling());
        assert!(
            session
                .status_lines()
                .iter()
                .any(|line| line.contains("snapshot_unavailable"))
        );

        // Analyze: a listed root queues a cancellable walk; Escape cancels it
        // and a late result installs nothing.
        let root = std::env::temp_dir();
        session.show_mode(Mode::Analyze);
        session.offer_analyze_roots(std::slice::from_ref(&root));
        assert!(session.analyze_open_listed(0));
        assert!(session.analyze_window().is_none());
        // The listed root is stored in normalized form: lowercase, forward
        // slashes, and no trailing separator. `temp_dir()` keeps the raw
        // Windows spelling, including its trailing separator.
        let expected = session.analyze_roots[0]
            .normalized
            .clone()
            .expect("normalized temp root");
        assert_eq!(
            session.analyze_scanning(),
            Some(std::path::Path::new(&expected))
        );
        let jobs = session.take_jobs();
        let Some(BackgroundJob::AnalyzeScan {
            generation, cancel, ..
        }) = jobs.into_iter().next()
        else {
            panic!("expected an analyze job");
        };
        session.apply_key(NavKey::Escape);
        assert!(session.analyze_scanning().is_none());
        assert!(devsweep_core::process::CancelObserver::is_cancel_requested(
            cancel.as_ref()
        ));
        assert!(!session.apply_job(JobResult::AnalyzeScan {
            generation,
            result: Err("late".to_string()),
        }));
        assert!(session.analyze_window().is_none());
    }

    #[test]
    fn window_software_segments_load_from_the_host_off_thread() {
        if crate::software_review::SoftwareReview::has_fixture(
            crate::workbench::SoftwareSegment::Startup,
        ) {
            return;
        }
        let mut session = Session::unbacked();
        session.enable_background_jobs();
        session.show_mode(Mode::Software);
        session.software_open_segment(crate::workbench::SoftwareSegment::Startup);
        assert!(session.software_review().loading());
        assert!(session.background_busy());
        let jobs = session.take_jobs();
        let Some(BackgroundJob::SoftwareHost {
            generation,
            segment,
        }) = jobs.into_iter().next()
        else {
            panic!("expected a software job");
        };
        assert_eq!(segment, crate::workbench::SoftwareSegment::Startup);
        let list = devsweep_core::software::SoftwareStartupListV1 {
            version: 1,
            observed_at_unix_ms: 0,
            sources: Vec::new(),
            entries: Vec::new(),
        };
        assert!(session.apply_job(JobResult::SoftwareHost {
            generation,
            load: Some(crate::software_review::SoftwareHostLoad::Startup(list)),
        }));
        assert!(!session.software_review().loading());
        assert!(!session.background_busy());
        assert!(session.software_review().host_backed());
    }
    use crate::clean::{cleanup, isolated_pytest_project};
    use crate::clean_workbench::{
        CleanSort, cleanup as workbench_cleanup, isolated_multi_cache_project,
    };

    #[test]
    fn three_downs_from_the_open_menu_highlight_history() {
        let mut session = Session::unbacked();
        session.toggle_menu();
        assert!(session.shell().menu_open());
        assert_eq!(session.shell().menu_index(), 0);
        assert_eq!(MENU[0], Surface::Settings);
        session.apply_key(NavKey::Down);
        session.apply_key(NavKey::Down);
        session.apply_key(NavKey::Down);
        assert!(session.shell().menu_open());
        assert_eq!(session.shell().menu_index(), 3);
        assert_eq!(MENU[3], Surface::History);
    }

    #[test]
    fn history_page_shows_store_rows_and_not_an_unread_empty_claim() {
        use devsweep_core::history::{
            HistoryDomain, HistoryListV1, HistoryOperationSummaryV1, HistoryStoreState,
            HistoryStoreStatusV1,
        };

        let mut session = Session::unbacked();
        session.shell.open_surface(Surface::History);
        session.apply_history_list(Some(HistoryListV1 {
            operations: vec![HistoryOperationSummaryV1 {
                domain: HistoryDomain::Clean,
                operation_id: "op-9".to_string(),
                first_timestamp_unix_ms: 1,
                last_timestamp_unix_ms: 2,
                outcome_code: "succeeded".to_string(),
                stable_codes: Vec::new(),
                partial: false,
                unknown: false,
                unsupported: false,
            }],
            stores: vec![HistoryStoreStatusV1 {
                domain: HistoryDomain::Clean,
                state: HistoryStoreState::Available,
                reason_code: None,
            }],
        }));
        let lines = crate::view::panel_lines(&session, None);
        assert!(
            lines.iter().any(|line| line == "clean | op-9 | succeeded"),
            "store row missing: {lines:?}"
        );
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("No audit operations")),
            "a loaded store must not claim it is empty: {lines:?}"
        );

        session.apply_history_list(Some(HistoryListV1 {
            operations: Vec::new(),
            stores: vec![HistoryStoreStatusV1 {
                domain: HistoryDomain::Clean,
                state: HistoryStoreState::Unavailable,
                reason_code: Some("io".to_string()),
            }],
        }));
        let lines = crate::view::panel_lines(&session, None);
        assert!(lines.iter().any(|line| line.contains("could not be read")));
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("No audit operations"))
        );
    }

    /// Waits for the worker terminal event, applies it, and returns the applied
    /// count. Progress events may arrive before the terminal payload.
    fn drain_until_idle(session: &mut Session) -> usize {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut applied = 0;
        while session.shell().activity() == Activity::Busy {
            if session.mailbox.len() > 0 {
                applied += session.drain_mail().applied;
            }
            assert!(
                Instant::now() < deadline,
                "worker did not finish within 10 seconds"
            );
            thread::sleep(Duration::from_millis(5));
        }
        applied
    }

    #[test]
    fn c10_ac1_route_back_preserves_state_and_stale_results_do_not_apply() {
        let mut shell = ShellState::new();
        let mut seen = vec![shell.mode()];
        while shell.mode() != Mode::Status {
            shell.apply_key(NavKey::Right);
            seen.push(shell.mode());
        }
        assert_eq!(
            seen,
            vec![
                Mode::Clean,
                Mode::Software,
                Mode::Optimize,
                Mode::Analyze,
                Mode::Status
            ]
        );
        while shell.mode() != Mode::Clean {
            shell.apply_key(NavKey::Left);
        }
        shell.apply_key(NavKey::Left);
        assert_eq!(shell.mode(), Mode::Clean);

        shell.toggle_menu();
        shell.apply_key(NavKey::Right);
        assert_eq!(shell.mode(), Mode::Clean);
        shell.apply_key(NavKey::Down);
        shell.apply_key(NavKey::Enter);
        assert_eq!(shell.surface(), Surface::Protection);
        assert!(!shell.menu_open());
        shell.apply_key(NavKey::Escape);
        assert_eq!(shell.surface(), Surface::Mode);

        shell.select(Mode::Clean, "cache");
        shell.select(Mode::Software, "app");
        let id = shell.begin_operation(OperationClass::ReadOnly);
        assert!(shell.apply_completion(id, "clean-report"));
        shell.open_surface(Surface::Settings);
        shell.apply_key(NavKey::Escape);
        shell.open_surface(Surface::History);
        shell.select_history("op-1");
        shell.show_mode(Mode::Analyze);
        assert_eq!(shell.selection(Mode::Clean), Some("cache"));
        assert_eq!(shell.report(Mode::Clean), Some("clean-report"));
        assert_eq!(shell.selection(Mode::Software), Some("app"));
        assert_eq!(shell.review(Mode::Clean), Review::Valid);
        shell.open_surface(Surface::History);
        assert_eq!(shell.history_selection(), Some("op-1"));
        assert_eq!(shell.report(Mode::Clean), Some("clean-report"));

        shell.show_mode(Mode::Clean);
        shell.set_scan_roots(vec!["D:\\work".to_string()]);
        assert_eq!(shell.report(Mode::Clean), Some("clean-report"));
        assert_eq!(shell.selection(Mode::Clean), None);
        assert_eq!(shell.review(Mode::Clean), Review::StaleScanRoots);
        assert_eq!(shell.selection(Mode::Software), Some("app"));
        shell.set_protections(vec!["D:\\keep".to_string()]);
        assert_eq!(shell.review(Mode::Clean), Review::StaleProtection);
        assert_eq!(shell.report(Mode::Clean), Some("clean-report"));

        let canceled = shell.begin_operation(OperationClass::ReadOnly);
        assert!(shell.cancel(canceled));
        assert!(!shell.apply_completion(canceled, "late"));
        assert_eq!(shell.report(Mode::Clean), Some("clean-report"));

        let replaced = shell.begin_operation(OperationClass::ReadOnly);
        let current = shell.begin_operation(OperationClass::ReadOnly);
        assert!(!shell.apply_completion(replaced, "old"));
        assert_eq!(shell.report(Mode::Clean), Some("clean-report"));
        assert!(shell.apply_completion(current, "fresh"));
        assert_eq!(shell.report(Mode::Clean), Some("fresh"));
        assert_eq!(shell.review(Mode::Clean), Review::Valid);

        let read = shell.begin_operation(OperationClass::ReadOnly);
        assert_eq!(
            shell.request_close(),
            CloseAction::CancelAndJoin { id: read }
        );
        assert!(!shell.apply_completion(read, "after-close"));
        assert_eq!(shell.report(Mode::Clean), Some("fresh"));
        assert!(shell.settle(read));
        assert!(!shell.apply_completion(read, "after-join"));

        shell.show_mode(Mode::Optimize);
        let mutating = shell.begin_operation(OperationClass::Mutating);
        assert!(!shell.cancel(mutating));
        assert_eq!(
            shell.request_close(),
            CloseAction::WaitOnly { id: mutating }
        );
        assert!(!shell.settle(mutating));
        assert!(shell.apply_completion(mutating, "finished"));
        assert_eq!(shell.report(Mode::Optimize), Some("finished"));
        assert_eq!(shell.report(Mode::Clean), Some("fresh"));
    }

    #[test]
    fn mode_change_does_not_cancel_mutating_operation() {
        let mut shell = ShellState::new();
        assert_eq!(shell.mode(), Mode::Clean);
        let id = shell.begin_operation(OperationClass::Mutating);
        shell.apply_key(NavKey::Right);
        assert_eq!(shell.mode(), Mode::Software);
        shell.show_mode(Mode::Optimize);
        assert_eq!(shell.mode(), Mode::Optimize);
        assert!(!shell.cancel(id));
        assert_eq!(shell.request_close(), CloseAction::WaitOnly { id });
        assert!(shell.apply_completion(id, "still-current"));
        assert_eq!(shell.report(Mode::Clean), Some("still-current"));
        assert_eq!(shell.report(Mode::Software), None);
        assert_eq!(shell.report(Mode::Optimize), None);
    }

    #[test]
    fn first_launch_does_not_scan() {
        let main = crate::source_lf(include_str!("main.rs"));
        let view = crate::source_lf(include_str!("view.rs"));
        let sweeper = format!("{}{}", "Sweep", "er");
        let spawn_live = format!("{}{}", "spawn_", "live");
        let full_scan_report = format!("{}{}", "full_scan_", "report");
        for source in [main, view] {
            assert!(
                !source.contains(&sweeper),
                "launch path must not construct a sweeper"
            );
            assert!(
                !source.contains(&spawn_live),
                "launch path must not spawn live work"
            );
            assert!(
                !source.contains(&full_scan_report),
                "launch path must not request a full scan report"
            );
        }

        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-first-launch",
            std::process::id()
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing a preference fixture outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("settings")).unwrap();
        let files = PreferenceFiles::new(
            root.join("settings"),
            root.join("protect").join("protected-paths.json"),
        );
        let mut session = Session::launch(files);
        assert!(session.shell.scan_roots().is_empty());
        assert_eq!(session.shell.activity(), Activity::Idle);
        assert_eq!(session.shell.request_close(), CloseAction::Quit);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_scan_roots_do_not_request_project_rows() {
        let mut session = Session::unbacked();
        assert!(session.shell.scan_roots().is_empty());
        assert!(!session.request_project_rows());
        assert_eq!(session.shell.activity(), Activity::Idle);
        assert!(session.shell.visible_row_ids(Mode::Clean).is_empty());
    }

    #[test]
    fn isolated_project_rows_apply_after_drain() {
        let root = isolated_pytest_project();
        let name = root
            .file_name()
            .and_then(|value| value.to_str())
            .expect("utf-8 fixture name");
        assert!(
            name.starts_with("devsweep-gpui-c11-"),
            "fixture directory must start with devsweep-gpui-c11-: {name}"
        );
        let mut session = Session::unbacked();
        session.set_scan_roots(vec![root.to_str().expect("utf-8 project root").to_string()]);
        assert!(session.request_project_rows());
        let deadline = Instant::now() + Duration::from_secs(10);
        while session.mailbox.len() == 0 {
            assert!(
                Instant::now() < deadline,
                "CleanRows was not posted within 10 seconds"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let poll = session.drain_mail();
        assert!(poll.applied > 0);
        assert!(!session.shell.visible_row_ids(Mode::Clean).is_empty());
        assert_eq!(session.shell.activity(), Activity::Idle);
        assert_eq!(session.shell.mode(), Mode::Clean);
        cleanup(&root);

        let mut empty = Session::unbacked();
        assert!(empty.shell.scan_roots().is_empty());
        assert!(!empty.request_project_rows());
        assert_eq!(empty.drain_mail().applied, 0);
        assert!(empty.shell.visible_row_ids(Mode::Clean).is_empty());
        assert_eq!(empty.shell.activity(), Activity::Idle);
        assert_eq!(empty.shell.mode(), Mode::Clean);
    }

    #[test]
    fn clean_row_window_keeps_offscreen_selection() {
        let mut session = Session::unbacked();
        assert_eq!(session.shell.mode(), Mode::Clean);
        let ids: Vec<String> = (0..10_000).map(|i| format!("clean-{i}")).collect();
        let far = ids[9000].clone();
        let before = crate::view::panel_lines(&session, None);
        session.shell.set_row_ids(Mode::Clean, ids);
        session.shell.set_row_window_start(Mode::Clean, 0);
        session.shell.select(Mode::Clean, far.clone());

        let visible = session.shell.visible_row_ids(Mode::Clean);
        assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
        assert!(!visible.contains(&far));
        assert_eq!(session.shell.selection(Mode::Clean), Some(far.as_str()));

        let lines = crate::view::panel_lines(&session, None);
        assert_eq!(lines.iter().filter(|line| *line == &far).count(), 1);
        assert!(lines.len().saturating_sub(before.len()) <= VISIBLE_ROW_CAPACITY + 1);
        assert!(
            lines.len() < 10_000,
            "panel_lines must not paint 10_000 rows: {}",
            lines.len()
        );

        session.show_mode(Mode::Software);
        session.show_mode(Mode::Clean);
        assert_eq!(session.shell.selection(Mode::Clean), Some(far.as_str()));

        session.shell.set_row_window_start(Mode::Clean, 8952);
        let visible = session.shell.visible_row_ids(Mode::Clean);
        assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
        assert!(visible.contains(&far));
        let lines = crate::view::panel_lines(&session, None);
        let marked = format!("> {far}");
        assert_eq!(lines.iter().filter(|line| *line == &marked).count(), 1);
        assert_eq!(lines.iter().filter(|line| *line == &far).count(), 0);
        assert!(lines.len().saturating_sub(before.len()) <= VISIBLE_ROW_CAPACITY + 1);
        assert!(lines.len() < 10_000);
    }

    #[test]
    fn clean_search_filters_without_dropping_selection() {
        let mut session = Session::unbacked();
        assert_eq!(session.shell.mode(), Mode::Clean);
        let ids: Vec<String> = (0..10_000).map(|i| format!("clean-{i}")).collect();
        let far = ids[9000].clone();
        session.shell.set_row_ids(Mode::Clean, ids.clone());
        session.shell.set_row_window_start(Mode::Clean, 0);
        session.shell.select(Mode::Clean, far.clone());

        session.shell.set_row_query(Mode::Clean, "clean-9000");
        let visible = session.shell.visible_row_ids(Mode::Clean);
        assert!(visible.contains(&far));
        assert_eq!(visible.len(), 1);
        let lines = crate::view::panel_lines(&session, None);
        let marked = format!("> {far}");
        assert_eq!(lines.iter().filter(|line| *line == &marked).count(), 1);
        assert_eq!(lines.iter().filter(|line| *line == &far).count(), 0);
        assert_eq!(session.shell.selection(Mode::Clean), Some(far.as_str()));

        session.shell.set_row_query(Mode::Clean, "missing-row");
        let visible = session.shell.visible_row_ids(Mode::Clean);
        assert!(visible.is_empty());
        assert_eq!(session.shell.selection(Mode::Clean), Some(far.as_str()));
        let lines = crate::view::panel_lines(&session, None);
        assert_eq!(lines.iter().filter(|line| *line == &far).count(), 1);
        assert!(
            lines.len() < 10_000,
            "panel_lines must stay far below 10_000 rows: {}",
            lines.len()
        );

        session.shell.set_row_query(Mode::Clean, "clean-90");
        let visible = session.shell.visible_row_ids(Mode::Clean);
        assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
        let matches: Vec<String> = ids
            .iter()
            .filter(|id| id.contains("clean-90"))
            .cloned()
            .collect();
        assert!(matches.len() > VISIBLE_ROW_CAPACITY);
        let lines = crate::view::panel_lines(&session, None);
        assert!(!matches.iter().all(|id| lines.iter().any(|line| line == id)));
        assert_eq!(session.shell.selection(Mode::Clean), Some(far.as_str()));

        let op = session.shell.begin_operation(OperationClass::ReadOnly);
        assert!(session.shell.cancel(op));
        let visible = session.shell.visible_row_ids(Mode::Clean);
        assert!(visible.contains(&far));
        assert!(visible.iter().any(|id| id == "clean-90"));
        assert!(!visible.iter().any(|id| id == "clean-0"));
        assert_eq!(session.shell.selection(Mode::Clean), Some(far.as_str()));

        session.shell.set_row_query(Mode::Clean, "");
        let visible = session.shell.visible_row_ids(Mode::Clean);
        assert_eq!(visible.len(), VISIBLE_ROW_CAPACITY);
        assert_eq!(visible, &ids[..VISIBLE_ROW_CAPACITY]);
        assert_eq!(session.shell.selection(Mode::Clean), Some(far.as_str()));
        let lines = crate::view::panel_lines(&session, None);
        assert_eq!(lines.iter().filter(|line| *line == &far).count(), 1);
    }

    #[test]
    fn visible_row_window_hides_offscreen_ids() {
        let mut session = Session::unbacked();
        session.show_mode(Mode::Software);
        let ids: Vec<String> = (0..10_000).map(|i| format!("row-{i}")).collect();
        let far = ids[9000].clone();
        let before = crate::view::panel_lines(&session, None);
        session.shell.set_row_ids(Mode::Software, ids);
        session.shell.set_row_window_start(Mode::Software, 0);
        session.shell.select(Mode::Software, far.clone());

        let visible = session.shell.visible_row_ids(Mode::Software);
        assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
        assert!(!visible.contains(&far));
        assert_eq!(session.shell.selection(Mode::Software), Some(far.as_str()));

        let lines = crate::view::panel_lines(&session, None);
        assert!(!lines.iter().any(|line| line == &far));
        assert!(lines.len().saturating_sub(before.len()) <= VISIBLE_ROW_CAPACITY);
        assert!(
            lines.len() < 10_000,
            "panel_lines must not paint 10_000 rows: {}",
            lines.len()
        );

        session.shell.set_row_window_start(Mode::Software, 8952);
        let visible = session.shell.visible_row_ids(Mode::Software);
        assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
        assert!(visible.contains(&far));
        let lines = crate::view::panel_lines(&session, None);
        let marked = format!("> {far}");
        assert!(
            lines.iter().any(|line| line == &marked || line == &far),
            "visible selection must stay in the bounded window: {lines:?}"
        );
        assert!(lines.len().saturating_sub(before.len()) <= VISIBLE_ROW_CAPACITY + 1);
        assert!(lines.len() < 10_000);

        session.show_mode(Mode::Optimize);
        session.apply_key(NavKey::Right);
        assert_eq!(session.shell.selection(Mode::Software), Some(far.as_str()));
    }

    #[test]
    fn mutating_single_flight_rejects_replacement() {
        let mut shell = ShellState::new();
        shell.show_mode(Mode::Software);
        let original = shell.begin_operation(OperationClass::Mutating);
        assert_eq!(shell.begin_operation(OperationClass::Mutating), original);
        assert_eq!(shell.begin_operation(OperationClass::ReadOnly), original);
        shell.show_mode(Mode::Optimize);
        shell.apply_key(NavKey::Right);
        assert!(!shell.cancel(original));
        assert_eq!(
            shell.request_close(),
            CloseAction::WaitOnly { id: original }
        );
        assert!(shell.apply_completion(original, "software-mutating"));
        assert_eq!(shell.report(Mode::Software), Some("software-mutating"));
        assert_eq!(shell.report(Mode::Optimize), None);
        assert_eq!(shell.report(Mode::Analyze), None);
    }

    #[test]
    fn mailbox_stops_at_32_and_a_canceled_worker_cannot_revive_state() {
        let mailbox = Arc::new(SharedMailbox::new());
        for seq in 0..32 {
            mailbox
                .try_post(MailEvent::Lifecycle { seq })
                .expect("capacity includes 32");
        }
        assert!(mailbox.try_post(MailEvent::Lifecycle { seq: 32 }).is_err());
        let mut shell = ShellState::new();
        let poll = shell.drain_mailbox(&mailbox);
        assert_eq!(poll.applied, 32);
        assert!(!poll.decorative_frame);
        shell.apply_key(NavKey::Right);
        assert_eq!(shell.mode(), Mode::Software);
        assert_eq!(shell.drain_mailbox(&mailbox).applied, 0);
        assert_eq!(shell.lifecycle_applied(), 32);

        shell.show_mode(Mode::Clean);
        let id = shell.begin_operation(OperationClass::ReadOnly);
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let caller = thread::current().id();
        let (worker_tx, worker_rx) = mpsc::channel();
        dispatch_blocking(Arc::clone(&mailbox), move || {
            worker_tx.send(thread::current().id()).unwrap();
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            MailEvent::Completion {
                id,
                report: "late".to_string(),
            }
        });
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("worker start");
        assert_ne!(caller, worker_rx.recv().expect("worker thread"));
        assert!(shell.cancel(id));
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while mailbox.len() == 0 {
            assert!(Instant::now() < deadline, "late result was not posted");
            thread::sleep(Duration::from_millis(10));
        }
        let poll = shell.drain_mailbox(&mailbox);
        assert_eq!(poll.applied, 1);
        assert!(!poll.decorative_frame);
        assert_eq!(shell.report(Mode::Clean), None);
        assert!(shell.rejected_results() >= 1);
    }

    #[test]
    fn c10_ac4_idle_minimized_and_hidden_schedule_no_decorative_frame() {
        let mut shell = ShellState::new();
        assert_eq!(shell.activity(), Activity::Idle);
        for presence in [Presence::Visible, Presence::Minimized, Presence::Hidden] {
            shell.set_presence(presence);
            assert!(!shell.request_decorative_frame());
            assert!(shell.decorative_deadline().is_none());
        }
        shell.set_motion(DesktopMotion::Reduced);
        shell.set_presence(Presence::Visible);
        assert!(!shell.request_decorative_frame());
        let _id = shell.begin_operation(OperationClass::ReadOnly);
        assert_eq!(shell.activity(), Activity::Busy);
        assert!(shell.decorative_deadline().is_none());
        assert_eq!(shell.decorative_frames_scheduled(), 0);
        assert!(shell.decorative_rejections() >= 4);
        let poll = shell.drain_mailbox(&SharedMailbox::new());
        assert!(!poll.decorative_frame);
    }

    #[test]
    fn launch_does_not_migrate_and_an_unchanged_save_writes_nothing() {
        let root =
            env::temp_dir().join(format!("devsweep-gpui-c10-{}-session", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let settings = root.join("settings");
        fs::create_dir_all(&settings).unwrap();
        let files = PreferenceFiles::new(
            settings.clone(),
            root.join("protect").join("protected-paths.json"),
        );
        let legacy = br#"{"schema_version":1,"theme":"light","font_family":"segoe_ui","text_scale_percent":110,"motion":"system","planet_fps":30,"status_interval_seconds":2,"status_process_limit":15,"hud_interval_seconds":2}"#;
        fs::write(settings.join("desktop-preferences-v1.json"), legacy).unwrap();
        let mut session = Session::launch(files);
        assert!(!settings.join("desktop-preferences-v2.json").exists());
        assert_eq!(session.theme(), DesktopTheme::Light);
        assert_eq!(session.text_scale(), 110);
        assert_eq!(
            session.shell().report(Mode::Clean),
            None,
            "launch does not invent a clean report"
        );
        assert!(!session.save_settings().unwrap());
        assert!(!settings.join("desktop-preferences-v2.json").exists());
        assert!(session.shell().scan_roots().is_empty());
        session.set_scan_roots(vec![root.join("missing").display().to_string()]);
        assert!(!settings.join("clean-scan-roots-v1.json").exists());
        assert!(session.save_scan_roots().is_err());
        assert!(!settings.join("clean-scan-roots-v1.json").exists());
        assert!(!settings.join("desktop-preferences-v2.json").exists());
        let protected = root.join("keep");
        fs::create_dir_all(&protected).unwrap();
        session.set_protections(vec![protected.display().to_string()]);
        assert!(!root.join("protect").join("protected-paths.json").exists());
        session.save_protections().unwrap();
        let protection_bytes = fs::read(root.join("protect").join("protected-paths.json")).unwrap();
        assert!(
            String::from_utf8(protection_bytes)
                .unwrap()
                .contains("keep")
        );
        assert!(!settings.join("clean-scan-roots-v1.json").exists());
        assert!(!settings.join("desktop-preferences-v2.json").exists());
        session.cycle_language();
        assert_eq!(session.locale(), Locale::ZhCn);
        assert_eq!(crate::copy::mode_name(session.locale(), "clean"), "清理");
        assert!(!settings.join("presentation-v1.json").exists());
        assert!(session.save_settings().unwrap());
        assert!(settings.join("presentation-v1.json").exists());
        assert!(
            !settings.join("desktop-preferences-v2.json").exists(),
            "a language save must not migrate desktop preferences"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clean_session_keyboard_flow_previews_and_refuses_a_stale_digest() {
        let root = isolated_multi_cache_project();
        let mut session = Session::unbacked();
        assert_eq!(session.shell().mode(), Mode::Clean);
        session.set_scan_roots(vec![root.display().to_string()]);

        // The landing render never scans; requesting rows is an explicit action.
        assert!(session.request_project_rows());
        assert_eq!(session.shell().activity(), Activity::Busy);
        assert!(drain_until_idle(&mut session) > 0);
        assert_eq!(session.shell().activity(), Activity::Idle);
        assert_eq!(session.clean_workbench().phase(), CleanPhase::Found);
        assert!(session.clean_workbench().rows().len() >= 3);
        assert_eq!(
            session.shell().visible_row_ids(Mode::Clean),
            session.clean_workbench().row_ids()
        );
        assert_eq!(
            session.clean_outcome(),
            None,
            "a completed scan with rows has no outcome notice"
        );

        // Alt+Down moves the row cursor without changing the mode; Space
        // toggles the focused row and keeps the marker in the painted panel.
        session.apply_key(NavKey::RowDown);
        let cursor = session
            .shell()
            .selection(Mode::Clean)
            .expect("row cursor")
            .to_string();
        assert_eq!(session.shell().mode(), Mode::Clean);
        let rule = session
            .clean_workbench()
            .row(&cursor)
            .expect("cursor row exists")
            .rule_id
            .clone();
        assert!(session.clean_toggle_focused_row(), "space selects the row");
        assert!(session.clean_workbench().is_selected(&cursor));
        let lines = crate::view::panel_lines(&session, None);
        let marked = format!("> [x] {rule}");
        assert!(
            lines.iter().any(|line| line.starts_with(&marked)),
            "panel missing {marked:?}: {lines:?}"
        );
        assert!(
            !session.clean_toggle_focused_row(),
            "space toggles the row back off"
        );
        assert!(
            session.clean_toggle_focused_row(),
            "space toggles the row on"
        );
        let summary = lines
            .iter()
            .find(|line| line.starts_with("Selected: "))
            .expect("selection summary line");
        assert!(
            summary.contains(&cursor),
            "the review summary lists the selected id: {summary}"
        );

        // The preview digest comes from the shared core services.
        assert!(session.clean_preview_selection());
        let preview = session
            .clean_workbench()
            .preview()
            .expect("preview")
            .clone();
        assert!(preview.digest.starts_with("sha256:"));
        assert_eq!(preview.count, 1);
        let lines = crate::view::panel_lines(&session, None);
        assert!(
            lines.iter().any(|line| line.contains(&preview.digest)),
            "panel must render the live digest: {lines:?}"
        );

        // Any selection change invalidates the digest. The confirm gate
        // refuses without starting a worker and never moves a file.
        let second = session
            .clean_workbench()
            .rows()
            .iter()
            .map(|row| row.id.clone())
            .find(|id| id != &cursor)
            .expect("second row");
        assert!(session.clean_click_row(&second));
        assert!(
            !session.clean_confirm_execute(),
            "a changed selection must refuse to execute"
        );
        assert_eq!(
            session.clean_outcome(),
            Some(CleanOutcomeNotice::StalePreview)
        );
        assert_eq!(
            session.shell().activity(),
            Activity::Idle,
            "a refused confirm must not start a worker"
        );
        assert_eq!(session.clean_workbench().phase(), CleanPhase::Found);
        assert!(session.clean_workbench().preview().is_none());
        let lines = crate::view::panel_lines(&session, None);
        assert_eq!(
            lines.last(),
            Some(&crate::copy::clean_outcome_stale_preview(Locale::En).to_string())
        );
        assert!(
            root.join(".pytest_cache").join("payload").is_file(),
            "a stale digest must never move the fixture"
        );
        workbench_cleanup(&root);
    }

    #[test]
    fn clean_session_partial_outcome_is_rendered_from_the_workbench_notice() {
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-session-missing-root",
            std::process::id()
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing a scan root outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        let mut session = Session::unbacked();
        session.set_scan_roots(vec![root.display().to_string()]);
        // A leftover notice must not survive the next scan result.
        session.set_clean_outcome(Some(CleanOutcomeNotice::Canceled));
        assert_eq!(session.clean_outcome(), Some(CleanOutcomeNotice::Canceled));

        assert!(session.request_project_rows());
        assert!(drain_until_idle(&mut session) > 0);
        assert_eq!(session.clean_workbench().phase(), CleanPhase::Failed);
        assert_eq!(session.clean_outcome(), Some(CleanOutcomeNotice::Partial));
        assert!(session.clean_workbench().failure_detail().is_some());
        assert!(session.clean_workbench().rows().is_empty());
        assert!(session.shell().visible_row_ids(Mode::Clean).is_empty());
        let lines = crate::view::panel_lines(&session, None);
        assert_eq!(
            lines.last(),
            Some(&crate::copy::clean_outcome_partial(Locale::En).to_string())
        );
        assert!(
            lines
                .iter()
                .any(|line| line == crate::copy::clean_phase_failed(Locale::En))
        );

        // Retry keeps the accepted project scope and stays explicit.
        assert!(session.clean_rescan());
        assert!(drain_until_idle(&mut session) > 0);
        assert_eq!(session.clean_outcome(), Some(CleanOutcomeNotice::Partial));
        assert_eq!(session.shell().activity(), Activity::Idle);
        workbench_cleanup(&root);
    }

    #[test]
    fn clean_session_search_sort_and_window_stay_bounded_and_keep_selection() {
        let root = isolated_multi_cache_project();
        let mut session = Session::unbacked();
        session.set_scan_roots(vec![root.display().to_string()]);
        assert!(session.request_project_rows());
        assert!(drain_until_idle(&mut session) > 0);
        assert_eq!(session.clean_workbench().phase(), CleanPhase::Found);

        session.apply_key(NavKey::RowDown);
        let cursor = session
            .shell()
            .selection(Mode::Clean)
            .expect("row cursor")
            .to_string();
        assert!(session.clean_toggle_focused_row());

        // The search field drives the existing row query and keeps the
        // selection even while the selected row is filtered out.
        assert!(session.clean_set_search("mypy"));
        assert_eq!(session.shell().row_query(Mode::Clean), "mypy");
        let visible = session.shell().visible_row_ids(Mode::Clean);
        assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
        assert!(visible.iter().all(|id| id.contains("mypy")), "{visible:?}");
        assert!(session.clean_workbench().is_selected(&cursor));
        assert!(
            !session.clean_set_search("mypy"),
            "unchanged query is a no-op"
        );
        assert!(session.clean_set_search(""));
        assert_eq!(
            session.shell().visible_row_ids(Mode::Clean),
            session.clean_workbench().row_ids()
        );

        // Sort cycles are projections over the accepted report: rows reorder
        // and the selection survives every step.
        let report_order = session.clean_workbench().rows().to_vec();
        assert!(session.clean_cycle_sort());
        assert_eq!(session.clean_workbench().sort(), CleanSort::Name);
        assert!(session.clean_workbench().is_selected(&cursor));
        assert_eq!(
            session.shell().visible_row_ids(Mode::Clean),
            session.clean_workbench().row_ids()
        );
        assert!(session.clean_cycle_sort());
        assert_eq!(session.clean_workbench().sort(), CleanSort::Size);
        assert!(session.clean_workbench().is_selected(&cursor));
        assert!(session.clean_cycle_sort());
        assert_eq!(session.clean_workbench().sort(), CleanSort::ReportOrder);
        assert_eq!(session.clean_workbench().rows(), report_order.as_slice());
        assert!(session.clean_workbench().is_selected(&cursor));

        // The bounded row window clamps at both ends.
        let window_len = session.shell().window_row_count(Mode::Clean);
        assert!(session.clean_scroll_rows(48));
        assert_eq!(session.shell().row_window_start(Mode::Clean), window_len);
        assert!(session.clean_scroll_rows(-48));
        assert_eq!(session.shell().row_window_start(Mode::Clean), 0);
        assert!(
            !session.clean_scroll_rows(-1),
            "scrolling past the start is a no-op"
        );
        assert!(session.clean_workbench().is_selected(&cursor));
        workbench_cleanup(&root);
    }

    #[test]
    fn clean_cycle_sort_reorders_a_found_workbench_without_changing_mode() {
        use crate::clean_workbench::{CleanRow, CleanRowScope};

        fn row(id: &str, rule_id: &str, bytes: u64) -> CleanRow {
            CleanRow {
                id: id.to_string(),
                plan_index: 0,
                rule_id: rule_id.to_string(),
                provider: "fixture".to_string(),
                scope: CleanRowScope::Global,
                path: Some(id.to_string()),
                bytes,
                size_complete: true,
                inspect_only: false,
                selected_by_default: false,
            }
        }

        let mut session = Session::unbacked();
        assert_eq!(session.shell().mode(), Mode::Clean);
        assert!(
            !session.clean_cycle_sort(),
            "a landing workbench is not Found"
        );
        assert_eq!(session.shell().mode(), Mode::Clean);

        session.clean.install_rows_for_test(vec![
            row("row-b", "node.dist", 10),
            row("row-a", "python.pytest_cache", 40),
            row("row-c", "python.mypy_cache", 40),
        ]);
        assert_eq!(session.clean_workbench().phase(), CleanPhase::Found);
        assert_eq!(
            session.clean_workbench().row_ids().to_vec(),
            ["row-b", "row-a", "row-c"]
        );

        assert!(session.clean_cycle_sort());
        assert_eq!(session.shell().mode(), Mode::Clean);
        assert_eq!(session.clean_workbench().sort(), CleanSort::Name);
        assert_eq!(
            session.clean_workbench().row_ids().to_vec(),
            ["row-b", "row-c", "row-a"]
        );
        assert_eq!(
            session.shell().visible_row_ids(Mode::Clean).to_vec(),
            ["row-b", "row-c", "row-a"]
        );

        session.show_mode(Mode::Software);
        assert!(session.clean_cycle_sort());
        assert_eq!(
            session.shell().mode(),
            Mode::Software,
            "sort must not change mode"
        );
        assert_eq!(session.clean_workbench().sort(), CleanSort::Size);
        assert_eq!(
            session.clean_workbench().row_ids().to_vec(),
            ["row-a", "row-c", "row-b"]
        );
    }

    #[test]
    fn clean_review_is_active_only_for_the_found_report_or_the_ai_review() {
        use crate::view::clean_review_is_active;
        use devsweep_core::ai::AiObservationV1;

        fn row(id: &str) -> crate::clean_workbench::CleanRow {
            crate::clean_workbench::CleanRow {
                id: id.to_string(),
                plan_index: 0,
                rule_id: "python.pytest_cache".to_string(),
                provider: "fixture".to_string(),
                scope: crate::clean_workbench::CleanRowScope::Global,
                path: Some(id.to_string()),
                bytes: 4,
                size_complete: true,
                inspect_only: false,
                selected_by_default: false,
            }
        }

        let mut session = Session::unbacked();
        assert!(!clean_review_is_active(&session), "landing is not a review");

        session.clean.install_rows_for_test(vec![row("row-a")]);
        assert!(clean_review_is_active(&session), "found rows are a review");

        session.show_mode(Mode::Software);
        assert!(
            !clean_review_is_active(&session),
            "another mode is not the Clean review"
        );
        session.show_mode(Mode::Clean);
        assert!(clean_review_is_active(&session));

        // Opening AI replaces the Found search field. Landing must not keep
        // Tab and Space bound to a field that is no longer painted.
        assert!(session.clean_ai_open());
        assert_eq!(session.clean_ai().phase(), AiSubviewPhase::Landing);
        assert!(
            !clean_review_is_active(&session),
            "an AI landing over a Found report is not a review"
        );

        session.clean_ai.on_scan_result(CleanAiScanPayload {
            observation: Some(AiObservationV1 {
                schema: "devsweep.ai.observation.v1".to_string(),
                version: 1,
                providers: Vec::new(),
            }),
            error: None,
        });
        assert_eq!(session.clean_ai().phase(), AiSubviewPhase::Review);
        assert!(
            clean_review_is_active(&session),
            "the AI review phase is a review"
        );

        session.open_menu_item(0);
        assert_eq!(session.shell().surface(), Surface::Settings);
        assert!(
            !clean_review_is_active(&session),
            "a menu surface is not a review"
        );
        session.apply_key(NavKey::Escape);
        assert_eq!(session.shell().surface(), Surface::Mode);
        assert!(clean_review_is_active(&session));

        assert!(session.clean_ai_close());
        assert_eq!(session.clean_workbench().phase(), CleanPhase::Found);
        assert!(
            clean_review_is_active(&session),
            "closing AI returns review to the Found report"
        );
    }

    #[test]
    fn filtered_projection_is_cached_across_reads() {
        let mut shell = ShellState::new();
        let ids: Vec<String> = (0..10_000).map(|i| format!("clean-{i}")).collect();
        shell.set_row_ids(Mode::Clean, ids);
        let baseline = shell.filter_rebuild_count(Mode::Clean);
        for _ in 0..100 {
            assert!(shell.visible_row_ids(Mode::Clean).len() <= VISIBLE_ROW_CAPACITY);
        }
        assert_eq!(
            shell.filter_rebuild_count(Mode::Clean),
            baseline,
            "reads must never rebuild the row projection"
        );

        // A query change rebuilds the filtered projection exactly once.
        shell.set_row_query(Mode::Clean, "clean-900");
        assert_eq!(shell.filter_rebuild_count(Mode::Clean), baseline + 1);
        for _ in 0..100 {
            let visible = shell.visible_row_ids(Mode::Clean);
            assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
            assert!(visible.iter().all(|id| id.contains("clean-900")));
        }
        assert_eq!(
            shell.filter_rebuild_count(Mode::Clean),
            baseline + 1,
            "filtered reads must be cached, not rebuilt per read"
        );

        // A new id set rebuilds the filtered projection exactly once more.
        shell.set_row_ids(Mode::Clean, vec!["clean-9000".to_string()]);
        assert_eq!(shell.filter_rebuild_count(Mode::Clean), baseline + 2);
        assert_eq!(shell.visible_row_ids(Mode::Clean), ["clean-9000"]);
        assert_eq!(shell.row_set_install_count(Mode::Clean), 2);
    }

    #[test]
    fn page_navigation_moves_over_the_full_ten_thousand_id_set() {
        use crate::clean_workbench::{CleanRow, CleanRowScope};
        use std::time::Instant;

        let started = Instant::now();
        let mut session = Session::unbacked();
        // The model-level injection keeps this test off the filesystem.
        let rows = (0..10_000)
            .map(|index| CleanRow {
                id: format!("clean-{index}"),
                plan_index: index as u32,
                rule_id: "python.fixture_rule".to_string(),
                provider: "python".to_string(),
                scope: CleanRowScope::Global,
                path: Some(format!("C:/fixture/clean-{index}")),
                bytes: 1,
                size_complete: true,
                inspect_only: false,
                selected_by_default: false,
            })
            .collect::<Vec<_>>();
        session.clean.install_rows_for_test(rows);
        let ids = session.clean.row_ids().to_vec();
        session.set_row_ids(Mode::Clean, ids);
        assert_eq!(session.shell().window_row_count(Mode::Clean), 10_000);
        assert_eq!(
            session.shell().visible_row_ids(Mode::Clean).len(),
            VISIBLE_ROW_CAPACITY
        );

        // PageDown and PageUp scroll over the full filtered count and clamp at
        // both ends.
        assert!(session.clean_scroll_rows(VISIBLE_ROW_CAPACITY as isize));
        assert_eq!(
            session.shell().row_window_start(Mode::Clean),
            VISIBLE_ROW_CAPACITY
        );
        assert!(session.clean_scroll_rows(20_000));
        assert_eq!(session.shell().row_window_start(Mode::Clean), 10_000);
        let visible = session.shell().visible_row_ids(Mode::Clean);
        assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
        assert!(visible.iter().any(|id| id == "clean-9999"));
        assert!(session.clean_scroll_rows(-20_000));
        assert_eq!(session.shell().row_window_start(Mode::Clean), 0);
        assert!(
            !session.clean_scroll_rows(-1),
            "scrolling past the start is a no-op"
        );

        // A far selection survives page moves and stays visible offscreen.
        session.shell.select(Mode::Clean, "clean-9000".to_string());
        assert!(session.clean_scroll_rows(VISIBLE_ROW_CAPACITY as isize));
        let visible = session.shell().visible_row_ids(Mode::Clean);
        assert!(!visible.iter().any(|id| id == "clean-9000"));
        assert_eq!(session.shell().selection(Mode::Clean), Some("clean-9000"));
        let lines = crate::view::panel_lines(&session, None);
        let marked = "> [ ] python.fixture_rule · C:/fixture/clean-9000";
        assert!(
            lines.iter().any(|line| line.starts_with(marked)),
            "an offscreen cursor row must stay painted: {lines:?}"
        );

        // Search keeps operating over the full id set while the window moved.
        assert!(session.clean_set_search("clean-9999"));
        let visible = session.shell().visible_row_ids(Mode::Clean);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0], "clean-9999");
        assert_eq!(session.shell().selection(Mode::Clean), Some("clean-9000"));
        assert!(session.clean_set_search(""));
        assert_eq!(
            session.shell().window_row_count(Mode::Clean),
            10_000,
            "clearing the query must restore the full filtered count"
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "10k page navigation must stay under one second: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn protection_toggle_persists_through_the_existing_store() {
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-protect-session",
            std::process::id()
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing a protection session outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("settings")).unwrap();
        let protection_path = root.join("protect").join("protected-paths.json");
        let files = PreferenceFiles::new(root.join("settings"), protection_path.clone());
        let mut session = Session::launch(files);
        let project = isolated_multi_cache_project();
        session.set_scan_roots(vec![project.display().to_string()]);
        assert!(session.request_project_rows());
        assert!(drain_until_idle(&mut session) > 0);
        assert_eq!(session.clean_workbench().phase(), CleanPhase::Found);
        let row_id = session
            .clean_workbench()
            .rows()
            .iter()
            .find(|row| row.rule_id == "python.pytest_cache")
            .expect("pytest row")
            .id
            .clone();
        let row_path = session
            .clean_workbench()
            .row(&row_id)
            .and_then(|row| row.path.clone())
            .expect("pytest path");

        assert!(session.clean_click_row(&row_id));
        assert!(session.clean_workbench().is_selected(&row_id));
        assert!(session.clean_toggle_row_protection(&row_id));
        assert!(session.clean_workbench().is_protected(&row_id));
        assert!(
            !session.clean_workbench().is_selected(&row_id),
            "protecting a row must drop it from the reviewed selection"
        );
        assert!(
            !session.clean_toggle_focused_row(),
            "a protected row refuses selection"
        );
        assert!(
            session.clean_workbench().row(&row_id).is_some(),
            "a protected row stays visible"
        );
        assert!(!session.protection_session_only());
        let stored = fs::read(&protection_path).expect("protection store written");
        // The JSON store escapes `\` on Windows, so compare the decoded
        // document instead of the raw text.
        let document: serde_json::Value =
            serde_json::from_slice(&stored).expect("protection store JSON");
        assert_eq!(document["version"], 1, "{document}");
        let paths = document["paths"].as_array().expect("stored paths array");
        assert_eq!(paths.len(), 1, "{document}");
        assert_eq!(paths[0].as_str(), Some(row_path.as_str()), "{document}");

        // Toggling again removes the exact entry from the same store.
        assert!(!session.clean_toggle_row_protection(&row_id));
        assert!(!session.clean_workbench().is_protected(&row_id));
        assert!(
            session.clean_toggle_focused_row(),
            "the unprotected focused row enters the selection again"
        );
        let stored = fs::read(&protection_path).expect("protection store updated");
        assert!(!String::from_utf8(stored).unwrap().contains("pytest_cache"));
        workbench_cleanup(&project);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ai_subview_review_flow_uses_explicit_home_and_bounded_window() {
        use crate::clean::cleanup as ai_cleanup;
        use crate::clean_ai::{AiCategory, AiSubviewPhase, isolated_ai_review_home};
        use devsweep_core::ai::AiProvider;

        let home = isolated_ai_review_home();
        let name = home
            .file_name()
            .and_then(|value| value.to_str())
            .expect("utf-8 fixture name");
        assert!(
            name.starts_with("devsweep-gpui-c11-"),
            "fixture directory must start with devsweep-gpui-c11-: {name}"
        );
        let mut session = Session::unbacked();

        // Opening the subview never scans.
        assert!(!session.clean_ai().open());
        assert!(session.clean_ai_open());
        assert_eq!(session.clean_ai().phase(), AiSubviewPhase::Landing);
        assert_eq!(session.shell().activity(), Activity::Idle);
        assert!(session.shell().visible_row_ids(Mode::Clean).is_empty());

        // The closed provider set cycles and returns to the default.
        assert_eq!(session.clean_ai().provider(), AiProvider::Codex);
        assert!(session.clean_ai_cycle_provider());
        assert_eq!(session.clean_ai().provider(), AiProvider::ClaudeCode);
        assert!(session.clean_ai_cycle_provider());
        assert_eq!(session.clean_ai().provider(), AiProvider::Cursor);
        assert!(session.clean_ai_cycle_provider());
        assert_eq!(session.clean_ai().provider(), AiProvider::Codex);

        // The scan is one explicit action against the explicit home only.
        assert!(session.clean_ai_scan_home(home.clone()));
        assert_eq!(session.shell().activity(), Activity::Busy);
        assert!(drain_until_idle(&mut session) > 0);
        assert_eq!(session.clean_ai().phase(), AiSubviewPhase::Review);
        assert_eq!(session.shell().activity(), Activity::Idle);
        let cache_ids = session.clean_ai().category_ids(AiCategory::Cache).to_vec();
        assert_eq!(cache_ids.len(), 3);
        assert_eq!(
            session.shell().visible_row_ids(Mode::Clean),
            cache_ids.as_slice()
        );
        let regen_a = cache_ids
            .iter()
            .find(|id| id.ends_with("cache/regen-a"))
            .expect("regen-a row")
            .clone();
        let regen_b = cache_ids
            .iter()
            .find(|id| id.ends_with("cache/regen-b"))
            .expect("regen-b row")
            .clone();

        // Search and the bounded window operate over the full AI id set.
        assert!(session.clean_set_search("regen"));
        assert_eq!(session.shell().visible_row_ids(Mode::Clean).len(), 2);
        assert!(session.clean_set_search("regen-a"));
        assert_eq!(session.shell().visible_row_ids(Mode::Clean).len(), 1);
        assert_eq!(
            session.shell().visible_row_ids(Mode::Clean)[0],
            regen_a.as_str()
        );
        assert!(session.clean_set_search(""));

        // Clicking selects one AI candidate; a sensitive row refuses.
        assert!(session.clean_click_row(&regen_a));
        assert!(session.clean_ai().is_selected(&regen_a));
        let sensitive = session
            .clean_ai()
            .category_ids(AiCategory::Sensitive)
            .to_vec();
        assert_eq!(sensitive.len(), 7);
        assert!(!session.clean_click_row(&sensitive[0]));
        assert!(!session.clean_ai().is_selected(&sensitive[0]));

        // The live digest comes from the shared core service and is painted.
        assert!(session.clean_ai_preview());
        let preview = session.clean_ai().preview().expect("preview").clone();
        assert!(preview.digest.starts_with("sha256:"));
        assert_eq!(preview.count, 1);
        let lines = crate::view::panel_lines(&session, None);
        assert!(
            lines.iter().any(|line| line.contains(&preview.digest)),
            "panel must render the AI digest: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("sensitive")),
            "panel must mark sensitive rows: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line == "AI tool review"),
            "{lines:?}"
        );

        // A changed selection refuses the confirm gate without starting work.
        assert!(session.clean_click_row(&regen_b));
        assert!(!session.clean_ai_confirm_execute());
        assert_eq!(
            session.clean_ai().notice(),
            Some(CleanOutcomeNotice::StalePreview)
        );
        assert_eq!(
            session.shell().activity(),
            Activity::Idle,
            "a refused AI confirm must not start a worker"
        );
        assert_eq!(session.clean_ai().phase(), AiSubviewPhase::Review);
        assert!(session.clean_ai().preview().is_none());

        // Recovery refresh is explicit, reads the same home, and copies
        // nothing; this fixture has no bundles.
        assert!(session.clean_ai_refresh_recovery());
        assert!(drain_until_idle(&mut session) > 0);
        assert!(session.clean_ai().recovery_loaded());
        assert!(session.clean_ai().recovery().is_empty());
        assert!(home.join("cache/regen-a/data.bin").is_file());

        // Escape leaves the subview before the surface back key.
        assert!(session.clean_back());
        assert!(!session.clean_ai().open());
        assert!(session.shell().visible_row_ids(Mode::Clean).is_empty());
        ai_cleanup(&home);
    }

    fn ai_home_env_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    struct AiHomeEnvGuard {
        previous: Option<std::ffi::OsString>,
    }

    impl AiHomeEnvGuard {
        fn replace(value: Option<&std::ffi::OsStr>) -> Self {
            let previous = env::var_os("DEVSWEEP_AI_REVIEW_HOME");
            unsafe {
                match value {
                    Some(value) => env::set_var("DEVSWEEP_AI_REVIEW_HOME", value),
                    None => env::remove_var("DEVSWEEP_AI_REVIEW_HOME"),
                }
            }
            Self { previous }
        }
    }

    impl Drop for AiHomeEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match self.previous.take() {
                    Some(value) => env::set_var("DEVSWEEP_AI_REVIEW_HOME", value),
                    None => env::remove_var("DEVSWEEP_AI_REVIEW_HOME"),
                }
            }
        }
    }

    #[test]
    fn window_ai_scan_does_not_read_a_real_profile_without_a_temp_home() {
        use devsweep_core::ai::AiProvider;

        let _guard = ai_home_env_lock().lock().expect("ai home env lock");
        let _restore = AiHomeEnvGuard::replace(None);
        let real = crate::clean::ai_provider_home_from_env(AiProvider::Codex);
        let mut session = Session::unbacked();
        session.show_mode(Mode::Clean);
        assert!(session.clean_ai_open());
        assert!(!session.clean_ai_scan());
        assert!(session.clean_ai().home().is_none());
        assert_eq!(session.shell().activity(), Activity::Idle);
        assert_eq!(session.shell().operation_id(), None);
        assert_eq!(
            session.clean_ai().failure_detail(),
            Some(crate::copy::clean_ai_isolated_home_required(Locale::En))
        );
        if let Some(real) = real {
            assert!(
                !session
                    .clean_ai()
                    .failure_detail()
                    .unwrap_or("")
                    .contains(&real.display().to_string()),
                "refusal must not name the real profile"
            );
            assert!(!session.clean_ai_scan_home(real));
            assert!(session.clean_ai().home().is_none());
            assert_eq!(session.shell().activity(), Activity::Idle);
        }
        if let Some(profile) = env::var_os("USERPROFILE") {
            assert!(!session.clean_ai_scan_home(PathBuf::from(profile)));
            assert!(session.clean_ai().home().is_none());
        }
    }

    #[test]
    fn window_ai_scan_uses_only_the_explicit_temp_home() {
        use crate::clean_ai::{AiCategory, isolated_ai_review_home};
        use devsweep_core::ai::AiProvider;

        let _guard = ai_home_env_lock().lock().expect("ai home env lock");
        let home = isolated_ai_review_home();
        let _restore = AiHomeEnvGuard::replace(Some(home.as_os_str()));
        let real = crate::clean::ai_provider_home_from_env(AiProvider::Codex);
        let mut session = Session::unbacked();
        session.show_mode(Mode::Clean);
        assert!(session.clean_ai_open());
        assert!(session.clean_ai_scan());
        assert!(drain_until_idle(&mut session) > 0);
        let scanned = session
            .clean_ai()
            .home()
            .expect("explicit home")
            .to_path_buf();
        assert_eq!(scanned, home);
        if let Some(real) = real {
            assert_ne!(scanned, real);
        }
        assert!(
            !session
                .clean_ai()
                .category_ids(AiCategory::Cache)
                .is_empty()
        );
        assert!(
            !session
                .clean_ai()
                .category_ids(AiCategory::Sessions)
                .is_empty()
        );
        assert!(
            !session
                .clean_ai()
                .category_ids(AiCategory::Worktrees)
                .is_empty()
        );
        assert!(session.clean_ai().sensitive_count() > 0);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn analyze_roots_keep_a_listed_drive_and_an_isolated_folder() {
        let drives = vec![AnalyzeRootChoice {
            input: "C:\\".to_string(),
            normalized: Some("C:\\".to_string()),
            error: None,
        }];
        let extras = vec![
            AnalyzeRootChoice {
                input: "C:\\".to_string(),
                normalized: Some("C:\\".to_string()),
                error: None,
            },
            AnalyzeRootChoice {
                input: "D:\\isolated".to_string(),
                normalized: Some("D:\\isolated".to_string()),
                error: None,
            },
        ];
        let merged = super::merge_analyze_roots(drives, extras);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].input, "C:\\");
        assert_eq!(merged[1].input, "D:\\isolated");
    }
}
