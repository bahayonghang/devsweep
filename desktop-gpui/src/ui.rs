//! DevSweep GPUI design system: the token layer of `desktop-gpui/DESIGN.md`.
//!
//! Four material layers. The field fills the window. Paper floats on the
//! field. Wells are flat fills inside paper. Overlays are popovers and bars.
//! Nothing blurs. Light, dark and Windows high contrast share the same roles.
//! Callers own state and attach click handlers.
//!
//! Every colour literal of the application lives in this file. Field names
//! follow the `DESIGN.md` role names: `colors.sheet-edge` is `sheet_edge`.
//!
//! Spacing and type use rems, and the shell sets the window rem size from the
//! stored text scale, so the whole layout scales together.

use std::sync::{Arc, LazyLock};

use gpui::{
    AnyElement, BoxShadow, Div, ElementId, FontWeight, Image, ImageFormat, ObjectFit, Pixels, Rems,
    Rgba, SharedString, Stateful, Svg, div, img, linear_color_stop, linear_gradient, point,
    prelude::*, px, relative, rems, rgb, rgba, svg,
};

use crate::preferences::DesktopTheme;

/// A colour and its alpha, kept together so a fill and its opacity cannot
/// drift apart. `rgb` is `0xRRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tint {
    pub rgb: u32,
    pub alpha: f32,
}

impl Tint {
    #[must_use]
    pub const fn new(rgb: u32, alpha: f32) -> Self {
        Self { rgb, alpha }
    }

    #[must_use]
    pub const fn solid(rgb: u32) -> Self {
        Self { rgb, alpha: 1.0 }
    }

    #[must_use]
    pub fn color(self) -> Rgba {
        rgb(self.rgb).alpha(self.alpha)
    }

    /// This tint with another tint of the same colour laid over it, as one
    /// fill. Hover and press steps stack `well` or `control_fill` this way.
    #[must_use]
    pub fn plus(self, over: Tint) -> Rgba {
        rgb(self.rgb).alpha(1.0 - (1.0 - self.alpha) * (1.0 - over.alpha))
    }
}

