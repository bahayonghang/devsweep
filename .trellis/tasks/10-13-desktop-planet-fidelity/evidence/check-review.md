# Independent Earth and desktop boundary review

Reviewed 2026-10-13 by the dispatched Trellis check agent. Intended task:
`10-13-desktop-planet-fidelity`; the hook's runtime-task pointer was not used
as the review scope. The parent PRD, this child's PRD/design/implementation
plan and check manifest, desktop specifications, and existing asset/render/
visual evidence were read before checking. No native build or cleanup ran.

## Findings (fixed)

- `desktop/src/stage/earth-renderer.ts:94` and `earth-renderer.test.ts`:
  initialization accepted a GPU buffer/texture upload which set a WebGL error
  without throwing. Planet would then regard the renderer as ready and hide
  the bundled still even though the GPU allocation had failed. A single
  initialization-only `getError` check now takes the existing disposal/null
  fallback path. There is no error query in `draw`.
- The added OUT_OF_MEMORY regression failed before the fix (a renderer was
  returned instead of null) and passed afterward. It asserts release of the
  texture, buffer, program and both shaders, no draw, and context retirement
  for a detached canvas. The successful-render test also asserts exactly one
  error query across initialization, multiple draws, disposal and a later draw.
- Error-state semantics were checked against the browser API documentation:
  [WebGL getError](https://developer.mozilla.org/en-US/docs/Web/API/WebGLRenderingContext/getError).
  No speculative context-exception wrapper or per-frame GPU synchronization
  was added.

Reviewed renderer file SHA-256 after the fix:
`5b15309fbcfd9262aef96474f2f1be8b7ca613b8a8aa3c7d7f825f0b7f8806ce`.

## Findings (not fixed by this reviewer)

These two local findings were sent to the runtime implementer, who owns Clean
files. They are outside this reviewer's edit ownership; they are not waived.

- `desktop/src/modes/clean/styles.css:14`: the new root fieldset uses literal
  `border-radius: 12px`. Use `var(--radius-card)` to follow the existing visual
  contract. The current radius test inspects shared CSS and misses Clean CSS.
- `desktop/src/modes/clean/coverage-results.test.tsx:38`: the all-skipped case
  changes a decoded report to `selected=1`, `attempted=0`, `skipped=1` and one
  outcome. The real wire invariant requires attempted=1. Correct the count and
  decode the final fixture shape so the result presentation test does not
  depend on a report production could never emit.

No further confirmed Earth defect or cleanup-authority leak was found in the
reviewed paths. Native acceptance and whole-programme performance work remain
open, as detailed below.

## Reviewed behavior and evidence

- `Planet.tsx` keeps texture readiness and renderer ownership local. Hidden
  first mounts defer primary decode/GPU initialization; visibility suspends the
  frame owner. Late completion checks disposal. Reduced motion has no recurring
  RAF and actual size/DPR changes redraw the still. Forced colors does not load
  the primary texture. Mode changes replace the GPU/2D canvas. Unmount removes
  listeners/observers and releases owned GPU objects; connected canvases remain
  usable during React effect replay.
- The primary and static fallback use local `BASE_URL` asset paths. The primary
  promise coalesces decoding and remembers failures. Independent PNG-header,
  size and SHA-256 checks match the distribution notice and source record:
  primary 1024x512, 658001 bytes, SHA-256
  `de1ded3b671bb14c3a518dbf4f2b932eb2fb0987621969df88a4d7e8036f8a60`;
  still 512x512, 250789 bytes, SHA-256
  `1826f0bd821a838717f4cd8756d3fa50c85ada38395f8a8bfaa638e0620cfe18`.
  Combined payload is 908790 bytes, below 1 MiB. The source-version mismatch is
  disclosed instead of assigning the catalogue version to the archive.
- The original procedural renderer and palettes remain in use for other modes.
  No new production package, online asset, domain state or IPC authority was
  introduced by Earth.
- The benchmark uses fresh exclusive output/profile directories, loopback
  ephemeral ports, a profile-matched browser debugger identity, bounded CDP
  handshakes/commands, and explicit teardown failure accounting. It does not
  attach to a user browser or run cleanup. Existing successful visual captures
  were reviewed through their evidence records; the full matrix was not rerun.
- Read-only Clean cross-layer review traced explicit roots through the closed
  bridge DTO, core validation and atomic persistence. Session validation stays
  separate from persistence errors. Scope edits and reviewed rescan requests
  invalidate prior confirmation state; rescan sends a target ID to core rather
  than constructing an action/path in React. Coverage remains display evidence,
  not selection or cleanup authority. The main-only/HUD boundary remains intact.

## Verification

Supported toolchain: Node 22.23.2, using its executable directly (not the global
npm shim). From `desktop/`:

```text
node node_modules/vitest/vitest.mjs run src/stage/Planet.test.tsx src/stage/earth-renderer.test.ts src/stage/earth-texture.test.ts --maxWorkers=2
node node_modules/eslint/bin/eslint.js src/stage vite.planet-benchmark.config.ts
node node_modules/typescript/bin/tsc --noEmit
```

- Targeted tests: **pass, 27/27** after the fix (20 Planet, 4 GPU, 3 loader).
- Scoped lint: **pass** after the fix.
- Full frontend type check: **pass** after the fix.
- Whole frontend test/lint/build: owned by runtime/final integration. Earlier
  full-suite results must not be represented as a run of this new regression.
- Rust `just ci`: not run here; native test/build slot belongs to the functional
  implementer and is serialized by the parent.

## Acceptance items still open

The child must remain in progress. This review does not close UI-AC1 through
UI-AC7 in full.

- Packaged Tauri startup with network disabled, embedded asset/fallback
  verification, actual Windows scale and final full-shell/root-control layout.
- Actual WebView2 minimize/restore behavior and twenty native route/visibility
  cycles, process/GPU resources, owned handles and memory plateau.
- Final native cold/warm and interaction measurements, including navigation,
  independently measured backend cancellation/join and paired still/animated
  process resources.
- The existing 47 ms cold p95 and 0.2 ms warm painted-callback p95 are retained
  component-build evidence. The initialization guard changes the final source
  identity and cold path; final integration must measure that build rather than
  relabeling earlier samples. The per-frame code is unchanged.
- Preserve the initial asynchronous 20-event relative interaction failure
  (+14.8%) alongside the later controlled 100-event pass (7.0 vs 7.6 ms p95),
  as the current evidence already does. JS submission measurements do not
  establish GPU completion, whole-app CPU or energy savings.
- Complete full desktop quality/wire-generation checks and the canonical
  repository gate after the remaining functional/runtime changes settle.
