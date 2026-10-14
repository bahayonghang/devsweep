# C3 bounded sizing scheduling diagnostic

C2's native Software p95 of 6265.962 ms remains a failed 5000 ms gate. The
candidate below addresses one demonstrated scheduling problem; it does not
replace the required native measurement.

The existing two-worker Rayon pool could retain unvisited independent siblings
inside the sequential fold assigned to a slow child. A bounded deterministic
gate reproduced 48 of 63 unblocked siblings visited before release: the other
15 could not be stolen independently. The candidate applies `with_max_len(1)`
to the two existing root/child parallel iterators. It adds no workers, cache or
metadata API and preserves no-follow inspection, budgets and coverage. The
focused sizing suite initially passed 24 tests. Independent review then found
a test-only gate-timeout assertion gap; that correction and the final full gate
must complete before the native candidate is frozen.

## Initial immutable CLI diagnostic

`dist/performance/cleanup-earth/C3-runtime/` preserves the initial source
snapshot, before that test-only correction. Its 453-file source archive SHA-256
is `7297091ae429fcd96f94d39aa692e9508be8451a5af87c143ba54a09c50f35c1`;
CLI SHA-256 is
`762e9de9066b2c2c34211038c65d156b11d7514a15c07e59f7638a44cb170745`.
The locked release build passed in 1 minute 8 seconds. All source and 11 frozen
frontend asset hashes matched afterward. Relative to C2, the sole production
source change is `filesystem/sizing.rs`. The separately measured navigation
extension to `render-benchmark.tsx` is also present in this source archive but
is not a packaged product input. Frontend assets are byte-identical C2 copies.

The exclusive read-only CLI diagnostic retained first-use observations, one
discarded warmup, then three pairs with build order alternating. Every invocation
used a fresh process/profile; OS file-cache coldness was uncontrolled. Raw argv,
outputs and 200 ms resource samples are under
`dist/performance/cleanup-earth/C3-software-cli-diagnostic-01/`.

| Observation | C2 | Initial C3 |
| --- | ---: | ---: |
| First use | 3032.948 ms | 3189.224 ms |
| Discarded warmup | 3053.938 ms | 2949.427 ms |
| Retained 1 | 3184.966 ms | 3071.038 ms |
| Retained 2 | 4262.571 ms | 3037.929 ms |
| Retained 3 | 3284.795 ms | 3241.118 ms |
| Retained median | 3284.795 ms | 3071.038 ms |

The median difference is approximately -6.51%, with only three diagnostic
samples per build. This is preliminary CLI evidence, not proof of a native
p95 improvement or satisfaction of the 5000 ms gate. Native IPC, persistent
processes and frontend activity differ from this one-shot CLI workload.

All ten processes exited naturally with code 0 and no owned leftovers. Each
contained 1100 entries. The complete V1 envelopes matched after excluding only
inventory observation time, size observation times and the inventory fingerprint
derived from those times. Every entry, eligibility fact, size value/state,
source status, ordering and envelope field otherwise remained identical.
The unmodified raw envelopes and fingerprints remain retained.

## Final frozen candidate

The test-only timeout guard now asserts that no automatic gate release occurred,
after explicitly releasing and joining the worker. Its exact regression passed.
The first full gate then exposed an existing fixture inheriting the build's
`CARGO_TARGET_DIR`: Cargo correctly returned the override while the fixture had
created only its default `target`. The independently reviewed correction isolates
that test in a child process without the override and asserts the exact fixture
target. Both the initial full-gate failure and the direct with/without-override
reproductions remain in `dist/`; no production scan authority was changed.

The final canonical `just ci` passed 745 tests with five existing ignored
diagnostics, plus format, check and Clippy. Final source is separately frozen at
`dist/performance/cleanup-earth/C3-final/`:

- Source archive: `1bce947fad5ef895a211621217b0436d265308d57b59912b1aa6a96701f984b3`.
- CLI: `3c44b362b2d0a6b8440a41e66cca36f2613395f695bbeb6b7c68ebd9ecc9a67a`.
- Native: `7426037283a60829ee1d124bbd5433c6c0a6005cea307a9df425d1c98cfa85b6`.

All 453 source and 11 frozen frontend asset hashes matched after both release
builds. The native build passed in 3 minutes 28 seconds. Packaging/origin and
new native Software acceptance are still pending at this checkpoint.
The initial diagnostic CLI has a different binary hash and remains a diagnostic;
formal final measurements use these final identities.

`equivalence-to-c2.json` records 450 unchanged source files. The three changed
files are sizing (two production scheduling statements plus tests), project
scanner (test-only environment isolation) and the separately built navigation
benchmark entry. All packaged frontend assets are byte-identical to C2.
Tauri transport/visibility/HUD/runtime, Status and output sinks, Analyze and
Optimize source files are unchanged. Retained C2 measurements remain explicitly
C2 observations over those unchanged subsystems. Shared sizing requires new
Software and Clean comparisons, not inherited timing claims.

`build-invocations.json` retains the actual historical process-local Cargo
environment override used during these builds. Subsequent builds use Cargo's
`--target-dir` argument per repository convention. Benchmark command processes
are separate; pending CLI/Earth protocols explicitly assert that
`CARGO_TARGET_DIR` is unset, preserving the scanner's real override semantics.

## Formal C3 result and diagnostic follow-up

The final native owner's packaged Clean smoke passed with complete CLI report
parity and normal closure. The subsequent fresh B3/C3 Software series retained
first use, a discarded warmup and five formal alternating pairs. C3 samples
were 4636.347, 5207.057, 5211.467, 5012.952 and 4671.259 ms: nearest-rank p95
5211.467 ms still **fails** 5000 ms. B3 p95 was 6570.115 ms. All fourteen
1100-entry inventories had equal semantics under the documented timestamp and
derived-fingerprint exclusions; both owned apps closed normally without force
or leftovers. Full formal details belong to the native operation evidence.

The [separate native phase profile](native-software-phase-profile.md) preserves
an instrumented source copy and artifact. It shows MSIX enumeration around one
second and installed-path sizing around 3.4–3.9 seconds, with little time in
queueing, assembly or the remaining request interval. Its single-window results
are diagnostic only; C3's formal failure remains unresolved. Final Clean timing
pairs and paired Earth process resources have not started while the final
candidate is still under diagnosis.
