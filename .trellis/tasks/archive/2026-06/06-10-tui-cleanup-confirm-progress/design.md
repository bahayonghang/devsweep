# Improve TUI cleanup confirmation and progress - Design

## Boundary

This task is limited to the TUI cleanup confirmation and execution feedback
path. It may touch `src/tui.rs` and narrowly scoped executor observation
support in `src/executor.rs`. It should not change scanner output, cleanup plan
serialization, provider rules, or the CLI cleanup contract.

## Current Flow

1. The user selects cleanup targets in the TUI.
2. Pressing `c` opens `Overlay::Confirm(ConfirmState)`.
3. The user types the required phrase and presses Enter.
4. The app emits `Effect::StartClean`.
5. The clean worker sends one `CleanProgress` message, runs
   `Executor::run_plan`, then sends `CleanFinished` or `JobFailed`.

## Proposed Flow

1. Build confirmation state from selected targets.
2. Add command preview data for selected `CleanAction::Command` targets.
   Previews should be explicitly display-only and derived from stored argv.
3. Render confirmation copy with:
   - cleanup strength
   - target count and estimated bytes
   - command-backed cleanup preview when present
   - required phrase and current input
   - key hints for confirm and cancel
4. Let the clean worker receive per-target progress callbacks from the executor
   and forward them as worker events.
5. Store cleanup progress as numeric state in the active job record.
6. Render progress with a ratatui gauge or equivalent stable-width progress
   line, plus current target/message text. Show this progress both in a
   transient cleanup-progress modal and in the existing Jobs/Logs panel.

## Data And Contracts

- Command execution must continue to use `CommandRequest { program, args, cwd }`.
- Command preview should not become a shell command string used for execution.
- Progress should use selected targets as the denominator.
- Skipped inspect-only targets count as completed progress when the executor
  finishes their per-target result.
- Existing `ExecutionReport` fields remain unchanged for CLI compatibility.

## Implementation Shape

- Extend `ConfirmState` with a small command-preview collection.
- Add a helper that formats selected command-backed targets for display.
- Extend worker progress events to carry optional numeric progress.
- Add a minimal executor observation hook, for example a `run_plan_observed`
  method or callback parameter used by TUI while keeping `run_plan` as the CLI
  entry point.
- Update render tests to assert command preview, key hints, and progress state.

## Tradeoffs

Rendering progress on both surfaces is the most discoverable option because the
user sees immediate feedback after confirming while Jobs/Logs remains the
durable job history. The tradeoff is a slightly larger TUI state model.

## Rollback

The change should be easy to roll back by removing the new TUI state/rendering
and executor observer hook. Cleanup execution and plan serialization should not
need migration because they are not part of the behavioral contract change.