/// Design tokens. Plain colours are `0xRRGGBB`; translucent ones are [`Tint`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub high_contrast: bool,
    pub dark: bool,
    /// L1 field: the two ends of the window gradient and its three blobs.
    pub field_start: u32,
    pub field_end: u32,
    pub blob_start: u32,
    pub blob_end: u32,
    pub blob_mid: u32,
    /// L2 paper: fill, opaque fallback, 1px edge, top highlight, soft shadow.
    pub sheet: Tint,
    pub sheet_solid: Tint,
    pub sheet_edge: Tint,
    pub sheet_highlight: Tint,
    pub sheet_shadow: Tint,
    /// L4 overlay. GPUI has no backdrop blur, so overlays are filled with
    /// `sheet_solid`; `overlay` is the fill for a stack that can blur and is
    /// kept so the token set equals the palette.
    #[allow(dead_code)]
    pub overlay: Tint,
    pub overlay_highlight: Tint,
    pub overlay_shadow: Tint,
    /// L3 neutral well, and the hover step of a row or a ghost button.
    pub well: Tint,
    /// Secondary button, neutral tag, segmented track.
    pub control_fill: Tint,
    /// 1px boundary of inputs and unchecked boxes. Nothing else is outlined.
    pub control_edge: Tint,
    /// Label, check mark and icon on an `input` fill.
    pub on_input: u32,
    pub ink: u32,
    /// Secondary text. Sits on paper or a well, not on the field.
    pub muted: u32,
    /// Input, completion, selection, links, the focus ring, the primary fill.
    pub input: u32,
    pub input_wash: u32,
    /// Output and result figures.
    pub output: u32,
    pub output_wash: u32,
    /// Risk tones, a product extension: risk, failure, destructive confirm.
    pub warning: u32,
    pub danger: u32,
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

    /// The same theme with opaque paper, for reduced transparency.
    #[must_use]
    pub fn with_solid_paper(mut self) -> Self {
        self.sheet = self.sheet_solid;
        self
    }

    /// Translucent tint of a token for soft fills (treemap cells, stripes).
    #[must_use]
    pub fn tint(&self, color: u32, alpha: u8) -> Rgba {
        if self.high_contrast {
            return rgb(self.field_start);
        }
        rgba((color << 8) | u32::from(alpha))
    }

    /// The wash a tone sits on: the two semantic wells for input and output,
    /// the neutral control fill for secondary text, and the tone itself at
    /// [`TONE_WASH_ALPHA`] for the risk tones.
    fn tone_wash(&self, color: u32) -> Rgba {
        if self.high_contrast {
            rgb(self.field_start)
        } else if color == self.input {
            rgb(self.input_wash)
        } else if color == self.output {
            rgb(self.output_wash)
        } else if color == self.muted || color == self.ink {
            self.control_fill.color()
        } else {
            Tint::new(color, TONE_WASH_ALPHA).color()
        }
    }

    /// Text colour of a tone on its wash. A neutral tag is labelled in `ink`.
    fn tone_ink(&self, tone: Tone) -> u32 {
        match tone {
            Tone::Neutral => self.ink,
            _ => self.tone(tone),
        }
    }

    #[must_use]
    pub fn tone(&self, tone: Tone) -> u32 {
        match tone {
            Tone::Neutral => self.muted,
            Tone::Accent | Tone::Success => self.input,
            Tone::Warning => self.warning,
            Tone::Danger => self.danger,
            Tone::Info => self.output,
        }
    }

    /// One distinguishable hue per content class, a product extension for
    /// treemap cells and their legend.
    #[must_use]
    pub fn category(&self, index: usize) -> u32 {
        if self.high_contrast {
            return self.ink;
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

/// Light group of `style:airy-glass`, colourway `mint`. The values equal
/// `palette style:airy-glass --palette mint --format json` and the front
/// matter of `desktop-gpui/DESIGN.md`. Change the colourway with `apply`.
pub const LIGHT: Theme = Theme {
    high_contrast: false,
    dark: false,
    field_start: 0xE7F6F1,
    field_end: 0xEDE7F8,
    blob_start: 0xCDEEE2,
    blob_end: 0xDCD8F6,
    blob_mid: 0xD8EBF6,
    sheet: Tint::new(0xFFFFFF, 0.62),
    sheet_solid: Tint::new(0xFFFFFF, 0.96),
    sheet_edge: Tint::new(0xFFFFFF, 0.70),
    sheet_highlight: Tint::new(0xFFFFFF, 0.55),
    sheet_shadow: Tint::new(0x1E3C5A, 0.14),
    overlay: Tint::new(0xFFFFFF, 0.82),
    overlay_highlight: Tint::new(0xFFFFFF, 0.65),
    overlay_shadow: Tint::new(0x1E3C5A, 0.22),
    well: Tint::new(0x24303A, 0.04),
    control_fill: Tint::new(0x24303A, 0.08),
    control_edge: Tint::new(0x24303A, 0.54),
    on_input: 0xFFFFFF,
    ink: 0x24303A,
    muted: 0x5C6B76,
    input: 0x187A56,
    input_wash: 0xE7F7F0,
    output: 0x6D4BD6,
    output_wash: 0xF3EEFC,
    warning: 0x805413,
    danger: 0x9F3B45,
};

/// Dark group of the same colourway (`dark.colors` in the palette output).
pub const DARK: Theme = Theme {
    high_contrast: false,
    dark: true,
    field_start: 0x0F1B1F,
    field_end: 0x18172B,
    blob_start: 0x12302A,
    blob_end: 0x2A2550,
    blob_mid: 0x142536,
    sheet: Tint::new(0x1C2530, 0.60),
    sheet_solid: Tint::new(0x1C2530, 0.96),
    sheet_edge: Tint::new(0xFFFFFF, 0.08),
    sheet_highlight: Tint::new(0xFFFFFF, 0.06),
    sheet_shadow: Tint::new(0x000000, 0.45),
    overlay: Tint::new(0x1E2834, 0.84),
    overlay_highlight: Tint::new(0xFFFFFF, 0.08),
    overlay_shadow: Tint::new(0x000000, 0.60),
    well: Tint::new(0xE7EDF3, 0.04),
    control_fill: Tint::new(0xE7EDF3, 0.08),
    control_edge: Tint::new(0xFFFFFF, 0.36),
    on_input: 0x1C2530,
    ink: 0xE7EDF3,
    muted: 0xA3AFBD,
    input: 0x5FD6A8,
    input_wash: 0x243B31,
    output: 0xB3A8FF,
    output_wash: 0x353346,
    warning: 0xEDC37E,
    danger: 0xFFA6A9,
};

/// Windows contrast themes, a product extension: the catalogue entry has no
/// high-contrast palette. Every surface is opaque and components draw 1px
/// borders in `ink`.
pub const HIGH_CONTRAST: Theme = Theme {
    high_contrast: true,
    dark: true,
    field_start: 0x000000,
    field_end: 0x000000,
    blob_start: 0x000000,
    blob_end: 0x000000,
    blob_mid: 0x000000,
    sheet: Tint::solid(0x000000),
    sheet_solid: Tint::solid(0x000000),
    sheet_edge: Tint::solid(0xFFFFFF),
    sheet_highlight: Tint::new(0xFFFFFF, 0.0),
    sheet_shadow: Tint::new(0x000000, 0.0),
    overlay: Tint::solid(0x000000),
    overlay_highlight: Tint::new(0xFFFFFF, 0.0),
    overlay_shadow: Tint::new(0x000000, 0.0),
    well: Tint::solid(0x000000),
    control_fill: Tint::solid(0x000000),
    control_edge: Tint::solid(0xFFFFFF),
    on_input: 0x000000,
    ink: 0xFFFFFF,
    muted: 0xFFFFFF,
    input: 0xFFFF00,
    input_wash: 0x1A1A1A,
    output: 0x00FFFF,
    output_wash: 0x000000,
    warning: 0xFFFF00,
    danger: 0xFF6060,
};

/// `material` tokens of `desktop-gpui/DESIGN.md`.
pub const FIELD_ANGLE: f32 = 160.0;
pub const BLOB_OPACITY: f32 = 0.70;
/// A blob is transparent from this fraction of its radius outwards.
pub const BLOB_FADE: f32 = 0.70;

/// Alpha of a risk tone under its own text (a tag, a banner, an icon tile).
pub const TONE_WASH_ALPHA: f32 = 0.12;

/// Blob placement: centre as a fraction of the window, then the two radii in
/// logical pixels. A starting point for a desktop window, not a token
/// (`DESIGN.md`, Known Gaps): start corner, end corner, a third corner.
const FIELD_BLOBS: [(f32, f32, f32, f32); 3] = [
    (0.12, 0.06, 672.0, 512.0),
    (0.92, 0.96, 640.0, 512.0),
    (0.82, 0.10, 480.0, 384.0),
];

/// Theme-independent colours, a product extension. The caption close button
/// hovers in the Windows close red with white ink. The Clean screen cover is
/// black in every theme, with a dim hint line.
pub const CAPTION_CLOSE_HOVER: u32 = 0xC42B1C;
pub const CAPTION_CLOSE_INK: u32 = 0xFFFFFF;
pub const SCREEN_COVER: u32 = 0x000000;
pub const SCREEN_COVER_TEXT: u32 = 0x91999F;

/// The colour of a border that is reserved but not drawn. A focusable control
/// keeps a 1px border at this colour, so the focus ring does not move it.
pub const NO_EDGE: gpui::Hsla = gpui::Hsla {
    h: 0.0,
    s: 0.0,
    l: 0.0,
    a: 0.0,
};

/// The bundled default typeface (SIL OFL 1.1, `assets/fonts/`). A font the
/// user picks in Settings wins over it.
pub const FONT_FAMILY: &str = "Figtree";
pub const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/Figtree-wght.ttf");

/// Mode tabs keep their text labels from this window width, in logical
/// pixels. It does not grow with the text scale.
pub const TAB_LABEL_MIN_WIDTH: Pixels = px(720.0);

/// Smallest window the layout is checked at, in logical pixels.
pub const WINDOW_MIN_WIDTH: f32 = 720.0;
pub const WINDOW_MIN_HEIGHT: f32 = 560.0;

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

// ---------------------------------------------------------------------------
// Material: field, paper, overlay
// ---------------------------------------------------------------------------

/// One blob: an SVG radial falloff from the blob colour at [`BLOB_OPACITY`]
/// to transparent at [`BLOB_FADE`]. The pinned GPUI has no radial gradient,
/// so `img()` rasterises this string (devsweep#5, recipe A(2)).
fn blob_image(color: u32) -> Arc<Image> {
    let svg = format!(
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256">"##,
            r##"<defs><radialGradient id="g">"##,
            r##"<stop offset="0" stop-color="#{rgb:06X}" stop-opacity="{alpha}"/>"##,
            r##"<stop offset="{edge}" stop-color="#{rgb:06X}" stop-opacity="0"/>"##,
            r##"</radialGradient></defs>"##,
            r##"<rect width="256" height="256" fill="url(#g)"/></svg>"##,
        ),
        rgb = color,
        alpha = BLOB_OPACITY,
        edge = BLOB_FADE,
    );
    Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
}

/// The blob images of one colour group. Built once: the image cache key is a
/// content hash, so a new `Image` per frame would hash per frame.
fn blob_art(t: &Theme) -> &'static [Arc<Image>; 3] {
    static LIGHT_ART: LazyLock<[Arc<Image>; 3]> =
        LazyLock::new(|| [LIGHT.blob_start, LIGHT.blob_end, LIGHT.blob_mid].map(blob_image));
    static DARK_ART: LazyLock<[Arc<Image>; 3]> =
        LazyLock::new(|| [DARK.blob_start, DARK.blob_end, DARK.blob_mid].map(blob_image));
    if t.dark { &DARK_ART } else { &LIGHT_ART }
}

