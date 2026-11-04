# C13 20-cycle open/minimize/close lifecycle measurement (round 4)

Date: 2026-10-18. Binary: `devsweep-gpui-spike.exe`, SHA-256
`DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34`. Launched 20 consecutive times from an empty temp directory.

Each cycle: create process -> wait for a non-offscreen UIA window ->
`ShowWindow(SW_MINIMIZE)` -> confirm `IsIconic` -> `ShowWindow(SW_RESTORE)`
-> confirm not iconic -> `CloseMainWindow` -> wait for exit -> count remaining
processes in the owned tree.

| Cycle | Open ms | Minimized | Restored | Exited | Leftover procs |
| --- | --- | --- | --- | --- | --- |
| 1 | 476 | True | True | True | 0 |
| 2 | 443 | True | True | True | 0 |
| 3 | 462 | True | True | True | 0 |
| 4 | 467 | True | True | True | 0 |
| 5 | 417 | True | True | True | 0 |
| 6 | 458 | True | True | True | 0 |
| 7 | 450 | True | True | True | 0 |
| 8 | 470 | True | True | True | 0 |
| 9 | 453 | True | True | True | 0 |
| 10 | 468 | True | True | True | 0 |
| 11 | 439 | True | True | True | 0 |
| 12 | 430 | True | True | True | 0 |
| 13 | 427 | True | True | True | 0 |
| 14 | 455 | True | True | True | 0 |
| 15 | 443 | True | True | True | 0 |
| 16 | 456 | True | True | True | 0 |
| 17 | 470 | True | True | True | 0 |
| 18 | 430 | True | True | True | 0 |
| 19 | 416 | True | True | True | 0 |
| 20 | 442 | True | True | True | 0 |

## Summary

| Metric | Value |
| --- | --- |
| Cycles completed | 20 / 20 |
| Median open-to-window | 453 ms |
| Max open-to-window | 476 ms |
| Every cycle minimized | True |
| Every cycle restored | True |
| Every cycle exited on close | True |
| Zero leftover processes every cycle | True |
| Failures | none |

## Verdict

**PASS** on the lifecycle-retention clause: "No monotonic worker/handle growth
or orphan child process after 20 cycles."

Twenty consecutive open/minimize/restore/close cycles each exited cleanly and left no process behind in the owned tree. There is no orphan accumulation across cycles.

## What this does not cover

C13-AC4 also asks for large snapshot rendering, native UIA selection and
CLI/service parity. Those remain **NOT_RUN**:

- The spike has no 10k-row treemap/process table to render.
- UIA selection still cannot be exercised because the app pins keyboard focus to
  the Note input (see `c09-keyboard-ime-round4.md`), so a keyboard or
  assistive-technology user cannot move a selection.
- The "no process kill based on displayed PID alone" clause is covered by the
  existing unit test `displayed_pid_does_not_kill`, not by this run.

So this closes only the 20-cycle lifecycle half of C13-AC4.
