# Verification Evidence - Generated DevSweep App Icon Integration

Recorded 2026-08-30 Asia/Shanghai from
`D:\Documents\Code\Rust\Exp\devsweep`. No dependency, manifest, lockfile,
configuration, React, TUI, CSS, or test file was edited. No installer was run.

## Source and stable product master

Source:
`.trellis/tasks/08-29-generated-app-icon-integration/research/assets/devsweep-icon-small-master.png`.

Commands and results:

```powershell
Get-FileHash -Algorithm SHA256 <source>
python -X utf8 -c <Pillow metadata and pixel inspection>
```

Exit `0` after correcting an initial PowerShell/Python quoting-only SyntaxError.
Verified PNG, 1254x1254, RGB bands only, no alpha, 956579 bytes, top-left pixel
`(7, 67, 54)`, SHA-256
`70527F3773669DDCE9FEFEB1CFC40D9FA5EFD5BF978CFC7BBB2F743DA8E308FD`.
Direct visual inspection showed two broad separated slabs, one crescent, one
sparkle, an opaque full-bleed emerald background, and an unclipped source mark.

`desktop/src/assets/` did not exist before this task. It was created and the
binary PNG was copied to
`desktop/src/assets/devsweep-icon-master.png`. Source and product size/hash are
identical. Text `apply_patch` cannot encode a binary PNG, so the exact binary
copy used PowerShell `Copy-Item`; all task Markdown/JSON/log changes used
`apply_patch`.

## Pre-generation inventory

Before generation, `desktop/src-tauri/icons/` contained exactly the 16 top-level
files frozen by `design.md`, with no directory. Names and SHA-256 values are in
`pre-generation-inventory.sha256`.

The five `desktop/src-tauri/tauri.conf.json` icon references before and after
generation were exactly:

```text
icons/32x32.png
icons/128x128.png
icons/128x128@2x.png
icons/icon.icns
icons/icon.ico
```

## Generation and closed inventory validation

Required command, from `desktop/`:

```powershell
rtk npm run tauri -- icon ./src/assets/devsweep-icon-master.png --output ./src-tauri/icons
```

Exit `0`; captured output is in `tauri-icon-generation-2026-08-30.log`.
Tauri CLI 2.11.4 also emitted `64x64.png`, `ios/`, and `android/`, which were not
in the proven pre-generation inventory or Exact Change List. Before deletion,
all three resolved absolute paths were verified to start with the resolved
`desktop/src-tauri/icons\` prefix. Only those newly generated extras were
removed; they are reproducible by rerunning the command. The final directory
contains no subdirectory and exactly the frozen 16 files.

Existing Pillow was used only as a decoder/validator. A closed assertion script
exited `0` and proved:

- exact file set: 16;
- exact PNG set: 14, each with the design dimension;
- all 14 PNGs decode as RGBA and every alpha channel has extrema `(255, 255)`;
- ICO frames exactly `16, 24, 32, 48, 64, 256` square;
- ICNS inventories exactly base sizes `16, 32, 128, 256, 512`, each at 1x and
  2x;
- the five configuration paths remain exact;
- extra icon directories: none.

Final SHA-256 values are in `post-generation-inventory.sha256`.

## Runtime path boundary

The product master is a stable `desktop/src/assets/` path and the Tauri config
references only `icons/...`. Scoped source/config searches found no reference
to this task directory, `$CODEX_HOME`, `generated_images`, or
`generated-images`. A broader diagnostic search did find old `.trellis` fixture
paths in `desktop/src/api/fixtures/scan-report.real.json`; that pre-existing
test fixture is outside the icon runtime/config boundary and is unchanged by
this task. No claim is made that the entire repository contains no historical
`.trellis` string.

## 32px direct evidence

The exact 32x32 product output is copied without transformation to
`32x32-1x.png`; both hashes are
`09E39547593449C2BEC66E12541E53752548BB6A88C9F776C6ED27C2EDC7FB52`.
Direct 1:1 inspection passed the frozen visual gate.

Pillow pixel facts:

- size `(32, 32)`, RGBA, alpha extrema `(255, 255)`;
- all four corners `(3, 66, 52, 255)`;
- two largest rectangular light components are distinct slabs with bboxes
  `(10,12)-(21,15)` and `(10,18)-(21,20)`;
- crescent light component bbox `(5,12)-(19,25)`;
- overall light foreground bbox `(5,7)-(23,25)` and no light pixel touches an
  image edge.

The icon has two distinguishable slabs, one readable crescent, intact safe
inset, solid corners, and no checkerboard or halo.

## Windows build and package

```powershell
rtk just desktop-web-check
```

Exit `0`: type generation, lint, typecheck, 8 test files / 45 tests, and Vite
build passed. Captured output: `desktop-web-check-2026-08-30.log`.

```powershell
rtk just desktop-build
```

Exit `0`: release build and one NSIS bundle passed. Captured output:
`desktop-build-2026-08-30.log`.

Artifacts were inspected without installing them:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/devsweep-desktop.exe` | 8726016 | `3F263A756CF9F0D58A702B33ECD8B9A6D0EC43BA7488DB5CE34366928574E1DE` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 2136184 | `184138DE0C94D208CC8042359B949D364228390F46E3296EFC6AA0A2E2CEBC53` |

