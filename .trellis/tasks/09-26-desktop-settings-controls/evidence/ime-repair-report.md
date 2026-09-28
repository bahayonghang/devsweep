# Settings font combobox IME repair

Date: 2026-09-28 UTC. Scope: the font combobox keyboard guard and focused tests.

## Native regression evidence

`native-ime-before-fix.json` preserves the WebView2 probe from
2026-09-28 03:10:51.960 through 03:10:58.309 UTC. The method was CDP native
renderer composition. The probe did not exercise the Windows IME candidate
window.

The recorded sequence was composition start, update to `宋`, composing input,
composing ArrowDown, composing Enter, then composition end. The input held
`宋` during composition. The old list still contained 249 options. The parent
confirmed that the unintended saved font became `kind: system`. The list
filters after composition ends; that timing is unchanged by this repair.

The persisted V2 preference SHA-256 changed during the composing Enter:

- Before: `854b6fd10b9003c21e28f74ef35dc0401ae339395ecd816b85cbf528403e6537`.
- During: `cd688a44654bbdd0307cfd556ad8536b5d2187a6f34dc86741b2767f5175578d`.

The record has `nativeComposingEnter: true` and `result: FAIL`. The later
Escape close check timed out after the unintended save disabled or blurred
the input.

## Cause and change

Installed Base UI 1.8.0 guards its Enter selection with `event.which === 229`
in `combobox/input/ComboboxInput.js`. The observed native composition event
has `isComposing: true` with ordinary ArrowDown and Enter key codes. Base UI
could therefore highlight the old list and emit `item-press`. The wrapper
accepted that changed value and called its save callback.

`desktop/src/components/SettingsCombobox.tsx:47` now handles input keydown
before the merged Base UI handler. When `nativeEvent.isComposing` is true or
`nativeEvent.keyCode` is 229, the handler calls the typed
`event.preventBaseUIHandler()` API. Base UI then skips its keyboard handler.
The wrapper does not call DOM `preventDefault()` or `stopPropagation()`.
Native composition defaults, event observation, and explicit pointer choice
remain available. No query, filter, dependency, or save-owner logic changed.

The prior IME test used key code 229 without a highlighted uncommitted
option. That case could pass while native `isComposing` Enter still saved
another option.

## Regression coverage and checks

The updated cases in `desktop/src/components/settings-choices.test.tsx` cover
the composition sequence, composing ArrowDown and Enter with ordinary key
codes, a previously highlighted option, the legacy 229 fallback, ordinary
Enter after composition, explicit pointer selection, and Escape dismissal.
They assert no unintended save, no canceled native default, retained input
text and focus, and one save for an explicit choice. Filtering assertions
run after composition end.

Commands ran from `desktop/` with Node 22 through mise:

| Check | Result |
| --- | --- |
| Before the guard: `mise exec node@22 -- npx vitest run src/components/settings-choices.test.tsx --maxWorkers=1` | Exit 1; 13 passed, 2 failed. The native composition sequence saved `system`; native composing Enter on a highlighted option saved `yahei`. |
| After the guard: `mise exec node@22 -- npx vitest run src/components/settings-choices.test.tsx src/preferences/fonts.test.tsx src/preferences/preferences.test.tsx --maxWorkers=1` | Exit 0; 3 files, 41 tests passed. Counts: controls 15, fonts 11, preferences 15. |
| `mise exec node@22 -- npx eslint src/components/SettingsCombobox.tsx src/components/settings-choices.test.tsx` | Exit 0. |
| `mise exec node@22 -- npm run typecheck` | Exit 0. |

`git diff --check` on the two source files also returned no findings.

## Source hashes and acceptance boundary

| File | SHA-256 |
| --- | --- |
| `desktop/src/components/SettingsCombobox.tsx` | `060c994390351faec055472cea68dde7a2f33a975c3edb1f047aa23fd6260a72` |
| `desktop/src/components/settings-choices.test.tsx` | `436f026b90dfb5c2fdf32ff367cec3305d69de13c8269b86fb9af267b720f746` |

These checks provide focused evidence for C-AC2 committed-value preservation,
single-choice saves, dismissal focus, and the C-AC5 IME keyboard boundary.
They do not close the complete C-AC2 or C-AC5 criteria. Popup geometry, native
high contrast, reduced motion, and Windows candidate-window interaction are
outside this repair's checks.

The parent owns the release rebuild and native WebView2 retest. This report
does not claim post-repair native acceptance. No Cargo or native build ran
in this repair scope.
