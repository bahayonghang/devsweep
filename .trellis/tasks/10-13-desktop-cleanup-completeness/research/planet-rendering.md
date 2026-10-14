# Research: Earth fidelity and bounded desktop rendering

- Query: Explain the artificial Earth in the supplied desktop screenshot, identify current render cost and lifecycle guarantees, and define a concrete UI child-task investigation and acceptance plan.
- Scope: internal source, specifications, archived task evidence, and the user-supplied screenshot. External asset research is owned by the parent session.
- Date: 2026-10-13
- Task: `.trellis/tasks/10-13-desktop-cleanup-completeness`
- Planning only: no product source, specs, dependencies, or runtime settings were changed; no new performance measurements were taken.

## Findings

### Product decision and evidence strength

The parent session relayed the user's explicit selection during this research: **real Earth geography using a locally bundled, redistributable texture**, with authorization to revise the earlier procedural-only restriction. A new runtime dependency was not approved. The renderer implementation should therefore be decided by fidelity and measured cost, not by preserving fictitious geography or assuming a 3D library is required.

The attachment shows a dark sphere with indistinct green land, pale coastal bands, blue water, and a dark lower-right hemisphere. It supports the visual complaint. A still screenshot cannot demonstrate frame rate, CPU/GPU usage, startup latency, memory growth, or frame pacing. The screenshot's approximate globe size is consistent with 232 CSS pixels at 125% display scale, but its actual DPR, app version, preferences, and build identity were not supplied, so this is an inference, not a measurement.

### Files found

| File | Responsibility |
| --- | --- |
| `desktop/src/stage/Planet.tsx` | Canvas sizing, source caching, frame loop, visibility, motion preferences, and disposal. |
| `desktop/src/stage/planet-renderer.ts` | Seeded texture generation, cached sphere projection/light geometry, and per-frame CPU pixel writes. |
| `desktop/src/stage/planet-palettes.ts` | Five per-mode seeds and colour bands; Clean uses invented ocean/land colours. |
| `desktop/src/stage/Planet.test.tsx` | Mocked frame-loop/accessibility tests and deterministic renderer assertions. |
| `desktop/src/stage/styles.css` | Responsive globe diameter, stage spacing, and forced-colour outline. |
| `desktop/src/stage/Stage.tsx` | Shared stage; mounts the default Planet and exposes a replacement hero slot. |
| `desktop/src/pages/ScanPage.tsx` | Clean idle/scanning stage with live phase, scope controls, and cancellation. |
| `desktop/src/app-shell/AppShell.tsx` | Only the active mode registration is rendered. |
| `desktop/src/preferences/appearance.ts` | System/local font stack, motion preference, and appearance application. |
| `desktop/src/preferences/store.ts` | Ordered preference snapshots; defaults come from the committed fixture. |
| `desktop/src/api/fixtures/desktop-preferences.json` | Default 30 FPS, system motion, and system font. |
| `desktop/package.json` | Node 22 requirement, current dependencies, and Analyze benchmark precedent. |
| `tools/native-acceptance.mjs` | Existing native WebView2 capture and layout driver. |
| `desktop/src/modes/analyze/render-benchmark.tsx` | Existing production-build benchmark component precedent. |
| `.trellis/tasks/archive/2026-09/09-23-desktop-mole-capsule-shell/` | Original procedural-planet product decision and stage design. |
| `.trellis/tasks/archive/2026-09/09-20-desktop-native-acceptance/evidence/` | Historical native layout/DPR evidence; not a planet timing benchmark. |
| `.trellis/tasks/archive/2026-09/09-20-desktop-operation-performance/evidence/record.md` | Historical resource measurements and warnings about production-build identity. |
| `docs/provenance.md` | Current claim that no image/texture backs the planets; requires a targeted factual update. |

### Current rendering algorithm and cost

