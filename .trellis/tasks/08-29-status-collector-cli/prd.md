# Implement Status collectors and CLI contracts

## Goal

Own bounded Win32 snapshots, availability states, sampling scheduler, process budgets, cancellation, JSON/NDJSON, broken-pipe handling, and resource tests.

## Requirements

- R1: Collect CPU by two `GetSystemTimes` samples, physical memory by
  `GlobalMemoryStatusEx`, fixed volumes by logical-drive/type/free-space APIs,
  network deltas by `GetIfTable2`, battery by `GetSystemPowerStatus`, and bounded
  process CPU/memory/I/O via documented ToolHelp/process APIs.
- R2: Every metric is tagged available, partial, unavailable, permission-denied,
  or unsupported. GPU/VRAM/thermal/fan/SMART/physical-disk activity are static
  unsupported capabilities. Never map missing hardware or access to zero.
- R3: Snapshot window is 500 ms. Live defaults 2 s and accepts 1-60 s; process
  refresh is 4 s, volumes/battery 30 s. Default 15/max100 returned, max4096
  enumerated, 150 ms cooperative process-detail budget. One sample is in flight;
  missed ticks skip and cancel/channel-close joins.
- R4: Process DTO exposes name, PID, CPU, memory, and I/O only—no path, command
  line, user, environment, handles, or long-term history. Sort and truncate only
  after bounded enumeration with the exact enumerated/returned/requested counts,
  4096 ceiling, 150 ms budget, `truncated_by_limit`, and `budget_exhausted`
  metadata from the frozen Status V1 schema.
- R5: `status snapshot` emits one versioned JSON document; `status live` emits
  only the four frozen NDJSON event shapes until explicit cancel/broken pipe.
  The collector preserves the CLI-owned primitive types, units, UTC/monotonic
  time bases, availability union, sequence, and terminal lifecycle exactly.
  Broken pipe produces the internal terminal reason, performs no further write,
  quietly cancels/joins, and exits 0. No service, tray, scheduler, persistence,
  PowerShell, WMI, or vendor command.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3, R4): Adapter fixtures cover counter wrap/reset, zero elapsed time, interface/
      process churn, access denial, missing battery, non-fixed volumes, sleep-sized
      gaps, process ceiling/budget, and unsupported metrics.
- [ ] AC2 (R3): Scheduler tests with fake time prove 500 ms snapshot, interval clamps,
      per-metric cadence, one in flight, skipped ticks, no backfill, cancellation/
      join, coordinator refusal, and channel close.
- [ ] AC3 (R2, R4, R5): JSON/NDJSON fixtures are locale-invariant, bounded, privacy-limited,
      one event per line, exact for every frozen field/type/unit/time basis,
      availability variant and truncation flag/count, and terminate correctly
      on normal/cancel/error/broken-pipe paths with joined producer evidence.
- [ ] AC4 (R1, R3, R4, R5): Existing `windows-sys` feature changes are minimal and documented;
      the exact current/additional flags and API mapping in `design.md` compile,
      focused tests, native resource/process evidence, and `just ci` pass without
      a new monitoring dependency or unused broad feature.

## Out of Scope

- Unsupported metrics above, health score, alerts, persistence, background agent,
  privileged process data, remote host, or vendor-specific collector.
