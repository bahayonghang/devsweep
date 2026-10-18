//! One native window. The capsule order is Clean, Software, Optimize, Analyze, Status.
//!
//! The beaver control uses the packaged DevSweep master PNG. No asset conversion
//! and no Mole artwork. Render does not request an animation frame.

use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use gpui::{
    App, AssetSource, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions, actions, div, img, prelude::*, px, rgb,
    size,
};
use gpui_platform::application;
#[cfg(windows)]
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};

use crate::{
    copy,
    input::{self, TextInput},
    preferences::DesktopTheme,
    shell::{Activity, CleanOutcomeNotice, MENU, Mode, Session, Surface, WorkbenchNotice},
};

const ICON_NAME: &str = "devsweep-icon-master.png";
const COPY_NAME: &str = "copy.txt";
const ICON_BYTES: &[u8] = include_bytes!("../assets/devsweep-icon-master.png");
const COPY_BYTES: &[u8] = include_bytes!("../assets/copy.txt");
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
        Back,
        Activate,
        MenuToggle,
        RequestProjectRows
    ]
);

pub struct PackagedAssets;

impl AssetSource for PackagedAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        match path {
            ICON_NAME => Ok(Some(Cow::Borrowed(ICON_BYTES))),
            COPY_NAME => Ok(Some(Cow::Borrowed(COPY_BYTES))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(vec![
            SharedString::from(ICON_NAME),
            SharedString::from(COPY_NAME),
        ])
    }
}

pub struct ShellView {
    focus_handle: FocusHandle,
    session: Session,
    save_error: Option<String>,
    note: Entity<TextInput>,
}

impl ShellView {
    fn prev(&mut self, _: &ModePrev, _: &mut Window, cx: &mut Context<Self>) {
        self.session.apply_key(crate::shell::NavKey::Left);
        cx.notify();
    }

    fn next(&mut self, _: &ModeNext, _: &mut Window, cx: &mut Context<Self>) {
        self.session.apply_key(crate::shell::NavKey::Right);
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

    fn back(&mut self, _: &Back, _: &mut Window, cx: &mut Context<Self>) {
        self.session.apply_key(crate::shell::NavKey::Escape);
        cx.notify();
    }

    fn activate(&mut self, _: &Activate, _: &mut Window, cx: &mut Context<Self>) {
        self.session.apply_key(crate::shell::NavKey::Enter);
        cx.notify();
    }

    fn menu_toggle(&mut self, _: &MenuToggle, _: &mut Window, cx: &mut Context<Self>) {
        self.session.toggle_menu();
        cx.notify();
    }

    fn request_project_rows(
        &mut self,
        _: &RequestProjectRows,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.session.request_project_rows() {
            return;
        }
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
                            || started.elapsed() >= Duration::from_secs(10)
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

impl Focusable for ShellView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ShellView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The beaver PNG is one still frame. Do not request a decorative animation frame.
        let _ = self.session.shell().decorative_deadline();
        let locale = self.session.locale();
        let shell = self.session.shell();
        let menu_open = shell.menu_open();
        let selected = shell.mode();
        let surface = shell.surface();
        let menu_index = shell.menu_index();
        let text_size = px(16.0 * f32::from(self.session.text_scale()) / 100.0);
        let palette = shell_palette_for(self.session.theme(), system_high_contrast());
        let tabs = Mode::ORDER
            .into_iter()
            .map(|mode| {
                let label = copy::mode_name(locale, mode.id());
                let active = mode == selected && surface == Surface::Mode;
                div()
                    .id(mode.element_id())
                    .role(gpui::Role::Tab)
                    .aria_label(label)
                    .aria_selected(active)
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
                    .child(label)
            })
            .collect::<Vec<_>>();
        let menu_items = MENU
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let label = surface_title(locale, item);
                div()
                    .id(surface_element_id(item))
                    .role(gpui::Role::MenuItem)
                    .aria_label(label)
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
                    .child(label)
            })
            .collect::<Vec<_>>();
        let lines = panel_lines(&self.session, self.save_error.as_deref());