1. **Texture creation is synchronous and procedural.** `Planet.tsx:19-27` lazily caches one RGB texture per mode in a module-level Map. `planet-renderer.ts:4-11` fixes the texture at 512×256 and uses five noise octaves. For every texture texel, `planet-renderer.ts:116-142` calculates sphere coordinates, a five-octave domain warp, another five-octave noise field, and colour-band interpolation. Each value-noise sample hashes eight lattice corners (`planet-renderer.ts:41-60`). That is 131,072 texels × 10 noise samples × 8 hashes = **10,485,760 hash evaluations per previously unseen mode**, derived from the code, not timed. All of this occurs inside the component effect before the first draw (`Planet.tsx:45-83`).
2. **This is not Earth geography.** Clean's seed 1307, frequency 2.1, ambient light 0.12, and six threshold colour bands are in `planet-palettes.ts:25-37`. The same scalar noise controls deep/shallow ocean, a narrow tan shore, vegetation, and pale high regions. There is no geographic land mask, real coastline, cloud layer, latitude-based polar ice, ocean specular term, atmospheric scattering, or terrain-normal shading in the current algorithm. Increasing only the source resolution cannot turn this into recognizable Earth.
3. **Projection and lighting are cached for a source diameter.** `planet-renderer.ts:156-201` clamps the diameter to 8..256, computes unit-sphere normals, row/longitude samples, edge alpha, fixed-direction Lambert lighting, and rim darkening. The hemisphere's dark appearance follows the small ambient term and the extra rim multiplier (`planet-renderer.ts:182-190`). This is a flat colour map lit on a smooth sphere, not a geographically detailed material.
4. **Every painted frame runs on the JavaScript main thread.** `renderPlanetFrame` clears the entire RGBA buffer, walks covered pixels, linearly interpolates two horizontally adjacent RGB texels, multiplies by cached light, and writes alpha (`planet-renderer.ts:206-226`). Latitude sampling is nearest-row, not bilinear (`planet-renderer.ts:176-181`). The component then calls `putImageData`, clears the display canvas, and scales the source with `drawImage` (`Planet.tsx:76-80`). There are no React state updates for each rotation frame.
5. **Resolution cap bounds CPU shading, but not the display backing store.** The source never exceeds 256² pixels. The display canvas is `round(clientWidth × devicePixelRatio)` on both axes without a DPR/backing-size ceiling (`Planet.tsx:59-74`). CSS diameter is 232px normally, 168px at ≤800px viewport width, and 136px at ≤520px (`stage/styles.css:1-2`, `:158-176`). Thus 232px at DPR 1.25 paints a 256px source into a 290px backing store; DPR 2 uses 464px. Higher DPR increases final clear/compositing area while source detail stays fixed. The screenshot's softness is consistent with this constraint, but cannot establish it as the sole cause.
6. **Memory is bounded for current texture cache, but resizing has transient work.** RGB data is 512 × 256 × 3 = 393,216 bytes per mode (384 KiB), or 1,966,080 bytes for all five modes (1.875 MiB), excluding object overhead. A full 256² RGBA frame is 256 KiB. Geometry uses five typed arrays totalling 17 bytes per covered pixel; creation also uses temporary JS arrays (`planet-renderer.ts:163-200`). Geometry/frame are rebuilt per effect mount or when the clamped source diameter changes, while textures survive route changes. Display/source canvas and browser/GPU storage are additional and not measured here.

### Existing lifecycle guarantees and specific gaps

