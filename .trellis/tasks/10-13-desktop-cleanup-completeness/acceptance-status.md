# Integrated acceptance status

This is a continuation ledger, not a release approval. The child PRDs retain
their original thresholds. A passing component or precheck does not close a
criterion that also requires final native or paired-runtime evidence.

## Functional child

| Criteria | Verified evidence | Remaining integration work |
| --- | --- | --- |
| F-AC1, F-AC2 | Rule/report/preview pipelines and provider parser/authority regressions; independent F1–F7 review and final Rust gate passed | No remaining source check at this checkpoint |
| F-AC3 | Packaged C2 from unrelated CWD; two-drive Unicode roots, dedupe/nested removal and persistence; actual ACL-denied root plus healthy project gives complete CLI/Tauri report equality | Fully isolated CLI also retains a global Maven target with the denied root; the combined native GUI global-survivor case remains unverified |
| F-AC4, F-AC6 | C2 native healthy/mixed/failed-zero/true-empty reports agree completely with CLI; partial exit class and distinct UI outcomes; React result/rescan tests and generated contracts pass | Execution scenarios use fake executors and retained-file sentinels, without user-cache cleanup |
| F-AC5, F-AC7 | Required rule matrix, fake-execution sentinels, config/tool/owner drift and wire-parity tests; final Rust gate passed | Unix native behavior is not verified on this Windows host |
| F-AC8 | Final `just ci` and Node 22/npm 10 frontend gates passed, including bilingual CLI reference, wire parity and packaged C2 Clean smoke | No remaining source or packaged Clean gate at this checkpoint |

## Earth child

| Criteria | Verified evidence | Remaining integration work |
| --- | --- | --- |
| UI-AC1 | Original/candidate browser matrix and final C2 native 32-case matrix with exact backing sizes; actual Windows DPI 120/DPR 1.25; preserved font | OS-scale switching unperformed; emulated DPR changes delivered no resize/media event |
| UI-AC2 | Licensed bounded local assets; C2 packaged offline primary/fallback passed | No remaining offline asset gate |
| UI-AC3 | Final C2 cold p95 24.9 ms, warm 0.1–0.2 ms, 93.6% less synchronous painted work; raw runs retained | Paired native still/animated host/tree resources; callback/submission is not GPU completion or whole-app CPU |
| UI-AC4, UI-AC5 | C2 twenty native minimize/restore/route cycles pause and release owned resources; memory/handles remain in bounded bands; final warm gaps 0/4316 | Native no-debug absolute resource gate belongs to runtime |
| UI-AC6 | Final 100-event Cancel p95 7 vs 7.7 ms and navigation 7 vs 7 ms; every destination committed; earlier failed short sample retained | Native backend join measured separately |
| UI-AC7 | C2 Node 22.23.2/npm 10.9.8 lint/typecheck/438 Vitest/5 Node/build passed; prospective specs and native evidence updated | Integrated runtime handoff |

## Runtime child

| Criteria | Verified evidence | Remaining integration work |
| --- | --- | --- |
| RT-AC1 | Original A, frozen frontend B and corrected B3 source/CLI/native hashes preserved; B3 native parity passed; failed prechecks retained | Final C identity and alternating raw samples |
| RT-AC2, RT-AC3 | Six repaired-provider B3/C2 golden cases pass complete JSON, output-byte and exit parity with independent semantic expectations; fixture hashes/timestamps unchanged | Timed equal-coverage Clean comparisons and resource guards remain separate from this correctness pass |
| RT-AC4 | 1k/10k production component interaction budget passes; final C2 paused 3/6s consumer, reload, actual minimize, Cancel and active-close checks pass; reload/minimize full reports equal B3 | Larger 1k reload exceeded a harness completion cap and was cooperatively canceled; retained separately, not counted as timed completion |
| RT-AC5 | Status/TUI bounded delivery tests, C1 real slow pipe, named-file cooperative terminal and create-new JSON pass | Native TUI resize remains unavailable (WinError 87); deterministic resize/redraw tests remain separate |
| RT-AC6, RT-AC8 | C2 five same-PID Status stops join; each retains 25 post-stop samples; every final-five CPU sample is 0% within pre-live +0.5 pp; thread floors 16/14/14/13/13. Actual minimize preserves live sampling; first HUD show plus twenty visibility cycles pass with zero hidden sampler/later events | Final candidate identity must distinguish later Software/sizing changes from this unchanged Status/HUD subsystem evidence |
| RT-AC7 | C2 no-debug idle passes: host CPU median-of-medians 0%, peak private 11.54 MiB, peak 25 threads; Analyze component layout/commit p95 1.8941/8.4 ms; native Optimize 2.247 ms and Analyze cancel ACK 1.953 ms pass | **Software still fails:** final C3 p95 5211.467 ms exceeds the unchanged 5000 ms limit; matched B3 p95 is 6570.115 ms. The earlier C2 failure remains preserved. Diagnose the actual native phases before another product change |
| RT-AC9 | Final frontend/Rust/wire gates and independent whole-program source review passed | Final native and component performance acceptance |

No task is marked complete by this ledger. Generated raw files remain under
ignored `dist/`; tracked child evidence describes identities, methods and
limitations without publishing user software inventories or cache contents.

C1 source gate: `dist/just-ci-integrated-review.txt` records format, workspace
all-target check, 743 passed tests (five existing ignored tests) and Clippy.
C2 source gate after measured native fixes: `dist/just-ci-c2-independent-review.txt`
records 744 passed and five existing ignored. C2 frontend gate:
`dist/frontend-c2-visibility-gate.txt`. Independent review scope is recorded in
runtime `evidence/independent-runtime-check.md` and `independent-c2-check.md`.

