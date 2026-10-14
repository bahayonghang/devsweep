# Native read-only operation acceptance

## Current status

The fixed C3-final package repeats the formal Software gate and still **fails**:
its five-sample warm p95 is 5211.467 ms, above the unchanged 5000 ms limit.
All fourteen complete inventories contain the same 1100 entries after only the
specified timestamp/fingerprint normalization. This semantic pass does not
waive the timing failure. No additional rounds were added.

The earlier C2 Software failure, C2 Optimize/Analyze passes and inherited
Analyze component results remain below with their exact build identities.
The separate C3 packaged Clean smoke passed, but overall Software performance
acceptance remains open. A separate instrumented native diagnostic may identify
phases; it must not overwrite these fixed-package samples or infer IPC cost
by subtracting separately measured CLI time.

The completed [observer/co-residency control](software-observer-controls.md)
does not support changing the formal method. Its one predeclared S/I/D/D/I/S
run is diagnostic only; both prior formal failures remain valid.

## C3-final formal Software repeat

Raw output: `dist/native-software-c3-final-01/`. Compact values, semantic rules,
actual launch environment, normal-close results and 43 raw hashes are in
[native-software-c3-final.json](native-software-c3-final.json).
Candidate native SHA-256:
`7426037283a60829ee1d124bbd5433c6c0a6005cea307a9df425d1c98cfa85b6`.
The preserved B3 hash is unchanged from the C2 comparison below.

The sequence used new isolated app profiles: one separate first-use,
one discarded warmup, then exactly five B3/C3 Software pairs, always in the
fixed B3 → C3 order within each pair (not an AB/BA reversal).
The driver and nearest-rank p95 method remain the same; five-sample p95 is the
maximum. Every sample invoked actual native Software inventory through the
real UI. No build or parallel benchmark ran in the slot. Both processes had
CARGO_TARGET_DIR absent, verified at final Popen after the measurement helper
restored HOME/USERPROFILE. The effective allowlisted environment is retained
for each launch; pre-restoration fixture settings are not substituted for it.

| Software inventory | B3 | C3-final |
| --- | ---: | ---: |
| Separate first-use | 5680.413 ms | 5153.404 ms |
| Discarded warmup | 5110.175 ms | 5209.815 ms |
| Warm sample 1 | 5030.680 ms | 4636.347 ms |
| Warm sample 2 | 4695.406 ms | 5207.057 ms |
| Warm sample 3 | 6570.115 ms | 5211.467 ms |
| Warm sample 4 | 6001.531 ms | 5012.952 ms |
| Warm sample 5 | 5098.194 ms | 4671.259 ms |
| **p95 / 5000 ms gate** | **6570.115 ms / FAIL** | **5211.467 ms / FAIL** |

The full fourteen-report semantic comparison passed: 1100 entries and the
same eight source statuses, with every non-time field identical. Exclusions
remain only inventory observation time, size observation times and their
derived inventory fingerprint. Raw unmodified reports, fingerprints and UI
completion times are retained. These native responses do not contain phase
timings; the reason for the remaining failure is not established by this run.

- B3: PID 148856, FILETIME `134363897703622758`, main HWND 51121238,
  port 52821. Normal WM_CLOSE once, exit 0, descendants settled in 261.170 ms.
- C3: PID 149624, FILETIME `134363897827719449`, main HWND 20842692,
  port 52831. Normal WM_CLOSE once, exit 0; descendants were already gone at
  the first post-exit sample (0.006 ms recorded wait).

Neither process timed out, needed forced teardown, or left owned descendants.
The instrumentation and OS file-cache limits of the method below apply.
The formal slot was released after the normal exits. C2 failure and initial
C3 CLI diagnostic artifacts were preserved unchanged; no timing from those
separate environments was subtracted to label an IPC overhead.

## Historical C2 paired native evidence

Raw output is `dist/native-operation-pairs-01/`; compact values, exact launch
identities, normal-close results, semantic rules and 97 raw artifact hashes are
in [native-operation-c2.json](native-operation-c2.json). Each build used a fresh
isolated app profile and the packaged executable from an unrelated working
directory. B3 SHA-256 is
`667145cb762a73d0ed8ee900bd8a52620e3262b953aae865e411eed34dfa7c78`;
C2 SHA-256 is
`5ffaefe242808e9a2074880667209a0ece6facef1b15ab5f782adf36cc2f2a59`.

