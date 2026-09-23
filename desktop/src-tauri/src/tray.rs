//! Tray icon, tray menu, and the HUD window lifecycle.
//!
//! The tray exists while the app runs. Left-click toggles the HUD window.
//! The HUD sampler runs only while the HUD is visible. Closing the main window
//! exits the app, which removes the tray icon. There is no autostart.

use std::collections::BTreeMap;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use devsweep_core::{
    presentation_settings::{PresentationLanguageTag, load_presentation_settings},
    status::{AvailabilityV1, StatusSnapshotV1, capture_snapshot},
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Rect, RunEvent, Runtime, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, Window, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::hud::{HUD_STATUS_EVENT, HUD_WINDOW_LABEL, HudSampler, HudStatusEvent};

/// Tray icon id.
pub(crate) const TRAY_ID: &str = "devsweep-tray";
/// Main window label from `tauri.conf.json`.
pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
const MENU_OPEN: &str = "tray-open";
const MENU_QUIT: &str = "tray-quit";
/// HUD logical size.
const HUD_WIDTH: f64 = 280.0;
const HUD_HEIGHT: f64 = 360.0;
/// Logical gap between the tray icon and the HUD.
const HUD_GAP: f64 = 8.0;
/// A tray click this soon after a blur-hide is the click that caused the blur.
const BLUR_CLICK_WINDOW: Duration = Duration::from_millis(300);
/// The HUD needs no process rows.
const HUD_PROCESS_LIMIT: u32 = 1;

/// App-managed HUD state.
#[derive(Default)]
pub(crate) struct HudState {
    sampler: HudSampler,
    hidden_at: Mutex<Option<Instant>>,
    exiting: AtomicBool,
}

impl HudState {
    /// Cancel and join the sampler. Used on hide and on app exit.
    pub(crate) fn shutdown(&self) {
        self.sampler.stop();
    }

    fn mark_hidden(&self) {
        if let Ok(mut hidden_at) = self.hidden_at.lock() {
            *hidden_at = Some(Instant::now());
        }
    }

    fn hidden_recently(&self) -> bool {
        self.hidden_at
            .lock()
            .ok()
            .and_then(|hidden_at| *hidden_at)
            .is_some_and(|at| at.elapsed() < BLUR_CLICK_WINDOW)
    }
}

/// Tray copy from the canonical catalogues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayCopy {
    open: String,
    quit: String,
    tooltip: String,
    tooltip_sample: String,
    unavailable: String,
}

impl TrayCopy {
    pub(crate) fn for_language(language: PresentationLanguageTag) -> &'static Self {
        static ENGLISH: OnceLock<TrayCopy> = OnceLock::new();
        static CHINESE: OnceLock<TrayCopy> = OnceLock::new();
        match language {
            PresentationLanguageTag::En => ENGLISH.get_or_init(|| {
                Self::from_catalogue(include_str!("../../../resources/i18n/en.json"))
            }),
            PresentationLanguageTag::ZhCn => CHINESE.get_or_init(|| {
                Self::from_catalogue(include_str!("../../../resources/i18n/zh-CN.json"))
            }),
        }
    }

    fn from_catalogue(source: &str) -> Self {
        let catalogue: serde_json::Value =
            serde_json::from_str(source).expect("embedded catalogue is valid JSON");
        let text = |key: &str| {
            catalogue["messages"][key]["forms"]["other"]
                .as_str()
                .unwrap_or_else(|| panic!("embedded catalogue lacks {key}"))
                .to_string()
        };
        Self {
            open: text("tray.v1.open"),
            quit: text("tray.v1.quit"),
            tooltip: text("tray.v1.tooltip"),
            tooltip_sample: text("tray.v1.tooltip.sample"),
            unavailable: text("hud.v1.unavailable"),
        }
    }

    /// Tooltip with CPU and memory percentages from one snapshot.
    pub(crate) fn sample_tooltip(&self, snapshot: &StatusSnapshotV1) -> String {
        let cpu = match &snapshot.cpu {
            AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => {
                Some(value.system_utilization_basis_points)
            }
            _ => None,
        };
        let memory = match &snapshot.memory {
            AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. }
                if value.total_bytes > 0 =>
            {
                let points = u128::from(value.used_bytes) * 10_000 / u128::from(value.total_bytes);
                Some(u32::try_from(points.min(10_000)).unwrap_or(10_000))
            }
            _ => None,
        };
        let percent = |points: Option<u32>| match points {
            Some(points) => format!("{}.{}%", points / 100, points % 100 / 10),
            None => self.unavailable.clone(),
        };
        let mut values = BTreeMap::new();
        values.insert("cpu", percent(cpu));
        values.insert("memory", percent(memory));
        interpolate(&self.tooltip_sample, &values)
    }
}