- Current preference is 15 or 30 painted FPS, default 30; one full turn is 96 seconds (`Planet.tsx:16`, `:38-42`; backend preference spec `desktop-preferences.md:58-61`). The RAF callback itself continues at display refresh rate, while `draw()` is throttled (`Planet.tsx:91-102`); a frame cap does not mean only 15/30 callbacks per second. On a 60 Hz display most callbacks perform only time arithmetic and branching.
- Hidden-document transitions cancel the owned frame; restoring visibility resumes without catching up all elapsed hidden time; unmount removes the listener and cancels the handle (`Planet.tsx:91-124`). Only the active shell registration renders (`AppShell.tsx:473-475`), so normal navigation removes the old stage. `Stage.tsx:43` does not pass `active` explicitly; the `active=false` path is currently principally an isolated component contract/test.
- Effective reduced motion is OS reduce **or** app reduced (`Planet.tsx:39-42`); it draws once and does not enter the animation loop (`:83-84`). Forced colours bypass drawing (`:47`) and use a CSS circle (`stage/styles.css:193-201`). Planet remains decorative and non-interactive (`Planet.tsx:128`; `stage/styles.css:15-20`). These are requirements to preserve.
- The first draw runs even if the document is already hidden (`Planet.tsx:83` precedes the `document.hidden` check in `start`). A new asset decoder must not introduce hidden retries or background animation; choose and test whether an initial hidden load is deferred.
- Null 2D contexts return early, leaving an empty decorative box rather than an explicit static fallback (`Planet.tsx:48-52`). No context-loss, image-decode failure, or unavailable-resource fallback exists because the current implementation has no image resources.
- Reduced-motion/inactive effects return before registering any resize hook. There is no ResizeObserver or DPR-change subscription. Animated mode notices size/DPR only on a draw; a still frame may therefore retain its old backing resolution after resizing (`Planet.tsx:59-84`). Preserve “one still pose/no continuous loop,” while allowing one redraw of that same pose when dimensions or DPR actually change; record this clarification in the spec.
- There is no viewport intersection handling. Hidden-document handling does not prove quiescence when the stage is scrolled offscreen or the native window is minimized/occluded. Verify WebView2's actual visibility events before adding a native listener. No native visibility wiring is justified solely by this screenshot.
- The Clean globe remains mounted during scanning (`ScanPage.tsx:123-141`, `Stage.tsx:43`). Decoration therefore competes with progress decoding/rendering; performance acceptance must include scan progress and Cancel responsiveness, not only an idle globe.

### What existing tests and historical evidence prove

- `Planet.test.tsx:103-136` checks 15/30 FPS throttling with a mocked RAF; `:139-178` checks hidden/resume, inactive, and unmount; `:181-209` checks reduced motion and forced-colour CSS; `:214-237` checks source cap, deterministic texture, transparent corners, and changed pixels after rotation. These prove control flow and pixel invariants, **not** native performance or visual Earth fidelity.
- The original archived PRD explicitly records the 2026-09-23 user choice of original procedural planets and no NASA/other photograph or Mole texture (`09-23-desktop-mole-capsule-shell/prd.md:12-24`), with image assets/WebGL/dependencies out of scope (`:91-96`). The new user selection supersedes that product restriction prospectively; do not rewrite the historical record.
- The historical native record dates to 2026-09-24 and a named binary/WebView2 version (`09-20-desktop-native-acceptance/evidence/record.md:3-11`). It records actual 125% scale and emulated 1/1.25/1.5/2 device factors (`:35-50`), and native screenshots. `tools/native-acceptance.mjs:163-175` records only the first canvas's width/height as `planet_canvas`; it does not time frames or inspect whether geography looks real. The historical 290×290 field is an output canvas size, **not** proof of a 290px source renderer.
- That native record leaves a 45-thread measurement against a 40-thread gate failed (`record.md:85-101`). The operation-performance record reports other failed gates and an explicit historical user pause (`09-20-desktop-operation-performance/evidence/record.md:17-24`). Neither observation attributes threads or operational latency to Planet, and neither should be presented as a current planet regression or a new task's automatic acceptance failure.
- The operation-performance record also documents a bare Cargo release binary serving no UI, fixed by a Tauri CLI build (`record.md:28-39`). A new native benchmark must establish the loaded UI/build identity, not merely launch an executable.
- No planet-specific startup timing, frame-paint p95, main-thread long-task count, GPU use, incremental CPU measurement, or repeated-switch leak measurement was found in the inspected files. No fresh native or browser run was performed during this research.

