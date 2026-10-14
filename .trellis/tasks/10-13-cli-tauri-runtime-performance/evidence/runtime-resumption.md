# Runtime resumption and corrected reference

The 2026-10-13 resumed session verified original A CLI, desktop and source archive
hashes against `baseline-a.md`. The deterministic fixture manifest still records
7,205 files and identity
`4067adeef4c76e4d6abc7bf8b3c09142b7219f315d8d9c199a9b393836898212`.

The functional agent froze source after the independent-review regressions.
B2 was captured after the first review fixes, then superseded by the Yarn
configuration finding below. A task-local capture
script at `dist/performance/cleanup-earth/capture-b2.py` archives actual working
tree source, including untracked modules, and checks every captured input after
each build. Tauri generated schemas are excluded because they are build outputs.
The archive has 282 files and SHA-256
`d3515af0f3407730cdb95371260b573d6e6efba8bc70ce3c50036e506c8890b4`.

The locked release CLI is preserved at
`dist/performance/cleanup-earth/B2/devsweep.exe`, SHA-256
`a2de05a857a505a30231fe62dce6a6b8b368117882f547c555164ded001a68cb`.
The build used workspace-local `CARGO_TARGET_DIR=dist` and completed in 1m29s.
Its post-build verification matched all 282 source files and the 11 frozen
frontend B asset files. A preliminary verification/copy command used the wrong
working directory and failed before copying; rerunning it at repository root
succeeded. The failed command changed no source or reference binary.

Native B2 embeds frozen frontend B, including its geographic Earth, using the
relative `frontendDist` override
`../../dist/performance/cleanup-earth/B/frontend-native`. Current optimized C2
frontend source is deliberately not compiled into this reference. Native source
verification matched all 282 inputs and all 11 frozen assets. The locked native
build completed in 4m03s, with preserved executable SHA-256
`6b1a2cdfac86b9770a9b6712db951638815cfa1b16db122050f5542531534fb6`.
The permitted B2 precheck loaded `http://tauri.localhost/#/clean` with offline
Earth ready at actual DPR 1.25. It closed normally with exit 0 and no leftover
owned processes. This superseded packaging check does not open the production
optimization gate.

During the native build, independent review found another Yarn Classic
configuration-search authority gap. `B2/superseded.json` and the native build
manifest explicitly keep the runtime gate closed. The build completed and
source hashes were verified before releasing the source freeze and Cargo slot.
No backend performance product edits began against B2. A corrected B3 is
required after the functional repair; B2's packaging precheck remains useful
but does not waive that functional gate.

## Narrow implementation boundaries

- Functional agent owns core scan/project publication and enumeration plus
  CLI/TUI/Status backpressure and redraw fairness.
- Runtime agent owns shared sizing, Software measurements, Tauri progress,
  HUD lifecycle and native paired measurements.
- Native acceptance agent owns packaged-origin/full-report parity and final
  Earth visibility/scaling/offline validation.

Sizing will stream nested names and batch root names while keeping the existing
two workers, fresh child budgets, singleton reviewed-size behavior, no-follow
inspection and noncanceled warning semantics. Deterministic iterator regressions
must prove bounded retained names and cancellation during enumeration.

Tauri will retain one trusted raw progress snapshot. Core-owned identity
validation still runs on each input; the display-only projection runs only when
publishing. Existing WebView decoding remains unchanged. Read-only inspection
also found an existing ordering defect: sending a newer eligible snapshot did
not clear an older pending one, allowing phase/terminal flush to regress the
preview. A regression will cover both flush paths. This intermediate publication
repair is separate from final-report B3/C semantic parity and timing benefits.

Software MSIX remains a measured failed gate until repeated release evidence
passes. Existing counters found 64,610 unique native entry inspections, so a
duplicate-path cache or extra worker pool is not justified. Native Unix behavior
is not exercised by this Windows session.

## Lifecycle instrumentation

The read-only harness can inspect Windows thread descriptions for an exact
owned PID/creation identity. It verifies the thread's current owner after opening
the thread handle and rechecks the process identity after enumeration. This
supports proving that the named HUD sampler exists once while visible and is
absent after hide/join. It does not infer sampler completion from a button label.
The nine `tools/tests/test_measure_readonly.py` tests pass, including process
identity rejection and real current-process thread enumeration.

