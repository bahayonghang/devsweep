# Earth renderer implementation evidence

Measured 2026-10-13. This is a production-browser component comparison, not
native application/process-tree or GPU-completion evidence. The child remains
in progress until integrated native acceptance and the full quality gate.

## Implementation

Clean now uses the pinned local Natural Earth map through a small dependency-free
WebGL1 renderer. The four other modes retain the original procedural Canvas
renderer. Source/output are bounded at 1024x512 and 512x512, with effective DPR
capped at 2. No cloud, network, texture framework or production package was added.
See [asset-source.md](./asset-source.md) for exact rights/conversion/checksums and
the [design decision](../design.md) for the measured Canvas-first investigation.

Local texture loading is asynchronous and cached once, including failures.
Missing/corrupt texture or missing/failed GPU context uses a local static Earth;
if that image also fails, an outline remains. Effective forced colors does not
load a texture. Hidden-first mounts defer texture/GPU work. A late decoded
texture cannot revive an unmounted renderer. Resize/DPR changes redraw still
poses without starting animation. All RAF, observer and event owners are
released; GPU texture/buffer/program/shaders are deleted. Detached canvas
contexts are retired; connected canvases remain reusable for React effect replay.

## Production benchmark method

Build: `desktop/vite.planet-benchmark.config.ts`. Baseline mode reads the three
original Planet source files from `e75a29d` with `git show`, without replacing
workspace files. Both builds mount the same real Stage/ScanPage and preference
context. Read-only progress replay uses the real OperationCoordinator and fixture
decoder; it invokes no native scan or cleanup operation.

Driver: `tools/planet-render-benchmark.mjs OUTPUT_DIR full baseline production`.
It owns one isolated hidden Chrome process and loopback server. The output stores
host, build/source hash, renderer, viewport, DPR and raw samples. Node 22.23.2 and
its bundled npm 10.9.8 were used. The baseline's full CLI/native/UI/source archive
was independently preserved by the runtime child before product edits.

Reference run: `evidence/dist/gpu-full/record.json`; 1440x900 CSS pixels, DPR2,
232px hero, Chrome 155, 24 logical processors. GPU reported ANGLE D3D11 over
NVIDIA GeForce RTX 5090 Laptop GPU. Browser media emulation disables OS reduced
motion only inside the benchmark so the animated case actually runs. An earlier
exploratory run inherited reduced motion and produced zero warm frames; it is
retained at `dist/canvas-quick/` and excluded from animated comparisons. The driver
now rejects insufficient warm sample counts.

Cold samples are 20 fresh document/React/module contexts; a 3s warm-up precedes
five 30s windows. Painted-RAF duration measures the synchronous callback which
submits a frame, including CPU rasterization for the original. It does not measure
GPU completion or whole-process CPU. Raw draw timestamps separately assess cadence.

| Measure | Original | Geographic GPU |
| --- | ---: | ---: |
| Cold first-frame p95 | 87ms | 47ms |
| Cold observed long tasks >=50ms | 20 of 20 (77-86ms) | 0 of 20 |
| Warm painted-RAF p95, five windows | 1.4 / 2.2 / 1.5 / 1.6 / 1.5ms | 0.2ms each |
| Median synchronous painted work per 30s | 873.9ms | 66.3ms |
| Gaps exceeding twice 30 FPS interval | 1 / 4311 | 0 / 4308 |

The median painted-work reduction is about 92.4%. This satisfies the intended
main-thread improvement on this host, but must not be reported as a 92.4%
whole-application CPU or GPU-energy reduction. Twenty unmount/remount cycles
recorded zero owned RAF handles and zero late draws after every unmount. Candidate
used JS heap cycled between 19.2MB and 20.1MB (decimal bytes), with no monotonic
growth. Browser GC and GPU resource accounting are separate from native private
bytes; the runtime integration must still measure those.

## Interaction result and protocol refinement

The initial 20-event asynchronous CDP click run had Cancel-to-post-commit-frame
p95 6.1ms (original) versus 7.0ms (candidate). Both pass the absolute 100ms budget,
but the candidate is 14.8% higher, so the initial relative 10% gate **failed**.
The raw samples are retained. A sub-millisecond difference between percentiles
with independent click/frame phases is insufficient evidence of its cause.

A predefined secondary check aligns each click to the same RAF boundary and
uses 100 events per build, reversing build order (candidate then original). It
keeps the same progress cadence, real controls and post-commit presentation
endpoint. This controls the frame-phase variable without changing the budget.
Both initial and secondary results must be reported; do not silently substitute
the more favorable run. The fixed 100-event secondary run measured 7.0ms
candidate p95 versus 7.6ms original p95 (7.9% lower): both absolute and relative
gates pass under that identical controlled protocol. The initial 20-event
relative failure remains above. Raw records are preserved in
[renderer-full.json](./renderer-full.json) and
[interaction-aligned.json](./interaction-aligned.json), including all individual
samples, ordering, build identity and host information.

The final renderer adds a React canvas key for mode changes after the full warm
run, so GPU/2D modes receive different canvas elements; this does not change the
measured Clean draw path. The secondary interaction build includes that change.

## Checks and integration handoff

- 26 focused Vitest cases passed (Planet lifecycle, bounded loader and GPU
  resource disposal). Stage/benchmark ESLint passed.
- Production baseline/candidate harness builds passed. The coordinator's later
  matrix builds also passed TypeScript after coverage DTO integration. Full
  desktop/repository quality gates remain owned by final integration.
- The inspected production screenshots show recognizable Africa/Europe and
  geographic ocean/polar coverage; no font/theme reset was made.
- The coordinator completed the locale/viewport/DPR browser matrix; see
  [visual-matrix.md](visual-matrix.md). Native offline/minimize/restore,
  process-tree CPU/private bytes and integrated responsiveness remain required.
- Matrix command after rebuilding the final benchmark:
  `node tools/planet-render-benchmark.mjs OUTPUT_DIR matrix production baseline`.
  It captures 32 locale/width/DPR cases plus light, forced colors, missing primary
  texture, both assets missing, unavailable GPU context and 15 FPS cases.
- Native evidence must use a Tauri CLI-built binary containing final UI assets.
  The runtime child owns native/build slots and the final process-tree comparison.