        div()
            .id("shell-root")
            .role(gpui::Role::Application)
            .aria_label(WINDOW_TITLE)
            .track_focus(&self.focus_handle)
            .key_context(if menu_open { "ShellMenu" } else { "ShellRoot" })
            .on_action(cx.listener(Self::prev))
            .on_action(cx.listener(Self::next))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::row_down))
            .on_action(cx.listener(Self::back))
            .on_action(cx.listener(Self::activate))
            .on_action(cx.listener(Self::menu_toggle))
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
            .child(
                div()
                    .id("mode-panel")
                    .role(gpui::Role::TabPanel)
                    .aria_label(panel_title(&self.session))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(lines.into_iter().enumerate().map(|(index, line)| {
                        div()
                            .id(gpui::ElementId::named_usize("panel-line", index))
                            .aria_label(SharedString::from(line.clone()))
                            .child(SharedString::from(line))
                    })),
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

pub(crate) fn panel_lines(session: &Session, save_error: Option<&str>) -> Vec<String> {
    let locale = session.locale();
    let shell = session.shell();
    let mut lines = Vec::new();
    for notice in session.notices() {
        lines.push(copy::preserved(locale, copy::store_name(locale, *notice)));
    }
    if let Some(error) = save_error {
        lines.push(error.to_string());
    }
    match shell.surface() {
        Surface::Mode => {
            let mode = shell.mode();
            lines.push(copy::mode_name(locale, mode.id()).to_string());
            lines.push(copy::mode_subtitle(locale, mode.id()).to_string());
            if let Some(report) = shell.report(mode) {
                lines.push(report.to_string());
            }
            match shell.review(mode) {
                crate::shell::Review::StaleScanRoots => {
                    lines.push(copy::stale_scan_roots(locale).to_string());
                }
                crate::shell::Review::StaleProtection => {
                    lines.push(copy::stale_protection(locale).to_string());
                }
                crate::shell::Review::Valid => {}
            }
            if matches!(mode, Mode::Clean | Mode::Software | Mode::Optimize) {
                let visible = shell.visible_row_ids(mode);
                let selected = (mode == Mode::Clean)
                    .then(|| shell.selection(mode))
                    .flatten();
                lines.extend(visible.iter().map(|id| {
                    if selected == Some(id.as_str()) {
                        format!("> {id}")
                    } else {
                        id.clone()
                    }
                }));
                if let Some(selection) = selected
                    && !visible.iter().any(|id| id == selection)
                {
                    lines.push(selection.to_string());
                }
            } else if mode == Mode::Analyze
                && let Some(line) = session.analyze_face_line()
            {
                lines.push(line);
            } else if mode == Mode::Status {
                if let Some(line) = session.status_sensor_line() {
                    lines.push(line);
                }
                if let Some(line) = session.status_pid_line() {
                    lines.push(line);
                }
            }
            if matches!(mode, Mode::Software | Mode::Optimize)
                && let Some(notice) = session.workbench_notice()
            {
                lines.push(workbench_notice_line(locale, notice));
            }
            if mode == Mode::Clean
                && let Some(notice) = session.clean_outcome()
            {
                lines.push(clean_outcome_line(locale, notice).to_string());
            }
        }
        Surface::Settings => {
            lines.push(copy::settings_title(locale).to_string());
            lines.push(copy::settings_subtitle(locale).to_string());
            lines.push(format!(
                "{}: {}",
                copy::language_label(locale),
                copy::language_option(locale)
            ));
            lines.push(format!(
                "{}: {}",
                copy::theme_label(locale),
                session.theme().as_str()
            ));
            lines.push(format!("{}%", session.text_scale()));
            lines.push(session.motion().as_str().to_string());
            lines.push(copy::project_folders(locale).to_string());
            lines.extend(shell.scan_roots().iter().cloned());
            lines.push(copy::separate_protection(locale).to_string());
        }
        Surface::Protection => {
            lines.push(copy::protection_title(locale).to_string());
            lines.push(copy::protection_subtitle(locale).to_string());
            lines.extend(shell.protections().iter().cloned());
        }
        Surface::Rules => {
            lines.push(copy::rules_title(locale).to_string());
            lines.push(copy::rules_subtitle(locale).to_string());
            if shell.rules().is_empty() {
                lines.push(copy::rules_empty(locale).to_string());
            } else {
                lines.extend(shell.rules().iter().cloned());
            }
        }
        Surface::History => {
            lines.push(copy::history_title(locale).to_string());
            lines.push(copy::history_subtitle(locale).to_string());
            lines.push(copy::history_no_replay(locale).to_string());
            if shell.history().is_empty() {
                lines.push(copy::history_empty(locale).to_string());
            } else {
                lines.extend(shell.history().iter().cloned());
            }
        }
    }
    lines
}

pub fn run(session: Session) {
    // The tray lives for this blocking run and is dropped when the app exits.
    // Unit tests do not call `run`, so they do not create a notification-area icon.
    #[cfg(windows)]
    let _tray = crate::tray::install_tray(crate::tray::RealTray::new());
    application()
        .with_assets(PackagedAssets)
        .run(move |cx: &mut App| {
            cx.bind_keys([
                KeyBinding::new("left", ModePrev, None),
                KeyBinding::new("right", ModeNext, None),
                KeyBinding::new("up", MenuUp, None),
                KeyBinding::new("down", MenuDown, None),
                KeyBinding::new("alt-down", RowDown, None),
                KeyBinding::new("escape", Back, None),
                KeyBinding::new("f9", RequestProjectRows, None),
                KeyBinding::new("f10", MenuToggle, None),
                KeyBinding::new("enter", Activate, None),
                KeyBinding::new("backspace", input::Backspace, Some("SpikeInput")),
                KeyBinding::new("delete", input::Delete, Some("SpikeInput")),
                KeyBinding::new("left", input::Left, Some("SpikeInput")),
                KeyBinding::new("right", input::Right, Some("SpikeInput")),
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
                        let note = cx.new(|cx| TextInput::new(window, cx, placeholder));
                        cx.new(|cx| ShellView {
                            focus_handle: cx.focus_handle(),
                            session,
                            save_error: None,
                            note,
                        })
                    },
                )
                .expect("open native window");
            window
                .update(cx, |view, window, cx| {
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
            vec![SharedString::from(ICON_NAME), SharedString::from(COPY_NAME)]
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
    fn panel_lines_use_their_text_as_accessible_names() {
        let source = include_str!("view.rs");
        let marker = r#".id(gpui::ElementId::named_usize("panel-line", index))"#;
        let start = source.find(marker).expect("panel-line row div must exist");
        let snippet = &source[start..];
        let aria = snippet
            .find(".aria_label(SharedString::from(line.clone()))")
            .expect("panel-line div must set aria_label from line.clone()");
        let child = snippet
            .find(".child(SharedString::from(line))")
            .expect("panel-line div must still paint line as the child");
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
    fn f10_then_enter_opens_reduced_motion_settings() {
        let source = include_str!("view.rs");
        assert!(
            source.contains(r#"KeyBinding::new("f10", MenuToggle, None)"#),
            "f10 must bind MenuToggle with context None"
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
