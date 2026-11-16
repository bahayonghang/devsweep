//! DevSweep GPUI design system.
//!
//! A single paper surface sits on a cool two-tone field. Controls and data
//! wells use solid fills. Light, dark and Windows high contrast share the
//! same control roles. Callers own state and attach click handlers.
//!
//! Spacing and type use rems, and the shell sets the window rem size from the
//! stored text scale, so the whole layout scales together.

use gpui::{
    AnyElement, BoxShadow, Div, ElementId, FontWeight, Pixels, Rems, Rgba, SharedString, Stateful,
    Svg, div, point, prelude::*, px, rems, rgb, rgba, svg,
};

use crate::preferences::DesktopTheme;

/// Semantic color tokens. Values are `0xRRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub high_contrast: bool,
    pub dark: bool,
    /// Endpoints of the window field.
    pub canvas: u32,
    pub canvas_end: u32,
    /// Paper, top bar, and footer.
    pub chrome: u32,
    /// Cards and panels.
    pub surface: u32,
    /// Inset wells, hovered rows, inputs.
    pub surface_alt: u32,
    /// Pressed or selected rows.
    pub surface_active: u32,
    pub border: u32,
    pub border_strong: u32,
    pub text: u32,
    /// Secondary text. One value per colour group.
    pub text_muted: u32,
    /// The selected mode pill and primary action.
    pub pill: u32,
    pub pill_text: u32,
    /// Focus ring, links, progress.
    pub accent: u32,
    pub success: u32,
    pub warning: u32,
    pub danger: u32,
    pub info: u32,
}

impl Theme {
    #[must_use]
    pub fn for_preferences(theme: DesktopTheme, system_dark: bool, high_contrast: bool) -> Self {
        if high_contrast {
            return HIGH_CONTRAST;
        }
        match theme {
            // `system` follows the Windows app mode reported by the window.
            DesktopTheme::System if system_dark => DARK,
            DesktopTheme::System => LIGHT,
            DesktopTheme::Light | DesktopTheme::CatppuccinLatte => LIGHT,
            DesktopTheme::Dark
            | DesktopTheme::CatppuccinMocha
            | DesktopTheme::Codex
            | DesktopTheme::Claude => DARK,
        }
    }

    /// The two field stops. With the window backdrop on they are translucent
    /// so the tinted backdrop shows through; otherwise they are opaque.
    #[must_use]
    pub fn field(&self, glass: bool) -> (Rgba, Rgba) {
        let alpha = if glass && !self.high_contrast {
            FIELD_GLASS_ALPHA
        } else {
            1.0
        };
        (
            rgb(self.canvas).alpha(alpha),
            rgb(self.canvas_end).alpha(alpha),
        )
    }

    /// Tint for the window backdrop: the field start colour, as
    /// (red, green, blue, alpha) bytes.
    #[must_use]
    pub fn backdrop_tint(&self) -> (u8, u8, u8, u8) {
        let [_, red, green, blue] = self.canvas.to_be_bytes();
        (red, green, blue, BACKDROP_TINT_ALPHA)
    }

    /// Fill for the workbench paper. Its opaque equivalent is used on fallback.
    #[must_use]
    pub fn sheet(&self, glass: bool) -> Rgba {
        let paper = rgb(self.chrome);
        if glass && !self.high_contrast {
            paper.alpha(if self.dark { 0.90 } else { 0.78 })
        } else {
            paper
        }
    }

    #[must_use]
    pub fn sheet_edge(&self, glass: bool) -> Rgba {
        if glass && !self.high_contrast {
            rgb(0xFFFFFF).alpha(if self.dark { 0.16 } else { 0.80 })
        } else {
            rgb(self.border)
        }
    }

    /// Translucent tint of a token for soft fills (badges, banners, tiles).
    #[must_use]
    pub fn tint(&self, color: u32, alpha: u8) -> Rgba {
        if self.high_contrast {
            return rgb(self.canvas);
        }
        rgba((color << 8) | u32::from(alpha))
    }

    /// Opaque status wash, including when a badge is inside a selected row.
    fn tone_well(&self, color: u32) -> Rgba {
        rgb(self.surface).blend(self.tint(color, 0x12))
    }

