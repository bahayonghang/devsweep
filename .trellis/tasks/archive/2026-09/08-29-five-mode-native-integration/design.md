# Design - Final Five-Mode Integration

Integrate in dependency order and maintain a release matrix keyed by surface,
mode, locale, authority level, result state, scale, and evidence type. The final
child owns shared glue and documentation only; failures in domain logic return to
the owning child rather than being patched around here.

Run automated gates before native scenarios. Native scenarios use a disposable
standard-user fixture set and explicit saved plans. Capture process trees to
prove no elevated child/UAC, lifecycle traces to prove cancellation and join,
and resource samples under one workload at a time. Antivirus exclusions are
recorded as environment setup and never counted as a correctness pass.

Release readiness requires every primary mode enabled together, all supporting
routes reachable, no compatibility alias, invariant machine fixtures across
locales, and truthful unsupported states. Rollback is the last known release;
new plan/audit schema versions are never read as old versions.

## Deterministic resource acceptance protocol

Use one recorded Windows 11 standard-user host and a release build. Record CPU
model, logical-core count, RAM, architecture, Windows build, power plan, Rust and
Node toolchains, commit and binary SHA-256, Defender state/exclusions, locale,
and every fixture digest. Keep the host, build, power plan, and fixture invariant
between baseline and candidate. Every workload has one unrecorded warm-up and
five measured repetitions. For each idle/live repetition, allow 30 seconds to
settle and retain the next 60 seconds. Sample every 200 ms.

CPU percentage is `delta process total processor time / elapsed wall time * 100`,
so 100% means one logical core. Memory is private bytes. Record process/thread
trees and integrity level for DevSweep and every child. Use nearest-rank p95,
the median of measured-run values, and the maximum sample as explicitly named
below. Raw CSV/JSON samples and the manifest are evidence; screenshots alone do
not pass a gate.

In relative thresholds, `idle` means the corresponding statistic from the five
idle repetitions: median-sample CPU for CPU comparisons and observed maximum for
private bytes/threads. `baseline` means the same named statistic from the five
accepted sizing-baseline repetitions.

For Status post-stop quiescence, the stop acknowledgement starts one fixed
5.0-second window of exactly 25 scheduled samples at 200 ms. Missing/dropped
samples fail the gate. PASS requires every one of the final five consecutive
samples (a continuous final 1.0-second hold) to satisfy both process CPU <= the
idle median-sample CPU + 0.5 percentage point and thread count <= the idle
observed maximum + 1. The first satisfying sample, a favorable last sample, or a
window median/maximum cannot substitute for this predicate, and the statistic
must not be changed after viewing results.

| Workload | Fixed scenario | Passing thresholds |
| --- | --- | --- |
| Idle desktop | App open, no operation, post-warm-up 60 s | median CPU <=1%; max private bytes <=192 MiB; max threads <=40 |
| Status snapshot | Five snapshot runs | p95 elapsed <=2 s; peak private bytes <=idle+64 MiB; max threads <=idle+4 |
| Status live | 2 s interval, 15 processes, post-warm-up 60 s | median CPU <=5%; p95 CPU <=15%; peak private bytes <=idle+64 MiB; max threads <=idle+4; within 5 s after stop CPU <=idle+0.5 percentage point and threads <=idle+1 |
| Clean scan | accepted sizing fixture and accepted sizing baseline on same host | candidate median elapsed/baseline median <=1.20; peak private bytes <=baseline+64 MiB; max threads <=baseline+2; no other heavy operation |
| Analyze | `analysis-250k-v1`, 250,000 nodes, two workers | accounted owned memory <=256 MiB; peak process private bytes <=512 MiB; p95 CPU <=200%; cancel acknowledgement p95 <=500 ms; UI layout p95 <=50 ms and React commit p95 <=100 ms after five warm-up and 30 measured navigations |
| Software inventory | frozen native/fixture inventory, five runs | p95 elapsed <=5 s; peak private bytes <=idle+128 MiB; max threads <=idle+6; no elevated child or UAC |
| Optimize list/preview | all eight catalogue entries, five runs | p95 elapsed <=1 s; peak private bytes <=idle+32 MiB; max threads <=idle+2 |
| Optimize execute | DNS flush plus each Settings launch individually | DNS completes <=10 s with no orphan child; each Settings launch call returns <=2 s; OS-owned Settings lifetime is recorded but excluded from DevSweep resource totals |

Any threshold miss is a failed AC4, not an advisory judgment. A host/fixture/build
change invalidates comparison and requires a new recorded baseline. The paused
`08-29-bound-sizing-concurrency` child must be explicitly re-approved, completed,
and accepted before the Clean comparison is run; this plan does not resume it.

## File-level change list

- `crates/devsweep-cli/src/application/mod.rs`: register only accepted command
  handlers and remove no compatibility alias outside the frozen CLI contract.
- `desktop/src-tauri/src/lib.rs` and `desktop/src/app-shell/registry.ts`: integrate
  accepted mode/support adapters without moving domain logic into glue.
- `crates/devsweep-cli/tests/five_mode_contract.rs`: add cross-mode coordinator,
  digest, locale, cancellation, stale-event, restart, and audit-version fixtures.
- `tools/measure-resources.ps1`: add the fixed 200 ms sampler, host manifest,
  nearest-rank statistics, threshold evaluation, and raw artifact retention.
- `docs/validation/five-mode-native.md`: record the native scenario matrix,
  evidence paths, PASS/FAIL/UNVERIFIED status, and environment exclusions.
- `README.md`, `docs/guide/cli-migration.md`, and `docs/safety-capability-matrix.md`:
  describe only accepted shipped behavior, breaking syntax, safety boundaries,
  unsupported capabilities, provenance, and rollback.
- `desktop/src-tauri/tauri.conf.json` and packaging metadata: integrate the accepted icon,
  version, provenance, and local package checks without signing or publishing.

Before editing `README.md`, record the current pre-existing user-owned hunks and
their line/context fingerprints. Apply documentation changes as separate hunks,
review hunk ownership before staging/commit, and preserve the recorded existing
hunks byte-for-byte unless the user separately authorizes changing them. The
generic change-list boundary is not sufficient because this task and the user
already overlap in the same file.
