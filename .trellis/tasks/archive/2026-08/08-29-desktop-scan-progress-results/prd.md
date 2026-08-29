# Desktop scan progress and live categorized results

## Goal

Make a desktop scan understandable and useful while it is still running: show
truthful phase-aware progress, continuously surface already-discovered cleanup
targets, and organize that evidence without weakening DevSweep's dry-run-first
safety model.

## Background

- The supplied screenshot shows an active combined project/global scan while the
  content area still presents the completed empty-state copy `No scan results`.
- The shared scanner already has a progress callback and the TUI can stage partial
  target snapshots, but the desktop bridge currently forwards only phase/message
  text and deliberately redacts the internal partial cleanup plan.
- The user selected continuous read-only target previews with a phase-aware
  indeterminate progress bar. Completed-phase-only refresh and a synthetic or
  newly engineered percentage are not the selected product behavior.
- Planning evidence and exact code anchors are recorded in
  `research/current-scan-flow.md`.

## Requirements

- **R1 — Truthful progress.** During an active scan, show the requested scan
  phases, the exact backend-owned current message, and an indeterminate progress
  bar. Do not derive a percentage from elapsed time, phase count, target count, or
  provider count when those values are not a real bounded total.
- **R2 — Continuous preview.** Emit and display cumulative read-only Scan
  Previews as fully constructed, safety-checked targets become available, rather
  than waiting for the whole project/global pipeline to finish.
- **R3 — Preview/report separation.** A Scan Preview must be visibly incomplete,
  stored separately from the completed Scan Report, and unable to drive selection,
  dry-run, confirmation, or execution. Only the final report establishes default
  selection and review authority.
- **R4 — Safe projection.** Desktop progress events must not expose trusted
  program, argv, cwd, direct cleanup-action authority, cleanup intent, or default
  selection. The purpose-built observation DTO must not be a submit-ready
  `UntrustedPlan`. Projection and validation belong to the Rust boundary; React
  must not recreate or strip action contracts.
- **R5 — Classification and stability.** Organize preview targets primarily by
  scope (`Projects`, `Global caches`), expose ecosystem counts
  (`Rust`, `Node`, `Python`, `Generic`), retain target kind as row detail, and keep
  cumulative snapshots deduplicated with deterministic ordering.
- **R6 — Correlation and bounded delivery.** Every desktop progress event must be
  correlated to one scan run and ordered within that run. Ignore stale, duplicate,
  or out-of-order events. Use a command-scoped ordered stream rather than the
  application-global event bus, and bound event delivery so a large recursive scan
  cannot flood Tauri or React.
- **R7 — State-specific recovery.** Starting, cancel-requested, canceled/partial,
  failed, empty-complete, completed-with-results, and rescan states must use
  distinct copy and behavior. Preserve the latest safe preview after cancellation
  or failure while keeping it non-interactive, preserve the previous completed
  report as recoverable authority during a failed rescan, and classify cancellation
  explicitly rather than inferring it from partial health.
- **R8 — Workbench presentation.** Preserve the quiet, dense Operate surface:
  scope controls and cancel action remain stable, progress gains clear hierarchy,
  live results replace the contradictory empty state, and no decorative card grid,
  hero treatment, or motion-only status is introduced.
- **R9 — Accessibility and responsive behavior.** The active scan must remain
  keyboard legible and screen-reader understandable, expose status without
  announcing the entire changing table, honor reduced motion, and work at the
  shipped desktop size and the established narrow fixture viewport.
- **R10 — Shared semantics.** Core remains the owner of scan aggregation,
  deduplication, ordering, and safe projections. The CLI's final text/JSON behavior
  remains compatible. The TUI consumes the same richer progress semantics without
  creating a second scanner model and treats staged targets as preview-only until
  `ScanFinished` promotes the final plan.

## Acceptance Criteria

- [ ] **AC1 (R1, R8).** An active scan renders a labelled indeterminate progress
  element, requested phase states, the exact backend message, and the Cancel action;
  no visible or accessible percentage is fabricated.
- [ ] **AC2 (R2).** A controlled single-phase fixture that discovers multiple
  targets produces and renders at least one cumulative preview before that phase
  and the overall scan finish.
- [ ] **AC3 (R3, R4).** Preview serialization contains display facts but no trusted
  program/argv/cwd/direct action, cleanup intent, default-selection field, or
  submit-ready plan envelope; preview rows expose no selection checkbox, select-all,
  dry-run action, confirmation path, or executable reducer transition.
- [ ] **AC4 (R5).** Live targets are grouped into Projects and Global caches with
  accurate ecosystem counts and target-kind detail; repeated cumulative snapshots
  do not duplicate rows and use deterministic ordering.
- [ ] **AC5 (R6).** Reducer/bridge tests prove that the active scan accepts only a
  higher sequence for its own scan id, ignores stale/out-of-order events, and the
  command-scoped ordered transport's bounded-delivery policy still flushes the
  latest snapshot at phase and terminal boundaries without cross-window or
  prior-invocation leakage.
- [ ] **AC6 (R7).** Starting with zero preview rows never shows completed-empty copy;
  cancel-requested keeps the current preview visible; failure/cancel preserves it
  as read-only evidence; a failed rescan does not destroy the previous completed
  report; explicit canceled, completed-partial, final-empty, and final-non-empty
  outcomes transition to distinct states.
- [ ] **AC7 (R3, R7).** A completed Scan Report atomically replaces the preview,
  applies final ranking/default selection, and clears all stale dry-run,
  confirmation, execution, and preview identity state.
- [ ] **AC8 (R9).** Automated accessibility assertions cover progress labelling,
  live-region behavior, disabled/absent preview selection, keyboard focus, and
  reduced motion; visual evidence covers 800×600 and 390×844 without page-level
  horizontal overflow.
- [ ] **AC9 (R10).** Core and TUI tests cover continuous cumulative snapshots,
  cancellation during target sizing, latest-job protection, preview-only blocking
  of TUI selection/dry-run/cleanup, and final parity; CLI text and JSON fixtures
  remain unchanged unless a separately documented compatibility need is found.
- [ ] **AC10 (R1–R10).** Generated TypeScript contracts are refreshed through the
  existing generator; desktop lint/typecheck/tests/build, relevant Rust tests, and
  `just ci` pass on the final tree.

## Out of Scope

- Numeric elapsed-time or percentage completion without a real bounded backend
  total.
- Selecting or dry-running targets before the completed Scan Report arrives.
- Changing cleanup rules, provider coverage, ranking policy, deletion authority,
  permanent-delete policy, Docker scope, or Cargo-home inspect-only behavior.
- Adding frontend production dependencies, a new styling framework, a virtualized
  table dependency, or a second scan implementation in desktop/CLI/TUI code.
- Redesigning dry-run, confirmation, execution, or final reporting beyond changes
  required to preserve their existing boundaries during the new scan flow.

## Deferred Items

- A future true-percentage design may introduce a two-pass or bounded-work-unit
  scanner, but it requires its own evidence and compatibility review.
- Native installed-app visual acceptance remains a separate manual gate from the
  controlled web fixture; browser/fixture evidence must not be reported as native
  IPC proof.
