# GPUI desktop runtime

Apply when changing the GPUI desktop app in `desktop-gpui/` (a standalone Cargo
workspace outside `just ci`; its gate is `just gpui`): native control focus, the
Status live lifecycle, or window material. Operation cancellation and cleanup
authority remain owned by `devsweep-core` and the GPUI `Session` job queue.

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

The window is opaque and requests no system backdrop. The application paints
every layer itself (`desktop-gpui/DESIGN.md`, "GPUI Native Adaptation"):
`ui::field` (opaque gradient plus three radial-gradient blobs), `ui::paper`
(translucent fill, 1px edge, top highlight, ground shadow), wells (flat fill,
no outline) and `ui::overlay` (the opaque paper colour, heavier shadow).

- `WindowBackgroundAppearance::Opaque` only. Do not call or reintroduce a
  window blur, acrylic or Mica request: devsweep#5 measured that the acrylic
  request does not give a reliable backdrop on Windows 10.0.26300, and
  `vendor/gpui_windows` no longer has `try_enable_window_blur`.
  `view::tests::the_window_requests_no_system_backdrop` guards this.
- `LIGHT` and `DARK` in `ui.rs` equal `palette style:airy-glass --palette mint
  --format json` of design-atlas. Change the colourway with `apply`, then copy
  the values; do not tune a token in place.
- Paper is not nested, and it has no `overflow_hidden`: the ground shadow is a
  child painted outside the paper's bounds.
- Wells and labelled buttons are bounded by fill. Only inputs and checkboxes
  draw the 1px `control_edge`.
- Secondary (`muted`) text sits on paper or a well, not on the field.

| System state | Tokens |
| --- | --- |
| Normal | The saved colour group, translucent paper. |
| Windows "Transparency effects" off (`UISettings::AdvancedEffectsEnabled` false) | The same group with `Theme::with_solid_paper()`. A failed query keeps the translucent paper. |
| Windows contrast theme | `ui::HIGH_CONTRAST`: opaque surfaces, 1px borders. It wins over the saved theme. |

Debug builds read `DEVSWEEP_PREVIEW_MATERIAL=solid|high-contrast` to show the
last two rows without changing a Windows setting. Release builds do not.

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
settings on every frame.

On transparent interactive surfaces draw keyboard focus with an inset ring.
A zero-blur outer shadow paints through an unfilled tab and can obscure its
icon. Verify with one tab selected and another focused in the native window.