The executed order was one fresh-profile first-use observation, one discarded
warmup, then five alternating B3/C2 pairs for each operation. The other owned
window was minimized and the sampled window restored, with 300 ms before its
three actual UI operations. No builds or other benchmarks ran during this
slot. Remote debugging and process-tree sampling were enabled; this is an
instrumented IPC test, not the uninstrumented idle resource gate. OS file-cache
coldness was uncontrolled. Nearest-rank p95 over five samples is the maximum.

| Native operation | B3 warm p95 | C2 warm p95 | Absolute limit | C2 result |
| --- | ---: | ---: | ---: | --- |
| Software inventory | 6525.158 ms | 6265.962 ms | 5000 ms | **FAIL** |
| Optimize catalogue | 9.140 ms | 2.247 ms | 1000 ms | PASS |
| Analyze cancel acknowledgement | 3.169 ms | 1.953 ms | 500 ms | PASS |
| Analyze cancel request to joined result | 4.071 ms | 3.334 ms | separately observed | all joined |

C2 retained Software samples were 6265.962, 5205.179, 5221.152, 5467.660 and
5569.316 ms. B3 retained samples were 6083.550, 6525.158, 4741.301, 5528.818 and
4593.849 ms. First-use Software observations were 6807.152 ms (B3) and
7254.746 ms (C2); discarded warmups were 6655.958 and 4832.570 ms respectively.
Those discarded observations remain in the raw output and compact sidecar.

### Semantic comparison and measurement boundary

All fourteen Software reports contained 1100 entries with eight available
source statuses. The postprocessor compared every field after excluding only
inventory observation time, each size observation time, and the inventory
fingerprint derived from those timestamps. Everything else, including every
entry and source status, was identical across all fourteen reports. The
fingerprint implementation explicitly includes inventory observation time
(`software/mod.rs`, `inventory_fingerprint`); raw fingerprints remain retained.
All fourteen Optimize reports contained the same eight catalogue entries;
only the per-request operation id was excluded from full comparison.

All fourteen Analyze attempts used the identical task-owned `many-1000`
fixture, acknowledged the same operation id, returned `canceled`, reached the
canceled UI state and joined the worker. Partial snapshots contained 145–413
nodes because cancellation arrives after different amounts of traversal.
Their node counts/bytes are retained, not required to be byte-identical.
There were no snapshot warnings. No uninstall, maintenance, cleanup, preview,
reveal or trash action ran.

The actual Software IPC response has observation timestamps, source statuses,
entries and fingerprint, but no per-ARP/MSI/MSIX stage durations or counts.
The existing launch `samples.jsonl` files retain whole owned process-tree
CPU/IO observations; they are not an internal-stage profiler. No new profiler
was run to explain or replace the failed gate.

### Owned lifecycle

- B3: PID 110828, FILETIME `134363874336786742`, main HWND 37359408,
  debug port 52711. One normal WM_CLOSE, exit 0; descendants settled in
  1262.653 ms, no timeout, forced teardown or owned leftovers.
- C2: PID 18608, FILETIME `134363874463087772`, main HWND 12260772,
  debug port 52721. One normal WM_CLOSE, exit 0; descendants settled in
  1261.138 ms, no timeout, forced teardown or owned leftovers.

Both returned `lifecycle_pass=true`; their separate launch folders preserve
`launch.json`, `explicit-close.json`, `summary.json` and `samples.jsonl`.
The timed slot was released immediately after both normal exits. Remaining
Status/HUD, CLI and Earth process-resource comparisons are separately owned.

`tools/native-operation-smoke.mjs` performs exactly one actual UI operation per
invocation: Software inventory, Optimize catalogue listing, or Analyze start
and cancellation. It verifies the launch PID and exact FILETIME, and verifies
the loopback debugger listener belongs to that process tree. It attaches only
to the packaged main `tauri.localhost` page. No bridge function or backend result
is replaced. No uninstall, startup mutation, maintenance, reveal, trash, preview
or cleanup action is requested.

The driver observes actual Network IPC request/response bodies. Duration uses
CDP monotonic `requestWillBeSent` and `loadingFinished` timestamps. Analyze
records cancellation acknowledgement separately from the terminal start result;
the current Tauri command awaits the blocking worker and releases the running
slot before returning that result. A completed analysis before cancellation is
an unavailable cancellation attempt, never a passing zero-latency sample.