`tools/native-status-lifecycle.mjs` accepts optional `--hud`. It preserves the
five same-process Status stop cycles and adds combined live Status with twenty
real tray HUD show/hide cycles. It observes actual callback values without
replacing them, requires a real snapshot for each visible cycle, checks named
sampler disappearance and zero later HUD events, and records raw facts. Node
22.23.2 syntax checking passes. Native measurements must still establish the
behavior; script existence is not acceptance evidence.

## B3 reference and diagnostic boundaries

After the final Yarn Classic search-authority repair and its focused gates,
backend source was frozen again. The B3 archive contains 282 inputs, SHA-256
`271ae722c629919ae4573765417fb5ac0fb1af35ec037262f4698a087d16e18f`.
The locked release CLI built in 1m20s and is preserved at
`dist/performance/cleanup-earth/B3/devsweep.exe`, SHA-256
`1cf858d35b1be1cddaaae6284b605e89b6055024d7162121c09531b5fdc91266`.
All source inputs and the 11 frozen B frontend assets still matched after the
CLI build. Native packaging uses the same relative frontend override and the
explicit Node 22.23.2 Tauri entry; the current final C frontend is not rebuilt
into B3. The native release built in 3m46s and is preserved at
`dist/performance/cleanup-earth/B3/devsweep-desktop.exe`, SHA-256
`667145cb762a73d0ed8ee900bd8a52620e3262b953aae865e411eed34dfa7c78`.
Post-native-build verification again matched all 282 source inputs and all
11 frontend assets. Native validation passed in
`dist/native-clean-b3-01-smoke/result.json`: packaged origin, offline primary
and blocked-primary fallback, explicit C/D roots with Unicode/spaces, root
deduplication/persistence/removal, actual keyboard Scan and full report equality
with the B3 CLI (two targets, 50 verified bytes). Normal close exited 0 with no
forced termination or leftovers. The coordinator released production performance
edits only after this result.

Two B2 lifecycle diagnostics are retained, with Cargo overlap explicitly
disqualifying them from performance acceptance. In `B2-status-hud-precheck-01`,
five Status cycles each observed cancellation acknowledgment, joined command
settlement, terminal sequence and no later events; explicit live monitoring
continued while minimized. The subsequent HUD check timed out before proving
a visible sampler. The owned process closed normally with exit 0 and no
leftovers. Native helper subprocesses now use `windowsHide: true` to avoid
creating focus-stealing console windows, but the HUD failure's cause remains
unconfirmed. `B2-hud-precheck-02` exited before the driver attached and therefore
ran no HUD cycles; its zero exit code is not HUD lifecycle acceptance. Neither
diagnostic justifies a product HUD change or a CPU/latency pass claim.

## First implementation checkpoint

The Tauri forwarder now retains one raw snapshot and validates every input;
projection occurs only on publication. A 100-input burst test proves two
projections (first/latest). New regressions cover stale pending replay after an
immediate newer preview, both phase and terminal flushing, and malformed input
that a later valid preview would otherwise supersede. All 11 focused Tauri scan
tests passed (`dist/performance/cleanup-earth/tauri-scan-focused.txt`).

Sizing now visits nested names as they arrive and batches at most 64 root names
on the unchanged two-worker pool. Deterministic iterator tests prove the retained
name bound, cancellation after seven of 10,000 names, fresh child budgets across
the final singleton chunk and unchanged nested budget warning counts. The
functional owner's combined verification passed 21 sizing tests and 77 scan tests.

## HUD first-show diagnosis

The unchanged B3 binary again failed the first-show test after helper consoles
were hidden. A second callback on the same PID kept the existing HUD visible
with exactly one sampler for two seconds. A separate run with no CDP captured
OS WinEvent ordering in `B3-hud-event-trace-01/hud-os-events.json`: creation at
6.8 ms, focus events at 98/104 ms while hidden, show at 115.8 ms, hide at 151.8 ms.
Both owned processes closed normally with no forced termination or leftovers.

