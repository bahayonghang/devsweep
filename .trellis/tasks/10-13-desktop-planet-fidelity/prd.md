# Planet visual fidelity and render performance

## Goal

Make Clean's Earth recognizable and clear at native Windows scaling while reducing avoidable render work and preserving scan responsiveness.

## Ownership and dependencies

- Parent: `10-13-desktop-cleanup-completeness` (CLI and Tauri cleanup/performance programme).
- Owns `desktop/src/stage/`, local Earth assets/notices, targeted visual-spec/provenance changes, and the Planet benchmark. Does not own scan controls, provider rules, or cleanup authority.
- Can implement independently after plan approval. Coordinate benchmark/build identity with `10-13-cli-tauri-runtime-performance`; integrated performance acceptance waits for that child's final run. Functional child owns root selection and Clean data presentation.

## Confirmed background and decision

- The user selected real geography with a local redistributable texture on 2026-10-13. Replace the old procedural-only Clean restriction prospectively; do not rewrite archived decisions.
- P1 visual defect: Clean uses fictitious noise geography (`desktop/src/stage/planet-palettes.ts:25-37`; `planet-renderer.ts:116-142`). Its source diameter is capped at 256px while the output follows uncapped DPR (`Planet.tsx:59-80`).
- P1 lifecycle gaps: still frames do not respond to size/DPR changes; null contexts leave an empty box; initial drawing precedes the hidden-document check (`Planet.tsx:48-84`). Existing route/unmount/hidden cancellation and 15/30 FPS settings already work and must remain.
- Main-thread procedural generation and per-pixel shading are code-confirmed costs, not measured bottlenecks. Exact findings, source anchors, alternatives, and historical evidence limits are in `../10-13-desktop-cleanup-completeness/research/planet-rendering.md`.

## Requirements

- UI-01: Clean displays geographically recognizable continents, oceans, and polar regions, with balanced lighting, stable edges, and no noisy invented coastline. Preserve the original DevSweep shell, other mode identities, and selected theme/font.
- UI-02: Bundle a small offline Earth texture and static fallback with exact source/version/checksum/transformation/redistribution evidence. Natural Earth is the researched candidate. No runtime texture download, Mole asset, or new production dependency.
- UI-03: Decoding and initialization must not gate Scan, root editing, navigation, or Cancel. Cache immutable texture/projection data only within explicit memory bounds; avoid per-frame allocations and repeated cold generation.
- UI-04: Preserve 15/30 painted FPS, elapsed-time rotation without resume bursts, reduced-motion still pose, forced-colour outline, decorative accessibility, and complete resource disposal. A still pose can redraw after a real size/DPR change without starting animation. Hidden/unmounted renderers do not recur or revive after async decode.
- UI-05: Missing/corrupt asset, unsupported context, resize, minimize/restore, and route changes leave a correctly sized fallback and a usable application. Verify actual WebView2 minimize behavior before adding native lifecycle code.
- UI-06: Supply production and native evidence separating visual quality from frame cost. New budgets below are proposed acceptance thresholds, not results. Freeze host and method before implementation comparisons.

## Acceptance criteria

- [ ] UI-AC1 (UI-01, UI-02): Same-pose baseline/candidate captures at 390/800/1024/1440 CSS px, both locales, and DPR 1/1.25/1.5/2 show recognizable geography, readable light/dark hemispheres and smooth edges. Record real Windows scale separately; no forced OS-scale change. Preserve user font choice.
- [ ] UI-AC2 (UI-02): App starts and renders Earth with network disabled. Asset and fallback are licensed and locally packaged; total compressed Earth payload <=1 MiB, decoded primary texture <=1024x512. Verify notices and packaged build, not only dev-server paths.
- [ ] UI-AC3 (UI-03, UI-06): On the declared reference host, cold first Earth p95 <=250ms after stage mount, no decorative synchronous task >=50ms, warm draw main-thread p95 <=4ms at 30 FPS. Report paired baseline comparison; target >=25% lower steady main-thread CPU when above measurement noise. A miss is a failure requiring an explicit decision, not a silent threshold edit.
- [ ] UI-AC4 (UI-04): Paint does not exceed selected FPS; <1% of warm idle frame gaps exceed twice its interval. Hidden/inactive/unmounted states have zero recurring owned frame/draw work; reduced motion draws a still pose only; forced colours needs no texture work.
- [ ] UI-AC5 (UI-04, UI-05): Tests cover late decode, load failure, missing context, hide/show, route/detail switch, resize/DPR in reduced motion, and unmount. Twenty switch/visibility cycles show bounded handles and a memory plateau; no stale render completion restarts the globe.
- [ ] UI-AC6 (UI-03, UI-06): With deterministic scan progress replay, Cancel/navigation event-to-visible-response p95 <=100ms and no >10% regression under the same workload. Backend cancel/join is measured separately by the runtime child.
- [ ] UI-AC7: Targeted tests and desktop lint/typecheck/test/build pass; update only the prospective visual/provenance contract that conflicts with the accepted asset direction. Include native evidence and the parent integration handoff.

### Verified portions at final C2 checkpoint (2026-10-13)

The complete criteria above remain open until the remaining runtime integration
is recorded. Earlier failures and superseded measurements remain in evidence.

- [x] UI-AC1: original/candidate browser matrix plus final packaged native C2
  EN/zh-CN × four widths × four DPRs. Fresh-mount native captures assert exact
  backing size, no horizontal overflow and preserved font. Actual Windows scale
  is 125%; other scales are explicitly emulated. See
  [native acceptance](evidence/native-earth-acceptance.md).
- [x] UI-AC2: licensed bounded local primary/fallback assets and final C2 packaged
  offline rendering passed. No runtime download or new production dependency.
- [x] UI-AC3 component portion: cold p95 24.9 ms, zero observed cold long tasks,
  warm painted callback p95 0.1–0.2 ms, and 93.6% less median synchronous painted
  work versus original. This is submission time, not whole-process CPU or GPU
  completion. Final paired native still/animated process evidence remains with
  runtime integration. See [C2 measurements](evidence/renderer-c2.md).
- [x] UI-AC4/UI-AC5: final 30 FPS gap count is 0/4316; twenty real native
  minimize/restore and route cycles have zero paused/late draws and release
  owned GPU resources, with bounded observed memory/handle bands. Tests cover
  still/resize/DPR, failures and late decode. Native CDP changed DPR without
  delivering media/resize events; real OS-scale switching was not performed.
- [x] UI-AC6 component portion: 100 aligned Cancel events p95 7.0 ms versus
  original 7.7 ms; 100 navigation events p95 7.0 ms for both builds, with actual
  AppShell destination commits. Both absolute and relative gates pass. Native
  backend cancellation/join remains a separate runtime measurement.
- [x] UI-AC7 source gate: C2 frontend lint/typecheck/438 Vitest/5 Node/build and
  744 Rust tests passed; prospective specs and native evidence are updated.

The pre-existing selected-tab hover contrast issue is recorded separately in
native evidence. The task does not claim a fix to unrelated shell CSS.

## Out of scope

Other-planet photographic redesign, a 3D framework, extra map layers, live Earth/weather data, 4K/8K downloads, global typography/theme redesign, and cleanup-policy changes.

## Artifact status

The user approved the parent and this child on 2026-10-13. Texture selection and measured renderer comparison are implementation work; no performance pass is inferred from approval. Product edits begin after the original baseline A is preserved.