fn interpolate(template: &str, values: &BTreeMap<&str, String>) -> String {
    let mut text = template.to_string();
    for (name, value) in values {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// Persisted presentation language. Without a saved choice the tray uses
/// English.
fn tray_language() -> PresentationLanguageTag {
    load_presentation_settings()
        .ok()
        .and_then(|settings| settings.language)
        .unwrap_or(PresentationLanguageTag::En)
}

/// Build the tray icon and its menu.
pub(crate) fn setup<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    let copy = TrayCopy::for_language(tray_language());
    let open = MenuItem::with_id(app, MENU_OPEN, &copy.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, &copy.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(&copy.tooltip)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN => show_main_window(app),
            MENU_QUIT => exit_app(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                toggle_hud(tray.app_handle(), rect);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Main-window close exits the app. HUD blur hides the HUD; a destroyed HUD
/// (for example Alt+F4) also stops the sampler.
pub(crate) fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    match (window.label(), event) {
        (MAIN_WINDOW_LABEL, WindowEvent::Destroyed) => exit_app(window.app_handle()),
        (HUD_WINDOW_LABEL, WindowEvent::Focused(false) | WindowEvent::Destroyed) => {
            hide_hud(window.app_handle());
        }
        _ => {}
    }
}

/// App exit cancels and joins the HUD sampler before the process ends.
pub(crate) fn on_run_event<R: Runtime>(app: &AppHandle<R>, event: &RunEvent) {
    if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
        app.state::<HudState>().shutdown();
    }
}

fn exit_app<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<HudState>();
    if state.exiting.swap(true, Ordering::SeqCst) {
        return;
    }
    state.shutdown();
    app.exit(0);
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_hud<R: Runtime>(app: &AppHandle<R>, tray: Rect) {
    let existing = app.get_webview_window(HUD_WINDOW_LABEL);
    if existing
        .as_ref()
        .is_some_and(|window| window.is_visible().unwrap_or(false))
    {
        hide_hud(app);
        return;
    }
    if app.state::<HudState>().hidden_recently() {
        return;
    }
    match existing {
        Some(window) => show_hud(app, &window, tray),
        None => {
            // WebView2 creation blocks on the event loop, so it must not run
            // on the event-loop thread that delivers tray events.
            let app = app.clone();
            std::thread::spawn(move || {
                if let Ok(window) = create_hud(&app) {
                    show_hud(&app, &window, tray);
                }
            });
        }
    }
}

fn create_hud<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    WebviewWindowBuilder::new(app, HUD_WINDOW_LABEL, WebviewUrl::App("hud.html".into()))
        .title("DevSweep")
        .inner_size(HUD_WIDTH, HUD_HEIGHT)
        .resizable(false)
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .build()
}

fn show_hud<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>, tray: Rect) {
    place_hud(app, window, tray);
    let _ = window.show();
    let _ = window.set_focus();
    start_sampler(app);
}

pub(crate) fn hide_hud<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(HUD_WINDOW_LABEL) {
        let _ = window.hide();
    }
    let state = app.state::<HudState>();
    state.shutdown();
    state.mark_hidden();
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let copy = TrayCopy::for_language(tray_language());
        let _ = tray.set_tooltip(Some(&copy.tooltip));
    }
}

fn start_sampler<R: Runtime>(app: &AppHandle<R>) {
    let copy = TrayCopy::for_language(tray_language());
    let emitter = app.clone();
    app.state::<HudState>().sampler.start(
        |cancel| capture_snapshot(HUD_PROCESS_LIMIT, Some(cancel)).ok(),
        move |event| {
            if let HudStatusEvent::Snapshot { snapshot } = &event {
                let tooltip = copy.sample_tooltip(snapshot);
                let handle = emitter.clone();
                // Tray calls must run on the event loop. Posting keeps the
                // sampler thread free, so hide can always join it.
                let _ = emitter.run_on_main_thread(move || {
                    if handle.state::<HudState>().sampler.is_running()
                        && let Some(tray) = handle.tray_by_id(TRAY_ID)
                    {
                        let _ = tray.set_tooltip(Some(tooltip));
                    }
                });
            }
            let _ = emitter.emit_to(HUD_WINDOW_LABEL, HUD_STATUS_EVENT, event);
        },
    );
}

