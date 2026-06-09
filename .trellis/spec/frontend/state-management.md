# State Management

> How TUI state is managed in this project.

---

## Overview

There is no interactive TUI state yet. The only current stateful behavior is the
serializable cleanup plan in `src/model.rs` and the placeholder draw call in
`src/tui.rs`.

When interactive TUI work begins, use a single explicit app state instead of
global mutable state or widget-owned side effects.

---

## State Categories

- Domain state: `CleanupPlan`, `CleanTarget`, risk, evidence, and actions from
  `src/model.rs`.
- View state: selected row, active tab, filter text, modal state, and help
  visibility. These should live in a future `AppState`.
- Worker state: scan/execution jobs, progress, cancellation, and logs. These
  should enter the app through explicit worker events.

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

---

## Server State

There is no server state. Filesystem scan results are local snapshots and should
be treated as stale after a new scan or cleanup job.

---

## Common Mistakes

- Do not let individual widgets own cleanup selection independently.
- Do not mutate `CleanupPlan` in render code.
- Do not keep a cached selected total without a clear invalidation path.
