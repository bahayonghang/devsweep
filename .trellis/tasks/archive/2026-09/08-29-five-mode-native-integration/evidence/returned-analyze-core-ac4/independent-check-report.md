# Independent Check — reopen five-mode AC4 (`analyze.p95_cpu_le_200`)

Working-tree copy of `08-29-analyze-core-ipc`. Not archived. Not committed. No production-code change by this checker.

Overall verdict: **PASS**

PASS requires all three: focused tests green, independent native walk-window process CPU p95 **≤ 200%**, and cancel tests still passing. All three held. `ANALYZE_WORKERS` remains 2. The 256 MiB accounted cap, 250,000 stored-node cap, and no-follow contract were not weakened.

## Reopen claim under review

The prior independent five-mode-v1 miss was walk-window process CPU p95 **202.4–203.7%** with two dedicated workers. The implementer added a cancel-checked `Sleep(1)` every 10 ms of worker wall time (not per node). This check recaptured tests and native CLI CPU on **new PIDs** against that walker, without reusing the implementer’s sample files.

## Native CPU (independent recapture)

Binary: `target/release/devsweep.exe` SHA-256 `46387a57944641fe4dde99353a69fe0b3c6d0627777cb044cc19129d92d0d43b` (5,255,168 bytes). Walker source is older than this binary; cargo reported the release CLI already up to date.

Command (one unrecorded-in-verdict warmup + five measured reps; warmup still sampled):

```text
analyze scan --root %TEMP%\devsweep-five-mode-v1\analysis-250k-v1 --format json --output <file>
```

Clock: `Stopwatch` from `Process.Start` until process exit. 200 ms samples of the CLI PID. No 600 s pad. stdout drained; machine document goes to `--output`. CPU% = delta `TotalProcessorTime` / wall × 100. p95: nearest-rank `ceil(0.95 * n) - 1`. For the five run-p95s, n=5 so walk-window p95 is the maximum run-p95.

Fixture: existing `analysis-250k-v1` under `%TEMP%\devsweep-five-mode-v1` (5000 dirs × 49 files). Every measured run stored 250,000 nodes and `completeness=partial_budget`.

Measured PIDs are all new versus the implementer recapture (`20300`, `55200`, `72660`, `47480`, `75688` plus warmup `52708`).

| Rep | PID | elapsed_ms | exit | cpu samples | run p95 % | run median % |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| warmup | 61556 | 25497.555 | 0 | 83 | 192.15 | 173.80 |
| 1 | 49752 | 25018.538 | 0 | 82 | **187.97** | 172.67 |
| 2 | 44980 | 24846.884 | 0 | 80 | **189.91** | 171.11 |
| 3 | 76108 | 25072.012 | 0 | 82 | **189.78** | 176.50 |
| 4 | 77804 | 25013.951 | 0 | 81 | **189.91** | 172.58 |
| 5 | 10956 | 24376.697 | 0 | 78 | **191.68** | 172.93 |

- All five measured run-p95s **≤ 200%** — **PASS**
- Median-of-p95s = **189.91%**
- Overall walk-window p95 (nearest-rank p95 of the five run-p95s) = **191.68%** ≤ 200% — **PASS**
- Raw: `independent-cpu-raw-20260831T203916Z/`, `independent-native-cpu-summary.json`, `independent-native-cpu-probe.log`

No walker self-fix round was required (CPU already ≤ 200% on round 1 of 3).

## Cancel

| Gate | Result |
| --- | --- |
| Debug `cancel_joins_and_marks_canceled` (included in locked `analysis` filter) | **PASS** (15-test suite) |
| Release `analysis_native_50k_v1_resource_gate` mid-walk cancel | **PASS** |

Independent release 50k evidence (`independent-native-50k-cancel-direct.log`):

- `cancel_to_join_ms = [3, 3, 3, 3, 4]`, nearest-rank p95 **4 ms** (cap 500 ms)
- `workers: 2`
- five resource samples: 51,007 nodes, 9,095,340 accounted bytes (≤ 268,435,456), two warnings
- denied branch remains access-denied incomplete zero-byte

Park is skipped when cancel is already requested, and cancel is re-checked immediately before `Sleep(1)`.

## Tests and whitespace

| Command | Exit | Log |
| --- | ---: | --- |
| `rtk cargo test --locked -p devsweep-core -- analysis` | 0 (15 passed) | `independent-core-analysis.log` |
| `rtk cargo test --locked -p devsweep-cli -- analyze` | 0 (13 passed) | `independent-cli-analyze.log` |
| `rtk git diff --check` | 0 | `independent-git-diff-check.log` |
| `rtk cargo test --locked --release -p devsweep-core -- analysis -- --nocapture` | 0 (16 passed) | `independent-core-analysis-release.log` |
| `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` | 0 (already up to date) | `independent-release-build.log` |
| Independent CLI CPU probe (warmup + 5 reps) | 0 | `independent-native-cpu-probe.log` |
| Direct release `analysis_native_50k_v1_resource_gate --nocapture` | 0 | `independent-native-50k-cancel-direct.log` |

`just ci` was not re-run (focused gates above).

## Contract and safety audit

- `ANALYZE_WORKERS = 2` in `crates/devsweep-core/src/analysis/model.rs`; dedicated pool still uses that count. Not reduced.
- `MAX_STORED_NODES = 250_000` and `MAX_OWNED_BYTES = 268_435_456` (256 MiB) unchanged. Native 250k CLI runs hit the node ceiling with `partial_budget`.
- Native adapter still uses `inspect_path_no_follow`. Walker production paths do not delete, trash, or spawn cleanup commands.
- Worker park is coarse (10 ms wall cadence, 1 ms sleep), after `process_item`, and only if cancel is not requested.
- Live-churn test waits until `z-deleted` is listed before deleting; that does not change production caps.

## Findings (fixed)

None. CPU recapture passed without a walker edit.

## Findings (not fixed)

None blocking AC4. `rtk cargo test ... -- --nocapture` still collapses to the rtk summary line, so cancel JSON was recaptured from the release test executable with `--nocapture`.

## Blockers

None.

## Checker actions

- Production code: none.
- Evidence: this report, independent probe script, CPU raw samples, test/cancel logs, and `check.jsonl` journal entries.
- Did not commit. Did not archive.
