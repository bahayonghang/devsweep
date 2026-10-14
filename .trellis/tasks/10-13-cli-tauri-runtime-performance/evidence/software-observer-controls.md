# Software observer and co-residency control

This one declared diagnostic does **not** support changing the formal method or
explaining away the fixed C3-final Software failure. The unchanged formal p95
is **5211.467 ms**, above the **5000 ms** gate. No extra rounds or replacement
samples were run. Values below are diagnostic observations, not a formal pass.

## Frozen protocol and identity

Raw evidence: `dist/performance/cleanup-earth/C3-native-profile/controls-01/`.
[Compact evidence](software-observer-controls.json) retains all block values,
setup timings, correlated phases, resource summaries, lifecycle results and
93 artifact hashes. `protocol.json` was written before either launch.
The ignored driver/runner/sampler copies leave the product and formal runner
unchanged. Candidate instrumented native SHA-256:
`b4d5f997df799392d252a5f99c7c4b582cdced5b23c3f751c57711a2ff59936c`.
B3 SHA-256: `667145cb762a73d0ed8ee900bd8a52620e3262b953aae865e411eed34dfa7c78`.

The frozen order was **S → I → D → D → I → S**, once. Each block had a 5 s
settle, one discarded inventory, then two retained inventories: **18 control
calls**. Candidate setup (4541.314 ms) and B3 setup (4769.358 ms) were separately
retained and excluded from those 18 calls. They are not hidden warmups.

- S: candidate only; B3 and its descendants absent.
- I: ready B3 actually minimized; only its periodic observer Event is cleared.
- D: the same minimized B3, with its 200 ms periodic observer enabled.

No application thread was paused. One candidate PID, its enabled 200 ms sampler,
one persistent CDP connection, actual Software UI actions and ready-page state
were retained throughout. The candidate remained visible and restored; B3 was
verified minimized before and after I/D calls. B3 closed and naturally settled
before the final S block. Effective launch environments record absent
CARGO_TARGET_DIR. All 20 full inventories match the formal C3 reference across
1100 entries, excluding only the two observation-time fields and their derived
fingerprint. Raw fingerprints and reports remain intact.

Presentation stayed at 1080×720 CSS px, DPR 1.25, English, the same recorded font,
reduced motion true and forced colours false. The recorded theme dataset was an
empty object; it does not independently identify a theme. The instrumented
binary and all presentation metadata were constant across the control calls.

## Observations

All values below are request-to-response milliseconds from actual Tauri IPC.

| Block / condition | Discarded warmup | Retained calls | Retained median |
| --- | ---: | ---: | ---: |
| 1 / S | 4349.453 | 4465.576, 4591.399 | 4528.487 |
| 2 / I | 4161.193 | 4324.953, 4290.228 | 4307.590 |
| 3 / D | 4717.479 | 4683.001, 4185.962 | 4434.482 |
| 4 / D | 4396.639 | 4399.113, 5120.794 | 4759.954 |
| 5 / I | 4465.258 | 4260.698, 4250.107 | 4255.403 |
| 6 / S | 4405.919 | 4451.325, 5216.881 | 4834.103 |

I minus S was **−220.897 ms** in the first half and **−578.701 ms** in the mirrored
half. Co-residency did not slow the candidate in either comparison. D minus I
was **+126.891 ms** and **+504.551 ms**, smaller than the D retained range of
**934.832 ms**. S's retained range was 765.556 ms and I's was 74.846 ms. These small
samples do not establish a separable observer correction; no significance or
noise bound is invented from them.

Every candidate report has all 13 native/core phase records, correlated by the
actual operation ID and inventory observation timestamp (19 reports including
setup; 247 phase rows). Retained MSIX sizing spans **3253.235–4092.252 ms** and
MSIX enumeration **856.435–1116.182 ms**. They account for most of the observed
latency and variation. ARP/MSI/MSIX sources run concurrently; assembly includes
fingerprinting. Overlapping phases must not be summed. A CDP-to-command remainder
includes serialization, transport and receiver work; it is not a pure IPC cost.

## Resource and interpretation limits

I records exactly zero B3 periodic samples. D records 21–26 B3 periodic samples
per retained call and **859.375–1375.000 CPU ms** spent in that observer's sampling
sections. The candidate observer remains active in every condition. Controller
CPU per retained call is 1109.375–1453.125 ms in S, 1109.375–1250.000 ms in I,
and 1937.500–2953.125 ms in D. B3's native host CPU delta is zero during all I/D
retained calls; some WebView descendants have nonzero deltas. Candidate host
private bytes after calls range from 9,609,216 to 12,001,280. Full candidate,
B3, Node-driver and Python-controller counters remain in the raw records.
CPU milliseconds sum work across threads and can exceed wall time.

The two sampler loops run as threads in **one Python process and share its GIL**.
The original formal comparison used independent Python controller processes.
I also adds the second controller's lifecycle polling. Thus these controls do
not isolate the original observer topology exactly. Common before/after
counters include boundary inspection; controller I/O includes both observers
and orchestration. Periodic per-thread CPU is measured separately, but per-thread
I/O is unavailable. Matched-tree deltas exclude new/unmatched process identities.

The supported conclusion is limited: this control does not justify a formal
method correction. Continue the product investigation of measured MSIX work;
preserve C2 and C3's formal failures and the declared 5000 ms requirement.

## Normal closure and checks

Candidate PID **149060**, FILETIME `134363919822884165`, and B3 PID **147856**,
FILETIME `134363920111532703`, each received one identity-checked WM_CLOSE and
exited 0. No forced teardown, timeout or owned leftovers occurred. Descendant
settlement was 247.152 ms and 1221.930 ms respectively. The native slot was released
only after both completed. Python compilation and Node syntax checks passed
before the single execution; semantic, phase, presentation, observer-gate and
lifecycle assertions all passed. No product source changed for this diagnostic.
