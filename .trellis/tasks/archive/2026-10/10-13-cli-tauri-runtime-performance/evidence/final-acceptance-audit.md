# Final acceptance mapping audit

2026-10-13. Independent documentation-only checkpoint against the parent and
three child PRDs: **32 acceptance criteria** (8 + 8 + 7 + 9). No build, test,
application, benchmark or scan ran for this audit. No threshold or task status
was changed. C3-final build completion was coordinator-reported at closeout;
its final native/CLI/Earth measurements are still owned by the measurement
agents. A planned measurement is not a pass in this document.

This audit checks acceptance coverage and evidence levels. It does not repeat
the completed independent source reviews. The current parent
[ledger](../../10-13-desktop-cleanup-completeness/acceptance-status.md),
[functional summary](../../10-13-cleanup-discovery-execution/evidence/acceptance-summary.md),
[runtime ledger](runtime-acceptance.md),
[Earth component record](../../10-13-desktop-planet-fidelity/evidence/renderer-c2.md)
and [native Earth record](../../10-13-desktop-planet-fidelity/evidence/native-earth-acceptance.md)
are the principal mappings; their linked raw-record boundaries remain binding.

## Parent criteria

| Criterion | Evidence disposition | Actual outstanding part |
| --- | --- | --- |
| P-AC1 | Real geography, provenance, offline packaging, component budgets and C2 native visual/lifecycle evidence are supported by UI-AC1 through UI-AC7 below. | Final runtime/Earth comparison and integration handoff; component painted time is not whole-app CPU. |
| P-AC2 | Golden coverage is supported by shared production rule/core tests, adapter/wire/decoder tests and complete native/CLI equal-input reports for the measured scopes. Custom-provider optimization parity uses B3, not broken A behavior. | Final C3 equal-coverage comparison must be recorded. The distinct F-AC3 GUI combination remains open; do not invent a packaged-native run of every provider as an additional P-AC2 requirement. |
| P-AC3 | Required matrix families have production scan/v2/registry/preview and explicit executable or inspect-only outcomes, recording runners and retained-data negatives. | No new source-level gap identified by this audit. Actual user cleanup is neither required nor authorized. |
| P-AC4 | Core/CLI/component/native records distinguish complete, partial, failed, canceled, inspect-only, owner command and Recycle Bin observations. Provider failures retain usable findings. | Keep fake-executor result scenarios labeled; they are not actual cleanup observations. |
| P-AC5 | Component interactions, C2 lifecycle, no-debug idle, Optimize and Analyze gates are recorded at their actual layers. | C2 Software failed; final C3 Software, paired Clean guards and native still/animated comparisons remain required. |
| P-AC6 | Independent source review plus invalidation, drift, stale-result, confirmation, audit and wait-only adapter tests support the unchanged authority boundary. | Preserve recorded wrong-scope/failed harness attempts; their read-only nature does not authorize another broad user-cache scan. |
| P-AC7 | Latest canonical Rust gate passes 745 tests with five existing ignored entries; unchanged frontend C2 gate passes lint/typecheck/build/wire, 438 Vitest and five Node tests. Native evidence and historical failures are retained. | Final affected C3 native acceptance still needs recording; an existing source gate is not that measurement. |
| P-AC8 | Approved planning artifacts, tree links, context manifests and research provenance are already verified in the planning-validation record. | None identified; no implementation completion is inferred from planning approval. |

## Functional criteria

The [functional summary](../../10-13-cleanup-discovery-execution/evidence/acceptance-summary.md)
and [rule matrix](../../10-13-cleanup-discovery-execution/evidence/functional-rule-matrix.md)
provide the detailed test references. Their seven-pass/one-open disposition is
consistent with the PRD at this checkpoint.