/// L1 field: the window's first child. An opaque two-end gradient with three
/// soft blobs. It fills its parent and clips the blobs. High contrast paints
/// a flat field with no blobs.
#[must_use]
pub fn field(t: &Theme) -> Div {
    let mut field = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden()
        .bg(linear_gradient(
            FIELD_ANGLE,
            linear_color_stop(rgb(t.field_start), 0.0),
            linear_color_stop(rgb(t.field_end), 1.0),
        ));
    if t.high_contrast {
        return field;
    }
    for ((x, y, rx, ry), image) in FIELD_BLOBS.into_iter().zip(blob_art(t)) {
        // A zero-size anchor at the blob centre; the field clips the picture.
        field = field.child(
            div()
                .absolute()
                .size(px(0.0))
                .left(relative(x))
                .top(relative(y))
                .child(
                    img(image.clone())
                        .absolute()
                        .left(px(-rx))
                        .top(px(-ry))
                        .w(px(2.0 * rx))
                        .h(px(2.0 * ry))
                        .object_fit(ObjectFit::Fill),
                ),
        );
    }
    field
}

/// CSS `0 <y>px <blur>px <spread>px <colour>`. GPUI takes the Gaussian sigma
/// as `blur_radius`, which is half of the CSS blur length.
fn soft_shadow(color: Tint, y: f32, css_blur: f32, spread: f32) -> BoxShadow {
    BoxShadow {
        color: color.color().into(),
        offset: point(px(0.0), px(y)),
        blur_radius: px(css_blur / 2.0),
        spread_radius: px(spread),
        inset: false,
    }
}

