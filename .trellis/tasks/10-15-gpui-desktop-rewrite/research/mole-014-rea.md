# Mole 0.14.0 static inspection

Inspected 2026-10-16 with REA 6.5.0 (`rea-agents`, provider `rea-dotnet-static`).
Commands: `inspect-managed-artifact` and `inspect-managed-members`.
REA did not load the assemblies, resolve them through a CLR, decompile C#, or
execute the product. No Mole maintenance action was run on this machine.

Target: installed Mole 0.14.0, product version
`0.14.0+4d502f4d3a5b0fceb845f91cb0f406dfbdca15dd`, file version 0.14.0.0.
Publisher metadata says Tw93. The running image was
`%LOCALAPPDATA%\Tw93.Mole\current\Mole.exe`.
`Mole.Platform.dll` SHA-256
`b494bc859b89a4182dd03f7535113721e815d3d6169db87d714801843afcaa41`,
target framework `.NETCoreApp,Version=v10.0`, x64, CIL.

This note records observed type names, state names, localization keys, and
program/argv literals. It does not copy implementation bodies, assets, poems,
or commercial flows into DevSweep.

## Assembly split

| Assembly | REA coverage | Role observed from type names |
| --- | --- | --- |
| `Mole.dll` | complete, 520 types | WinUI shell, pages, overlays, localization keys |
| `Mole.Presentation.dll` | complete, 245 types | Per-mode view models and stage enums |
| `Mole.Platform.dll` | complete, 335 types | Windows adapters: optimize host, DISM, startup, status samplers |
| `Mole.Core.dll` | whole-assembly member decode hit the 64 MiB representation budget and returned no types; the optimize and clean types named below were selected explicitly and returned `coverage: complete` | Domain catalogues, path guards, scan and startup models |

`Mole.Core.dll` absence claims are limited to the selected types. Unselected
Core types are unknown, not empty.

## UI structure

The supplied Clean screenshot matches the landing composition implied by the
shell types: a custom window, a centered mode capsule, a planet, one line of
copy, and one primary button.

Shell types:

- `Mole.App.MainWindow` owns the window. `Mole.App.TabDefinition` supplies the
  mode tabs. `Mole.App.Views.ITabPage` is the page contract.
- `Mole.App.Services.Tool` names the avatar menu entries observed as fields:
  Settings, KeepScreenOn, CleanScreen, Doctor, CheckForUpdates, About.
- Pages: `CleanPage`, `SoftwarePage`, `OptimizePage`, `AnalyzePage`,
  `StatusPage`.
- Overlays, not extra top-level modes: clean review, clean result, details,
  settings, doctor, activation, onboarding, about, update, process explanation.
- `TrayPanelHost` and `TrayPanelView` are a separate compact status surface.
- Planet types (`PlanetKind`, `PlanetRenderer`, `PlanetMotion`, `PlanetView`,
  `PlanetGpu`) draw the hero. Loose files under `Assets\Planets` include earth,
  mercury, mars, jupiter, and sun. DevSweep may use its existing licensed Earth
  still. It does not package these files.
- Named brushes in string literals include a dark field and light foreground
  steps (`FgPrimaryBrush`, `FgSecondaryBrush`, `FgMutedBrush`, `SuccessBrush`,
  `EarthInkBrush`, `MercuryInkBrush`). No hex values were recovered. DevSweep
  keeps its own palette.

Stage enums, field names, coverage complete:

| Surface | States |
| --- | --- |
| Clean | Landing, Scanning, Found, AllClean, Cleaning, Completed, Failed |
| Software segments | Uninstall, Updates, Startup |
| Optimize | Landing, Running, Completed, Failed |
| Optimize activity | Repair, RequestingAdmin, RunningAdmin, RepairCheckingDisk, RepairRequestingAdmin, RepairingImage, Group, Task |
| Analyze | Landing, Scanning, Detail, Failed |
| Status | Landing, Live |

Clean landing is the idle hero. Scanning and later states replace that hero
with progress, then a review, then a result. Software leftovers exist as
`LeftoverRowViewModel`, not as a fourth segment. Optimize landing is a single
entry into grouped tasks. Analyze landing picks a drive, then scans into a
detail view that has both `TreemapCanvas` and rows. Status landing becomes a
live card and process table.

Clean string keys name sections: app, system, logs, developer, ai, browsers,
design, communication, leftovers, recycle bin. Two clean actions are named:
permanent clean, and move to recycle bin. DevSweep does not add permanent
delete. Browser, design, communication, and mail-like stores stay out of
automatic selection under the parent safety rules.

