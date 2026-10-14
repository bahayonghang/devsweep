# Production Earth visual matrix

2026-10-13; captured by the main coordinator after the renderer handoff.
This proves production-browser rendering, not native WebView2/offline acceptance.

Both benchmark builds passed TypeScript and Vite under Node 22.23.2 / bundled
npm 10.9.8. Commands in `desktop/`:

```text
npm run benchmark:planet:build -- --mode baseline
npm run benchmark:planet:build -- --mode production
```

From the repository root, with the scoped native-browser permission needed by
this host's sandbox:

```text
node tools/planet-render-benchmark.mjs .trellis/tasks/10-13-desktop-planet-fidelity/evidence/dist/visual-matrix-20261013-native matrix baseline production
```

Each build produced 32 same-pose captures: English/Chinese, widths
390/800/1024/1440 CSS px, DPR 1/1.25/1.5/2. Six additional cases cover light theme,
forced colors, missing texture, both images missing, unavailable WebGL, and 15 FPS.
All 64 locale/size captures reported no horizontal overflow and one still draw
with zero recurring owned RAF. All candidate normal captures reached `ready`;
the largest buffer was 464px, below the 512px cap. Fonts were not reset by Planet.

Visual inspection of narrow Chinese, medium Chinese, wide English, light theme,
baseline Chinese, forced colors and failure fallbacks confirms recognizable
continents, brighter readable hemispheres, stable round edges and intact controls.
Missing primary texture or WebGL shows the static geographic globe; both images
missing and forced colors show a correctly sized outline. Every failure case has
zero recorded draw/RAF recurrence. The five-second 15 FPS case painted 72 frames
for both builds (no cap exceedance).

The isolated browser exited normally with code 0; no forced teardown was needed
for this successful matrix. [visual-matrix-summary.json](visual-matrix-summary.json)
records every capture's build identity, dimensions, font, rendering facts and PNG
checksum. Complete raw JSON and PNGs remain in the ignored directory above.

The preceding sandbox attempt at `dist/visual-matrix-20261013` stalled before
capturing any page. Its old WebSocket handshake had no deadline. The driver was
fixed to use a fresh exclusive profile, ephemeral ports, verified profile endpoint,
bounded handshakes/commands, and explicit teardown accounting. The failed attempt's
controller required identity-checked teardown after its browser exited; its
`failed-attempt.json` is retained and its lifecycle result is failed/unavailable,
not pooled into successful measurements. No user browser was attached or closed.

The component harness does not mount the new root-selection controls or full
application shell. Full scope-layout, packaged offline, actual OS scale,
minimize/restore and process-tree evidence remain separate integration criteria.
