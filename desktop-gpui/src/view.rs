//! One native window. The capsule order is Clean, Software, Optimize, Analyze, Status.
//!
//! The in-window beaver control uses the packaged DevSweep master PNG. The
//! window and notification-area glyph use the offline-derived multi-size ICO
//! from `icon.rs`. The Clean landing shows the packaged licensed Earth still.
//! Render does not request an animation frame and never starts a scan.

use std::{
    borrow::Cow,
    path::PathBuf,
    time::{Duration, Instant},
};

use gpui::{
    Action, App, AssetSource, Bounds, Context, ElementId, Entity, FocusHandle, Focusable,
    KeyBinding, PathPromptOptions, SharedString, TitlebarOptions, Window, WindowBounds,
    WindowOptions, actions, div, img, prelude::*, px, rgb, size,
};
use gpui_platform::application;
#[cfg(windows)]
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};

use crate::{
    clean_ai::{AiCategory, AiSubviewPhase},
    clean_map::{self, MapBounds, MapInput, ReviewBand},
    clean_workbench::{CleanLandingPrompt, CleanPhase, CleanRow, CleanScope, CleanSort},
    copy,
    input::{self, TextInput},
    preferences::DesktopTheme,
    shell::{Activity, CleanOutcomeNotice, MENU, Mode, Session, Surface, WorkbenchNotice},
};

const ICON_NAME: &str = "devsweep-icon-master.png";
const COPY_NAME: &str = "copy.txt";
const EARTH_NAME: &str = "earth-still.png";
const ICON_BYTES: &[u8] = include_bytes!("../assets/devsweep-icon-master.png");
const COPY_BYTES: &[u8] = include_bytes!("../assets/copy.txt");
// Byte-identical to desktop/public/earth/still.png (SHA-256
// 1826f0bd821a838717f4cd8756d3fa50c85ada38395f8a8bfaa638e0620cfe18, Natural
// Earth public domain; provenance in desktop/public/THIRD_PARTY_NOTICES.txt).
const EARTH_BYTES: &[u8] = include_bytes!("../assets/earth-still.png");
const WINDOW_TITLE: &str = "DevSweep";

const CANVAS: u32 = 0x11162a;
const TEXT: u32 = 0xeef1f7;
const CAPSULE: u32 = 0x1b2138;
const CAPSULE_BORDER: u32 = 0x313a5a;
const PILL: u32 = 0xeef1f7;
const PILL_TEXT: u32 = 0x12172a;
const MENU_BG: u32 = 0x1a2035;

const LIGHT_CANVAS: u32 = 0xf3f5fa;
const LIGHT_TEXT: u32 = 0x192238;
const LIGHT_CAPSULE: u32 = 0xe4e9f2;
const LIGHT_CAPSULE_BORDER: u32 = 0xb8c2d4;
const LIGHT_PILL: u32 = 0x263750;
const LIGHT_PILL_TEXT: u32 = 0xffffff;
const LIGHT_MENU_BG: u32 = 0xffffff;

const HIGH_CONTRAST_CANVAS: u32 = 0x000000;
const HIGH_CONTRAST_TEXT: u32 = 0xffffff;
const HIGH_CONTRAST_CAPSULE: u32 = 0x000000;
const HIGH_CONTRAST_CAPSULE_BORDER: u32 = 0xffffff;
const HIGH_CONTRAST_PILL: u32 = 0xffffff;
const HIGH_CONTRAST_PILL_TEXT: u32 = 0x000000;
const HIGH_CONTRAST_MENU_BG: u32 = 0x000000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ShellPalette {
    canvas: u32,
    text: u32,
    capsule: u32,
    capsule_border: u32,
    pill: u32,
    pill_text: u32,
    menu_bg: u32,
}

const DARK_PALETTE: ShellPalette = ShellPalette {
    canvas: CANVAS,
    text: TEXT,
    capsule: CAPSULE,
    capsule_border: CAPSULE_BORDER,
    pill: PILL,
    pill_text: PILL_TEXT,
    menu_bg: MENU_BG,
};

const LIGHT_PALETTE: ShellPalette = ShellPalette {
    canvas: LIGHT_CANVAS,
    text: LIGHT_TEXT,
    capsule: LIGHT_CAPSULE,
    capsule_border: LIGHT_CAPSULE_BORDER,
    pill: LIGHT_PILL,
    pill_text: LIGHT_PILL_TEXT,
    menu_bg: LIGHT_MENU_BG,
};

const HIGH_CONTRAST_PALETTE: ShellPalette = ShellPalette {
    canvas: HIGH_CONTRAST_CANVAS,
    text: HIGH_CONTRAST_TEXT,
    capsule: HIGH_CONTRAST_CAPSULE,
    capsule_border: HIGH_CONTRAST_CAPSULE_BORDER,
    pill: HIGH_CONTRAST_PILL,
    pill_text: HIGH_CONTRAST_PILL_TEXT,
    menu_bg: HIGH_CONTRAST_MENU_BG,
};

fn shell_palette(theme: DesktopTheme) -> ShellPalette {
    match theme {
        DesktopTheme::Light => LIGHT_PALETTE,
        DesktopTheme::Dark
        | DesktopTheme::System
        | DesktopTheme::CatppuccinLatte
        | DesktopTheme::CatppuccinMocha
        | DesktopTheme::Codex
        | DesktopTheme::Claude => DARK_PALETTE,
    }
}

fn shell_palette_for(theme: DesktopTheme, high_contrast: bool) -> ShellPalette {
    if high_contrast {
        HIGH_CONTRAST_PALETTE
    } else {
        shell_palette(theme)
    }
}

fn system_high_contrast() -> bool {
    #[cfg(windows)]
    {
        windows_high_contrast_on()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(windows)]
fn windows_high_contrast_on() -> bool {
    let mut contrast = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    // SPI_GETHIGHCONTRAST only. Do not call SPI_SETHIGHCONTRAST or pass update flags.
    let queried = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some((&raw mut contrast).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
        )
    };
    queried.is_ok() && contrast.dwFlags.contains(HCF_HIGHCONTRASTON)
}

actions!(
    shell_view,
    [
        ModePrev,
        ModeNext,
        MenuUp,
        MenuDown,
        RowDown,
        RowUp,
        Back,
        Activate,
        MenuToggle,
        RequestProjectRows,
        ToggleRow,
        CleanPageUp,
        CleanPageDown,
        TabCleanReview,
        ShiftTabCleanReview,
        CycleCleanSort,
        OpenAiReview,
        ToggleCleanScreen,
        ToggleKeepScreen
    ]
);

pub struct PackagedAssets;

impl AssetSource for PackagedAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        match path {
            ICON_NAME => Ok(Some(Cow::Borrowed(ICON_BYTES))),
            COPY_NAME => Ok(Some(Cow::Borrowed(COPY_BYTES))),
            EARTH_NAME => Ok(Some(Cow::Borrowed(EARTH_BYTES))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(vec![
            SharedString::from(ICON_NAME),
            SharedString::from(COPY_NAME),
            SharedString::from(EARTH_NAME),
        ])
    }
}

pub struct ShellView {
    focus_handle: FocusHandle,
    session: Session,
    save_error: Option<String>,
    note: Entity<TextInput>,
    search: Entity<TextInput>,
}

/// Route a resolved mode action to the session.
///
/// The shell root owns the `ModePrev`/`ModeNext` listeners, and it sits on the
/// Note's action dispatch path, so the Note's `alt-left`/`alt-right` chord
/// arrives here through the same bubbling path `down`/`up` already use. Keeping
/// the decision on the action (not the keystroke) is what lets a test drive a
/// resolved keystroke through the real handler.
fn route_mode_action(action: &dyn Action, session: &mut Session) -> bool {
    if action.partial_eq(&ModePrev) {
        session.apply_key(crate::shell::NavKey::Left);
    } else if action.partial_eq(&ModeNext) {
        session.apply_key(crate::shell::NavKey::Right);
    } else {
        return false;
    }
    true
}

impl ShellView {
    fn prev(&mut self, action: &ModePrev, _: &mut Window, cx: &mut Context<Self>) {
        route_mode_action(action, &mut self.session);
        cx.notify();
    }

    fn next(&mut self, action: &ModeNext, _: &mut Window, cx: &mut Context<Self>) {
        route_mode_action(action, &mut self.session);
        cx.notify();
    }

    fn up(&mut self, _: &MenuUp, _: &mut Window, cx: &mut Context<Self>) {
        self.session.apply_key(crate::shell::NavKey::Up);
        cx.notify();
    }

    fn down(&mut self, _: &MenuDown, _: &mut Window, cx: &mut Context<Self>) {
        self.session.apply_key(crate::shell::NavKey::Down);
        cx.notify();
    }

    fn row_down(&mut self, _: &RowDown, _: &mut Window, cx: &mut Context<Self>) {
        self.session.apply_key(crate::shell::NavKey::RowDown);
        cx.notify();
    }

    fn row_up(&mut self, _: &RowUp, _: &mut Window, cx: &mut Context<Self>) {
        self.session.apply_key(crate::shell::NavKey::RowUp);
        cx.notify();
    }

    /// Space toggles the focused Clean row only while review is active.
    ///
    /// The Note's own context receives the key. Propagate when the search
    /// field owns the caret, or when review is not active, so the character
    /// stays text. A real toggle does not propagate, so the Note does not
    /// gain a space.
    fn toggle_row(&mut self, _: &ToggleRow, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.focus_handle(cx).is_focused(window) {
            cx.propagate();
            return;
        }
        if !self.clean_review_active() {
            cx.propagate();
            return;
        }
        self.session.clean_toggle_focused_row();
        cx.notify();
    }

    /// Tab moves between the Note and the Clean search field.
    ///
    /// The search field is focused only while review is active and the Note
    /// already has focus. Tab from the search field returns to the Note.
    /// Startup and a non-review surface never move focus here.
    fn tab_clean_review(
        &mut self,
        _: &TabCleanReview,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.clean_review_active() {
            cx.propagate();
            return;
        }
        if self.search.focus_handle(cx).is_focused(window) {
            window.focus(&self.note.focus_handle(cx), cx);
            return;
        }
        if self.note.focus_handle(cx).is_focused(window) {
            window.focus(&self.search.focus_handle(cx), cx);
            return;
        }
        cx.propagate();
    }

    /// Shift-Tab from the search field returns to the Note during review.
    /// It does not focus the search field, and it does not move focus outside
    /// review.
    fn shift_tab_clean_review(
        &mut self,
        _: &ShiftTabCleanReview,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.clean_review_active() && self.search.focus_handle(cx).is_focused(window) {
            window.focus(&self.note.focus_handle(cx), cx);
            return;
        }
        cx.propagate();
    }

    /// F7 cycles Clean sort in the Found phase only. It does not change mode.
    fn cycle_clean_sort(&mut self, _: &CycleCleanSort, _: &mut Window, cx: &mut Context<Self>) {
        match self.session.shell().mode() {
            Mode::Software => self.session.software_cycle_sort(),
            Mode::Status => self.session.status_cycle_sort(),
            _ => {
                self.session.clean_cycle_sort();
            }
        }
        cx.notify();
    }

    /// Clean mode on the mode surface, with a Found report or the AI review.
    fn clean_review_active(&self) -> bool {
        clean_review_is_active(&self.session)
    }

    fn clean_page_up(&mut self, _: &CleanPageUp, _: &mut Window, cx: &mut Context<Self>) {
        self.session
            .clean_scroll_rows(-(crate::shell::VISIBLE_ROW_CAPACITY as isize));
        cx.notify();
    }

    fn clean_page_down(&mut self, _: &CleanPageDown, _: &mut Window, cx: &mut Context<Self>) {
        self.session
            .clean_scroll_rows(crate::shell::VISIBLE_ROW_CAPACITY as isize);
        cx.notify();
    }

    fn back(&mut self, _: &Back, _: &mut Window, cx: &mut Context<Self>) {
        // Clean Screen must restore before any other Escape handling, and it
        // must not lock the Note.
        if self.session.screen_covered() {
            self.session.apply_key(crate::shell::NavKey::Escape);
            cx.notify();
            return;
        }
        // Escape first stops an active worker, then leaves the AI review
        // subview, and only then is the surface back key.
        if !self.session.clean_request_cancel() && !self.session.clean_back() {
            self.session.apply_key(crate::shell::NavKey::Escape);
        }
        cx.notify();
    }

    fn activate(&mut self, _: &Activate, _: &mut Window, cx: &mut Context<Self>) {
        let before_operation = self.session.shell().operation_id();
        self.session.apply_key(crate::shell::NavKey::Enter);
        let started_worker = self.session.shell().operation_id() != before_operation
            && self.session.shell().activity() == Activity::Busy;
        self.after_clean_action(started_worker, cx);
    }

    fn menu_toggle(&mut self, _: &MenuToggle, _: &mut Window, cx: &mut Context<Self>) {
        self.session.toggle_menu();
        cx.notify();
    }

    /// F8 opens the AI review subview. Opening does not scan.
    fn toggle_clean_screen(
        &mut self,
        _: &ToggleCleanScreen,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session.screen_covered() {
            self.session.apply_key(crate::shell::NavKey::Escape);
        } else {
            self.session.begin_clean_screen();
        }
        cx.notify();
    }

    fn toggle_keep_screen(
        &mut self,
        _: &ToggleKeepScreen,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session.stored_lease_count() > 0 {
            let _ = self.session.stop_screen_on();
        } else {
            let service = std::sync::Arc::new(
                devsweep_core::status::KeepAwakeService::production(),
            );
            let _ = self.session.start_screen_on(
                service,
                "gpui-window",
                std::time::Duration::from_secs(30 * 60),
            );
        }
        cx.notify();
    }

    fn open_ai_review(&mut self, _: &OpenAiReview, _: &mut Window, cx: &mut Context<Self>) {
        if self.session.shell().menu_open()
            || self.session.shell().surface() != Surface::Mode
            || self.session.shell().mode() != Mode::Clean
        {
            return;
        }
        self.session.clean_ai_open();
        cx.notify();
    }

    fn request_project_rows(
        &mut self,
        _: &RequestProjectRows,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let started = self.session.request_project_rows();
        self.after_clean_action(started, cx);
    }

    fn clean_open_scope(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.session.clean_open_scope_choice();
        cx.notify();
    }

    fn clean_scope(&mut self, scope: CleanScope, cx: &mut Context<Self>) {
        let started = self.session.clean_choose_scope(scope);
        self.after_clean_action(started, cx);
    }

    fn clean_confirm_global(&mut self, cx: &mut Context<Self>) {
        let started = self.session.clean_confirm_global_scan();
        self.after_clean_action(started, cx);
    }

    fn clean_cancel(&mut self, cx: &mut Context<Self>) {
        self.session.clean_request_cancel();
        cx.notify();
    }

