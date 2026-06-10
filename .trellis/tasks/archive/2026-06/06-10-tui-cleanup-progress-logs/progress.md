# TUI cleanup progress and logs - Progress

## 2026-06-10

- Activated the Trellis task after user approval and kept implementation in
  inline mode.
- Added typed `ExecutionTargetStatus` values to executor progress callbacks so
  the TUI no longer parses outcome strings to classify target results.
- Kept execution audit JSONL compatibility intact; no audit schema or cleanup
  execution ownership changes were made.
- Reworked TUI cleanup progress state from a single latest message to
  per-target result items keyed by `TargetId`.
- Initialized pending progress rows immediately after accepted confirmation.
- Updated worker progress handling to show `OK`, `FAILED`, `SKIPPED`, and
  `PENDING` result labels with final failure details merged from
  `ExecutionReport.failures`.
- Specialized the cleanup progress modal so the summary and progress bar are
  centered and the result list is shown below them.
- Replaced string-only TUI log entries with typed in-memory app logs containing
  sequence, level, source, job id, optional target id, and message fields.
- Updated Jobs/Logs rendering to format the structured app log projection while
  leaving durable cleanup history in the executor-owned audit JSONL file.
- Captured the typed in-memory app log convention in
  `.trellis/spec/frontend/state-management.md`.

## Verification

- `cargo test executor::tests::observed_execution` passed.
- `cargo test tui::tests::cleanup_progress` passed after adjusting the log
  projection test to account for the progress overlay.
- `cargo test tui::tests` passed.
- `cargo fmt --all -- --check` initially reported formatting differences; ran
  `cargo fmt --all`.
- `cargo test --all-targets` passed with 41 tests.
- `cargo clippy --all-targets -- -D warnings` passed.
- `just ci` passed.

## Notes

- An attempted `cargo test` command with two filters failed because Cargo accepts
  only one positional test filter. Subsequent runs used a common prefix or full
  test target.
- The unrelated untracked `devsweep-audit.jsonl` was preserved.