### Font complaint: not established

The screenshot's serif-like glyphs are visible, but the user requested Earth and cleanup changes, not a typography reset. Current defaults are `"Segoe UI Variable", system-ui, "Microsoft YaHei UI", sans-serif` (`preferences/appearance.ts:22`). A committed installed family intentionally precedes the same fallbacks (`:25-28`), and `applyAppearance` writes the chosen stack and scale (`:45-51`). The spec explicitly supports system and installed families (`component-guidelines.md:211-215`). Therefore the screenshot does not prove a broken fallback or unstyled page. A useful verification row is “capture committed font preference and resolved fonts for EN/zh-CN at real DPR”; do not silently overwrite the user's chosen font or widen the Earth task into a font redesign.

### Minimal implementation options and recommendation

The user has chosen **real geography/local texture**; this table compares the remaining rendering choices, not whether to revert that decision.

| Approach | Benefit | Cost / risk | Planning position |
| --- | --- | --- | --- |
| Existing Canvas2D sphere with a decoded local geographic texture | Smallest change; reuse lifecycle, projection, palette routing, and most tests; no dependency; eliminates noise-generation cold work for Clean. | CPU pixel work still grows with source diameter; image decode, vertical filtering, and failure state need explicit handling; 256px cap may remain too soft. | Recommended first prototype and baseline candidate. Compare source caps 384 and 512 at bounded DPR; choose the lowest that passes visual and measured budgets. |
| Small dependency-free WebGL renderer with a local texture | Pixel sampling and lighting move to GPU; can support sharper high-DPR Earth with less main-thread work. | More lifecycle state and GPU resources, context loss/restoration, shader compilation, GPU/driver variability, and fallback. New code is not automatically faster on all hosts. | Escalate only if the Canvas candidate misses the agreed budget/fidelity combination; benchmark with the same source texture and pose. No npm 3D engine is implicitly approved. |
| Locally bundled pre-rendered still of the approved Earth | Minimal ongoing work, consistent appearance, survives render-context failure. | No rotation, less theme flexibility; needs provenance and a build-time/static derivative. | Required fallback candidate and reduced-motion option; not the chosen normal experience unless separately accepted. |

Do not introduce workers, OffscreenCanvas, a general renderer framework, multiple cloud/specular/normal maps, 4K/8K assets, or dynamic downloads as initial scope. Use one modest local equirectangular Earth texture (1024×512 is a candidate input size, not an approved asset) and preserve known continents, oceans, and poles. Improve lighting balance and edge filtering without colouring Earth land according to mode risk. Keep the other four mode identities intact unless the parent PRD explicitly expands their redesign. A one-file asset loader and a narrow renderer boundary can support Canvas and static fallback without prematurely abstracting every rendering backend.

### Asset/provenance requirements and precise spec changes

The parent handles authoritative external sourcing. Before implementation locks the asset, record its exact upstream URL, author/provider, source revision/date, redistribution terms, attribution requirements, original checksum, any crop/downsample/colour changes, and the checksum/dimensions of the shipped derivative. Ship it locally; no runtime network requests or dependency on mole.fit availability. Do not assume NASA provenance automatically settles every attribution/trademark restriction. Do not copy Mole's bitmap or rendered globe.

Required prospective documentation changes, owned by the main session/spec workflow:

