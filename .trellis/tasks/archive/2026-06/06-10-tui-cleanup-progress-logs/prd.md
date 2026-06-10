# TUI cleanup progress and logs

## Goal

Make cleanup execution feedback actionable in the TUI. The cleanup progress
modal should center the progress summary/bar and show a per-target result list
so users can immediately see which targets succeeded, failed, or were skipped.
The task must also define a small log system for job and cleanup events without
weakening the existing cleanup safety model.

## User Value

When cleanup finishes with mixed results, the user should not have to infer what
happened from a single summary line or leave the modal to inspect a raw audit
file. The TUI should show the outcome of each selected target and keep related
job/log records coherent enough for follow-up debugging.

## Confirmed Facts

- The screenshot shows the `Cleanup progress` modal after 3 selected targets:
  1 succeeded, 1 failed, and 1 skipped. The modal currently shows a summary and
  a progress bar, but no target-level outcome list.
- `src/tui.rs::CleanupProgress` currently stores only `job_id`, `completed`,
  `total`, a single `message`, and `finished`.
- `src/tui.rs::render_cleanup_progress` renders `Progress: ...`, the bar, and
  `Current: ...` as generic left-aligned modal body lines.
- `src/executor.rs::run_plan_with_progress` already emits one progress callback
  per selected target with `completed`, `total`, `target_id`, and a text
  `message`.
- `src/executor.rs` already writes durable per-target audit JSONL records for
  success, skipped, and failed actions. That audit file is the existing durable
  execution record.
- `src/tui.rs` has a Jobs/Logs tab, but app logs are currently untyped strings
  capped at 200 entries.
- Project guidelines require TUI render code to stay pure, executor cleanup
  commands to keep program and argv separate, and diagnostics to avoid stdout
  pollution for JSON-producing commands.

## Requirements

1. Center the cleanup progress summary and progress bar within the progress
   modal content area.
2. Show a per-target cleanup result list in the progress modal.
   - Pending targets should be distinguishable before their result arrives.
   - Successful targets should be labeled as succeeded.
   - Failed targets should be labeled as failed and include the failure message
     once available.
   - Skipped targets should be labeled as skipped and include the skip reason
     once available.
3. Preserve final cleanup progress after `CleanFinished` until the user closes
   it, matching the existing observable-finish behavior.
4. Keep cleanup safety boundaries unchanged.
   - Do not change scanner target discovery.
   - Do not change selected cleanup plan semantics.
   - Do not change command/trash execution ownership.
   - Do not enable permanent delete.
5. Design the log system as typed application log entries in `App`, with fields
   sufficient for rendering and filtering by job/target later:
   - monotonic sequence number
   - level or status
   - source/category
   - optional job id
   - optional target id
   - display message
6. Keep durable execution history in the existing audit JSONL owner. Do not add
   a database or hidden persistent history in this task.
7. Update the Jobs/Logs tab to render the structured app log projection rather
   than relying on raw string-only log records.
8. Keep render code deterministic and side-effect free.
9. Cover state transitions and render behavior with focused tests.

## Acceptance Criteria

- [x] Cleanup progress render tests prove the summary/bar are centered and the
      modal lists target outcomes with text labels, not color alone.
- [x] On accepted cleanup confirmation, progress state is initialized with one
      pending list item per selected target in selected-plan order.
- [x] Per-target progress events update the matching list item without parsing
      human-formatted strings.
- [x] A mixed cleanup result can show succeeded, failed, and skipped targets in
      the progress modal.
- [x] Failure messages from the execution report are surfaced on failed target
      rows after `CleanFinished`.
- [x] The final progress modal remains dismissible with Enter/Esc and still
      advertises logs access in the footer where applicable.
- [x] Jobs/Logs renders structured app log entries for scan, clean, progress,
      final summary, audit path, and errors.
- [x] Existing audit JSONL fields and execution behavior remain compatible.
- [x] No new filesystem mutation, process execution, or audit writes happen from
      render functions.
- [x] `just ci` passes before implementation is reported complete.

## Out Of Scope

- New cleanup providers or scan rules.
- Cleanup plan JSON schema changes.
- Audit JSONL schema changes unless a small internal test-only visibility
  change is needed for assertions.
- Persistent application history beyond the existing explicit audit JSONL file.
- Real cancellation of an already-running cleanup command.
- Redesigning the whole TUI information architecture.

## Resolved Decisions

- Log-system scope: use typed in-memory TUI logs plus the existing execution
  audit JSONL as durable history. A separate persistent app log browser is out
  of scope for this task.

## Notes

- This is a complex task because it crosses executor progress events, TUI app
  state, rendering, and logs.