The pinned Tauri 2.11.5 `WebviewWindowBuilder::focused` controls both window and
WebView initial focus. The default true reaches Wry 0.55.1's creation-time
`controller.MoveFocus`; `visible(false)` alone does not disable it. The hidden
HUD therefore acquired focus during construction, and a queued blur could hide
its later show. The narrow candidate change creates it with `focused(false)`;
the existing explicit focus after show remains. The actual first-show and
twenty-cycle native driver is the regression check. Candidate runtime verification
is pending, so the issue is not yet claimed resolved. No sampler-cache redesign
or blur-dismiss policy change was made.

## Native consumer retention and acknowledgment

`tools/native-scan-consumer.mjs` observes actual scan IPC and unchanged channel
callbacks in an owned packaged WebView. On the B3 100-project fixture, pausing
the renderer for three seconds yielded 20 progress callbacks / 558,150 bytes
within the first 100 ms after resume. A six-second pause yielded 26 callbacks /
829,408 bytes. Complete scans delivered 83 / 81 callbacks and 4,480,454 /
4,424,729 bytes respectively; last progress arrived 33 / 19 ms after the
backend command response. Both complete reports contained 100 targets and both
owned launches closed normally with no force or leftovers. Raw records are
`B3-scan-paused-3s-ipv4/ui/result.json` and `B3-scan-paused-6s/ui/result.json`.
Cargo overlap excludes these diagnostic durations from performance acceptance.

The separate failed port-9469 setup was an IPv6-only owned listener; IPv4
refused and the IPv6 client was denied. It ran no scan and was normally closed.
Using an OS-allocated available IPv4 loopback port resolved setup. A later
attempt to reuse the three-second launch after its scheduled close also ran
no scan; the six-second record instead uses a fresh process/profile.

These observations and the pinned Tauri channel source establish downstream
retention; one raw backend slot alone was insufficient. The candidate adds
`scan_progress_ack(scan_id, sequence)`. A worker sends one packet and waits on
a condition variable until the frontend has closed-decoded and processed it.
Cancel, main-window destruction and exit wake the waiter without another send.
The main event loop never waits. The final scan result follows the final ack;
future/zero acknowledgments fail, duplicate acknowledgments are harmless, and
stale/completed scan IDs have no effect. All 14 focused native scan tests pass,
including delayed exact acknowledgment, final settlement, cancel/renderer loss,
send failure and earlier projection/order regressions. Root owns matching
frontend/invoke inventory and page-load cancellation fallback.

Candidate native paused/reload/minimized/cancel/close behavior is still pending.
The upstream private Tauri queue may retain one previously enqueued packet after
a full renderer reload; this work does not claim that private application-wide
storage is purged. Ordinary route changes retain the renderer and consume or
cancel its existing channel normally.

## Frozen integrated candidate C1

After the independent sizing cancellation and observer-ranking fixes, canonical
`just ci` passed on the frozen source (`dist/just-ci-integrated-review.txt`).
The final frontend gate passed lint/typecheck, 432 Vitest tests, five Node tests,
production build and generated checks. Candidate capture preserves 452 backend,
frontend and build-input files plus 11 production assets under
`dist/performance/cleanup-earth/C1-runtime`; benchmark output is not packaged.
The source archive SHA-256 is
`fdc82564b080d44146da0e5fb2e627d984fe3ec8cee7f42bbc0da6ea9b624215`.
The copied main/state assets are `main-3IVAegRC.js` / `state-Kqc0RST2.js`.

The locked CLI release build completed in 1m04s. Its preserved executable hash is
`a5881ebf98eed803581619db2cf0cb967093cf0151887c67b274511047701277`.
All 452 source inputs and 11 copied assets still matched after this build.
Native release packaging completed in 3m01s with unchanged source/assets. The
preserved desktop hash is
`8f917025fefc66e40bb7a04591cce1b092b4901ae4597c161778fe07381efc72`.
Full native Clean and performance acceptance remain separate pending checks.
The read-only ownership/lifecycle tooling separately passed nine sampler tests
and three Status-summary tests; these do not substitute for native execution.

`C1-hud-sanity-01/ui/hud-result.json` passed the actual first-show regression and
20 show/hide cycles. Each visible cycle had exactly one named HUD sampler and
an actual snapshot; each hidden/joined cycle had zero and no later publication.
Explicit main-window Status remained active throughout the HUD exercise and
then canceled/joined. The owned host exited normally with code 0; no forced
teardown or owned leftovers remained, and WebView helpers settled naturally in
1.425 seconds. This establishes the narrow `focused(false)` repair against the
preserved B3 event-order failure. It does not claim CPU improvement from the
instrumented lifecycle trace.

