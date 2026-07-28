# Implementation Plan: Bounded Process Runner

## Preconditions and activation gates

- Approve direct production dependencies required for native Windows Job
  Objects and Unix process-group signaling (`windows-sys` and the selected
  minimal Unix FFI crate/surface).
- Agree the cancellation observer signature with
  `07-28-true-cancellation`; use a no-op adapter until its token exists.
- Do not change provider rule policy, durable audit schema, or the cleanup-plan
  wire contract in this child.

## Ordered work

1. Add child/grandchild helper fixtures plus fail-red tests for timeout, tree
   cleanup, endless output, non-UTF-8, nonzero exit, cancellation observation,
   and neutral cwd.
2. Define request/policy/result types, bounded ring-buffer capture, lossless
   parser-facing bytes, and sanitized display/audit output conversion.
3. Implement deadline calculation and the portable runner supervisor with a
   bounded post-termination wait.
4. Implement and dynamically test Unix process-group and Windows Job Object
   backends. Add a documented macOS dynamic test row.
5. Add neutral temporary cwd behavior and explicit caller override support.
6. Migrate provider probes, map typed failures to diagnostics, and ensure one
   provider timeout does not abort the global phase.
7. Migrate executor command execution, retain program/argv separation, and
   pass sanitized/truncated diagnostics to its current reporting path.
8. Run focused process/provider/executor fixtures, measure configured probe and
   global deadlines, then run `just ci`.

## Verification matrix

| Scenario | Required evidence |
| --- | --- |
| hung child/grandchild | timeout returns within policy; descendant PID is gone after grace period |
| endless stdout/stderr | retained tail is capped, truncation is marked, and process RSS remains bounded |
| non-UTF-8/nonzero exit | deterministic typed result with no panic and sanitized tail output |
| provider timeout | timed-out probe returns typed diagnostic while later providers still run |
| neutral cwd | probe run under a fixture project with `.npmrc` observes neutral cwd behavior |
| control characters | UI/audit-facing message contains no executable terminal control sequence |
| cancellation hand-off | no-op observer preserves current behavior; injected observer yields `Canceled` before next action |

Run tree tests on Windows and Linux, plus the macOS process-group row. A test
that cannot prove child/grandchild termination is a no-go for the corresponding
backend, not a skipped pass. Run `cargo test --all-targets` and `just ci` after
all migrations.

## Review and rollback points

- Review that no call path uses `Command::output()` or a shell string after
  migration.
- Review reader shutdown to avoid deadlock when a child exits while pipes are
  still draining.
- Review every displayed/audited process message for the shared sanitizer.
- Before activation, record dependency approval and cancellation-interface
  agreement; without either, keep this task in planning.
