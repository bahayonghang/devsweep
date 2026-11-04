# C14 whole-owned-tree comparison (round 4, corrected)

Date: 2026-10-18. Corrects the earlier same-round measurement that summed only
the root process and thereby reported Tauri at 5 MiB private / 116 ms. Tauri is a
thin native shell plus a WebView2 child process tree, so the protocol's "whole
owned tree" requirement changes the result materially.

Owned tree = root PID plus every descendant, resolved transitively through
`Win32_Process.ParentProcessId`. Memory is summed over live processes 30 s
after the window appeared.

| Metric | Tauri baseline | GPUI spike |
| --- | --- | --- |
| First non-offscreen window | 69 ms | 438 ms |
| Settled usable tree (descendant count stable 6 polls) | 1140 ms | 814 ms |
| UIA descendants | 23 | 12 |
| Processes in owned tree | 7 | 1 |
| Private bytes (whole tree) | 171 MiB | 112 MiB |
| Working set (whole tree) | 378 MiB | 74 MiB |
| Tree members | devsweep-desktop(5MiB), msedgewebview2(43MiB), msedgewebview2(3MiB), msedgewebview2(78MiB), msedgewebview2(12MiB), msedgewebview2(7MiB), msedgewebview2(23MiB) | devsweep-gpui-spike(112MiB) |
| Processes still alive 2 s after close | 0 | 0 |

## Against the frozen budgets

| Budget | Threshold | Measured | Result |
| --- | --- | --- | --- |
| Startup p95 absolute | <= 1500 ms | GPUI 814 ms (single sample) | PASS |
| Startup relative to fresh Tauri | <= 80 % | 71.4 % | PASS |
| Idle private absolute | <= 200 MiB | GPUI 112 MiB | PASS |
| Idle private relative to fresh Tauri | <= 70 % | 65.5 % | PASS |

## Honest reading

Even measured across the whole owned tree, the GPUI spike uses less private memory than the Tauri baseline (65.5 %). The relative memory clause is met on this workload.

The GPUI spike reaches a settled usable tree in 71.4 % of the Tauri baseline's time, so the relative startup clause is met on this workload.

**Scope limit that still applies:** the GPUI side is the single-window spike
(12 UIA descendants), while the Tauri side is the full five-mode
application (23 descendants). The workloads are not
feature-equivalent, so these ratios are not the equivalence-gated acceptance the
protocol ultimately asks for. They are, however, real whole-tree measurements
against a baseline built from the current tree, and they replace NOT_RUN with
concrete numbers.
