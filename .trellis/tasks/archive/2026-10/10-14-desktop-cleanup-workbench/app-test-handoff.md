# App and shell test adaptation — 2026-10-14

Ownership was limited to `desktop/src/App*.test.tsx` and
`desktop/src/app-shell/*.test.*`; only `desktop/src/App.test.tsx` required edits.
No production code, Clean mode tests, Review tests, shell routing or window
lifecycle implementation was changed in this subtask.

## Behavior retained and adapted

- Replaced the old single Scan/Earth assumptions with named project/global
  actions and explicit overview-to-detail transitions. Project mocks now use
  the Rust-generated project-only report with all targets initially unselected.
- Preserved StrictMode activation, load/save failure and stale-response
  handling, language persistence, close/unmount drain, coordinator cancellation
  and join-before-start, malformed progress cancellation, and read-only stopped
  previews. Preview tests assert bounded busy/read-only state instead of a
  removed decorative stage.
- Project cleanup now explicitly selects a target, previews, deselects to
  invalidate authority, and obtains a new preview before confirmation. Execution
  receives the original project plan, exact selected IDs and current digest.
- Added a separate global command path preserving Cargo-home inspect-only
  refusal, an irreversible-command confirmation, partial capacity meaning and
  no false claim of physically freed space. Added a two-project test preserving
  the partial select-all indeterminate state and full original-plan preview.
- Added root-settings failure coverage: the project action is unavailable while
  global scan remains enabled and sends `include_projects=false`,
  `include_global=true`, `roots=[]`.
- Locale/Settings round trip now explicitly asserts the completed report and
  review action remain available before a second scan, with identical
  locale-neutral request flags and roots.

## Evidence

Before adaptation: 21 test failures, all in App.test.tsx, against old Scan/Earth
selectors or layout assumptions. The preserved baseline report is generated
output at `dist/app-tests-before.json`.

Final command with Node 22.23.2:

```text
node node_modules/vitest/vitest.mjs run src/App.test.tsx src/App.window.test.tsx src/app-shell --reporter=json --outputFile=../dist/app-tests-final.json
```

Result: **7 files, 90 tests passed, 0 failed**. App.test.tsx has 41 passing tests;
the unchanged App.window and app-shell tests account for the other 49.
TypeScript `--noEmit`, scoped ESLint for App and app-shell tests, and
`git diff --check` also passed after the final edit.

The intermediate duplicate safety-caption matcher was scoped to the project
overview region because the closed native dialog also retains that caption in
the DOM. No product change was made to satisfy the matcher.

Native screenshot/keyboard and real scan/preview acceptance belongs to parent/F;
these fixture tests do not establish native or performance acceptance.
