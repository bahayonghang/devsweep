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

## GPUI window material

The app applies one translucent work surface over a window-level gradient. Data
rows and input wells use opaque fills. Keep the saved theme and text scale when
changing material. `WindowMaterial::wants_blur` uses logical viewport pixels.

| Window state | Renderer and surface |
| --- | --- |
| Width ≥ 960, Windows advanced effects enabled, normal contrast | Request blur once on entry. Paint translucent surfaces only after `gpui_windows::try_enable_window_blur(&Window)` succeeds. |
| Width < 960, high contrast, advanced effects disabled or unavailable | Opaque window and paper. High contrast takes precedence over the saved palette. |
| Non-composited swap chain or native accent call fails | Opaque fallback even at full width. `GPUI_DISABLE_DIRECT_COMPOSITION=1` exercises the unsupported-renderer case on Windows. |

Refresh system preferences on appearance notifications, including
`WM_SETTINGCHANGE` with an unchanged light/dark palette. Do not poll Windows
settings on every frame. A native request returning success and a screenshot
of a pastel gradient are insufficient to establish visible transparency. On
Windows compare a pixel inside the owned window over two controlled backgrounds,
then repeat at narrow width or with direct composition disabled.

On transparent interactive surfaces draw keyboard focus with an inset ring.
A zero-blur outer shadow paints through an unfilled tab and can obscure its
icon. Verify with one tab selected and another focused in the native window.
