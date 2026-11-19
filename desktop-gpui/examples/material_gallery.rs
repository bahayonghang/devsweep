//! Material gallery: the airy-glass recipes on the pinned GPUI, in one window.
//!
//! Spike for bahayonghang/devsweep#5 (parent spec
//! bahayonghang/my-ai-cli-toolkit#47). It depends on `gpui` only and touches
//! no product code. The top half shows each recipe alone; the bottom half is
//! one complete sample (field, papers, blocks, buttons, an input, an overlay).
//!
//! Run from `desktop-gpui/`: `cargo run --locked --example material_gallery`,
//! or `just gpui-gallery` from the repository root.
//!
//! Keys (the toolbar buttons do the same): `t` theme, `b` blob recipe of the
//! sample field, `m` window material, `a` field alpha, `p` overlay.
//!
//! Arguments set the first frame, for scripted captures:
//! `--dark`, `--blob off|shadow|svg|png|linear`,
//! `--material opaque|transparent|blurred|mica|mica-alt`,
//! `--field-alpha <0..1>`, `--no-overlay`, `--width <px>`, `--height <px>`.

use std::{borrow::Cow, sync::Arc};

use gpui::{
    AnyElement, App, Bounds, BoxShadow, Context, Div, FocusHandle, FontFeatures, FontWeight, Image,
    ImageFormat, KeyDownEvent, ObjectFit, Rgba, SharedString, Stateful, TextRun, TitlebarOptions,
    Window, WindowBackgroundAppearance, WindowBounds, WindowOptions, div, font, img,
    linear_color_stop, linear_gradient, point, prelude::*, px, relative, rgb, size,
};
use gpui_platform::application;

/// The variable font is embedded, so the gallery does not depend on an
/// installed Figtree. DirectWrite registers it under its typographic family.
const FIGTREE: &[u8] = include_bytes!("../assets/fonts/Figtree-wght.ttf");
const FONT_FAMILY: &str = "Figtree";

/// Candidate (3): pre-rendered blobs, one file per colour and theme.
const BLOB_PNG: [[&[u8]; 3]; 2] = [
    [
        include_bytes!("material_gallery/blob-light-1.png"),
        include_bytes!("material_gallery/blob-light-2.png"),
        include_bytes!("material_gallery/blob-light-3.png"),
    ],
    [
        include_bytes!("material_gallery/blob-dark-1.png"),
        include_bytes!("material_gallery/blob-dark-2.png"),
        include_bytes!("material_gallery/blob-dark-3.png"),
    ],
];

const PAPER_RADIUS: f32 = 20.0;
const WELL_RADIUS: f32 = 16.0;
const FIELD_ANGLE: f32 = 160.0;

/// Where the three blobs of the sample field sit: centre as a fraction of the
/// field, then the two radii in px. From SkillPort's
/// `radial-gradient(42rem 32rem at 12% 6%, ..)` and its two siblings.
const FIELD_BLOBS: [(f32, f32, f32, f32); 3] = [
    (0.12, 0.06, 672.0, 512.0),
    (0.92, 0.96, 640.0, 512.0),
    (0.82, 0.10, 480.0, 384.0),
];

/// A blob is transparent from this fraction of its radius outwards.
const BLOB_EDGE: f32 = 0.70;

fn main() {
    let launch = match Launch::parse(std::env::args().skip(1)) {
        Ok(launch) => launch,
        Err(message) => {
            eprintln!("material_gallery: {message}");
            std::process::exit(2);
        }
    };
    application().run(move |cx: &mut App| {
        if let Err(error) = cx.text_system().add_fonts(vec![Cow::Borrowed(FIGTREE)]) {
            eprintln!("material_gallery: add_fonts failed: {error}");
        }
        let families = cx.text_system().all_font_names();
        println!(
            "material_gallery: font family {FONT_FAMILY:?} registered={}",
            families.iter().any(|name| name == FONT_FAMILY)
        );
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(launch.width), px(launch.height)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Material gallery".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(960.0), px(600.0))),
                // Question E: only the public appearance API is used.
                window_background: launch.material.appearance(),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Gallery::new(launch, cx));
                let focus = view.read(cx).focus.clone();
                window.focus(&focus, cx);
                view
            },
        );
        if let Err(error) = opened {
            eprintln!("material_gallery: open_window failed: {error}");
            cx.quit();
        }
        cx.activate(true);
    });
}

/// How candidate A paints one blob.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlobRecipe {
    Off,
    /// (1) A large blurred `BoxShadow` on an unfilled element.
    Shadow,
    /// (2) `img()` of an SVG with a `radialGradient`, built from the token.
    Svg,
    /// (3) `img()` of a pre-rendered PNG.
    Png,
    /// (4) Four `linear_gradient` quadrants, colour to transparent.
    Linear,
}

impl BlobRecipe {
    const ALL: [Self; 5] = [Self::Off, Self::Shadow, Self::Svg, Self::Png, Self::Linear];

    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Shadow => "shadow",
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Linear => "linear",
        }
    }

    fn next(self) -> Self {
        let at = Self::ALL.iter().position(|recipe| *recipe == self);
        Self::ALL[(at.unwrap_or(0) + 1) % Self::ALL.len()]
    }
}

/// Question E: the public `WindowBackgroundAppearance` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Material {
    Opaque,
    Transparent,
    Blurred,
    Mica,
    MicaAlt,
}

