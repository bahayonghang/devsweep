# Implementation Plan: Durable Action Journal

## Preconditions and activation gates

- Consume `ValidatedPlan` digest from `07-28-plan-validation` and the shared
  process-output sanitizer from `07-28-process-runner-cancellation`.
- Agree the app-data helper and transient protected-path hand-off with
  `07-28-central-safety-policy`.
- Approve any direct cross-platform file-locking dependency needed at the
  project MSRV; do not emulate a lock with an unsafe stale sentinel.

## Ordered work

1. Add a fault-injectable journal writer and fail-red tests for started
   write/flush/sync failure, terminal write/flush/sync degradation, and
   process interruption between records.
2. Define versioned event records, run/sequence allocation, replay result, and
   structured report states. Ensure action facts come from `AuthorizedAction`
   and actual execution outcomes.
3. Implement started-before-side-effect protocol, partial-report preservation,
   terminal records, halt-on-audit-failure scheduling, and replay of unfinished
   starts or malformed final lines.
4. Implement shared app-data default location, safe no-follow open, private
   permissions, same-directory atomic setup, and exclusive writer lock.
5. Register default and overridden audit paths with SafetyPolicy for the run;
   reject symlink/reparse replacement and competing writers before side effects.
6. Migrate CLI and TUI runtime to one audit configuration/default path and pass
   sanitized ProcessRunner diagnostics through record/report paths.
7. Update README durability language and run crash/replay, concurrency,
   path-replacement, and platform-permission fixtures.
8. Run focused executor/TUI/CLI tests, inspect replay output, then run `just ci`.

## Verification matrix

| Failure or behavior | Required evidence |
| --- | --- |
| started write/flush/sync fails at N | Nth runner/trash call is zero; later actions do not start; first N-1 reports/records remain intact |
| terminal persistence fails | action is reported as audit-unknown/degraded; later actions do not start |
| interruption after started | replay produces `started_unconfirmed`, never a fabricated success |
| record authority | command/trash record has actual path, cwd, executable, exit code, run/seq, digest, and bytes |
| noisy/control stderr | record/UI hold sanitized bounded tail only |
| competing writers | second writer fails before any side effect and JSONL lines remain non-interleaved |
| path replacement | symlink/reparse audit-path replacement is rejected before journal/cleanup action |
| platform permissions | default file is user-private on Windows and Unix equivalents |

Run fault injection on all hosts. Run no-follow, permission, locking, and
reparse tests on Windows and Unix; lack of actual platform evidence is a no-go
for the secure-storage portion. Run `cargo test --all-targets` and `just ci`.

## Review and rollback points

- Review every side-effect branch to prove `action_started` precedes it.
- Review journal failures so no completed facts disappear from `ExecutionReport`.
- Review custom audit-path registration in SafetyPolicy before executor starts.
- Do not activate until upstream digest/sanitizer and secure locking decisions
  are recorded; do not weaken default durability to improve throughput.
