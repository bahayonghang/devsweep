# C14 idle CPU after the demand-driven frame pump (round 13)

Date: 2026-10-19. Source commit `0d691dd299fe69a827cde5b1805d2b0d0286b884`
(`perf(gpui): 按需刷新空闲窗口帧`). The measured binary was built from that same
source before the commit and was not rebuilt afterward.

Binary: `desktop-gpui/dist/goal-gpui-release/release/devsweep-gpui-spike.exe`
SHA-256 `6DD3806B20E3A8A1032F73609E4BB2F98FD44F4A5C8ED35BFBB97E489D9C5C47`,
14193152 bytes. `dist/` is not in the commit.

Protocol: 30 s settled idle, then 150 samples. Each sample's wall time is the
stopwatch time to the next 200 ms boundary. CPU is
`100 * sum(owned-process TotalProcessorTime delta ms) / wall ms`, one logical
core. Nearest-rank: index `ceil(p * 150) - 1`. Budgets stay median <= 0.5% and
p95 <= 1.0%. rustc 1.99.0 is not an MSRV 1.88.0 pass.

## Round 13: whole owned tree

Toolhelp32 enumerated descendants on every sample. Settle ids `181968`.
`MAX_ALIVE=1`. `DEPARTED_OBSERVATIONS=0`. `SEEN_IDS=181968`. Tree private at
settle `118923264` bytes (113.4 MiB). A second root read was `125763584` bytes.
`EXIT=0`. Window appeared.

| Metric | Value | Budget | Result |
| --- | --- | --- | --- |
| Median | 0 | <= 0.5% | PASS |
| p95 | 0 | <= 1.0% | PASS |
| Nonzero samples | 4 of 150 | — | — |
| Max sample | 15.1188% | — | — |
| Tree private | 113.4 MiB | <= 200 MiB | PASS |

Raw samples:

```
0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15.1188, 0, 14.7198, 7.8445, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.7838, 0, 0, 0, 0, 0, 0, 0, 0, 0
```

Sorted, index 74 (median) and index 142 (p95) are both 0. The four nonzero
values sit at the top of the sorted array.

## Rounds 11 and 12: root process only

Those runs sampled `TotalProcessorTime` of the root process and did not
enumerate children. Round 13 found one owned process, so the root was the owned
tree on that launch. Both exited 0 after `WM_CLOSE`.

Round 11: PID 130284, private `119013376`, nonzero 1, median 0, p95 0, max
8.3629. Raw: 149 zeros, then `8.3629`.

Round 12: PID 173960, private `117895168`, nonzero 1, median 0, p95 0, max
7.7113. The nonzero sample is the 56th value (`7.7113`); the other 149 are 0.

## What this does not close

C14-AC1, C14-AC2, and C14-AC3 stay unchecked. Relative-to-Tauri, the other
protocol rows, a packaged installer, and a native action beyond launch stay
NOT_RUN. Historical p95 7.4405% and 7.7351% remain in
`c14-idle-cpu-round3.md` and `c14-idle-cpu-round10.md`.

Locked tests on rustc 1.99.0, target `desktop-gpui/dist/goal-gpui-test`:
`cargo test --locked -p gpui_windows -- frame_pump::tests::empty_queue_yields_nothing_and_duplicate_enqueue_yields_once`
exit 0 (1 passed); `cargo test --locked` exit 0 (62 passed, 0 failed).