1. `.trellis/spec/desktop-frontend/component-guidelines.md:257-269`: replace “only procedural/from per-mode seed” and blanket image/texture/WebGL ban with the user-approved Clean real-Earth/local-asset contract; preserve decorative semantics, 15/30 FPS choices, visibility/disposal, and accessibility. Any source-resolution change replaces the explicit 256px cap with an independently justified source/backing limit and measurement protocol. Clarify a still pose may redraw on actual size/DPR changes without animation.
2. `component-guidelines.md:274-277`: carve out the approved licensed Earth asset from the blanket photographic/NASA/texture prohibition while preserving the Mole-asset ban, original product identity, unrelated glass/gradient restrictions, and truthful cleanup copy. Do not globally lift unrelated visual restrictions.
3. `.trellis/spec/desktop-frontend/index.md:95-104`: update the completion checklist's procedural-only and no-planet-photograph wording consistently, keeping reduced motion, forced colours, responsive EN/zh-CN, and font preference requirements.
4. `docs/provenance.md:46-51`: replace the now-inaccurate all-planets-seeded/no-texture assertion with actual per-mode generation and the selected asset record. Use the existing `desktop/public/THIRD_PARTY_NOTICES.txt` for redistribution notices when required.
5. `.trellis/spec/desktop-frontend/index.md:32-33` still requires explicit approval for a new production dependency. The user approved an asset, not a dependency. Raw platform Canvas/WebGL APIs do not require adding a package.

### Proposed benchmark method — no results yet

Follow the repository's production-build harness rule (`desktop-frontend/index.md:69-83`) and reuse its native CDP approach. A benchmark added during implementation should mount the real Planet/Stage and real preference state, not a simplified pixel loop alone.

1. Identify baseline and candidate assets/binaries/bundled chunks by hash, renderer/source/output resolutions, preference FPS, viewport, effective DPR, CPU/GPU, logical-core count, OS, power mode, WebView2 version, and build mode. Build with the supported Node 22 toolchain and Tauri build path. Use an isolated app profile; do not change OS scaling or user preferences.
2. Take at least 20 cold mount/load samples in fresh renderer contexts; separately time module/texture decode or generation, geometry initialization, and time to first visible non-fallback frame. Include normal launch without an automation-attached timing distortion in interpretation.
3. For warm steady state use a discarded 3s warm-up followed by five 30s windows per candidate; retain raw painted-frame duration and RAF timestamps. Report median/p95 and cadence, plus long tasks. Measure JS pixel work separately from `putImageData`/draw submission; do not call submission completion a GPU-completion measurement. Where available collect tracing/compositor data separately.
4. Compare 15/30 FPS; 390/800/1024/1440 CSS widths; DPR 1/1.25/1.5/2; normal and reduced motion; forced colours; light/dark appearance. Performance focus can use the largest source/output pair plus one narrow case, while visual coverage spans the full required layout matrix. Record actual OS scale separately from device emulation.
5. On the same host use paired alternating still-vs-animated runs to measure incremental CPU/private bytes of the relevant app/WebView process tree. CPU reports must state whether normalized across all logical cores; optionally report one-core-equivalent as a separate value. Avoid using a noisy whole-machine CPU snapshot as Planet cost.
6. Include scan progress replay plus clicking Cancel, mode changes, stage-to-details navigation, 20 show/hide cycles, 20 mode switches, resize/DPR change under reduced motion, and native minimize/restore. Assert no repeated draws while hidden/inactive, no callbacks/resources after disposal, and no stale image completion resurrects an unmounted renderer.
7. Visual capture uses the same texture, pose, CSS diameter, background, and DPR for baseline/candidate. Inspect geographic recognition, coastline sampling, sphere edge aliasing, readable dark-side shape, colour balance, and animation temporal stability. A successful geographic screenshot is separate from performance success.
8. Explicitly test missing/corrupt asset, Canvas unavailable, and (if used) WebGL unavailable/context lost. Fallback must be local, static, decorative, correctly sized, and must not prevent Scan or mode navigation. Do not repeatedly retry failed assets every frame.

### Proposed acceptance budgets — provisional, not measured or inherited

These are concrete starting budgets for the child PRD. Freeze their host/method before comparing candidates; record a failed result rather than lowering a threshold after measurement. A renderer spike may justify a reviewed budget change before the implementation is accepted.

