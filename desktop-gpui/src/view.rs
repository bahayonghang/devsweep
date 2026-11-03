//! One native window. The capsule order is Clean, Software, Optimize, Analyze, Status.
//!
//! Layout: a top bar (beaver menu, centered mode capsule, screen toggles), a
//! scrollable page built from the `ui` design system, and a status bar that
//! holds the Note. Each page lives in a `view/page_*.rs` submodule.
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
    FontWeight, KeyBinding, PathPromptOptions, SharedString, TitlebarOptions, Window, WindowBounds,
    WindowOptions, actions, div, img, prelude::*, px, rems, rgb, size,
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
    input::{self, InputStyle, TextInput},
    preferences::DesktopTheme,
    shell::{Activity, CleanOutcomeNotice, MENU, Mode, Session, Surface, WorkbenchNotice},
    ui::{self, ButtonKind, Theme, Tone},
};

mod page_analyze;
mod page_clean;
mod page_status;
mod page_support;
mod page_workbench;

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
            other => Ok(ui::icon_bytes(other).map(Cow::Borrowed)),
        }
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut names = vec![
            SharedString::from(ICON_NAME),
            SharedString::from(COPY_NAME),
            SharedString::from(EARTH_NAME),
        ];
        names.extend(
            ui::ICONS
                .iter()
                .map(|name| SharedString::from(format!("icons/{name}.svg"))),
        );
        Ok(names)
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

    fn toggle_keep_screen(&mut self, _: &ToggleKeepScreen, _: &mut Window, cx: &mut Context<Self>) {
        if self.session.stored_lease_count() > 0 {
            let _ = self.session.stop_screen_on();
        } else {
            let service =
                std::sync::Arc::new(devsweep_core::status::KeepAwakeService::production());
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

    fn analyze_pick_root(&mut self, cx: &mut Context<Self>) {
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
                let _ = this.session.analyze_open_isolated_root(root);
                cx.notify();
            });
        })
        .detach();
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let high_contrast = system_high_contrast();
        let t = Theme::for_preferences(self.session.theme(), high_contrast);
        // Spacing and type are rem based, so the stored text scale scales the
        // whole layout, not only the root font.
        window.set_rem_size(px(16.0 * f32::from(self.session.text_scale()) / 100.0));
        self.sync_inputs(&t, cx);
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
        if selected == Mode::Analyze {
            self.session.prepare_analyze_landing();
        }
        self.dispatch_jobs(cx);
        let status_main = selected == Mode::Status && surface == Surface::Mode;
        self.session.status_set_main_visible(status_main);
        let review_surface = self.clean_review_active();
        let key_context = if menu_open {
            "ShellMenu"
        } else if review_surface {
            "ShellRoot CleanReview"
        } else {
            "ShellRoot"
        };
        let shell_access = accessible_control(locale, AccessibleSlot::Shell);
        let top_bar = self.top_bar(&t, locale, selected, surface, menu_open, cx);
        let page = self.page(&t, cx);
        let busy =
            self.session.shell().activity() == Activity::Busy || self.session.background_busy();
        let hint = copy::mode_keys_hint(locale);
        let note_label = copy::pick(locale, "Note", "笔记");

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
            // A click on a non-focusable surface must not strand the keyboard:
            // with nothing focused, return focus to the Note so every chord
            // keeps working. A click on the search field focuses it first.
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if window.focused(cx).is_none() {
                        window.focus(&this.note.focus_handle(cx), cx);
                    }
                }),
            )
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(t.canvas))
            .text_color(rgb(t.text))
            .text_size(rems(0.9375))
            .child(top_bar)
            .child(
                div()
                    .id("page-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div().w_full().flex().flex_col().items_center().child(
                            div()
                                .id("mode-panel")
                                .role(gpui::Role::TabPanel)
                                .aria_label(panel_title(&self.session))
                                .w_full()
                                .max_w(rems(76.0))
                                .px(rems(2.0))
                                .pt(rems(1.75))
                                .pb(rems(3.0))
                                .flex()
                                .flex_col()
                                .gap(rems(1.25))
                                .child(page),
                        ),
                    ),
            )
            .child(
                div()
                    .id("status-bar")
                    .flex()
                    .flex_row()
                    .flex_shrink_0()
                    .items_center()
                    .gap(rems(1.0))
                    .h(rems(3.0))
                    .px(rems(1.25))
                    .bg(rgb(t.chrome))
                    .border_t_1()
                    .border_color(rgb(t.border))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_shrink_0()
                            .items_center()
                            .gap(rems(0.45))
                            .child(ui::dot(if busy { t.warning } else { t.success }))
                            .child(
                                div()
                                    .text_size(rems(0.78))
                                    .text_color(rgb(t.text_muted))
                                    .child(if busy {
                                        copy::pick(locale, "Working…", "正在处理…")
                                    } else {
                                        copy::pick(locale, "Ready", "就绪")
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .id("keys-hint")
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(rems(0.74))
                            .text_color(rgb(t.text_subtle))
                            .aria_label(hint)
                            .child(hint),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_shrink_0()
                            .items_center()
                            .gap(rems(0.5))
                            .w(rems(17.0))
                            .child(ui::icon("history", t.text_subtle).size(rems(0.85)))
                            .child(
                                div()
                                    .text_size(rems(0.74))
                                    .text_color(rgb(t.text_subtle))
                                    .child(note_label),
                            )
                            .child(div().flex_1().min_w_0().child(self.note.clone())),
                    ),
            )
            .when(menu_open, |root| {
                root.child(self.brand_menu(&t, locale, menu_index, cx))
            })
            .when(self.session.screen_covered(), |root| {
                root.child(
                    div()
                        .id("clean-screen-cover")
                        .role(gpui::Role::Status)
                        .aria_label(copy::clean_screen_active(locale))
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(rgb(0x000000))
                        .flex()
                        .items_end()
                        .justify_center()
                        .pb(rems(2.0))
                        .child(
                            div()
                                .text_size(rems(0.8))
                                .text_color(rgb(0x5A6274))
                                .child(copy::clean_screen_active(locale)),
                        ),
                )
            })
    }
}