    fn clean_pick_root(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            let picked = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let Some(root) = picked else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                let started = this.session.clean_pick_scan_root(root);
                this.after_clean_action(started, cx);
            });
        })
        .detach();
    }

    fn clean_pick_suggestion(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        let started = self.session.clean_pick_scan_root(root);
        self.after_clean_action(started, cx);
    }

    fn clean_retry(&mut self, cx: &mut Context<Self>) {
        let started = self.session.clean_rescan();
        self.after_clean_action(started, cx);
    }

    fn clean_preview(&mut self, cx: &mut Context<Self>) {
        self.session.clean_preview_selection();
        cx.notify();
    }

    fn clean_confirm_execute(&mut self, cx: &mut Context<Self>) {
        let started = self.session.clean_confirm_execute();
        self.after_clean_action(started, cx);
    }

    fn clean_sort(&mut self, cx: &mut Context<Self>) {
        self.session.clean_cycle_sort();
        cx.notify();
    }

    fn clean_select_recommended(&mut self, cx: &mut Context<Self>) {
        self.session.clean_select_recommended();
        cx.notify();
    }

    fn clean_clear_selection(&mut self, cx: &mut Context<Self>) {
        self.session.clean_clear_selected();
        cx.notify();
    }

    fn clean_ai_open(&mut self, cx: &mut Context<Self>) {
        self.session.clean_ai_open();
        cx.notify();
    }

    fn clean_ai_close(&mut self, cx: &mut Context<Self>) {
        self.session.clean_ai_close();
        cx.notify();
    }

    fn clean_ai_provider(&mut self, cx: &mut Context<Self>) {
        self.session.clean_ai_cycle_provider();
        cx.notify();
    }

    fn clean_ai_category(&mut self, category: AiCategory, cx: &mut Context<Self>) {
        self.session.clean_ai_set_category(category);
        cx.notify();
    }

    fn clean_ai_scan(&mut self, cx: &mut Context<Self>) {
        let started = self.session.clean_ai_scan();
        self.after_clean_action(started, cx);
    }

    fn clean_ai_preview(&mut self, cx: &mut Context<Self>) {
        self.session.clean_ai_preview();
        cx.notify();
    }

    fn clean_ai_confirm_execute(&mut self, cx: &mut Context<Self>) {
        let started = self.session.clean_ai_confirm_execute();
        self.after_clean_action(started, cx);
    }

    fn clean_ai_clear_selection(&mut self, cx: &mut Context<Self>) {
        self.session.clean_ai_clear_selected();
        cx.notify();
    }

    fn clean_ai_refresh_recovery(&mut self, cx: &mut Context<Self>) {
        let started = self.session.clean_ai_refresh_recovery();
        self.after_clean_action(started, cx);
    }

    fn clean_ai_restore(&mut self, bundle_id: String, cx: &mut Context<Self>) {
        let started = self.session.clean_ai_restore_plan(bundle_id);
        self.after_clean_action(started, cx);
    }

    /// Starts work or a prompt change and, when a worker started, drains its
    /// mailbox until the shell reports idle again.
    fn after_clean_action(&mut self, started: bool, cx: &mut Context<Self>) {
        if started {
            self.start_clean_poll(cx);
        } else {
            cx.notify();
        }
    }

    fn start_clean_poll(&mut self, cx: &mut Context<Self>) {
        cx.notify();
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            loop {
                let stop = this
                    .update(cx, |this, cx| {
                        let poll = this.session.drain_mail();
                        if poll.applied > 0 {
                            cx.notify();
                        }
                        this.session.shell().activity() == Activity::Idle
                            || started.elapsed() >= Duration::from_secs(30 * 60)
                    })
                    .unwrap_or(true);
                if stop {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
        })
        .detach();
    }

    fn save(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.save_error = match self.session.save_settings() {
            Ok(_) => None,
            Err(error) => Some(error.to_string()),
        };
        cx.notify();
    }

    /// Keep the Note at the app text scale. The setter notifies only on a real
    /// change, so this render-time sync settles after one pass.
    fn sync_note_text_scale(&self, cx: &mut Context<Self>) {
        let percent = self.session.text_scale();
        self.note
            .update(cx, |note, cx| note.set_text_scale_percent(percent, cx));
        let locale = self.session.locale();
        self.search.update(cx, |search, cx| {
            search.set_text_scale_percent(percent, cx);
            search.set_placeholder(copy::clean_search_placeholder(locale), cx);
        });
    }
}

/// True when Tab and Space should operate on the Clean review.
///
/// A Found report counts only while the AI subview is closed, because opening
/// that subview replaces the search field. An open subview counts only in its
/// Review phase. Landing, scanning, and other phases leave the Note's keys
/// as text.
pub(crate) fn clean_review_is_active(session: &Session) -> bool {
    if session.shell().surface() != Surface::Mode || session.shell().mode() != Mode::Clean {
        return false;
    }
    if session.clean_ai().open() {
        return session.clean_ai().phase() == AiSubviewPhase::Review;
    }
    session.clean_workbench().phase() == CleanPhase::Found
}

impl Focusable for ShellView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ShellView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_note_text_scale(cx);
        // The beaver PNG and the Earth still are single frames. Do not request
        // a decorative animation frame.
        let _ = self.session.shell().decorative_deadline();
        let locale = self.session.locale();
        let (menu_open, selected, surface, menu_index) = {
            let shell = self.session.shell();
            (
                shell.menu_open(),
                shell.mode(),
                shell.surface(),
                shell.menu_index(),
            )
        };
        let text_size = px(16.0 * f32::from(self.session.text_scale()) / 100.0);
        let high_contrast = system_high_contrast();
        let palette = shell_palette_for(self.session.theme(), high_contrast);
        if selected == Mode::Analyze {
            self.session.prepare_analyze_landing();
        }
        let status_main = selected == Mode::Status && surface == Surface::Mode;
        self.session.status_set_main_visible(status_main);
        let show_map = disk_map_active(&self.session);
        let review_surface = self.clean_review_active();
        let key_context = if menu_open {
            "ShellMenu"
        } else if review_surface {
            "ShellRoot CleanReview"
        } else {
            "ShellRoot"
        };
        let tabs = Mode::ORDER
            .into_iter()
            .map(|mode| {
                let access = accessible_control(locale, AccessibleSlot::Mode(mode));
                let active = mode == selected && surface == Surface::Mode;
                div()
                    .id(mode.element_id())
                    .role(gpui::Role::Tab)
                    .accessibility_id(access.id)
                    .aria_label(access.label)
                    .aria_selected(active)
                    .when(active, |tab| tab.aria_active_descendant())
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .when(active, |tab| {
                        tab.bg(rgb(palette.pill)).text_color(rgb(palette.pill_text))
                    })
                    .when(!active, |tab| tab.text_color(rgb(palette.text)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session.show_mode(mode);
                        cx.notify();
                    }))
                    .child(access.label)
            })
            .collect::<Vec<_>>();
        let menu_items = MENU
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let access = accessible_control(locale, AccessibleSlot::Menu(item));
                div()
                    .id(surface_element_id(item))
                    .role(gpui::Role::MenuItem)
                    .accessibility_id(access.id)
                    .aria_label(access.label)
                    .px_3()
                    .py_2()
                    .cursor_pointer()
                    .when(menu_index == index, |row| {
                        row.bg(rgb(palette.pill)).text_color(rgb(palette.pill_text))
                    })
                    .when(menu_index != index, |row| row.text_color(rgb(palette.text)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session.open_menu_item(index);
                        cx.notify();
                    }))
                    .child(access.label)
            })
            .collect::<Vec<_>>();
        let entries = panel_entries(&self.session, self.save_error.as_deref());
        let shell_access = accessible_control(locale, AccessibleSlot::Shell);
        let controls = self.clean_controls(locale, palette, cx);
        let disk_map =
            show_map.then(|| disk_map_element(&self.session, locale, palette, high_contrast, cx));

        div()
            .id("shell-root")
            .role(gpui::Role::Application)
            .accessibility_id(shell_access.id)
            .aria_label(shell_access.label)
            .track_focus(&self.focus_handle)
            .key_context(key_context)
            .on_action(cx.listener(Self::prev))
            .on_action(cx.listener(Self::next))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::row_down))
            .on_action(cx.listener(Self::row_up))
            .on_action(cx.listener(Self::toggle_row))
            .on_action(cx.listener(Self::tab_clean_review))
            .on_action(cx.listener(Self::shift_tab_clean_review))
            .on_action(cx.listener(Self::cycle_clean_sort))
            .on_action(cx.listener(Self::clean_page_up))
            .on_action(cx.listener(Self::clean_page_down))
            .on_action(cx.listener(Self::back))
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::menu_toggle))
            .on_action(cx.listener(Self::open_ai_review))
            .on_action(cx.listener(Self::toggle_clean_screen))
            .on_action(cx.listener(Self::toggle_keep_screen))
            .on_action(cx.listener(Self::request_project_rows))
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .p_4()
            .gap_3()
            .bg(rgb(palette.canvas))
            .text_color(rgb(palette.text))
            .text_size(text_size)
            .child(
                div()
                    .id("capsule")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .bg(rgb(palette.capsule))
                    .border_1()
                    .border_color(rgb(palette.capsule_border))
                    .rounded_md()
                    .child(
                        div()
                            .id("beaver-menu")
                            .role(gpui::Role::Button)
                            .aria_label(copy::menu_label(locale))
                            .aria_expanded(menu_open)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.session.toggle_menu();
                                cx.notify();
                            }))
                            .child(
                                img(ICON_NAME)
                                    .id("beaver-icon")
                                    .size(px(28.))
                                    .with_fallback(|| div().child("DevSweep").into_any_element()),
                            ),
                    )
                    .child(
                        div()
                            .id("mode-tabs")
                            .role(gpui::Role::TabList)
                            .aria_label(copy::modes_label(locale))
                            .flex()
                            .flex_row()
                            .gap_3()
                            .children(tabs),
                    ),
            )
            .when(menu_open, move |root| {
                root.child(
                    div()
                        .id("brand-menu")
                        .role(gpui::Role::Menu)
                        .aria_label(copy::menu_label(locale))
                        .flex()
                        .flex_col()
                        .bg(rgb(palette.menu_bg))
                        .border_1()
                        .border_color(rgb(palette.capsule_border))
                        .rounded_md()
                        .children(menu_items),
                )
            })
            .child(self.utility_row(locale, palette, cx))
            .children(controls)
            .child(
                div()
                    .id("mode-panel")
                    .role(gpui::Role::TabPanel)
                    .aria_label(panel_title(&self.session))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .when_some(disk_map, |panel, map| panel.child(map))
                    .when_some(
                        analyze_treemap_element(&self.session, locale, palette),
                        |panel, map| panel.child(map),
                    )
                    .children(
                        entries
                            .into_iter()
                            .enumerate()
                            .filter_map(|(index, entry)| {
                                match entry {
                                    PanelEntry::Text(line) => Some(
                                        div()
                                            .id(gpui::ElementId::named_usize("panel-line", index))
                                            .role(gpui::Role::ListItem)
                                            .aria_label(SharedString::from(line.clone()))
                                            .child(SharedString::from(line))
                                            .into_any_element(),
                                    ),
                                    // The disk map paints these rows in its recommendation column.
                                    PanelEntry::CleanRow { .. } if show_map => None,
                                    PanelEntry::CleanRow { id, text, selected } => {
                                        Some(interactive_row_element(
                                            "clean-row",
                                            id,
                                            text,
                                            selected,
                                            cx,
                                        ))
                                    }
                                    PanelEntry::AiRow { id, text, selected } => Some(
                                        interactive_row_element("ai-row", id, text, selected, cx),
                                    ),
                                }
                            }),
                    ),
            )
            .child(self.note.clone())
            .when(surface == Surface::Settings, |root| {
                root.child(
                    div()
                        .id("settings-actions")
                        .flex()
                        .flex_row()
                        .gap_3()
                        .child(
                            div()
                                .id("settings-language")
                                .role(gpui::Role::Button)
                                .aria_label(copy::language_label(locale))
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(rgb(palette.capsule))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.session.cycle_language();
                                    cx.notify();
                                }))
                                .child(copy::language_label(locale)),
                        )
                        .child(
                            div()
                                .id("settings-theme")
                                .role(gpui::Role::Button)
                                .aria_label(copy::theme_label(locale))
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(rgb(palette.capsule))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.session.cycle_theme();
                                    cx.notify();
                                }))
                                .child(copy::theme_label(locale)),
                        )
                        .child(
                            div()
                                .id("settings-save")
                                .role(gpui::Role::Button)
                                .aria_label(copy::save_label(locale))
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(rgb(palette.pill))
                                .text_color(rgb(palette.pill_text))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)))
                                .child(copy::save_label(locale)),
                        ),
                )
            })
            .when(surface != Surface::Mode, |root| {
                root.child(
                    div()
                        .id("surface-back")
                        .role(gpui::Role::Button)
                        .aria_label(copy::back_label(locale))
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.apply_key(crate::shell::NavKey::Escape);
                            cx.notify();
                        }))
                        .child(copy::back_label(locale)),
                )
            })
    }
}

