# Research: Main Window Controls

- Query: Replace the native caption controls and preserve window and operation lifecycle behavior.
- Scope: Repository and installed Tauri SDK.
- Task creation date: 2026-09-25, as recorded by the local task tool.

## Findings

| Evidence | Current behavior | Planning consequence |
| --- | --- | --- |
| `desktop/src-tauri/tauri.conf.json:15` | Main window title is DevSweep; initial size is 1080 by 720, minimum is 900 by 600; resizing is enabled. No decoration override is set. | Add an explicit main-window decoration setting only when the replacement controls are ready. Preserve sizes. |
| `desktop/src/app-shell/AppShell.tsx:349` | AppShell renders a centered capsule header. | Place a compact custom titlebar above the capsule. Do not overlay window buttons on navigation. |
| `desktop/src/app-shell/AppShell.tsx:103` and `:469` | The settings deep link exists and renders a language selector. | Expand the existing route; do not add another settings route or sixth primary mode. |
| `desktop/src/lifecycle.ts:22` and `:43` | The Tauri close listener awaits the controller's shared drain. The controller caches one drain promise. | Custom close must request a native close event through the same adapter. The button must not call destroy directly. |
| `desktop/src/App.tsx:125` | App registers the close listener and guards effect cleanup across React replay. | Preserve listener ownership, asynchronous registration cleanup, and StrictMode behavior. |
| `desktop/src-tauri/src/tray.rs:194` | Main-window destruction exits the app. HUD blur/destruction hides the HUD and stops sampling. | Keep close as application exit. Do not introduce close-to-tray or autostart preferences. |
| `desktop/src-tauri/src/tray.rs:255` | The HUD already uses an undecorated window. | The new titlebar targets the main window. Keep the HUD's compact window behavior. |
| `desktop/src-tauri/capabilities/default.json:8` | Main-window permissions grant event listen/unlisten and destroy only. | Add the exact window permissions needed by the new adapter. |
| `desktop/src-tauri/src/lib.rs:66` | The HUD may invoke only the presentation settings read command. | Never copy main-window control or settings-write permissions to the HUD. |
| `.trellis/spec/desktop-frontend/type-safety.md:43` | Only three frontend files may import Tauri APIs. | Extend `lifecycle.ts` for window operations. Components receive an injected typed bridge. |

## Installed SDK Evidence

The installed SDK contains `isMaximized`, `toggleMaximize`, `minimize`,
`startDragging`, and `onResized` in
`desktop/node_modules/@tauri-apps/api/window.d.ts:402`, `:697`, `:708`,
`:1085`, and `:1214`. These are local dependency observations, not a claim
about a newer SDK release.

`desktop/node_modules/@tauri-apps/api/window.js:1632` awaits the close
handler, then calls destroy unless the handler prevented the default action.
The current application relies on that ordering. A replacement close button
must use the close request path and preserve the existing drain.

The generated local schema lists the following granular permissions:

- `core:window:allow-close`, `desktop/src-tauri/gen/schemas/desktop-schema.json:1280`.
- `core:window:allow-is-maximized`, same file `:1382`.
- `core:window:allow-minimize`, same file `:1418`.
- `core:window:allow-start-dragging`, same file `:1688`.
- `core:window:allow-toggle-maximize`, same file `:1712`.

Keep generated schemas and `node_modules` unchanged. Capability tests must
cover the explicit application-owned permission list.

## Design Direction

Use one application-drawn titlebar with a product label, drag region, and
three original SVG controls: minimize, maximize/restore, and close. Use real
buttons with localized accessible names, focus states, hover states, and
disabled/pending states. A button must not start dragging.

Native window state owns whether the second button is maximize or restore.
Read the initial state and refresh it after resize events and successful
toggle requests. Ignore stale state reads after disposal. Do not infer
maximization from CSS dimensions.

Request close through `lifecycle.ts`. Preserve wait-only cleanup completion,
cooperative cancellation/join, and the existing HUD shutdown. Minimize must
not close or cancel a cleanup operation.

## User Decisions And Spec Changes

The user explicitly requested custom-drawn minimize and maximize controls.
The native-chrome clause in
`.trellis/spec/desktop-frontend/component-guidelines.md:6` must change during
the implementation task. The remainder of the capsule and stage layout stays.

The user also confirmed dark, light, and system themes for every page and the
HUD, with dark as default. The dark-only clause at `:145` must change during
the settings task. The plan must retain mode accents, semantic colors,
reduced motion, forced colors, and readable warning/confirmation surfaces.

## Validation Boundary

- Component/bridge tests: actions, accessible labels, maximize state races,
  listener cleanup, pending/error states, and non-drag interactive controls.
- Lifecycle tests: repeated close, active operation drain, wait-only cleanup,
  and React effect replay. Preserve existing tests at their current seam.
- Native Windows evidence: drag, double-click maximize/restore, edge resize,
  taskbar minimize/restore, Win+Arrow snapping, Alt+F4, monitor/work-area
  behavior, and scale-aware controls. Browser mocks cannot prove these rows.
- Record actual Windows scale separately from WebView scale and CSS viewport
  emulation. Do not change the user's display configuration.

## Caveats And Deferred Work

Windows 11 maximize-hover Snap Layouts are not established by the current
React button or the inspected SDK signatures. Do not claim support without a
native result. A custom non-client hit-test implementation is outside the
initial scope; reconsider only with an explicit scope decision. Standard
edge and keyboard snapping remain required native checks.

This task records focused evidence for the new titlebar. It does not close
the existing `09-20-desktop-native-acceptance` task or replace its pending
operator rows.
