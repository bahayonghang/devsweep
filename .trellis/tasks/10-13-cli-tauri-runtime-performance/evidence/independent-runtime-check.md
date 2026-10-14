# Independent integrated source review

2026-10-13. The dispatched Trellis checker reviewed the runtime child, then
extended the same checkpoint to the parent and all three children after the
coordinator released the integrated Phase 2.2 source. Existing functional F1-F7
verification remains in the functional child's `evidence/independent-check.md`;
this review does not relabel the earlier reference builds or native prechecks.

## Findings fixed

### R1: cancellation at the end of size enumeration reported a complete undercount

In `crates/devsweep-core/src/filesystem/sizing.rs`, the final `Iterator::next`
could request cancellation and return `None`. The pending root batch was then
skipped by a cancellation guard without setting `complete = false`. A singleton
could report a trusted zero, and a final partial batch after 64 visited entries
could report a trusted truncated total.

The pending batch now enters the existing cancel-aware walkers. They perform no
new filesystem inspection after cancellation, retain observed lower-bound bytes,
and mark the estimate incomplete with `Canceled`. The new deterministic test
failed before the fix with `pending bytes must not be trusted`; it covers 1, 2,
65 and 66 names in dedicated and serial modes. All 23 sizing tests passed after
the fix, including existing no-follow/denied-directory and singleton/fan-out
budget regressions. The two-worker and 64-name bounds are unchanged.

### R2: progress observation latched freshness before the common final ranking

In `crates/devsweep-core/src/scan/mod.rs`, `ScanPublisher::discovered` ranked the
shared accumulator in place. The freshness guard permanently clears default
selection. An Interactive candidate first observed before the seven-day floor
could therefore remain deselected after crossing it, while TerminalOnly applied
its first guard at the later final boundary and selected the same candidate.

Publication now clones once, deduplicates/ranks that owned snapshot and emits it;
the accumulator is unchanged. Both policies retain the existing common project
and global phase-boundary ranking. A deterministic regression publishes a fresh
candidate and then ranks both policies' original candidates at the same later
time. It failed before the fix with different selection/evidence and passes now.
The publication cadence test now checks the raw accumulator separately from the
ranked snapshot. All 20 tests matching `scan::tests` passed, including complete
151-target policy equality and the first/99/100 ms materialization proof.

## Production paths reviewed

- Scanner/project cancellation, terminal-only CLI Clean and reviewed rescan,
  cumulative snapshot projection, final-only authority promotion, and size
  confidence/budget boundaries. The observer fix changes no stable fixture
  target facts away from the freshness threshold; B3 remains the comparison
  reference. Final candidate report equality still needs measurement.
- TUI mailbox capacity, same-job/same-phase replacement, phase/latest/terminal
  order, finite 32-event drain, dirty redraw, direct reducer dispatch refusals,
  terminal-last worker behavior and receiver-close-before-join teardown.
  Read-only cancellation and wait-only Clean/uninstall/maintenance teardown
  follow distinct production branches.
- Status lossless 16-record queue plus cancellation/terminal reserve of two,
  canceled catch-up loop, actual `LiveControl::join`, receiver-close wakeup and
  broken-pipe adapter. Tests of synthetic production queues do not establish
  native sink latency or interruptibility of a blocked OS pipe write.
- Tauri normal scan and rescan route through the same one-unacknowledged-packet
  delivery. Exact acknowledgement validation, lock/condition-variable ordering,
  cancel wakeup and worker-owned slot release were traced. React acknowledges
  only decoded, matching progress after listener success; errors use the
  invocation's cancellation callback. Main-page reload/navigation start,
  native destruction and app exit cancel the wait; normal minimize does not.
  Tauri may retain one last unconsumed packet after page loss: this is not a
  claim that its private IPC queue was purged.
- Root state invalidation, reviewed-target identity handoff, full-set selection
  behind pagination, closed wire registration, and Earth async readiness,
  hidden/resize/forced-color/reduced-motion/fallback/disposal paths. The local
  RGB texture and bounded GPU renderer remain separate from cleanup authority.