| Criterion | Evidence disposition | Boundary or missing evidence |
| --- | --- | --- |
| F-AC1 | Pass: real conversion/validation/preview covers existing rules, Go with unrelated findings, configured roots and nearest-marker nested Python. | No test-only validated-plan shortcut establishes this pass. |
| F-AC2 | Pass: parsing/probe failures, spaces/Unicode/CRLF, benign stderr, ambiguous/truncated/nonzero output, deadlines, absent tools and Python fallback are covered. | Native tool shims prove the finite supported contract, not every installed historical tool version. |
| F-AC3 | Partial: packaged unrelated-CWD/two-drive roots, overlap/deduplication/removal, persistence/recovery, distinct protection UI and actual denied-root plus healthy-project native/CLI equality pass. | One **same GUI report** still needs actual denied selected root, healthy explicit project and healthy isolated global provider. Separate project-only GUI and all-scope CLI reports cannot be combined into this pass. Functional owner is investigating safe isolation. |
| F-AC4 | Pass: incomplete zero-target versus truly empty, full report/diagnostic parity, partial CLI exit and all-skipped/result presentation are covered. | Execution UI scenarios use controlled reports/fake runners, which the acceptance allows. |
| F-AC5 | Pass: all finite required matrix families have positive/negative production pipelines or explicit inspect-only refusal; retained data and live drift are checked. | Windows owner-state seam and actual ToolHelp/SID/no-sharing helper evidence remain distinct; no real Cloud Files provider is claimed. |
| F-AC6 | Pass: mixed/command-only/inspect-only/recent/unknown/failure results, reviewed rescan and stale-preview invalidation are covered by core/component/adapter tests. | No claim that full owner-store occupancy equals prune yield or that Recycle Bin occupancy is physically freed space. |
| F-AC7 | Pass: tampering, same-path tool/config changes, marker/owner/protection drift and stale digest fail before fake side effects; shared services and wire parity remain intact. | Unix native branches were not executed on this Windows host. |
| F-AC8 | Pass for recorded Windows scope: Rust/frontend/wire/bilingual-reference gates, packaged C2 Clean and isolated native command-shim checks. | C3 changes shared sizing, so final affected parity/timing belongs to runtime; it does not invalidate unchanged functional proofs automatically. |

The exact F-AC3 gap is an observation gap, not proof of a product defect. It
requires no repeated two-drive case and no real cleanup. If safe native
isolation cannot be established, keep this clause unverified; do not lower it
to two unrelated reports. Acceptance with that exception would need an explicit
user decision. Provisioning an account/VM or changing machine settings would
also need separate scope. This audit grants neither that scope nor a broad
current-user cache scan.

## Earth criteria

| Criterion | Evidence disposition | Boundary or missing evidence |
| --- | --- | --- |
| UI-AC1 | Pass: original/candidate browser comparisons and C2 native 32-case locale/width/DPR matrix show exact backing sizes, geographic content and preserved font. | Actual Windows scale is 125%; remaining DPR values are emulated. No forced OS-scale switch is required by the PRD. |
| UI-AC2 | Pass: local licensed primary/fallback, exact source version discrepancy/checksums, 908790-byte total payload, 1024x512 primary and packaged offline rendering. | The stale acquisition note's earlier packaging TODO is superseded by native acceptance, not new unfinished work. |
| UI-AC3 | Component gates pass: cold p95 24.9 ms, zero observed cold tasks >=50 ms, warm 0.1–0.2 ms and 93.6% less median synchronous painted work. | Painted CPU/submission duration does not establish GPU completion, whole-app CPU or energy savings. Final paired native resource integration remains pending; do not relabel the 93.6% figure. |
| UI-AC4 | Pass: selected FPS/still behavior and zero recurring hidden/unmounted work are covered; warm 30-FPS gap count is 0/4316; native minimization supplies real visibility proof. | Ongoing requested backend work is intentionally separate from decorative suspension. |
| UI-AC5 | Pass: late decode/failure/context/resize/DPR/unmount tests and twenty actual native route/minimize cycles establish bounded observed resources and no stale restart. | Native DPR-only CDP changes emitted no resize/media events; actual cross-monitor/OS-scale transitions were not measured. Fresh-mount exact-DPR and deterministic observer tests retain their narrower claims. |
| UI-AC6 | Pass at the specified component endpoint: 100 Cancel events p95 7.0 vs 7.7 ms, 100 real AppShell navigation commits 7.0 vs 7.0 ms; absolute and relative gates pass. | Reset/preparation is excluded. This is event-to-post-commit presentation opportunity; native backend cancellation/join is reported separately. No extra native 100-event p95 matrix is required. |
| UI-AC7 | Source/prospective-spec/native evidence pass; later C3 product frontend assets remain unchanged by the sizing candidate, subject to final manifest verification. | Parent runtime integration handoff remains pending; old checkpoint counts must not be summed or mislabeled as new runs. |

## Runtime criteria

