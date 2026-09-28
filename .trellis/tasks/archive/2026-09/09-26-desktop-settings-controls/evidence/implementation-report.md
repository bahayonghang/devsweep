# Settings Controls Implementation

Date: 2026-09-26. Active child: `09-26-desktop-settings-controls`.
Ready for the independent code check. Native visual acceptance remains with
the parent integration task.

## Changes

- Added `desktop/src/components/SettingsSelect.tsx` and
  `desktop/src/components/SettingsCombobox.tsx`. The wrappers use only the
  approved Base UI Select and Combobox entry points. Values remain typed.
  Query, highlighting, hover, Escape, outside click, Tab, and choosing the
  committed value do not write preferences. Explicit item selection calls one
  callback. The caller supplies the committed value after persistence succeeds.
- Updated `desktop/src/preferences/SettingsPage.tsx`: seven fixed-choice
  Select controls, one searchable font Combobox with the existing three
  choices, and three native-radio theme previews. Appearance, language, and
  performance share aligned rows. Reset patches and performance values remain
  the existing values. Language saving remains independent.
- Updated `desktop/src/styles.css` and
  `desktop/src/preferences/appearance.css`. The Settings container and title
  align at a 1040px maximum width. Rows stack below 800px; theme tiles use one
  column below 480px. Controls use a 40px minimum height. Popups portal to the
  document root, use inherited appearance tokens, disable Select overlap, use
  a 6px offset and 8px collision padding, and constrain width/scroll height to
  the library's available-space variables. Selected options show a check;
  highlighted options have a distinct background and outline.
- Added two font search/empty strings to `resources/i18n/en.json` and
  `resources/i18n/zh-CN.json`. The existing i18n adapter imports these source
  catalogues directly. No generated catalogue or IPC output was edited.
- Added `desktop/src/components/settings-choices.test.tsx`; extended
  `desktop/src/preferences/preferences.test.tsx` and
  `desktop/src/styles.test.ts`. Updated only affected language-control
  interactions and value assertions in `desktop/src/App.test.tsx` and
  `desktop/src/app-shell/AppShell.test.tsx`. Prior navigation changes remain.
- Updated control-specific guidance in
  `.trellis/spec/desktop-frontend/component-guidelines.md` and
  `.trellis/spec/desktop-frontend/type-safety.md`.

The dependency and lockfile already contained the interrupted worker's
installation at handoff. The final tree pins `@base-ui/react` 1.8.0. Installed
package metadata states MIT, copyright 2019 Material-UI SAS, and React/React DOM
peers `^17 || ^18 || ^19`. The installed app uses React/React DOM 19.2.8.
The date-fns peers are optional and are not used by these scoped imports.
No other production dependency was added by this child.

## Validation

All commands below ran from `desktop/` with Node 22 via mise.

| Command | Result |
| --- | --- |
| `mise exec node@22 -- npx vitest run src/components/settings-choices.test.tsx src/preferences/preferences.test.tsx src/App.test.tsx src/app-shell/AppShell.test.tsx src/styles.test.ts src/i18n` | Exit 0; 6 files, 110 tests passed |
| `mise exec node@22 -- npm run lint` | Exit 0 |
| `mise exec node@22 -- npm run typecheck` | Exit 0 |
| `mise exec node@22 -- npm run build` | Exit 0; production Vite build |
| `mise exec node@22 -- npm ls --omit=dev --depth=0` | Exit 0; Base UI 1.8.0, React/React DOM 19.2.8 |
| `git diff --check` from repository root | Exit 0 |

The 110 tests comprise 10 choice interaction tests, 15 preference tests, 38 App
tests, 30 shell tests, 12 style tests, and 5 i18n tests.

Interaction evidence covers pointer and Enter selection, keyboard highlight,
typeahead, Chinese composition and IME Enter, case-insensitive alias search,
long bilingual names, Escape, outside dismissal, Tab continuation, focus
restoration, selected indicators, missing committed names, no-op current
selection, empty results, loading/unavailable status, disabled controls,
committed values during save/failure, retry, independent language persistence,
group reset boundaries, and reduced-motion frame-limit descriptions.