/// CSS `inset 0 1px 0 <colour>`: a 1px highlight along the top edge.
fn top_highlight(color: Tint) -> BoxShadow {
    BoxShadow {
        color: color.color().into(),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: true,
    }
}

/// The part of an outer shadow that falls below its element, and nothing
/// else. GPUI paints an outer shadow under the whole element, so a
/// translucent fill would show it through. This is a child of the paper: a
/// clip box hanging below the bottom edge, with a caster inside it that
/// lines up with the paper's sides (devsweep#5, B).
fn ground_shadow(shadow: BoxShadow, radius: Pixels) -> Div {
    let reach = f32::from(shadow.offset.y) + 3.0 * f32::from(shadow.blur_radius);
    let drop = (1.0 - f32::from(shadow.offset.y) - f32::from(shadow.spread_radius)).max(0.0);
    div()
        .absolute()
        // Insets are measured from inside the 1px edge.
        .left(px(-reach - 1.0))
        .right(px(-reach - 1.0))
        .top(relative(1.0))
        .mt(px(1.0))
        .h(px(reach))
        .overflow_hidden()
        .child(
            div()
                .absolute()
                .left(px(reach))
                .right(px(reach))
                .top(px(drop - 2.0 * reach))
                .h(px(2.0 * reach))
                .rounded(radius)
                .shadow(vec![shadow]),
        )
}

/// L2 paper: translucent fill, 1px translucent edge, 1px top highlight and
/// one soft shadow with a negative spread. Paper sits directly on the field
/// and does not nest; group its content with wells ([`card`]).
///
/// Do not clip a paper (`overflow_hidden`): its shadow is a child that hangs
/// below it. Clip an inner element when content needs it.
#[must_use]
pub fn paper(t: &Theme) -> Div {
    let paper = div()
        .relative()
        .flex()
        .flex_col()
        .rounded(RADIUS_SHEET)
        .bg(t.sheet.color())
        .border_1()
        .border_color(t.sheet_edge.color());
    if t.high_contrast {
        return paper;
    }
    paper
        .shadow(vec![top_highlight(t.sheet_highlight)])
        .child(ground_shadow(
            soft_shadow(t.sheet_shadow, 12.0, 40.0, -16.0),
            RADIUS_SHEET,
        ))
}