impl Material {
    const ALL: [Self; 5] = [
        Self::Opaque,
        Self::Transparent,
        Self::Blurred,
        Self::Mica,
        Self::MicaAlt,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Opaque => "opaque",
            Self::Transparent => "transparent",
            Self::Blurred => "blurred",
            Self::Mica => "mica",
            Self::MicaAlt => "mica-alt",
        }
    }

    fn next(self) -> Self {
        let at = Self::ALL.iter().position(|material| *material == self);
        Self::ALL[(at.unwrap_or(0) + 1) % Self::ALL.len()]
    }

    fn appearance(self) -> WindowBackgroundAppearance {
        match self {
            Self::Opaque => WindowBackgroundAppearance::Opaque,
            Self::Transparent => WindowBackgroundAppearance::Transparent,
            Self::Blurred => WindowBackgroundAppearance::Blurred,
            Self::Mica => WindowBackgroundAppearance::MicaBackdrop,
            Self::MicaAlt => WindowBackgroundAppearance::MicaAltBackdrop,
        }
    }
}

/// The field alphas the `a` key steps through. 0 leaves the client area
/// unpainted outside the papers, which is the probe for question E.
const FIELD_ALPHAS: [f32; 4] = [1.0, 0.9, 0.5, 0.0];

/// State of the first frame, from the command line.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Launch {
    dark: bool,
    blob: BlobRecipe,
    material: Material,
    field_alpha: f32,
    overlay: bool,
    width: f32,
    height: f32,
}

impl Default for Launch {
    fn default() -> Self {
        Self {
            dark: false,
            blob: BlobRecipe::Svg,
            material: Material::Opaque,
            field_alpha: 1.0,
            overlay: true,
            width: 1280.0,
            height: 1040.0,
        }
    }
}

impl Launch {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut launch = Self::default();
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
            match flag.as_str() {
                "--dark" => launch.dark = true,
                "--no-overlay" => launch.overlay = false,
                "--blob" => {
                    let name = value("--blob")?;
                    launch.blob = BlobRecipe::ALL
                        .into_iter()
                        .find(|recipe| recipe.name() == name)
                        .ok_or(format!("unknown blob recipe {name:?}"))?;
                }
                "--material" => {
                    let name = value("--material")?;
                    launch.material = Material::ALL
                        .into_iter()
                        .find(|material| material.name() == name)
                        .ok_or(format!("unknown window material {name:?}"))?;
                }
                "--field-alpha" => {
                    launch.field_alpha = number(&value("--field-alpha")?, 0.0, 1.0)?;
                }
                "--width" => launch.width = number(&value("--width")?, 960.0, 4000.0)?,
                "--height" => launch.height = number(&value("--height")?, 600.0, 4000.0)?,
                other => return Err(format!("unknown argument {other:?}")),
            }
        }
        Ok(launch)
    }
}

fn number(text: &str, low: f32, high: f32) -> Result<f32, String> {
    match text.parse::<f32>() {
        Ok(value) if (low..=high).contains(&value) => Ok(value),
        _ => Err(format!("{text:?} is not a number from {low} to {high}")),
    }
}

/// The decoded blob images of one theme, kept for the life of the window so
/// the image cache is hit by the same `Arc<Image>` on every frame.
struct BlobArt {
    svg: [Arc<Image>; 3],
    png: [Arc<Image>; 3],
}

impl BlobArt {
    fn new(tokens: &Tokens, png: [&'static [u8]; 3]) -> Self {
        Self {
            svg: tokens
                .blobs
                .map(|blob| blob_svg(Tint::new(blob, tokens.blob_alpha))),
            png: png.map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec()))),
        }
    }
}

/// Candidate (2): the blob as an SVG string. No file: the colour and the alpha
/// come from the token, and `img()` rasterises it through resvg.
fn blob_svg(tint: Tint) -> Arc<Image> {
    let svg = format!(
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256">"##,
            r##"<defs><radialGradient id="g">"##,
            r##"<stop offset="0" stop-color="#{rgb:06X}" stop-opacity="{alpha}"/>"##,
            r##"<stop offset="{edge}" stop-color="#{rgb:06X}" stop-opacity="0"/>"##,
            r##"</radialGradient></defs>"##,
            r##"<rect width="256" height="256" fill="url(#g)"/></svg>"##,
        ),
        rgb = tint.rgb,
        alpha = tint.alpha,
        edge = BLOB_EDGE,
    );
    Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
}

/// Measured advance widths of the type specimens, in logical px.
struct TypeMetrics {
    /// (weight, width) of `WEIGHT_SAMPLE` at `SPECIMEN_SIZE`.
    weights: Vec<(f32, f32)>,
    /// Widths of the two digit strings, proportional then tabular.
    figures: [[f32; 2]; 2],
}

const WEIGHTS: [f32; 4] = [400.0, 600.0, 650.0, 700.0];
const WEIGHT_SAMPLE: &str = "Hamburgefonstiv 0123456789";
const FIGURE_SAMPLES: [&str; 2] = ["111,111.11", "000,000.00"];
const SPECIMEN_SIZE: f32 = 20.0;

fn tabular_figures() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

impl TypeMetrics {
    fn measure(window: &Window) -> Self {
        let width = |text: &'static str, weight: f32, features: FontFeatures| {
            let mut face = font(FONT_FAMILY);
            face.weight = FontWeight(weight);
            face.features = features;
            let run = TextRun {
                len: text.len(),
                font: face,
                color: gpui::black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let line =
                window
                    .text_system()
                    .shape_line(text.into(), px(SPECIMEN_SIZE), &[run], None);
            f32::from(line.width)
        };
        let metrics = Self {
            weights: WEIGHTS
                .iter()
                .map(|&weight| {
                    (
                        weight,
                        width(WEIGHT_SAMPLE, weight, FontFeatures::default()),
                    )
                })
                .collect(),
            figures: [FontFeatures::default(), tabular_figures()]
                .map(|features| FIGURE_SAMPLES.map(|text| width(text, 700.0, features.clone()))),
        };
        for (weight, width) in &metrics.weights {
            println!("material_gallery: weight={weight} width={width:.2}");
        }
        for (label, widths) in ["proportional", "tabular"].iter().zip(metrics.figures) {
            println!(
                "material_gallery: figures={label} widths={:.2},{:.2}",
                widths[0], widths[1]
            );
        }
        metrics
    }
}