| Area | Proposed budget |
| --- | --- |
| UI availability | Globe loading must not gate the Scan button, scope controls, route navigation, or cancel state. No decorative synchronous long task ≥50ms in the reference production trace. |
| First useful Earth | Cold first visible Earth frame p95 ≤250ms after its stage is mounted; a correctly sized local still/fallback may appear earlier. Report decode/init stages separately. |
| Warm frame main-thread work | p95 ≤4ms per painted frame at 30 FPS and the maximum contracted source/backing size; target a ≥25% reduction in comparable steady-state main-thread CPU time versus the current renderer if the baseline is sufficiently above measurement noise. Both absolute and paired results must be reported. |
| Cadence | No more than the selected 15/30 paint FPS; no catch-up bursts after visibility restoration; fewer than 1% of warm painted-frame gaps exceed twice the selected interval under the idle reference workload. |
| Interaction under scan updates | Cancel/navigation event-to-visible-response p95 ≤100ms in the replay harness and no regression beyond 10% against the same baseline workload. Backend cancellation acknowledgement/join remain separately owned metrics. |
| Hidden/inactive/static | Zero recurring owned RAF/draw work after suspension/disposal; still pose only under effective reduced motion; one redraw allowed for a changed size/DPR or asset readiness. Forced colours needs no texture decode/render work. |
| Allocation/lifecycle | One visible renderer and one decoded Clean texture; zero surviving renderer handles after unmount; no monotonically increasing resource count across 20 cycles. Report memory plateau rather than asserting browser GC happens at a fixed time. |
| Size | Start with ≤1 MiB compressed Earth payload, ≤1024×512 decoded input, bounded source ≤512px and output ≤512px, effective DPR cap ≤2 for this 232px hero. These are candidate caps to reconcile with visual acceptance, not new global app caps. |

A lower frame rate alone is not a successful optimization if visible motion becomes uneven; a sharper source alone is not a successful redesign if it blocks the scan interface. Prefer the simplest renderer that meets both visual and measured gates. Keep accepted baseline evidence alongside candidate results.

### Related specifications and external references

- `.trellis/spec/desktop-frontend/index.md`: frontend approval boundary, quality commands, responsive/accessibility matrix, production and native benchmark evidence contract.
- `.trellis/spec/desktop-frontend/component-guidelines.md`: current visual system and exact procedural restriction to revise.
- `.trellis/spec/desktop-frontend/state-management.md`: preserve Clean selection/digest authority while changing decoration and stage composition.
- `.trellis/spec/desktop-frontend/type-safety.md`: keep asset/renderer presentation work out of cleanup authority and IPC contracts.
- `.trellis/spec/backend/desktop-preferences.md:58-61`, `:110-120`: existing frame/motion/font settings and OS accessibility overrides.
- External references: none newly fetched in this subtask; parent session owns mole.fit/Mole comparison and authoritative asset sourcing. No external graphics-library version or redistribution claim is asserted here. Current local `desktop/package.json:6-8`, `:22-47` requires Node ≥22 <23 and has no dedicated graphics-library dependency.

## Caveats / Not Found

- No live screenshot recreation, pixel output capture, test run, or measured benchmark was executed; timings above are proposals, and arithmetic cost estimates are code-derived.
- Existing native captures are historical build evidence. They do not verify the current screenshot build, current font selection, current window scale, or a Planet-specific performance target.
- The scope of this research is the Earth/UI child. Cleanup discovery and execution coverage are separate parent/child research; no UI rendering option broadens file-deletion authority.
- The exact redistributable texture and its legal/provenance record remain owned by parent research. If source selection is incomplete, implementation must preserve a local static fallback and may not silently fetch or copy Mole assets.
- A realistic Earth does not imply that the four other mode planets should also become photographic; preserve that boundary unless the parent requirements say otherwise.