| Criterion | Evidence disposition | Actual outstanding part |
| --- | --- | --- |
| RT-AC1 | A, corrected B3 and C1/C2 identities, failed attempts, exact fixture/toolchain/host/protocol records and raw samples are preserved. | Record final C3 build/assets and completed paired measurement identities/argv/results; build success alone is insufficient. |
| RT-AC2 | Deterministic zero unused terminal snapshots, bounded publication/projection work, enumeration/budget/no-follow regressions and B3/C2 repaired-provider/native report equality pass. | Final C3 B3 parity and healthy A comparison remain assigned. A comparisons may omit only the coverage field absent from A, with the rest of the complete envelope/order/exit checked. |
| RT-AC3 | Eliminated-work counts and the two-worker sibling scheduling regression prove concrete avoided work; C3 source gate passes. | Final comparable Clean median <=1.20x, private <=+64 MiB and threads <=+2 require the planned matched samples. Timing diagnostics alone do not close this gate. |
| RT-AC4 | Pass at declared levels: frozen 1k/10k component interaction budgets, full selection/search retention, bounded lossless queues and native 3/6-second pause/reload/minimize/cancel/close. | O(1) refers to snapshot count, not payload bytes independent of result size. The capped 1k native completion attempt stays failed/unavailable; its cooperative cancel is separate evidence. No native 10k completion deadline is invented. |
| RT-AC5 | Structural TUI idle/burst fairness and bounded Status channel proofs pass; real slow pipe/broken pipe, named NDJSON terminal and JSON/create-new checks pass on C1/C2 CLI identity. | Native TUI resize is unavailable (WinError 87). Slow regular-file reader evidence is not artificially delayed filesystem writes; deterministic channel bounds are separate. Do not invent a native TUI p95 requirement or claim unobserved OS behavior. |
| RT-AC6 | C2 native decoration/active-work separation, twenty Earth/HUD cycles, no hidden HUD sampler/later events and wait-only fake adapters pass. | Final presentation comparison is assigned. If C3 only changes sizing, label retained Status/HUD/frontend evidence as C2 observations of unchanged subsystems, not C3 measurements. |
| RT-AC7 | C2 no-debug inherited host gate passes (0% observed median, 11.54 MiB peak private, 25 threads); component Analyze layout/commit and native Optimize/Analyze ACK/join pass. | **C2 Software p95 6265.962 ms fails 5000 ms.** Final C3 native Software must pass or remain failed. No diagnostic CLI improvement or relative p95 reduction waives the absolute limit. |
| RT-AC8 | Pass: five C2 same-PID Status cycles, each joined then 25 scheduled 200-ms samples; all final-five CPU samples 0%, baseline 0% +0.5 pp; thread floors 16/14/14/13/13. Clean cancel and wait-only boundaries are separately recorded. | These are unchanged-subsystem C2 observations, not permission to force-stop work or invent a universal Clean join deadline. |
| RT-AC9 | Latest canonical Rust gate passes 745+5 ignored; frontend/wire/component/native lifecycle checks retain their source identities. | Integrate final C3 comparisons with equality and every remaining failure/unavailable item before criterion closure. |

The runtime owner confirmed the already planned CLI set covers frozen many-100,
overlapping roots, 5000-file wide, deep and Unicode workloads from the original
7205-file fixture. Each workload retains first use, a discarded warmup and five
AB/BA alternating pairs, full B3/C3 envelopes and exits, healthy A comparisons,
200-ms root/tree samples, fixture pre/post hashes and normal owned-process exit.
This plan covers the outstanding Clean comparison axes without adding new
workloads. It is not completed evidence yet.

For short-lived processes, insufficient resource samples remain **unavailable**
with reason; they are not zero and do not establish a failed product. Do not
invent a longer sampling-duration acceptance threshold after seeing a short
run. Measured resource guards, unavailable rows and their scope must be reported
separately rather than silently turning the entire matrix into a pass.

## Remaining delivery risks and evidence boundaries

The final assigned slots cover the genuine pending acceptance work identified
here: F-AC3's combined GUI scope observation, C3 Software's unchanged absolute
gate, C3 equal-coverage Clean correctness/resource pairs and native Earth
still/animated comparison. This audit found no additional unassigned PRD-native
matrix necessary to make those results reviewable.

Native Windows remains the verified platform. Unix execution, real Cloud Files
providers, native TUI resize, actual OS-scale transitions and artificially slow
filesystem writes retain their stated unavailable/unperformed boundaries.
They must not be fabricated as passes; neither should they become unrelated
new native acceptance criteria. Real cleanup, account provisioning, OS setting
changes and broad user-cache scans are not implied by any missing measurement.

Before final delivery, refresh checkpoint-oriented summary identities where
they still call C2 the current candidate or say the C3 full gate is pending.
Preserve those underlying historical records and C2's failed Software samples.
Reuse unchanged subsystem evidence explicitly, verify C3 affected paths, and
keep component timings, native IPC/join, host-PID limits and full-process-tree
comparisons separate. No whole-tree 192 MiB/40-thread ceiling is authorized.

