# Fix TUI cleanup confirmation feedback - Design

## Boundary

This task is limited to `src/tui.rs` confirmation and progress state. It should
not change scanner output, cleanup execution semantics, cleanup plan
serialization, or provider behavior.

## Current Flow

1. Pressing `c` opens `Overlay::Confirm(ConfirmState)`.
2. The user types the required phrase and presses Enter.
3. `handle_confirm_key` accepts only exact phrase matches.
4. On mismatch, the app logs a message but renders no inline modal feedback.
5. On acceptance, the app clears the overlay, starts a clean job, and emits
   `Effect::StartClean`.
6. The clean worker sends `CleanProgress`, runs the executor, then sends
   `CleanFinished` or `JobFailed`.
7. The event loop drains all queued worker events before rendering.

## Proposed Flow

1. Extend confirmation state with optional inline feedback.
2. On invalid Enter, store an inline feedback message in `ConfirmState` and keep
   the confirmation modal open.
3. Update confirmation copy so the required phrase and Enter behavior are
   explicit.
4. On valid Enter, create the clean job and immediately set cleanup progress to
   `0 / total` before returning `Effect::StartClean`.
5. Keep cleanup progress visible after `CleanFinished` by turning the progress
   modal into a final result display instead of clearing it immediately.
6. Continue logging final audit details to Jobs/Logs.

## Data And Contracts

- `ConfirmState` remains TUI-only and may add a field for inline feedback.
- `CleanupProgress` remains TUI-only and may add a status/result field if it
  keeps the completed state visible.
- `WorkerEvent` and executor contracts do not need to change for this task.
- Command execution continues to use program and argv separately.

## Tradeoffs

Keeping the confirmation phrase preserves safety but requires clearer feedback.
The alternative, allowing Enter to execute without the phrase, would reduce
friction but conflicts with the existing irreversible-cleanup safety model and
the prior task acceptance criteria.

Keeping the final progress modal visible makes fast cleanups observable. The
tradeoff is that a completed modal remains on screen until the user dismisses it
or another overlay replaces it.

## Rollback

Rollback is local to TUI state/rendering and tests. Cleanup execution and plan
formats are unchanged, so no migration or data cleanup is required.