impl ShellView {
    fn utility_row(
        &self,
        locale: copy::Locale,
        palette: ShellPalette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let button = |id: &'static str, label: &'static str, action: fn(&mut ShellView, &mut Context<ShellView>)| {
            div()
                .id(id)
                .role(gpui::Role::Button)
                .aria_label(label)
                .px_3()
                .py_2()
                .rounded_md()
                .bg(rgb(palette.capsule))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    action(this, cx);
                    cx.notify();
                }))
                .child(label)
        };
        div()
            .id("utility-row")
            .flex()
            .flex_row()
            .gap_3()
            .flex_wrap()
            .child(button(
                "keep-screen-on",
                copy::keep_screen_on_label(locale),
                |this, _| {
                    if this.session.stored_lease_count() > 0 {
                        let _ = this.session.stop_screen_on();
                    } else {
                        let service = std::sync::Arc::new(
                            devsweep_core::status::KeepAwakeService::production(),
                        );
                        let _ = this.session.start_screen_on(
                            service,
                            "gpui-window",
                            std::time::Duration::from_secs(30 * 60),
                        );
                    }
                },
            ))
            .child(button(
                "clean-screen",
                copy::clean_screen_label(locale),
                |this, _| {
                    if this.session.screen_covered() {
                        this.session.apply_key(crate::shell::NavKey::Escape);
                    } else {
                        this.session.begin_clean_screen();
                    }
                },
            ))
            .child(button(
                "software-refresh",
                copy::status_action_snapshot(locale),
                |this, _| {
                    if this.session.shell().mode() == Mode::Software {
                        this.session.software_refresh();
                    } else if this.session.shell().mode() == Mode::Status {
                        this.session.apply_key(crate::shell::NavKey::Enter);
                    }
                },
            ))
            .child(button(
                "status-live",
                if self.session.status_phase() == crate::status_panel::StatusPhase::Live {
                    copy::status_action_live_stop(locale)
                } else {
                    copy::status_action_live_start(locale)
                },
                |this, cx| {
                    if this.session.status_phase() == crate::status_panel::StatusPhase::Live {
                        this.session.status_stop_live();
                    } else if this.session.status_start_live().is_ok() {
                        this.start_status_poll(cx);
                    }
                },
            ))
            .into_any_element()
    }

    fn start_status_poll(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                let stop = this
                    .update(cx, |this, cx| {
                        let visible = this.session.shell().mode() == Mode::Status
                            && this.session.shell().surface() == Surface::Mode;
                        if this.session.poll_status_live() && visible {
                            cx.notify();
                        }
                        this.session.status_phase() != crate::status_panel::StatusPhase::Live
                    })
                    .unwrap_or(true);
                if stop {
                    break;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(250))
                    .await;
            }
        })
        .detach();
    }

    /// Clean controls for the active phase. Clean mode only.
    fn clean_controls(
        &self,
        locale: copy::Locale,
        palette: ShellPalette,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if self.session.shell().mode() != Mode::Clean
            || self.session.shell().surface() != Surface::Mode
        {
            return None;
        }
        if self.session.clean_ai().open() {
            return Some(self.clean_ai_controls(locale, palette, cx));
        }
        let workbench = self.session.clean_workbench();
        let mut controls = div()
            .id("clean-controls")
            .role(gpui::Role::Group)
            .aria_label(copy::mode_name(locale, "clean"))
            .flex()
            .flex_col()
            .items_start()
            .gap_3();
        let mut actions = div()
            .id("clean-actions")
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_3();
        match workbench.phase() {
            CleanPhase::Landing => {
                controls = controls.child(
                    img(EARTH_NAME)
                        .id("clean-earth")
                        .w(px(180.))
                        .h(px(180.))
                        .with_fallback(|| {
                            div().child(SharedString::from("Earth")).into_any_element()
                        }),
                );
                match workbench.prompt() {
                    CleanLandingPrompt::None => {
                        actions = actions.child(clean_button(
                            "clean-scan-primary",
                            copy::clean_scan_primary(locale).to_string(),
                            true,
                            palette,
                            cx,
                            |this, window, cx| this.clean_open_scope(window, cx),
                        ));
                        actions = actions.child(clean_ai_open_button(locale, palette, cx));
                    }
                    CleanLandingPrompt::ScopeChoice => {
                        // The two scope controls derive their element ids from
                        // the scope identity itself.
                        for scope in [CleanScope::Project, CleanScope::Global] {
                            let label = match scope {
                                CleanScope::Project => copy::clean_scope_project(locale),
                                CleanScope::Global => copy::clean_scope_global(locale),
                            };
                            let filled = scope == CleanScope::Project;
                            actions = actions.child(clean_button(
                                ElementId::Name(SharedString::from(format!(
                                    "clean-scope-{}",
                                    scope.id()
                                ))),
                                label.to_string(),
                                filled,
                                palette,
                                cx,
                                move |this, _, cx| this.clean_scope(scope, cx),
                            ));
                        }
                        actions = actions.child(clean_button(
                            "clean-scope-back",
                            copy::back_label(locale).to_string(),
                            false,
                            palette,
                            cx,
                            |this, _, cx| {
                                this.session.clean_cancel_prompt();
                                cx.notify();
                            },
                        ));
                    }
                    CleanLandingPrompt::RootRequired => {
                        for (index, suggestion) in workbench.suggestions().iter().enumerate() {
                            let label = suggestion.display().to_string();
                            let root = suggestion.clone();
                            actions = actions.child(clean_button(
                                ElementId::Name(SharedString::from(format!(
                                    "clean-root-suggestion-{index}"
                                ))),
                                label,
                                false,
                                palette,
                                cx,
                                move |this, _, cx| this.clean_pick_suggestion(root.clone(), cx),
                            ));
                        }
                        actions = actions.child(clean_button(
                            "clean-root-choose",
                            copy::clean_root_choose(locale).to_string(),
                            true,
                            palette,
                            cx,
                            |this, _, cx| this.clean_pick_root(cx),
                        ));
                        actions = actions.child(clean_button(
                            "clean-root-back",
                            copy::back_label(locale).to_string(),
                            false,
                            palette,
                            cx,
                            |this, _, cx| {
                                this.session.clean_cancel_prompt();
                                cx.notify();
                            },
                        ));
                    }
                    CleanLandingPrompt::ConfirmGlobal => {
                        actions = actions.child(clean_button(
                            "clean-global-confirm",
                            copy::clean_global_scan_action(locale).to_string(),
                            true,
                            palette,
                            cx,
                            |this, _, cx| this.clean_confirm_global(cx),
                        ));
                        actions = actions.child(clean_button(
                            "clean-global-back",
                            copy::back_label(locale).to_string(),
                            false,
                            palette,
                            cx,
                            |this, _, cx| {
                                this.session.clean_cancel_prompt();
                                cx.notify();
                            },
                        ));
                    }
                }
            }
            CleanPhase::Scanning | CleanPhase::Cleaning => {
                let phase_id = workbench.phase().id();
                actions = actions.child(clean_button(
                    ElementId::Name(SharedString::from(format!("clean-cancel-{phase_id}"))),
                    copy::clean_cancel_action(locale).to_string(),
                    false,
                    palette,
                    cx,
                    |this, _, cx| this.clean_cancel(cx),
                ));
            }
            CleanPhase::Found => {
                controls = controls.child(self.search.clone());
                let sort_label = match workbench.sort() {
                    CleanSort::ReportOrder => copy::clean_sort_report_order(locale),
                    CleanSort::Name => copy::clean_sort_name(locale),
                    CleanSort::Size => copy::clean_sort_size(locale),
                };
                actions = actions.child(clean_button(
                    "clean-sort",
                    sort_label.to_string(),
                    false,
                    palette,
                    cx,
                    |this, _, cx| this.clean_sort(cx),
                ));
                actions = actions.child(clean_button(
                    "clean-select-recommended",
                    copy::clean_select_recommended(locale).to_string(),
                    false,
                    palette,
                    cx,
                    |this, _, cx| this.clean_select_recommended(cx),
                ));
                actions = actions.child(clean_button(
                    "clean-clear-selection",
                    copy::clean_clear_selection(locale).to_string(),
                    false,
                    palette,
                    cx,
                    |this, _, cx| this.clean_clear_selection(cx),
                ));
                actions = actions.child(clean_button(
                    "clean-preview",
                    copy::clean_preview_action(locale).to_string(),
                    false,
                    palette,
                    cx,
                    |this, _, cx| this.clean_preview(cx),
                ));
                actions = actions.child(clean_button(
                    "clean-confirm",
                    format!(
                        "{} · {}",
                        copy::clean_confirm_action(locale),
                        copy::clean_selection_count(
                            locale,
                            workbench.selected_count(),
                            workbench.rows().len()
                        )
                    ),
                    workbench.preview_matches_current(),
                    palette,
                    cx,
                    |this, _, cx| this.clean_confirm_execute(cx),
                ));
                actions = actions.child(clean_ai_open_button(locale, palette, cx));
                if let Some(cursor) = self.session.clean_focused_row_id() {
                    let protected = workbench.is_protected(&cursor);
                    let protect_label = if protected {
                        copy::clean_unprotect_action(locale)
                    } else {
                        copy::clean_protect_action(locale)
                    };
                    let protect_id = cursor.clone();
                    actions = actions.child(clean_button(
                        "clean-protect-row",
                        protect_label.to_string(),
                        protected,
                        palette,
                        cx,
                        move |this, _, cx| {
                            this.session.clean_toggle_row_protection(&protect_id);
                            cx.notify();
                        },
                    ));
                    let ignored = workbench.is_ignored(&cursor);
                    let ignore_label = if ignored {
                        copy::clean_unignore_action(locale)
                    } else {
                        copy::clean_ignore_action(locale)
                    };
                    let ignore_id = cursor.clone();
                    actions = actions.child(clean_button(
                        "clean-ignore-row",
                        ignore_label.to_string(),
                        ignored,
                        palette,
                        cx,
                        move |this, _, cx| {
                            this.session.clean_toggle_row_ignored(&ignore_id);
                            cx.notify();
                        },
                    ));
                }
            }
            CleanPhase::AllClean | CleanPhase::Completed | CleanPhase::Failed => {
                actions = actions.child(clean_button(
                    "clean-retry",
                    copy::clean_retry_action(locale).to_string(),
                    true,
                    palette,
                    cx,
                    |this, _, cx| this.clean_retry(cx),
                ));
                actions = actions.child(clean_ai_open_button(locale, palette, cx));
            }
        }
        Some(controls.child(actions).into_any_element())
    }

    /// AI subview controls. The subview replaces the Clean controls while it
    /// is open; every action here is an explicit user action.
    fn clean_ai_controls(
        &self,
        locale: copy::Locale,
        palette: ShellPalette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let view = self.session.clean_ai();
        let mut controls = div()
            .id("clean-ai-controls")
            .role(gpui::Role::Group)
            .aria_label(copy::clean_ai_title(locale))
            .flex()
            .flex_col()
            .items_start()
            .gap_3();
        let mut actions = div()
            .id("clean-ai-actions")
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_3();
        if !matches!(
            view.phase(),
            AiSubviewPhase::Scanning | AiSubviewPhase::Cleaning
        ) {
            actions = actions.child(clean_button(
                "clean-ai-provider",
                copy::clean_ai_provider(locale, view.provider().as_str()),
                false,
                palette,
                cx,
                |this, _, cx| this.clean_ai_provider(cx),
            ));
        }
        match view.phase() {
            AiSubviewPhase::Landing | AiSubviewPhase::Failed | AiSubviewPhase::Completed => {
                actions = actions.child(clean_button(
                    "clean-ai-scan",
                    copy::clean_ai_scan_action(locale).to_string(),
                    view.phase() == AiSubviewPhase::Landing,
                    palette,
                    cx,
                    |this, _, cx| this.clean_ai_scan(cx),
                ));
            }
            AiSubviewPhase::Scanning | AiSubviewPhase::Cleaning => {
                let phase_id = view.phase().id();
                actions = actions.child(clean_button(
                    ElementId::Name(SharedString::from(format!("clean-ai-cancel-{phase_id}"))),
                    copy::clean_cancel_action(locale).to_string(),
                    false,
                    palette,
                    cx,
                    |this, _, cx| this.clean_cancel(cx),
                ));
            }
            AiSubviewPhase::Review => {
                controls = controls.child(self.search.clone());
                for category in AiCategory::ALL {
                    let label = format!(
                        "{} {}",
                        copy::clean_ai_category_name(locale, category.id()),
                        view.category_count(category)
                    );
                    let active = view.category() == category;
                    actions = actions.child(clean_button(
                        ElementId::Name(SharedString::from(format!(
                            "clean-ai-category-{}",
                            category.id()
                        ))),
                        label,
                        active,
                        palette,
                        cx,
                        move |this, _, cx| this.clean_ai_category(category, cx),
                    ));
                }
                actions = actions.child(clean_button(
                    "clean-ai-clear-selection",
                    copy::clean_clear_selection(locale).to_string(),
                    false,
                    palette,
                    cx,
                    |this, _, cx| this.clean_ai_clear_selection(cx),
                ));
                actions = actions.child(clean_button(
                    "clean-ai-preview",
                    copy::clean_preview_action(locale).to_string(),
                    false,
                    palette,
                    cx,
                    |this, _, cx| this.clean_ai_preview(cx),
                ));
                actions = actions.child(clean_button(
                    "clean-ai-confirm",
                    format!(
                        "{} · {}",
                        copy::clean_ai_confirm_action(locale),
                        copy::clean_ai_selection_count(
                            locale,
                            view.selected_count(),
                            view.selectable_count()
                        )
                    ),
                    view.preview_matches_current(),
                    palette,
                    cx,
                    |this, _, cx| this.clean_ai_confirm_execute(cx),
                ));
            }
        }
        if view.home().is_some()
            && !matches!(
                view.phase(),
                AiSubviewPhase::Scanning | AiSubviewPhase::Cleaning
            )
        {
            actions = actions.child(clean_button(
                "clean-ai-recovery-refresh",
                copy::clean_ai_recovery_refresh(locale).to_string(),
                false,
                palette,
                cx,
                |this, _, cx| this.clean_ai_refresh_recovery(cx),
            ));
            for bundle in view.recovery() {
                let bundle_id = bundle.id.clone();
                actions = actions.child(clean_button(
                    ElementId::Name(SharedString::from(format!("clean-ai-restore-{bundle_id}"))),
                    format!("{} · {}", copy::clean_ai_restore_action(locale), bundle.id),
                    false,
                    palette,
                    cx,
                    move |this, _, cx| this.clean_ai_restore(bundle_id.clone(), cx),
                ));
            }
        }
        actions = actions.child(clean_button(
            "clean-ai-close",
            copy::clean_ai_close_action(locale).to_string(),
            false,
            palette,
            cx,
            |this, _, cx| this.clean_ai_close(cx),
        ));
        controls.child(actions).into_any_element()
    }
}

/// The explicit entry into the AI review subview. Opening never scans.
fn clean_ai_open_button(
    locale: copy::Locale,
    palette: ShellPalette,
    cx: &mut Context<ShellView>,
) -> gpui::AnyElement {
    clean_button(
        "clean-ai-open",
        copy::clean_ai_open_action(locale).to_string(),
        false,
        palette,
        cx,
        |this, _, cx| this.clean_ai_open(cx),
    )
}

/// Found, cleaning, and completed scans with rows show the disk map.
/// Landing never scans, and the AI subview keeps its own list.
fn disk_map_active(session: &Session) -> bool {
    let shell = session.shell();
    shell.surface() == Surface::Mode
        && shell.mode() == Mode::Clean
        && !session.clean_ai().open()
        && matches!(
            session.clean_workbench().phase(),
            CleanPhase::Found | CleanPhase::Cleaning | CleanPhase::Completed
        )
        && !session.clean_workbench().rows().is_empty()
}

fn map_inputs(session: &Session) -> Vec<MapInput<'_>> {
    let workbench = session.clean_workbench();
    session
        .shell()
        .filtered_row_ids(Mode::Clean)
        .iter()
        .filter_map(|id| {
            let row = workbench.row(id)?;
            Some(MapInput {
                id: row.id.as_str(),
                rule_id: row.rule_id.as_str(),
                path: row.path.as_deref(),
                bytes: row.bytes,
                size_complete: row.size_complete,
                selected_by_default: row.selected_by_default,
                selectable: workbench.row_selectable(row),
            })
        })
        .collect()
}

fn disk_map_summary_line(session: &Session) -> String {
    let summary = clean_map::summarize(&map_inputs(session));
    copy::clean_map_summary_line(
        session.locale(),
        summary.mapped_bytes,
        summary.mapped_count,
        summary.recommended_count,
        summary.review_count,
        summary.blocked_count,
        summary.partial_count,
    )
}

