# Implement - Integrate Generated DevSweep App Icon

Start only this child after the user approves the latest parent/child planning
summary and `task.py start 08-29-generated-app-icon-integration` succeeds.

## 1. Promote the selected source

1. Verify the task asset metadata and SHA-256 against
   `research/icon-generation.md`.
2. Copy it to `desktop/src/assets/devsweep-icon-master.png`.
3. Verify the product copy has the same SHA-256 and remains opaque RGB.

## 2. Regenerate native outputs

1. Record the pre-generation 16-file inventory from `design.md`.
2. Run the exact local Tauri command from `desktop/`.
3. Require the same 16 filenames after generation.
4. Decode all 16 outputs; verify all 14 PNG dimensions, the exact ICO frame set,
   and the exact ICNS base-size/scale set from `design.md`; recheck the five
   config paths. Use the already-available validation decoder without adding a
   project dependency.
5. Review binary changes and exclude generated build/dependency directories.

## 3. Run the native small-size gate and produce the shell handoff

1. Inspect regenerated `32x32.png` at 1:1.
2. Launch/build the Windows desktop application and inspect the title-bar and
   taskbar icons at native size.
3. Record whether both cache slabs, the crescent, padding, and clean background
   remain distinguishable.
4. If any criterion fails, stop and return to planning for a new generated
   source; do not continue packaging this asset.
5. Record the stable product path, SHA-256, opacity, exact native inventory,
   16/32 px evidence, and decorative-image accessible-name recommendation for
   `08-29-desktop-shell-navigation-brand`; do not edit React/TUI/CSS/tests.

## 4. Validation and review

```powershell
rtk just desktop-web-check
rtk just desktop-build
rtk git diff --check
rtk just ci
```

Review gates:

- exact stable master plus a decoded, dimension-checked 16-output inventory;
- no task-directory or `$CODEX_HOME` runtime reference;
- a complete shell handoff with no React/TUI/CSS/test file changes;
- no fake transparency claim;
- Windows direct evidence separated from macOS/Linux `UNVERIFIED` evidence;
- unrelated files excluded.

## Visual Evidence Record (implementation fills this)

- Product/task SHA-256 match: PASS. Both
  `.trellis/tasks/08-29-generated-app-icon-integration/research/assets/devsweep-icon-small-master.png`
  and `desktop/src/assets/devsweep-icon-master.png` are 956579-byte, 1254x1254
  opaque RGB PNGs with SHA-256
  `70527F3773669DDCE9FEFEB1CFC40D9FA5EFD5BF978CFC7BBB2F743DA8E308FD`;
  top-left pixel `(7, 67, 54)`.
- 32px 1:1 result: PASS. `desktop/src-tauri/icons/32x32.png` and task-owned
  `evidence/32x32-1x.png` share SHA-256
  `09E39547593449C2BEC66E12541E53752548BB6A88C9F776C6ED27C2EDC7FB52`.
  At native 1:1 size, two distinct slabs and one crescent remain recognizable;
  the light foreground bbox is `(5, 7)-(23, 25)`, does not touch an edge, all
  four corners are `(3, 66, 52, 255)`, and alpha extrema are `(255, 255)`.
- Windows title-bar/taskbar result: PASS. Direct release-app screenshots are
  `evidence/windows-native-titlebar.png` and
  `evidence/windows-native-taskbar-devsweep.png`. The title-bar 16px-class icon
  and the UIA-identified `devsweep - 1 running window` taskbar icon both retain
  two slabs, one crescent, unclipped padding, and a solid background without a
  checkerboard or halo. Every launch was closed through `CloseMainWindow()`;
  root exit was `0` where reported and final owned-process residue was `0`.
- Windows package: PASS. `target/release/devsweep-desktop.exe` is 8726016 bytes,
  SHA-256
  `3F263A756CF9F0D58A702B33ECD8B9A6D0EC43BA7488DB5CE34366928574E1DE`;
  `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` is 2136184 bytes,
  SHA-256
  `184138DE0C94D208CC8042359B949D364228390F46E3296EFC6AA0A2E2CEBC53`.
- Shell handoff path/inventory/accessibility recommendation: import only
  `desktop/src/assets/devsweep-icon-master.png`; it is intentionally opaque.
  Native inventory is the exact 16 files in `design.md`. When the visible image
  is adjacent to the DevSweep product text that supplies the accessible name,
  render the image as decorative (for example empty alt text / hidden from the
  accessibility tree); the shell child owns final DOM, responsive, and
  accessibility verification.
- macOS/Linux native appearance: `UNVERIFIED`. `icon.icns` and the cross-platform
  PNGs decode and their frame/dimension contracts pass on Windows, but neither
  native platform appearance was observed.
- Complete command, inventory, hash, screenshot, process, and gate evidence is
  recorded in `evidence/verification-2026-08-30.md`; full gate logs are adjacent.
