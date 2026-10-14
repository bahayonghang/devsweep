# Runtime acceptance ledger

This ledger preserves the approved budgets. Pending native measurements are not
replaced by component timings or deterministic tests. The current immutable
candidate is `C3-final`: native SHA-256
`7426037283a60829ee1d124bbd5433c6c0a6005cea307a9df425d1c98cfa85b6`, CLI
`3c44b362b2d0a6b8440a41e66cca36f2613395f695bbeb6b7c68ebd9ecc9a67a`.
Its [source/build and C2 equivalence record](software-c3-scheduler.md) keeps the
initial C3 diagnostic and failed C2 Software observations separate. C3 native
acceptance and affected final measurements are pending; C2 measurements below
retain their actual identities. Earlier archives and superseded attempts are
described in [runtime resumption](runtime-resumption.md) and
[references](reference-builds.md).

| Criterion | Current status | Evidence and remaining gate |
| --- | --- | --- |
| RT-AC1 | Pending final measurement set | Immutable A/B3/C2/C3-final, fixture manifests, exact argv, owned PID/FILETIME and normal-exit records are retained. Final CLI/native resource comparisons are pending. Every failed/invalid-scope attempt stays available. |
| RT-AC2 | Partial; remaining healthy A comparison | Full B3/C2 envelopes and exits match all six repaired-provider cases; native 100-target paused/reload/minimized reports match B3 in every field. Terminal policy emits zero unused snapshots; 151-target terminal/interactive final plan and health are identical. Healthy A/C and timed B3/C fixture pairs remain pending. |
| RT-AC3 | Pending paired Clean guards | Deterministic eliminated work is established: 100 discoveries materialize two snapshots at the cadence boundary; 100 raw Tauri observations produce two DTO projections. Root enumeration retains at most 64 names, nested names stream singly, and inherited fresh-child budgets remain intact. The 1.20× median, +64 MiB private and +2 thread guards await five alternating warm pairs. |
| RT-AC4 | Pass with explicit evidence boundaries | Frozen 1k/10k component interactions pass 100 ms p95 (10k row/select-all 14 ms; preview 13.5 ms), all targets retained. Native pauses of 3 s and 6 s each deliver one packet in the first 100 ms after resume, with exact acknowledgments and final settlement ordering. Reload/cancel/active close pass. The long 1k native attempt exceeded its harness observation cap and is retained as failed completion evidence; its later cooperative cancellation is separately recorded. |
| RT-AC5 | Pass; native console resize unavailable | TUI 10k replacements retain one pending value; finite 32-event service interval; 100 idle render checks draw once. The Status 16+1+1 lossless channel cancels/joins with an undrained receiver. The real slow-pipe, broken-pipe, named JSON create-new and named NDJSON terminal tests pass on the identical C1/C2 CLI bytes. Native TUI startup/help/Esc/Q and idle screen pass; native resize returned WinError 87 and remains unavailable. |
| RT-AC6 | Lifecycle pass; paired presentation resources pending | C2 actual native minimize stops decoration while Clean progresses. Twenty Earth route/minimize/restore cycles release resources. C2 first-show plus 20 HUD cycles and simultaneous live Status pass; every hidden HUD has zero samplers and no later publication. Explicit Status continues while minimized. Preview/execute wait-only behavior is covered by fake adapters; no real cleanup is used as evidence. Paired A/C still/animated resources remain pending. |
| RT-AC7 | C3 Software still fails; unchanged-subsystem gates pass on C2 | C3-final five-pair Software p95 is 5211.467 ms against the unchanged 5000 ms limit; its failed series remains retained alongside C2's 6265.962 ms failure. C2 Analyze component layout 1.8941 ms/commit 8.4 ms pass 50/100 ms. C2 exact 548 s no-debugger idle passes: host 0.0% median, 12,103,680 bytes peak private, 25 threads. C2 Optimize p95 2.247 ms and Analyze acknowledgment p95 1.953 ms pass 1000/500 ms, every Analyze attempt joined. Their source-equivalence boundary is explicit; they are not relabeled C3 runs. |
| RT-AC8 | Pass | Five actual C2 same-PID stop cycles each retain 25 scheduled 200 ms post-join samples. Every final-five CPU sample is 0.0%, below the same-instance 0.0% baseline +0.5 percentage points. Thread floors are 16, 14, 14, 13, 13; final growth is below +1. ACK is 2–7 ms and join 4–13 ms. |
| RT-AC9 | Partial; final evidence closeout pending | Final C3 canonical `just ci` passed 745 tests with five existing ignored diagnostics; the initial test-environment failure is retained. C2 frontend 438 Vitest/5 Node, lint/typecheck/build/wire checks remain valid for byte-identical production assets and unchanged production frontend source. Native Clean/Earth and component gates retain their own build identities. Remaining native/CLI timing gates must complete before this criterion closes. |

## Supporting evidence

