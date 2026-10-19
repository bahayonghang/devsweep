# C14 idle CPU: reproducible p95 failure (round 10)

Date: 2026-10-19. Binary `devsweep-gpui-spike.exe` release, launched from an
empty temp directory. Protocol method: 30 s settled idle, then 200 ms samples
over a 30 s retained interval (150 samples), CPU as percentage of one logical
core, nearest-rank percentiles.

## Result: p95 exceeds the budget, reproducibly

| Metric | Value | Budget | Result |
| --- | --- | --- | --- |
| Median | **0 %** | <= 0.5 % | **PASS** |
| p90 | 7.7351 % | — | — |
| **p95** | **7.7351 %** | <= 1.0 % | **FAIL** |
| Mean | 1.988 % | — | — |
| Max sample | 15.4703 % | — | — |
| Samples above 1.0 % | **38 of 150** | — | — |
| Private bytes (settled) | 113 MiB | <= 200 MiB | PASS |

Raw samples (% of one core, 150 values):

```
0, 0, 0, 0, 0, 7.7351, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.2005, 0, 0, 0, 0, 0, 0, 0, 7.6593,
0, 0, 7.2674, 0, 0, 15.4703, 0, 0, 0, 0, 0, 0, 0, 7.7736, 0, 0, 0, 7.7351, 0, 7.7736,
0, 0, 7.8125, 0, 0, 0, 0, 0, 0, 7.622, 0, 7.697, 7.7351, 0, 7.7351, 0, 7.5483, 0, 0,
7.697, 0, 0, 7.6593, 0, 0, 0, 0, 7.6593, 7.2674, 7.7351, 7.622, 0, 7.697, 7.7351, 0,
7.585, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.7736, 0, 0, 0, 0, 7.697, 7.7736,
7.7351, 0, 0, 7.7351, 0, 0, 7.697, 0, 0, 0, 7.697, 7.7736, 0, 0, 7.697, 7.622, 7.2005,
0, 7.7351, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7.7351, 7.697, 0, 0, 0, 0,
0, 0, 0, 7.1674, 0, 0, 0, 0
```

## This reproduces the round-3 failure

Round 3 measured p95 = 7.4405 % on the same shape of workload. This run measures
p95 = 7.7351 %. Two independent runs, both far above the 1.0 % budget, with a
median of exactly 0. The failure is **stable and reproducible**, not a fluke of
one noisy window. The round-3 figure stands; this round confirms it.

## Spike structure

38 of 150 samples are non-zero, and essentially every non-zero sample is one or
two 15 ms scheduler quanta (~7.7 % of a 200 ms window is ~15 ms). Gaps between
spikes were 1, 2, 3, 4 and 8 samples, so there is **no single fixed cadence**.

Total CPU over the 30 s window was about 60 ms. That is genuinely low in absolute
terms — the process is idle almost all the time — but the protocol is written as
a distribution budget, and a p95 of 7.7 % fails it regardless of the small total.

## Where the work comes from

The shell schedules **no recurring timer on the idle landing**:

- `view.rs:4` — "Render does not request an animation frame."
- `shell.rs:829` — "C10 paints stills only. No presence or activity schedules a
  repeating frame."
- The only polling loop is `request_project_rows` (`view.rs:251-278`), which runs
  only after an explicit F9 / project-rows request, while activity is non-idle,
  bounded to 10 s. It cannot fire on an idle landing.
- `tray.rs` and `status_panel.rs` hold no sampler in this build.

So the bursts are **not** shell logic. The plausible source is the windowing and
graphics stack (compositor/DWM interactions, cursor blink, or accesskit polling),
but that was **not instrumented**, so it is a hypothesis and is labelled as one.
A real attribution needs an ETW/CPU-sampling capture of the idle process, which
was not taken.

## Consequence

C14-AC1's idle-CPU p95 clause is a **reproducible FAIL** against the frozen
1.0 % budget. The budget is not relaxed and the result is not averaged away. It
also means the earlier round-3 conclusion was right and the round-10 attempt to
overturn it was a measurement bug in the percentile helper (PowerShell treated a
one-element sort result as a scalar), now corrected.
