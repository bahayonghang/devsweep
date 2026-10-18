# C14 native performance samples (round 3)

Date: 2026-10-18 (America/New_York).
Binary: `desktop-gpui/dist/goal-gpui-release/release/devsweep-gpui-spike.exe`
SHA-256: D1E778D32CAE1E7D0AB99BD8FFCEAEA1090A4AB2CA2F6CDE424C7D20E90EE061
Bytes: 14179328
Launched from an empty temp directory. Machine load at run time was low
(12 percent, no concurrent cargo/rustc).

Method: process create -> first non-offscreen UI Automation window for the new
PID, polled every 5 ms with `ControlViewWalker` root children. Percentiles are
nearest-rank over the retained raw array. This measures the GPUI spike, which is
one window; it is not the five-mode shell.

## Startup (20 retained launches)

Raw samples (ms): 625, 590, 561, 548, 616, 537, 552, 534, 547, 515, 577, 541, 514, 582, 529, 518, 487, 602, 523, 502

| Metric | Value |
| --- | --- |
| min | 487 ms |
| p50 | 541 ms |
| p95 | 616 ms |
| max | 625 ms |

Against the frozen protocol budget "process-cold first usable window
p95 <= 1500 ms": **PASS** on the absolute limit.
The relative clause (`<= 80% of a fresh Tauri baseline`) is **NOT_RUN**: no
fresh Tauri baseline was built in this round, so the comparison cannot be made
and is not claimed.

## Idle resources (30 s settled after a 30 s warm-up)

| Metric | Value | Budget | Result |
| --- | --- | --- | --- |
| Private bytes | 112 MiB | <= 200 MiB | PASS |
| Working set | 74 MiB | (not budgeted) | recorded |
| CPU over 30 s | 3.542 % of one core | median <= 0.5 %, p95 <= 1.0 % | FAIL |
| Child processes | 0 | (owned tree) | recorded |

The relative memory clause (`<= 70% of a fresh Tauri baseline`) is **NOT_RUN**
for the same reason as the startup ratio.

## Lifecycle (20 open/close cycles)

| Metric | Value |
| --- | --- |
| Clean exits | 20 / 20 |
| Failed exits | 0 / 20 |
| Orphan processes after the run | 1 |

## Not measured

- Warm window open/restore (p95 <= 500 ms): not exercised.
- Navigation / filter / preview UI response (p95 <= 100 ms): the spike has no
  backend jobs to drive.
- 10k-row frame interval: the spike paints no virtualized list.
- Hidden decoration frames: not instrumented.
- Software inventory p95: belongs to the CLI/service path, not this binary.
- Core scan non-regression and cooperative cancel/close: not run.
- High contrast, reduced motion, and Narrator: not run.

Nothing above is a five-mode GPUI acceptance. It is single-window spike
measurement.