impl ShellView {
    /// Runs queued read-only jobs (host software lists, Status snapshots,
    /// Analyze walks) on worker threads and applies each result back on the
    /// UI thread. Session generations drop superseded results.
    fn dispatch_jobs(&mut self, cx: &mut Context<Self>) {
        for job in self.session.take_jobs() {
            let slot: std::sync::Arc<std::sync::Mutex<Option<crate::shell::JobResult>>> =
                std::sync::Arc::default();
            let writer = slot.clone();
            let spawned = std::thread::Builder::new()
                .name("devsweep-gpui-job".to_string())
                .spawn(move || {
                    let result = job.run();
                    if let Ok(mut guard) = writer.lock() {
                        *guard = Some(result);
                    }
                });
            if spawned.is_err() {
                continue;
            }
            cx.spawn(async move |this, cx| {
                loop {
                    let ready = slot.lock().ok().and_then(|mut guard| guard.take());
                    if let Some(result) = ready {
                        let _ = this.update(cx, |this, cx| {
                            this.session.apply_job(result);
                            cx.notify();
                        });
                        break;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(40))
                        .await;
                }
            })
            .detach();
        }
    }

    /// Keeps both text fields on the app scale, locale, and design tokens.
    fn sync_inputs(&self, t: &Theme, cx: &mut Context<Self>) {
        let percent = self.session.text_scale();
        let locale = self.session.locale();
        let base = InputStyle {
            background: t.surface_alt,
            border: t.border,
            focus_border: t.accent,
            text: t.text,
            placeholder: t.text_subtle,
            caret: t.accent,
            selection: (t.accent << 8) | 0x55,
            font_px: 14.0,
            line_px: 20.0,
        };
        self.note.update(cx, |note, cx| {
            note.set_text_scale_percent(percent, cx);
            note.set_placeholder(copy::note_placeholder(locale), cx);
            note.set_style(
                InputStyle {
                    background: t.canvas,
                    font_px: 12.5,
                    line_px: 17.0,
                    ..base
                },
                cx,
            );
        });
        self.search.update(cx, |search, cx| {
            search.set_text_scale_percent(percent, cx);
            search.set_style(base, cx);
            search.set_placeholder(copy::clean_search_placeholder(locale), cx);
        });
    }

