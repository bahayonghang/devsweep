# State Management

> How TUI state is managed in this project.

---

## Overview

Interactive TUI state lives in `crates/devsweep-cli/src/tui/app/mod.rs::App`. Use this single
explicit app state instead of global mutable state, per-view stores, or
widget-owned side effects. Cohesive child modules may implement transitions,
but `App::update(UiEvent) -> Vec<Effect>` remains the root reducer.

---

## State Categories

- Domain state: `CleanupPlan`, `CleanTarget`, risk, evidence, and actions from
  `devsweep_core::model`.
- View state: selected row, active tab, filter text, modal state, and help
  visibility. These live in `App`.
- Worker state: scan/execution jobs, progress, cancellation, and logs. These
  enter the app through explicit `WorkerEvent` values. Runtime-owned channels,
  cancellation tokens, and worker permits are not duplicated in `App`.
- App logs: recent TUI logs are typed in-memory records owned by `App`. They may
  include level, source, job id, target id, and display message fields. Durable
  cleanup action history remains the executor-owned audit JSONL contract, not a
  hidden TUI database.

---

## When To Use Global State

Do not use global mutable state. Promote state to the app model when more than
one view needs it or when it must survive redraws.

---

## Derived State

Derived totals such as selected bytes should be computed from target selection
state and `CleanTarget.estimated_bytes`. Do not store a second independent total
unless performance proves it necessary, and then keep tests that verify it stays
in sync.

Selection state should store `TargetId` values, not row numbers. The row index is
only a cursor into the current filtered view and must be clamped or validated
before use.

Interactive default selection is a pure projection of `selected_by_default` for
executable targets. The TUI must not re-inspect paths or duplicate live safety
policy while building that projection. `execution::SafetyPolicy` remains the
final authority immediately before a side effect, including the self-executable
containment check.

Opening cleanup confirmation freezes an immutable validated manifest and its
digest. Selection or scan changes must not reconstruct or silently update that
manifest. Current scan updates invalidate the confirmation, stale worker events
are rejected by job identity/state, and inventory observations never enter the
cleanup target selection.

---

## Server State

There is no server state. Filesystem scan results are local snapshots and should
be treated as stale after a new scan or cleanup job.

---

## Common Mistakes

- Do not let individual widgets own cleanup selection independently.
- Do not mutate `CleanupPlan` in render code.
- Do not keep a cached selected total without a clear invalidation path.
- Do not let filter changes leave the selected row pointing past the visible
  target list.
- Do not add local path-prefix or current-executable checks to default
  selection. Preserve the scan/ranking hint and rely on execution-time live
  revalidation for cleanup authority.
- Do not store worker threads, senders, cancellation flags, or clean-worker
  single-flight state in `App`; those belong to `runtime/mod.rs`.
- Do not let view modules mutate state or maintain a second confirmation,
  selection, progress, or inventory snapshot.