struct Gallery {
    focus: FocusHandle,
    input_focus: FocusHandle,
    dark: bool,
    blob: BlobRecipe,
    material: Material,
    field_alpha: f32,
    overlay: bool,
    art: [BlobArt; 2],
    metrics: Option<TypeMetrics>,
}

impl Gallery {
    fn new(launch: Launch, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            input_focus: cx.focus_handle(),
            dark: launch.dark,
            blob: launch.blob,
            material: launch.material,
            field_alpha: launch.field_alpha,
            overlay: launch.overlay,
            art: [
                BlobArt::new(&Tokens::light(), BLOB_PNG[0]),
                BlobArt::new(&Tokens::dark(), BLOB_PNG[1]),
            ],
            metrics: None,
        }
    }

    fn tokens(&self) -> Tokens {
        if self.dark {
            Tokens::dark()
        } else {
            Tokens::light()
        }
    }

    fn art(&self) -> &BlobArt {
        &self.art[usize::from(self.dark)]
    }

    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.dark = !self.dark;
        cx.notify();
    }

    fn next_blob(&mut self, cx: &mut Context<Self>) {
        self.blob = self.blob.next();
        cx.notify();
    }

    fn next_material(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.material = self.material.next();
        window.set_background_appearance(self.material.appearance());
        cx.notify();
    }

    fn next_field_alpha(&mut self, cx: &mut Context<Self>) {
        let at = FIELD_ALPHAS
            .iter()
            .position(|alpha| *alpha == self.field_alpha);
        self.field_alpha = FIELD_ALPHAS[at.map_or(0, |at| (at + 1) % FIELD_ALPHAS.len())];
        cx.notify();
    }

    fn toggle_overlay(&mut self, cx: &mut Context<Self>) {
        self.overlay = !self.overlay;
        cx.notify();
    }

    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "t" => self.toggle_theme(cx),
            "b" => self.next_blob(cx),
            "m" => self.next_material(window, cx),
            "a" => self.next_field_alpha(cx),
            "p" => self.toggle_overlay(cx),
            _ => {}
        }
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.metrics.is_none() {
            self.metrics = Some(TypeMetrics::measure(window));
        }
        let t = self.tokens();
        div()
            .size_full()
            .relative()
            .font_family(FONT_FAMILY)
            .text_size(px(14.0))
            .text_color(rgb(t.ink))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key))
            .when(self.field_alpha > 0.0, |root| {
                root.child(field(&t, self.art(), self.blob, self.field_alpha))
            })
            .child(
                div()
                    .id("content")
                    .relative()
                    .size_full()
                    .overflow_y_scroll()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .child(self.toolbar(&t, cx))
                    .child(self.recipes(&t))
                    .child(self.sample(&t, window, cx)),
            )
    }
}

// ---------------------------------------------------------------------------
// L1 field
// ---------------------------------------------------------------------------

fn field_gradient(t: &Tokens) -> gpui::Background {
    linear_gradient(
        FIELD_ANGLE,
        linear_color_stop(rgb(t.field[0]), 0.0),
        linear_color_stop(rgb(t.field[1]), 1.0),
    )
}

/// The window field: the two-end gradient and up to three blobs. `alpha` below
/// 1 lets a window backdrop through (question E); the product default is 1.
fn field(t: &Tokens, art: &BlobArt, recipe: BlobRecipe, alpha: f32) -> Div {
    let mut field = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden()
        .opacity(alpha)
        .bg(field_gradient(t));
    for (index, (x, y, rx, ry)) in FIELD_BLOBS.into_iter().enumerate() {
        let tint = Tint::new(t.blobs[index], t.blob_alpha);
        field = field.child(
            blob(recipe, tint, art, index, rx, ry)
                .left(relative(x))
                .top(relative(y)),
        );
    }
    field
}

/// One blob with radii `rx` and `ry`, as a zero-size anchor at its centre.
/// The caller positions the anchor; the parent clips it.
fn blob(recipe: BlobRecipe, tint: Tint, art: &BlobArt, index: usize, rx: f32, ry: f32) -> Div {
    let anchor = div().absolute().size(px(0.0));
    let picture = |image: &Arc<Image>| {
        img(image.clone())
            .absolute()
            .left(px(-rx))
            .top(px(-ry))
            .w(px(2.0 * rx))
            .h(px(2.0 * ry))
            .object_fit(ObjectFit::Fill)
    };
    match recipe {
        BlobRecipe::Off => anchor,
        BlobRecipe::Shadow => anchor.child(shadow_blob(tint, rx, ry)),
        BlobRecipe::Svg => anchor.child(picture(&art.svg[index])),
        BlobRecipe::Png => anchor.child(picture(&art.png[index])),
        BlobRecipe::Linear => anchor.children(linear_blob(tint, rx, ry)),
    }
}

