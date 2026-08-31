# Implement - Status Collectors and CLI

Start only after later approval and the frozen CLI contract.

## 1. Add small Win32 adapters and tagged DTOs

1. Record the HEAD `windows-sys` flags (do not revert Optimize
   ProcessStatus/SystemInformation), then add only
   `Win32_NetworkManagement_IpHelper`, `Win32_NetworkManagement_Ndis`
   (required windows-sys 0.59 cfg gate for `GetIfTable2` / `MIB_IF_ROW2`),
   `Win32_System_Diagnostics_ToolHelp`, and `Win32_System_Power`; map each API
   as listed in `design.md` and prove no broader/unused flag.
2. Add the exact CPU, memory, fixed-volume, network, battery, and bounded process
   adapters with handle/table cleanup on every path.
3. Implement the CLI-owned `AvailabilityV1`/`StatusSnapshotV1` fields, integer
   units/time bases, process truncation metadata, and privacy-limited process
   fields before scheduler work.
4. Add counter reset/wrap, zero elapsed, churn, missing battery, permission, and
   4,096/150 ms process-bound fixtures.

Focused validation:

```powershell
rtk cargo test -p devsweep-core status::system
rtk cargo test -p devsweep-core status::process
```

Rollback point: remove collector modules and feature flags together; never map
an unavailable metric to zero.

## 2. Add scheduler and exact CLI streams

Implement 500 ms snapshot, 2 s default/1..60 s live, 4 s process, 30 s volume/
battery cadence, one sample, skipped ticks, cancel/join, and the exact four
frozen NDJSON events/sequence/terminal behavior. Broken-pipe tests assert no
second write, internal `broken_pipe`, joined producer, and exit 0.

```powershell
rtk cargo test -p devsweep-core status::sampler
rtk cargo test -p devsweep-cli status
```

Rollback point: unregister CLI Status and join the sampler; no background
thread/service/tray/persistence may remain.

## 3. Resource, native, and full gates

Run one warm-up plus five snapshots and one 60-second live sample with 200 ms
process sampling; record raw CPU-time deltas, private bytes, thread counts,
latency, skipped ticks, and post-exit quiescence against the Status parent limits.

```powershell
rtk git diff --check
rtk just ci
```

Native evidence covers sleep-sized gaps where reproducible, adapter/process/
interface churn, missing battery, permission denial, 1/60-second bounds,
broken pipe, coordinator refusal, and no background work. Stop for a new
monitoring dependency, WMI/PowerShell/vendor tool, sensitive process field,
resource threshold miss, or CLI change.
