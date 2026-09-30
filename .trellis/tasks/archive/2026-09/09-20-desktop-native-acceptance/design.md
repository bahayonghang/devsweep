# Technical design

## Acceptance matrix

Create one evidence row per mode, locale, width, scale, and interaction path.
Each row carries artifact hash, host/build metadata, fixture hash, expected
authority/lifecycle result, observed result, evidence paths, and status:
pass, fail, skipped, unavailable, or `UNVERIFIED`. Screenshots prove visual
state only; logs and process/resource samples prove lifecycle and resource
claims.

Status rules (TPR-07): `pass` and `fail` are measured results. A measured
`fail` freezes the row and is routed to the owning child; it is never
relabelled `UNVERIFIED`. `UNVERIFIED` is only for evidence that could not be
captured, with reason and owner. Rows inherited from the performance child
keep that child's status.

## Scale and width evidence (TPR-05)

Launch the release binary with isolated `LOCALAPPDATA` and the WebView2
`--force-device-scale-factor` argument for 100/125/150/200%, as the existing
driver does (`tools/measure-resources.ps1:552-554`); record `devicePixelRatio`
and a screenshot per scale. Record the unchanged current actual Windows
display scale from the host as its own row. No agent or operator changes
Windows display settings; other actual Windows scales are non-gating
`UNVERIFIED`. Label widths as real window size (the native window minimum is
900×600 per `desktop/src-tauri/tauri.conf.json:18-19`) or CSS viewport
emulation; a 390 or 800 px row that cannot be a real window size is recorded
as emulation, not as a native window.

## Lifecycle evidence (TPR-02, TPR-06)

Use the performance child's operation table for every cancel/restart row:
request, acknowledgement, and join events are those defined per operation
(Analyze `data-status`, Status `data-status`, Clean scan reducer events,
Software/Optimize `cancel_requested` to terminal result, wait-only Clean
dry-run/execute completion). Desktop Status stop rows repeat the same-live-PID
post-stop window from that child's design: the stop is the UI stop-live
control, the app PID is present in all 25 samples, and that child's desktop
quiescence predicate is applied (CPU final-five hold; no per-rep thread
clause; post-stop thread floor stable across the stops). Process-tree captures prove no orphan worker or child
CLI process; a CLI exit or a killed process is never a desktop quiescence
observation.

## Package and executable identity (TPR-03)

Inputs are the local build outputs of `just desktop-build`:
`target/release/devsweep-desktop.exe` and
`target/release/bundle/nsis/*-setup.exe`. Checks:

1. SHA-256 of both files, recorded with the commit and build log.
2. Version resource (PowerShell `VersionInfo` of each file) product name and
   product/file version match `desktop/src-tauri/tauri.conf.json`
   `productName` and `version`.
3. Authenticode status (`Get-AuthenticodeSignature`) recorded as `NotSigned`;
   any other status is a `fail` because the build is unsigned by contract.
4. Running identity: launch the hashed executable with isolated
   `LOCALAPPDATA`; the main window title is `DevSweep`
   (`tauri.conf.json:15`), the process name is `devsweep-desktop`, and the
   process executable path is the hashed file; the process tree shows only
   the app and its WebView2 children.

The installer is never executed. Install mode, shortcuts, uninstall entries,
and installed icon are non-gating `UNVERIFIED` rows.

## Rows handed off by the performance child

- `idle.max_threads_le_40`: measured `fail` in the performance child's
  release runs (45 and 46 threads on the idle desktop instance; an earlier
  run of the same binary measured 29-35). Decision (user, 2026-09-23): this
  child re-measures it natively. The row stays `fail` until a native run
  measures a pass; the archived threshold is not changed.
- Desktop binary identity: only the Tauri CLI build (`npm run tauri --
  build`, which `just desktop-build` runs) embeds the frontend assets. A
  plain `cargo build --release -p devsweep-desktop` binary loads `devUrl`
  and serves no application; it is never an acceptance artifact.
- Operation-table rows without release latency (Clean scan, Clean
  dry-run/execute, Software, Optimize, route change/window close): the
  performance child records them `UNVERIFIED` with this child as owner.

## Isolation and safety

Use a disposable `LOCALAPPDATA` and deterministic fixtures. Never point the
run at real cleanup targets or infer native behavior from a fixture-only pass.
Capture process tree and resource samples after cancellation and after
normal completion.

## Failure handling

Freeze the failing artifact and row, classify the failure by owning child, and
stop promotion of that criterion. Do not patch product code here, broaden the
matrix to hide a failure, or relabel missing evidence as a pass. The parent
receives the matrix and explicit remaining `UNVERIFIED` items.
