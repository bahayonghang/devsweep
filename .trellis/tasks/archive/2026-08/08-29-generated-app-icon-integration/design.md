# Design - Generated DevSweep App Icon Integration

## 1. Exact Change List

| Path | Planned change |
| --- | --- |
| `desktop/src/assets/devsweep-icon-master.png` | new stable copy of selected generated source |
| `desktop/src-tauri/icons/32x32.png` | regenerated; PNG 32x32 |
| `desktop/src-tauri/icons/128x128.png` | regenerated; PNG 128x128 |
| `desktop/src-tauri/icons/128x128@2x.png` | regenerated; PNG 256x256 |
| `desktop/src-tauri/icons/icon.png` | regenerated; PNG 512x512 |
| `desktop/src-tauri/icons/icon.ico` | regenerated ICO with square frames at 16, 24, 32, 48, 64, 256px |
| `desktop/src-tauri/icons/icon.icns` | regenerated ICNS with 16, 32, 128, 256, 512px at 1x and 2x |
| `desktop/src-tauri/icons/Square30x30Logo.png` | regenerated; PNG 30x30 |
| `desktop/src-tauri/icons/Square44x44Logo.png` | regenerated; PNG 44x44 |
| `desktop/src-tauri/icons/Square71x71Logo.png` | regenerated; PNG 71x71 |
| `desktop/src-tauri/icons/Square89x89Logo.png` | regenerated; PNG 89x89 |
| `desktop/src-tauri/icons/Square107x107Logo.png` | regenerated; PNG 107x107 |
| `desktop/src-tauri/icons/Square142x142Logo.png` | regenerated; PNG 142x142 |
| `desktop/src-tauri/icons/Square150x150Logo.png` | regenerated; PNG 150x150 |
| `desktop/src-tauri/icons/Square284x284Logo.png` | regenerated; PNG 284x284 |
| `desktop/src-tauri/icons/Square310x310Logo.png` | regenerated; PNG 310x310 |
| `desktop/src-tauri/icons/StoreLogo.png` | regenerated; PNG 50x50 |

`desktop/src-tauri/tauri.conf.json` is inspected but should not need a content
change because its five icon paths already name generated outputs.

## 2. Asset Flow

```text
task research selected master
          |
          +--> desktop/src/assets/devsweep-icon-master.png
                    +--> local Tauri icon CLI
                              |
                              +--> desktop/src-tauri/icons/* (16 files)
```

No runtime path crosses into `.trellis/`. The task asset is provenance; the
product master is the build input.

## 3. Native Generation Contract

Run from `desktop/`:

```powershell
rtk npm run tauri -- icon ./src/assets/devsweep-icon-master.png --output ./src-tauri/icons
```

Before generation, record the 16-file inventory. After generation, require the
same inventory, decode all 16 files, dimension-check all 14 PNGs against the
table, and verify the exact ICO/ICNS embedded frame inventories plus the five
paths referenced by `tauri.conf.json`.

Expected container inventories:

```text
ICO square frames: 16, 24, 32, 48, 64, 256 px
ICNS base sizes: 16, 32, 128, 256, 512 px, each at 1x and 2x
```

Use an already-available decoder for validation only; do not add a project
dependency. The current host's Pillow decoder has already demonstrated that it
can expose both inventories.
Review every generated binary diff; do not stage `desktop/dist/`,
`desktop/node_modules/`, `target/`, or `dist/`.

## 4. Shell Handoff Contract

Record the master path, SHA-256, opacity, generated inventory, 16/32px evidence,
and recommendation that visible icon images be decorative when adjacent DevSweep
text is the accessible name. The shell child decides final DOM/TUI placement and
owns accessibility/responsive tests. This child edits no React/TUI/CSS file.

## 5. Small-Size and Visual Gate

The selected source already uses broad slabs and a thick inter-slab gap. That is
design intent, not proof. Proof comes after Tauri resampling:

- inspect `32x32.png` at 1:1;
- launch the Windows application and inspect native title-bar and taskbar icons;
- verify two separate slabs, one crescent, unclipped padding, no halo/checkerboard;
- hand the evidence and stable import path to the shell child.

If the criteria fail at either native size, stop. The rollback is to retain the
current product icons/header and return to image generation with the failed
preview as evidence. Do not improvise a CSS-only or post-processed fix.

## 6. Compatibility and Rollback

- No IPC, state, scan, cleanup, React/TUI, or persisted contract changes.
- Rollback restores the previous 16 native outputs, removes the product master,
  and leaves the current shell untouched.
- Windows visual evidence is direct. macOS/Linux output files can be decoded,
  but native appearance is not asserted without those systems.
