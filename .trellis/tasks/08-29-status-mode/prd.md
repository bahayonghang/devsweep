# Deliver bounded Windows Status mode

## Goal

Own read-only Windows snapshot/live collectors, CLI/TUI/Desktop presentation, availability semantics, resource budgets, and native evidence.

## Requirements

- R1: Deliver Status as a bounded read-only snapshot and explicit live mode using
  documented Win32 APIs through the existing dependency baseline, plus CLI/TUI/
  Desktop and native resource evidence.
- R2: Supported metrics are CPU, physical memory, fixed-volume capacity, network
  interfaces, battery/power, and bounded process CPU/memory/I/O. GPU/VRAM/
  thermal/fan/SMART/physical-disk activity are explicit unsupported/unavailable,
  never zero or synthesized health.
- R3: Complete collector/CLI before presentation. The umbrella owns schema parity,
  live lifecycle, resource gates, recursive evidence, and rollback only.
- R4: Apply numeric native resource gates on the recorded release-build host:
  snapshot p95 <=2 s, peak private bytes <=idle+64 MiB, threads <=idle+4;
  60-second live median CPU <=5% and p95 <=15% of one logical core, peak private
  bytes <=idle+64 MiB, threads <=idle+4; within five seconds after exit, threads
  return to <=idle+1 and CPU to <=idle+0.5 percentage point. Post-stop PASS uses
  exactly 25 samples at 200 ms over five seconds and requires both bounds in
  every one of the final five consecutive samples; any missing sample fails.
- R5: Treat the CLI task's `StatusSnapshotV1`, `AvailabilityV1`, and four live
  event shapes as the sole JSON/NDJSON/Tauri contract. Preserve integer
  units/time bases, process truncation metadata, sequence, and terminal/broken-
  pipe lifecycle unchanged across collector and presentation children.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3): Recursive children prove interval/process bounds, one sample, skipped
      ticks, cancel/join/broken-pipe, availability semantics, locale-invariant
      snapshots, exact wire fields/types/units/time bases and truncation
      metadata, no sensitive process fields, and no background work/history.
- [ ] AC2 (R1, R2, R5): CLI/TUI/Desktop match the same frozen snapshot/event bytes
      and clearly distinguish available/partial/unavailable/permission/
      unsupported in both languages; runtime decoders reject renamed, omitted,
      localized, or extra variants.
- [ ] AC3 (R1, R3, R4): Native resource evidence covers idle, snapshot, live, navigation exit,
      sleep/resume or interface/process churn where reproducible, and no overlap
      with heavy modes. Exit evidence contains all 25 scheduled samples and the
      final-five continuous-hold evaluation from the integration protocol.

## Out of Scope

- Background/tray service, notifications, persistent history, health score,
  vendor tools, PowerShell/WMI collectors, remote metrics, or privileged inspection.