The C3 canonical gate's inherited `CARGO_TARGET_DIR=dist` failure and approved
test-child isolation are fully disclosed in
[independent-c2-check.md](independent-c2-check.md#c3-canonical-gate-and-fixture-isolation).
Future validation should use Cargo's explicit `--target-dir` argument under
the existing quality guideline. This audit does not request another equivalent
gate run and makes no task-completion or release-approval decision.

## Software measurement-method audit after the C3 failure

This later static audit read the approved PRD/design, historical resource
script, actual formal C3 driver and raw order/window records. It ran no app,
test, build or diagnostic. The formal C3 Software result remains **FAIL**:
five-sample p95 **5211.467 ms >5000 ms**, with complete inventory parity.
Coordinator-reported single-window phase diagnostics near 4.4 seconds are
neither an acceptance run nor proof of observer interference.

### Binding method versus harness choice

- Runtime PRD `RT-01`, `RT-AC1` and `RT-AC7` require identified comparable
  inputs/builds, cold/warm separation, raw evidence and the unchanged 5000 ms
  Software ceiling. `RT-08` prohibits threshold relaxation after a failure.
- Runtime `design.md:54–58` requires separate fresh-process/profile first use,
  a discarded warmup, at least five alternating baseline/candidate pairs,
  nearest-rank reporting, 200 ms root/owned-tree resources and matched CDP
  instrumentation. `design.md:62` says a measured miss is fail.
- Neither the PRD nor this design requires baseline and candidate applications
  to remain alive simultaneously. Co-residency is a harness choice, not the
  stated workload or an accepted additional two-instance performance target.
- Historically `tools/measure-resources.ps1:1215` feeds the Software gate from
  `Measure-CliWorkload`; lines 698–731 perform one discarded warmup then five
  sequential calls to `Measure-CliRun`, whose line 594 starts a fresh process.
  Line 1933 applies the 5000 ms limit. This establishes no dual-app requirement.
  It does **not** authorize replacing the now-declared native IPC gate with a
  faster CLI/core-only measurement or subtracting separately measured times.
- Current native operation timing uses actual UI input and CDP monotonic
  request/loadingFinished timestamps (`tools/native-operation-smoke.mjs:64–81`).
  Keep that endpoint for any prospective corrected native method. Core phase
  traces explain cost; they are not a substitute acceptance clock.

### Actual formal order and visibility

The preserved outer driver is
`dist/performance/cleanup-earth/run-native-software-c3-final-pairs.mjs`, SHA-256
`e0d145f4b04d1e136ca490de2fbe5420f34d0a134044cc79d494d85059745e7b`.
Lines 28–32 always run `['B3', 'C']` for first use, warmup and all five retained
rounds. The actual raw `dist/native-software-c3-final-01/progress.json` is thus
`B3,C` seven times. It is alternating builds, **not AB/BA order reversal or a
counterbalanced experiment**. Current formal native notes say "alternating"
without explicitly claiming AB/BA; future descriptions should state fixed
B3-then-C3 order. The separate CLI plan's AB/BA description is not a description
of this native run. No raw order or failed sample is rewritten.

Line 31 minimizes the inactive app and restores the sampled one; line 32 waits
300 ms. `measure-readonly.py:253–268` identifies the exact owned main window
and uses `ShowWindow(6)` for minimization. Its `windows()` uses `IsIconic`
(`:233–235`), so the retained samples establish actual native state rather than
only a requested action. B3 PID 148856 / HWND 51121238 has seven matching
minimized intervals in its `samples.jsonl`. The nearest raw window sample to
each of the seven C inventory observation timestamps (differences -43 to +62 ms)
has `visible:true,minimized:true`.

For the slowest retained C sample, the B3 minimized interval begins at raw
Unix ns `1791916293654264200` and the next restored observation is
`1791916303902336100`. The sample at `1791916298365569300` records minimized
true, near the C inventory timestamp `1791916298329` ms. These periodic state
observations are not continuous Win32 tracing, but they corroborate the exact
driver action. A control must preserve this actual minimized B3 state; making
the second app invisible by a different mechanism would change another factor.

Each launch controller independently calls `WindowsSampler.tree` every 200 ms
(`measure-readonly.py:412–422`). `tree()` takes a system process snapshot,
enumerates entries and inspects its owned tree (`:137–205`); desktop samples
also enumerate windows. Thus two controllers duplicate recurring observation
work. This is a concrete possible interference mechanism, not measured proof
that it caused the remaining 211.467 ms gate miss. Small IPC-minus-core time
also cannot exclude interference inside the core's I/O/scheduling work.

### Proposed bounded three-condition control, before execution

The following is a reviewable diagnostic proposal, **not an executed run or a
new acceptance threshold**. Freeze the exact instrumented native candidate
hash, phase-trace schema, B3 hash, driver hash, effective environment, profile,
route/media/viewport and sample schedule before launching it. Use one candidate
PID throughout, on the same ready Software route, restored/visible. Its own
CDP connection, phase tracing and 200 ms root/tree sampler remain identical in
all conditions. No source change, new product cache or observer adjustment may
occur between cells.

| Condition | Idle B3 application | B3 periodic resource observer |
| --- | --- | --- |
| S | Absent; all its owned descendants have exited normally | Absent/inactive |
| I | Ready Software route, no active job, actual native minimized state | Paused for the timed calls; ownership/lifecycle checks remain outside them |
| D | Same ready/minimized app and state as I | Active at the formal 200 ms cadence |

Keep the candidate's sampling present; a sampler-free candidate would not
isolate the extra observer and would cease matching the declared resource
protocol. Pause/resume only the diagnostic observer's collection, not a product
operation or a process. Keep its controller/lifecycle bookkeeping equivalent
between I and D and measure observer CPU/I/O deltas with common before/after
counters, rather than adding another recurring observer to one condition.
The fixed B3 preparation and any first inventory occur outside candidate timing
and remain recorded; OS cache coldness stays uncontrolled.

Predeclare exactly **S → I → D → D → I → S**. Each block gets the same five-second
state settle, one recorded-but-discarded candidate warmup, and two retained
actual UI inventories. This is **18 candidate calls: six warmups and twelve
retained measurements, four per condition**. The fixed within-block readiness
and pause policy applies equally to every call. Keep any initial fresh-profile
first-use observation separate and never add it to these twelve measurements.
If an identity, scope, phase correlation, semantic or safe-lifecycle prerequisite
fails, preserve the failed control and stop; do not replace unfavorable rows.

For every retained call save native IPC duration and same-operation core phase
spans, all inventory fields under the existing timestamp-only comparison rule,
counts/bytes/confidence/source statuses, window/job state and resource records.
Report each raw value and block summaries. Compare I-minus-S for co-residency
and D-minus-I for the second observer in both halves of the mirrored schedule.
The repeated D blocks help expose local variation; reversal prevents a purely
one-way warmup trend from being mistaken for an effect. This small diagnostic
does not establish a tail percentile or make any call under 5000 ms a pass.

Evidence for a correction needs a reproducible direction on reversal and an
effect distinguishable from within-condition variation, with unchanged complete
results and phase records consistent with the mechanism. If the sign reverses,
the spread dominates, or both differences are small, label the explanation
inconclusive/unsupported and retain the product investigation. Do not run extra
rounds until one appears favorable. Stop after this declared control set.

### Conditional correction and final decision boundary

A publicly documented method correction is permissible **if the control supports
an incidental harness effect**: dual residency is not mandated. Choose only the
factor the evidence supports. If only duplicate observation matters, a single
active measurement collector or shared 200 ms process snapshot can preserve
the original persistent-process workload and required measured-tree resources.
If co-residency independently matters, single-instance sequential native runs
are a possible correction. Neither outcome is established by this static audit.

Publish the rationale, control records, exact revised method and limits before
the new formal run. Both B3 and C3 must use that same method and retain the
actual native IPC endpoint, same installed inventory/coverage, power/media/
profile policy, 200 ms measured-tree resources and normal owned teardown. Freeze
five pair orders explicitly as **B3,C3 / C3,B3 / B3,C3 / C3,B3 / B3,C3**; five
pairs cannot be perfectly balanced, so disclose the 3:2 starting-order split.
Retain every sample and nearest-rank p95 over exactly five warm values per build.
The limit remains **5000 ms**.

For a single-instance version, do not silently describe each newly launched
process as a later sample of the old persistent PID. One concrete comparable
method is a fresh process/profile per side of each pair, separately recorded
first use, one discarded warmup, then one retained warm inventory before normal
close and complete descendant settlement. That is five retained warm values
per build plus separately retained first-use/warmup rows for all ten launches;
it changes process warmth semantics and must be stated explicitly. If only the
extra sampler is corrected, preserve persistent PIDs and the original separate
first-use/warmup structure instead.

Use uninstrumented fixed product binaries for the new formal gate, not the phase
diagnostic artifact. The instrumented control supports the rationale only.
Keep C2 and C3's original formal failures as valid observations under their
recorded method. A corrected-method pass, if obtained, is a new qualified result,
not evidence that those failures never occurred. If the control does not support
a correction, keep the current method and investigate the demonstrated MSIX
enumeration/sizing cost without weakening authority. If the declared corrected
formal run also misses, it remains failed; this audit proposes no repeated
attempts to find a passing tail and no threshold change.