/// Treemap plus recommendation cards for the accepted scan.
///
/// Tiles are a projection of scanned targets. Clicking a tile uses the same
/// row selection as the list. This view does not compose a cleanup script.
fn analyze_treemap_element(
    session: &Session,
    locale: copy::Locale,
    palette: ShellPalette,
) -> Option<gpui::AnyElement> {
    let window = session.analyze_window()?;
    let rects = window.treemap();
    if rects.is_empty() {
        return None;
    }
    let mut canvas = div()
        .id("analyze-treemap")
        .relative()
        .w(px(crate::analyze::TREEMAP_WIDTH))
        .h(px(crate::analyze::TREEMAP_HEIGHT))
        .overflow_hidden()
        .rounded_md()
        .bg(rgb(palette.capsule))
        .border_1()
        .border_color(rgb(palette.capsule_border))
        .child(
            div()
                .p_2()
                .text_size(px(12.))
                .child(copy::analyze_treemap_title(locale)),
        );
    for rect in rects.iter().filter(|rect| rect.id != 0).take(24) {
        let selected = window.board().list_selection() == Some(rect.id);
        canvas = canvas.child(
            div()
                .id(gpui::ElementId::named_usize("analyze-tile", rect.id as usize))
                .absolute()
                .left(px(rect.x))
                .top(px(rect.y))
                .w(px(rect.width.max(1.0)))
                .h(px(rect.height.max(1.0)))
                .border_1()
                .border_color(rgb(palette.capsule_border))
                .when(selected, |tile| tile.bg(rgb(palette.pill)))
                .child(format!("{}", rect.id)),
        );
    }
    Some(canvas.into_any_element())
}

fn disk_map_element(
    session: &Session,
    locale: copy::Locale,
    palette: ShellPalette,
    high_contrast: bool,
    cx: &mut Context<ShellView>,
) -> gpui::AnyElement {
    let map = clean_map::project_disk_map(
        &map_inputs(session),
        MapBounds {
            width: clean_map::MAP_CANVAS_WIDTH,
            height: clean_map::MAP_CANVAS_HEIGHT,
        },
    );
    let workbench = session.clean_workbench();
    let selected_bytes = workbench.selected_ids().iter().fold(0u64, |total, id| {
        total.saturating_add(workbench.row(id).map(|row| row.bytes).unwrap_or(0))
    });
    let ink = rgb(clean_map::category_ink(high_contrast));
    let mut canvas = div()
        .id("clean-disk-map-canvas")
        .relative()
        .w(px(clean_map::MAP_CANVAS_WIDTH))
        .h(px(clean_map::MAP_CANVAS_HEIGHT))
        .overflow_hidden()
        .rounded_md()
        .bg(rgb(palette.capsule))
        .border_1()
        .border_color(rgb(palette.capsule_border));
    if map.groups.is_empty() {
        canvas = canvas.child(
            div()
                .p_3()
                .text_color(rgb(palette.text))
                .child(copy::clean_map_empty(locale)),
        );
    }
    for (group_index, group) in map.groups.iter().enumerate() {
        let (x, y, width, height) = inset_rect(group.x, group.y, group.width, group.height);
        let title = if group.other {
            copy::clean_map_other(locale, group.represented)
        } else {
            group.title.clone()
        };
        let group_label = format!(
            "{} {} {}",
            title,
            copy::clean_bytes_short(group.bytes),
            copy::clean_map_category(locale, group.category.as_str())
        );
        let mut tile = div()
            .relative()
            .w(px(width.max(1.0)))
            .h(px(height.max(1.0)))
            .overflow_hidden()
            .rounded_md()
            .bg(rgb(clean_map::category_fill(group.category, high_contrast)))
            .border_1()
            .border_color(rgb(clean_map::category_border(
                group.category,
                high_contrast,
            )))
            .text_color(ink);
        if height >= 36.0 && width >= 48.0 {
            tile = tile.child(
                div()
                    .absolute()
                    .left(px(4.))
                    .top(px(1.))
                    .text_size(px(11.))
                    .child(SharedString::from(format!(
                        "{title} {}",
                        copy::clean_bytes_short(group.bytes)
                    ))),
            );
        }
        for (leaf_index, leaf) in group.leaves.iter().enumerate() {
            let (leaf_x, leaf_y, leaf_w, leaf_h) =
                inset_rect(leaf.x, leaf.y, leaf.width, leaf.height);
            let leaf_title = if leaf.other {
                copy::clean_map_other(locale, leaf.represented)
            } else {
                leaf.title.clone()
            };
            let selected = leaf
                .id
                .as_deref()
                .is_some_and(|id| workbench.is_selected(id));
            let label = format!(
                "{} {} {} {} {}",
                copy::clean_map_category(locale, leaf.category.as_str()),
                leaf_title,
                copy::clean_bytes_short(leaf.bytes),
                copy::clean_map_band(locale, leaf.band.as_str()),
                if selected {
                    copy::clean_row_selected_marker(locale)
                } else {
                    copy::clean_row_unselected_marker(locale)
                }
            );
            let element_id = match leaf.id.as_deref() {
                Some(id) => format!("clean-map-tile-{id}"),
                None => format!("clean-map-other-{group_index}-{leaf_index}"),
            };
            let mut leaf_tile = div()
                .id(ElementId::Name(SharedString::from(element_id)))
                .absolute()
                .left(px(leaf_x))
                .top(px(leaf_y))
                .w(px(leaf_w.max(1.0)))
                .h(px(leaf_h.max(1.0)))
                .overflow_hidden()
                .rounded_md()
                .px_1()
                .bg(rgb(clean_map::category_fill(leaf.category, high_contrast)))
                .border_1()
                .border_color(rgb(if selected {
                    0x1D4E89
                } else {
                    clean_map::category_border(leaf.category, high_contrast)
                }))
                .text_color(ink)
                .aria_label(SharedString::from(label.clone()));
            if let Some(id) = leaf.id.clone() {
                leaf_tile = leaf_tile
                    .role(gpui::Role::Button)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session.clean_click_row(&id);
                        cx.notify();
                    }));
            }
            if leaf_w >= 48.0 && leaf_h >= 22.0 {
                leaf_tile = leaf_tile.child(
                    div()
                        .text_size(px(12.))
                        .child(SharedString::from(leaf_title)),
                );
                if leaf_h >= 40.0 {
                    leaf_tile = leaf_tile.child(
                        div()
                            .text_size(px(11.))
                            .child(SharedString::from(copy::clean_bytes_short(leaf.bytes))),
                    );
                }
            }
            tile = tile.child(leaf_tile);
        }
        canvas = canvas.child(
            div()
                .id(ElementId::Name(SharedString::from(format!(
                    "clean-map-group-{group_index}"
                ))))
                .absolute()
                .left(px(x))
                .top(px(y))
                .w(px(width.max(1.0)))
                .h(px(height.max(1.0)))
                .aria_label(SharedString::from(group_label))
                .child(tile),
        );
    }

    let mut recommendations = div()
        .id("clean-map-recommendations")
        .role(gpui::Role::Group)
        .aria_label(copy::clean_map_recommendations(locale))
        .w(px(300.))
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_color(rgb(palette.text))
                .child(copy::clean_map_recommendations(locale)),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .child(copy::clean_map_recommendations_hint(locale)),
        );
    let cursor = session.shell().selection(Mode::Clean);
    for id in session.shell().visible_row_ids(Mode::Clean) {
        let Some(row) = workbench.row(id) else {
            continue;
        };
        let focused = cursor == Some(id.as_str());
        let selected = workbench.is_selected(id);
        let text = crate::clean_workbench::row_text(
            locale,
            row,
            selected,
            focused,
            row_protected(session, row),
            workbench.is_ignored(id),
        );
        let band = clean_map::review_band(workbench.row_selectable(row), row.selected_by_default);
        recommendations = recommendations.child(recommendation_card(
            locale,
            palette,
            high_contrast,
            row,
            band,
            text,
            selected,
            focused,
            cx,
        ));
    }
    recommendations = recommendations.child(
        div()
            .text_size(px(12.))
            .text_color(rgb(palette.text))
            .child(SharedString::from(format!(
                "{} · {}",
                copy::clean_bytes_short(selected_bytes),
                copy::clean_selection_count(
                    locale,
                    workbench.selected_count(),
                    workbench.rows().len()
                )
            ))),
    );

    let summary = map.summary;
    let mut legend = div().flex().flex_row().flex_wrap().gap_2();
    for category in &map.categories {
        legend = legend.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .w(px(10.))
                        .h(px(10.))
                        .rounded_md()
                        .bg(rgb(clean_map::category_fill(*category, high_contrast)))
                        .border_1()
                        .border_color(rgb(clean_map::category_border(*category, high_contrast))),
                )
                .child(copy::clean_map_category(locale, category.as_str())),
        );
    }

    div()
        .id("clean-disk-map")
        .role(gpui::Role::Group)
        .aria_label(copy::clean_map_title(locale))
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .rounded_md()
        .bg(rgb(palette.menu_bg))
        .border_1()
        .border_color(rgb(palette.capsule_border))
        .text_color(rgb(palette.text))
        .child(copy::clean_map_title(locale))
        .child(
            div()
                .text_size(px(12.))
                .child(copy::clean_map_subtitle(locale)),
        )
        .when(
            !session.shell().row_query(Mode::Clean).is_empty(),
            |header| {
                header.child(
                    div()
                        .text_size(px(12.))
                        .child(copy::clean_map_filtered_note(locale)),
                )
            },
        )
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .child(map_stat_card(
                    "clean-map-mapped",
                    copy::clean_map_card_mapped(locale),
                    copy::clean_bytes_short(summary.mapped_bytes),
                    copy::clean_map_count(locale, summary.mapped_count),
                    palette,
                ))
                .child(map_stat_card(
                    "clean-map-recommended",
                    copy::clean_map_card_recommended(locale),
                    copy::clean_bytes_short(summary.recommended_bytes),
                    copy::clean_map_count(locale, summary.recommended_count),
                    palette,
                ))
                .child(map_stat_card(
                    "clean-map-review",
                    copy::clean_map_card_review(locale),
                    copy::clean_bytes_short(summary.review_bytes),
                    copy::clean_map_count(locale, summary.review_count),
                    palette,
                ))
                .child(map_stat_card(
                    "clean-map-blocked",
                    copy::clean_map_card_blocked(locale),
                    copy::clean_bytes_short(summary.blocked_bytes),
                    copy::clean_map_count(locale, summary.blocked_count),
                    palette,
                )),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_start()
                .gap_3()
                .child(div().flex().flex_col().gap_2().child(canvas).child(legend))
                .child(recommendations),
        )
        .into_any_element()
}

fn map_stat_card(
    id: &'static str,
    label: &'static str,
    value: String,
    detail: String,
    palette: ShellPalette,
) -> gpui::AnyElement {
    div()
        .id(id)
        .flex_1()
        .min_w(px(120.))
        .px_2()
        .py_2()
        .rounded_md()
        .bg(rgb(palette.capsule))
        .border_1()
        .border_color(rgb(palette.capsule_border))
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_size(px(12.)).child(label))
        .child(div().text_size(px(18.)).child(SharedString::from(value)))
        .child(div().text_size(px(12.)).child(SharedString::from(detail)))
        .into_any_element()
}

fn recommendation_card(
    locale: copy::Locale,
    palette: ShellPalette,
    high_contrast: bool,
    row: &CleanRow,
    band: ReviewBand,
    text: String,
    selected: bool,
    focused: bool,
    cx: &mut Context<ShellView>,
) -> gpui::AnyElement {
    let category = clean_map::category_of(&row.rule_id);
    let title = clean_map::leaf_title(&row.rule_id, row.path.as_deref());
    let subtitle = match row.path.as_deref() {
        Some(path) => path.to_string(),
        None => row.rule_id.clone(),
    };
    let element_id = SharedString::from(format!("clean-row-{}", row.id));
    let clicked = row.id.clone();
    div()
        .id(ElementId::Name(element_id.clone()))
        .role(gpui::Role::ListItem)
        .accessibility_id(element_id)
        .aria_label(SharedString::from(text))
        .aria_selected(selected)
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| {
            this.session.clean_click_row(&clicked);
            cx.notify();
        }))
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(if focused {
            palette.pill
        } else {
            palette.capsule_border
        }))
        .bg(rgb(if selected {
            palette.capsule
        } else {
            palette.menu_bg
        }))
        .text_color(rgb(palette.text))
        .child(
            div()
                .w(px(8.))
                .h(px(28.))
                .rounded_md()
                .bg(rgb(clean_map::category_fill(category, high_contrast))),
        )
        .child(if selected {
            copy::clean_row_selected_marker(locale)
        } else {
            copy::clean_row_unselected_marker(locale)
        })
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .overflow_hidden()
                .child(SharedString::from(format!(
                    "{title} · {}",
                    copy::clean_map_band(locale, band.as_str())
                )))
                .child(
                    div()
                        .text_size(px(12.))
                        .overflow_hidden()
                        .child(SharedString::from(subtitle)),
                ),
        )
        .child(SharedString::from(copy::clean_bytes_short(row.bytes)))
        .into_any_element()
}

fn inset_rect(x: f32, y: f32, width: f32, height: f32) -> (f32, f32, f32, f32) {
    if width > 4.0 && height > 4.0 {
        (x + 1.0, y + 1.0, width - 2.0, height - 2.0)
    } else {
        (x, y, width.max(1.0), height.max(1.0))
    }
}

/// One interactive row. Clean rows keep their established `clean-row-` element
/// and accessibility ids; AI rows use their own prefix so no existing
/// accessible id changes.
fn interactive_row_element(
    prefix: &'static str,
    id: String,
    text: String,
    selected: bool,
    cx: &mut Context<ShellView>,
) -> gpui::AnyElement {
    let element_id = SharedString::from(format!("{prefix}-{id}"));
    let clicked = id;
    div()
        .id(ElementId::Name(element_id.clone()))
        .role(gpui::Role::ListItem)
        .accessibility_id(element_id)
        .aria_label(SharedString::from(text.clone()))
        .aria_selected(selected)
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| {
            this.session.clean_click_row(&clicked);
            cx.notify();
        }))
        .child(SharedString::from(text))
        .into_any_element()
}

/// One interactive Clean control. Rendered as a role=Button with an accessible
/// name and a stable id.
fn clean_button(
    id: impl Into<ElementId>,
    label: String,
    filled: bool,
    palette: ShellPalette,
    cx: &mut Context<ShellView>,
    on_click: impl Fn(&mut ShellView, &mut Window, &mut Context<ShellView>) + 'static,
) -> gpui::AnyElement {
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(SharedString::from(label.clone()))
        .px_3()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .when(filled, |button| {
            button
                .bg(rgb(palette.pill))
                .text_color(rgb(palette.pill_text))
        })
        .when(!filled, |button| {
            button
                .bg(rgb(palette.capsule))
                .text_color(rgb(palette.text))
        })
        .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
        .child(SharedString::from(label))
        .into_any_element()
}

/// True when a session protection covers the row ownership path.
fn row_protected(session: &Session, row: &CleanRow) -> bool {
    session.clean_workbench().is_protected(&row.id)
}

fn panel_title(session: &Session) -> String {
    let locale = session.locale();
    match session.shell().surface() {
        Surface::Mode => copy::mode_name(locale, session.shell().mode().id()).to_string(),
        other => surface_title(locale, other).to_string(),
    }
}

fn surface_title(locale: copy::Locale, surface: Surface) -> &'static str {
    match surface {
        Surface::Mode => copy::modes_label(locale),
        Surface::Settings => copy::settings_title(locale),
        Surface::Protection => copy::protection_title(locale),
        Surface::Rules => copy::rules_title(locale),
        Surface::History => copy::history_title(locale),
    }
}

fn surface_element_id(surface: Surface) -> &'static str {
    match surface {
        Surface::Mode => "surface-mode",
        Surface::Settings => "menu-settings",
        Surface::Protection => "menu-protection",
        Surface::Rules => "menu-rules",
        Surface::History => "menu-history",
    }
}

