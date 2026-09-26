# Custom Window Controls: Implementation Evidence

## Product Changes

- `desktop/src/lifecycle.ts`: typed `WindowControlBridge`, native method adapter, race-safe maximize state controller, resize subscription cleanup, retriable failures, and a local browser fixture bridge.
- `desktop/src/app-shell/WindowTitlebar.tsx`: original SVG controls, localized accessible names and tooltips, button pending state, blank-region drag and double-click handling, and visible retry feedback.
- `desktop/src/App.tsx`: one titlebar above the shell and presentation-store gates. Controls remain available when language loading fails. The existing App close listener and lifecycle drain remain the close owner.
- `desktop/src/main.tsx`: fixture mode supplies the local lifecycle/window bridge. Fixture controls make no native calls.
- `desktop/src/styles.css`: separate sticky titlebar row, 46 by 40 CSS-pixel minimum button targets, semantic theme colors, focus indicators, and forced-colors rules. The capsule retains its centered layout.
- `resources/i18n/en.json` and `resources/i18n/zh-CN.json`: matching `window.v1.*` control and error messages.
- `desktop/src-tauri/tauri.conf.json`: main-window `decorations: false`; initial and minimum sizes stay unchanged.
- `desktop/src-tauri/capabilities/default.json` and `desktop/src-tauri/src/lib.rs`: exact main-only permissions and the corresponding assertion. HUD capabilities stay unchanged.

## Tests

New tests are in `desktop/src/window-controls.test.ts`, `desktop/src/app-shell/WindowTitlebar.test.tsx`, and `desktop/src/App.window.test.tsx`. The existing `desktop/src/App.test.tsx` now provides local window controls.

The tests cover native-method delegation, external resize state, stale/late reads, read failure recovery, listener retry and cleanup, StrictMode replay, duplicate toggles, localized labels and keyboard activation, non-drag buttons, title-region drag/double-click, presentation-store failure, minimize preserving work, repeated close, cooperative work, and wait-only execution. The native-close fixture records destruction only after the shared drain resolves.

Commands ran from `desktop/`:

1. `mise exec node@22 -- npx vitest run src/window-controls.test.ts src/app-shell/WindowTitlebar.test.tsx src/lifecycle.test.ts src/App.window.test.tsx src/App.test.tsx` — 5 files, 55 tests passed.
2. After adding recovery for a subscription failure followed by an action failure: `mise exec node@22 -- npx vitest run src/window-controls.test.ts src/api/ipc-boundary.test.ts src/i18n/index.test.ts src/styles.test.ts` — 4 files, 22 tests passed. The controller suite now has 5 tests.
3. `mise exec node@22 -- npm run lint` — passed after the final product edit.
4. `mise exec node@22 -- npm run typecheck` — passed after the final product edit.

## Remaining Integration Evidence

The parent owns the combined desktop build/test and `just ci` gates. No Rust test or native Windows window check ran in this implementation pass. The current tests cannot establish native drag, edge resize, taskbar restore, Win+Arrow, Alt+F4, work-area/monitor behavior, or native DPI behavior. Those acceptance rows remain open.

Parent integration must review the titlebar with the settings child's dark/light/system themes and font settings. The titlebar consumes shared semantic tokens and inherits typography; it has no separate theme state. Maximize-hover Snap Layouts remain deferred by the approved scope.
