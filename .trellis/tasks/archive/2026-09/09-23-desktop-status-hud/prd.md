# Status parity: GPU/thermal/battery, process sort and pin, tray HUD

## Goal

Close the Status gaps against the Mole desktop Status view where Windows has
a supported, non-elevated source: GPU utilization, thermal-zone temperature,
battery detail on the stage, a sortable process table with pinned rows, and a
tray HUD that shows the main metrics without opening the main window.

## Dependency and boundary

- Parent: `09-20-desktop-mole-tauri-performance`.
- Starts after `09-23-desktop-mole-capsule-shell`.
- Changes `devsweep-core/src/status/`, the Tauri status adapter and tray
  setup, `desktop/src-tauri/Cargo.toml` (enable the `tray-icon` feature of
  the existing `tauri` dependency), Tauri config/capabilities for one HUD
  window, generated wire types, and `desktop/src/modes/status/`.

## Requirements

- R1 GPU: core samples `\GPU Engine(*)\Utilization Percentage` through PDH
  and reports the maximum engine-type sum per adapter as
  `gpu: AvailabilityV1<GpuV1>`. When PDH or the counter is missing, the value
  is `unavailable` with a reason code; it is never zero. `GpuUtilization`
  leaves `unsupported_capabilities` only when the probe succeeds.
- R2 Thermal: core samples `\Thermal Zone Information(*)\Temperature`
  (Kelvin) through PDH and reports each zone in tenths of °C as
  `thermal: AvailabilityV1<ThermalV1>`. Missing counters → `unavailable`.
  Fan speed stays unsupported (no documented non-elevated Windows source).
- R3 Battery: the stage shows charge, AC state, and remaining time from the
  existing `PowerV1`; no battery → the line is absent, not zero.
- R4 Processes: the process table sorts by CPU, private bytes, or name
  (ascending/descending, keyboard reachable column headers with
  `aria-sort`). A pin control keeps up to 5 PIDs at the top across refreshes
  for the life of the Status view; a pinned PID that exits shows "exited"
  once and is then removed. No process kill or priority change.
- R5 Stage: the Status first screen shows the planet, CPU percentage as the
  primary number, and memory/GPU/temperature/battery as the secondary line,
  with an action to open the full dashboard detail view. Values update from
  the live stream while the mode is active.
- R6 Tray HUD: while the app runs, a tray icon exists. Its tooltip shows CPU
  and memory percentage. Left-click toggles a small borderless HUD window
  near the tray with CPU, memory, GPU, temperature, disk, network, and
  battery. The HUD samples every 2 s only while visible and stops and joins
  its sampler when hidden or when the app exits. Right-click menu: "Open
  DevSweep", "Quit". Closing the main window exits the app and removes the
  tray icon; there is no autostart and no background process after exit.

## Acceptance Criteria

- [ ] AC1: Core tests cover GPU and thermal parsing from PDH fixture values,
      missing-counter → `unavailable`, and no fake zero.
- [ ] AC2: Wire types regenerate; decoders reject unknown fields; desktop
      renders `unavailable` labels for GPU/thermal in both locales.
- [ ] AC3: Process sort and pin tests: sort order per key, `aria-sort`,
      pin limit 5, pinned exited PID handling.
- [ ] AC4: HUD sampler lifecycle test: start on show, stop and join on hide
      and on app exit; no sample after hide.
- [ ] AC5: Manual `just tdev` check: tray tooltip updates, HUD toggles, Quit
      exits, closing the main window leaves no `devsweep-desktop` process.
- [ ] AC6: `just ci`, desktop gates, and `types:generate -- --check` pass.

## Out of scope

- Fan control or fan speed, SMART, per-process GPU, process kill, priority
  change, autostart, menu-bar-style always-on HUD after the main window
  closes, and paired-device batteries.
