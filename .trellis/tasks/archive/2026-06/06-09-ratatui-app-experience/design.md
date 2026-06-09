# Ratatui app experience design

## Scope

This child task owns the interactive terminal UI layer for the existing
cleanup-plan model, project scanner, global providers, and executor boundaries.
It should not change scanner discovery semantics, executor safety semantics, or
provider command discovery unless the TUI reveals a contract bug.

## Current Code Boundary

- `src/tui.rs` currently renders a placeholder and is the primary target for
  this task.
- `src/model.rs` already defines the cleanup-plan contract used by scanner,
  provider, executor, CLI, and TUI.
- `src/scanner.rs` and `src/providers.rs` produce `CleanupPlan` values.
- `src/executor.rs` consumes selected targets through `ExecutionRequest`.

## Architecture

Use a TEA-style boundary inside the TUI module:

- Model: `App` owns active tab, selected target IDs, cursor position, filters,
  overlay state, jobs, logs, and quit flag.
- Update: `App::update` consumes key and worker events, mutates state, and
  returns side-effect requests such as start scan, start clean, cancel job, or
  quit.
- View: `render_app` and helper render functions read `App` only and draw
  widgets. View code must not scan directories, run commands, move files, or
  walk trees.
- Effects: the terminal event loop interprets update effects and runs scanner /
  executor work outside render functions.

Keep the first implementation in `src/tui.rs` unless the file becomes hard to
review. Splitting into `src/tui/*` is allowed only if it removes real complexity.

## UI Structure

The TUI should expose these surfaces:

- Dashboard tab for all targets and summary totals.
- Global tab filtered to global provider targets.
- Projects tab filtered to project targets.
- Rules tab as a read-only MVP view of built-in rule/provider coverage.
- Jobs/Logs tab showing scan/clean job state and recent log records.
- Details panel showing path, scope, risk, action, reversibility, and evidence
  for the selected target.
- Confirmation overlay that uses weaker copy for trash-backed cleanup and
  stronger typed confirmation for irreversible command-backed cleanup.
- Keyboard help overlay.

## Data Flow

1. `App::new` starts with an empty plan.
2. `s` queues a scan job effect; the event loop runs project scan for the
   current directory and global provider discovery outside render code.
3. Worker scan completion replaces app targets and restores default selections
   from `selected_by_default`.
4. Selection state is stored in the TUI as `TargetId`s and converted back into a
   selected `CleanupPlan` only when starting a clean job.
5. `c` opens confirmation only when at least one target is selected.
6. Clean confirmation queues an executor job effect; executor results are
   reported back as worker events and appended to jobs/logs.
7. Cancellation is represented as a job state transition. Existing scanner and
   executor internals are not interrupted unless they later expose cancellation
   hooks.

## Safety And Compatibility

- Default TUI state must not execute cleanup.
- Dry-run preview is display-only.
- Render tests should use `ratatui::backend::TestBackend`.
- Command-backed and trash-backed confirmation copy must differ.
- Permanent delete remains disabled by the existing executor contract.
- Docker should not be added as MVP functionality in this task.

## Trade-Offs

- A single-file TUI keeps this child scoped, but the model/update/view sections
  must remain visibly separated.
- Cancellation is initially a UI/job-state request because existing scanner and
  executor APIs are synchronous.
- Totals may be derived from the in-memory target list during render; no
  filesystem size estimation may happen in view code.