/// Candidate (1). The element has no fill; its shadow is the blob. The core is
/// a stadium of 0.3 of each radius and the gaussian carries the rest, so the
/// blob is clear of colour at about 0.7 of the radius. `blur_radius` is the
/// gaussian sigma in this GPUI.
fn shadow_blob(tint: Tint, rx: f32, ry: f32) -> Div {
    const CORE: f32 = 0.30;
    const SIGMA: f32 = 0.17;
    div()
        .absolute()
        .left(px(-CORE * rx))
        .top(px(-CORE * ry))
        .w(px(2.0 * CORE * rx))
        .h(px(2.0 * CORE * ry))
        .rounded_full()
        .shadow(vec![BoxShadow {
            color: tint.color().into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(SIGMA * (rx + ry) / 2.0),
            spread_radius: px(0.0),
            inset: false,
        }])
}

/// Candidate (4). A two-stop linear gradient cannot fall off radially. Four
/// quadrants, each fading away from the centre across its own diagonal, add up
/// to a diamond: alpha depends on |x|/rx + |y|/ry.
///
/// This GPUI does not place a slanted gradient the way CSS does: it squashes
/// the direction by the box's aspect and measures the line against one side,
/// so 0% and 100% are not the corners. The angle and the two stops below undo
/// that (see `gradient_color` in the platform shader).
fn linear_blob(tint: Tint, rx: f32, ry: f32) -> Vec<Div> {
    let clear = Tint::new(tint.rgb, 0.0);
    // Where the centre corner of a quadrant lands on the gradient line. The
    // two side corners land on 0.5.
    let centre = 0.5 - rx.max(ry) / rx.hypot(ry);
    let edge = centre + BLOB_EDGE * (0.5 - centre);
    // (left, top, x direction, y direction) of each quadrant, away from the centre.
    [
        (-rx, -ry, -1.0, -1.0),
        (0.0, -ry, 1.0, -1.0),
        (0.0, 0.0, 1.0, 1.0),
        (-rx, 0.0, -1.0, 1.0),
    ]
    .into_iter()
    .map(|(left, top, x, y): (f32, f32, f32, f32)| {
        let angle = (y * rx * rx).atan2(x * ry * ry).to_degrees() + 90.0;
        div()
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(rx))
            .h(px(ry))
            .bg(linear_gradient(
                angle.rem_euclid(360.0),
                linear_color_stop(tint.color(), centre),
                linear_color_stop(clear.color(), edge),
            ))
    })
    .collect()
}

// ---------------------------------------------------------------------------
// L2 paper, L3 blocks, L4 overlay
// ---------------------------------------------------------------------------

/// CSS `0 <y>px <blur>px <spread>px <colour>`. GPUI takes the gaussian sigma
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

/// The part of an outer shadow that falls below its element, and nothing else.
///
/// GPUI paints an outer shadow under the whole element, so a translucent fill
/// shows it through (CSS clips an outer shadow to the outside of the box).
/// This is a child of the paper: a clip box hanging below the bottom edge, and
/// inside it a caster that lines up with the paper's sides. Only the shadow
/// under the bottom edge survives the clip; with a negative spread that is
/// nearly all a CSS engine would show.
///
/// GPUI drops a shadow whose unblurred box misses the clip, however far its
/// blur reaches. So the caster sits low enough for that box to enter the clip
/// by 1px: 5px lower than CSS for `0 12px 40px -16px`, 1px for the overlay.
fn ground_shadow(shadow: BoxShadow, radius: f32) -> Div {
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
                .rounded(px(radius))
                .shadow(vec![shadow]),
        )
}

/// L2 paper: translucent fill, 1px translucent edge, 1px top highlight and
/// one soft shadow with a negative spread. Do not clip a paper
/// (`overflow_hidden`): its shadow is a child that hangs outside it.
fn paper(t: &Tokens) -> Div {
    div()
        .rounded(px(PAPER_RADIUS))
        .bg(t.paper.color())
        .border_1()
        .border_color(t.paper_edge.color())
        .shadow(vec![top_highlight(t.paper_highlight)])
        .child(ground_shadow(
            soft_shadow(t.paper_shadow, 12.0, 40.0, -16.0),
            PAPER_RADIUS,
        ))
}

/// L4 overlay: a more solid fill and one heavier shadow. No blur.
fn overlay(t: &Tokens) -> Div {
    div()
        .rounded(px(WELL_RADIUS))
        .bg(t.overlay.color())
        .border_1()
        .border_color(t.paper_edge.color())
        .shadow(vec![top_highlight(t.overlay_highlight)])
        .child(ground_shadow(
            soft_shadow(t.overlay_shadow, 24.0, 64.0, -24.0),
            WELL_RADIUS,
        ))
}

/// L3 block: a flat fill, no border. Semantic blocks take the semantic colour
/// at 12%; neutral blocks take `ink` at a low alpha.
fn block(fill: Tint) -> Div {
    div().rounded(px(WELL_RADIUS)).bg(fill.color())
}

fn semantic_fill(semantic: u32) -> Tint {
    Tint::new(semantic, SEMANTIC_FILL_ALPHA)
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

/// A labelled button is a filled pill with no border.
fn pill(id: &'static str, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(32.0))
        .px(px(14.0))
        .flex()
        .flex_none()
        .items_center()
        .rounded_full()
        .cursor_pointer()
        .whitespace_nowrap()
        .child(label.into())
}

fn primary_button(t: &Tokens, id: &'static str, label: &'static str) -> Stateful<Div> {
    pill(id, label)
        .bg(rgb(t.input))
        .text_color(rgb(t.on_input))
        .font_weight(FontWeight(650.0))
}

fn secondary_button(t: &Tokens, id: &'static str, label: impl Into<SharedString>) -> Stateful<Div> {
    let rest = Tint::new(t.ink, 0.08);
    let hover = Tint::new(t.ink, 0.14);
    pill(id, label)
        .bg(rest.color())
        .hover(move |style| style.bg(hover.color()))
}

fn caption(t: &Tokens, text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(12.0))
        .text_color(rgb(t.muted))
        .child(text.into())
}