    #[must_use]
    pub fn tone(&self, tone: Tone) -> u32 {
        match tone {
            Tone::Neutral => self.text_muted,
            Tone::Accent => self.accent,
            Tone::Success => self.success,
            Tone::Warning => self.warning,
            Tone::Danger => self.danger,
            Tone::Info => self.info,
        }
    }

    /// One distinguishable hue per content class, tuned for the field.
    #[must_use]
    pub fn category(&self, index: usize) -> u32 {
        if self.high_contrast {
            return self.text;
        }
        const DARK_HUES: [u32; 7] = [
            0x6E9BFF, 0x3DD68C, 0xB28CFF, 0xFF7A9C, 0x45D1E0, 0xA3ADC2, 0xF5B649,
        ];
        const LIGHT_HUES: [u32; 7] = [
            0x2F6FE4, 0x169C55, 0x7C4DDB, 0xD63B66, 0x0E9AAB, 0x6B7591, 0xC77D00,
        ];
        let hues = if self.dark { DARK_HUES } else { LIGHT_HUES };
        hues[index % hues.len()]
    }
}

pub const DARK: Theme = Theme {
    high_contrast: false,
    dark: true,
    canvas: 0x142322,
    canvas_end: 0x232032,
    chrome: 0x20292F,
    surface: 0x252F36,
    surface_alt: 0x29353A,
    surface_active: 0x30443E,
    border: 0x40504F,
    border_strong: 0x859E94,
    text: 0xEDF4F2,
    text_muted: 0xB3C2BF,
    pill: 0xA4DEC8,
    pill_text: 0x142823,
    accent: 0x87D9BA,
    success: 0x96DAAC,
    warning: 0xEDC37E,
    danger: 0xFFA6A9,
    info: 0xC6B5FF,
};

pub const LIGHT: Theme = Theme {
    high_contrast: false,
    dark: false,
    canvas: 0xE7F6F1,
    canvas_end: 0xEDE7F8,
    chrome: 0xFFFFFF,
    // One fill step below the paper, so a borderless well keeps an edge.
    surface: 0xF4F8F6,
    surface_alt: 0xEAF2EE,
    surface_active: 0xDCEDE5,
    border: 0xD9E5DF,
    border_strong: 0x78877F,
    text: 0x24303A,
    text_muted: 0x505F68,
    pill: 0x21624D,
    pill_text: 0xFFFFFF,
    accent: 0x187A56,
    success: 0x216D47,
    warning: 0x805413,
    danger: 0x9F3B45,
    info: 0x6D4BD6,
};

pub const HIGH_CONTRAST: Theme = Theme {
    high_contrast: true,
    dark: true,
    canvas: 0x000000,
    canvas_end: 0x000000,
    chrome: 0x000000,
    surface: 0x000000,
    surface_alt: 0x000000,
    surface_active: 0x1A1A1A,
    border: 0xFFFFFF,
    border_strong: 0xFFFFFF,
    text: 0xFFFFFF,
    text_muted: 0xFFFFFF,
    pill: 0xFFFFFF,
    pill_text: 0x000000,
    accent: 0xFFFF00,
    success: 0x00FF00,
    warning: 0xFFFF00,
    danger: 0xFF6060,
    info: 0x00FFFF,
};

/// Field alpha over the window backdrop, and the alpha of the backdrop tint.
/// Together they keep the field the dominant colour over any desktop
/// (`desktop-gpui/DESIGN.md`, "Material without backdrop-filter").
pub const FIELD_GLASS_ALPHA: f32 = 0.90;
pub const BACKDROP_TINT_ALPHA: u8 = 0x99;

/// Theme-independent colours (`desktop-gpui/DESIGN.md`, "Fixed colours").
/// The caption close button hovers in the Windows close red with white ink.
/// The Clean screen cover is black in every theme, with a dim hint line.
pub const CAPTION_CLOSE_HOVER: u32 = 0xC42B1C;
pub const CAPTION_CLOSE_INK: u32 = 0xFFFFFF;
pub const SCREEN_COVER: u32 = 0x000000;
pub const SCREEN_COVER_TEXT: u32 = 0x91999F;

/// Mode tabs keep their text labels from this window width, in logical
/// pixels. It does not grow with the text scale.
pub const TAB_LABEL_MIN_WIDTH: Pixels = px(720.0);

