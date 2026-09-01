# Quality Guidelines

> Code quality standards for frontend development.

---

## Overview

Frontend quality for this project means terminal UI quality: render functions
must be pure, responsive, testable with ratatui `TestBackend`, and separated
from scanner/executor side effects.

---

## Forbidden Patterns

- Do not scan directories, execute commands, move files, or write audit logs
  from TUI render functions.
- Do not block redraws with large directory walks or size estimation.
- Do not print diagnostics to `stdout` from paths that can also emit JSON.
- Do not duplicate domain model fields in UI-only structs without a clear
  projection reason.
- Do not import render modules from app/reducer modules or runtime services from
  render modules. Shared presentation belongs in `tui/display.rs`.

---

## Required Patterns

- Keep render functions deterministic for a given state.
- Use ratatui `TestBackend` for render smoke tests.
- Display risk and action semantics from typed model fields, not from path-name
  guesses.
- Keep cleanup confirmation explicit once execution support is added.
- Keep `render_app` as the root render seam and pass immutable typed state to
  view modules. Rendering must not call `App::update` or worker/service APIs.

Current render test example:

```rust
let app = App::with_plan(representative_plan());
let backend = TestBackend::new(120, 32);
let mut terminal = Terminal::new(backend).expect("test terminal");

terminal
    .draw(|frame| render_app(frame, &app))
    .expect("representative app renders");
```

---

### Scenario: Scan health, inventory, and grouped cache presentation

#### 1. Scope / Trigger

- Trigger: rendering scan or inventory health, capacity totals, sizing warnings,
  diagnostics, inspect-only findings, or high-volume cache groups in the TUI.

#### 2. Signatures

- Shared inputs: `ScanHealth`, `ScanTotals`, `ScanDiagnostic`, and
  `CleanTarget.sizing_warnings` from `devsweep_core::model`.
- Inventory input: `InventoryReport { observations, health, orphan_pnpm_store }`
  from `devsweep_core::inventory`.
- TUI boundary: scan and inventory workers deliver typed results to
  `App::update`; render helpers consume the resulting `App` state only.

#### 3. Contracts

- Render verified bytes separately from partial lower bounds and unknown-target
  counts. Do not label lower-bound bytes as reclaimable verified capacity.
- Render diagnostics and sizing warnings from their typed fields; no widget may
  reparse JSON or recreate scanner logic.
- `__pycache__` grouping is a display projection only. Exact target IDs,
  selection state, actions, and cleanup confirmation still refer to the
  underlying `CleanupPlan` targets.
- Inventory is a separate display-only projection. It may own an observation
  cursor and scroll position, but it must never populate `App.targets`,
  `selected_ids`, a cleanup confirmation, or an executor request. Its refresh
  worker accepts cancellation and stores only an `InventoryReport` snapshot.
- An inspect-only pnpm finding renders its candidate store, configured store,
  and observed project-reference evidence. It does not become a selection or
  cleanup action.

#### 4. Validation & Error Matrix

- Partial health -> the header/details expose partial state and diagnostics.
- Empty or hidden warnings -> details remain valid and do not shift selection.
- Collapsed group -> child rows hide visually but retain their selected IDs.
- Narrow terminal -> health and primary actions remain readable without relying
  on color alone.
- Inventory refresh while cleanup is active -> no inventory worker starts and
  the existing cleanup selection stays unchanged.
- Canceled inventory -> the worker reports a terminal canceled job; it does not
  replace the prior inventory snapshot.

#### 5. Good/Base/Bad Cases

- Good: expanding a Python cache group changes only presentation, not selected
  bytes or target identity.
- Good: browsing an inventory observation changes only the inventory cursor;
  space or cleanup confirmation cannot select that observation.
- Base: a complete scan with no diagnostics renders zero partial/unknown totals.
- Base: an inventory report with no observations renders a stable empty view.
- Bad: summing partial lower bounds into the verified total or deleting grouped
  paths from render code.
- Bad: converting a capacity observation or pnpm finding into a `CleanTarget`
  just to reuse target-list selection widgets.

#### 6. Tests Required

- State tests cover health propagation and group collapse/expansion while
  preserving selection.
- `TestBackend` tests cover partial health, typed diagnostic/warning details,
  grouped rows, and narrow layouts.
- Inventory state tests assert worker results retain targets and `selected_ids`
  unchanged; runtime tests assert the worker emits only inventory events;
  `TestBackend` tests assert configured pnpm and project-reference evidence
  remain visible.

#### 7. Wrong vs Correct

Wrong:

```rust
// A display group cannot own a second cleanup selection schema.
group.selected = true;
```

Correct:

```rust
// The group projects existing target IDs back to App-owned selection state.
let selected = group.target_ids.iter().all(|id| app.is_selected(id));
```

Wrong:

```rust
// An inventory observation is not cleanup authority.
app.targets.push(observation.into_clean_target());
```

Correct:

```rust
// Keep the read-only report outside the cleanup-plan state.
app.inventory_report = Some(report);
```

---

## Testing Requirements

- Every new view should have at least a smoke render test with `TestBackend`.
- State update logic should have unit tests independent of terminal rendering.
- Worker/event code should test cancellation and failure events before cleanup
  execution is connected.
- Confirmation behavior must have tests for both trash-backed and irreversible
  command-backed selections.
- Cross-module reducer tests live in `app/tests.rs`, fake-service runtime tests
  in `runtime/tests.rs`, and observable full/degraded/narrow/modal render tests
  in `render/tests.rs`. Moving code must preserve named behavior coverage and a
  nonzero focused test count.

---

## Code Review Checklist

- Is render code free of filesystem, process, and cleanup side effects?
- Does the view consume typed model/state instead of raw JSON?
- Does the UI remain usable without relying on color alone?
- Are new render paths covered by `TestBackend` or state-level tests?
