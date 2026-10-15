# DevSweep beaver identity

The original beaver was generated with the built-in `image_gen` tool on
2026-10-14. Mole informed the approachable animal direction; no Mole artwork
was used. Exact prompts are retained in `prompts/`. Compact v1 was rejected
for a forehead blemish and excess detail; compact v2 is the selected mark.

The full character is `devsweep-beaver-full.png`, retained for larger brand
surfaces without adding it to the application bundle. The compact master is
`../src/assets/devsweep-icon-master.png`. Both are the original 1254x1254 RGBA
generation outputs, unchanged after generation. Charcoal, warm ivory, sage,
the two teeth and neckerchief connect the two variants. The images have subtle
tonal variation; they are not exactly three flat colors. Most compact interior
alpha values are 253–254 rather than 255. The faintest outer pixels have alpha
1. No background removal, repainting, color quantization or alpha thresholding
is applied by the packaging script.

The compact silhouette above alpha 8 occupies `(156,182)-(1091,1126)` in the
1254px square: at least 128px clear margin to meaningful content. Actual
16/24/32/48/64px ICO frames and 20/128px resized views were inspected on white,
dark green and the existing navy surface. The face/teeth and sage accent remain
recognizable. This visual review does not replace native tray/taskbar review.

## Reproduce the desktop family

Use the project's installed Node 22 and pinned Tauri CLI 2.11.4. From the
repository root, choose a new ignored output directory:

```powershell
$brandNode = 'C:\Users\lyh\AppData\Local\mise\installs\node\22.23.2\node.exe'
& $brandNode desktop/scripts/prepare-brand-icons.mjs dist/brand-icons-new
```

The script invokes Tauri's icon converter with program/arguments separated.
It copies only the existing sixteen desktop filenames into `icons/`; the
converter's additional 64px/mobile outputs remain in ignored staging.
The generated ICNS container has nondeterministic record order in this CLI
version, so the script sorts encoded chunk types without changing payloads.
All ten decoded ICNS sizes were verified pixel-identical before/after sorting.
Two complete conversions produced identical hashes for all sixteen files and
the complete manifest. Review `icons/` and `manifest.json`, then copy them to
`desktop/src-tauri/icons/` and `desktop/branding/manifest.json` together.

`manifest.json` pins source, prompt and derived hashes, dimensions, Node version
and converter version. Source images need no Python dependency to package.
Pillow 12.3.0 was used only for separate read-only alpha/format/frame inspection.

## Consumers and validation boundary

| Consumer | Asset contract |
| --- | --- |
| AppShell brand button | Existing `src/assets/devsweep-icon-master.png` import; decorative `alt=""`/`aria-hidden`, adjacent product name unchanged |
| Tauri bundle | Existing 32px, 128px, 256px PNG and ICO/ICNS paths in `tauri.conf.json` |
| Window/taskbar and tray | Tauri default window icon; tray reuses `app.default_window_icon()` |
| Windows generated family | Existing Square*Logo/StoreLogo and `icon.png` retained and regenerated; no new platform integration claimed |

One webview master and sixteen native files changed. Five native files are
explicitly configured bundle entries; eleven retain the prior generated
family. Shell/tray code, configuration paths and semantic vector glyphs are
unchanged. Actual native webview/taskbar/tray and installer acceptance for the
new mark remains **NOT_RUN** until the final F integration build.

Prior assets and hashes are preserved under `dist/brand-icons-e/prior/` and the
parent's pre-edit source archive. Task inspection/reproducibility evidence is
under `.trellis/tasks/10-14-devsweep-animal-identity/evidence/` until archival.
