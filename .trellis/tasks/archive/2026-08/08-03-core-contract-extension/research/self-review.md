# Core Contract Extension Self-Review

Date: 2026-08-03

## Scope Review

- The change is limited to core serialization and execution contracts,
  mechanical CLI/TUI constructor compatibility, focused tests, directly
  affected backend specs, and this task's evidence.
- Existing CLI and TUI behavior remains unchanged. Their execution requests
  omit the new optional confirmation digest, while the future Tauri bridge can
  require the selection-aware digest without adding another execution path.
- `scan --json` remains field-for-field equivalent to the archived child-1
  baseline.

## Contract Review

- Core boundary types use the existing snake_case and typed-tag serde style,
  with round-trip and unknown-field coverage.
- `ConfirmationDigest` binds the canonical validated manifest and sorted,
  deduplicated selection. Unknown and inspect-only selections fail atomically;
  stale execute confirmations fail before audit or side effects.
- Every deduplicated selected target receives one terminal outcome, including
  dry-run and targets not dispatched after cancellation or fail-closed audit
  termination. Report counters, failures, outcomes, and selected-capacity
  totals are covered by consistency tests.
- Duplicate selections are normalized with a structured note, irreversible
  commands are explicit in the public action projection, and capacity keeps
  verified, partial-lower-bound, and unknown confidence.

## Safety Review

- Default dry-run behavior is unchanged, permanent deletion remains disabled,
  and command program/argv remain separate.
- Scanner and model code still create plans only; this task adds no deletion,
  trash, shell, or GUI execution path.
- Audit failures remain fail-closed. A safety-skip audit write failure records
  a failed outcome, halts later dispatch, and marks remaining selected targets
  skipped; the focused regression proves later runners are not called.
- New public names and fields use estimated-recoverable terminology rather
  than claiming that storage was freed.

## Acceptance Verdict

- `just ci` passes with 232 tests. Focused executor, scan serde, public API,
  missing-docs, Trellis validation, formatting, Clippy, and whitespace gates
  pass.
- The deterministic JSON comparison has zero diff; both normalized snapshots
  are 5,828 bytes with the same SHA-256 digest.
- Exact commands and results are recorded in `research/verification.md`.
- No blocking review finding remains for this child task.