fn place_hud<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>, tray: Rect) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let position = tray.position.to_physical::<f64>(scale);
    let size = tray.size.to_physical::<f64>(scale);
    let tray = PixelRect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };
    let work = app
        .monitor_from_point(tray.x + tray.width / 2.0, tray.y + tray.height / 2.0)
        .ok()
        .flatten()
        .map(|monitor| {
            let area = monitor.work_area();
            PixelRect {
                x: f64::from(area.position.x),
                y: f64::from(area.position.y),
                width: f64::from(area.size.width),
                height: f64::from(area.size.height),
            }
        });
    let (x, y) = hud_position(
        tray,
        HUD_WIDTH * scale,
        HUD_HEIGHT * scale,
        HUD_GAP * scale,
        work,
    );
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PixelRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Center the HUD over the tray icon, above it when there is room, and keep it
/// inside the monitor work area.
pub(crate) fn hud_position(
    tray: PixelRect,
    width: f64,
    height: f64,
    gap: f64,
    work: Option<PixelRect>,
) -> (i32, i32) {
    let mut x = tray.x + tray.width / 2.0 - width / 2.0;
    let mut y = tray.y - height - gap;
    if let Some(work) = work {
        if y < work.y {
            y = tray.y + tray.height + gap;
        }
        x = x.clamp(work.x, (work.x + work.width - width).max(work.x));
        y = y.clamp(work.y, (work.y + work.height - height).max(work.y));
    }
    (x.round() as i32, y.round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_snapshot() -> StatusSnapshotV1 {
        serde_json::from_str(include_str!("../../src/api/fixtures/status/snapshot.json"))
            .expect("status fixture decodes")
    }

    #[test]
    fn tray_copy_comes_from_both_canonical_catalogues() {
        let english = TrayCopy::for_language(PresentationLanguageTag::En);
        let chinese = TrayCopy::for_language(PresentationLanguageTag::ZhCn);
        assert_eq!(english.open, "Open DevSweep");
        assert_eq!(english.quit, "Quit");
        assert_eq!(chinese.open, "打开 DevSweep");
        assert_eq!(chinese.quit, "退出");
        for copy in [english, chinese] {
            assert!(copy.tooltip_sample.contains("{cpu}"));
            assert!(copy.tooltip_sample.contains("{memory}"));
        }
    }

    #[test]
    fn sample_tooltip_shows_cpu_and_memory_percentages() {
        let snapshot = fixture_snapshot();
        assert_eq!(
            TrayCopy::for_language(PresentationLanguageTag::En).sample_tooltip(&snapshot),
            "DevSweep: CPU 43.2%, memory 50.0%"
        );
        assert_eq!(
            TrayCopy::for_language(PresentationLanguageTag::ZhCn).sample_tooltip(&snapshot),
            "DevSweep：CPU 43.2%，内存 50.0%"
        );
    }

    #[test]
    fn sample_tooltip_never_shows_zero_for_a_missing_metric() {
        let mut snapshot = fixture_snapshot();
        snapshot.cpu = AvailabilityV1::unavailable("counter_missing");
        snapshot.memory = AvailabilityV1::unsupported("platform_unsupported");
        assert_eq!(
            TrayCopy::for_language(PresentationLanguageTag::En).sample_tooltip(&snapshot),
            "DevSweep: CPU Unavailable, memory Unavailable"
        );
    }

    #[test]
    fn hud_opens_above_a_bottom_tray_and_below_a_top_tray() {
        let work = PixelRect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1040.0,
        };
        let bottom = PixelRect {
            x: 1800.0,
            y: 1050.0,
            width: 24.0,
            height: 24.0,
        };
        assert_eq!(
            hud_position(bottom, 280.0, 360.0, 8.0, Some(work)),
            (1640, 680)
        );
        let top_work = PixelRect {
            x: 0.0,
            y: 40.0,
            width: 1920.0,
            height: 1040.0,
        };
        let top = PixelRect {
            x: 100.0,
            y: 8.0,
            width: 24.0,
            height: 24.0,
        };
        assert_eq!(
            hud_position(top, 280.0, 360.0, 8.0, Some(top_work)),
            (0, 40)
        );
        assert_eq!(hud_position(bottom, 280.0, 360.0, 8.0, None), (1672, 682));
    }

    #[test]
    fn the_blur_click_window_ignores_only_an_immediate_click() {
        let state = HudState::default();
        assert!(!state.hidden_recently());
        state.mark_hidden();
        assert!(state.hidden_recently());
        *state.hidden_at.lock().unwrap() = Some(Instant::now() - BLUR_CLICK_WINDOW);
        assert!(!state.hidden_recently());
    }

    #[test]
    fn shutdown_joins_a_running_sampler() {
        let state = HudState::default();
        assert!(state.sampler.start(|_| Some(fixture_snapshot()), |_| {}));
        state.shutdown();
        assert!(!state.sampler.is_running());
    }
}