The inherited idle protocol was verified directly at
`tools/measure-resources.ps1:1464-1477,2056`: one PID, eight seconds startup,
a discarded 30-second settle plus 60-second sample window, then five measured
30-second settle plus 60-second windows. The candidate will retain that exact
548-second protocol with no debugger, median of five CPU medians, and maximum
private bytes/threads across all retained samples. The limits remain one
logical-core CPU percent, 192 MiB private bytes and 40 host threads. This is
separate from CDP lifecycle samples and is not a comparative idle speed claim.
The native acceptance observer also confirmed this host currently reports
`prefers-reduced-motion: reduce`; the default Earth is a still frame. The
uninstrumented idle run preserves that real setting. Animated Earth acceptance
uses explicitly labeled per-WebView media emulation and changes no OS setting.

## C1 native findings before final timing acceptance

The native Earth checker confirmed a minimized Windows HWND while WebView2
continued reporting `document.hidden === false`. In the preserved diagnostic,
the renderer drew 44 frames and ran 217 RAF callbacks over 1.506 seconds after
actual native minimization. This is an application lifecycle defect, not a
test passed by changing the visibility assertion. The C2 repair publishes a
plain boolean `main-presentation-visibility` event to the main window on native
resize/focus events, based on `!is_minimized && is_visible`; loss of focus alone
does not suspend presentation. A main-only read command supplies initial state.
Only decorative rendering consumes this state; domain operations keep running.

A separate short no-debugger C1 run (`C1-idle-diagnostic-01`) found 47 host
threads in all 74 samples after 30 seconds, with peak private bytes 14,249,984.
The native thread-name observation counted 24 `tokio-rt-worker`, 20 unnamed,
and one each main/Mojo/EB worker. It exited normally after 45 seconds, with no
force or leftovers. This is a diagnostic, not the full inherited idle gate.
The pinned Tauri 2.11.5 default runtime calls Tokio `Runtime::new()`, which uses
the host's 24 logical CPUs for async dispatch. C2 caps async workers to
`min(available_parallelism, 4)` while retaining the existing separate blocking
executor and runtime through the native event loop. Tokio 1.53.1 was already
locked/transitively present; only the desktop direct dependency entry was added
to access its builder. No package/version or blocking-pool limit changed.

C1 remains immutable evidence, including both failures. C2 must be captured and
rebuilt, then repeat native minimization, active-work continuation and the full
548-second uninstrumented idle protocol before any final resource pass claim.

## Frozen C2 build

The C2 canonical `just ci` and independent source review passed. Desktop has 92
passing tests and two existing ignored diagnostics, including the actual bounded
runtime/blocking-executor isolation test. A first desktop run caught the new
module missing from `service_boundary::ADAPTER_SOURCES`; its failed log is retained,
the source inventory was updated, and all tests/Clippy then passed. Frontend C2
passed lint/typecheck, 438 Vitest tests, five Node tests, build and generated checks.

`C2-runtime/source.tar` contains 453 frozen source/build-input files, SHA-256
`5420ec3087c8b4b61514cc3289484dccd5c0ee149a07661acbf45add8a5ec566`.
Its 11 copied production assets include `main-Q-4uoIpp.js` and
`state-itqXXLsR.js`. Locked CLI build verification produced the same CLI bytes as
C1 (`a5881ebf98eed803581619db2cf0cb967093cf0151887c67b274511047701277`).
The native build completed in 2m30s and is preserved with SHA-256
`5ffaefe242808e9a2074880667209a0ece6facef1b15ab5f782adf36cc2f2a59`.
All 453 source inputs, the exact input file inventory and all 11 asset bytes
matched after building. The frozen copy is separate from subsequent component
benchmark output. First debug-enabled C2 idle observation found 24 host threads;
this is diagnostic only and does not replace the inherited uninstrumented gate.

## C2 acknowledged native progress diagnostics