    fn top_bar(
        &self,
        t: &Theme,
        locale: copy::Locale,
        selected: Mode,
        surface: Surface,
        menu_open: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let tabs = Mode::ORDER
            .into_iter()
            .map(|mode| {
                let access = accessible_control(locale, AccessibleSlot::Mode(mode));
                let active = mode == selected && surface == Surface::Mode;
                let hover_text = t.text;
                div()
                    .id(mode.element_id())
                    .role(gpui::Role::Tab)
                    .accessibility_id(access.id)
                    .aria_label(access.label)
                    .aria_selected(active)
                    .when(active, |tab| tab.aria_active_descendant())
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.45))
                    .h(rems(2.1))
                    .px(rems(0.95))
                    .rounded_full()
                    .text_size(rems(0.85))
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .when(active, |tab| {
                        tab.bg(rgb(t.pill)).text_color(rgb(t.pill_text))
                    })
                    .when(!active, |tab| {
                        tab.text_color(rgb(t.text_muted))
                            .hover(move |s| s.text_color(rgb(hover_text)))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session.show_mode(mode);
                        cx.notify();
                    }))
                    .child(
                        ui::icon(
                            mode_icon(mode),
                            if active { t.pill_text } else { t.text_muted },
                        )
                        .size(rems(0.95)),
                    )
                    .child(access.label)
            })
            .collect::<Vec<_>>();
        let keep_on = self.session.stored_lease_count() > 0;
        let toggle = |id: &'static str, icon: &'static str, label: &'static str, on: bool| {
            let border = if on { t.accent } else { t.border };
            let color = if on { t.accent } else { t.text_muted };
            let hover_bg = t.surface_alt;
            div()
                .id(id)
                .role(gpui::Role::Button)
                .aria_label(label)
                .aria_toggled(if on {
                    gpui::Toggled::True
                } else {
                    gpui::Toggled::False
                })
                .flex()
                .items_center()
                .justify_center()
                .size(rems(2.2))
                .rounded_full()
                .border_1()
                .border_color(rgb(border))
                .when(on, |b| b.bg(t.tint(t.accent, 0x1E)))
                .cursor_pointer()
                .hover(move |s| s.bg(rgb(hover_bg)))
                .tooltip(move |_, cx| cx.new(|_| TooltipText(label.into())).into())
                .child(ui::icon(icon, color).size(rems(1.0)))
        };
        div()
            .id("top-bar")
            .flex()
            .flex_row()
            .flex_shrink_0()
            .items_center()
            .h(rems(3.75))
            .px(rems(1.25))
            .bg(rgb(t.chrome))
            .border_b_1()
            .border_color(rgb(t.border))
            .child(
                div().flex_1().flex().flex_row().items_center().child(
                    div()
                        .id("beaver-menu")
                        .role(gpui::Role::Button)
                        .aria_label(copy::menu_label(locale))
                        .aria_expanded(menu_open)
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(rems(0.55))
                        .h(rems(2.3))
                        .pl(rems(0.3))
                        .pr(rems(0.6))
                        .rounded(rems(0.6))
                        .cursor_pointer()
                        .when(menu_open, |b| b.bg(rgb(t.surface_alt)))
                        .hover(|s| s.bg(rgb(t.surface_alt)))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.toggle_menu();
                            cx.notify();
                        }))
                        .child(
                            img(ICON_NAME)
                                .id("beaver-icon")
                                .size(rems(1.75))
                                .with_fallback(|| div().child("DevSweep").into_any_element()),
                        )
                        .child(
                            div()
                                .text_size(rems(0.95))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(WINDOW_TITLE),
                        )
                        .child(
                            ui::icon("chevron-right", t.text_subtle)
                                .size(rems(0.8))
                                .when(menu_open, |i| i.text_color(rgb(t.text))),
                        ),
                ),
            )
            .child(
                div()
                    .id("mode-tabs")
                    .role(gpui::Role::TabList)
                    .aria_label(copy::modes_label(locale))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.15))
                    .p(rems(0.25))
                    .rounded_full()
                    .bg(rgb(t.surface))
                    .border_1()
                    .border_color(rgb(t.border))
                    .children(tabs),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .items_center()
                    .gap(rems(0.5))
                    .child(
                        toggle(
                            "keep-screen-on",
                            "sun",
                            copy::keep_screen_on_label(locale),
                            keep_on,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_keep_screen_on();
                            cx.notify();
                        })),
                    )
                    .child(
                        toggle(
                            "clean-screen",
                            "monitor",
                            copy::clean_screen_label(locale),
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.session.begin_clean_screen();
                            cx.notify();
                        })),
                    ),
            )
            .into_any_element()
    }

    fn brand_menu(
        &self,
        t: &Theme,
        locale: copy::Locale,
        menu_index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let items = MENU
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let access = accessible_control(locale, AccessibleSlot::Menu(item));
                let highlighted = menu_index == index;
                let hover_bg = t.surface_alt;
                div()
                    .id(surface_element_id(item))
                    .role(gpui::Role::MenuItem)
                    .accessibility_id(access.id)
                    .aria_label(access.label)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(rems(0.65))
                    .px(rems(0.7))
                    .py(rems(0.5))
                    .rounded(rems(0.5))
                    .cursor_pointer()
                    .when(highlighted, |row| row.bg(rgb(t.surface_active)))
                    .when(!highlighted, |row| row.hover(move |s| s.bg(rgb(hover_bg))))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session.open_menu_item(index);
                        cx.notify();
                    }))
                    .child(ui::icon(surface_icon(item), t.text_muted))
                    .child(div().flex_1().text_size(rems(0.875)).child(access.label))
            })
            .collect::<Vec<_>>();
        div()
            .absolute()
            .top(rems(3.75))
            .left_0()
            .size_full()
            .child(
                div()
                    .id("brand-menu-backdrop")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.session.toggle_menu();
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("brand-menu")
                    .role(gpui::Role::Menu)
                    .aria_label(copy::menu_label(locale))
                    .absolute()
                    .top(rems(0.35))
                    .left(rems(1.0))
                    .w(rems(15.0))
                    .flex()
                    .flex_col()
                    .gap(rems(0.1))
                    .p(rems(0.35))
                    .bg(rgb(t.surface))
                    .border_1()
                    .border_color(rgb(t.border_strong))
                    .rounded(rems(0.8))
                    .shadow_lg()
                    .children(items),
            )
            .into_any_element()
    }

    /// The page body for the active surface.
    fn page(&mut self, t: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        match self.session.shell().surface() {
            Surface::Mode => match self.session.shell().mode() {
                Mode::Clean => self.clean_page(t, cx),
                Mode::Software => self.software_page(t, cx),
                Mode::Optimize => self.optimize_page(t, cx),
                Mode::Analyze => self.analyze_page(t, cx),
                Mode::Status => self.status_page(t, cx),
            },
            Surface::Settings => self.settings_page(t, cx),
            Surface::Protection => self.protection_page(t, cx),
            Surface::Rules => self.rules_page(t, cx),
            Surface::History => self.history_page(t, cx),
        }
    }

    fn toggle_keep_screen_on(&mut self) {
        if self.session.stored_lease_count() > 0 {
            let _ = self.session.stop_screen_on();
        } else {
            let service =
                std::sync::Arc::new(devsweep_core::status::KeepAwakeService::production());
            let _ = self.session.start_screen_on(
                service,
                "gpui-window",
                std::time::Duration::from_secs(30 * 60),
            );
        }
    }

    /// Session-wide banners shared by every page: preserved stores, a save
    /// error, stale review notices, and the keep-awake lease.
    fn shared_banners(&self, t: &Theme) -> Vec<gpui::AnyElement> {
        let locale = self.session.locale();
        let mut banners = Vec::new();
        for notice in self.session.notices() {
            banners.push(ui::any(ui::banner(
                t,
                Tone::Warning,
                copy::preserved(locale, copy::store_name(locale, *notice)),
            )));
        }
        if let Some(error) = self.save_error.as_deref() {
            banners.push(ui::any(ui::banner(t, Tone::Danger, error.to_string())));
        }
        if self.session.shell().surface() == Surface::Mode {
            let mode = self.session.shell().mode();
            match self.session.shell().review(mode) {
                crate::shell::Review::StaleScanRoots => banners.push(ui::any(ui::banner(
                    t,
                    Tone::Warning,
                    copy::stale_scan_roots(locale),
                ))),
                crate::shell::Review::StaleProtection => banners.push(ui::any(ui::banner(
                    t,
                    Tone::Warning,
                    copy::stale_protection(locale),
                ))),
                crate::shell::Review::Valid => {}
            }
            if let Some(report) = self.session.shell().report(mode) {
                banners.push(ui::any(ui::banner(t, Tone::Info, report.to_string())));
            }
        }
        if let Some(line) = self.session.screen_lease_line(locale) {
            banners.push(ui::any(ui::banner(t, Tone::Info, line)));
        }
        banners
    }

    /// Standard page header: icon tile, title, subtitle, trailing actions.
    fn page_header(
        &self,
        t: &Theme,
        icon: &'static str,
        title: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
        actions: Vec<gpui::AnyElement>,
    ) -> gpui::AnyElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(rems(1.0))
            .child(ui::icon_tile(t, icon, t.accent).size(rems(2.75)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(rems(0.15))
                    .child(
                        div()
                            .text_size(rems(1.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title.into()),
                    )
                    .child(
                        div()
                            .text_size(rems(0.88))
                            .text_color(rgb(t.text_muted))
                            .child(subtitle.into()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_shrink_0()
                    .items_center()
                    .gap(rems(0.5))
                    .children(actions),
            )
            .into_any_element()
    }

    /// One Status snapshot, the same path as Enter on the Status surface.
    fn status_refresh(&mut self, cx: &mut Context<Self>) {
        if self.session.shell().mode() == Mode::Status {
            self.session.apply_key(crate::shell::NavKey::Enter);
        }
        cx.notify();
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

#[cfg(test)]
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

/// Plain tooltip body for icon-only controls.
struct TooltipText(SharedString);

impl Render for TooltipText {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(rems(0.5))
            .py(rems(0.25))
            .rounded(rems(0.4))
            .bg(rgb(0x232D4A))
            .text_color(rgb(0xEEF1F7))
            .text_size(rems(0.75))
            .child(self.0.clone())
    }
}

fn mode_icon(mode: Mode) -> &'static str {
    match mode {
        Mode::Clean => "clean",
        Mode::Software => "software",
        Mode::Optimize => "optimize",
        Mode::Analyze => "analyze",
        Mode::Status => "status",
    }
}

fn surface_icon(surface: Surface) -> &'static str {
    match surface {
        Surface::Mode => "clean",
        Surface::Settings => "settings",
        Surface::Protection => "shield",
        Surface::Rules => "rules",
        Surface::History => "history",
    }
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

#[cfg(test)]
/// One entry of the text projection of the active surface. Clean and AI rows
/// carry the metadata the designed pages use for their interactive rows.
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

#[cfg(test)]
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

/// The panel as text lines, derived from [`panel_entries`].
///
/// The window paints designed pages (cards, tables, tiles) from the same
/// session state; this projection keeps every fact those pages show as one
/// line each so tests can assert state, notices and row names without a
/// window. Interactive rows reuse these exact strings as accessible names.
#[cfg(test)]
pub(crate) fn panel_lines(session: &Session, save_error: Option<&str>) -> Vec<String> {
    panel_entries(session, save_error)
        .into_iter()
        .map(|entry| entry.text().to_string())
        .collect()
}

#[cfg(test)]
fn mode_row_text(session: &Session, mode: Mode, id: &str, focused: bool) -> String {
    let locale = session.locale();
    if mode == Mode::Software
        && let Some(line) = session.software_review().row_line(id, locale, focused)
    {
        return line;
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

#[cfg(test)]
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
            if matches!(
                mode,
                Mode::Clean | Mode::Software | Mode::Optimize | Mode::Analyze
            ) {
                let visible = shell.visible_row_ids(mode);
                let cursor = matches!(
                    mode,
                    Mode::Clean | Mode::Software | Mode::Optimize | Mode::Analyze
                )
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
                    if let Some(notice) = session.analyze_notice() {
                        entries.push(PanelEntry::Text(notice.to_string()));
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
                entries.push(PanelEntry::Text(
                    session.optimize_board().phase_line(locale),
                ));
                for line in session.optimize_board().group_lines(locale) {
                    entries.push(PanelEntry::Text(line));
                }
                for line in crate::workbench::optimize_row_lines(locale) {
                    entries.push(PanelEntry::Text(line));
                }
            }
            if mode == Mode::Analyze && session.analyze_window().is_none() {
                if session.analyze_roots().is_empty() && session.analyze_landing_prepared() {
                    entries.push(PanelEntry::Text(
                        copy::analyze_empty_title(locale).to_string(),
                    ));
                    entries.push(PanelEntry::Text(
                        copy::analyze_empty_detail(locale).to_string(),
                    ));
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

/// Review-host override of the initial window size (`WIDTHxHEIGHT`). Only the
/// opt-in Linux preview build reads it; the Windows product never does.
#[cfg(feature = "linux-preview")]
fn preview_window_size() -> Option<(f32, f32)> {
    let value = std::env::var("DEVSWEEP_PREVIEW_WINDOW").ok()?;
    let (w, h) = value.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

#[cfg(not(feature = "linux-preview"))]
fn preview_window_size() -> Option<(f32, f32)> {
    None
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

            let (width, height) = preview_window_size().unwrap_or((1240.0, 820.0));
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
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
                        let mut session = session;
                        // The window never blocks its UI thread on host reads.
                        session.enable_background_jobs();
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
        let light = Theme::for_preferences(DesktopTheme::Light, false);
        let dark = Theme::for_preferences(DesktopTheme::Dark, false);
        assert_ne!(light.canvas, dark.canvas);
        assert_ne!(light.text, dark.text);
        assert_eq!(light.canvas, 0xf3f5fa);
        assert!(!light.dark);
    }

    #[test]
    fn high_contrast_palette_is_distinct() {
        let high_dark = Theme::for_preferences(DesktopTheme::Dark, true);
        let high_light = Theme::for_preferences(DesktopTheme::Light, true);
        let dark = Theme::for_preferences(DesktopTheme::Dark, false);
        let light = Theme::for_preferences(DesktopTheme::Light, false);

        assert_eq!(high_dark.canvas, 0x000000);
        assert_eq!(high_dark.text, 0xffffff);
        assert_eq!(high_dark, high_light);
        assert_ne!(high_dark, dark);
        assert_ne!(high_dark, light);
        assert_ne!(dark, light);
        // Every themed system tone stays distinct from the high-contrast field.
        for tone in [Tone::Success, Tone::Warning, Tone::Danger, Tone::Info] {
            assert_ne!(high_dark.tone(tone), high_dark.canvas);
        }
    }

    #[test]
    fn c10_ac4_packaged_icon_and_copy_are_offline() {
        let icon = PackagedAssets.load(ICON_NAME).unwrap().expect("icon");
        assert!(icon.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(PackagedAssets.load("mole-planet.png").unwrap().is_none());
        let listed = PackagedAssets.list("").unwrap();
        assert_eq!(
            &listed[..3],
            &[
                SharedString::from(ICON_NAME),
                SharedString::from(COPY_NAME),
                SharedString::from(EARTH_NAME)
            ]
        );
        // Everything else is an original packaged stroke icon.
        for extra in &listed[3..] {
            assert!(
                extra.starts_with("icons/") && extra.ends_with(".svg"),
                "{extra}"
            );
            assert!(PackagedAssets.load(extra).unwrap().is_some(), "{extra}");
        }
        assert_eq!(listed.len(), 3 + ui::ICONS.len());
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
    fn interactive_rows_use_their_row_text_as_accessible_names() {
        // The designed rows paint structured cells, but their accessible name
        // stays the full row line (selection, rebuild cost, protection, ignore,
        // confidence), so UIA and the text projection read the same facts.
        let source = include_str!("view/page_clean.rs");
        let row = source
            .find("fn clean_row(")
            .expect("the Clean row builder must exist");
        let snippet = &source[row..];
        let text = snippet
            .find("crate::clean_workbench::row_text(")
            .expect("the row name must come from row_text");
        let aria = snippet
            .find(".aria_label(SharedString::from(text))")
            .expect("the row must use that text as its accessible name");
        let list_item = include_str!("ui.rs")
            .find(".role(gpui::Role::ListItem)")
            .expect("list rows must keep Role::ListItem");
        assert!(text < aria);
        assert!(list_item > 0);
        let ai = include_str!("view/page_clean.rs");
        assert!(ai.contains("let text = view.row_text(locale, id, focused)"));
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