## Optimize behavior that can become CLI

`Mole.Platform.WindowsOptimizeHost` and `Mole.Platform.ElevatedOptimize`
implement the maintenance host. `Mole.Core.Optimize.IOptimizeHost` is the
domain surface: `Refresh`, `PrecheckAsync`, `RunAsync`, `RunElevatedAsync`,
`IsOnLowBattery`. Elevated work is one batch (`ElevatedCodec`) whose methods
are `SyncClock`, `Retrim`, `RetrimVolumes`, `ClearUpdateCache`, `ImageHealth`,
and `ImageRepair`. DevSweep keeps one operation per plan and does not add a
generic elevated script host.

Observed program and API facts:

| Reference key | Evidence | DevSweep CLI row |
| --- | --- | --- |
| DNS | `WindowsOptimizeHost.FlushDns` calls `DnsFlushResolverCache` | Keep `dns.flush` |
| Retrim | Elevated `defrag.exe` with argument `/L`. Precheck reads scheduled task `\Microsoft\Windows\Defrag\ScheduledDefrag`. Skip keys: not SSD, disk problem, recently optimized | `maintenance.retrim`, execute |
| Clock probe | `w32tm.exe` arguments observed as `/stripchart /computer:` plus ` /samples:3 /period:1 /dataonly` | `maintenance.clock_probe`, read-only execute. Fixed NTP host. Arguments stay separate argv elements |
| Clock sync | Elevated `w32tm.exe` `/resync`, service name `W32Time` | `maintenance.clock_sync`, separate execute plan |
| Image health | `NativeDism` calls `DismCheckImageHealth` and, separately, `DismRestoreImageHealth`. Skip keys: image repairable, image unrepairable, restart pending | `maintenance.image_scan` and `maintenance.image_repair`. Scan never starts repair |
| Dead startup | `DeadStartup.Remove` backs up Run values to a `.reg` file (`startup-` plus `yyyyMMdd-HHmmss`) before delete. Hives are HKCU, HKLM, and WOW6432Node `...\CurrentVersion\Run`, plus gone `.lnk` targets under the Startup folders | `maintenance.dead_startup`, execute, backup required. Same startup core as C05 |
| Update cache | Skip when `wuauserv` or `BITS` is busy, or a reboot is pending. Paths named `SoftwareDistribution\Download`. Delivery Optimization uses WMI `MSFT_DeliveryOptimizationFile` and checks `IsPinned` | `maintenance.update_cache`, guidance. Permanent delete stays unavailable, so this row does not execute |
| Disk health | `PhysicalDiskHealth` reads device id and health. `DiskHealth.Verdict` does not show a repair command | `optimize health`, read-only, no plan |
| Winget | Literal `winget.exe` only. `WingetRepair.Decide` and `FromExitCode` exist. No repair argv was present in the inspected user strings | `maintenance.winget_repair`, guidance, until argv is evidenced. Software update remains the supported winget path |
| Icon cache / Start search hosts | Strings `StartMenuExperienceHost`, `SearchHost`, `SearchApp`, and a `SHChangeNotify` method. `Process.Kill` is present on the host | `maintenance.icon_cache`, guidance. Optimize does not kill shell processes |
| Search service | Registry value `SYSTEM\CurrentControlSet\Services\WSearch\Start`. Skip key `win.optimize.skip.searchOff` | `maintenance.search`, guidance. Existing `settings.search` handoff stays |

Other skip conditions observed as keys or methods: low battery, admin required
for startup removal, action not needed, recently optimized. Local cooldown
state is `optimize.json`. DevSweep records cooldown in the existing optimize
audit, not in a copied state file.

`WindowsRegistryCleaner` is a separate platform type. It is not an optimize
catalogue command. DevSweep does not add a registry cleaner.

Group keys on the optimize page: `optimize.group.launchSpeed`,
`optimize.group.networkSearch`, `optimize.group.maintenance`. Task title keys:
icon cache, dead startup, dns, search, clock, retrim, image health, update
cache, winget, disk health, plus start-menu and search repair labels.

## What this does not establish

Static CIL does not prove runtime order, elevation UX, or which service
`NativeServices.Start` restarts. Winget repair arguments were not recovered.
Whole-assembly Core member inspection did not run. Screenshot numbers are not
measurements from this inspection. Mole license, activation, poems, planet
textures, and runner sprites stay out of DevSweep.