- Current runtime/publication/delivery specifications, Earth notices and
  previously recorded browser/native evidence boundaries. Browser GPU-submit
  timings are not whole-process CPU or GPU completion measurements. Earlier
  failed interaction samples remain recorded rather than being called passes.

No additional confirmed source defect remains from this integrated pass. No
cleanup, uninstall, maintenance or user-data mutation was used for verification.

## Verification and acceptance boundary

Focused checks after the two fixes:

- `cargo test --locked --target-dir dist -p devsweep-core --lib filesystem::sizing::tests`:
  23 passed.
- `cargo test --locked --target-dir dist -p devsweep-core --lib scan::tests`:
  20 passed.
- Scoped `git diff --check`: passed (Git reported only line-ending notices).

The canonical final `just ci` runs with `CARGO_TARGET_DIR=dist` and the real
Windows token needed by isolated ACL/executable-identity fixtures. Its raw log
is `dist/just-ci-integrated-review.txt`; final outcome is appended below.
The coordinator owns the separate final frontend gate (Node 22/bundled npm 10).

This source checkpoint does not close native C acceptance, immutable C build
identity, equal-scope B3/C measurements, paused-renderer/reload recovery in real
WebView2, native Earth lifecycle/scale/offline packaging or process resources.
Those remain owned by the runtime/native agents and parent requirement map.
Passing quality gates cannot substitute for those outstanding measurements.

## Final quality result

`just ci` completed with exit code 0 on the frozen source. Formatting, workspace
all-target `cargo check`, tests and all-target Clippy with `-D warnings` passed.
The preserved log records:

| Suite | Passed | Existing ignored |
| --- | ---: | ---: |
| CLI unit | 191 | 0 |
| CLI contracts / five-mode / provider authority | 12 / 7 / 5 | 0 |
| Core unit | 436 | 3 |
| Core public API | 1 | 0 |
| Tauri desktop unit / wire parity | 91 | 2 |

Ignored diagnostic/fixture entrypoints were not changed or counted as passes.
Cargo was released to the runtime owner for the immutable C release build.
The coordinator's latest frontend log is `dist/frontend-integrated-ack-gate.txt`.
The source Earth's two hashes and lengths were independently checked against
the notices: 658,001 + 250,789 bytes = 908,790 bytes, below the 1 MiB payload cap.
This filesystem check is separate from the final packaged/offline native check.

The checker also inspected the coordinator's latest frontend log: Node 22.23.2
and bundled npm 10.9.8, lint, TypeScript, 58 Vitest files / 432 tests, five Node
tests and production build passed. It is independent of the Rust quality log.

## Post-freeze measurement-harness review

At the coordinator's request, only `tools/status-sink-smoke.py` was changed after
the product source freeze. Its original blocking `readline` had no read deadline;
its `subprocess.run(timeout=...)` would kill a timed-out snapshot process despite
the generic `forced_termination: false` claim. Parse/identity/missing-file errors
could also exit before recording a result.

The harness now polls an unbuffered Windows pipe with a shared 10-second read
deadline and a documented 16 MiB single-record harness bound. Snapshot children
write to file sinks and use `Popen.wait`, which does not terminate on timeout.
Timeouts preserve the owned PID/creation identity, report `left_running` and stop
subsequent attempts. A top-level finally writes `result.json` on probe failure.
The five stream records must be newline-terminated, start at sequence zero and
retain their existing consecutive-sequence/single-operation/V1 checks.

`py_compile` and scoped diff checks passed. Harmless Python children verified
exact record boundaries, an unread-pipe deadline, natural snapshot exit, and a
snapshot timeout that leaves its child alive until it exits naturally with code
zero. An injected parse failure verified a persisted failed result with no
forced termination. These are driver checks; they are not the actual release
Status measurement. The coordinator will run that against the frozen C CLI.