The subsequent C3 source gate, `dist/just-ci-c3-independent-review.txt`, passes
745 tests (five existing ignored), format/check and Clippy. It includes the
bounded scheduling regression and the marker-fixture environment isolation.
The first failure was caused by the validation process exporting
`CARGO_TARGET_DIR=dist`, which the real Cargo probe correctly treats as target
authority; it is retained separately. The final test child removes that
variable and asserts the fixture's exact target path. Future validation uses
Cargo's `--target-dir` argument as the existing quality guidelines require.
C3's native timing and parity remain separate gates.

C1 packaged Clean passed complete CLI/Tauri equality for healthy two-drive
Unicode roots, mixed missing roots, failed zero-target scope and a genuinely
empty scope. Its failed Earth minimize and 47-thread idle results remain in
evidence; C2 must be checked again before final native acceptance.

C2's additional actual ACL fixture is recorded in functional
`evidence/native-clean-acl-c2.json`. Its original DACL and all four sentinels
were restored exactly. Project-only GUI and all-scope isolated CLI acceptance
are separate evidence; neither implies the unperformed combined GUI case.
An earlier driver requested the wrong scope and performed a read-only global
scan; that failure is retained and excluded from fixture acceptance. No preview
or cleanup ran. Future scope assertions use semantic labels and captured IPC.

C2 packaged identity is
`5ffaefe242808e9a2074880667209a0ece6facef1b15ab5f782adf36cc2f2a59`.
Its repeated Clean checks passed in `dist/native-clean-c2-final-01-smoke/`.
The first C2 Earth run completed twenty native route/minimize/restore cycles
with paused RAF/draw and released GPU objects, then failed a responsive-size
harness assertion. Corrected checks reuse that exact cycle evidence. The final
fresh-mount matrix asserts exact backing sizes in all 32 cases; a separate probe
shows native CDP DPR emulation emits no media/resize event. Actual OS scale was
not changed. Final C2 no-debug idle passed in `C2-idle-final-01`, with normal
exit 0 and no forced teardown or owned leftovers. The separately measured tree
peaked at 227,717,120 private bytes, 267 threads and seven processes; host-only
thresholds are not applied to those tree totals. Comparative foreground/runtime
acceptance remains pending. An existing selected-tab hover contrast defect is documented without
expanding this task's source changes.

The final C2 native operation set retains 42 observations: separate first use,
discarded warmup and five alternating B3/C2 pairs for Software, Optimize and
Analyze cancellation. All Software reports contain the same 1,100 entries
after excluding observation timestamps and their derived fingerprint; Optimize
reports match, and Analyze cancellations join. These semantic passes do not
waive the failed Software latency. Both owned apps closed normally with no
leftovers. Runtime `evidence/native-operation-acceptance.md` and
`native-operation-c2.json` record raw samples, comparison boundaries and hashes.

C2 Status/HUD acceptance subsequently passed in
`dist/performance/cleanup-earth/C2-status-hud-final-01/`, with normal exit 0 and
no owned leftovers. Software follow-up investigates two-worker work distribution
without reducing coverage, changing metadata authorization, or adding workers.
A later candidate must receive its own identity and affected-path measurements;
C2's failed Software sample is never overwritten or relabeled.

The final C3 candidate is now frozen separately: source archive
`1bce947fad5ef895a211621217b0436d265308d57b59912b1aa6a96701f984b3`, CLI
`3c44b362b2d0a6b8440a41e66cca36f2613395f695bbeb6b7c68ebd9ecc9a67a`, and native
`7426037283a60829ee1d124bbd5433c6c0a6005cea307a9df425d1c98cfa85b6`.
All 453 source files and 11 packaged frontend assets matched after the builds.
Compared with C2, the only product changes are the two sizing dispatch grain
limits; the other source differences are tests and a benchmark-only entry.
C3's packaged Clean/offline/fallback and complete CLI report parity pass for
healthy, missing/partial, failed-zero and empty scopes, with normal exit 0 and
no owned leftovers. Unchanged-subsystem C2 observations retain their original
identity. Paired CLI/Earth measurements remain pending.

The final C3 Software set retains first use, one discarded warmup and five
alternating B3/C3 pairs. C3 warm samples are 4636.347, 5207.057, 5211.467,
5012.952 and 4671.259 ms: p95 **5211.467 ms fails 5000 ms**. All fourteen
1100-entry reports agree after only the predefined observation-time and derived
fingerprint exclusions. Both apps close normally with no leftovers, and final
launch environments have no `CARGO_TARGET_DIR`. Runtime
`evidence/native-software-c3-final.json` retains the observations and raw hashes.
An isolated instrumented copy will diagnose the native stages; neither its
timings nor the earlier CLI diagnostic can replace the failed formal result.

C4 overlap tests have since passed for the four overlap cases, 28 sizing
tests, and 58 software tests, with two ignored native diagnostics left
unrun. The Rust gate then passed 753 tests and five ignored diagnostics.
C4-final is frozen at source
`899df4cb2507c45b4e5881e4a40217f1ffcf7e44c460590ba34ed4b8b34b0bc7`, CLI
`26b428fb16d5180a61bc8f86ec397428023f78fc937e119717a5df30aaab6c6a`, and native
`7686605676f16138d5cd6b9c2b83b9cfab0f1dc293ee81e283832ee451aec2dd`. Provider
parity and the Clean/Earth schedule passed under their existing rules; short
Clean resource gates are unavailable, not passes. That schedule does not
replace the failed C3 Software p95, which was not rerun. The combined native
GUI case is still unverified: run 02 is prepared, but Computer Use could not
present its app-approval prompt in this session, so the fixture was not
launched.