Those identities are the build launched for the native screenshots below. The
independent `trellis-check` then repeated `rtk just desktop-build` successfully;
the rebuilt ignored artifacts were an 8,726,016-byte executable with SHA-256
`52AB06E466958B8B5DFF80CF2537BFF9963FF0236EC8EAB5FBB9DBD4B1F5E0C3`
and a 2,134,955-byte NSIS bundle with SHA-256
`0E419FA92F713418C1545FC82DF7B705C64281136A359F8013F3A6BC53FCACF0`.
The build-instance hash difference did not change the 17-file product diff;
neither package was installed.

## Native Windows evidence and graceful shutdown

The built release executable, not a browser mock, was launched. Its native
window title was `devsweep`, window handle nonzero, and bounds were
`(26,26)-(840,664)`.

Evidence files:

| Evidence | Bytes | SHA-256 | Finding |
| --- | ---: | --- | --- |
| `windows-native-titlebar.png` | 5856 | `C6469CBACD426D168604E0C52766A38759A5492DE29D7452378004401A58BBE4` | 16px-class title-bar icon: two slabs and one crescent readable, unclipped, solid background |
| `windows-native-taskbar-devsweep.png` | 9334 | `CA06C1F91B2D0EED622A22C7140F0BC381705001C31C05D862AA446762CEFF2D` | UIA exact button `devsweep - 1 running window`, native icon readable with the same identity |
| `windows-native-window.png` | 23882 | `86E1C36552EB7E7AA9DF45B1664A5215430ED26AA8BB9D05744A667CFD208769` | Full real application window and native title bar |

The first full-taskbar capture used `Shell_TrayWnd` from the wrong monitor and
did not show DevSweep, so it was not accepted and was deleted. Windows UI
Automation then found exactly one `devsweep - 1 running window` button. A first
attempt to retain its COM element across enumeration returned stale zero bounds;
the still-running process was immediately recovered with `CloseMainWindow()`.
The changed method copied primitive bounds during enumeration and found
`Left=1196, Top=1380, Width=133, Height=60`, from which the accepted screenshot
was captured.

For the full-window run, the recorded process tree contained one
`devsweep-desktop.exe` root and its WebView2 descendants. `CloseMainWindow()`
returned true; the root exited within 15 seconds with code `0`; every recorded
owned PID and every `devsweep-desktop` process was absent after a two-second
settle. The UIA probe and accepted taskbar-capture run were also closed via
`CloseMainWindow()` and ended with zero DevSweep residue. No process was killed.

The current desktop header still contains the pre-existing `D` brand tile.
Replacing that visible shell mark belongs to
`08-29-desktop-shell-navigation-brand`; this task correctly changes only native
application/package icons and the stable handoff asset.

## Final gates and diff boundary

```powershell
rtk git diff --check
rtk just ci
```

Both exited `0`. `just ci` ran formatting check, offline zero-package update,
locked workspace all-target check/tests, and Clippy with `-D warnings`. The only
displayed warning was existing MSVC linker stdout while producing the desktop
DLL import library. Complete CI output is in `just-ci-2026-08-30.log`.

The independent `trellis-check` repeated `rtk just ci` after its product,
container, and native-evidence audit; it exited `0`. Its complete 25,601-byte
log is `independent-check-just-ci-2026-08-30.log`, SHA-256
`B2ED7A0B5506F747257CACB3EF08785CE9319DA37E65FE2AA794134EEBEB6057`.

`rtk git diff --name-only -- desktop` listed exactly the 16 regenerated tracked
icon files. `desktop/src/assets/devsweep-icon-master.png` is the sole untracked
product file in the owned desktop scope. There is no icon subdirectory and no
React, TUI, CSS, test, config, manifest, or lockfile product diff for this task.

## Shell handoff and verification boundary

- Stable import: `desktop/src/assets/devsweep-icon-master.png`.
- Source hash: `70527F3773669DDCE9FEFEB1CFC40D9FA5EFD5BF978CFC7BBB2F743DA8E308FD`.
- Semantics: intentionally opaque full-bleed icon; never describe it as
  transparent.
- Identity: two slabs, one crescent, one source sparkle; 32px and native Windows
  title/taskbar gates pass.
- Accessibility: when adjacent DevSweep text supplies the accessible name,
  treat the image as decorative. The shell owner decides the exact DOM/TUI
  implementation and owns responsive/accessibility tests.
- macOS/Linux: files decode and their frame inventories pass on Windows;
  native appearance remains `UNVERIFIED` because those hosts were not used.

No cleanup, installation, uninstallation, system-setting change, signing,
publication, release, commit, archive, or push was performed.