- [Scanner/TUI/Status deterministic work counts and lifecycle tests](scanner-tui-status-implementation.md), followed by [independent corrections/review](independent-runtime-check.md) and [C2 gate](independent-c2-check.md).
- [Six repaired-provider golden cases](repaired-provider-golden-parity.md): Go query dispatch, configured roots, npm shim, nested tox/Python, healthy-plus-missing partial, Yarn delegation. Complete envelopes, bytes and exits match without normalization.
- [Large-result component measurements](frontend-rendering.md): separate decode/reducer/DOM/render records and full 10k selection proof; not native OS input or GPU completion.
- [Slow pipe and create-new JSON](status-sink-acceptance.md), [real console/file terminal](cli-console-acceptance.md). C2 CLI is byte-identical to C1, so these are valid retained CLI checks; that identity does not substitute for changed native-build checks.
- [Native operations and Analyze components](native-operation-acceptance.md).
- [Native Earth lifecycle and actual/emulated scaling boundaries](../../10-13-desktop-planet-fidelity/evidence/native-earth-acceptance.md).

The final resource plan uses separate matched A/C still/animated process-tree
windows, five alternating pairs with 5 s state settle and 15 s retained per state.
Both builds receive identical CDP/media/RAF observers. This comparison includes
all build differences and has no invented absolute GPU/process-tree budget.
Software/Optimize/Analyze, Status/HUD, CLI pairs and this presentation comparison
run serially after the uninstrumented idle protocol. No threshold is relaxed
after a miss, and unavailable short samples are not reported as zero.

The [instrumented native Software phase diagnostic](native-software-phase-profile.md)
locates the dominant work in MSIX enumeration and installed-path sizing. Its
single-window observations are distinct from formal paired acceptance and do
not close C3's failed 5000 ms gate.

## Final uninstrumented idle

`dist/performance/cleanup-earth/C2-idle-final-01/` preserves the predeclared
protocol, exact launch, all 200 ms samples, five-window summary and separate
`tree-resources.json`. The app had no debug port and retained the host's actual
reduced-motion preference. One PID 146356 / FILETIME 134363868007288317 ran the
full 548 s: 8 s startup, discarded 90 s, then five 30 s settle/60 s retained windows.
Each retained window contains 292–293 samples.

All inherited host-PID gates pass: median of five CPU medians 0.0% (limit 1%),
maximum private 12,103,680 bytes / 11.54 MiB (limit 192 MiB), maximum 25 threads
(limit 40). CPU uses the historical definition 100% = one logical core; 0.0% is
the observed counter result at its resolution, not a claim of zero physical work.
The separate owned tree reached 227,717,120 bytes private, 267 threads and 7 processes.
Every retained window had a stable process-identity set, with integrated tree
CPU 0–0.0262%. No tree limit is substituted for the host limits.

Normal scheduled WM_CLOSE produced exit 0. Helpers settled naturally in 257.8 ms,
with no forced teardown or owned leftovers. C1's earlier 47-thread short run
remains a diagnostic with a different duration, not a matched formal speedup
comparison. The final absolute gate demonstrates the bounded-runtime repair.

## C2 native operation failure retained

The native owner completed 42 read-only operation samples: separate first use,
one discarded warmup, and five alternating B3/C2 pairs for Software, Optimize
and Analyze cancellation. Both owned apps closed normally with exit 0 and no
force or leftovers. C2 Software p95 was 6265.962 ms against 5000 ms; B3 was
6525.158 ms and also failed. The relative change does not pass the absolute gate.
The full raw series stays retained while its source is profiled. Optimize
p95 2.247 ms and Analyze acknowledgment p95 1.953 ms passed; maximum Analyze
join was 3.334 ms. Operation details and precise inventory comparison exclusions
belong to [native operation acceptance](native-operation-acceptance.md).

Product C2 remains immutable. Any verified Software repair receives a new
candidate identity and repeats the affected gate; C2 is never relabeled C3.

The [C3 sibling-scheduling diagnostic](software-c3-scheduler.md) preserves its
separate initial CLI/source identity. Three diagnostic pairs show a modest CLI
median reduction with equal complete inventory semantics. This does not close
the native gate. The final candidate still requires its independent full gate,
new native artifact and affected Software/Clean measurements. Shared sizing is
the only proposed production change from C2; inherited frontend, visibility,
Status/HUD and asynchronous-runtime evidence must be labeled as C2 observations
over unchanged subsystems, never as measurements of the new native artifact.

## C2 Status/HUD lifecycle and post-join resources

`dist/performance/cleanup-earth/C2-status-hud-final-01/` contains five real UI
Status start/stop cycles, actual native minimization, simultaneous live Status
and 20 HUD visibility cycles. `resource-summary.json` correlates the IPC join
timestamps with the first 25 scheduled 200 ms samples after each join. The
same-instance pre-live CPU median is 0.0%; all 25 final-five samples across the
five cycles are 0.0%, within the unchanged +0.5 percentage-point ceiling.
Thread floors are 16, 14, 14, 13, 13. Acknowledgments range from 2 to 7 ms and
joins from 4 to 13 ms; each stream has a terminal and no later publication.

All 20 HUD shows have one actual sampler and a fresh snapshot; each hide joins
the sampler, leaves zero named HUD samplers, and produces no later snapshot.
Explicit main-window Status remains live through minimization and the HUD
exercise, then cancels and joins. These are separately labeled instrumented
lifecycle/resource observations, not the uninstrumented idle record.

Exact host PID 127684 exited normally with code 0 after the driver completed.
Its helpers settled naturally in 252.3 ms; no forced teardown or owned process
remained. The controller, raw samples, five stop records, minimized proof,
combined Status/HUD stream and individual visibility records are retained.