/// Type scale (`desktop-gpui/DESIGN.md`, "Type"). One rem is 16px times the
/// saved text scale. View code uses these four sizes and no others.
pub const TYPE_DISPLAY: Rems = rems(2.0);
pub const TYPE_TITLE: Rems = rems(1.0);
pub const TYPE_BODY: Rems = rems(0.875);
pub const TYPE_META: Rems = rems(0.75);

/// Spacing scale: 8, 12, 16, 24, 32 and 48px at 100% text scale.
pub const SPACE_1: Rems = rems(0.5);
pub const SPACE_2: Rems = rems(0.75);
pub const SPACE_3: Rems = rems(1.0);
pub const SPACE_4: Rems = rems(1.5);
pub const SPACE_5: Rems = rems(2.0);
pub const SPACE_6: Rems = rems(3.0);

/// Radii. Buttons, tags and segments are pills (`rounded_full`).
pub const RADIUS_SHEET: Pixels = px(20.0);
pub const RADIUS_WELL: Pixels = px(16.0);
pub const RADIUS_ITEM: Pixels = px(12.0);
pub const RADIUS_CHECK: Pixels = px(4.0);

/// The one shadow, under the paper: `0 20px 48px rgba(70, 90, 120, 0.08)`.
#[must_use]
pub fn sheet_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x465A_7814).into(),
        offset: point(px(0.0), px(20.0)),
        blur_radius: px(48.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// Solid light pill. One per view.
    Primary,
    /// Raised neutral.
    Secondary,
    /// Text only until hovered.
    Ghost,
    /// Destructive confirmation.
    Danger,
    /// Unavailable action. Callers omit activation handlers in this state.
    Disabled,
}

/// Packaged icon names. Every entry is an original stroke drawing under
/// `assets/icons/`.
pub const ICONS: &[&str] = &[
    "alert",
    "analyze",
    "battery",
    "check",
    "check-circle",
    "chevron-left",
    "chevron-right",
    "clean",
    "code",
    "cpu",
    "disk",
    "drive",
    "external",
    "eye",
    "folder",
    "globe",
    "gpu",
    "history",
    "info",
    "lock",
    "memory",
    "monitor",
    "moon",
    "network",
    "optimize",
    "pause",
    "play",
    "refresh",
    "rocket",
    "rules",
    "search",
    "settings",
    "shield",
    "software",
    "sort",
    "sparkle",
    "status",
    "stop",
    "sun",
    "thermal",
    "trash",
    "wrench",
    "window-max",
    "window-min",
    "window-restore",
    "x",
];

/// Embedded bytes for one packaged icon path (`icons/<name>.svg`).
#[must_use]
pub fn icon_bytes(path: &str) -> Option<&'static [u8]> {
    macro_rules! table {
        ($($name:literal),* $(,)?) => {
            match path {
                $(concat!("icons/", $name, ".svg") => Some(include_bytes!(concat!("../assets/icons/", $name, ".svg")) as &[u8]),)*
                _ => None,
            }
        };
    }
    table!(
        "alert",
        "analyze",
        "battery",
        "check",
        "check-circle",
        "chevron-left",
        "chevron-right",
        "clean",
        "code",
        "cpu",
        "disk",
        "drive",
        "external",
        "eye",
        "folder",
        "globe",
        "gpu",
        "history",
        "info",
        "lock",
        "memory",
        "monitor",
        "moon",
        "network",
        "optimize",
        "pause",
        "play",
        "refresh",
        "rocket",
        "rules",
        "search",
        "settings",
        "shield",
        "software",
        "sort",
        "sparkle",
        "status",
        "stop",
        "sun",
        "thermal",
        "trash",
        "wrench",
        "window-max",
        "window-min",
        "window-restore",
        "x",
    )
}

/// A monochrome icon painted in `color`.
#[must_use]
pub fn icon(name: &'static str, color: u32) -> Svg {
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .size(rems(1.0))
        .flex_shrink_0()
        .text_color(rgb(color))
}