/// L4 overlay: popovers, menus and fixed bars. A more solid fill than paper
/// and one heavier shadow. The pinned GPUI has no backdrop blur, so the fill
/// is `sheet_solid`.
#[must_use]
pub fn overlay(t: &Theme) -> Div {
    let overlay = div()
        .relative()
        .flex()
        .flex_col()
        .rounded(RADIUS_WELL)
        .bg(t.sheet_solid.color())
        .border_1()
        .border_color(t.sheet_edge.color());
    if t.high_contrast {
        return overlay;
    }
    overlay
        .shadow(vec![top_highlight(t.overlay_highlight)])
        .child(ground_shadow(
            soft_shadow(t.overlay_shadow, 24.0, 64.0, -24.0),
            RADIUS_WELL,
        ))
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
    /// Filled with `input`. One per view.
    Primary,
    /// Filled with `control_fill`.
    Secondary,
    /// Label only until hovered.
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
/// focus draws an inset ring: a 1px `on_input` edge with a 2px `input` ring
/// inside it. On paper the ring shows; on an `input` fill the edge shows.
/// A drop shadow would fill transparent tabs behind their icon and make a
/// focused tab look selected.
/// The element must already paint a 1px border, transparent if needed.
pub trait KeyboardFocus: InteractiveElement + Sized {
    #[must_use]
    fn keyboard_focus(self, t: &Theme) -> Self {
        let ring = t.input;
        let edge = t.on_input;
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

/// Darkening step of a filled button on hover and on press.
const FILL_HOVER_STEP: f32 = 0.15;
const FILL_PRESS_STEP: f32 = 0.32;

/// Opacity of a disabled control.
const DISABLED_OPACITY: f32 = 0.5;

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

/// A labelled button is a filled pill. Fill and label identify it, so it has
/// no outline; the transparent 1px border is where the focus ring draws.
fn button_base(
    t: &Theme,
    id: impl Into<ElementId>,
    label: SharedString,
    kind: ButtonKind,
) -> Stateful<Div> {
    let filled = |fill: u32| {
        (
            Some(rgb(fill)),
            rgb(fill).blend(rgb(t.ink).alpha(FILL_HOVER_STEP)),
            rgb(fill).blend(rgb(t.ink).alpha(FILL_PRESS_STEP)),
        )
    };
    let (bg, hover, pressed) = match kind {
        ButtonKind::Primary => filled(t.input),
        ButtonKind::Danger => filled(t.danger),
        ButtonKind::Secondary | ButtonKind::Disabled => (
            Some(t.control_fill.color()),
            t.control_fill.plus(t.well),
            t.control_fill.plus(t.control_fill),
        ),
        ButtonKind::Ghost => (None, t.well.color(), t.control_fill.color()),
    };
    let fg = button_ink(t, kind);
    let outlined = t.high_contrast && !matches!(kind, ButtonKind::Primary | ButtonKind::Danger);
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
        .border_color(NO_EDGE)
        .when(outlined, |b| b.border_color(rgb(t.ink)))
        .when_some(bg, |b, bg| b.bg(bg))
        .text_color(rgb(fg))
        .text_size(TYPE_BODY)
        .font_weight(FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .when(kind == ButtonKind::Disabled, |b| {
            b.opacity(DISABLED_OPACITY)
                .cursor_not_allowed()
                .a11y_synthetic_children(|tree| tree.parent_node().set_disabled())
        })
        .when(kind != ButtonKind::Disabled, |b| {
            b.cursor_pointer()
                .hover(move |s| s.bg(hover))
                .active(move |s| s.bg(pressed))
                .keyboard_focus(t)
        })
}

fn button_ink(t: &Theme, kind: ButtonKind) -> u32 {
    match kind {
        ButtonKind::Primary | ButtonKind::Danger => t.on_input,
        ButtonKind::Secondary | ButtonKind::Ghost | ButtonKind::Disabled => t.ink,
    }
}

/// Button with a leading icon.
pub fn icon_button(
    t: &Theme,
    id: impl Into<ElementId>,
    icon_name: &'static str,
    label: impl Into<SharedString>,
    kind: ButtonKind,
) -> Stateful<Div> {
    let label = label.into();
    let base = button_base(t, id, label.clone(), kind);
    base.child(icon(icon_name, button_ink(t, kind)).size(rems(0.95)))
        .child(label)
}

/// L3 neutral well: a flat fill inside paper. Fill carries the boundary; only
/// high contrast draws an edge.
#[must_use]
pub fn card(t: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .bg(t.well.color())
        .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.ink)))
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
                .text_color(rgb(t.ink))
                .child(title.into()),
        )
}

/// Upper-case eyebrow label above a value.
#[must_use]
pub fn eyebrow(t: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .text_size(TYPE_META)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(t.muted))
        .child(text.into())
}

/// Muted supporting text. It sits on paper or a well, never on the field.
#[must_use]
pub fn muted(t: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .text_size(TYPE_BODY)
        .text_color(rgb(t.muted))
        .child(text.into())
}

/// Pill tag in a status tone: a wash and a label, no outline.
#[must_use]
pub fn badge(t: &Theme, text: impl Into<SharedString>, tone: Tone) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .h(rems(1.375))
        .px(SPACE_1)
        .rounded_full()
        .bg(t.tone_wash(t.tone(tone)))
        .when(t.high_contrast, |b| b.border_1().border_color(rgb(t.ink)))
        .text_size(TYPE_META)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(t.tone_ink(tone)))
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

/// Rounded square holding an icon, on the wash of its tone.
#[must_use]
pub fn icon_tile(t: &Theme, name: &'static str, color: u32) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(rems(2.25))
        .rounded(RADIUS_ITEM)
        .bg(t.tone_wash(color))
        .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.ink)))
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
        .bg(t.control_fill.color())
        .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.ink)))
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
        .bg(t.control_fill.color())
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
                    rgb(t.input)
                } else {
                    t.tint(t.input, if index % 2 == 0 { 0xC0 } else { 0x55 })
                })
        }))
}