`C2-scan-consumer-01` completed both 3-second and 6-second paused-renderer
scans on the unchanged 100-target fixture. The full report in each run equals
the B3 report in every field. C2 received 4/5 packets (182,829/272,684 bytes),
with exactly one packet in the first 100 ms after each resume. Every packet
had its exact POST acknowledgment; the last received progress preceded the
completed command response by 6 ms. Neither command settled while paused.
The original per-run count accidentally included OPTIONS preflights; raw
requests and assertions remain intact, and `ack-post-summary.json` explicitly
corrects the counts to 4/5 without replacing the original evidence.

Reload attempt failures are retained. The first immediate fresh scan returned
`scan_already_running` while the canceled predecessor was joining. A subsequent
1k attempt accidentally restored the default global toggle after reload and
is invalid for fixture-only acceptance. The harness now reapplies the UI scope
and asserts every actual request's roots/project/global fields. The corrected
1k attempt accepted a new project-only scan after six busy retries and
continued delivering acknowledged progress. Its completion exceeded the
harness's 180-second observation cap, so it is not a completion/performance pass.
The real Cancel button then returned to a non-selectable canceled state after
13.292 seconds (at 203 observed targets); this is a measured cooperative join,
not a universal latency guarantee. The initial root-entry readiness race is
also retained; the harness now waits for the enabled Add control.

All three owned diagnostic apps exited normally with code 0 and no forced
teardown or owned leftovers. Their helpers settled naturally in 273 ms,
241 ms, and 1.209 seconds respectively. Complete reload/minimize/close
acceptance is repeated on the 100-target fixture rather than relabeling the
long 1k attempt as completed.

`C2-scan-consumer-05` subsequently passed all four real native lifecycle modes
on the unchanged 100-target fixture. Reload preserved three transient
`scan_already_running` responses before the predecessor joined, then accepted
and completed the new scan. Actual native `IsIconic` minimization retained
advancing progress/acknowledgments and completed normally. Both complete reports
equal B3 in every field. Real UI cancellation returned `canceled`, and an active
scan closed through normal WM_CLOSE with host exit 0, no forced teardown and no
owned leftovers. Each request has a saved label-based Projects/Global caches
preflight plus exact wire-scope validation; tests never inferred scope from a
button's position. Attempt 04 is retained as a harness failure: after a busy
rejection the existing failed-scan stage hides scope controls behind Show
details. The corrected driver follows that real control before its preflight.

After C2 capture, the root agent extended only the separately built navigation
benchmark entry/tool. The frozen archive and packaged production inputs remain
C2; the current full workspace inventory is not claimed to match the archive.

The final inherited no-debugger idle run passed at
`C2-idle-final-01`: all five retained windows held292–293 samples, host CPU
median-of-medians0.0%, maximum private12,103,680 bytes and maximum25threads.
The separate tree peaks were227,717,120 bytes,267threads and7processes, with
integrated per-window CPU0–0.0262%. The same owned host exited0 after scheduled
WM_CLOSE; helpers settled in257.8ms, without force or leftovers. Exact
protocol, counters and interpretation are consolidated in
[the acceptance ledger](runtime-acceptance.md).

## Final C3 candidate checkpoint

C2 later failed the actual native Software gate at 6265.962 ms p95. A bounded
deterministic regression identified independent siblings retained in a slow
Rayon fold. The final candidate changes only two production scheduling
statements in shared sizing; it preserves two workers, metadata/no-follow
behavior and coverage budgets. Test-only gate and environment isolation fixes
were independently reviewed. Latest `just ci` passed 745 tests with five
existing ignored diagnostics; the initial full-gate fixture failure is retained.

`C3-final` is a new immutable archive, not a renamed C2 or initial C3 diagnostic.
Source SHA-256 is
`1bce947fad5ef895a211621217b0436d265308d57b59912b1aa6a96701f984b3`;
CLI is `3c44b362b2d0a6b8440a41e66cca36f2613395f695bbeb6b7c68ebd9ecc9a67a`;
native is `7426037283a60829ee1d124bbd5433c6c0a6005cea307a9df425d1c98cfa85b6`.
All 453 source and 11 frontend asset hashes matched after release builds.
`equivalence-to-c2.json` explicitly identifies unchanged subsystems and the
separately built navigation benchmark extension. New native acceptance and
affected Software/Clean comparisons are pending at this checkpoint; details
and every initial diagnostic remain in
[the C3 scheduling evidence](software-c3-scheduler.md).
