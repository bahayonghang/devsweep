# Current Scan Flow And Planning Evidence

## Scope And Evidence Boundary

This note records current repository behavior for the desktop scan-progress
planning task. It does not authorize implementation. The supplied screenshot is
visual evidence of the current active-scan state; code and tests are the source
of truth for contracts and safety boundaries.

## Current Cross-Layer Flow

```text
                              non-interactive CLI
                              discards every progress callback
                             /
Sweeper::ScanProgress -------+-- TUI WorkerEvent::ScanProgress
  phase + message            |     keeps internal partial CleanupPlan
  optional CleanupPlan       |     -> job-scoped ScanSnapshot -> targets/categories
                             \
                              Tauri progress_for_ipc
                                forces partial = None
                                -> strict TS decoder accepts only null
                                -> reducer stores phase/message only
                                -> ReviewPage sees scan = null
                                -> completed empty-state remains visible
```

## Confirmed Findings

### Core scan contract

- `ScanProgress` contains only `phase`, a human message, and an optional cumulative
  internal `CleanupPlan`; it has no total-work or percentage field
  (`crates/devsweep-core/src/scan/mod.rs:109-128`).
- The pipeline currently emits `partial: None` at phase starts and a cumulative,
  ranked partial plan only after the project phase and after the global phase
  finish (`crates/devsweep-core/src/scan/mod.rs:221-274`).
- Project scanning accumulates targets through recursive traversal and returns the
  whole outcome; its entry point has cancellation but no incremental target/progress
  sink (`crates/devsweep-core/src/scan/project/mod.rs:85-146`).
- Global scanning calls provider builders in a fixed sequence and returns one plan;
  it likewise has no per-provider progress sink
  (`crates/devsweep-core/src/scan/global/mod.rs:34-58`).
- The persisted/report-facing conversion deliberately strips direct executable
  actions and produces an `UntrustedPlan`, but the conversion is currently
  crate-private (`crates/devsweep-core/src/plan/mod.rs:209-229`).

### CLI and TUI behavior

- The non-interactive `scan` CLI supplies a no-op progress callback and prints or
  serializes only the final report
  (`crates/devsweep-cli/src/application/commands.rs:17-35`).
- The TUI forwards the core partial plan through a job-scoped worker event
  (`crates/devsweep-cli/src/tui/runtime/workers.rs:22-65`).
- The TUI accepts partials only for the latest active scan, stages them as partial
  health, replaces visible targets, preserves valid selection overrides, and
  invalidates confirmation state
  (`crates/devsweep-cli/src/tui/app/worker.rs:262-377`).
- The TUI's existing category vocabulary is `Scope` (Global/Projects) plus
  `Ecosystem` (Rust/Node/Python/Generic); target kind remains row-level detail
  (`crates/devsweep-cli/src/tui/render/targets.rs:25-40`).

### Desktop boundary and UI behavior

- Tauri currently clears `ScanProgress.partial` before emitting `scan://progress`
  because the internal `CleanupPlan` contains trusted action details
  (`desktop/src-tauri/src/scan.rs:12-18`).
- `scan_start` forwards that redacted event from the blocking scan worker and
  returns the final `ScanReport` separately
  (`desktop/src-tauri/src/commands.rs:46-67`).
- The generated TypeScript contract fixes `ScanProgress.partial` to `null`, and
  the runtime decoder rejects any non-null value
  (`desktop/src/api/types.gen.ts:236-240`,
  `desktop/src/api/contract.ts:212-217`).
- The reducer stores only the latest progress event while `phase === "scanning"`;
  it does not maintain a preview plan. `scan_requested` also clears the previous
  report (`desktop/src/state/app-state.ts:4-17`,
  `desktop/src/state/app-state.ts:53-67`).
- `App` continues rendering `ReviewPage` during scanning. Because `state.scan` is
  null, `ReviewPage` renders completed-empty-state copy, producing the screenshot's
  contradictory `No scan results` state
  (`desktop/src/App.tsx:62-71`, `desktop/src/pages/ReviewPage.tsx:6-9`).
- `ScanPage` currently offers only a spinner plus phase/message text; its reduced-
  motion fallback stops animation but does not add structural progress
  (`desktop/src/pages/ScanPage.tsx:10-20`, `desktop/src/styles.css:31-36`,
  `desktop/src/styles.css:111`).

