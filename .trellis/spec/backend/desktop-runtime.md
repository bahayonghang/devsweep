# GPUI desktop runtime

Apply when changing the GPUI desktop app in `desktop-gpui/` (a standalone Cargo
workspace outside `just ci`; its gate is `just gpui`): native control focus, the
Status live lifecycle, or window material. Operation cancellation and cleanup
authority remain owned by `devsweep-core` and the GPUI `Session` job queue.

For Software inventory icons, date/source presentation and the pre-uninstall
read-only directory preview, also read [Software Evidence](./software-evidence.md).
Those background reads share the Session job queue and never grant host-action
authority.

## GPUI native control focus

In `desktop-gpui`, mode tabs use real per-control keyboard focus through
`KeyboardFocus::keyboard_focus` (`tab_index(0)`). Keep `Role::Tab`, stable
accessibility IDs, labels, and `aria_selected(active)` on those controls.
Do not also apply `aria_active_descendant()` to a directly focusable tab:
the pinned GPUI prepaint registers real focus first, then rejects an active
descendant that is the focused node itself. Debug builds panic with
`set_active_descendant called on the focused node` once accessibility is active.

The active-descendant pattern is for a child of a focused container, not an
alias for selected state. Preserve real focus rather than disabling accessibility
or suppressing the assertion. Verify changes with accessibility enabled, mouse
clicks (including the already-selected tab), Tab/Shift+Tab, and keyboard
activation in the native debug build. Source guards and state tests do not
replace that runtime check.

## GPUI Status live lifecycle

Status samples while its page is on screen. Every `ShellView` render calls
`Session::status_set_main_visible(mode == Status && surface == Mode)`;
`StatusPanel::reconcile` starts the one live subscription when the main panel
is visible and not paused (or the tray is visible) and stops it otherwise. The
tray starts hidden (`main.rs` no longer marks it visible); the tray panel task
owns setting it. Pause stops and joins the sampler and freezes the 60-sample
trends; entering the page again clears the pause.

- Only a session with background jobs (the window) calls `spawn_live`; tests
  and `Session::unbacked` keep the subscription as bookkeeping and never take
  the process-wide live permit.
- Each open bumps `status_poll_generation` and leaves one poll request. The
  view takes it and runs one 250 ms poll loop that ends as soon as
  `status_poll_current(generation)` is false (closed, paused, terminal, or a
  newer subscription). Close joins the producer and drops the receiver, so no
  sample arrives after leaving.
- A redraw happens only when a sample or terminal was applied while the page
  is visible. Trends are painted with `canvas` + `PathBuilder` on that render;
  nothing requests an animation frame.
- A failed `spawn_live` or a producer terminal sets `StatusLiveFault`; render
  does not retry, and Resume closes the old control before starting again.
- Network totals and link type use the hardware Ethernet/Wi-Fi interfaces when
  any is listed; Windows repeats each adapter's bytes on its filter-driver
  layers, so summing every interface overcounts.

## GPUI window material

The app applies one translucent work surface over a window-level gradient. Data
rows and input wells use opaque fills. Keep the saved theme and text scale when
changing material. `WindowMaterial::wants_blur` uses logical viewport pixels.

| Window state | Renderer and surface |
| --- | --- |
| Width ≥ 960, Windows advanced effects enabled, normal contrast | Request blur on entry, and again with the new tint when the theme changes while blur is active. A failed request is not retried until the window leaves and re-enters this state. Paint translucent surfaces only after `gpui_windows::try_enable_window_blur(&Window, tint)` succeeds. |
| Width < 960, high contrast, advanced effects disabled or unavailable | Opaque window and paper. High contrast takes precedence over the saved palette. |
| Non-composited swap chain or native accent call fails | Opaque fallback even at full width. `GPUI_DISABLE_DIRECT_COMPOSITION=1` exercises the unsupported-renderer case on Windows. |

The backdrop is tinted, not clear. `gpui_windows::try_enable_window_blur(&Window, tint)`
takes the acrylic colour as (red, green, blue, alpha); the app passes
`Theme::backdrop_tint()`, the field start colour at `ui::BACKDROP_TINT_ALPHA`
(0x99), and paints the field gradient over it at `ui::FIELD_GLASS_ALPHA` (0.90)
through `Theme::field(glass)`. On fallback the field and the paper are opaque.
`WindowMaterial::update(window, tint)` requests the backdrop on entering the
blur condition and sends a new tint when the theme changes while blur is
active. A request that failed stays failed until the window leaves and
re-enters the condition; a new tint does not retry it.

Luminance rule: the light field must read at least 0.70 WCAG relative
luminance over a black window behind it; otherwise paint the field opaque and
keep only the paper translucent. With the constants above the light field
measured 0.7787 (mint end) and 0.7273 (lavender end) over black on Windows
10.0.26300 at 120 dpi, so the tinted backdrop is kept. The margin at the
lavender end is 0.027. Re-measure with a controlled black and white window
behind the app whenever either constant, the field colours or the vendored
blur call changes; a transparent-black tint measured 0.4739.

Light wells are a fill step below the paper (`#FFFFFF`, `#F4F8F6`, `#EAF2EE`,
`#DCEDE5`), because on the opaque fallback a white well on white paper has no
edge and wells carry no border in normal contrast. High contrast keeps 1px
borders.

`DesktopTheme::System` follows the Windows app mode. `ShellView` reads
`Window::appearance()` on render into `WindowMaterial.system_dark` and passes
it to `Theme::for_preferences(theme, system_dark, high_contrast)`;
`observe_window_appearance` re-renders on a change, so no restart is needed.
High contrast overrides every saved theme. The title-bar toggle resolves its
direction from the palette on screen, and writes an explicit light or dark
draft. First launch stays dark. Switching the Windows app mode with the app
open is a manual check: tests cover the resolution for both modes, not the
OS notification.

Refresh system preferences on appearance notifications, including
`WM_SETTINGCHANGE` with an unchanged light/dark palette. Do not poll Windows
settings on every frame. A native request returning success and a screenshot
of a pastel gradient are insufficient to establish visible transparency. On
Windows compare a pixel inside the owned window over two controlled backgrounds,
then repeat at narrow width or with direct composition disabled.

On transparent interactive surfaces draw keyboard focus with an inset ring.
A zero-blur outer shadow paints through an unfilled tab and can obscure its
icon. Verify with one tab selected and another focused in the native window.