jsdom does not read the `FocusOptions.preventScroll` getter used by Base UI's
outside-dismissal feature detection. The component test installs a scoped
focus spy that reads this option and delegates to jsdom focus. The production
code uses Base UI's default focus lifecycle. Native WebView2 focus and IME
operation remain parent evidence. The tests do not simulate native rendering.

Style tests calculate WCAG contrast for actual dark/light token values.
Selection text meets 4.5:1; control borders against raised/canvas surfaces and
focus against the highlighted background meet 3:1. Existing general palette,
radius, forced-color, and reduced-motion tests also pass.

An initial typecheck after the new tests found a missing `this: HTMLElement`
annotation in the focus spy. The annotation is present, and the final
standalone typecheck and build both pass. Earlier native-select assertions
were updated to the new role/visible-value interface before the final tests.

## Production size

Compare with the pre-Base-UI baseline recorded in the navigation child's
`evidence/implementation-report.md`. Generated `desktop/dist` files remain
ignored.

| Output | Bytes | Build-reported gzip kB |
| --- | ---: | ---: |
| `main-x8JxH8L3.js` | 173938 | 43.01 |
| `state-Bkmn2BDR.js` | 647361 | 163.88 |
| `hud-DGI7Ez0m.js` | 2686 | 1.18 |
| `main-DD1IOE9e.css` | 55057 | 10.06 |
| `hud-BzDJWnO3.css` | 3810 | 1.38 |

The state chunk increased by 157563 bytes and 54.44 build-reported gzip kB.
The main JavaScript byte count is unchanged. Vite reports a chunk above
500 kB after minification. The build succeeds; no chunk-splitting change is
included in this control task.

## Acceptance boundary

- C-AC1: shared styles, labels/descriptions, committed display, selected checks,
  and disabled states are implemented and covered by focused tests. Native
  screen-reader and visual evidence remains pending.
- C-AC2: portals, collision/scroll constraints, dismissal, focus, explicit
  selection, and no writes during highlight are implemented. Interaction and
  portal tests pass. Viewport-edge geometry remains parent evidence.
- C-AC3: common sections/row markup, 1040px alignment, 800px row threshold,
  480px theme threshold, and scale-aware control text are implemented. No
  pixel-level claim is made for 390/800/1024/1440px or 125 percent text.
- C-AC4: load/save/unavailable states, failed-save retention, independent
  language saving, frame-limit reasons, and current-value no-op tests pass.
- C-AC5: keyboard/typeahead/filter/IME event tests and long bilingual names
  pass. Forced colors and reduced motion have shared root rules. Native open
  popup rendering, WebView2 CSP, scaling, and screen-reader evidence remain
  parent integration work.

No app installation, system scaling change, native font enumeration, new
palette, schema migration, commit, archive, or task-status change was made.
The parent owns the final complete desktop and Rust gates.

## Font child handoff

`SettingsCombobox<Value extends string | number>` accepts the same
`SettingsChoiceProps<Value>` as Select: `id`, `label`, optional `description`,
committed `value`, readonly `options`, optional `disabled`/`busy`, and
`onValueChange(value)`. Each `SettingsOption<Value>` has `value`, `label`, and
optional readonly `aliases`. Search uses the display label and every alias,
case-insensitively, with no result cap. All option rows use the current UI font.

The Combobox also accepts localized `placeholder`, `emptyLabel`, and optional
`status: { kind: "loading" | "unavailable"; message: string }`. Status is
associated with the input and announced; loading sets `aria-busy`. Status
alone does not disable available options or other settings.

The font child should supply catalogue-backed options, keep the system choice,
map the canonical selected value to its V2 patch in SettingsPage, and own
enumeration/refresh/missing-font notices outside the picker. A saved value
absent from options remains visible as its string value. The child must supply
stable, collision-free option values for the system choice and font families.
The existing input/query state never commits a string typed by the user.
The parent still requires the planned 2000-family catalogue check.
