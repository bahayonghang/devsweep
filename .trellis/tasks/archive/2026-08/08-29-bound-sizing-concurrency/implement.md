# Implement - Bound Filesystem Sizing Concurrency

Resume this already `in_progress`, paused child only after the user approves the
latest parent/child planning summary. Continue the existing checkpoint; do not
reset its diff or treat its outstanding measurement record as verified.

## 0. Baseline

1. Snapshot every pre-existing dirty path outside this child's declared change
   list and reconfirm that all of them remain unrelated and excluded.
2. Build the unmodified release binary and copy it to the ignored
   `target/devsweep-bench/unbounded/` directory.
3. Add the temporary `reports_current_global_rayon_workers` test-only probe,
   run the exact command from `design.md`, and record its
   `rayon::current_num_threads()` output with the logical CPU count and
   `RAYON_NUM_THREADS` environment state. Remove the probe before product edits.
4. Record the absolute root, deterministic entry count, and release command.
5. Ensure no sibling build/test/package or other CPU/disk-heavy workload is
   running, then run/discard one warm-up. Retain the baseline executable.

## 1. Refactor root fan-out without behavior change

1. Extract the depth-zero child operation with its per-child local budget.
2. Add explicit `Dedicated` and `SerialFallback` root execution modes.
3. Implement the serial mode first and prove parity against the current
   parallel behavior on complete and low-budget fixtures.
4. Keep the nested shared-budget sequential loop unchanged.

Focused gate:

```powershell
rtk cargo test -p devsweep-core filesystem::sizing
```

## 2. Add the dedicated pool

1. Add the lazy cached pool initialization with candidate ceiling `2`.
2. Route every production parallel root fan-out through `pool.install(...)`.
3. Route cached pool failure only to the explicit serial fallback.
4. Add direct pool-size and synchronized peak-worker tests.

## 3. Add deterministic regressions

1. Replace the false `serial_estimate_tree` helper with a real forced-serial
   path.
2. Add low entry-budget and low max-depth fixtures.
3. Add synchronized mid-walk cancellation for dedicated and serial modes with
   bounded waits and the `< 250 ms after release` assertion.
4. Retain pre-cancel, reparse/no-follow, bytes, mtime, completeness, and warning
   assertions.

## 4. Select the ceiling

1. Build and preserve the two-worker candidate.
2. Run one discarded warm-up for the candidate.
3. Collect five alternating A/B pairs against the preserved baseline binary.
4. Sort each five-sample set, take the third value, and apply the 1.20 formula.
5. If two fails, repeat the exact protocol with four. If four fails, stop.
6. Write all evidence below before broader checks.

Keep this complete warm-up and paired-sample window repository-exclusive. Do not
run the icon child, `just ci`, desktop builds, packaging, or unrelated scans
until the final candidate sample is recorded.

## 5. Validation and review

```powershell
rtk cargo test -p devsweep-core filesystem::sizing
rtk cargo test -p devsweep-desktop scan
rtk git diff --check
rtk just ci
```

Review gates:

- no `build_global()` or uncapped `par_iter()` for production sizing;
- only the explicit serial fallback bypasses the dedicated pool;
- no contract/config/dependency changes;
- no change to per-root-child local budget semantics;
- unrelated files remain untouched.

## Measurement Record (implementation fills this)

- Host / logical CPUs: Windows x86_64, `24` logical CPUs from
  `[Environment]::ProcessorCount`.
- `RAYON_NUM_THREADS` state: unset at Process, User, and Machine scopes.
- Effective baseline global pool workers: `24`, printed by the temporary exact
  `reports_current_global_rayon_workers` probe; the probe was removed before the
  final diff.
- Absolute root / deterministic entry count:
  `D:\Documents\Code\Rust\Exp\devsweep` / `110393`, counted with
  `(Get-ChildItem -LiteralPath $root -Force -Recurse -ErrorAction Stop |
  Measure-Object).Count` immediately before the exclusive window.
- Exact command: `<binary> scan D:\Documents\Code\Rust\Exp\devsweep --projects
  --json | Out-Null`.
- Release build identities:
  - A / unbounded: `target/devsweep-bench/unbounded/devsweep.exe`, `2673664`
    bytes, SHA-256
    `C89EB17F2CCFA98A7393E15A6AA338E92F25001FCD2459E90792C685669128FA`.
  - B / two workers: `target/devsweep-bench/two-workers/devsweep.exe`, `2686976`
    bytes, SHA-256
    `1D87A025C595F936F242F5EB91FF7DF6BD4F4CD890754CBED52EDFC82BD7463A`.
- Warm-up policy: one discarded run per binary
- Discarded warm-ups: A `31810.878 ms`; B `33945.057 ms`.
- Pair order: `A/B, B/A, A/B, B/A, A/B`
- Paired samples in execution order:
  `A 35715.874 / B 40222.620`,
  `B 38281.145 / A 38016.762`,
  `A 36388.675 / B 36281.985`,
  `B 42232.281 / A 36582.462`,
  `A 38117.836 / B 39999.052` milliseconds. Every invocation exited `0`.
- Unbounded samples sorted / median:
  `[35715.874, 36388.675, 36582.462, 38016.762, 38117.836]` /
  `36582.462 ms`.
- Two-worker samples sorted / median:
  `[36281.985, 38281.145, 39999.052, 40222.620, 42232.281]` /
  `39999.052 ms`.
- Four-worker samples sorted / median: not run because two workers passed.
- Formula and ratio: `39999.052 <= 36582.462 * 1.20 = 43898.9544`;
  `39999.052 / 36582.462 = 1.093394`, PASS.
- Selected ceiling / peak observed workers: `2` / exactly `2`; the direct test
  pool also reported `2` threads.
- Window note: no cargo/rustc/devsweep process existed at the final precondition
  check, and no build/test/package/icon work ran until the fifth pair completed.
  Each scan emitted the same non-fatal `ref/repo/putzen-rs` Cargo metadata
  warning and still exited `0`.
- Detailed command, exit, warning, and gate evidence:
  `evidence/verification-2026-08-30.md`; complete `just ci` output:
  `evidence/just-ci-2026-08-30.log`.
