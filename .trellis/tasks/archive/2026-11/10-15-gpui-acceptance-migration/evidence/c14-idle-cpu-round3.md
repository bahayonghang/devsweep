# C14 idle CPU sampled per protocol (round 3)

Date: 2026-10-18 (America/New_York).
Binary SHA-256: D1E778D32CAE1E7D0AB99BD8FFCEAEA1090A4AB2CA2F6CDE424C7D20E90EE061
Bytes: 14179328. Single window, empty unrelated working directory.

Method follows research/performance-protocol.md: 30 s settled idle after the
window appeared, then 200 ms samples over a 30 s retained interval. CPU is the
process TotalProcessorTime delta divided by wall-clock delta, expressed as a
percentage of one logical core. 150 retained samples.

Raw samples (% of one core, 4 dp):
0, 0, 0, 0, 0, 0, 7.4052, 0, 0, 0, 7.3357, 0, 7.3357, 0, 0, 0, 7.4052, 0, 0, 7.3703, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.3014, 14.7406, 0, 14.7406, 7.4405, 7.4405, 0, 0, 0, 7.3357, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.4405, 7.4761, 0, 0, 0, 0, 7.4761, 7.3703, 0, 7.2674, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.3703, 0, 0, 0, 0, 0, 0, 0, 0, 7.4405, 7.4761, 7.5483, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.4405, 7.4052, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.4405, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

| Metric | Value | Budget | Result |
| --- | --- | --- | --- |
| Median | 0 % | <= 0.5 % | PASS |
| p95 | 7.4405 % | <= 1.0 % | FAIL |
| Mean | 1.2334 % | (not budgeted) | recorded |
| Max single sample | 14.7406 % | (not budgeted) | recorded |
| Private bytes (settled) | 113 MiB | <= 200 MiB | PASS |
| Working set | 75 MiB | (not budgeted) | recorded |
| Child processes | 0 | owned tree | recorded |

Shutdown: WM_CLOSE, exited=True, exit code 0.
Processes named devsweep-gpui-spike remaining 1 s after close: 0.

## Note on the earlier single-window figure

An earlier run in this round reported 3.542 % from one 30 s window. That was an
undivided average, not the protocol's median/p95 over 200 ms samples, and it is
superseded by the table above rather than deleted. The difference is expected:
a single 30 s average is dominated by any one scheduling spike, while the
protocol asks for the distribution.

This is single-window spike measurement. It is not five-mode GPUI acceptance,
and the relative clauses against a fresh Tauri baseline remain NOT_RUN because
no fresh Tauri baseline was built in this round.
