# Earth rendering design

## Smallest implementation path

Retain the existing decorative `Planet` component and shared stage composition. Give Clean one locally decoded geographic RGB/RGBA source while the four other modes retain their current generated sources. Reuse the existing Canvas2D sphere projection and lifecycle; improve latitude/longitude filtering and lighting balance. Async asset readiness must not become React workflow or cleanup state.

Compare bounded 384px and 512px source diameters, with effective DPR <=2 and output backing diameter <=512px for the existing 232 CSS-pixel hero. Select the smallest source that meets visual and CPU acceptance. Reuse typed buffers and projection/light tables; rebuild only on actual contracted source-size changes. Do not add a general rendering framework.

The user chose the asset direction, not a graphics package. If this Canvas candidate cannot meet both sharpness and cost, run one dependency-free WebGL comparison using the same texture/pose/workload. Record shader/context/resource/fallback implications and decision in this document before integrating. A new runtime package or materially changed scope returns to planning; it is not an automatic fallback.

## Asset contract

Use the authoritative candidate recorded in `../10-13-desktop-cleanup-completeness/research/mole-reference-and-assets.md`. Record original URL, version/date, provider, public-domain/redistribution statement, source hash, offline conversion command, output hash and dimensions. Ship only the small derivative and a static pose fallback, not the large source archive. Preserve accurate continents and poles; do not invent geography with noise. Put distribution information in `desktop/public/THIRD_PARTY_NOTICES.txt` and `docs/provenance.md`.

No runtime network access is needed. A decode or context failure renders the bundled still image; forced-colour mode renders the existing outline. Never retry a corrupt source every frame. If the fallback itself is missing, keep the outline and operation controls available.

## Lifecycle

States are local presentation states: not-ready, visible-animated, visible-still, suspended, fallback, disposed. Prefer the current effect/refs unless tests prove a dedicated state type is necessary.

- Create at most one active frame owner for the mounted Planet; cache one decoded Clean texture.
- Bind decode completion to the mounted generation and ignore stale completion.
- Defer initial drawing while hidden; resume with one current pose, not accumulated frames.
- Still mode receives one redraw for asset readiness or a real size/DPR change. It has no perpetual RAF loop.
- Unmount/route/detail replacement disconnects observers/listeners and cancels the frame owner.
- Keep OS reduced motion and forced colours authoritative. Preserve 15/30 settings and turn duration unless a separate product decision changes them.
- Verify WebView2 visibility/minimize/restore in the native run; add native visibility plumbing only if the observed browser signal is insufficient, using the existing lifecycle adapter.

## Contract changes and compatibility

Before product code, narrowly update `.trellis/spec/desktop-frontend/component-guidelines.md:257-277`, the matching completion checklist in `desktop-frontend/index.md`, and provenance. Permit the accepted Clean texture and measured resolution cap, preserve the Mole-asset ban and other layout/accessibility rules. Clarify still-pose redraws. No saved preference migration, IPC field, target id, plan, digest, or execution change is required.

## Validation protocol

Reuse the production-build benchmark and native capture patterns identified in the research. Mount the real Stage/Planet and preference state. Record binary/chunk/texture hashes, CPU/GPU, logical-core count, power mode, WebView2, viewport/DPR, source/output sizes and FPS. Capture 20 cold mounts; warm up 3 seconds, then record five 30-second windows for each candidate. Keep raw draw durations, paint gaps, long tasks, and resource samples. JS draw submission is not GPU-completion time.

Pair still/animated process-tree measurements on the same host, alternate baseline/candidate order, and record CPU normalization. Exercise idle and scan replay, native visibility changes and repeated mount/disposal. Coordinate interaction/resource collection with the runtime child so Planet and scan costs are separable.

## Rollback

Keep the preserved pre-change build/evidence. The texture/renderer change must be independently revertible without reverting functional scan fixes. A static local Earth is the failure fallback, not a reason to hide a failed animated performance gate. Record any accepted trade-off explicitly.

## Implementation renderer decision (2026-10-13)

The bounded Canvas candidate was implemented and measured before choosing a GPU
path. At the 232 CSS-pixel hero with DPR 2, the original 256px Canvas source cost
about 110.8ms of measured painted-RAF callback time over a 5s exploratory window.
The 384px geographic Canvas candidate cost 223.1ms; the 464px output from the
512px-cap candidate cost 340.3ms. Their frame p95 values were 1/2/3.3ms. The
sharper candidates met the absolute 4ms budget but increased ongoing main-thread
work rather than meeting the intended reduction. This warrants the planned
dependency-free GPU comparison. These short measurements are exploratory, not
the full acceptance run; raw records and preserved candidate source are under
`evidence/dist/canvas-quick-animated/` and `evidence/dist/canvas-source/`.

The selected candidate uses WebGL1 only for Clean: one local 1024x512 RGB
texture, one triangle, bilinear longitude/latitude sampling, fixed lighting and
one turn uniform. Main-thread per-frame work is bounded uniform updates and one
draw submission; the shader performs sphere projection. Output remains bounded
at 512px/DPR2. The other four modes retain the original Canvas renderer and
source algorithm. There is no new production package, worker or runtime asset
download. A second quick comparison reduced painted-RAF work from 110ms to
13.5ms over 5s (p95 1ms to 0.2ms). This is JS submission time, not GPU execution
or whole-app CPU.

Shader/link/context failures use the locally generated static Earth. Texture
failure does not retry on every route/frame; forced colors bypass texture work.
Disposal deletes owned textures, buffers, shaders and programs. A detached
canvas also releases its context through WEBGL_lose_context when available;
connected canvases remain valid for React effect replay and preference changes.
Mode changes replace the canvas because a canvas cannot switch context types.
Full cold/warm, lifecycle, browser matrix and native acceptance remain recorded
separately and must pass before declaring the child complete.
