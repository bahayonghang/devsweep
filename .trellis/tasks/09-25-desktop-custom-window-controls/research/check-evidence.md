# Custom Window Controls: Review Evidence

## Scope

The review used `09-25-desktop-custom-window-controls` from the dispatch. The
saved hook context named the settings child. The review therefore loaded this
window child's PRD, design, implementation plan, check manifest, referenced
specs, parent window research, and implementation evidence directly.

Reviewed product files:

- `desktop/src/lifecycle.ts`
- `desktop/src/app-shell/WindowTitlebar.tsx`
- Window integration in `desktop/src/App.tsx` and `desktop/src/main.tsx`
- Titlebar rules in `desktop/src/styles.css`
- `desktop/src-tauri/tauri.conf.json` and both capability files
- Window capability assertions in `desktop/src-tauri/src/lib.rs`
- Existing close and HUD shutdown paths in `desktop/src-tauri/src/tray.rs`
- Window controller, titlebar, App window, App, and lifecycle tests

## Findings (fixed)

No code defects were found. No product files were changed in this review.

## Findings (not fixed)

No code defects remain from this review. Native acceptance remains open and
belongs to the parent integration review. Unit tests do not establish Windows
dragging, physical double-click, edge resize, taskbar restore, Win+Arrow,
Alt+F4, monitor work-area behavior, or native DPI behavior.

The titlebar routes `mousedown` with `detail === 2` to maximize/restore. Native
testing must check that physical double-click produces that sequence after the
first native drag request. The component test supplies the event sequence;
the component test does not establish the native result. No native defect is
claimed without native evidence.

Combined dark/light/system, fonts, forced colors, and viewport evidence remains
with the settings child and parent. The full desktop and Rust gates also remain
with the parent. Maximize-hover Snap Layouts remain outside the approved scope.

## Verified Code Paths

- The production bridge delegates minimize, toggleMaximize, isMaximized,
  startDragging, close, and onResized to the current native window. Components
  import no Tauri API.
- Maximize state comes from native reads. Per-connection sequence checks reject
  stale reads. Disposal rejects late read results and removes both installed
  and late-arriving resize listeners. Listener registration triggers a new read
  to cover a resize during registration.
- Pending actions prevent duplicate toggles. Failed reads retain the last
  confirmed state. State, action, and subscription failures expose retry. A
  retry also repairs a missing subscription after a later action failure.
- Drag handling belongs to the explicit title region. Buttons are siblings of
  that region. Right-click and interactive control events do not start dragging.
- The close button requests native close. App retains the existing lifecycle
  listener and shared drain. The installed SDK's `onCloseRequested` implementation
  awaits the handler before calling `destroy`. No button calls `destroy`.
- Cooperative and wait-only test runs remain owned until their join promise
  settles. Repeated close requests and StrictMode replay share the drain.
  Minimize does not cancel active work.
- The default capability applies only to `main` and grants the exact window
  methods. The HUD capability still grants only event listen/unlisten.
- Main dimensions remain 1080 by 720 with a 900 by 600 minimum. Only the main
  decoration flag changes. Main destruction still exits the application and
  joins the HUD sampler through the existing backend path.
- The titlebar remains present during language loading and store failure. Both
  locales provide accessible names and tooltips. Keyboard activation and focus
  pass the component tests.

## Verification

All commands ran with the repository's Node 22 toolchain from `desktop/`.

| Check | Command | Result |
| --- | --- | --- |
| Focused tests | `mise exec node@22 -- npx vitest run src/window-controls.test.ts src/app-shell/WindowTitlebar.test.tsx src/lifecycle.test.ts src/App.window.test.tsx src/App.test.tsx` | Pass: 5 files, 56 tests |
| Lint | `mise exec node@22 -- npm run lint` | Pass |
| TypeCheck | `mise exec node@22 -- npm run typecheck` | Pass |

Focused test counts: lifecycle 4, window controller 5, titlebar 7, App window 3,
and App 37. No real cleanup or other system mutation was performed.

The window-owned product diff also passed `git diff --check`. The desktop
build, complete test suite, `just ci`, and native acceptance were not run by
this reviewer. The parent owns those checks for the combined change set.
