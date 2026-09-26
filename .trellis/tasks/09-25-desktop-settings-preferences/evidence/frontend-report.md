# Frontend Implementation Report

## Result

The frontend settings implementation is ready for the combined gate. Product
changes stay within desktop/src, the existing fixture generator, and the
English/Simplified Chinese catalogues. Existing custom titlebar changes remain
in place. No production dependency was added.

## Configuration Matrix

| Field | Default | Implemented consumer and timing |
| --- | --- | --- |
| theme | dark | Main/HUD shared palette after commit; system follows the OS media query. |
| font_family | system | Main/HUD local system, Segoe UI, or Microsoft YaHei UI stack after commit. |
| text_scale_percent | 100 | Main/HUD labels, controls, tables, headings, titlebar text; 100/110/125 percent. Main base 14 px, HUD base 13 px. |
| motion | system | OS reduced motion or saved reduced motion stops the Planet and decorative CSS transitions. |
| planet_fps | 30 | Actual Planet drawing uses 15 or 30 FPS. The 96-second turn and 256-pixel source cap remain. |
| status_interval_seconds | 2 | Settings and Status use the committed store. An active Status edit saves, cancels, joins, then starts its replacement. |
| status_process_limit | 15 | Snapshot and live requests use the saved returned-row limit on the next start. |
| hud_interval_seconds | 2 | The page states next-show timing. The backend owns sampler application. |
| language | Existing resolver | Existing language bridge and file remain separate. Group resets exclude language. |

## Changed Areas

- preferences/store.ts: subscribe-before-get, committed sequence ordering,
  stale/disposed response rejection, failure state, reload, and visibility read.
- preferences/PreferencesProvider.tsx, appearance.ts, appearance.css: shared
  main/HUD appearance, media listener cleanup, closed local font stacks, real
  font scaling, semantic dark/light colors, forced colors, reduced motion.
- preferences/SettingsPage.tsx: appearance, language, and performance groups;
  separate atomic resets; saved-value display; save/loading/unavailable
  feedback and retry instructions. Controls use stable settings-* IDs.
- App.tsx, AppShell.tsx, main.tsx: preference gate, Settings route/menu, and
  fixture injection while retaining the old language gate and custom controls.
- api/bridge.ts, contract.ts, fixtures, generator, generated types: exact
  snapshot/patch wire contract and desktop-preferences-changed decoding.
- stage/Planet.tsx: effective motion and frame cap reach the frame loop.
- modes/status/StatusWorkbench.tsx: committed interval and row arguments,
  commit-before-restart, failed-save retention, and disposed request guards.
  state.ts retains backend-reported runtime values and no independent saved
  interval defaults. StrictMode's discarded initial snapshot effect does not
  start a second request.
- Main/mode/support/HUD CSS: scale-aware typography and complete palette use.
  Analyze's fixed dark color-scheme was removed. Error text and destructive
  fills use separate contrast-safe tokens. HUD root overflow keeps content
  reachable in the existing native window size.
- hud/main.tsx: shared preferences and fixture-only language/status support.
- resources/i18n/en.json and zh-CN.json: parallel desktop preference labels.
  Existing shell.v1.settings strings remain unchanged because TUI uses them.

## Validation

- Node 22 npm run lint: passed.
- Node 22 npm run typecheck: passed.
- Node 22 npm run types:generate: passed. Generated output comes from the
  three desktop preference fixture files and enum catalogue.
- Focused Vitest: **111 passed, 10 files**. See frontend-focused.log.
- Final media-layout-effect adjustment: preferences and Planet tests rerun,
  **20 passed, 2 files**.

Focused tests cover exact decoding and invalid bounds; subscribe/get order;
late initialization, event ordering, listener disposal, visibility reload;
unavailable-store recovery; committed-only save behavior; group reset scope;
local fonts/scales; OS theme changes; actual 15 FPS/reduced-motion scheduling;
both Status command argument paths; delayed save/cancel/join/restart order;
save failure retaining an active Status run; generated-type consistency; and
the exact three allowed Tauri import sites. Semantic text, muted, error, warning,
and success tokens meet a 4.5:1 contrast test against all three principal
surfaces in both palettes.

An earlier intended focused npm run test invocation ran all Vitest tests
because the package script contains &&. Changed-copy and style assertions were
corrected. Later focused runs use npm exec -- vitest. The parent owns the final
combined full suite, build, and Rust gates.

## Evidence Limits

Fixture mode (VITE_FIXTURE_BRIDGE=1) supports main and /hud.html without native
preference/language/status calls. Its store is local to each preview entry;
fixture windows do not simulate cross-process persistence or native events.
Native event permissions and persisted HUD sampler cadence are backend
responsibilities. This report makes no native-window or native-scale acceptance
claim. The parent owns browser/native evidence and labels.

No cleanup, uninstall, optimization, user-settings-file mutation, commit, or
task archive was performed by this frontend implementation. Existing operation
performance and native acceptance findings remain unchanged.