/// AccessKit author id and name. GPUI `.id` stays inside the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AccessibleControl {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccessibleSlot {
    Shell,
    Note,
    CleanSearch,
    Mode(Mode),
    Menu(Surface),
}

pub(crate) fn accessible_control(locale: copy::Locale, slot: AccessibleSlot) -> AccessibleControl {
    match slot {
        AccessibleSlot::Shell => AccessibleControl {
            id: "shell-root",
            label: WINDOW_TITLE,
        },
        AccessibleSlot::Note => AccessibleControl {
            id: "note-input",
            label: "Note",
        },
        AccessibleSlot::CleanSearch => AccessibleControl {
            id: "clean-search-input",
            label: copy::clean_search_placeholder(locale),
        },
        AccessibleSlot::Mode(mode) => AccessibleControl {
            id: mode.element_id(),
            label: copy::mode_name(locale, mode.id()),
        },
        AccessibleSlot::Menu(item) => AccessibleControl {
            id: surface_element_id(item),
            label: surface_title(locale, item),
        },
    }
}

fn workbench_notice_line(locale: copy::Locale, notice: &WorkbenchNotice) -> String {
    match notice {
        WorkbenchNotice::ProviderUiPending => {
            copy::workbench_provider_ui_pending(locale).to_string()
        }
        WorkbenchNotice::PrivilegeDecline => copy::workbench_privilege_decline(locale).to_string(),
        WorkbenchNotice::RebootRequired => copy::workbench_reboot_required(locale).to_string(),
        WorkbenchNotice::FailedUninstall => copy::workbench_failed_uninstall(locale).to_string(),
        WorkbenchNotice::ChangedSelection { selection_id } => {
            copy::workbench_changed_selection(locale, selection_id)
        }
    }
}

fn clean_outcome_line(locale: copy::Locale, notice: CleanOutcomeNotice) -> &'static str {
    match notice {
        CleanOutcomeNotice::Unavailable => copy::clean_outcome_unavailable(locale),
        CleanOutcomeNotice::Empty => copy::clean_outcome_empty(locale),
        CleanOutcomeNotice::Partial => copy::clean_outcome_partial(locale),
        CleanOutcomeNotice::Failed => copy::clean_outcome_failed(locale),
        CleanOutcomeNotice::Canceled => copy::clean_outcome_canceled(locale),
        CleanOutcomeNotice::StalePreview => copy::clean_outcome_stale_preview(locale),
        CleanOutcomeNotice::ConfirmationRequired => {
            copy::clean_outcome_confirmation_required(locale)
        }
        CleanOutcomeNotice::Mixed => copy::clean_outcome_mixed(locale),
        CleanOutcomeNotice::TrashReported => copy::clean_outcome_trash_reported(locale),
    }
}

/// One panel row. Clean and AI rows carry the metadata that makes them
/// interactive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PanelEntry {
    Text(String),
    CleanRow {
        id: String,
        text: String,
        selected: bool,
    },
    AiRow {
        id: String,
        text: String,
        selected: bool,
    },
}

impl PanelEntry {
    /// The text this entry paints. Used by the test-only text projection of
    /// [`panel_entries`]; the window paints each entry with its own element.
    #[cfg(test)]
    pub(crate) fn text(&self) -> &str {
        match self {
            Self::Text(text) => text,
            Self::CleanRow { text, .. } => text,
            Self::AiRow { text, .. } => text,
        }
    }
}

/// The panel as text lines, derived from [`panel_entries`] so it mirrors what
/// the window paints.
///
/// The render path paints `panel_entries` directly (Clean rows stay
/// interactive there), so this projection is only consumed by tests and
/// other text-only assertions. They still assert the real rendered state:
/// every line here is the exact text of a painted entry.
#[cfg(test)]
pub(crate) fn panel_lines(session: &Session, save_error: Option<&str>) -> Vec<String> {
    panel_entries(session, save_error)
        .into_iter()
        .map(|entry| entry.text().to_string())
        .collect()
}

fn mode_row_text(session: &Session, mode: Mode, id: &str, focused: bool) -> String {
    let locale = session.locale();
    if mode == Mode::Software {
        if let Some(line) = session.software_review().row_line(id, locale, focused) {
            return line;
        }
    }
    if mode == Mode::Optimize {
        if let Some(line) = session.optimize_board().row_line(id, locale, focused) {
            return line;
        }
        if let Some(group) = crate::optimize_board::group_from_id(id) {
            let mark = if focused { "> " } else { "" };
            return format!(
                "{mark}{id} | {}",
                copy::optimize_group_name(locale, group.key())
            );
        }
    }
    if focused {
        format!("> {id}")
    } else {
        id.to_string()
    }
}

pub(crate) fn panel_entries(session: &Session, save_error: Option<&str>) -> Vec<PanelEntry> {
    let locale = session.locale();
    let shell = session.shell();
    // The chord line stays at the top so `lines.last()` keeps meaning "newest
    // notice", and so the hint survives any surface or notice state.
    let mut entries = vec![PanelEntry::Text(copy::mode_keys_hint(locale).to_string())];
    if shell.surface() == Surface::Mode && shell.mode() == Mode::Clean {
        entries.push(PanelEntry::Text(copy::clean_keys_hint(locale).to_string()));
    }
    for notice in session.notices() {
        entries.push(PanelEntry::Text(copy::preserved(
            locale,
            copy::store_name(locale, *notice),
        )));
    }
    if let Some(error) = save_error {
        entries.push(PanelEntry::Text(error.to_string()));
    }
    match shell.surface() {
        Surface::Mode => {
            let mode = shell.mode();
            entries.push(PanelEntry::Text(
                copy::mode_name(locale, mode.id()).to_string(),
            ));
            entries.push(PanelEntry::Text(
                copy::mode_subtitle(locale, mode.id()).to_string(),
            ));
            if let Some(report) = shell.report(mode) {
                entries.push(PanelEntry::Text(report.to_string()));
            }
            match shell.review(mode) {
                crate::shell::Review::StaleScanRoots => {
                    entries.push(PanelEntry::Text(copy::stale_scan_roots(locale).to_string()));
                }
                crate::shell::Review::StaleProtection => {
                    entries.push(PanelEntry::Text(copy::stale_protection(locale).to_string()));
                }
                crate::shell::Review::Valid => {}
            }
            if mode == Mode::Clean {
                if session.clean_ai().open() {
                    push_clean_ai_lines(session, &mut entries);
                } else {
                    push_clean_review_lines(session, &mut entries);
                }
            }
            if matches!(mode, Mode::Clean | Mode::Software | Mode::Optimize | Mode::Analyze)
            {
                let visible = shell.visible_row_ids(mode);
                let cursor = matches!(mode, Mode::Clean | Mode::Software | Mode::Optimize | Mode::Analyze)
                    .then(|| shell.selection(mode))
                    .flatten();
                let ai_open = mode == Mode::Clean && session.clean_ai().open();
                for id in visible {
                    let focused = cursor == Some(id.as_str());
                    if ai_open {
                        if let Some(entry) = clean_ai_row_entry(session, id, focused) {
                            entries.push(entry);
                        } else {
                            entries.push(PanelEntry::Text(if focused {
                                format!("> {id}")
                            } else {
                                id.clone()
                            }));
                        }
                    } else if mode == Mode::Clean && session.clean_workbench().row(id).is_some() {
                        entries.push(clean_row_entry(session, id, focused));
                    } else {
                        entries.push(PanelEntry::Text(mode_row_text(session, mode, id, focused)));
                    }
                }
                if let Some(selection) = cursor
                    && !visible.iter().any(|id| id == selection)
                {
                    // An offscreen cursor stays visible as a plain text line.
                    let offscreen = if ai_open {
                        session
                            .clean_ai()
                            .row_text(locale, selection, true)
                            .map(PanelEntry::Text)
                    } else if mode == Mode::Clean {
                        session.clean_workbench().row(selection).map(|row| {
                            PanelEntry::Text(crate::clean_workbench::row_text(
                                locale,
                                row,
                                session.clean_workbench().is_selected(selection),
                                true,
                                row_protected(session, row),
                                session.clean_workbench().is_ignored(selection),
                            ))
                        })
                    } else {
                        Some(PanelEntry::Text(format!("> {selection}")))
                    };
                    entries
                        .push(offscreen.unwrap_or_else(|| PanelEntry::Text(selection.to_string())));
                }
                if mode == Mode::Analyze {
                    let analyze_lines = session.analyze_window_lines();
                    if analyze_lines.is_empty() {
                        if let Some(line) = session.analyze_face_line() {
                            entries.push(PanelEntry::Text(line));
                        }
                    } else {
                        for line in analyze_lines {
                            entries.push(PanelEntry::Text(line));
                        }
                    }
                }
            } else if mode == Mode::Status {
                if let Some(line) = session.status_sensor_line() {
                    entries.push(PanelEntry::Text(line));
                }
                if let Some(line) = session.status_pid_line() {
                    entries.push(PanelEntry::Text(line));
                }
                for line in session.status_lines() {
                    entries.push(PanelEntry::Text(line.clone()));
                }
            }
            if mode == Mode::Software {
                for line in crate::workbench::software_row_lines(locale) {
                    entries.push(PanelEntry::Text(line));
                }
                if session.software_review().is_open() {
                    for line in session.software_review().lines() {
                        entries.push(PanelEntry::Text(line.clone()));
                    }
                }
            }
            if mode == Mode::Optimize {
                entries.push(PanelEntry::Text(session.optimize_board().phase_line(locale)));
                for line in session.optimize_board().group_lines(locale) {
                    entries.push(PanelEntry::Text(line));
                }
                for line in crate::workbench::optimize_row_lines(locale) {
                    entries.push(PanelEntry::Text(line));
                }
            }
            if mode == Mode::Analyze && session.analyze_window().is_none() {
                if session.analyze_roots().is_empty() && session.analyze_landing_prepared() {
                    entries.push(PanelEntry::Text(copy::analyze_empty_title(locale).to_string()));
                    entries.push(PanelEntry::Text(copy::analyze_empty_detail(locale).to_string()));
                }
                for (index, choice) in session.analyze_roots().iter().enumerate() {
                    let state = choice.error.clone().unwrap_or_else(|| "ready".to_string());
                    entries.push(PanelEntry::Text(format!(
                        "root:{index} | {} | {state}",
                        choice.input
                    )));
                }
            }
            if mode == Mode::Status {
                let phase = if session.status_phase() == crate::status_panel::StatusPhase::Live {
                    copy::status_phase_live(locale)
                } else {
                    copy::status_phase_landing(locale)
                };
                entries.push(PanelEntry::Text(format!("phase={phase}")));
                if session.status_phase() == crate::status_panel::StatusPhase::Live {
                    entries.push(PanelEntry::Text(copy::status_live_started(locale, 1, 8)));
                }
                if let Some(snapshot) = session.status_snapshot() {
                    for line in crate::status_panel::group_lines(snapshot, locale) {
                        entries.push(PanelEntry::Text(line));
                    }
                    for line in crate::status_panel::process_lines(
                        snapshot,
                        session.status_query(),
                        session.status_sort(),
                    ) {
                        entries.push(PanelEntry::Text(line));
                    }
                }
            }
            if session.screen_covered() {
                entries.push(PanelEntry::Text(
                    copy::clean_screen_active(locale).to_string(),
                ));
            }
            if let Some(line) = session.screen_lease_line(locale) {
                entries.push(PanelEntry::Text(line));
            }
            if matches!(mode, Mode::Software | Mode::Optimize) {
                for line in session.workbench_review_lines() {
                    entries.push(PanelEntry::Text(line.clone()));
                }
                if let Some(notice) = session.workbench_notice() {
                    entries.push(PanelEntry::Text(workbench_notice_line(locale, notice)));
                }
            }
            if mode == Mode::Clean {
                let notice = if session.clean_ai().open() {
                    session.clean_ai().notice()
                } else {
                    session.clean_outcome()
                };
                if let Some(notice) = notice {
                    entries.push(PanelEntry::Text(
                        clean_outcome_line(locale, notice).to_string(),
                    ));
                }
            }
        }
        Surface::Settings => {
            entries.push(PanelEntry::Text(copy::settings_title(locale).to_string()));
            entries.push(PanelEntry::Text(
                copy::settings_subtitle(locale).to_string(),
            ));
            entries.push(PanelEntry::Text(format!(
                "{}: {}",
                copy::language_label(locale),
                copy::language_option(locale)
            )));
            entries.push(PanelEntry::Text(format!(
                "{}: {}",
                copy::theme_label(locale),
                session.theme().as_str()
            )));
            entries.push(PanelEntry::Text(format!("{}%", session.text_scale())));
            entries.push(PanelEntry::Text(session.motion().as_str().to_string()));
            entries.push(PanelEntry::Text(copy::project_folders(locale).to_string()));
            entries.extend(shell.scan_roots().iter().cloned().map(PanelEntry::Text));
            entries.push(PanelEntry::Text(
                copy::separate_protection(locale).to_string(),
            ));
        }
        Surface::Protection => {
            entries.push(PanelEntry::Text(copy::protection_title(locale).to_string()));
            entries.push(PanelEntry::Text(
                copy::protection_subtitle(locale).to_string(),
            ));
            entries.extend(shell.protections().iter().cloned().map(PanelEntry::Text));
        }
        Surface::Rules => {
            entries.push(PanelEntry::Text(copy::rules_title(locale).to_string()));
            entries.push(PanelEntry::Text(copy::rules_subtitle(locale).to_string()));
            if shell.rules().is_empty() {
                entries.push(PanelEntry::Text(copy::rules_empty(locale).to_string()));
            } else {
                entries.extend(shell.rules().iter().cloned().map(PanelEntry::Text));
            }
        }
        Surface::History => {
            entries.push(PanelEntry::Text(copy::history_title(locale).to_string()));
            entries.push(PanelEntry::Text(copy::history_subtitle(locale).to_string()));
            entries.push(PanelEntry::Text(
                copy::history_no_replay(locale).to_string(),
            ));
            match shell.history_store() {
                crate::shell::HistoryStoreView::Unavailable => {
                    entries.push(PanelEntry::Text(
                        copy::history_unavailable(locale).to_string(),
                    ));
                }
                crate::shell::HistoryStoreView::Ready => {
                    entries.extend(shell.history().iter().cloned().map(PanelEntry::Text));
                }
                crate::shell::HistoryStoreView::NotLoaded
                | crate::shell::HistoryStoreView::Empty => {
                    entries.push(PanelEntry::Text(copy::history_empty(locale).to_string()));
                }
            }
        }
    }
    entries
}

