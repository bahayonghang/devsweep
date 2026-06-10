# State Management

> How TUI state is managed in this project.

---

## Overview

Interactive TUI state lives in `src/tui.rs::App`. Use this single explicit app
state instead of global mutable state or widget-owned side effects.

---

## State Categories

- Domain state: `CleanupPlan`, `CleanTarget`, risk, evidence, and actions from
  `src/model.rs`.
- View state: selected row, active tab, filter text, modal state, and help
  visibility. These live in `App`.
- Worker state: scan/execution jobs, progress, cancellation, and logs. These
  enter the app through explicit `WorkerEvent` values.
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

Interactive default selection must use the shared path-safety helper to exclude
targets that contain the running `devsweep` executable. This prevents a target
hidden by the active tab, such as the current checkout's Rust `target/`, from
being executed by surprise when the TUI is launched with `just dev`.

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
- Do not rebuild default-selection filtering with local path-prefix logic in the
  TUI. Reuse the shared path-safety helper so UI selection and executor safety
  stay consistent.