/// Keyboard reachability for a clickable surface.
///
/// The element joins the window tab order in paint order, Enter and Space
/// fire its click listeners through GPUI's keyboard click, and a keyboard
/// focus draws an inset ring. A drop shadow would fill transparent tabs
/// behind their icon and make a focused tab look selected.
/// The element must already paint a 1px border, transparent if needed.
pub trait KeyboardFocus: InteractiveElement + Sized {
    #[must_use]
    fn keyboard_focus(self, t: &Theme) -> Self {
        let ring = t.accent;
        let edge = t.pill_text;
        self.tab_index(0).focus_visible(move |s| {
            s.border_color(rgb(edge)).shadow(vec![BoxShadow {
                color: rgb(ring).into(),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(0.0),
                spread_radius: px(3.0),
                inset: true,
            }])
        })
    }
}

impl KeyboardFocus for Stateful<Div> {}

/// Pressed fill step over the hover fill for neutral controls.
const PRESS_STEP: f32 = 0.12;

/// Base clickable button. Callers add `.on_click`.
pub fn button(
    t: &Theme,
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
) -> Stateful<Div> {
    let label = label.into();
    button_base(t, id, label.clone(), kind).child(label)
}

fn button_base(
    t: &Theme,
    id: impl Into<ElementId>,
    label: SharedString,
    kind: ButtonKind,
) -> Stateful<Div> {
    let (bg, fg, border) = match kind {
        ButtonKind::Primary => (t.pill, t.pill_text, t.pill),
        ButtonKind::Secondary => (t.surface_alt, t.text, t.border_strong),
        ButtonKind::Ghost => (t.surface, t.text_muted, t.surface),
        ButtonKind::Danger => (t.danger, t.pill_text, t.danger),
        ButtonKind::Disabled => (t.surface_alt, t.text_muted, t.border),
    };
    let (hover, pressed) = match kind {
        ButtonKind::Primary | ButtonKind::Danger => (
            rgb(bg).blend(rgb(t.text).alpha(0.15)),
            rgb(bg).blend(rgb(t.text).alpha(0.32)),
        ),
        _ => (
            rgb(t.surface_active),
            rgb(t.surface_active).blend(rgb(t.text).alpha(PRESS_STEP)),
        ),
    };
    let ghost = kind == ButtonKind::Ghost;
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label)
        .flex()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .gap(SPACE_1)
        .h(rems(2.25))
        .px(SPACE_3)
        .rounded_full()
        .border_1()
        .border_color(rgb(border))
        .when(!ghost, |b| b.bg(rgb(bg)))
        .when(ghost, |b| b.border_color(gpui::transparent_black()))
        .text_color(rgb(fg))
        .text_size(TYPE_BODY)
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .when(kind == ButtonKind::Disabled, |b| {
            b.a11y_synthetic_children(|tree| tree.parent_node().set_disabled())
        })
        .when(kind != ButtonKind::Disabled, |b| {
            b.cursor_pointer()
                .hover(move |s| s.bg(hover).border_color(rgb(fg)))
                .active(move |s| s.bg(pressed))
                .keyboard_focus(t)
        })
}

/// Button with a leading icon.
pub fn icon_button(
    t: &Theme,
    id: impl Into<ElementId>,
    icon_name: &'static str,
    label: impl Into<SharedString>,
    kind: ButtonKind,
) -> Stateful<Div> {
    let fg = match kind {
        ButtonKind::Primary | ButtonKind::Danger => t.pill_text,
        ButtonKind::Secondary => t.text,
        ButtonKind::Ghost => t.text_muted,
        ButtonKind::Disabled => t.text_muted,
    };
    let label = label.into();
    let base = button_base(t, id, label.clone(), kind);
    base.child(icon(icon_name, fg).size(rems(0.95)))
        .child(label)
}

/// Opaque content well nested within the workbench paper. Fill carries the
/// boundary; only high contrast draws an edge.
#[must_use]
pub fn card(t: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .bg(rgb(t.surface))
        .when(t.high_contrast, |d| {
            d.border_1().border_color(rgb(t.border))
        })
        .rounded(RADIUS_WELL)
        .p(SPACE_3)
}

/// Card title row with optional trailing content.
#[must_use]
pub fn card_header(t: &Theme, title: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(SPACE_2)
        .child(
            div()
                .text_size(TYPE_TITLE)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(t.text))
                .child(title.into()),
        )
}

/// Upper-case eyebrow label above a value.
#[must_use]
pub fn eyebrow(t: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .text_size(TYPE_META)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(t.text_muted))
        .child(text.into())
}

/// Muted supporting text.
#[must_use]
pub fn muted(t: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .text_size(TYPE_BODY)
        .text_color(rgb(t.text_muted))
        .child(text.into())
}