/// Clean phase, root, selection, preview digest and result lines.
fn push_clean_review_lines(session: &Session, entries: &mut Vec<PanelEntry>) {
    let locale = session.locale();
    let shell = session.shell();
    let workbench = session.clean_workbench();
    let phase_line = match workbench.phase() {
        CleanPhase::Landing => match workbench.prompt() {
            CleanLandingPrompt::None => None,
            CleanLandingPrompt::ScopeChoice => Some(copy::clean_scope_prompt(locale).to_string()),
            CleanLandingPrompt::RootRequired => Some(copy::clean_root_prompt(locale).to_string()),
            CleanLandingPrompt::ConfirmGlobal => {
                Some(copy::clean_global_confirm(locale).to_string())
            }
        },
        CleanPhase::Scanning => Some(copy::clean_phase_scanning(locale, workbench.found_so_far())),
        CleanPhase::Found => Some(copy::clean_phase_found(locale, workbench.rows().len())),
        CleanPhase::AllClean => Some(copy::clean_phase_all_clean(locale).to_string()),
        CleanPhase::Cleaning => Some(copy::clean_phase_cleaning(locale).to_string()),
        CleanPhase::Completed => Some(copy::clean_phase_completed(locale).to_string()),
        CleanPhase::Failed => Some(copy::clean_phase_failed(locale).to_string()),
    };
    if let Some(line) = phase_line {
        entries.push(PanelEntry::Text(line));
    }
    if disk_map_active(session) {
        entries.push(PanelEntry::Text(disk_map_summary_line(session)));
    }
    for root in shell.scan_roots() {
        entries.push(PanelEntry::Text(copy::clean_root_line(locale, root)));
    }
    if workbench.root_session_only() {
        entries.push(PanelEntry::Text(
            copy::clean_root_session_only(locale).to_string(),
        ));
    }
    if session.protection_session_only() {
        entries.push(PanelEntry::Text(
            copy::clean_protection_session_only(locale).to_string(),
        ));
    }
    if let Some(detail) = workbench.failure_detail() {
        entries.push(PanelEntry::Text(detail.to_string()));
    }
    if matches!(
        workbench.phase(),
        CleanPhase::Found | CleanPhase::Cleaning | CleanPhase::Completed
    ) && !workbench.rows().is_empty()
    {
        let selected_ids = workbench.selected_ids();
        entries.push(PanelEntry::Text(copy::clean_selection_count(
            locale,
            selected_ids.len(),
            workbench.rows().len(),
        )));
        // Bounded review summary straight from the selection set, so an
        // offscreen or filtered selection stays visible as an identity list.
        if !selected_ids.is_empty() {
            let sample = selected_ids
                .iter()
                .take(4)
                .map(String::as_str)
                .collect::<Vec<_>>();
            entries.push(PanelEntry::Text(copy::clean_selection_summary(
                locale,
                &sample,
                selected_ids.len(),
            )));
        }
    }
    if let Some(preview) = workbench.preview() {
        entries.push(PanelEntry::Text(copy::clean_preview_digest_line(
            locale,
            &preview.digest,
            preview.count,
        )));
    }
    if let Some((moved, total)) = workbench.result_counts() {
        entries.push(PanelEntry::Text(copy::clean_result_counts(
            locale, moved, total,
        )));
    }
}

/// One interactive Clean row entry with its selection state.
fn clean_row_entry(session: &Session, id: &str, focused: bool) -> PanelEntry {
    let workbench = session.clean_workbench();
    let row = workbench.row(id).expect("row was checked");
    let selected = workbench.is_selected(id);
    let text = crate::clean_workbench::row_text(
        session.locale(),
        row,
        selected,
        focused,
        row_protected(session, row),
        workbench.is_ignored(id),
    );
    PanelEntry::CleanRow {
        id: id.to_string(),
        text,
        selected,
    }
}

/// One interactive AI row entry with its selection state.
fn clean_ai_row_entry(session: &Session, id: &str, focused: bool) -> Option<PanelEntry> {
    let view = session.clean_ai();
    let text = view.row_text(session.locale(), id, focused)?;
    Some(PanelEntry::AiRow {
        id: id.to_string(),
        text,
        selected: view.is_selected(id),
    })
}

