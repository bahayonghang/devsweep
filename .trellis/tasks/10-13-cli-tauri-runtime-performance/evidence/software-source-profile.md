# Original A Software source profile

Measured 2026-10-13 with the preserved original release CLI, SHA-256
`b7280f91304ec965c025e9d110967896d8b4f1d068603fff792d2ed2eb0c8fd6`.
Raw observations, stdout and 200 ms resource samples remain ignored at
`dist/performance/cleanup-earth/A-source-profile/`. The tracked
`software-source-summary.json` contains aggregate counts/timings only.

Each source ran seven fresh processes and isolated profiles: first invocation
separate, one discarded warmup, then five measured runs. Source order alternated
forward/reverse between rounds. OS file cache is uncontrolled. Native builds,
tests and frontend benchmarks were coordinated to pause through the measurement
slot. Subsequent timestamps prove the final B frontend build was later (last
profile ended 09:52:17; final assets written 09:53:34/46), but an earlier
1.8-second Vite build did not retain exact start/end times. These source splits
are diagnostic profiles, not final isolated acceptance runs. Paired release
acceptance must use a fresh exclusive slot. Coordination also does not control
unrelated host activity or remove observed timing variance.
Every invocation had an explicit 120-second bound, exited 0 and left no owned
process. All sources were available and entry counts stayed unchanged. Each
source's five warm documents were identical after removing only observation
timestamps and the derived timestamp-bearing fingerprint; normalized SHA-256
values are retained in the summary JSON.

| Source | Entries | First invocation ms | Warm median ms | Warm p95 ms |
| --- | ---: | ---: | ---: | ---: |
| ARP | 620 | 97.24 | 85.83 | 102.93 |
| MSI | 293 | 118.59 | 125.24 | 126.29 |
| MSIX | 187 | 5879.39 | 5321.61 | 8642.61 |
| All | 1100 | 5426.01 | 7266.56 | 11523.43 |

Nearest-rank p95 for five measured samples is the maximum. All inventory still
**fails** the inherited 5-second gate. The all-source implementation already
overlaps sources, so the difference between separate and combined runs is not
evidence that times should add; the measured variance also prevents subtracting
these totals to infer exact internal phase cost.

The dominant source is MSIX. MSI concurrency/property changes are not justified
by this profile. MSIX root-process CPU across the sampled interval was
5.98–13.81 CPU-seconds over 4.06–8.64 wall-seconds (includes two sizing workers);
peak private memory was 5.50–5.79 MiB and peak threads 10. This suggests CPU work
inside the MSIX path but does not yet distinguish enumeration from sizing.
Sub-200 ms ARP/MSI calls lack sufficient resource samples and are not reported
as zero-resource operations.

Reproduction uses `tools/measure-readonly.py run --exe
dist/performance/cleanup-earth/A/devsweep.exe --fixtures
dist/performance/cleanup-earth/fixtures --out <fresh-output-directory>
--workload software-msix --deadline 120`; substitute `software-arp`,
`software-msi` or `software` for the other closed read-only sources. No install,
uninstall, maintenance or cleanup was performed.

The opt-in ignored Windows test
`software::msix::phase_benchmark::profile_msix_inventory_phases`, calling the
unchanged real MTA enumeration and installed-path sizing in separate timed
spans. It emits only durations, counts, state and remaining MTA worker count.
Adding this test does not change production inventory behavior or waive the
requirement to freeze corrected B before runtime optimization.

One exclusive-slot diagnostic ran the preserved debug test executable
`dist/native-test-isolation/core-current.exe`, SHA-256
`3e170670b4a264e5255df3b493ef888463a701c233d6b625bb8bfc142fcdabaa`, with
`software::msix::phase_benchmark::profile_msix_inventory_phases --exact --ignored
--nocapture --test-threads=1`. Enumeration took 1086.89 ms, installed-path sizing
8859.67 ms, total 9946.69 ms. All 187 sizes were available, none partial/unknown,
and zero MTA workers remained. Test passed and process exited 0.

This debug, single-run phase diagnostic identifies installed-path sizing as
the dominant phase. It is not an uninstrumented release timing or acceptance
sample. Shared sizing optimization must retain native no-follow checks, size
confidence and fresh per-child budgets, then prove release B/C parity and timing.

## Entry-count diagnostic

The subsequent preserved test executable
`dist/native-test-isolation/core-msix-counters.exe`, SHA-256
`502a49a0fcea7664984d4b45150827ee1690992edef5a548b858fab6b9c0060b`,
contains both opt-in phase and counting tests. In an exclusive native slot,
`profile_msix_metadata_calls --exact --ignored --nocapture --test-threads=1`
observed 187 installed paths, 187 complete sizes and 64,610 native entry
inspections. Every inspected path was unique: zero repeated inspections,
zero combined-inspection fallbacks and zero extra reparse probes. Summed time
inside native inspection was 3811.34 ms. This uses serial package order and
test counters, so it is an operation-count diagnostic, not production latency.

The same executable's normal phase test then measured 809.74 ms enumeration,
2790.80 ms sizing and 3600.83 ms total. All 187 paths were distinct, all sizes
available, and zero MTA workers remained. Both tests passed and exited 0.
Raw records are `dist/native-test-isolation/msix-metadata-counts-host.txt` and
`msix-phases-host.txt`.

The initial sandbox invocation returned zero packages despite source state
`available`; it is retained as `msix-metadata-counts.txt` and is **not** a valid
workload or performance sample. The two host runs used scoped read-only
execution outside the sandbox and matched the baseline's 187 packages.

No duplicate-path cache or extra worker pool is justified by these counts.
The large timing variance also means this faster single debug run cannot
replace the failed release p95 record. Preserve the existing two workers and
native no-follow inspections; evaluate bounded enumeration/cancellation, then
repeat alternating matched release B/C measurements.