/// Keep digits aligned while respecting the user's chosen typeface.
pub fn tabular_numerals() -> gpui::FontFeatures {
    static FEATURES: LazyLock<gpui::FontFeatures> =
        LazyLock::new(|| gpui::FontFeatures(vec![("tnum".into(), 1)].into()));
    FEATURES.clone()
}

/// Metric tile: label, large value, detail line, on a surface the caller
/// supplies. On the field that surface is a [`paper`].
#[must_use]
pub fn stat_on(
    surface: Div,
    t: &Theme,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    color: u32,
) -> Div {
    let label = label.into();
    let value = value.into();
    let detail = detail.into();
    surface
        .flex_1()
        .min_w(rems(9.0))
        .p(SPACE_3)
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
                .text_color(rgb(t.ink))
                .child(value),
        )
        .child(
            div()
                .text_size(TYPE_META)
                .text_color(rgb(t.muted))
                .child(detail),
        )
}

fn tone_glyph(tone: Tone) -> &'static str {
    match tone {
        Tone::Success => "check-circle",
        Tone::Warning | Tone::Danger => "alert",
        _ => "info",
    }
}

/// Inline banner for notices: a well in the wash of its tone, inside paper.
#[must_use]
pub fn banner(t: &Theme, tone: Tone, text: impl Into<SharedString>) -> Div {
    let color = t.tone(tone);
    let text = text.into();
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(SPACE_1)
        .px(SPACE_3)
        .py(SPACE_1)
        .rounded(RADIUS_ITEM)
        .bg(t.tone_wash(color))
        .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.ink)))
        .child(icon(tone_glyph(tone), color))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(TYPE_BODY)
                .text_color(rgb(t.ink))
                .child(text),
        )
}

/// A notice that sits directly on the field: a strip of paper with the tone
/// icon. Paper does not nest, so inside paper use [`banner`].
#[must_use]
pub fn notice(t: &Theme, tone: Tone, text: impl Into<SharedString>) -> Div {
    let text = text.into();
    paper(t)
        .flex_row()
        .items_start()
        .gap(SPACE_1)
        .px(SPACE_3)
        .py(SPACE_2)
        .child(icon(tone_glyph(tone), t.tone(tone)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(TYPE_BODY)
                .text_color(rgb(t.ink))
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
                .bg(t.control_fill.color())
                .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.ink)))
                .child(icon(icon_name, t.muted).size(rems(1.5))),
        )
        .child(
            div()
                .text_size(TYPE_TITLE)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(t.ink))
                .child(title.into()),
        )
        .child(
            div()
                .max_w(rems(30.0))
                .text_center()
                .text_size(TYPE_BODY)
                .text_color(rgb(t.muted))
                .child(body.into()),
        )
}

/// Segmented-control track: a `control_fill` pill with no outline.
#[must_use]
pub fn segmented(t: &Theme) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .p(px(3.0))
        .gap(px(2.0))
        .rounded_full()
        .bg(t.control_fill.color())
        .when(t.high_contrast, |d| d.border_1().border_color(rgb(t.ink)))
}

/// One segment. The selected one is the input wash with an `input` label;
/// the others are `ink` labels. Callers add `.on_click`.
pub fn segment(
    t: &Theme,
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
) -> Stateful<Div> {
    let label = label.into();
    let hover = t.well.color();
    let pressed = t.control_fill.color();
    let outlined = t.high_contrast && active;
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
        .border_color(NO_EDGE)
        .when(outlined, |s| s.border_color(rgb(t.ink)))
        .keyboard_focus(t)
        .when(active, |s| s.bg(rgb(t.input_wash)).text_color(rgb(t.input)))
        .when(!active, |s| {
            s.text_color(rgb(t.ink))
                .hover(move |h| h.bg(hover))
                .active(move |s| s.bg(pressed))
        })
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

/// Off: a 1px `control_edge` box with no fill, so the edge mixes over paper
/// as measured. On or mixed: an `input` fill with the mark in `on_input`.
pub fn checkbox_state(t: &Theme, state: gpui::Toggled, enabled: bool) -> Div {
    let checked = state != gpui::Toggled::False;
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(rems(1.1))
        .rounded(RADIUS_CHECK)
        .border_1()
        .border_color(if checked {
            rgb(t.input)
        } else {
            t.control_edge.color()
        })
        .when(checked, |c| c.bg(rgb(t.input)))
        .when(!enabled, |c| c.opacity(DISABLED_OPACITY))
        .when(state == gpui::Toggled::True, |c| {
            c.child(icon("check", t.on_input).size(rems(0.8)))
        })
        .when(state == gpui::Toggled::Mixed, |c| {
            c.child(div().w(rems(0.5)).h(px(2.0)).bg(rgb(t.on_input)))
        })
}

/// Base interactive list row. It has no fill of its own: hover is the
/// neutral well, selection the input wash, and the cursor an `input` edge.
pub fn list_row(
    t: &Theme,
    id: impl Into<ElementId>,
    selected: bool,
    focused: bool,
) -> Stateful<Div> {
    let hover = t.well.color();
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
            rgb(t.input)
        } else {
            NO_EDGE.into()
        })
        .when(selected, |r| r.bg(rgb(t.input_wash)))
        .when(!selected, |r| r.hover(move |h| h.bg(hover)))
        .cursor_pointer()
        .keyboard_focus(t)
}

