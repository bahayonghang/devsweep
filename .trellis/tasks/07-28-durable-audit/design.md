# Design: Durable Action Journal

## Journal protocol

The audit log is an append-only JSONL journal with a per-run UUID and monotonic
sequence number. Every authorized side effect has exactly one `action_started`
record before runner/trash invocation, followed by one terminal record when
durable recording succeeds: `action_finished`, `action_skipped`, or
`action_canceled`. A replay marks a started record without a terminal record as
`started_unconfirmed`; it never infers success from absence of failure.

`action_started` writes a complete line, flushes, and calls `sync_data` before
the side effect. A write, flush, or sync failure prevents that action, records
an audit-blocked execution-report entry when possible, and stops scheduling new
actions while retaining earlier report entries. A terminal write/flush failure
after a side effect makes its durable outcome unknown and stops later actions.
If a terminal `sync_data` attempt fails after a successful write/flush, the
report distinguishes the observed action outcome from degraded durable
confirmation; it does not claim a fully durable terminal record.

## Record authority

Records are built from `AuthorizedAction` and actual runner/trash outcomes, not
from target id or UI labels. Fields include run id, sequence, canonical plan
digest, canonical action path, action identity, cwd, resolved executable,
program/argv summary where relevant, exit code, estimated and actual bytes,
and sanitized/truncated diagnostics. The shared sanitizer is consumed from
`07-28-process-runner-cancellation`; raw stderr never enters UI or journal.

`ExecutionReport` becomes a partial-preserving report: completed actions remain
visible after a later audit failure, the affected action has an explicit
audit-blocked/unknown state, and the halt reason is structured. This prevents a
write failure from discarding facts already recorded or observed.

## Secure storage and concurrency

Default storage is an app-data audit directory resolved through the shared
platform app-data helper established with SafetyPolicy: a private `audit.jsonl`
plus a same-directory lock. `--audit-log` remains an override but receives the
same no-follow, permission, locking, and parent-directory validation. The live
audit file and lock path are registered as transient `ProtectedSubtree` entries
for the execution so an authorized cleanup target cannot contain them.

The writer opens a non-reparse/non-symlink final path, uses user-private file
permissions (Unix 0600 equivalent and Windows user ACL), and holds an
exclusive cross-process lock for the entire run. A second process fails before
its first side effect. Writer safety requires the same native Windows/Unix FFI
surface planned by ProcessRunner; if stable standard-library locking is not
available at the supported MSRV, use the smallest vetted direct locking crate
only after dependency approval. A partially written final JSONL line is treated
as journal-integrity degradation during replay, never silently skipped as a
valid record.

## API boundaries and compatibility

Plan digest comes only from `ValidatedPlan`; the audit task creates no second
digest. SafetyPolicy owns authorization and protection matching; audit supplies
the active path set for each run. TUI and CLI both construct the same journal
configuration and receive the same structured report. This task does not add a
history UI or a new public replay command; it supplies a tested replay API for
future UI/CLI work and for startup integrity warnings.

## Rollback and operations

The default CWD audit path is removed in favor of app-data. Rollback must retain
the new journal format reader for existing records or explicitly version the
file name; it must not silently write a mixed legacy/new stream. README claims
must state the exact `started` durability guarantee and the terminal-record
degradation behavior.
