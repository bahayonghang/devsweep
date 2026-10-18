//! Five-mode shell state, explicit preference edits, and operation fences.
//!
//! Navigation tests call this model directly. Rendering does not own cleanup
//! authority. A completion applies only when its operation id is still current
//! and has not been canceled.

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use crate::{
    analyze::AnalyzeBoard,
    clean::project_target_ids,
    copy::{Locale, StoreName},
    preferences::{
        self, DesktopMotion, DesktopPatch, DesktopPreferences, DesktopTheme, LanguageTag,
        PreferenceFiles, StoreError,
    },
    status_panel::StatusPanel,
};
use devsweep_core::{
    ai::{AiError, AiExecuteReportV1, AiExit},
    optimize::PrecheckReason,
    software::{AdapterOutcome, SoftwareExecutionOutcome, SoftwarePlanError},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavKey {
    Left,
    Right,
    Up,
    Down,
    RowDown,
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
    Completion { id: u64, report: String },
    CleanRows { id: u64, ids: Vec<String> },
    Lifecycle { seq: u64 },
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
    clean_outcome: Option<CleanOutcomeNotice>,
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
            clean_outcome: None,
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

    pub fn clean_outcome(&self) -> Option<CleanOutcomeNotice> {
        self.clean_outcome
    }

    pub fn set_clean_outcome(&mut self, notice: Option<CleanOutcomeNotice>) {
        self.clean_outcome = notice;
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
        memory.row_ids = ids;
        memory.rebuild_filtered_rows();
        memory.clamp_window_start();
    }

    pub fn set_row_window_start(&mut self, mode: Mode, start: usize) {
        let memory = &mut self.memories[mode.index()];
        memory.row_window_start = start.min(memory.window_ids().len());
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
        let batch = {
            let mut events = mailbox.events.lock().expect("mailbox lock");
            let count = events.len().min(32);
            events.drain(..count).collect::<Vec<_>>()
        };
        for event in &batch {
            self.apply_mail(event);
        }
        Poll {
            applied: batch.len(),
            decorative_frame: self.decorative_deadline().is_some(),
        }
    }

    fn apply_mail(&mut self, event: &MailEvent) {
        match event {
            MailEvent::Lifecycle { .. } => self.lifecycle_applied += 1,
            MailEvent::Completion { id, report } => {
                let _ = self.apply_completion(*id, report.clone());
            }
            MailEvent::CleanRows { id, ids } => {
                self.apply_clean_rows(*id, ids.clone());
            }
        }
    }

    fn apply_clean_rows(&mut self, id: u64, ids: Vec<String>) {
        let Some(operation) = &self.operation else {
            self.rejected_results += 1;
            return;
        };
        if operation.id != id || operation.canceled {
            self.rejected_results += 1;
            return;
        }
        self.set_row_ids(Mode::Clean, ids);
        self.operation = None;
        self.activity = Activity::Idle;
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
    status_panel: Option<StatusPanel>,
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
            status_panel: None,
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

    pub fn clean_outcome(&self) -> Option<CleanOutcomeNotice> {
        self.shell.clean_outcome()
    }

    pub fn set_clean_outcome(&mut self, notice: Option<CleanOutcomeNotice>) {
        self.shell.set_clean_outcome(notice);
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
        self.shell.apply_key(key);
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
    }

    pub fn toggle_menu(&mut self) {
        self.shell.toggle_menu();
    }

    pub fn open_menu_item(&mut self, index: usize) {
        self.shell.open_menu_item(index);
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

    pub fn cycle_theme(&mut self) {
        self.draft.theme = self.draft.theme.cycle();
    }

    pub fn set_scan_roots(&mut self, roots: Vec<String>) {
        self.shell.set_scan_roots(roots);
    }

    /// Load Clean rows from the first project scan root. Empty roots do not scan.
    pub fn request_project_rows(&mut self) -> bool {
        let root = match self.shell.scan_roots.first() {
            Some(root) if !root.is_empty() => PathBuf::from(root),
            _ => return false,
        };
        let id = self.shell.begin_operation(OperationClass::ReadOnly);
        let mailbox = Arc::clone(&self.mailbox);
        dispatch_blocking(mailbox, move || match project_target_ids(&root) {
            Ok(ids) => MailEvent::CleanRows { id, ids },
            Err(_) => MailEvent::CleanRows {
                id,
                ids: Vec::new(),
            },
        });
        true
    }

    pub fn drain_mail(&mut self) -> Poll {
        self.shell.drain_mailbox(&self.mailbox)
    }

    pub fn set_protections(&mut self, paths: Vec<String>) {
        self.shell.set_protections(paths);
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

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        sync::{Arc, mpsc},
        thread,
        time::{Duration, Instant},
    };

    use super::*;
    use crate::clean::{cleanup, isolated_pytest_project};

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
        let main = include_str!("main.rs");
        let view = include_str!("view.rs");
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
        assert!(lines.iter().any(|line| line == &far));
        assert!(lines.len().saturating_sub(before.len()) <= VISIBLE_ROW_CAPACITY);
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
}
