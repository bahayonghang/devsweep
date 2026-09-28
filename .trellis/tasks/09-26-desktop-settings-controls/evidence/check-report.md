# Settings Controls Independent Check

Date: 2026-09-26. Result: **PASS for the controls code-review gate**.
The host-font child can proceed. Parent native/integration acceptance remains
pending, as assigned in the review dispatch.

## Findings (fixed)

### 1. Keyboard search lost input while opening the font popup

- File: `desktop/src/components/SettingsCombobox.tsx:30-39`.
- Reproduction: Tab to the closed font input, press Ctrl+A, and type `yahei`.
  The regression test received `ahei`. A second case, Ctrl+A then Backspace
  before typing, received `System UIyahei`. Neither case wrote a preference.
- Cause: Base UI reports the input change before the open change. The wrapper
  reset the query on every open change. With an empty replacement, the popup
  stayed closed and its controlled input displayed the committed label again.
- Fix: Preserve the query when input opens the popup. Explicit input changes
  open the query view, including an empty query. Closing still clears the
  query and restores the committed label. Selection still requires
  `item-press`, a non-null option, and a changed committed value.
- Regression: Two keyboard replacement cases at
  `desktop/src/components/settings-choices.test.tsx:98-109`. Both assert the
  complete query, the matching option, and no preference callback.
- Spec: Added the keyboard-open and empty-query rule to the existing Settings
  choices section in `.trellis/spec/desktop-frontend/component-guidelines.md`.

### 2. Select keyboard test assumed focus completed between bundled keys

- File: `desktop/src/components/settings-choices.test.tsx:43-62`.
- Issue: An affected-suite run reached Enter without the expected changed
  selection callback. The test sent ArrowDown, End, and Enter as one sequence
  without waiting for the popup focus lifecycle.
- Fix: Wait for the selected option to receive focus after opening, then wait
  for the last option to receive focus before Enter. The test retains the
  no-write-on-highlight and Escape assertions. Production Select code did not
  change.

## Findings (not fixed)

- One intermediate run of the existing Select typeahead test could not find
  the English option after the initial click. The final affected suite passed
  that test. The cause is not established. No production change is justified
  by that single observation. Record any recurrence during parent integration;
  passing tests were not repeated solely to increase the run count.
- Native WebView2/CSP, screen-reader behavior, IME behavior on Windows, popup
  edge geometry, forced colors, and screenshots at the four widths remain
  assigned to parent integration. DOM tests and CSS review do not establish
  those native results. No code-review blocker remains in the controls scope.

## Contract review

- Select and Combobox preserve typed option values and use the approved
  scoped Base UI entry points. Search aliases affect filtering only. No
  query, hover, highlight, dismissal, or current-value selection sends a
  preference patch. The SettingsPage call sites each construct one existing
  field patch. Theme tiles retain native radio semantics.
- SettingsPage uses the existing committed preference store. Loading, saving,
  unavailable storage, failed saves, retry, group reset boundaries, reduced
  motion, and independent language persistence remain covered by preference
  tests. This child does not change the store or persistence schema.
- Popups use document portals, root palette tokens, a 6px offset and 8px
  collision padding. Select disables overlapping alignment. Popup width and
  height use available-space variables, with internal scrolling. CSS defines
  the shared 1040px layout, 800px row breakpoint, 480px theme breakpoint,
  selected checks, distinct highlights, focus outlines, and disabled states.
- Font options retain full labels and aliases. Loading/unavailable status is
  localized by the caller and does not disable other settings. A missing
  committed option remains visible. Native enumeration and the 2000-family
  check belong to the font child/parent boundary.
- Dependency manifest and lockfile pin Base UI 1.8.0. The implementation
  evidence records MIT licensing, React 19 peers, scoped imports, and the
  production-size change. Only two source-catalogue strings were added; no
  generated IPC or catalogue output was hand-edited.

## Verification

Commands ran from `desktop/` with Node 22 through mise, except diff checking
at the repository root.

| Check | Result |
| --- | --- |
| `mise exec node@22 -- npx vitest run src/components/settings-choices.test.tsx src/preferences/preferences.test.tsx` | Exit 0; 2 files, 27 tests passed (12 choice tests, 15 preference tests) |
| `mise exec node@22 -- npm run lint` | Exit 0 |
| `mise exec node@22 -- npm run typecheck` | Exit 0 |
| `git diff --check` | Exit 0; Git emitted existing LF-to-CRLF notices |

The original first-character regression failed before the fix. The explicit
Backspace variant also failed before the empty-query fix. The final focused
run passed both regressions, the existing pointer/keyboard/IME/dismissal
cases, and preference integration tests.

The implementation report records the earlier 110-test, lint, typecheck, and
production-build gate. The reviewer did not rerun the unchanged App, shell,
i18n, or style suites, nor the production build. The earlier build is evidence
for the implementation before these local fixes; the parent owns the final
integrated build and native checks.

Reviewer edits are limited to SettingsCombobox, its existing choice tests, the
affected component guideline, and this report. Navigation and unrelated work
remain intact. No commit, archive, task-status change, application install,
font enumeration, palette addition, or system-setting change was performed.
