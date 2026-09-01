# Verification — reopen five-mode AC4 (`analyze.p95_cpu_le_200`)

Working-tree copy of `08-29-analyze-core-ipc`. Not archived. Not committed.

## Change

Two dedicated Analyze workers (`ANALYZE_WORKERS = 2`, not the global Rayon pool) were pinning walk-window process CPU at ~200% plus coordinator/kernel, so nearest-rank p95 of 200 ms samples sat at 202–204%. Each worker now takes a coarse `Sleep(1)` after work, every 10 ms of worker wall time — not per node — and only after a cancel check. That keeps the two-worker ceiling and leaves accounted 256 MiB, the 250k cap, cancel-to-join, no-follow, and cleanup-authority-free contracts unchanged. The five-mode CPU formula is still `delta TotalProcessorTime / Stopwatch wall * 100`.

The native live-churn test now waits until `read_dir` lists `z-deleted` before deleting, so a slower sibling test holding `ANALYZE_JOB` cannot delete those files before they are queued.

## Native CPU (release `devsweep.exe`)

Command (one unrecorded warmup + five measured reps):

```text
analyze scan --root %TEMP%\devsweep-five-mode-v1\analysis-250k-v1 --format json --output <file>
```

Clock: `Stopwatch` from `Process.Start` until process exit. 200 ms samples of the CLI PID. No 600 s pad. stdout drained; machine document goes to `--output`. CPU% = delta `TotalProcessorTime` / wall × 100. p95: nearest-rank `ceil(0.95 * n) - 1` (same as `tools/measure-resources.ps1`). For the five run-p95s, n=5 so that protocol p95 is the maximum run-p95.

Fixture: existing `analysis-250k-v1` under `%TEMP%\devsweep-five-mode-v1` (5000 dirs × 49 files). Every measured run stored 250,000 nodes and `completeness=partial_budget`.

| Rep | elapsed_ms | exit | cpu samples | run p95 % | run median % |
| --- | ---: | ---: | ---: | ---: | ---: |
| warmup | 25846.207 | 0 | 85 | 192.78 | 174.09 |
| 1 | 25355.171 | 0 | 84 | **190.15** | 174.86 |
| 2 | 25315.543 | 0 | 82 | **189.80** | 173.82 |
| 3 | 25588.153 | 0 | 83 | **193.51** | 175.21 |
| 4 | 25655.370 | 0 | 85 | **189.74** | 174.03 |
| 5 | 25525.152 | 0 | 83 | **188.81** | 175.03 |

- All five measured run-p95s **≤ 200%** — **PASS**
- Median-of-p95s = **189.80%**
- Protocol statistic (nearest-rank p95 of the five run-p95s, i.e. the max) = **193.51%** ≤ 200%
- Binary: `target/release/devsweep.exe` SHA-256 `46387a57944641fe4dde99353a69fe0b3c6d0627777cb044cc19129d92d0d43b` (5,255,168 bytes)
- Raw: `cpu-raw-20260831T202941Z/`, `native-cpu-summary.json`, `native-cpu-probe.log`

Prior independent five-mode-v1 miss on this host was walk-window p95 **203.39%** (desktop IPC loading→`partial`, ~12 s, 62 samples). A 20 ms park interval still produced two CLI run-p95s just over 200% (201.26% / 200.49%); the 10 ms cadence is the recorded gate.

## Cancel

Release `analysis_native_50k_v1_resource_gate` (park enabled; captured around the 10 ms cadence work): `cancel_to_join_ms = [3, 3, 3, 3, 3]`, p95 **3 ms** (cap 500 ms). The full `--release` analysis filter was re-run after the 10 ms interval (16 passed). Debug `cancel_joins_and_marks_canceled` still passes (cancel requested before walk, so workers never park).

## Tests

| Command | Exit | Log |
| --- | ---: | --- |
| `rtk cargo test --locked -p devsweep-core -- analysis` | 0 (15 passed) | `core-analysis-tests.log` |
| `rtk cargo test --locked --release -p devsweep-core -- analysis` | 0 (16 passed) | `core-analysis-release-tests.log` |
| `rtk cargo test --locked -p devsweep-cli -- analyze` | 0 (13 passed) | `cli-analyze-tests.log` |
| `rtk git diff --check` | 0 | `git-diff-check.log` |
| `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` | 0 | `release-build.log` |
| Native CLI CPU probe (warmup + 5 reps) | 0 | `native-cpu-probe.log` |

`just ci` was not re-run (focused gates above).

## Source files

- `crates/devsweep-core/src/analysis/walker.rs` — 10 ms worker park after each processed item; cancel checked before sleep
- `crates/devsweep-core/src/analysis/tests.rs` — live-churn mutator waits for `z-deleted` listing
