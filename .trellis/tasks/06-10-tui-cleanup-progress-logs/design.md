# TUI cleanup progress and logs - Design

## Scope

This task improves the cleanup execution feedback path from executor progress
events to TUI state, progress-modal rendering, and Jobs/Logs projection. It must
not change cleanup discovery, selection semantics, command execution ownership,
permanent-delete behavior, or the existing audit JSONL contract.

## Current State

The existing execution path is:

```text
selected targets -> Effect::StartClean -> run_clean_worker
  -> Executor::run_plan_with_progress -> WorkerEvent::CleanProgress
  -> App::handle_worker_event -> CleanupProgress/message + string log
  -> render_cleanup_progress / render_jobs_logs
```

The executor already has the core event boundary needed for this feature:
`ExecutionProgress` includes the completed count, total count, target id, and a
message for each selected target. The weakness is that the message is a
human-formatted string and does not carry a typed status. The TUI then formats
that string again, stores only the latest cleanup message, and logs plain
strings.

The audit JSONL writer already owns durable per-target action records. It opens
the audit file before target execution, appends one line per target, and records
success, skipped, and failed statuses. That remains the durable history layer.

## Data Flow

Planned flow:

```text
selected cleanup plan
  -> App initializes CleanupProgress { items: Pending per target }
  -> run_clean_worker executes selected targets
  -> Executor emits typed ExecutionProgress { target_id, status, message }
  -> WorkerEvent::CleanProgress carries typed status
  -> App updates the matching item and appends typed AppLogEntry
  -> CleanFinished merges final failure messages and audit path
  -> render_cleanup_progress shows centered summary/bar + result list
  -> render_jobs_logs shows jobs + structured log projection
```

## Executor Contract

Add a small public status enum beside `ExecutionProgress`:

```rust
pub enum ExecutionTargetStatus {
    Succeeded,
    Failed,
    Skipped,
}
```

Extend `ExecutionProgress` with `status: ExecutionTargetStatus`. Keep
`message` as the display/detail string:

- success: `completed`
- skipped: skip reason
- failed: error string, not just `failed`

This prevents the TUI from parsing display text and keeps the executor as the
owner of action outcome classification. It does not serialize a new file format.

## TUI State

Replace the single-message cleanup progress state with a list projection:

```rust
struct CleanupProgress {
    job_id: JobId,
    completed: usize,
    total: usize,
    summary: String,
    finished: bool,
    items: Vec<CleanupProgressItem>,
}

struct CleanupProgressItem {
    target_id: TargetId,
    label: String,
    status: CleanupItemStatus,
    detail: Option<String>,
}

enum CleanupItemStatus {
    Pending,
    Succeeded,
    Failed,
    Skipped,
}
```

The list is initialized from the selected cleanup plan before worker events
arrive. Updates match by `TargetId`, not row index. Labels should use existing
display helpers such as `target_title`, `compact_target_id`, and `display_path`
so Windows verbatim prefix cleanup stays UI-only.

On `CleanFinished`, preserve the modal and merge `ExecutionReport.failures` into
failed rows so the final list can show detailed failure messages even if the
progress callback has already rendered a short message.

## Log System

Use three separate logging concepts and keep their ownership clear:

1. `tracing` diagnostics: process diagnostics to stderr, owned by
   `src/config.rs`.
2. Execution audit JSONL: durable per-target cleanup action record, owned by
   `src/executor.rs`.
3. TUI app logs: in-memory UI projection for recent scan/clean/job events,
   owned by `src/tui.rs::App`.

For this task, implement only the third concept as a typed in-memory structure:

```rust
struct AppLogEntry {
    seq: u64,
    level: AppLogLevel,
    source: AppLogSource,
    job_id: Option<JobId>,
    target_id: Option<TargetId>,
    message: String,
}
```

Keep the current bounded retention behavior, but make the cap apply to typed
entries. This keeps the TUI useful without introducing persistence, retention
policy, privacy questions, or migrations.

## Rendering

`render_cleanup_progress` should become specialized instead of relying only on
the generic modal helper:

- Use the same centered modal frame.
- Render the summary line and progress bar centered.
- Render the target result list below the centered summary.
- Use text labels such as `PENDING`, `OK`, `FAILED`, and `SKIPPED` so status is
  readable without color.
- Truncate or cap visible rows in small terminals and show an overflow line when
  necessary.
- Preserve close/cancel hints: running cleanup shows `x cancel`; finished
  cleanup shows `Enter/Esc close`.

The Jobs/Logs tab can keep the current two-panel layout, but log rows should be
formatted from typed fields instead of storing preformatted strings as the only
source of truth.

## Compatibility

- Existing audit JSONL remains append-only and unchanged.
- Existing `ExecutionReport` fields remain available for CLI behavior.
- CLI `clean` output is unchanged.
- Tests that construct `ExecutionProgress`, `WorkerEvent::CleanProgress`, or
  `CleanupProgress` need mechanical updates for the new typed fields.

## Tradeoffs

Adding typed progress status is slightly more code than keeping text messages,
but it avoids fragile string parsing at the UI boundary and matches the
cross-layer guideline for event/payload ownership.

Keeping app logs in memory is intentionally smaller than a persistent log
database. The tradeoff is that TUI logs disappear when the process exits, while
cleanup action history remains available through the explicit audit JSONL file.

## Rollback

Rollback is local to `src/executor.rs` progress payloads and `src/tui.rs` state
and rendering. Since no persisted schema changes are planned, rollback should
not require data migration or cleanup.