## B3 mechanics evidence

Generated records: `dist/native-operations-b3-mechanics-01/`, with operation
directories ending in `-software`, `-optimize`, `-analyze`, and `-analyze-repeat`.
The exact executable SHA-256 is
`667145cb762a73d0ed8ee900bd8a52620e3262b953aae865e411eed34dfa7c78`.
The app used PID 18480, FILETIME `134363799837024499`, debug port 51887 and its
own profile, starting from unrelated `dist/native-clean-cwd`.

| Mechanics sample | Observed actual IPC duration |
| --- | --- |
| Software first inventory, 1100 entries | 7312.102 ms |
| Optimize catalogue | 2.225 ms |
| Analyze cancel acknowledgement 1 / 2 | 2.049 / 1.474 ms |
| Cancel request to joined result 1 / 2 | 2.855 / 2.409 ms |

These are single mechanics observations during concurrent implementation/build
work. Software first-use exceeded the inherited 5000 ms threshold; it remains
recorded without being relabeled as a warm p95 result or a performance pass.
Analyze used only `dist/performance/cleanup-earth/fixtures/many-1000`. The second
same-process attempt successfully started after the first returned and then
canceled and joined, establishing repeatability of the driver.

`explicit-close.json` records one normal WM_CLOSE. The controller exited 0,
without timeout or forced teardown; no owned process remained. WebView helpers
settled naturally in 1449.981 ms, under the five-second bound.

## Final repeated method

Keep one separate first-use observation for each fresh profile; OS file-cache
coldness is uncontrolled. Discard one warmup per operation/build, then alternate
B3 and C for at least five pairs using the same driver and fixture. Preserve
every raw IPC record and result; compute nearest-rank p95 (five samples means
the worst sample). Software inventory budget is 5000 ms; Optimize catalogue is
1000 ms; Analyze acknowledgement is 500 ms and every attempt must join.

Do not run these timing samples during other benchmarks or builds. Native
resource sampling and Status/HUD acceptance remain with the runtime owner.

## Inherited Analyze component measurements

The frozen C2 component build passed both inherited gates in
`dist/performance/cleanup-earth/analyze-component-C2-measured-01/`:

| Measurement | p95 | Gate | Other bounds |
| --- | ---: | ---: | --- |
| Node product layout | 1.8941 ms | 50 ms | 513 rectangles |
| Production-browser React commit | 8.4 ms | 100 ms | 513 rectangles; 764 DOM elements (limit 900) |

Each used five discarded warmups and thirty retained samples, nearest-rank
p95 at rank 29. Both used fixture hash
`sha256:42685e0971165c2b59362cb44f251d4699e282bc647fd2f407d85559830ef37f`.
The 226 source/public/i18n inputs and seven production build artifacts were
verified against manifest SHA-256
`aa72a3bc2a780d4502d3af5ebf543fd7a6e57885667a4e1a1119aaf01f07c575`
before execution. These are frozen C2 component measurements, not native
WebView rendering or IPC latency. All raw samples, source/build hashes and
browser console records remain in the measurement directory.

The owned browser was Chrome 155, PID 142104, isolated profile
`devsweep-react-commit-4XoQJs`, debug port 13348. `Browser.close` through its
own reported endpoint ended the root browser process normally with exit 0;
no force termination or profile deletion was performed. Node was 22.23.2.

The inherited layout gate is a Node call to the real `layoutTreemap`, using
10,000 deterministic children, 1200 by 700 bounds, five discarded warmups and
30 samples. `tools/analyze-layout-benchmark.mjs` preserves the historical
fixture and nearest-rank calculation from `measure-resources.ps1`, with source
and fixture hashes. Limits remain 513 rectangles and p95 at most 50 ms.

The inherited React gate uses the existing production Vite
`desktop/src/modes/analyze/render-benchmark.tsx` harness and real AnalyzePage,
with five warmups and 30 commits, p95 at most 100 ms, 513 rectangles and 900 DOM
elements. This is a production-browser component measurement using the harness
fixture bridge, not native IPC or native WebView commit timing. No production
instrumentation is added to imply otherwise.

`tools/run-analyze-react-commit.mjs` now closes its own browser using the owned
debug endpoint's `Browser.close`, waits for process exit, retains the unique
profile, and treats a failed component budget as failure. It no longer force
kills the browser or recursively deletes its profile. The historical large
resource script is not executed.