/// Pill badge in a status tone.
#[must_use]
pub fn badge(t: &Theme, text: impl Into<SharedString>, tone: Tone) -> Div {
    let color = t.tone(tone);
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .h(rems(1.375))
        .px(SPACE_1)
        .rounded_full()
        .bg(t.tone_well(color))
        .when(t.high_contrast, |b| b.border_1().border_color(rgb(t.text)))
        .text_size(TYPE_META)
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(color))
        .whitespace_nowrap()
        .child(text.into())
}

/// A filled status dot.
#[must_use]
pub fn dot(color: u32) -> Div {
    div()
        .size(rems(0.5))
        .flex_shrink_0()
        .rounded_full()
        .bg(rgb(color))
}

/// Rounded square holding an icon, tinted by tone.
#[must_use]
pub fn icon_tile(t: &Theme, name: &'static str, color: u32) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(rems(2.25))
        .rounded(RADIUS_ITEM)
        .bg(t.tint(color, 0x22))
        .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.text)))
        .child(icon(name, color).size(rems(1.15)))
}

/// Horizontal meter. `fraction` is clamped to 0..=1.
#[must_use]
pub fn meter(t: &Theme, fraction: f32, color: u32) -> Div {
    let fraction = if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.0
    };
    div()
        .w_full()
        .h(rems(0.4))
        .rounded_full()
        .bg(rgb(t.surface_active))
        .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.text)))
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .w(gpui::relative(fraction))
                .rounded_full()
                .bg(rgb(color)),
        )
}

/// Indeterminate progress: a static striped track. Reduced motion and the
/// no-decorative-frame rule mean it does not animate.
#[must_use]
pub fn progress_track(t: &Theme) -> Div {
    div()
        .w_full()
        .h(rems(0.4))
        .rounded_full()
        .bg(rgb(t.surface_active))
        .overflow_hidden()
        .flex()
        .flex_row()
        .gap(SPACE_1)
        .children((0..24).map(|index| {
            div()
                .h_full()
                .w(rems(0.9))
                .flex_shrink_0()
                .rounded_full()
                .bg(if t.high_contrast {
                    rgb(t.accent)
                } else {
                    t.tint(t.accent, if index % 2 == 0 { 0xC0 } else { 0x55 })
                })
        }))
}

/// Keep digits aligned while respecting the user's chosen typeface.
pub fn tabular_numerals() -> gpui::FontFeatures {
    static FEATURES: std::sync::LazyLock<gpui::FontFeatures> =
        std::sync::LazyLock::new(|| gpui::FontFeatures(vec![("tnum".into(), 1)].into()));
    FEATURES.clone()
}

/// Metric tile: label, large value, detail line.
#[must_use]
pub fn stat(
    t: &Theme,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    color: u32,
) -> Div {
    let label = label.into();
    let value = value.into();
    let detail = detail.into();
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(rems(9.0))
        .p(SPACE_3)
        .rounded(RADIUS_ITEM)
        .bg(rgb(t.surface_alt))
        .when(t.high_contrast, |d| {
            d.border_1().border_color(rgb(t.border))
        })
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(SPACE_1)
                .child(dot(color))
                .child(eyebrow(t, label)),
        )
        .child(
            div()
                .text_size(TYPE_DISPLAY)
                .font_features(tabular_numerals())
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(t.text))
                .child(value),
        )
        .child(
            div()
                .text_size(TYPE_META)
                .text_color(rgb(t.text_muted))
                .child(detail),
        )
}

/// Inline banner for notices.
#[must_use]
pub fn banner(t: &Theme, tone: Tone, text: impl Into<SharedString>) -> Div {
    let color = t.tone(tone);
    let glyph = match tone {
        Tone::Success => "check-circle",
        Tone::Warning | Tone::Danger => "alert",
        _ => "info",
    };
    let text = text.into();
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(SPACE_1)
        .px(SPACE_3)
        .py(SPACE_1)
        .rounded(RADIUS_ITEM)
        .bg(t.tone_well(color))
        .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.text)))
        .child(icon(glyph, color))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(TYPE_BODY)
                .text_color(rgb(t.text))
                .child(text),
        )
}

