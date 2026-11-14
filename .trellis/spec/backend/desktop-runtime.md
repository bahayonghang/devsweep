# Desktop dispatch and native presentation lifecycle

## 1. Scope / Trigger

Apply when changing Tauri runtime creation, main-window minimize/restore,
decorative rendering or the window presentation bridge. Operation cancellation
and cleanup authority remain owned by their existing coordinators.

## 2. Signatures

- `async_runtime::build() -> io::Result<tokio::runtime::Runtime>` creates the
  desktop command runtime before the Tauri builder. `run` retains it until the
  native application exits and installs its handle with `tauri::async_runtime::set`.
- Main-only `main_presentation_visibility` returns a boolean. The
  `main-presentation-visibility` event carries the same closed boolean to main.
- `observeWindowPresentation(handler, bridge?) -> disconnect` owns the frontend
  initial read and native event subscription. Browser fixtures publish visible
  without native calls.

## 3. Contracts

- Use at most `min(available_parallelism, 4)` async dispatch workers, at least
  one when host parallelism is unavailable. Leave the separate blocking pool
  unchanged. Blocking domain jobs must not occupy an async dispatch worker.
- Tokio is an explicit dependency on the already locked runtime package; do
  not introduce a second runtime version or change process/global environment
  variables to control worker count. Keep the runtime alive through `app.run`.
- Native presentation is `!window.is_minimized() && window.is_visible()`.
  Resize/focus events trigger a fresh native query. Losing focus alone does not
  mean hidden. A query failure suspends decoration conservatively.
- WebView2 can remain `document.hidden == false` after actual OS minimization.
  Planet combines native presentation visibility with document visibility for
  every start/tick/draw/resize and late texture readiness path.
- Subscribe before the initial query. A newer event invalidates that query's
  result. A late subscription must immediately release on disposal; late events
  and read completions cannot restart an unmounted renderer.
- Visibility stops only decorative callbacks. Scans, Analyze, requested live
  Status and other domain operations retain their existing minimize policy.
  Do not freeze WebView execution or pause IPC to stop the planet.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Minimized HWND, DOM still visible | Zero recurring decorative RAF/draw work |
| Restore or resize after minimize | Resume at the existing FPS; no catch-up burst |
| Native-hidden first mount or pending decode | No Earth texture/GPU initialization or resumed frame until visible |
| Unknown/malformed native value, failed query/subscription | Decorative fallback/still state; domain work remains usable |
| New event while initial read is pending | New event wins; stale read is ignored |
| Renderer unmount before subscription/read completes | Release listener and ignore completion |
| Blocking domain job waits | Async acknowledgment/cancellation dispatch still progresses |

## 5. Good / Base / Bad Cases

- Good: native minimize cancels Planet's RAF while a requested scan keeps
  receiving and acknowledging progress.
- Base: ordinary browser visibility handling works without a native bridge.
- Bad: treating loss of focus as minimization, suspending all JavaScript, or
  changing cleanup/monitoring policy to satisfy a decorative CPU gate.

## 6. Tests Required

- Real runtime worker-count and blocking-isolation regression.
- Boolean visibility decoder, subscribe/read ordering, stale completion,
  subscription/read failure and asynchronous disposal tests.
- Planet native-hidden initial/late decode, minimize/restore with DOM still
  visible, resize and unmount; retained renderer rather than recreation per event.
- Packaged native IsIconic plus draw/RAF counters across twenty cycles, and
  requested scan/Status continuation while minimized.
- Preserve failed native prechecks. Re-run the inherited uninstrumented idle
  gate and operation latency gates after runtime configuration changes; a unit
  worker-count test alone is not a host-thread or performance pass.

## 7. Wrong vs Correct

Wrong: assume `document.visibilityState` proves the native window is visible,
or limit the blocking pool to reduce an async dispatch thread count.

Correct: measure the actual OS window, bridge presentation state narrowly, and
keep native operation acceptance separate from decoration and pool configuration.

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
