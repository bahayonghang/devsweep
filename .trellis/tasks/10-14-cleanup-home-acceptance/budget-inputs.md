# Frozen measurement inputs

Use the current parent's pre-edit baseline, not historical numeric results, for before/after comparisons.

- Existing Clean production harness: `desktop/src/modes/clean/render-benchmark.tsx`, driven by `tools/clean-render-benchmark.mjs`; 5 warmups, 30 measured samples, nearest-rank p95, unchanged 100/1k/10k reports and eligible selection assertions. Keep the existing 100-row rendering limit and retained full-report counts.
- The inherited Clean interaction threshold is p95 <=100 ms for large preview/review input, recorded in `.trellis/tasks/archive/2026-10/10-13-cli-tauri-runtime-performance/prd.md` RT-AC4 and `evidence/frontend-rendering.md`. Do not use the new baseline's observed time as a newly relaxed threshold.
- That component harness excludes the new homepage ownership/advisory grouping. Add an appropriate production component scenario for that surface; report its sample method and any absent prior comparable measurement honestly. No arbitrary PASS budget is invented for a new metric.
- Native/whole-process resources and other inherited budgets are described in the archived runtime task's RT-AC7 and design. Unmeasured items stay NOT_RUN; successful fixture paint timings do not prove native input or idle CPU.
- Run measured candidate rendering after concurrent compilation/tests have stopped. Preserve matching before/after build identities, raw samples, CSS viewport, DPR and process teardown. DPR emulation is not a real Windows scale change.