/// Centered empty or idle state.
#[must_use]
pub fn empty_state(
    t: &Theme,
    icon_name: &'static str,
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(SPACE_2)
        .py(SPACE_6)
        .px(SPACE_4)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(rems(3.25))
                .rounded_full()
                .bg(rgb(t.surface_alt))
                .when(t.high_contrast, |d| {
                    d.border_1().border_color(rgb(t.border))
                })
                .child(icon(icon_name, t.text_muted).size(rems(1.5))),
        )
        .child(
            div()
                .text_size(TYPE_TITLE)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(t.text))
                .child(title.into()),
        )
        .child(
            div()
                .max_w(rems(30.0))
                .text_center()
                .text_size(TYPE_BODY)
                .text_color(rgb(t.text_muted))
                .child(body.into()),
        )
}

/// Segmented-control track.
#[must_use]
pub fn segmented(t: &Theme) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .rounded_full()
        .bg(rgb(t.surface_alt))
        .when(t.high_contrast, |d| {
            d.border_1().border_color(rgb(t.border))
        })
}

/// One segment. Callers add `.on_click`.
pub fn segment(
    t: &Theme,
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
) -> Stateful<Div> {
    let label = label.into();
    let hover_bg = t.surface_active;
    let pressed = rgb(t.surface_active).blend(rgb(t.text).alpha(PRESS_STEP));
    div()
        .id(id)
        .role(gpui::Role::Tab)
        .aria_label(label.clone())
        .aria_selected(active)
        .flex()
        .items_center()
        .gap(SPACE_1)
        .h(rems(2.0))
        .px(SPACE_2)
        .rounded_full()
        .text_size(TYPE_BODY)
        .font_weight(if active {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::MEDIUM
        })
        .whitespace_nowrap()
        .cursor_pointer()
        .border_1()
        .border_color(gpui::transparent_black())
        .keyboard_focus(t)
        .when(active, |s| {
            s.bg(rgb(t.surface_active))
                .text_color(rgb(t.text))
                .border_color(rgb(t.border_strong))
        })
        .when(!active, |s| {
            s.text_color(rgb(t.text_muted))
                .hover(move |h| h.bg(rgb(hover_bg)).text_color(rgb(t.text)))
        })
        .active(move |s| s.bg(pressed))
        .child(label)
}

/// Checkbox glyph.
#[must_use]
pub fn checkbox(t: &Theme, checked: bool, enabled: bool) -> Div {
    checkbox_state(
        t,
        if checked {
            gpui::Toggled::True
        } else {
            gpui::Toggled::False
        },
        enabled,
    )
}

pub fn checkbox_state(t: &Theme, state: gpui::Toggled, enabled: bool) -> Div {
    let checked = state != gpui::Toggled::False;
    let border = if checked { t.accent } else { t.border_strong };
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(rems(1.1))
        .rounded(RADIUS_CHECK)
        .border_1()
        .border_color(rgb(border))
        .when(checked, |c| c.bg(rgb(t.accent)))
        .when(!enabled, |c| c.opacity(0.4))
        .when(state == gpui::Toggled::True, |c| {
            c.child(icon("check", t.pill_text).size(rems(0.8)))
        })
        .when(state == gpui::Toggled::Mixed, |c| {
            c.child(div().w(rems(0.5)).h(px(2.0)).bg(rgb(t.pill_text)))
        })
}

/// Base interactive list row. Focus draws an accent edge; selection a fill.
pub fn list_row(
    t: &Theme,
    id: impl Into<ElementId>,
    selected: bool,
    focused: bool,
) -> Stateful<Div> {
    let hover_bg = t.surface_alt;
    div()
        .id(id)
        .role(gpui::Role::ListItem)
        .aria_selected(selected)
        .flex()
        .flex_row()
        .items_center()
        .gap(SPACE_2)
        .px(SPACE_2)
        .py(SPACE_1)
        .rounded(RADIUS_ITEM)
        .border_1()
        .border_color(if focused {
            rgb(t.accent)
        } else {
            gpui::transparent_black().into()
        })
        .bg(rgb(if selected {
            t.surface_active
        } else {
            t.surface
        }))
        .when(!selected, |r| r.hover(move |h| h.bg(rgb(hover_bg))))
        .cursor_pointer()
        .keyboard_focus(t)
}