/// AI subview phase, provider, categories, selection, digest and recovery.
///
/// Category rows themselves are painted by the bounded row window, so this
/// function only pushes the non-row lines.
fn push_clean_ai_lines(session: &Session, entries: &mut Vec<PanelEntry>) {
    let locale = session.locale();
    let view = session.clean_ai();
    entries.push(PanelEntry::Text(copy::clean_ai_title(locale).to_string()));
    entries.push(PanelEntry::Text(
        copy::clean_ai_subtitle(locale).to_string(),
    ));
    entries.push(PanelEntry::Text(copy::clean_ai_provider(
        locale,
        view.provider().as_str(),
    )));
    if let Some(home) = view.home() {
        entries.push(PanelEntry::Text(copy::clean_ai_home_line(
            locale,
            &home.display().to_string(),
        )));
    }
    entries.push(PanelEntry::Text(
        copy::clean_ai_keys_hint(locale).to_string(),
    ));
    let phase_line = match view.phase() {
        AiSubviewPhase::Landing => copy::clean_ai_landing_prompt(locale).to_string(),
        AiSubviewPhase::Scanning => copy::clean_ai_phase_scanning(locale).to_string(),
        AiSubviewPhase::Review => {
            copy::clean_ai_phase_review(locale, view.rows().len(), view.sensitive_count())
        }
        AiSubviewPhase::Cleaning => copy::clean_ai_phase_cleaning(locale).to_string(),
        AiSubviewPhase::Completed => copy::clean_ai_phase_completed(locale).to_string(),
        AiSubviewPhase::Failed => copy::clean_ai_phase_failed(locale).to_string(),
    };
    entries.push(PanelEntry::Text(phase_line));
    if let Some(detail) = view.failure_detail() {
        entries.push(PanelEntry::Text(detail.to_string()));
    }
    let counts = AiCategory::ALL
        .into_iter()
        .map(|category| (category.id(), view.category_count(category)))
        .collect::<Vec<_>>();
    entries.push(PanelEntry::Text(copy::clean_ai_category_counts(
        locale, &counts,
    )));
    entries.push(PanelEntry::Text(copy::clean_ai_active_category(
        locale,
        copy::clean_ai_category_name(locale, view.category().id()),
        view.selectable_count(),
    )));
    if view.hidden_tool_rows() > 0 {
        entries.push(PanelEntry::Text(copy::clean_ai_tool_rows(
            locale,
            view.hidden_tool_rows(),
        )));
    }
    if !view.rows().is_empty() {
        entries.push(PanelEntry::Text(copy::clean_ai_selection_count(
            locale,
            view.selected_count(),
            view.selectable_count(),
        )));
        if view.total_selected_count() > view.selected_count() {
            let others = AiCategory::ALL
                .into_iter()
                .filter(|category| *category != view.category())
                .filter_map(|category| {
                    let selected = view.selection_for(category).len();
                    (selected > 0).then(|| {
                        format!(
                            "{} {selected}",
                            copy::clean_ai_category_name(locale, category.id())
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join(" · ");
            if !others.is_empty() {
                entries.push(PanelEntry::Text(copy::clean_ai_other_selections(
                    locale, &others,
                )));
            }
        }
    }
    if let Some(preview) = view.preview() {
        entries.push(PanelEntry::Text(copy::clean_ai_digest_line(
            locale,
            &preview.digest,
            preview.count,
        )));
    }
    entries.push(PanelEntry::Text(
        copy::clean_ai_recovery_title(locale).to_string(),
    ));
    if view.recovery_loaded() {
        if view.recovery().is_empty() {
            entries.push(PanelEntry::Text(
                copy::clean_ai_recovery_empty(locale).to_string(),
            ));
        } else {
            for bundle in view.recovery() {
                entries.push(PanelEntry::Text(copy::clean_ai_recovery_line(
                    locale,
                    &bundle.id,
                    &bundle.content_sha256,
                    &bundle.relative_path,
                )));
            }
        }
    }
    if let Some(plan) = view.planned_restore()
        && let Some(target) = plan.targets.first()
    {
        entries.push(PanelEntry::Text(copy::clean_ai_restore_ready(
            locale,
            &target.relative_path,
            &target.content_sha256,
        )));
    }
    if let Some(bundle) = view.restore_refused_bundle() {
        entries.push(PanelEntry::Text(format!(
            "{} {bundle}",
            copy::clean_ai_restore_refused(locale)
        )));
    }
}

pub fn run(session: Session) {
    // The tray lives for this blocking run and is dropped when the app exits.
    // Unit tests do not call `run`, so they do not create a notification-area icon.
    #[cfg(windows)]
    let _tray = crate::tray::install_tray(crate::tray::RealTray::new());
    // Window HICONs also live for the whole run; `WM_SETICON` does not copy.
    #[cfg(windows)]
    let window_icons = crate::icon::WindowIcons::load();
    application()
        .with_assets(PackagedAssets)
        .run(move |cx: &mut App| {
            cx.bind_keys([
                KeyBinding::new("left", ModePrev, None),
                KeyBinding::new("right", ModeNext, None),
                KeyBinding::new("up", MenuUp, None),
                KeyBinding::new("down", MenuDown, None),
                KeyBinding::new("alt-down", RowDown, None),
                KeyBinding::new("alt-up", RowUp, None),
                // Space toggles the focused row. SpikeInput is the Note's own
                // context; CleanReview stays for a focused shell outside the Note.
                // The handler propagates when the key must stay text.
                KeyBinding::new("space", ToggleRow, Some("CleanReview")),
                KeyBinding::new("space", ToggleRow, Some("SpikeInput")),
                // Tab reaches the search field only from the focused Note during
                // review. Shift-Tab from that field returns to the Note.
                KeyBinding::new("tab", TabCleanReview, Some("SpikeInput")),
                KeyBinding::new("shift-tab", ShiftTabCleanReview, Some("SpikeInput")),
                // F7 cycles Found-phase sort and does not change mode.
                KeyBinding::new("f7", CycleCleanSort, None),
                KeyBinding::new("pageup", CleanPageUp, None),
                KeyBinding::new("pagedown", CleanPageDown, None),
                KeyBinding::new("escape", Back, None),
                KeyBinding::new("f8", OpenAiReview, None),
                KeyBinding::new("f8", OpenAiReview, Some("SpikeInput")),
                KeyBinding::new("f9", RequestProjectRows, None),
                KeyBinding::new("f11", ToggleCleanScreen, None),
                KeyBinding::new("f11", ToggleCleanScreen, Some("SpikeInput")),
                KeyBinding::new("f12", ToggleKeepScreen, None),
                KeyBinding::new("f12", ToggleKeepScreen, Some("SpikeInput")),
                KeyBinding::new("escape", Back, Some("SpikeInput")),
                KeyBinding::new("f10", MenuToggle, None),
                KeyBinding::new("f10", MenuToggle, Some("SpikeInput")),
                KeyBinding::new("down", MenuDown, Some("SpikeInput")),
                KeyBinding::new("up", MenuUp, Some("SpikeInput")),
                KeyBinding::new("enter", Activate, None),
                KeyBinding::new("enter", Activate, Some("SpikeInput")),
                KeyBinding::new("backspace", input::Backspace, Some("SpikeInput")),
                KeyBinding::new("delete", input::Delete, Some("SpikeInput")),
                KeyBinding::new("left", input::Left, Some("SpikeInput")),
                KeyBinding::new("right", input::Right, Some("SpikeInput")),
                KeyBinding::new("alt-left", ModePrev, Some("SpikeInput")),
                KeyBinding::new("alt-right", ModeNext, Some("SpikeInput")),
                KeyBinding::new("home", input::Home, Some("SpikeInput")),
                KeyBinding::new("end", input::End, Some("SpikeInput")),
            ]);
            cx.on_window_closed(|cx, _window_id| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1040.), px(720.)), cx);
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some(SharedString::from(WINDOW_TITLE)),
                            appears_transparent: false,
                            ..Default::default()
                        }),
                        app_id: Some("devsweep".to_string()),
                        focus: true,
                        show: true,
                        ..Default::default()
                    },
                    move |window, cx| {
                        let placeholder = copy::note_placeholder(session.locale());
                        let text_scale_percent = session.text_scale();
                        let note = cx
                            .new(|cx| TextInput::new(window, cx, placeholder, text_scale_percent));
                        // The row search never steals startup focus. Its edits
                        // drive the existing row query through the shell model.
                        let search = cx.new(|cx| {
                            TextInput::new_secondary(
                                cx,
                                copy::clean_search_placeholder(session.locale()),
                                "clean-search-input",
                                accessible_control(session.locale(), AccessibleSlot::CleanSearch),
                            )
                        });
                        let view = cx.new(|cx| ShellView {
                            focus_handle: cx.focus_handle(),
                            session,
                            save_error: None,
                            note,
                            search: search.clone(),
                        });
                        view.update(cx, |_view, cx| {
                            cx.observe(&search, |this, observed, cx| {
                                let query = observed.read(cx).content().to_string();
                                match this.session.shell().mode() {
                                    Mode::Software => this.session.software_set_query(query),
                                    Mode::Optimize => this.session.optimize_set_query(query),
                                    Mode::Status => this.session.status_set_query(query),
                                    _ => {
                                        this.session.clean_set_search(query);
                                    }
                                }
                                cx.notify();
                            })
                            .detach();
                        });
                        view
                    },
                )
                .expect("open native window");
            window
                .update(cx, |view, window, cx| {
                    // The titlebar and taskbar read these HICONs from the real
                    // HWND; the handles are kept alive by `window_icons`.
                    #[cfg(windows)]
                    let _ = window_icons.apply(window);
                    // EntityInputHandler only receives characters while the Note
                    // control keeps keyboard focus. Do not retarget this to the
                    // shell root.
                    window.focus(&view.note.focus_handle(cx), cx);
                    cx.activate(true);
                })
                .expect("focus note");
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copy::Locale;
    use crate::preferences::DesktopMotion;
    use crate::shell::NavKey;
    use devsweep_core::{
        ai::{AiError, AiExecuteReportV1, AiExit},
        optimize::PrecheckReason,
        software::{AdapterOutcome, SoftwareExecutionOutcome, SoftwarePlanError},
    };

    #[test]
    fn light_theme_canvas_is_not_dark() {
        let light = shell_palette(DesktopTheme::Light);
        assert_ne!(light.canvas, 0x11162a);
        assert_ne!(light.text, 0xeef1f7);
    }

    #[test]
    fn high_contrast_palette_is_distinct() {
        let high_dark = shell_palette_for(DesktopTheme::Dark, true);
        let high_light = shell_palette_for(DesktopTheme::Light, true);
        let dark = shell_palette_for(DesktopTheme::Dark, false);
        let light = shell_palette_for(DesktopTheme::Light, false);

        assert_eq!(high_dark.canvas, 0x000000);
        assert_eq!(high_dark.text, 0xffffff);
        assert_eq!(high_dark, high_light);
        assert_ne!(high_dark, dark);
        assert_ne!(high_dark, light);
        assert_ne!(dark, light);
        assert_eq!(light.canvas, 0xf3f5fa);
        assert_eq!(dark.canvas, 0x11162a);
        assert_eq!(shell_palette(DesktopTheme::Light), light);
        assert_eq!(shell_palette(DesktopTheme::Dark), dark);
    }

    #[test]
    fn c10_ac4_packaged_icon_and_copy_are_offline() {
        let icon = PackagedAssets.load(ICON_NAME).unwrap().expect("icon");
        assert!(icon.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(PackagedAssets.load("mole-planet.png").unwrap().is_none());
        assert_eq!(
            PackagedAssets.list("").unwrap(),
            vec![
                SharedString::from(ICON_NAME),
                SharedString::from(COPY_NAME),
                SharedString::from(EARTH_NAME)
            ]
        );
        // The landing still is the repository's licensed Earth image, not a
        // newly fetched asset. The packaged bytes must stay byte-identical to
        // desktop/public/earth/still.png (the constant in this file documents
        // the same hash), and it is a single still frame: the render path
        // paints it with `img` and never requests an animation frame.
        let earth = PackagedAssets
            .load(EARTH_NAME)
            .unwrap()
            .expect("earth still");
        assert!(earth.starts_with(b"\x89PNG\r\n\x1a\n"));
        let earth_hash = {
            use sha2::{Digest, Sha256};
            let digest = Sha256::digest(earth.as_ref());
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        assert_eq!(
            earth_hash, "1826f0bd821a838717f4cd8756d3fa50c85ada38395f8a8bfaa638e0620cfe18",
            "packaged Earth still must stay byte-identical to desktop/public/earth/still.png"
        );
        assert!(!earth.is_empty());
        let view_source = include_str!("view.rs");
        assert!(
            view_source.contains("desktop/public/earth/still.png"),
            "the earth-still reuse must stay documented in view.rs"
        );
        let copy_bytes = PackagedAssets.load(COPY_NAME).unwrap().expect("copy");
        let packaged_copy = std::str::from_utf8(copy_bytes.as_ref()).unwrap();
        assert!(packaged_copy.contains("Clean, Software, Optimize, Analyze, Status"));
        assert!(packaged_copy.contains("清理、软件、优化、分析、状态"));
        assert!(packaged_copy.contains("清理只来自已保存的预览计划"));
        assert!(packaged_copy.contains("No asset conversion"));
        assert!(packaged_copy.contains("existing DevSweep master PNG"));
        assert!(!packaged_copy.contains("http://"));
        assert!(!packaged_copy.contains("https://"));
        assert!(!packaged_copy.to_ascii_lowercase().contains("mole"));
    }

    #[test]
    fn workbench_notices_render_in_panel_lines() {
        let mut session = Session::unbacked();
        assert_eq!(session.locale(), Locale::En);
        session.show_mode(Mode::Software);

        let selection_id = "selection-42";
        let notices = [
            WorkbenchNotice::from_software_adapter(AdapterOutcome::Unfinished)
                .expect("unfinished adapter is provider UI pending"),
            WorkbenchNotice::from_precheck_reason(PrecheckReason::UacDeclined)
                .expect("uac_declined is privilege decline"),
            WorkbenchNotice::from_software_execution(SoftwareExecutionOutcome::RebootRequired)
                .expect("reboot required maps"),
            WorkbenchNotice::from_software_execution(SoftwareExecutionOutcome::Failed)
                .expect("failed uninstall maps"),
            WorkbenchNotice::from_software_plan_error(&SoftwarePlanError::StaleSelection(
                selection_id.to_string(),
            ))
            .expect("stale selection maps"),
        ];
        assert_eq!(PrecheckReason::UacDeclined.code(), "uac_declined");
        assert_eq!(
            WorkbenchNotice::from_software_adapter(AdapterOutcome::Success),
            None,
            "success is not a workbench notice"
        );
        assert_eq!(
            WorkbenchNotice::from_software_execution(SoftwareExecutionOutcome::Removed),
            None,
            "removed is not a workbench notice"
        );

        let en = [
            "The provider UI is still pending.".to_string(),
            "Elevation was declined.".to_string(),
            "A reboot is required.".to_string(),
            "The uninstall failed.".to_string(),
            format!(
                "The selection changed. The previous plan is not confirmed. Selection id: {selection_id}."
            ),
        ];
        let zh = [
            "提供程序界面仍在等待处理。".to_string(),
            "已拒绝提升权限。".to_string(),
            "需要重新启动计算机。".to_string(),
            "卸载失败。".to_string(),
            format!("选择已更改。先前计划未确认。选择标识：{selection_id}。"),
        ];

        session.set_workbench_notice(None);
        let baseline = panel_lines(&session, None);
        for expected in &en {
            assert!(
                !baseline.iter().any(|line| line == expected),
                "no notice must not add {expected}"
            );
        }

        let mut seen_en = Vec::new();
        for (notice, expected) in notices.iter().zip(en.iter()) {
            session.set_workbench_notice(Some(notice.clone()));
            let lines = panel_lines(&session, None);
            assert_eq!(lines.len(), baseline.len() + 1);
            assert!(
                lines.contains(expected),
                "panel_lines missing {expected:?}: {lines:?}"
            );
            assert_eq!(lines.last(), Some(expected));
            let lower = expected.to_ascii_lowercase();
            assert!(!lower.contains("succeed"));
            assert!(!lower.contains("removed"));
            assert!(!lower.contains("uninstalled"));
            seen_en.push(expected.clone());
        }
        assert_distinct(&seen_en);

        session.set_workbench_notice(Some(notices[0].clone()));
        session.show_mode(Mode::Optimize);
        assert!(panel_lines(&session, None).contains(&en[0]));
        session.show_mode(Mode::Clean);
        assert!(!panel_lines(&session, None).contains(&en[0]));
        session.show_mode(Mode::Software);
        assert!(panel_lines(&session, None).contains(&en[0]));

        session.cycle_language();
        assert_eq!(session.locale(), Locale::ZhCn);
        session.set_workbench_notice(None);
        let zh_baseline = panel_lines(&session, None);
        let mut seen_zh = Vec::new();
        for (notice, expected) in notices.iter().zip(zh.iter()) {
            session.set_workbench_notice(Some(notice.clone()));
            let lines = panel_lines(&session, None);
            assert_eq!(lines.len(), zh_baseline.len() + 1);
            assert!(
                lines.contains(expected),
                "panel_lines missing {expected:?}: {lines:?}"
            );
            assert_eq!(lines.last(), Some(expected));
            seen_zh.push(expected.clone());
        }
        assert_distinct(&seen_zh);
        for (english, chinese) in en.iter().zip(zh.iter()) {
            assert_ne!(english, chinese);
        }

        session.set_workbench_notice(None);
        assert_eq!(panel_lines(&session, None), zh_baseline);
    }

    #[test]
    fn clean_outcome_lines_render_in_panel_lines() {
        let mut session = Session::unbacked();
        assert_eq!(session.locale(), Locale::En);
        session.show_mode(Mode::Clean);

        let unavailable = CleanOutcomeNotice::from_ai_error(&AiError {
            exit: AiExit::Unavailable,
            code: "source_unavailable",
            message: "AI home is absent".to_string(),
        })
        .expect("source_unavailable is unavailable");
        let empty = CleanOutcomeNotice::Empty;
        let partial = CleanOutcomeNotice::from_ai_error(&AiError {
            exit: AiExit::Partial,
            code: "incomplete_observation",
            message: "session enumeration stopped at the bound".to_string(),
        })
        .expect("partial exit is partial");
        let failed = CleanOutcomeNotice::from_ai_error(&AiError {
            exit: AiExit::Failed,
            code: "execute_failed",
            message: "AI execute failed".to_string(),
        })
        .expect("failed exit is failed");
        let canceled = CleanOutcomeNotice::from_ai_error(&AiError {
            exit: AiExit::Canceled,
            code: "canceled",
            message: "canceled before dispatch".to_string(),
        })
        .expect("canceled is canceled");
        let stale = CleanOutcomeNotice::from_ai_error(&AiError {
            exit: AiExit::Stale,
            code: "stale_confirmation",
            message: "the live AI identity does not match the preview digest".to_string(),
        })
        .expect("stale_confirmation is stale preview");
        let confirmation = CleanOutcomeNotice::from_ai_error(&AiError {
            exit: AiExit::Usage,
            code: "confirmation_required",
            message: "AI execute requires confirmation".to_string(),
        })
        .expect("confirmation_required is confirmation required");
        let trash = CleanOutcomeNotice::from_ai_execute_report(&AiExecuteReportV1 {
            schema: "devsweep.ai.execute.v1".to_string(),
            version: 1,
            outcome: "succeeded".to_string(),
            bundle_ids: vec!["bundle-1".to_string()],
            moved: vec!["target-1".to_string()],
            code: None,
        })
        .expect("succeeded with moved is trash reported");
        let mixed = CleanOutcomeNotice::from_ai_execute_report(&AiExecuteReportV1 {
            schema: "devsweep.ai.execute.v1".to_string(),
            version: 1,
            outcome: "partial".to_string(),
            bundle_ids: vec!["bundle-1".to_string()],
            moved: vec!["target-1".to_string()],
            code: None,
        })
        .expect("partial with moved is mixed");
        assert_eq!(mixed, CleanOutcomeNotice::Mixed);
        assert_eq!(
            CleanOutcomeNotice::from_ai_error(&AiError {
                exit: AiExit::Usage,
                code: "confirmation_required",
                message: "AI execute requires confirmation".to_string(),
            }),
            Some(CleanOutcomeNotice::ConfirmationRequired)
        );
        assert_eq!(stale, CleanOutcomeNotice::StalePreview);
        assert_eq!(
            CleanOutcomeNotice::from_ai_execute_report(&AiExecuteReportV1 {
                schema: "devsweep.ai.execute.v1".to_string(),
                version: 1,
                outcome: "succeeded".to_string(),
                bundle_ids: Vec::new(),
                moved: Vec::new(),
                code: None,
            }),
            None,
            "success with no moved list is not a Clean outcome notice"
        );

        let notices = [
            unavailable,
            empty,
            partial,
            failed,
            canceled,
            stale,
            confirmation,
            trash,
            mixed,
        ];
        let en = [
            "The source is unavailable.".to_string(),
            "There is nothing to clean.".to_string(),
            "The result is partial.".to_string(),
            "The clean failed.".to_string(),
            "The clean was canceled.".to_string(),
            "The preview digest is stale.".to_string(),
            "Confirmation is required.".to_string(),
            "The isolated trash move was reported.".to_string(),
            "Some targets moved and some did not.".to_string(),
        ];
        let zh = [
            "来源不可用。".to_string(),
            "没有可清理的内容。".to_string(),
            "结果为部分完成。".to_string(),
            "清理失败。".to_string(),
            "清理已取消。".to_string(),
            "预览摘要已过期。".to_string(),
            "需要确认。".to_string(),
            "已报告隔离的移出。".to_string(),
            "部分目标已移出，部分目标未移出。".to_string(),
        ];

        session.set_clean_outcome(None);
        let baseline = panel_lines(&session, None);
        for expected in &en {
            assert!(
                !baseline.iter().any(|line| line == expected),
                "no notice must not add {expected}"
            );
        }

        let mut seen_en = Vec::new();
        for (notice, expected) in notices.iter().zip(en.iter()) {
            session.set_clean_outcome(Some(*notice));
            let lines = panel_lines(&session, None);
            assert_eq!(lines.len(), baseline.len() + 1);
            assert_eq!(
                lines.iter().filter(|line| *line == expected).count(),
                1,
                "panel_lines must add exactly one {expected:?}: {lines:?}"
            );
            assert_eq!(lines.last(), Some(expected));
            let lower = expected.to_ascii_lowercase();
            assert!(!lower.contains("succeed"));
            seen_en.push(expected.clone());
        }
        for (index, line) in seen_en.iter().enumerate() {
            assert_eq!(
                seen_en.iter().filter(|other| *other == line).count(),
                1,
                "duplicate English notice line at {index}: {line}"
            );
        }
        assert_eq!(seen_en.len(), 9);
        assert!(
            !en[7].contains("Recycle Bin") && !en[7].contains("回收站"),
            "trash-reported English must not name the Recycle Bin: {}",
            en[7]
        );
        assert!(
            !en[8].contains("Recycle Bin") && !en[8].contains("回收站"),
            "mixed English must not name the Recycle Bin: {}",
            en[8]
        );

        session.cycle_language();
        assert_eq!(session.locale(), Locale::ZhCn);
        session.set_clean_outcome(None);
        let zh_baseline = panel_lines(&session, None);
        let mut seen_zh = Vec::new();
        for (notice, expected) in notices.iter().zip(zh.iter()) {
            session.set_clean_outcome(Some(*notice));
            let lines = panel_lines(&session, None);
            assert_eq!(lines.len(), zh_baseline.len() + 1);
            assert_eq!(
                lines.iter().filter(|line| *line == expected).count(),
                1,
                "panel_lines missing {expected:?}: {lines:?}"
            );
            assert_eq!(lines.last(), Some(expected));
            assert!(!expected.contains("成功"));
            seen_zh.push(expected.clone());
        }
        for (index, line) in seen_zh.iter().enumerate() {
            assert_eq!(
                seen_zh.iter().filter(|other| *other == line).count(),
                1,
                "duplicate Chinese notice line at {index}: {line}"
            );
        }
        assert_eq!(seen_zh.len(), 9);
        for (english, chinese) in en.iter().zip(zh.iter()) {
            assert_ne!(english, chinese);
        }
        assert!(
            !zh[7].contains("Recycle Bin") && !zh[7].contains("回收站"),
            "trash-reported Chinese must not name the Recycle Bin: {}",
            zh[7]
        );
        assert!(
            !zh[8].contains("Recycle Bin") && !zh[8].contains("回收站"),
            "mixed Chinese must not name the Recycle Bin: {}",
            zh[8]
        );

        session.set_clean_outcome(None);
        assert_eq!(panel_lines(&session, None), zh_baseline);
    }

    #[test]
    fn mixed_execute_report_is_not_trash_reported() {
        fn execute_report(outcome: &str) -> AiExecuteReportV1 {
            AiExecuteReportV1 {
                schema: "devsweep.ai.execute.v1".to_string(),
                version: 1,
                outcome: outcome.to_string(),
                bundle_ids: Vec::new(),
                moved: vec!["target-1".to_string()],
                code: None,
            }
        }

        let mut session = Session::unbacked();
        session.show_mode(Mode::Clean);

        let mixed = CleanOutcomeNotice::from_ai_execute_report(&execute_report("partial"))
            .expect("partial with one moved id maps");
        assert_eq!(mixed, CleanOutcomeNotice::Mixed);
        session.set_clean_outcome(Some(mixed));
        let mixed_line = panel_lines(&session, None)
            .pop()
            .expect("Clean panel has a mixed line");
        assert_eq!(mixed_line, "Some targets moved and some did not.");
        assert_ne!(mixed_line, copy::clean_outcome_trash_reported(Locale::En));

        let trash = CleanOutcomeNotice::from_ai_execute_report(&execute_report("succeeded"))
            .expect("succeeded with one moved id maps");
        assert_eq!(trash, CleanOutcomeNotice::TrashReported);
        session.set_clean_outcome(Some(trash));
        let trash_line = panel_lines(&session, None)
            .pop()
            .expect("Clean panel has a trash-reported line");

        let failed = CleanOutcomeNotice::from_ai_execute_report(&execute_report("failed"))
            .expect("failed with one moved id maps");
        assert_eq!(failed, CleanOutcomeNotice::Failed);
        session.set_clean_outcome(Some(failed));
        let failed_line = panel_lines(&session, None)
            .pop()
            .expect("Clean panel has a failed line");

        for line in [&mixed_line, &trash_line, &failed_line] {
            let lower = line.to_ascii_lowercase();
            assert!(
                !lower.contains("recycle bin was emptied"),
                "must not say the Recycle Bin was emptied: {line}"
            );
            assert!(
                !(lower.contains("recycle bin") && lower.contains("emptied")),
                "must not say the Recycle Bin was emptied: {line}"
            );
        }
    }

    #[test]
    fn selected_clean_row_is_marked_in_panel_lines() {
        let mut session = Session::unbacked();
        session.set_row_ids(Mode::Clean, vec!["row-a".to_string(), "row-b".to_string()]);
        session.apply_key(NavKey::RowDown);
        let lines = panel_lines(&session, None);
        assert!(
            lines.iter().any(|line| line == "> row-a"),
            "first RowDown must mark row-a: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line == "row-b"),
            "row-b stays unmarked: {lines:?}"
        );
        assert!(
            !lines.iter().any(|line| line == "> row-b"),
            "row-b must not be marked: {lines:?}"
        );

        session.apply_key(NavKey::RowDown);
        let lines = panel_lines(&session, None);
        assert!(
            lines.iter().any(|line| line == "row-a"),
            "row-a is unmarked after second RowDown: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line == "> row-b"),
            "second RowDown must mark row-b: {lines:?}"
        );
        assert_eq!(session.shell().mode(), Mode::Clean);
    }

    #[test]
    fn accessible_control_ids_and_labels() {
        let english_clean = accessible_control(Locale::En, AccessibleSlot::Mode(Mode::Clean));
        assert_eq!(english_clean.id, "mode-clean");
        assert_eq!(english_clean.label, "Clean");

        let chinese_clean = accessible_control(Locale::ZhCn, AccessibleSlot::Mode(Mode::Clean));
        assert_eq!(chinese_clean.id, "mode-clean");
        assert_eq!(chinese_clean.label, "清理");

        let shell = accessible_control(Locale::En, AccessibleSlot::Shell);
        assert_eq!(shell.id, "shell-root");
        assert_eq!(shell.label, "DevSweep");

        let note = accessible_control(Locale::En, AccessibleSlot::Note);
        assert_eq!(note.id, "note-input");
        assert_eq!(note.label, "Note");
    }

    #[test]
    fn panel_lines_use_their_text_as_accessible_names() {
        let source = include_str!("view.rs");
        let marker = r#".id(gpui::ElementId::named_usize("panel-line", index))"#;
        let start = source.find(marker).expect("panel-line row div must exist");
        let snippet = &source[start..];
        let role = snippet
            .find(".role(gpui::Role::ListItem)")
            .expect("panel-line div must set Role::ListItem");
        let aria = snippet
            .find(".aria_label(SharedString::from(line.clone()))")
            .expect("panel-line div must set aria_label from line.clone()");
        let child = snippet
            .find(".child(SharedString::from(line))")
            .expect("panel-line div must still paint line as the child");
        assert!(
            !snippet[..child].contains("GenericContainer"),
            "panel-line div must not set GenericContainer"
        );
        assert!(role < aria, "Role::ListItem must come before aria_label");
        assert!(
            aria < child,
            "aria_label from line.clone() must come before the child"
        );
    }

    #[test]
    fn alt_down_moves_clean_selection_without_changing_mode() {
        let source = include_str!("view.rs");
        assert!(
            source.contains(r#"KeyBinding::new("alt-down", RowDown, None)"#),
            "alt-down must bind RowDown with context None"
        );

        let mut session = Session::unbacked();
        session.set_row_ids(
            Mode::Clean,
            vec![
                "clean-0".to_string(),
                "clean-1".to_string(),
                "clean-2".to_string(),
            ],
        );
        session.apply_key(NavKey::RowDown);
        session.apply_key(NavKey::RowDown);
        assert_eq!(session.shell().selection(Mode::Clean), Some("clean-1"));
        assert_eq!(session.shell().mode(), Mode::Clean);
        session.apply_key(NavKey::Down);
        assert_eq!(session.shell().mode(), Mode::Software);
    }

    #[test]
    fn clean_review_keys_are_bound_on_the_note_and_startup_keeps_note_focus() {
        let source = include_str!("view.rs");
        let run_at = source
            .find("pub fn run(session: Session)")
            .expect("run must own the binding list and startup focus");
        let tests_at = source[run_at..]
            .find("\n#[cfg(test)]\n")
            .map(|offset| run_at + offset)
            .expect("tests follow run");
        let run = &source[run_at..tests_at];
        for binding in [
            r#"KeyBinding::new("space", ToggleRow, Some("SpikeInput"))"#,
            r#"KeyBinding::new("space", ToggleRow, Some("CleanReview"))"#,
            r#"KeyBinding::new("tab", TabCleanReview, Some("SpikeInput"))"#,
            r#"KeyBinding::new("shift-tab", ShiftTabCleanReview, Some("SpikeInput"))"#,
            r#"KeyBinding::new("f7", CycleCleanSort, None)"#,
        ] {
            assert!(run.contains(binding), "missing binding: {binding}");
        }
        assert!(
            run.contains("window.focus(&view.note.focus_handle(cx), cx)"),
            "startup must focus the Note"
        );
        assert!(
            !run.contains("search.focus_handle"),
            "startup must not focus the search field"
        );

        let session = Session::unbacked();
        assert_eq!(session.shell().mode(), Mode::Clean);
        let hint = copy::clean_keys_hint(Locale::En).to_string();
        assert!(
            panel_lines(&session, None).contains(&hint),
            "the Clean hint must be visible on the mode surface"
        );
    }

    #[test]
    fn mode_chord_routes_to_the_mode_action_while_the_note_owns_the_caret() {
        let mut session = Session::unbacked();
        assert_eq!(session.shell().mode(), Mode::Clean);

        // The Note's SpikeInput context keeps plain `left`/`right` for the caret,
        // so the mode chord must be a different keystroke that still resolves to
        // ModeNext/ModePrev on the shell's dispatch path.
        assert!(
            route_mode_action(&ModeNext, &mut session),
            "ModeNext must be a mode action"
        );
        assert_eq!(session.shell().mode(), Mode::Software);
        assert!(route_mode_action(&ModeNext, &mut session));
        assert_eq!(session.shell().mode(), Mode::Optimize);
        assert!(
            route_mode_action(&ModePrev, &mut session),
            "ModePrev must be a mode action"
        );
        assert_eq!(session.shell().mode(), Mode::Software);
        assert!(route_mode_action(&ModePrev, &mut session));
        assert_eq!(session.shell().mode(), Mode::Clean);

        // An unrelated shell action is not a mode action and must not move mode.
        let before = session.shell().mode();
        assert!(
            !route_mode_action(&MenuToggle, &mut session),
            "MenuToggle must not route as a mode action"
        );
        assert_eq!(session.shell().mode(), before);
        assert!(
            !session.shell().menu_open(),
            "routing must not toggle the menu"
        );
        assert!(!route_mode_action(&RowDown, &mut session));
        assert_eq!(session.shell().mode(), before);
        assert!(!route_mode_action(&Back, &mut session));
        assert_eq!(session.shell().mode(), before);
    }

    #[test]
    fn mode_chord_is_bound_in_the_note_context_and_reaches_the_shell_handlers() {
        let source = include_str!("view.rs");
        for binding in [
            r#"KeyBinding::new("alt-left", ModePrev, Some("SpikeInput"))"#,
            r#"KeyBinding::new("alt-right", ModeNext, Some("SpikeInput"))"#,
        ] {
            assert!(source.contains(binding), "missing chord binding: {binding}");
        }
        // The caret keeps the bare arrows inside the Note.
        for binding in [
            r#"KeyBinding::new("left", input::Left, Some("SpikeInput"))"#,
            r#"KeyBinding::new("right", input::Right, Some("SpikeInput"))"#,
        ] {
            assert!(source.contains(binding), "caret binding lost: {binding}");
        }

        // The chord resolves to ModePrev/ModeNext, which no Note element handles.
        // They bubble to the shell root, so the Note must be rendered as a
        // descendant of the element that carries the prev/next listeners.
        let root = source
            .find(r#".id("shell-root")"#)
            .expect("shell root element must exist");
        let note = source
            .find(".child(self.note.clone())")
            .expect("shell root must render the Note as a child");
        assert!(root < note, "the Note must sit inside the shell root");
        for handler in [
            ".on_action(cx.listener(Self::prev))",
            ".on_action(cx.listener(Self::next))",
        ] {
            let at = source
                .find(handler)
                .unwrap_or_else(|| panic!("shell root must register {handler}"));
            assert!(
                root < at && at < note,
                "{handler} must be on the shell root above the Note"
            );
        }
        // The Note must not swallow the chord on its own element. The Note and
        // the Clean search share this render path, and the constructor pins
        // the Note element identity.
        let note_source = include_str!("input.rs");
        assert!(
            note_source.contains(r#"element_id: "note-input""#),
            "the Note constructor must keep the note-input element identity"
        );
        let note_start = note_source
            .find(".id(self.element_id)")
            .expect("note element must exist");
        let note_element = &note_source[note_start..];
        let note_end = note_element
            .find(".on_mouse_down")
            .expect("note element must end at its mouse handler");
        assert!(
            !note_element[..note_end].contains("ModePrev")
                && !note_element[..note_end].contains("ModeNext"),
            "the Note must not claim the mode chord"
        );
    }

    #[test]
    fn mode_keys_hint_renders_in_every_panel() {
        let mut session = Session::unbacked();
        assert_eq!(session.locale(), Locale::En);
        let hint = copy::mode_keys_hint(Locale::En);
        assert!(hint.contains("Alt+Left"));
        assert!(hint.contains("Alt+Right"));
        for surface in [Surface::Mode, Surface::Settings, Surface::Protection] {
            session.show_mode(Mode::Clean);
            match surface {
                Surface::Mode => {}
                other => {
                    let index = MENU
                        .iter()
                        .position(|item| *item == other)
                        .expect("surface is in MENU");
                    session.toggle_menu();
                    session.open_menu_item(index);
                }
            }
            assert_eq!(session.shell().surface(), surface);
            let lines = panel_lines(&session, None);
            assert_eq!(
                lines.first(),
                Some(&hint.to_string()),
                "the chord hint must lead the {surface:?} panel: {lines:?}"
            );
            assert_eq!(
                lines.iter().filter(|line| *line == hint).count(),
                1,
                "the chord hint must appear once in {surface:?}: {lines:?}"
            );
        }

        session.cycle_language();
        assert_eq!(session.locale(), Locale::ZhCn);
        let zh = copy::mode_keys_hint(Locale::ZhCn);
        assert_ne!(zh, hint);
        assert!(zh.contains("Alt+"));
        assert_eq!(panel_lines(&session, None).first(), Some(&zh.to_string()));
    }

    #[test]
    fn f10_then_enter_opens_reduced_motion_settings() {
        let source = include_str!("view.rs");
        assert!(
            source.contains(r#"KeyBinding::new("f10", MenuToggle, None)"#),
            "f10 must bind MenuToggle with context None"
        );
        assert!(
            source.contains(r#"KeyBinding::new("down", MenuDown, Some("SpikeInput"))"#),
            "down must also bind while the Note holds SpikeInput focus"
        );
        assert!(
            source.contains(r#"KeyBinding::new("f8", OpenAiReview, Some("SpikeInput"))"#),
            "f8 must open AI review without scanning"
        );
        assert!(
            source.contains(r#"KeyBinding::new("enter", Activate, None)"#),
            "enter must bind Activate with context None"
        );

        let mut session = Session::unbacked();
        session.set_motion(DesktopMotion::Reduced);
        assert_eq!(MENU[0], Surface::Settings);
        session.toggle_menu();
        session.apply_key(NavKey::Enter);
        assert_eq!(session.shell().surface(), Surface::Settings);
        let text = panel_lines(&session, None).join("\n");
        assert!(
            text.to_ascii_lowercase().contains("reduced"),
            "settings panel text must contain reduced: {text}"
        );
    }

    #[test]
    fn found_scan_summary_names_the_disk_map() {
        let mut session = Session::unbacked();
        session.install_clean_rows_for_test(vec![
            CleanRow {
                id: "modules".to_string(),
                plan_index: 0,
                rule_id: "node.node_modules".to_string(),
                provider: "node".to_string(),
                scope: crate::clean_workbench::CleanRowScope::Project {
                    root: "C:/work/app".to_string(),
                },
                path: Some("C:/work/app/node_modules".to_string()),
                bytes: 13_800_000_000,
                size_complete: true,
                inspect_only: false,
                selected_by_default: false,
            },
            CleanRow {
                id: "target".to_string(),
                plan_index: 1,
                rule_id: "rust.target".to_string(),
                provider: "rust".to_string(),
                scope: crate::clean_workbench::CleanRowScope::Project {
                    root: "C:/work/app".to_string(),
                },
                path: Some("C:/work/app/target".to_string()),
                bytes: 4_400_000_000,
                size_complete: false,
                inspect_only: false,
                selected_by_default: true,
            },
        ]);
        let lines = panel_lines(&session, None);
        let summary = lines
            .iter()
            .find(|line| line.starts_with("Disk map:"))
            .expect("disk map summary");
        assert!(summary.contains("2 targets"), "{summary}");
        assert!(summary.contains("1 recommended"), "{summary}");
        assert!(summary.contains("1 to review"), "{summary}");
        assert!(summary.contains("1 size is a lower bound."), "{summary}");
        assert!(lines.iter().any(|line| line.contains("node.node_modules")));
        assert!(lines.iter().any(|line| line.contains("rust.target")));
        session.cycle_language();
        let zh = panel_lines(&session, None)
            .into_iter()
            .find(|line| line.starts_with("磁盘地图"))
            .expect("chinese disk map summary");
        assert!(zh.contains("建议清理"), "{zh}");
        assert!(zh.contains("需要复查"), "{zh}");
    }

    #[test]
    fn disk_map_summary_does_not_replace_the_outcome_line() {
        let mut session = Session::unbacked();
        session.install_clean_rows_for_test(vec![CleanRow {
            id: "target".to_string(),
            plan_index: 0,
            rule_id: "rust.target".to_string(),
            provider: "rust".to_string(),
            scope: crate::clean_workbench::CleanRowScope::Project {
                root: "C:/work/app".to_string(),
            },
            path: Some("C:/work/app/target".to_string()),
            bytes: 1024,
            size_complete: true,
            inspect_only: false,
            selected_by_default: true,
        }]);
        session.set_clean_outcome(Some(CleanOutcomeNotice::StalePreview));
        let lines = panel_lines(&session, None);
        assert!(lines.iter().any(|line| line.starts_with("Disk map:")));
        assert_eq!(
            lines.last().map(String::as_str),
            Some(copy::clean_outcome_stale_preview(Locale::En))
        );
    }

    fn assert_distinct(lines: &[String]) {
        for (index, line) in lines.iter().enumerate() {
            assert_eq!(
                lines.iter().filter(|other| *other == line).count(),
                1,
                "duplicate notice line at {index}: {line}"
            );
        }
        assert_eq!(lines.len(), 5);
    }
}