/// Thin divider: `ink` at the control-fill alpha, not a gray fence.
#[must_use]
pub fn divider(t: &Theme) -> Div {
    div().w_full().h(px(1.0)).bg(if t.high_contrast {
        rgb(t.ink)
    } else {
        t.control_fill.color()
    })
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
                .text_color(rgb(t.ink))
                .child(title.into()),
        )
        .child(
            div()
                .truncate()
                .text_size(TYPE_META)
                .text_color(rgb(t.muted))
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
    fn theme_resolution_follows_preference_then_contrast() {
        assert_eq!(
            Theme::for_preferences(DesktopTheme::Light, false, true),
            HIGH_CONTRAST
        );
        assert_eq!(
            Theme::for_preferences(DesktopTheme::Light, true, false),
            LIGHT
        );
        assert_eq!(
            Theme::for_preferences(DesktopTheme::System, true, false),
            DARK
        );
    }

    /// Per-channel sRGB source-over, rounded to 8 bits at each layer. GPUI
    /// composites the same way to within 1/255 (devsweep#5, C).
    fn over(top: Tint, under: u32) -> u32 {
        let mix = |shift: u32| {
            let above = f64::from((top.rgb >> shift) & 0xFF);
            let below = f64::from((under >> shift) & 0xFF);
            let alpha = f64::from(top.alpha);
            (alpha * above + (1.0 - alpha) * below + 0.5).floor() as u32
        };
        (mix(16) << 16) | (mix(8) << 8) | mix(0)
    }

    fn luminance(color: u32) -> f64 {
        let channel = |shift: u32| {
            let c = f64::from((color >> shift) & 0xFF) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }

    fn contrast(a: u32, b: u32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// The eight field backgrounds of a group: its two ends, and each blob at
    /// the blob opacity over each end.
    fn fields(t: &Theme) -> Vec<u32> {
        let mut fields = vec![t.field_start, t.field_end];
        for blob in [t.blob_start, t.blob_end, t.blob_mid] {
            for end in [t.field_start, t.field_end] {
                fields.push(over(Tint::new(blob, BLOB_OPACITY), end));
            }
        }
        fields
    }

    fn papers(t: &Theme) -> Vec<u32> {
        fields(t).into_iter().map(|f| over(t.sheet, f)).collect()
    }

    /// Lowest contrast of a foreground over a set of backgrounds. `fg` gets
    /// the background and returns the composited foreground.
    fn worst(backgrounds: &[u32], fg: impl Fn(u32) -> u32) -> f64 {
        backgrounds
            .iter()
            .map(|&bg| contrast(fg(bg), bg))
            .fold(f64::INFINITY, f64::min)
    }

    fn assert_ratio(name: &str, measured: f64, published: f64) {
        assert!(
            (measured - published).abs() < 0.006,
            "{name}: token layer gives {measured:.2}, the palette table says {published:.2}"
        );
    }

    /// The token values reproduce the measured table that `palette
    /// style:airy-glass --palette mint --format json` prints (`contrast` and
    /// `dark.contrast`). A token that drifts from the palette fails here.
    #[test]
    fn tokens_reproduce_the_palette_contrast_table() {
        // (theme, ink-sheet, muted-sheet, muted-well, ink-field, muted-field,
        //  control-edge-sheet, input-sheet, output-sheet, input-wash, output-wash)
        for (t, published) in [
            (
                LIGHT,
                [12.16, 4.96, 4.62, 10.19, 4.16, 3.17, 4.79, 5.22, 4.79, 5.08],
            ),
            (
                DARK,
                [13.04, 6.90, 6.21, 12.86, 6.81, 3.24, 8.55, 7.30, 6.69, 5.81],
            ),
        ] {
            let fields = fields(&t);
            let papers = papers(&t);
            let wells: Vec<u32> = papers.iter().map(|&p| over(t.well, p)).collect();
            let measured = [
                worst(&papers, |_| t.ink),
                worst(&papers, |_| t.muted),
                worst(&wells, |_| t.muted),
                worst(&fields, |_| t.ink),
                worst(&fields, |_| t.muted),
                worst(&papers, |paper| over(t.control_edge, paper)),
                worst(&papers, |_| t.input),
                worst(&papers, |_| t.output),
                contrast(t.input, t.input_wash),
                contrast(t.output, t.output_wash),
            ];
            let names = [
                "ink-sheet-worst",
                "muted-sheet-worst",
                "muted-well-worst",
                "ink-field-worst",
                "muted-field-worst",
                "control-edge-sheet",
                "input-sheet-worst",
                "output-sheet-worst",
                "input-wash",
                "output-wash",
            ];
            for ((name, measured), published) in names.into_iter().zip(measured).zip(published) {
                assert_ratio(name, measured, published);
            }
        }
    }

    /// Pairs the product adds on top of the entry: the risk tones, labels on
    /// fills, text on the selection wash and on overlays, icons on the field.
    #[test]
    fn product_pairs_meet_contrast_on_the_worst_background() {
        for t in [LIGHT, DARK] {
            let fields = fields(&t);
            let papers = papers(&t);
            let overlays: Vec<u32> = fields.iter().map(|&f| over(t.sheet_solid, f)).collect();
            let solid = |fill: u32| move |_: u32| fill;
            let checks: [(&str, f64, f64); 15] = [
                ("on_input on input", contrast(t.on_input, t.input), 4.5),
                ("on_input on danger", contrast(t.on_input, t.danger), 4.5),
                ("ink on input wash", contrast(t.ink, t.input_wash), 4.5),
                ("muted on input wash", contrast(t.muted, t.input_wash), 4.5),
                ("ink on output wash", contrast(t.ink, t.output_wash), 4.5),
                ("ink on overlay", worst(&overlays, solid(t.ink)), 4.5),
                ("muted on overlay", worst(&overlays, solid(t.muted)), 4.5),
                ("input on overlay", worst(&overlays, solid(t.input)), 4.5),
                (
                    "control edge on overlay",
                    worst(&overlays, |bg| over(t.control_edge, bg)),
                    3.0,
                ),
                ("warning on paper", worst(&papers, solid(t.warning)), 4.5),
                ("danger on paper", worst(&papers, solid(t.danger)), 4.5),
                ("warning on its wash", tone_on_wash(&papers, t.warning), 4.5),
                ("danger on its wash", tone_on_wash(&papers, t.danger), 4.5),
                (
                    "ink on control fill",
                    papers
                        .iter()
                        .map(|&p| contrast(t.ink, over(t.control_fill, p)))
                        .fold(f64::INFINITY, f64::min),
                    4.5,
                ),
                // Icon-only controls on the field: 3:1 for the icon.
                ("muted icon on field", worst(&fields, solid(t.muted)), 3.0),
            ];
            for (name, ratio, minimum) in checks {
                assert!(
                    ratio >= minimum,
                    "{} {name}: {ratio:.2} < {minimum}",
                    if t.dark { "dark" } else { "light" }
                );
            }
        }
    }

    fn tone_on_wash(papers: &[u32], tone: u32) -> f64 {
        papers
            .iter()
            .map(|&p| contrast(tone, over(Tint::new(tone, TONE_WASH_ALPHA), p)))
            .fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn reduced_transparency_and_high_contrast_make_paper_opaque() {
        for t in [LIGHT, DARK] {
            assert!(t.sheet.alpha < t.sheet_solid.alpha);
            let solid = t.with_solid_paper();
            assert_eq!(solid.sheet, t.sheet_solid);
            assert_eq!(solid.sheet.rgb, t.sheet.rgb, "same hue, higher opacity");
        }
        for tint in [
            HIGH_CONTRAST.sheet,
            HIGH_CONTRAST.sheet_solid,
            HIGH_CONTRAST.well,
            HIGH_CONTRAST.control_fill,
            HIGH_CONTRAST.control_edge,
        ] {
            assert_eq!(tint.alpha, 1.0);
        }
        assert_eq!(HIGH_CONTRAST.with_solid_paper(), HIGH_CONTRAST);
    }

    #[test]
    fn stacked_tints_of_one_colour_are_one_fill() {
        let stacked = LIGHT.control_fill.plus(LIGHT.well);
        assert!((stacked.a - (1.0 - 0.92 * 0.96)).abs() < 1e-6);
        assert_eq!(stacked.alpha(1.0), rgb(LIGHT.ink));
    }

    #[test]
    fn the_bundled_font_is_a_truetype_file_with_its_licence() {
        assert_eq!(&FONT_BYTES[..4], &[0, 1, 0, 0]);
        let licence = include_str!("../assets/fonts/Figtree-OFL.txt");
        assert!(licence.contains("SIL OPEN FONT LICENSE Version 1.1"));
        let notices = include_str!("../THIRD_PARTY_NOTICES.txt");
        assert!(notices.contains("Figtree"));
        assert!(notices.contains("SIL OPEN FONT LICENSE Version 1.1"));
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