/// Thin divider.
#[must_use]
pub fn divider(t: &Theme) -> Div {
    div().w_full().h(px(1.0)).bg(rgb(t.border))
}

/// Primary + secondary text column that truncates.
#[must_use]
pub fn two_line(
    t: &Theme,
    title: impl Into<SharedString>,
    subtitle: impl Into<SharedString>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .child(
            div()
                .truncate()
                .text_size(TYPE_BODY)
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(t.text))
                .child(title.into()),
        )
        .child(
            div()
                .truncate()
                .text_size(TYPE_META)
                .text_color(rgb(t.text_muted))
                .child(subtitle.into()),
        )
}

/// Human byte size, binary units.
#[must_use]
pub fn bytes(value: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else if size >= 100.0 {
        format!("{size:.0} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

/// A path as shown to the user: the Windows verbatim prefix (`\\?\` and
/// `\\?\UNC\`, with either slash) is dropped and the drive letter is upper
/// case. Separators and everything after the drive stay as given. A verbatim
/// path that is neither a drive nor a UNC path, such as a volume-GUID path,
/// is shown unchanged. Accessible names and anything handed back to the
/// system keep the original path.
#[must_use]
pub fn display_path(path: &std::path::Path) -> String {
    display_path_text(&path.to_string_lossy())
}

/// [`display_path`] for a path already held as text, such as a saved root or
/// a breadcrumb that starts with one.
#[must_use]
pub fn display_path_text(path: &str) -> String {
    let is_sep = |c: char| c == '\\' || c == '/';
    let mut chars = path.chars();
    let verbatim = chars.next().is_some_and(is_sep)
        && chars.next().is_some_and(is_sep)
        && chars.next() == Some('?')
        && chars.next().is_some_and(is_sep);
    // The four prefix characters are ASCII, so byte offsets are safe.
    let rest = if verbatim { &path[4..] } else { path };
    let unc = verbatim
        && rest
            .get(..3)
            .is_some_and(|head| head.eq_ignore_ascii_case("UNC"))
        && rest[3..].chars().next().is_some_and(is_sep);
    if unc {
        // `\\?\UNC\server\share` is `\\server\share`.
        let sep = &path[..1];
        return format!("{sep}{sep}{}", &rest[4..]);
    }
    let mut head = rest.chars();
    match (head.next(), head.next()) {
        (Some(drive), Some(':')) if drive.is_ascii_alphabetic() => {
            format!("{}{}", drive.to_ascii_uppercase(), &rest[1..])
        }
        // Without a drive the verbatim prefix is part of the name: a
        // volume-GUID or device path has no shorter form.
        _ => path.to_string(),
    }
}

/// Erases a concrete element for heterogeneous children.
pub fn any(element: impl IntoElement) -> AnyElement {
    element.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_packaged_and_original_svg() {
        for name in ICONS {
            let bytes = icon_bytes(&format!("icons/{name}.svg")).expect(name);
            let text = std::str::from_utf8(bytes).unwrap();
            assert!(text.starts_with("<svg"), "{name}");
            assert!(!text.contains("http://") || text.contains("http://www.w3.org/2000/svg"));
            assert!(!text.to_ascii_lowercase().contains("mole"));
        }
        assert!(icon_bytes("icons/missing.svg").is_none());
    }

    #[test]
    fn paper_is_translucent_only_in_normal_glass_mode() {
        for theme in [DARK, LIGHT] {
            assert!(theme.sheet(true).a < 1.0);
            assert_eq!(theme.sheet(false), rgb(theme.chrome));
            assert_ne!(theme.canvas, theme.canvas_end);
        }
        assert_eq!(HIGH_CONTRAST.sheet(true), HIGH_CONTRAST.sheet(false));
        assert_eq!(HIGH_CONTRAST.sheet_edge(true), rgb(HIGH_CONTRAST.border));
        assert_eq!(
            Theme::for_preferences(DesktopTheme::Light, false, true),
            HIGH_CONTRAST
        );
    }

    fn assert_contrast(foreground: u32, background: Rgba, minimum: f32) {
        fn luminance(color: Rgba) -> f32 {
            let [r, g, b] = [color.r, color.g, color.b].map(|c| {
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            });
            0.2126 * r + 0.7152 * g + 0.0722 * b
        }
        let fg = luminance(rgb(foreground));
        let bg = luminance(background);
        let ratio = (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05);
        assert!(
            ratio >= minimum,
            "#{foreground:06x} on {background:?}: {ratio:.2} < {minimum}"
        );
    }

    #[test]
    fn text_and_control_edges_meet_contrast_on_paper_and_wells() {
        for t in [DARK, LIGHT, HIGH_CONTRAST] {
            for background in [
                rgb(t.canvas).blend(t.sheet(true)),
                rgb(t.canvas_end).blend(t.sheet(true)),
                t.sheet(false),
                rgb(t.surface),
                rgb(t.surface_alt),
                rgb(t.surface_active),
            ] {
                for foreground in [t.text, t.text_muted] {
                    assert_contrast(foreground, background, 4.5);
                }
                assert_contrast(t.accent, background, 3.0);
                assert_contrast(t.border_strong, background, 3.0);
            }
            for tone in [
                Tone::Neutral,
                Tone::Accent,
                Tone::Success,
                Tone::Warning,
                Tone::Danger,
                Tone::Info,
            ] {
                let color = t.tone(tone);
                assert_contrast(color, t.tone_well(color), 4.5);
            }
            for fill in [t.pill, t.danger, t.accent] {
                assert_contrast(t.pill_text, rgb(fill), 4.5);
            }
        }
    }

    /// Wells have no border in normal contrast, so each fill step must differ
    /// from the one it sits on: paper, well, hovered or inset, selected.
    #[test]
    fn wells_are_a_fill_step_away_from_the_paper() {
        for t in [DARK, LIGHT] {
            let steps = [t.chrome, t.surface, t.surface_alt, t.surface_active];
            for pair in steps.windows(2) {
                assert_ne!(pair[0], pair[1], "{steps:06x?}");
            }
        }
    }

    #[test]
    fn the_field_is_translucent_only_over_the_window_backdrop() {
        for t in [DARK, LIGHT] {
            let (start, end) = t.field(true);
            assert_eq!(start.a, FIELD_GLASS_ALPHA);
            assert_eq!(end.a, FIELD_GLASS_ALPHA);
            assert_eq!(t.field(false), (rgb(t.canvas), rgb(t.canvas_end)));
            let (red, green, blue, alpha) = t.backdrop_tint();
            assert_eq!(
                u32::from_be_bytes([0, red, green, blue]),
                t.canvas,
                "the backdrop is tinted with the field colour"
            );
            assert_eq!(alpha, BACKDROP_TINT_ALPHA);
        }
        assert_eq!(
            HIGH_CONTRAST.field(true),
            (rgb(HIGH_CONTRAST.canvas), rgb(HIGH_CONTRAST.canvas_end))
        );
    }

    #[test]
    fn display_paths_drop_the_verbatim_prefix_and_capitalise_the_drive() {
        use std::path::Path;
        for (given, shown) in [
            (r"\\?\c:\work\app\target", r"C:\work\app\target"),
            (r"\\?\C:\", r"C:\"),
            (r"c:\users\me", r"C:\users\me"),
            ("//?/d:/code/app", "D:/code/app"),
            ("d:/code/app", "D:/code/app"),
            (r"\\?\UNC\server\share\dir", r"\\server\share\dir"),
            (r"\\?\unc\server\share", r"\\server\share"),
            ("//?/UNC/server/share", "//server/share"),
            (r"\\server\share\dir", r"\\server\share\dir"),
            (r"\\?\Volume{0a1b}\dir", r"\\?\Volume{0a1b}\dir"),
            ("//?/Volume{0a1b}/dir", "//?/Volume{0a1b}/dir"),
            (r"\\?\GLOBALROOT\Device\x", r"\\?\GLOBALROOT\Device\x"),
            ("/home/me/app", "/home/me/app"),
            ("relative/dir", "relative/dir"),
            ("c: / users / me", "C: / users / me"),
            ("c", "c"),
            ("", ""),
            (r"\\?\", r"\\?\"),
            ("é:x", "é:x"),
        ] {
            assert_eq!(display_path_text(given), shown, "{given}");
            assert_eq!(display_path(Path::new(given)), shown, "{given}");
        }
    }

    #[test]
    fn byte_labels_use_binary_units() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1536), "1.5 KB");
        assert_eq!(bytes(13_800_000_000), "12.9 GB");
    }
}
