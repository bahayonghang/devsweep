# Decompose TUI internals

## Goal

Make the TUI implementation navigable and locally testable by splitting reducer
concerns, worker runtime, view rendering, and shared display formatting into
private cohesive modules while preserving one `App` state owner and all current
interaction behavior.

## Parent And Ordering

- Parent: `08-02-reorganize-rust-source-architecture`.
- Depends on completed children `08-03-untangle-core-contracts-rules` and
  `08-03-modularize-backend-runtime-discovery` so backend imports are stable.
- Must pass `just ci` and be archived before the final entrypoint/docs child.

## Confirmed Problems

- `App` owns target and inventory state, selection projections, filters,
  overlays, jobs, logs, cleanup progress, scan snapshots, and quit coordination
  in one 2,012-line production implementation (`src/tui/app.rs:33`).
- One reducer implementation handles keyboard, overlay, confirmation, worker,
  selection, scan staging, jobs, logs, and quit transitions
  (`src/tui/app.rs:110` through `src/tui/app.rs:1534`).
- `render.rs` has more than eighty production functions spanning all views,
  modals, layouts, styles, sanitizers, and formatters.
- App state imports display helpers from the render module
  (`src/tui/app.rs:18`), reversing the intended update-to-render dependency.
- `runtime.rs` combines production/fake service adapters, the event loop,
  effect dispatch, worker lifecycle, cancellation registry, and worker tests.

## Requirements

- Keep exactly one `App` state owner and one root reducer interface:
  `App::update(UiEvent) -> Vec<Effect>`.
- Split input/overlay handling, target and inventory selection projections,
  worker-result transitions, job/log/progress handling, and confirmation
  snapshots into private app implementation modules.
- Keep `UiEvent`, `Effect`, and `WorkerEvent` typed and explicit; do not replace
  them with callbacks, strings, or per-view state stores.
- Split runtime service adapters, event/effect dispatch, and worker functions
  where each resulting module owns meaningful behavior.
- Split rendering by cohesive views and overlays while keeping `render_app` as
  the root render interface.
- Move pure display text/path/action/command formatting shared by app state and
  render code into a neutral private TUI display module. App code must not
  import render modules.
- Keep render code deterministic and free of filesystem scans, process calls,
  execution, state mutation, and expensive size work.
- Preserve all key bindings, tabs, filters, selection/default overrides,
  pycache grouping, inventory isolation, confirmation freezing/digest checks,
  jobs/logs, cancellation, quit coordination, tombstones, progress, narrow
  layout, and display sanitization behavior.
- Relocate tests to the module/interface that owns the behavior; retain
  `TestBackend` coverage for complete, degraded, narrow, modal, and progress
  states.
- Update affected frontend Trellis specs to describe the final TUI tree and
  remove the now-obsolete instruction not to split `render.rs`.

## Acceptance Criteria

- [ ] `tui::run()` remains the TUI composition seam and no additional crate
      public TUI interface is introduced.
- [ ] `App` remains the only mutable UI state owner; derived selection/capacity
      values are not duplicated as independently mutable state.
- [ ] App/reducer modules do not import render modules, scanner/provider
      implementations, or `Executor`; they emit typed effects instead.
- [ ] Runtime remains the only owner of threads, channels, cancellation tokens,
      clean-worker single-flight, and effect dispatch.
- [ ] Render modules consume immutable typed state only and do not mutate
      cleanup-plan values or perform side effects.
- [ ] Frozen confirmation manifests, digest revalidation, stale worker-event
      rejection, legal job transitions, and cancellation truthfulness remain
      covered and passing.
- [ ] Inventory observations remain display-only and cannot enter target
      selection or execution.
- [ ] Existing full/degraded/narrow `TestBackend` render tests and display
      hygiene tests pass from the new module owners.
- [ ] Frontend directory/state/component guidance matches the implemented tree.
- [ ] `just ci` passes.

## Out Of Scope

- Visual redesign, new views or key bindings, copy changes, new state-management
  framework, generic widget framework, performance changes, and backend
  behavior changes.
- Final `main.rs`/`lib.rs` narrowing and repository-wide architecture map audit.
