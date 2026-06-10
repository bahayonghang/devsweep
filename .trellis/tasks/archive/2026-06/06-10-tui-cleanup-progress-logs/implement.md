# TUI cleanup progress and logs - Implementation Plan

## Preconditions

- Stay in planning until the user approves implementation.
- Before editing code in Phase 2, load `trellis-before-dev` and the relevant
  frontend/backend spec detail files.
- Preserve the unrelated untracked `devsweep-audit.jsonl` in the working tree.

## Implementation Checklist

1. Add focused failing tests for executor progress status.
   - Verify success, failure, and skipped targets emit typed status values.
   - Verify failed progress carries the failure message.
   - Verify existing audit JSONL records and `ExecutionReport` counts still
     match current behavior.

2. Extend executor progress payloads.
   - Add `ExecutionTargetStatus`.
   - Add `status` to `ExecutionProgress`.
   - Emit typed status from each `ActionStatus` branch.
   - Keep command/trash execution and audit writes unchanged.

3. Add cleanup progress list state in the TUI.
   - Initialize list items from `selected_cleanup_plan()` on accepted
     confirmation.
   - Store one item per selected target, keyed by `TargetId`.
   - Update item status/detail on `WorkerEvent::CleanProgress`.
   - Merge `ExecutionReport.failures` on `CleanFinished`.
   - Verify selected target order is stable and no row-index matching is used.

4. Introduce typed app log entries.
   - Replace string-only `LogEntry` with typed fields and a sequence counter.
   - Add small helper methods for scan, clean, target progress, final summary,
     audit path, and error logs.
   - Preserve bounded retention.
   - Keep display formatting inside render helpers.

5. Redesign cleanup progress rendering.
   - Center the summary line and progress bar.
   - Render the per-target result list with text status labels.
   - Cap visible list rows and show an overflow line for long selections.
   - Preserve running and finished footer/modal actions.
   - Keep render functions pure.

6. Update Jobs/Logs rendering.
   - Format typed app log entries in the Logs panel.
   - Keep job rows concise and readable.
   - Ensure mixed cleanup results are visible both in the modal and logs.

7. Run targeted tests during development.
   - `cargo test executor::tests::observed_execution_reports_per_target_progress`
   - TUI state tests for accepted confirmation and mixed clean results.
   - TUI render tests for centered progress and outcome list.

8. Run final verification.
   - `cargo fmt --all -- --check`
   - `cargo test --all-targets`
   - `cargo clippy --all-targets -- -D warnings`
   - `just ci`

## Review Gates

- Do not call `task.py start` until the user approves implementation.
- After implementation, review `git diff --stat` and confirm changes are
  limited to executor/TUI tests and any required task progress notes.
- If `just ci` fails, fix the cause before reporting completion.

## Risk And Rollback Points

- If typed executor progress causes broad test churn, keep the enum small and
  avoid changing `ExecutionReport` or audit JSONL.
- If the progress modal becomes too tall, keep the centered summary/bar and cap
  list rows rather than widening the modal or redesigning the full layout.
- If app log typing grows beyond recent UI events, stop and split persistent log
  storage into a separate task.