### Prior decisions that this task must reopen explicitly

- The archived desktop MVP deliberately required indeterminate progress because
  the core contract had no total and prohibited fabricated percentages
  (`.trellis/tasks/archive/2026-08/08-03-tauri-desktop-app/design.md:90-98`).
- The same design deliberately redacted the internal partial plan before the
  webview boundary. This task may replace the `null`-only projection with a safe,
  display-only DTO, but must not regress the underlying authority boundary
  (`.trellis/tasks/archive/2026-08/08-03-tauri-desktop-app/design.md:92-97`).

## UX Diagnosis

1. **State contradiction:** active-scan status and completed-empty-state copy are
   visible simultaneously.
2. **Weak progress hierarchy:** scope toggles, one spinner, a long backend sentence,
   and Cancel share one toolbar row; there is no persistent phase structure or
   accumulated evidence summary.
3. **Lost earned value:** when project scanning has finished and global sizing is
   running, the core already has ranked project targets, but the desktop discards
   them.
4. **Classification underuse:** the table exposes kind/ecosystem/scope per row but
   does not provide scan-time category summaries or grouping.
5. **Narrow-layout pressure:** the 1,040 px minimum table is scrollable, while the
   active toolbar collapses to two grid columns. A live-results design needs a
   compact category summary and must not add card grids or another horizontal
   control row.

## Selected Contract Shape

Keep two distinct concepts:

- **Scan progress:** correlated lifecycle information (`scan_id`, phase, exact
  backend message, and determinate work only where the backend has a real bounded
  total). Its visual bar may be indeterminate.
- **Scan preview:** a cumulative, read-only projection of discovered target facts.
  It must omit direct cleanup actions, remain separate from the final `ScanReport`,
  and never populate the reducer field used by dry-run or execution.

The projection owner is core, not React. Use a purpose-built observation DTO that
omits cleanup intent and default selection as well as trusted actions; do not reuse
a submit-ready `UntrustedPlan`, reconstruct intents, or strip trusted fields ad hoc
in Tauri/TypeScript.

For presentation, group primarily by scan scope (`Projects`, `Global caches`),
show ecosystem counts as compact filters/summary, and retain target kind in the
dense table. During scanning, rows should be visibly labelled `Discovered so far`,
selection controls and the action bar should be absent or disabled, and final
ranking/default selection should apply only when the completed report arrives.

## Product Options

| Option | Contract | Benefit | Cost / limitation |
| --- | --- | --- | --- |
| Continuous safe preview | Indeterminate phase bar plus bounded, cumulative read-only target snapshots | Closest to the requested live experience; no invented percentage | Requires progress sinks below `Sweeper` and event coalescing/correlation |
| Phase checkpoint preview | Existing project/global completion snapshots converted to a safe preview | Smallest complete change; immediately fixes the screenshot during global scanning | Project scanning can remain visually silent for a long time |
| True numeric percentage | New discovered/total work-unit contract plus preview stream | Familiar determinate bar where totals are real | Discovery total is unknowable without a larger two-pass/refactored scanner; work units do not predict elapsed time |

## Risks And Follow-Up Evidence

- A continuous stream needs a bounded emission policy (semantic checkpoint and/or
  time coalescing) so recursive scans do not flood Tauri/React or repeatedly
  re-render a large table.
- Desktop progress events currently have no scan/job identifier. The reducer only
  checks the broad `scanning` phase, so a new correlated `scan_id` should prevent
  late events from one scan being accepted by a later scan. The selected design
  carries the stream over a command-scoped ordered Tauri channel with monotonic
  sequence numbers rather than extending the application-global event bus.
- Global cancellation is observed by provider probes, but the global scanner
  returns only a plan and the pipeline has no post-global cancellation health
  check (`crates/devsweep-core/src/scan/mod.rs:250-274`). Truthful canceled/partial
  final-state behavior needs an explicit regression before implementation claims.
- Cumulative snapshots retain the core's deterministic shared ranking, so later
  larger targets may move earlier rows. Stable target ids, replacement semantics,
  and no insertion animation mitigate visual churn; consumers must not invent a
  separate append-only order.