fn title(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(16.0))
        .font_weight(FontWeight(650.0))
        .child(text.into())
}

impl Gallery {
    fn toolbar(&self, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let theme = if self.dark { "深色" } else { "浅色" };
        paper(t)
            .flex()
            .flex_none()
            .flex_wrap()
            .items_center()
            .gap(px(8.0))
            .px(px(20.0))
            .py(px(12.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(180.0))
                    .child(title("材质画廊"))
                    .child(caption(t, "GPUI 322cd509 · airy-glass 起点数值")),
            )
            .child(
                secondary_button(t, "theme", format!("T 主题：{theme}"))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_theme(cx))),
            )
            .child(
                secondary_button(t, "blob", format!("B 色斑：{}", self.blob.name()))
                    .on_click(cx.listener(|this, _, _, cx| this.next_blob(cx))),
            )
            .child(
                secondary_button(t, "material", format!("M 窗口：{}", self.material.name()))
                    .on_click(cx.listener(|this, _, window, cx| this.next_material(window, cx))),
            )
            .child(
                secondary_button(t, "alpha", format!("A 底场 α：{:.2}", self.field_alpha))
                    .on_click(cx.listener(|this, _, _, cx| this.next_field_alpha(cx))),
            )
            .child(
                secondary_button(t, "overlay", "P 浮层")
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_overlay(cx))),
            )
            .into_any_element()
    }

    // -----------------------------------------------------------------------
    // Part 1: each recipe alone
    // -----------------------------------------------------------------------

    fn recipes(&self, t: &Tokens) -> AnyElement {
        paper(t)
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(12.0))
            .p(px(20.0))
            .child(title("A · 色斑的四种做法"))
            .child(self.blob_row(t))
            .child(title("B · 纸的四个成分　C · 内容块的合成色"))
            .child(paper_row(t))
            .child(caption(
                t,
                format!(
                    "最差背景上的对比度：ink/纸 {:.2} · muted/纸 {:.2} · ink/底场 {:.2} · muted/底场 {:.2} · 控件边界/纸 {:.2}",
                    t.contrast_on_paper(t.ink),
                    t.contrast_on_paper(t.muted),
                    t.contrast_on_field(t.ink),
                    t.contrast_on_field(t.muted),
                    t.control_edge_contrast_on_paper(),
                ),
            ))
            .into_any_element()
    }

    /// Question A: the same blob, four ways, each on its own opaque field.
    fn blob_row(&self, t: &Tokens) -> Div {
        let tint = Tint::new(t.blobs[0], t.blob_alpha);
        let tile = |recipe: BlobRecipe, label: &'static str| {
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(180.0))
                .gap(px(6.0))
                .child(
                    div()
                        .relative()
                        .h(px(136.0))
                        .rounded(px(WELL_RADIUS))
                        .overflow_hidden()
                        .bg(rgb(t.field[1]))
                        .child(
                            blob(recipe, tint, self.art(), 0, 150.0, 95.0)
                                .left(relative(0.5))
                                .top(relative(0.5)),
                        ),
                )
                .child(caption(t, label))
        };
        div()
            .flex()
            .flex_wrap()
            .gap(px(12.0))
            .child(tile(BlobRecipe::Shadow, "(1) BoxShadow，大模糊"))
            .child(tile(BlobRecipe::Svg, "(2) img() + SVG radialGradient"))
            .child(tile(BlobRecipe::Png, "(3) img() + PNG"))
            .child(tile(BlobRecipe::Linear, "(4) linear_gradient ×4"))
    }

    // -----------------------------------------------------------------------
    // Part 2: the complete sample
    // -----------------------------------------------------------------------

    fn sample(&self, t: &Tokens, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        div()
            .relative()
            .flex()
            .flex_none()
            .items_start()
            .gap(px(16.0))
            .child(sidebar(t))
            .child(self.workbench(t, window, cx))
            .child(self.type_card(t))
            .into_any_element()
    }

    /// Status, then the number, then the action; blocks; an input.
    fn workbench(&self, t: &Tokens, window: &Window, cx: &mut Context<Self>) -> Div {
        let figure = |label: &'static str, value: &'static str, fill: Tint, ink: u32| {
            block(fill)
                .flex_1()
                .p(px(14.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(caption(t, label))
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight(700.0))
                        .font_features(tabular_figures())
                        .text_color(rgb(ink))
                        .child(value),
                )
        };
        let focused = self.input_focus.is_focused(window);
        let input_focus = self.input_focus.clone();
        // An input keeps a 3:1 boundary: `control_edge` at 1px. Focus draws an
        // inset ring, so nothing paints outside the control.
        let input = div()
            .id("filter")
            .track_focus(&self.input_focus)
            .h(px(36.0))
            .w(px(240.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .rounded(px(12.0))
            .border_1()
            .border_color(t.control_edge.color())
            .text_color(rgb(t.muted))
            .when(focused, |input| {
                input.border_color(rgb(t.input)).shadow(vec![BoxShadow {
                    color: rgb(t.input).into(),
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(0.0),
                    spread_radius: px(2.0),
                    inset: true,
                }])
            })
            .on_click(cx.listener(move |_, _, window, cx| {
                window.focus(&input_focus, cx);
                cx.notify();
            }))
            .child("筛选路径…");
        paper(t)
            .flex_1()
            .min_w(px(360.0))
            .p(px(20.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(caption(t, "已扫描 3 个位置 · 12 分钟前"))
                    .child(
                        div()
                            .text_size(px(32.0))
                            .font_weight(FontWeight(700.0))
                            .font_features(tabular_figures())
                            .child("12.48 GB"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(figure("可回收", "8.20 GB", semantic_fill(t.input), t.input))
                    .child(figure(
                        "已选择",
                        "4.28 GB",
                        semantic_fill(t.output),
                        t.output,
                    ))
                    .child(figure("跳过", "3 项", t.neutral_fill, t.ink)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.0))
                    .child(input)
                    .child(div().flex_1())
                    .child(
                        // The open menu hangs from its trigger: it covers the
                        // bottom edge of this paper and the field below it.
                        div()
                            .relative()
                            .child(
                                secondary_button(t, "sort", "排序 ▾").on_click(
                                    cx.listener(|this, _, _, cx| this.toggle_overlay(cx)),
                                ),
                            )
                            .when(self.overlay, |anchor| anchor.child(sort_menu(t))),
                    )
                    .child(primary_button(t, "clean", "移入回收站")),
            )
    }

    /// Question D: three weights of the embedded variable font, and
    /// tabular figures through `font_features`.
    fn type_card(&self, t: &Tokens) -> Div {
        let weights = self.metrics.iter().flat_map(|metrics| &metrics.weights);
        let figures = self.metrics.as_ref().map(|metrics| metrics.figures);
        let figure_column = |label: &'static str, features: FontFeatures, widths: [f32; 2]| {
            div()
                .flex()
                .flex_col()
                .items_end()
                .child(caption(
                    t,
                    format!("{label} · {:.1} / {:.1}px", widths[0], widths[1]),
                ))
                .children(FIGURE_SAMPLES.map(|text| {
                    div()
                        .text_size(px(SPECIMEN_SIZE))
                        .font_weight(FontWeight(700.0))
                        .font_features(features.clone())
                        .child(text)
                }))
        };
        paper(t)
            .w(px(400.0))
            .flex_none()
            .p(px(20.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(title("D · Figtree 可变字体"))
            .children(weights.map(|&(weight, width)| {
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(SPECIMEN_SIZE))
                            .font_weight(FontWeight(weight))
                            .whitespace_nowrap()
                            .child(WEIGHT_SAMPLE),
                    )
                    .child(caption(t, format!("{weight:.0} · {width:.1}px")))
            }))
            .when_some(figures, |card, figures| {
                card.child(
                    div()
                        .pt(px(8.0))
                        .flex()
                        .justify_between()
                        .child(figure_column(
                            "比例数字",
                            FontFeatures::default(),
                            figures[0],
                        ))
                        .child(figure_column("tnum", tabular_figures(), figures[1])),
                )
            })
    }
}

/// Question B: the four parts of a paper added one at a time, on an opaque
/// field colour so each pixel can be checked against a computed value. The
/// last tile is question C: blocks on a paper fill, with nothing else.
fn paper_row(t: &Tokens) -> Div {
    let backdrop = t.field[0];
    let tile = |label: SharedString, specimen: Div| {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(130.0))
            .gap(px(6.0))
            .child(
                div()
                    .h(px(132.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(WELL_RADIUS))
                    .overflow_hidden()
                    .bg(rgb(backdrop))
                    .child(specimen),
            )
            .child(caption(t, label))
    };
    let sheet = || div().w(px(108.0)).h(px(76.0)).rounded(px(PAPER_RADIUS));
    let shadow = soft_shadow(t.paper_shadow, 12.0, 40.0, -16.0);
    let highlight = top_highlight(t.paper_highlight);
    let edged = || {
        sheet()
            .bg(t.paper.color())
            .border_1()
            .border_color(t.paper_edge.color())
    };
    // A ruler for `blur_radius`: an unfilled box whose shadow is opaque, with
    // no offset and no spread. The 10% to 90% ramp across its edge is 2.56
    // sigma wide.
    let ruler = div().w(px(84.0)).h(px(76.0)).shadow(vec![BoxShadow {
        color: rgb(t.paper_shadow.rgb).into(),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(10.0),
        spread_radius: px(0.0),
        inset: false,
    }]);
    let swatch = |fill: Tint| block(fill).w(px(28.0)).h(px(44.0));
    let blocks = sheet()
        .w(px(128.0))
        .bg(t.paper.color())
        .flex()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .child(swatch(semantic_fill(t.input)))
        .child(swatch(semantic_fill(t.output)))
        .child(swatch(t.neutral_fill));
    div()
        .flex()
        .flex_wrap()
        .gap(px(12.0))
        .child(tile("填充".into(), sheet().bg(t.paper.color())))
        .child(tile("+ 1px 描边".into(), edged()))
        .child(tile(
            "+ 顶部内高光".into(),
            edged().shadow(vec![highlight.clone()]),
        ))
        .child(tile(
            "+ 负扩散柔影（直接画）".into(),
            edged().shadow(vec![shadow.clone(), highlight]),
        ))
        .child(tile(
            "只有柔影".into(),
            sheet().shadow(vec![shadow.clone()]),
        ))
        .child(tile(
            "柔影只留纸外".into(),
            edged()
                .shadow(vec![top_highlight(t.paper_highlight)])
                .child(ground_shadow(shadow, PAPER_RADIUS)),
        ))
        .child(tile("模糊标尺 blur_radius 10".into(), ruler))
        .child(tile(
            // The values a CSS engine would composite on this backdrop.
            format!(
                "C · 计算值 纸 #{:06X} 块 #{:06X} #{:06X} #{:06X}",
                t.paper_over(backdrop),
                t.block_over(t.input, backdrop),
                t.block_over(t.output, backdrop),
                over(t.neutral_fill, t.paper_over(backdrop)),
            )
            .into(),
            blocks,
        ))
}

fn sidebar(t: &Tokens) -> Div {
    let row = |label: &'static str, selected: bool| {
        div()
            .h(px(36.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .rounded(px(12.0))
            .when(selected, |row| {
                row.bg(semantic_fill(t.input).color())
                    .text_color(rgb(t.input))
                    .font_weight(FontWeight(650.0))
            })
            .child(label)
    };
    paper(t)
        .w(px(180.0))
        .flex_none()
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(row("清理", true))
        .child(row("软件", false))
        .child(row("优化", false))
        .child(row("分析", false))
        .child(row("状态", false))
}

/// L4: an open menu, positioned by its trigger.
fn sort_menu(t: &Tokens) -> Div {
    let row = |label: &'static str, selected: bool| {
        div()
            .h(px(34.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .rounded(px(10.0))
            .when(selected, |row| row.bg(t.neutral_fill.color()))
            .child(label)
    };
    overlay(t)
        .absolute()
        .right(px(0.0))
        .top(px(38.0))
        .w(px(220.0))
        .p(px(8.0))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(row("按大小排序", true))
        .child(row("按最近使用排序", false))
        .child(row("只看可回收", false))
        .child(
            div()
                .px(px(12.0))
                .py(px(6.0))
                .child(caption(t, "浮层 · 比纸更实，一条更重的阴影")),
        )
}

impl Tint {
    fn color(self) -> Rgba {
        Rgba {
            a: self.alpha as f32,
            ..rgb(self.rgb)
        }
    }
}

/// A colour with an alpha, before it meets a background.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Tint {
    rgb: u32,
    alpha: f64,
}

impl Tint {
    const fn new(rgb: u32, alpha: f64) -> Self {
        Self { rgb, alpha }
    }
}

/// Source-over of `top` on an opaque `under`, mixed per sRGB channel (the CSS
/// way) and rounded to 8 bits. Same arithmetic as `over` in design-atlas
/// `scripts/palette.py`, so the ratios below match its output.
fn over(top: Tint, under: u32) -> u32 {
    let mix = |shift: u32| {
        let above = f64::from((top.rgb >> shift) & 0xFF);
        let below = f64::from((under >> shift) & 0xFF);
        (top.alpha * above + (1.0 - top.alpha) * below + 0.5).floor() as u32
    };
    (mix(16) << 16) | (mix(8) << 8) | mix(0)
}

/// WCAG 2.x relative luminance of an opaque sRGB colour.
fn luminance(rgb: u32) -> f64 {
    let linear = |shift: u32| {
        let channel = f64::from((rgb >> shift) & 0xFF) / 255.0;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0)
}

/// WCAG 2.x contrast ratio of two opaque colours.
fn contrast(a: u32, b: u32) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Every colour of the gallery. The values are the "起点数值" of
/// bahayonghang/my-ai-cli-toolkit#47; the two semantic colours come from
/// `desktop-gpui/DESIGN.md` (the spec table lists none).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Tokens {
    /// L1: the two ends of the field gradient.
    field: [u32; 2],
    /// L1: up to three blobs, each painted at `blob_alpha`.
    blobs: [u32; 3],
    blob_alpha: f64,
    /// L2 paper.
    paper: Tint,
    paper_edge: Tint,
    paper_highlight: Tint,
    paper_shadow: Tint,
    /// L4 overlay.
    overlay: Tint,
    overlay_highlight: Tint,
    overlay_shadow: Tint,
    /// Boundary of an input, a checkbox, a radio and a switch.
    control_edge: Tint,
    ink: u32,
    muted: u32,
    /// Semantic colours: input or completion, and output.
    input: u32,
    output: u32,
    /// Text on a fill of `input` (the primary button).
    on_input: u32,
    /// L3 neutral block: `ink` at a low alpha.
    neutral_fill: Tint,
}

/// L3 semantic blocks are the semantic colour at this alpha over the paper.
const SEMANTIC_FILL_ALPHA: f64 = 0.12;

impl Tokens {
    const fn light() -> Self {
        Self {
            field: [0xE7F6F1, 0xEDE7F8],
            blobs: [0xCDEEE2, 0xDCD8F6, 0xD8EBF6],
            blob_alpha: 0.70,
            paper: Tint::new(0xFFFFFF, 0.62),
            paper_edge: Tint::new(0xFFFFFF, 0.70),
            paper_highlight: Tint::new(0xFFFFFF, 0.55),
            paper_shadow: Tint::new(0x1E3C5A, 0.14),
            overlay: Tint::new(0xFFFFFF, 0.82),
            overlay_highlight: Tint::new(0xFFFFFF, 0.65),
            overlay_shadow: Tint::new(0x1E3C5A, 0.22),
            control_edge: Tint::new(0x24303A, 0.54),
            ink: 0x24303A,
            muted: 0x5C6B76,
            input: 0x187A56,
            output: 0x6D4BD6,
            on_input: 0xFFFFFF,
            neutral_fill: Tint::new(0x24303A, 0.05),
        }
    }

    const fn dark() -> Self {
        Self {
            field: [0x0F1B1F, 0x18172B],
            blobs: [0x12302A, 0x2A2550, 0x142536],
            blob_alpha: 0.70,
            paper: Tint::new(0x1C2530, 0.60),
            paper_edge: Tint::new(0xFFFFFF, 0.08),
            paper_highlight: Tint::new(0xFFFFFF, 0.06),
            paper_shadow: Tint::new(0x000000, 0.45),
            overlay: Tint::new(0x1E2834, 0.84),
            overlay_highlight: Tint::new(0xFFFFFF, 0.08),
            overlay_shadow: Tint::new(0x000000, 0.60),
            control_edge: Tint::new(0xFFFFFF, 0.36),
            ink: 0xE7EDF3,
            muted: 0xA3AFBD,
            input: 0x87D9BA,
            output: 0xC6B5FF,
            on_input: 0x0F1B1F,
            neutral_fill: Tint::new(0xE7EDF3, 0.05),
        }
    }

    /// The paper over an opaque field colour.
    fn paper_over(&self, field: u32) -> u32 {
        over(self.paper, field)
    }

    /// An L3 semantic block: `semantic` at 12% over the paper over `field`.
    fn block_over(&self, semantic: u32, field: u32) -> u32 {
        over(
            Tint::new(semantic, SEMANTIC_FILL_ALPHA),
            self.paper_over(field),
        )
    }

    /// Every colour the field can show: each blob at its alpha over each end.
    fn field_backgrounds(&self) -> impl Iterator<Item = u32> + '_ {
        self.field.iter().flat_map(move |&end| {
            self.blobs
                .iter()
                .map(move |&blob| over(Tint::new(blob, self.blob_alpha), end))
        })
    }

    /// Every colour the paper can show: the paper over each field background.
    fn paper_backgrounds(&self) -> impl Iterator<Item = u32> + '_ {
        self.field_backgrounds()
            .map(move |field| self.paper_over(field))
    }

    /// Lowest contrast of `text` against the paper, over every background.
    fn contrast_on_paper(&self, text: u32) -> f64 {
        self.paper_backgrounds()
            .map(|paper| contrast(text, paper))
            .fold(f64::INFINITY, f64::min)
    }

    /// Lowest contrast of `text` placed directly on the field.
    fn contrast_on_field(&self, text: u32) -> f64 {
        self.field_backgrounds()
            .map(|field| contrast(text, field))
            .fold(f64::INFINITY, f64::min)
    }

    /// Lowest contrast of the 1px control boundary against the paper it sits on.
    fn control_edge_contrast_on_paper(&self) -> f64 {
        self.paper_backgrounds()
            .map(|paper| contrast(over(self.control_edge, paper), paper))
            .fold(f64::INFINITY, f64::min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Worked example from the parent spec: white 0.78 over #E7F6F1 is #FAFDFC.
    #[test]
    fn a_tint_over_an_opaque_colour_mixes_per_srgb_channel() {
        let white_78 = Tint::new(0xFFFFFF, 0.78);
        assert_eq!(over(white_78, 0xE7F6F1), 0xFAFDFC);
    }

    fn assert_ratio(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.01,
            "measured {actual:.3}, the spec table says {expected}"
        );
    }

    // The measured table in the parent spec ("这组数值的实测结果", worst
    // background: every blob at its alpha over every field end, then paper).
    #[test]
    fn starting_values_reproduce_the_measured_contrast_table() {
        let light = Tokens::light();
        assert_ratio(light.contrast_on_paper(light.ink), 12.16);
        assert_ratio(light.contrast_on_paper(light.muted), 4.96);
        assert_ratio(light.contrast_on_field(light.ink), 10.19);
        assert_ratio(light.contrast_on_field(light.muted), 4.16);
        // The table says 3.29 for the light control boundary. That figure
        // reproduces only with SkillPort's ink (#1F2A37 at 0.54). With the
        // `ink` token the table names (#24303A) the worst background gives
        // 3.17, cross-checked outside this file. Both clear the 3:1 gate.
        assert_ratio(light.control_edge_contrast_on_paper(), 3.17);
        let skillport_ink = Tokens {
            control_edge: Tint::new(0x1F2A37, 0.54),
            ..light
        };
        assert_ratio(skillport_ink.control_edge_contrast_on_paper(), 3.29);

        let dark = Tokens::dark();
        assert_ratio(dark.contrast_on_paper(dark.ink), 13.04);
        assert_ratio(dark.contrast_on_paper(dark.muted), 6.90);
        assert_ratio(dark.contrast_on_field(dark.ink), 12.86);
        assert_ratio(dark.contrast_on_field(dark.muted), 6.81);
        assert_ratio(dark.control_edge_contrast_on_paper(), 3.24);
    }

    // Question C, worked by hand. Light: white 0.62 over #E7F6F1 is #F6FCFA,
    // and #187A56 at 0.12 over that is #DBECE6. Dark: #1C2530 at 0.60 over
    // #0F1B1F is #172129, and #C6B5FF at 0.12 over that is #2C3343.
    #[test]
    fn a_semantic_block_on_paper_has_a_predictable_colour() {
        let light = Tokens::light();
        assert_eq!(light.paper_over(light.field[0]), 0xF6FCFA);
        assert_eq!(light.block_over(light.input, light.field[0]), 0xDBECE6);

        let dark = Tokens::dark();
        assert_eq!(dark.paper_over(dark.field[0]), 0x172129);
        assert_eq!(dark.block_over(dark.output, dark.field[0]), 0x2C3343);
    }

    #[test]
    fn arguments_set_the_first_frame() {
        let args = "--dark --blob png --material mica --field-alpha 0 --no-overlay --width 960";
        let launch = Launch::parse(args.split(' ').map(String::from));
        assert_eq!(
            launch,
            Ok(Launch {
                dark: true,
                blob: BlobRecipe::Png,
                material: Material::Mica,
                field_alpha: 0.0,
                overlay: false,
                width: 960.0,
                ..Launch::default()
            })
        );
        assert_eq!(Launch::parse([]), Ok(Launch::default()));
        assert!(Launch::parse(["--blob".to_string(), "blur".to_string()]).is_err());
        assert!(Launch::parse(["--width".to_string()]).is_err());
    }
}
