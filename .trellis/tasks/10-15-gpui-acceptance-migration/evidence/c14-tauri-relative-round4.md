> **SUPERSEDED — read c14-tree-comparison-round4.md instead.**
>
> This file measured only the root process of each build. Tauri is a thin native
> shell that spawns a WebView2 child-process tree, so the root process reports a
> misleading 5 MiB / 116 ms while its whole owned tree is 171 MiB / 1140 ms to a
> settled usable state. The protocol explicitly requires the **whole owned
> tree**, so the numbers below are wrong for acceptance purposes. They are kept
> rather than deleted so the mistake stays visible. The corrected measurement is
> in `c14-tree-comparison-round4.md`.
# C14 relative performance against a fresh Tauri baseline (round 4)

Date: 2026-10-18. Same host, same empty working directory, AB/BA alternation,
machine otherwise idle.

| Build | Binary | SHA-256 | Bytes |
| --- | --- | --- | --- |
| Tauri baseline | `dist/tauri-baseline-target/release/devsweep-desktop.exe` | 5A64470D5C334F19AF07DB7F7C58CE56094231ABAF4C5611493C8B180D71FBC4 | 13306880 |
| GPUI candidate | `desktop-gpui/dist/goal-gpui-release/release/devsweep-gpui-spike.exe` | DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34 | 14179328 |

The Tauri baseline was built this round with
`cargo build --locked --release -p devsweep-desktop --target-dir dist/tauri-baseline-target`
(exit 0). The frontend `desktop/dist` was produced by the same `npm run tauri
-- build` invocation, so the WebView content is the current tree, not a stale
bundle.

**Important scope caveat:** the GPUI side is the single-window spike, not the
five-mode shell. The Tauri side is the real five-mode application. These are not
feature-equivalent workloads, so the ratios below are indicative, not the
feature-equivalent comparison the protocol ultimately requires.

## Startup: process create -> first non-offscreen UIA window

20 retained launches each, alternating order.

| Metric | Tauri baseline | GPUI spike | Ratio |
| --- | --- | --- | --- |
| p50 | 116 ms | 488 ms | 420.7 % |
| p95 | 154 ms | 583 ms | 378.6 % |

Raw Tauri (ms): 222, 154, 145, 120, 117, 117, 124, 116, 109, 107, 122, 133, 111, 119, 109, 107, 104, 109, 111, 113
Raw GPUI (ms): 482, 531, 466, 545, 583, 541, 614, 488, 504, 520, 520, 562, 501, 473, 438, 485, 488, 461, 477, 451

Budget: p95 <= 1500 ms **and** <= 80 % of the fresh Tauri baseline.
Result: **FAIL** (GPUI p95 583 ms vs Tauri p95 154 ms = 378.6 %).

## Idle private memory: 30 s settled

| Metric | Tauri baseline | GPUI spike | Ratio |
| --- | --- | --- | --- |
| Private bytes | 5 MiB | 113 MiB | 2260 % |
| Working set | 28 MiB | 74 MiB | — |
| Child processes | 1 | 0 | — |

Budget: <= 200 MiB **and** <= 70 % of the fresh Tauri baseline.
Result: **FAIL** (GPUI 113 MiB vs Tauri 5 MiB = 2260 %).

## Interpretation

The GPUI spike is **slower** to first window than the Tauri baseline. The relative startup clause is therefore not met even on the absolute limit's side; this is recorded as a FAIL rather than excused by the scope caveat.

The GPUI spike uses **more** private memory than the Tauri baseline, so the relative memory clause is not met.

Because the two sides are not feature-equivalent (single window vs five modes),
these numbers do **not** satisfy C14's "equivalent current baseline/candidate
workloads" requirement on their own. They are the first real relative
measurement and they replace the earlier NOT_RUN with a concrete result, while
the feature-equivalence gap stays explicitly open.

