# Strict Desktop presentation-store verification

Date: 2026-08-30

Independent `trellis-check` rejected the earlier Desktop adapter because it
rendered an OS/English-derived locale before the persisted store loaded, fell
back after load failure, and changed the visible locale before save success.
The user selected the strict fail-closed route.

## Implemented state and boundary

- `App` owns a closed `loading | ready | unavailable` presentation-store state.
- `loading` renders a bilingual status surface and no `AppShell`.
- A rejected, corrupt, unknown, or newer store response reaches `unavailable`,
  renders one stable bilingual accessible alert, exposes no `data-locale`, and
  never renders English or Chinese shell content as a fallback.
- A successful load resolves a persisted closed tag first; a successful load
  with `language: null` may then use the already-approved OS/English resolver.
- Language saving keeps the previous locale and controlled select value,
  disables the select, and exposes a status message while awaiting the bridge.
- Visible locale changes only when the returned closed tag exactly matches the
  requested tag. Rejection or mismatch keeps the prior locale/select value and
  shows recovery-oriented copy.
- Effect-generation and in-flight guards ignore late load/save settlement after
  dependency cleanup or unmount, with all promises handled.

## Focused tests

Command:

`rtk npm test -- src/App.test.tsx src/app-shell/AppShell.test.tsx src/i18n/index.test.ts`

Final result: exit 0; App 18/18, AppShell 8/8, i18n 4/4 (30/30 total).

The focused matrix directly covers pending load/no shell, failed load/no
fallback or locale claim, successful load, pending save preserving the old
locale, matching save success, rejected save preservation, mismatched response
preservation, and late load/save settlement after unmount.

The first focused run retained the new strict tests as PASS but exposed ten
legacy workflow tests that omitted the presentation bridge. Their production
default Tauri invocation correctly failed closed, so those tests could not find
Scan. The repair injects the explicit successful fake store into those unrelated
workflow fixtures; product behavior was not weakened. The second run passed
30/30.

## Broad gates and repair evidence

- First `rtk just desktop-web-check`: recipe returned 0 because the `just`
  recipe uses semicolons, but its lint stage reported one error and the run was
  treated as failed. Complete diagnostic:
  `evidence/logs/strict-store-desktop-web-check-first-failure.log`.
- Failure: `react-hooks/set-state-in-effect` rejected a redundant synchronous
  `setPresentation({status: "loading"})`. The state is initialized to loading;
  removing only that redundant assignment preserved the strict startup gate.
- Focused `rtk npm run lint` and `rtk npm run typecheck`: exit 0.
- Repeated `rtk just desktop-web-check`: exit 0; lint, typecheck, 11 test files
  and 72 tests, plus deterministic production Vite build passed.
- `rtk just desktop-build`: exit 0; release executable and one NSIS bundle.
- `rtk git diff --check`: exit 0.
- `python ./.trellis/scripts/task.py validate
  .trellis/tasks/08-29-desktop-shell-navigation-brand`: exit 0.
- `rtk just ci`: exit 0; fmt, offline update with zero packages, workspace
  check/tests, and clippy with `-D warnings`. Complete RTK log:
  `evidence/logs/strict-store-just-ci-full.log`.

Final build artifacts:

- `target/release/devsweep-desktop.exe`: 9,789,952 bytes; SHA-256
  `41AE180E442D0BE1A9B833FBBB82D9F53A64B1F6FB1FC28AC49261B1EA7A6175`.
- `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe`: 3,117,611 bytes;
  SHA-256
  `0F61F3E1FF96A5BE5C6B368DCE8C060F282B8EE69E1AC6BA68C7D5936AB71485`.

Final post-report audit: `git diff --check` exit 0, task validation exit 0,
owned `devsweep` process count 0, and owned listener count 0. The strict repair
changed only `desktop/src/App.tsx`, `desktop/src/App.test.tsx`,
`desktop/src/app-shell/AppShell.tsx`, `desktop/src/i18n/index.ts`, and
`desktop/src/styles.css`, plus task-owned evidence/report artifacts.
