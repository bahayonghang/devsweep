# Design — Status parity and tray HUD

## Core (`crates/devsweep-core/src/status/`)

- `pdh.rs` (Windows only): open one PDH query per sampler with wildcard
  counters `\GPU Engine(*)\Utilization Percentage` and
  `\Thermal Zone Information(*)\Temperature`; collect twice across the
  existing sample window. Add the `Win32_System_Performance` feature to the
  existing `windows-sys` dependency; no new crate.
- GPU aggregation: instance names contain `phys_<n>` and `engtype_<type>`.
  Sum utilization per `(phys, engtype)`, take the maximum engine type per
  adapter, clamp to 100%. Report basis points like `CpuV1`.
- Thermal: Kelvin → tenths of °C; drop zones reporting 0 K as invalid.
- `StatusSnapshotV1` gains `gpu` and `thermal` `AvailabilityV1` fields; both
  join the degraded/warning aggregation. Update `UnsupportedCode` handling so
  a code is listed only when the probe is unavailable.
- CLI status presentation shows the new rows; machine JSON stays
  locale-neutral.

## Tauri

- `Cargo.toml`: `tauri = { version = "2.11.3", features = ["tray-icon"] }`.
- `tray.rs`: build the tray in `setup`; menu items "Open DevSweep" and "Quit";
  left-click toggles window `hud` (created lazily: 280x360, no decorations,
  skip taskbar, always on top, positioned from the tray click rect, hidden on
  blur).
- `HudSampler`: owns a `FlagCancelObserver` and a thread that calls the
  status sampler every 2 s while `hud` is visible, emits `hud-status` events
  to the `hud` window, and updates the tray tooltip. Hide → cancel + join.
  App exit (`RunEvent::ExitRequested`) → cancel + join before exit.
- Main window close → `app.exit(0)`; the tray is dropped with the app.
- Capabilities: allow the `hud` window the event listen permission only.

## Desktop

- `hud.html` + `src/hud/main.tsx` second Vite entry; renders compact metric
  rows from `hud-status` events through a closed decoder. No bridge commands.
- `modes/status/`: `processes.ts` pure sort/pin selectors; table header
  buttons with `aria-sort`; pin toggle per row.
- Status stage on `Stage` with CPU as the primary number.

## Rollback

GPU/thermal (core), process sort/pin (frontend), and tray HUD (Tauri + HUD
entry) are separable; revert the tray HUD alone if lifecycle checks fail.
