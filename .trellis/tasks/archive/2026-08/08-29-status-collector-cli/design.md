# Design - Status Collectors and CLI

Define small Win32 adapter traits and one `StatusSampler`. A snapshot captures
baseline counters, waits cooperatively for the 500 ms window, then captures the
second sample and composes tagged metrics. Live owns one sampler future; a
deadline tick arriving while it runs is skipped. Slow metric groups reuse cached
tagged values only within their documented cadence and carry sample timestamps.

The process collector enumerates at most 4096 ids, stops detail work at 150 ms,
returns at most the requested capped count, and never opens query rights beyond
documented counters. All handles/tables are freed on every path. Counter reset or
topology churn yields partial/unavailable rather than negative or spiked values.

The core DTO and serializers implement the CLI contract's
`AvailabilityV1<T>`, `StatusSnapshotV1`, and `status_started`,
`status_snapshot`, `tick_skipped`, and `status_terminal` event objects without a
second adapter-local shape. System CPU is whole-system basis points; process CPU
is basis points of one logical core; byte/rate/time fields and UTC versus
monotonic clocks follow that contract. The process collector populates all
count/ceiling/budget/truncation fields before sorting/truncation. The Tauri type
generator consumes the same Rust DTO, and golden JSON -> decoder -> JSON fixtures
must round-trip without field loss or renaming.

## Exact windows-sys feature/API map

Keep the existing direct features
`Win32_Foundation`, `Win32_Security`, `Win32_Storage_FileSystem`,
`Win32_System_JobObjects`, `Win32_System_Threading`, and the Optimize-era
`Win32_System_ProcessStatus` / `Win32_System_SystemInformation` flags already
present at HEAD (do not revert them). Add only:

| Added feature | Required API |
| --- | --- |
| `Win32_NetworkManagement_IpHelper` | `GetIfTable2` and matching table free routine |
| `Win32_NetworkManagement_Ndis` | windows-sys 0.59 compile gate: `GetIfTable2`, `MIB_IF_TABLE2`, and `MIB_IF_ROW2` are `#[cfg(feature = "Win32_NetworkManagement_Ndis")]` because `InterfaceLuid` is `NET_LUID_LH` (crate source `windows-sys-0.59.0/src/Windows/Win32/NetworkManagement/IpHelper/mod.rs` lines 107–110 and 1655–1711). This is not an umbrella and is not optional if Status uses `GetIfTable2`. |
| `Win32_System_Diagnostics_ToolHelp` | `CreateToolhelp32Snapshot` and process enumeration |
| `Win32_System_Power` | `GetSystemPowerStatus` |

`GetProcessMemoryInfo` and `GlobalMemoryStatusEx` remain supplied by the
already-present ProcessStatus / SystemInformation flags. `GetSystemTimes`
remains supplied by the existing `Win32_System_Threading`; fixed-volume
enumeration remains supplied by the existing `Win32_Storage_FileSystem`. Cargo
feature inspection and Windows all-target compilation must prove every added
flag is used and no umbrella feature is enabled.

CLI serializers use the frozen envelope/events. Broken pipe creates the internal
terminal lifecycle reason, performs no second write to the closed sink, signals
cancellation, and joins the sampler before normal exit. Feature additions to
the existing `windows-sys` dependency are exactly the table above. Rollback
removes Status registration/features without persisted data migration.

The CLI consumes only frozen `status snapshot` and `status live` syntax,
including process/interval ranges and human-TTY versus NDJSON behavior; this
task does not edit parser definitions.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-core/Cargo.toml` | retain HEAD windows-sys flags (including Optimize ProcessStatus/SystemInformation) and add only IpHelper, Ndis (GetIfTable2 cfg gate), ToolHelp, and Power |
| `crates/devsweep-core/src/status/mod.rs` | Status service, V1 DTO, availability union |
| `crates/devsweep-core/src/status/system.rs` | CPU/memory/volume/battery adapters |
| `crates/devsweep-core/src/status/network.rs` | `GetIfTable2` topology/delta adapter |
| `crates/devsweep-core/src/status/process.rs` | bounded ToolHelp/process counters/privacy |
| `crates/devsweep-core/src/status/sampler.rs` | snapshot/live cadence, skip, cancel/join |
| `crates/devsweep-core/src/status/tests.rs` | fake-time/counter/churn/resource tests |
| `crates/devsweep-core/src/lib.rs` | export read-only Status service |
| `crates/devsweep-cli/src/application/commands/status.rs` | frozen snapshot/live JSON/NDJSON handler |
| `crates/devsweep-cli/src/application/presentation/status.rs` | bilingual human snapshot/live renderer |

The CLI contract task first creates the sole `application/commands/mod.rs` and
`application/presentation/mod.rs` roots; this task fills only the frozen
`commands/status.rs` and `presentation/status.rs` submodules.

No file persists Status samples. Rollback removes Status features/registration
and leaves no data migration.
