# Research: Status collectors for the new metrics, version strategy, GPUI chart API

- **Query**: implement.md step 1. For each new metric in design.md "core 契约", find the Windows source, elevation needs, cost, failure reason code and confidence. Decide V1-additive vs V2. List what breaks in the Tauri closed decoder. Confirm a GPUI canvas/path API at the pinned rev.
- **Scope**: mixed (repo code and specs, read-only probes on this host)
- **Date**: 2026-11-13
- **Probe host**: Windows 11 build 26300.9974 (26H2), **not elevated** (`IsInRole(Administrator) = False`), en-US UI, laptop with one Li-ion battery, NVIDIA + Intel GPUs plus three virtual display adapters (IDD).
- **Probe method**: `typeperf`, PowerShell `Get-ItemProperty`/`Get-CimInstance`, and in-memory C# P/Invoke via `Add-Type`, all read-only. The scripts are in the session scratchpad only. **Production code must not spawn any of these.** They were used only to check the APIs.

Confidence labels: **V** = verified by a probe on this host or by an existing call in the repo. **I** = inferred from documentation or headers only.

---

## 1. Current state (what already exists)

| File | Relevant facts |
|---|---|
| `crates/devsweep-core/src/status/mod.rs:62` | `STATUS_SNAPSHOT_VERSION = 1` |
| `crates/devsweep-core/src/status/mod.rs:194-209` | `StatusSnapshotV1` groups: cpu, memory, volumes, network, power, gpu, thermal, processes, unsupported_capabilities. **No `deny_unknown_fields`** on any Status struct. |
| `crates/devsweep-core/src/status/mod.rs:216-223` | `supported_group_is_degraded` excludes gpu/thermal, so optional probes do not change the outcome. |
| `crates/devsweep-core/src/status/mod.rs:383-392, 520-544` | `UnsupportedCode` = gpu_utilization, vram, thermal, fan, smart, physical_disk_activity. `physical_disk_activity` is **always** listed, and its reason is always `not_supported_v1`. |
| `crates/devsweep-core/src/status/pdh.rs` | One `PdhProbe` per sampler cache. It opens the query, adds English wildcard counters with `PdhAddEnglishCounterW`, primes once, then collects once per sample (`pdh.rs:158-235`). `read_counter` uses `PDH_FMT_DOUBLE \| PDH_FMT_NOCAP100` (`pdh.rs:271-273`). |
| `crates/devsweep-core/src/status/system.rs:229-250` | `GlobalMemoryStatusEx` is already called on every tick. Only `ullTotalPhys`/`ullAvailPhys` are used. |
| `crates/devsweep-core/src/status/system.rs:382-408` | Power comes from `GetSystemPowerStatus` only. |
| `crates/devsweep-core/src/status/network.rs:124-170` | `GetIfTable2` runs on every tick. `row_to_counters` reads LUID, Alias and octets. `MIB_IF_ROW2.Type`/`PhysicalMediumType`/`InterfaceAndOperStatusFlags` are already in the row but unused. |
| `crates/devsweep-core/src/status/sampler.rs:81-98, 357-548` | `SamplerCache` (live worker keeps it across ticks, snapshot builds a fresh one). Volumes and power refresh every 30 s (`VOLUME_POWER_REFRESH_MS`), processes every 4 s, while cpu/net/memory/PDH run every tick. There are no extra threads. |
| `crates/devsweep-core/src/optimize/windows.rs:106-130` | `os_build()` already calls `RtlGetVersion` (feature `Wdk_System_SystemServices`). |
| `crates/devsweep-core/src/software/arp.rs:295-340`, `optimize/maintenance.rs:1098-1125` | Registry reads use **local `#[link(name="advapi32")] extern "system"` declarations**, because the windows-sys feature `Win32_System_Registry` is **not** enabled. |
| `crates/devsweep-core/src/optimize/maintenance.rs:701-757` | Existing `CreateFileW(access 0)` + `DeviceIoControl` read-only pattern (seek-penalty query). |

Enabled features in `crates/devsweep-core/Cargo.toml` (windows-sys 0.61.2): `Wdk_System_SystemServices, Win32_Foundation, Win32_NetworkManagement_IpHelper, Win32_NetworkManagement_Ndis, Win32_Security, Win32_Storage_FileSystem, Win32_System_Diagnostics_ToolHelp, Win32_System_IO, Win32_System_Ioctl, Win32_System_JobObjects, Win32_System_Performance, Win32_System_Power, Win32_System_ProcessStatus, Win32_System_RestartManager, Win32_System_SystemInformation, Win32_System_Threading, Win32_UI_Shell, Win32_UI_WindowsAndMessaging`. `windows` 0.62.2 has only WinRT features. **No `sysinfo`, `wmi` or `winreg` in devsweep-core.** (`sysinfo` appears only transitively in `desktop-gpui/Cargo.lock`.)

### Existing reason codes (all are free-form `String` in `AvailabilityV1`)

`access_denied, api_unavailable, platform_unsupported, zero_elapsed, counter_reset, sample_gap, inconsistent_counters, interface_churn, not_sampled, volume_incomplete, process_limit, detail_budget_exhausted, process_access_denied, pdh_unavailable, counter_missing, collect_failed, counter_read_failed, no_valid_data, no_instances`.

`UnsupportedCapabilityV1` has exactly one reason code, `not_supported_v1`, and one state, `unsupported`. Both closed decoders accept any lowercase ASCII identifier as a reason code (`cli_contract.rs:1306-1316`; `contract.ts` `nonEmptyString`). So **new reason codes do not break either decoder.** New `UnsupportedCode` variants **do** break both decoders (closed enum).

---

## 2. Per-metric findings

| Metric | Source (API / counter / registry) | windows-sys feature | Elevation / driver / service / WMI / process | Cost and cadence | Unavailable reason (existing → new) | Conf. |
|---|---|---|---|---|---|---|
| **host.cpu_model** | Registry `HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0` value `ProcessorNameString` (REG_SZ; trim it). Probe value: `Intel(R) Core(TM) Ultra 9 275HX`. Alternative on x86/x64 only: CPUID brand string (`core::arch::x86_64::__cpuid` 0x80000002-4), which does not work on ARM64. | No new feature if it follows the arp.rs/maintenance.rs local-extern pattern. Otherwise add `Win32_System_Registry`. | None. HKLM read works as a standard user. | One registry read, under 1 ms. **Static: read once per sampler cache/session.** | `access_denied`, `api_unavailable`. New: none needed. | V (value read non-elevated); CPUID alternative I |
| **host.os_version** | `RtlGetVersion` gives major.minor.build (already in `optimize/windows.rs:110-124`). Registry `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` values `DisplayVersion` (`26H2`) and `UBR` (`9974`) give the friendly release and revision. **Do not use `ProductName`**: on this Windows 11 host it says `Windows 10 Pro` (V). | None, or registry as above. | None. | Under 1 ms. **Static.** | `api_unavailable` (RtlGetVersion status ≠ 0). Missing `DisplayVersion`/`UBR` gives a shorter string, not a failure. | V |
| **host.boot_time_unix_ms** | `unix_now_ms() - GetTickCount64()`. Probe: the tick count matched `Win32_OperatingSystem.LastBootUpTime` (19:51:26) within the probe's resolution. GetTickCount64 includes sleep time. With Fast Startup it reflects the last full kernel boot, the same as Task Manager uptime (I). Round to seconds so it does not jitter by ±1 ms between samples (I). | `Win32_System_SystemInformation` (enabled) | None. | Negligible. Compute once per session, or every tick (both are cheap). | Effectively cannot fail. Use `api_unavailable` defensively. | V |
| **disk_activity** (system read/write B/s) | PDH `\PhysicalDisk(_Total)\Disk Read Bytes/sec` and `\PhysicalDisk(_Total)\Disk Write Bytes/sec`, added to the **existing** `PdhProbe` query with `PdhAddEnglishCounterW`. These are rate counters, so the existing prime-then-collect cycle covers them. Probe: read 1.04 MB/s and write 8.2 MB/s, non-elevated. | `Win32_System_Performance` (enabled) | None. They are kernel perf counters with no service. Counters can be disabled by policy or `diskperf` (I), which shows up as `counter_missing`. | Two more counters on a query that is already collected every tick. Marginal cost (I). **Live loop.** | `pdh_unavailable, counter_missing, collect_failed, counter_read_failed, no_valid_data, no_instances` (all existing PDH codes) | V |
| **CpuV1.frequency_mhz** | PDH `\Processor Information(_Total)\Processor Frequency` (nominal MHz; probe value 2300) × `\Processor Information(_Total)\% Processor Performance` / 100 (probe value 193-199 %, so about 4.5 GHz effective, the same formula Task Manager uses). `% Processor Performance` exceeds 100, and `read_counter` already uses `PDH_FMT_NOCAP100`. **Rejected sources:** `CallNtPowerInformation(ProcessorInformation)` returned `CurrentMhz == MaxMhz` (2700/2100 P/E cores, static), and registry `~MHz` is static (3072). Both V. | `Win32_System_Performance` (enabled) | None. | Two more counters on the existing query. **Live loop.** | existing PDH codes as above | V |
| **MemoryV1.commit_used_bytes / commit_limit_bytes** | Already returned by the `GlobalMemoryStatusEx` call in `system.rs:237`: limit = `ullTotalPageFile`, used = `ullTotalPageFile - ullAvailPageFile`. Probe: limit 98 208 624 640 and used 64 149 155 840, **identical** to `GetPerformanceInfo` CommitLimit/CommitTotal × PageSize. Microsoft documents `ullTotalPageFile` as "the smaller of the system or current process limit", so the two can differ inside a Job with a commit limit (I). `GetPerformanceInfo` (feature `Win32_System_ProcessStatus`, enabled) is always system-wide. | none new | None. | Zero (same call) or one more cheap call. **Live loop.** | Same as memory: `api_unavailable`, `access_denied`, `inconsistent_counters` (avail > total) | V |
| **GpuAdapterV1.name** | `D3DKMTEnumAdapters2` + `D3DKMTQueryAdapterInfo(KMTQAITYPE_DRIVER_DESCRIPTION = 65)` → `D3DKMT_DRIVER_DESCRIPTION.DriverDescription`, then `D3DKMTCloseAdapter`. The key is the adapter LUID, so it joins directly to the PDH `adapter_id` prefix `luid_0x{High:08X}_0x{Low:08X}`. Probe: names for all 6 adapters (NVIDIA, Intel, Basic Render, three IDD virtual displays). `KMTQAITYPE_ADAPTERREGISTRYINFO (8)` failed with `0xC0000034` for Basic Render and the IDDs, so it is not suitable as the primary source. **Alternatives:** DXGI `IDXGIFactory1::EnumAdapters1` (`windows` feature `Win32_Graphics_Dxgi`, COM, I). Registry `HKLM\SOFTWARE\Microsoft\DirectX\{guid}\Description + AdapterLuid` (V): stale entries from earlier boots, and it **lacked one adapter**. One PDH LUID seen in a `typeperf -qx` listing (`0x1DA3F`) matched no D3DKMT adapter, so such entries get name `null`. | **New: `Wdk_Graphics_Direct3D`** (gdi32 thunks). DXGI would need `Win32_Graphics_Dxgi` on the `windows` crate. | None. These are user-mode gdi32 thunks, not COM or WMI. | Probe: about 0.4 ms for the full enumeration (including PowerShell interop). **Static per session.** Optionally refresh on the 30 s volume/power cadence or when PDH shows an unknown LUID, for eGPU/IDD hot-plug (I). | Per-adapter `name: null`. Enumeration failure has no existing code; `api_unavailable` fits. | V (D3DKMT probe non-elevated) |
| **PowerV1.full_charge_basis_points** | `SetupDiGetClassDevsW(&GUID_DEVCLASS_BATTERY, …, DIGCF_PRESENT\|DIGCF_DEVICEINTERFACE)` → `SetupDiEnumDeviceInterfaces` → `SetupDiGetDeviceInterfaceDetailW` → `CreateFileW(path, GENERIC_READ, FILE_SHARE_READ\|WRITE, OPEN_EXISTING)` → `IOCTL_BATTERY_QUERY_TAG` → `IOCTL_BATTERY_QUERY_INFORMATION(BatteryInformation)` → `BATTERY_INFORMATION{DesignedCapacity, FullChargedCapacity, Capabilities}`. Value = Full/Designed × 10000. Probe: designed 99072, full 99072, caps 0x80000000 (system battery, not `BATTERY_CAPACITY_RELATIVE`). If `BATTERY_CAPACITY_RELATIVE` is set, the units are not mWh, so health and rate are not meaningful (I). Full can exceed Designed on new packs (I). `CM_Get_Device_Interface_ListW(&GUID_DEVICE_BATTERY)` is an equivalent enumerator. | IOCTL codes, `BATTERY_*` structs, and `GUID_DEVICE_BATTERY` are in `Win32_System_Power` (enabled). `CreateFileW` and `DeviceIoControl` are enabled. **New: `Win32_Devices_DeviceAndDriverInstallation`** for SetupDi/CM, or a local `extern "system"` declaration following the registry precedent. | **No elevation (V).** `GENERIC_READ` alone was enough for TAG/INFO/STATUS. Microsoft's sample uses READ\|WRITE, which is not needed here. Uses the in-box battery class driver; no new driver, service, or WMI. | Probe: about 0.44 ms for enumerate + open + 3 IOCTLs. Static-ish: refresh with the existing 30 s power cadence. | No battery: keep the existing invariant (`battery_present:false`, every battery field `null`). New codes needed: `no_battery_device` (power status says present but no device found), `capacity_relative`, `not_reported` (designed = 0). | V |
| **PowerV1.cycle_count** | `BATTERY_INFORMATION.CycleCount` (same IOCTL). Probe: **0**. `root/wmi BatteryCycleCount` also gave 0. Firmware that does not track cycles reports 0 (I), so treat 0 as "not reported", not as a real zero. | as above | as above | Same call. 30 s cadence. | New: `not_reported` | V (value 0); "0 = not reported" I |
| **PowerV1.rate_milliwatts** | `IOCTL_BATTERY_QUERY_STATUS` with `BATTERY_WAIT_STATUS{BatteryTag, Timeout:0}` → `BATTERY_STATUS.Rate` (signed mW: negative = discharging, positive = charging). `BATTERY_UNKNOWN_RATE` (0x80000000) means unknown. Probe: `PowerState=0x1` (AC online), `Rate=0`, Capacity 94118 mWh. Rate 0 on AC while not charging is a real measurement. Multi-battery machines: sum the rates (I). | as above | as above | About 0.1 ms per call (I, part of the 0.44 ms total). At the 30 s power cadence the rate can be up to 30 s old. `age_ms` already reports this. | New: `unknown_rate`, `capacity_relative` | V |
| **NetworkInterfaceV1.kind** | `MIB_IF_ROW2.Type` from the **existing** `GetIfTable2` row: `IF_TYPE_ETHERNET_CSMACD = 6` → ethernet, `IF_TYPE_IEEE80211 = 71` → wireless, everything else (24 loopback, 131 tunnel, 23 PPP, 243 WWAN, 53 proprietary virtual) → other. Constants are in `Win32_NetworkManagement_IpHelper` (enabled). Probe (`Get-NetAdapter -IncludeHidden`): **Hyper-V `vEthernet`, `vSwitch`, Bluetooth PAN and WAN miniports also report type 6**. Only `InterfaceAndOperStatusFlags.HardwareInterface` (V: True only for physical `以太网` and `WLAN`) or `PhysicalMediumType` (802.3 = 14, Native 802.11 = 9) separates physical from virtual. | none new | None. | Zero extra calls. **Live loop.** | Mapping always yields a value on Windows, so `None` happens only on non-Windows or for unmapped types, depending on the open question below. | V |

### Placement in the sampler (cadence summary)

- **Static, once per `SamplerCache`**: host (cpu_model, os_version, boot_time), GPU names. The snapshot path builds a fresh cache, so a one-shot snapshot pays about 1 ms once.
- **Every tick, inside the existing calls**: disk_activity and frequency (4 more counters on the one `PdhProbe` query, still one `PdhCollectQueryData`), commit (same `GlobalMemoryStatusEx`), network kind (same `GetIfTable2` rows).
- **30 s power cadence**: battery IOCTLs, alongside `GetSystemPowerStatus`.
- **No new threads, no process spawning, no WMI/COM, no drivers or services** with the D3DKMT route for GPU names. DXGI would be COM.

---

## 3. Version strategy

### Spec text

- `quality-guidelines.md:76-78`: "JSON uses one V1 envelope with `{schema_version,command,outcome,data,warnings,error}`. Status NDJSON emits closed V1 lifecycle events …"
- `quality-guidelines.md:108`: "wrong Status field/type/unit/terminal … -> test/build failure."
- `quality-guidelines.md:117-118`: "the CLI and future Tauri Status fixtures decode through one closed V1 test contract and are byte-identical"
- `quality-guidelines.md:137-139`: "Closed Status snapshot/live fixture tests … plus byte-identical CLI/Tauri fixtures."
- `bounded-delivery.md` §3: "JSON remains one V1 envelope, and NDJSON retains its closed record schema."
- Additive-in-V1 precedents: `quality-guidelines.md:219-222`: "`ExecutionEvidence.estimated_bytes` is optional and additive (`serde(default, skip_serializing_if …)`; the record keeps `deny_unknown_fields`) … Old lines parse with `None`." `scan-coverage.md:17-33, 55-57`: `coverage` is added with `serde(default)`, "ScanReport remains version 1", and missing legacy coverage decodes as empty.
- V2 precedent: `database-guidelines.md` "Migrations" and `desktop-preferences.md` (closed V2 with read-only V1 migration). This applies to **persisted files**. Status snapshots are never persisted (`status/mod.rs:1-3`: "never persists samples").
- Status-specific precedent: commit `9755a22` added the **required** `gpu` and `thermal` groups to `StatusSnapshotV1`. It kept `schema_version: 1`, updated both closed decoders, the fixtures and the generated types in the same change, and recorded the contract as the "Status PDH probes" scenario (`quality-guidelines.md:635-720`).

### Conclusion: stay on V1, extend additively in one lockstep change

The new fields can stay in `StatusSnapshotV1`, `CpuV1`, `MemoryV1`, `GpuAdapterV1`, `PowerV1` and `NetworkInterfaceV1`, with `schema_version` remaining 1. Reasons:

1. "Closed V1" in this repo means every consumer-side decoder enumerates the exact key set. It does not mean the schema is frozen forever. The gpu/thermal extension did exactly this and stayed V1.
2. Status JSON is a command/IPC boundary, not a persisted store. No old file needs migrating, and the only closed consumers are in this repo and change in the same commit.
3. A V2 bump would break `tools/status-sink-smoke.py:159` and `:174`, which assert `schema_version == 1`. It would also force a dual-version decoder in both the CLI tests and Tauri for no reader that actually needs V1.

Conditions for staying V1:
- **Rust read compatibility** (design: "old snapshot JSON is still readable"). New `Option<T>` struct fields already deserialize missing keys as `None` with serde derive. The new **group** fields `host` and `disk_activity` are `AvailabilityV1<…>`, not `Option`, so they need `#[serde(default = …)]` returning, for example, `unavailable("not_sampled")`. Otherwise an old JSON without `host` fails to deserialize. No Status struct uses `deny_unknown_fields`, so older Rust readers already ignore the new keys.
- **Serialization shape**: existing Status `Option` fields serialize as explicit `null` (for example `PowerV1.charge_basis_points`), and both closed decoders expect the key to be present. To match, new optional fields should be emitted as `null`, without `skip_serializing_if`. The audit-record precedent uses `skip_serializing_if`, so this differs from it.
- Parent design.md:33 asks for this conclusion to be copied into `11-12-gpui-mole-parity-airy-glass/research/`. That is outside this agent's write scope; the main agent should copy it.

### What breaks in the Tauri closed decoder (`desktop/src/api/contract.ts`) and what must change

| Location | Breaks because | Required change |
|---|---|---|
| `SNAPSHOT_KEYS` `:1474-1477` + `exact(input, SNAPSHOT_KEYS)` `:1663` | The new top-level `host` and `disk_activity` keys are unknown, so **every** snapshot is rejected. That affects the Status page, live `status_snapshot` events and the HUD `hud-status` snapshot payload. | Add both keys. Add `decodeHostValue` and `decodeDiskActivityValue` through `decodeAvailability`. |
| `decodeCpuValue` `:1517` | `exact(["system_utilization_basis_points"])` | Add `frequency_mhz` (nullable unsigned). |
| `decodeMemoryValue` `:1523` | exact key list | Add `commit_used_bytes` and `commit_limit_bytes` (nullable). Consider adding the invariant used ≤ limit. |
| `decodeNetworkValue` iface `:1556` | exact key list | Add `kind` (nullable `oneOf` of the closed enum). |
| `decodePowerValue` `:1569-1575` | exact key list, plus the invariant "missing battery must not report charge" | Add the 3 nullable fields. Extend the invariant so `battery_present:false` requires all of them to be null. |
| `decodeGpuValue` adapter `:1590` | exact key list | Add `name` (nullable string). |
| `UNSUPPORTED_CODES` / expected-list check `:1471-1473, :1667-1672` | Today `physical_disk_activity` is always expected. If core makes it conditional on `disk_activity` having a value (like gpu/thermal), the decoder must apply the same rule, or it throws "unsupported capabilities do not match". Any new `UnsupportedCode` must be added here. | Mirror whatever rule core adopts (see open question 2). |
| `desktop/src/api/types.gen.ts` | Generated by `desktop/scripts/generate-types.mjs:54-57` from fixtures `status/snapshot.json`, `status/live-snapshot.json`, `status/snapshot-completed.json` and `status/hud-snapshot.json`. Do not hand-edit it. | Regenerate after the fixtures are updated. Fixtures should include both a value and a `null` for each nullable field so quicktype infers `T \| null` (I). |
| Fixtures | `desktop/src/api/fixtures/status/{snapshot,snapshot-completed,event-snapshot,live-snapshot,hud-snapshot}.json`; `desktop/src/api/fixtures/contract-variants.json` (`DesktopStatusSnapshotResult` around `:236-262`, status events around `:298-343`) | Update all of them. `desktop/src/api/fixtures/status/snapshot.json` is **byte-identical** to `crates/devsweep-cli/tests/fixtures/status/snapshot.json` (V, `cmp`), and they must stay so. |

The same applies on the CLI side. `crates/devsweep-cli/tests/cli_contract.rs` contains its own closed decoder (`status_v1_contract`: `ClosedObject::finish` rejects unknown fields at `:739-746`, structs around `:470-600`, `UnsupportedCode` at `:574-581`, expected-list logic around `:837`, and `verify_closed_schema_rejections` at `:1561+`). It compares `fixtures/cli/status-snapshot.json` against `status-snapshot.tauri.json` and `status-live.ndjson` against `status-live.tauri.ndjson` (`:1361-1425`). All four fixtures plus `fixtures/status/snapshot.json` need the new fields.

Rust struct-literal sites that stop compiling when fields are added (they are fixed in the same change): devsweep-core `status/{health,mod,network,pdh,sampler,system,tests}.rs`; devsweep-cli `presentation/status.rs` (6), `tui/modes/status/tests.rs` (1), `tests/cli_contract.rs` (12); `desktop/src-tauri/src/{hud,tray}.rs` (1 each); `desktop-gpui/src/status_panel.rs` (4). `status/health.rs:386` maps `UnsupportedCode` to a string and must cover any new variant.

Tool scripts: `tools/status-sink-smoke.py` checks only `schema_version == 1` and the envelope shape, so it is unaffected by V1 additions (V by reading `:153-180`). `tools/summarize-native-status.py` reads resource-sampler output, not snapshot fields, so it is unaffected (V by reading). Both should still be run once per implement.md step 5.

---

## 4. GPUI: line chart API at the pinned rev

- Pin: `desktop-gpui/Cargo.toml` uses `gpui`, `gpui_platform` and `gpui_windows` from Zed rev `322cd509dcccaee1f6bc44725d270598ae8171ad`. `gpui_windows` is patched to `desktop-gpui/vendor/gpui_windows` (same commit per `vendor/gpui_windows/NOTICE.md`; only the idle frame pump and zoom are changed).
- Source checked: `~/.cargo/git/checkouts/zed-a70e2ad075855582/322cd50/crates/gpui/`
  - `src/elements/canvas.rs:10-19`: `pub fn canvas<T>(prepaint: FnOnce(Bounds<Pixels>, &mut Window, &mut App) -> T, paint: FnOnce(Bounds<Pixels>, T, &mut Window, &mut App)) -> Canvas<T>`, re-exported via `elements/mod.rs:17`.
  - `src/path_builder.rs:25` `PathBuilder`, `:88` `stroke(width: Pixels)`, `:96` `fill()`, `:125` `move_to`, `:131` `line_to`, `:244` `build() -> Result<Path<Pixels>, Error>` (lyon tessellation, `Cargo.toml:113`), re-exported at `src/gpui.rs:194`.
  - `src/window.rs:4773` `Window::paint_path(&mut self, path: Path<Pixels>, color: impl Into<Background>)` (paint phase only).
  - Working example: `examples/painting.rs` (imports `canvas, PathBuilder, StrokeOptions`).
  - The vendored Windows renderer draws path batches: `vendor/gpui_windows/src/directx_renderer.rs:402-404` (`PrimitiveBatch::Paths` → `draw_paths_to_intermediate`). It uses an MSAA intermediate pass for paths (`:28`), which costs extra GPU work on frames that contain paths (I). This only matters on redraw.
- `desktop-gpui/src` has no current use of `canvas`, `PathBuilder` or `paint_path` (grep found none).
- **Conclusion: a 60-point polyline is supported** (`canvas` + `PathBuilder::stroke` + `move_to`/`line_to` + `paint_path`). It needs no animation frame (`request_animation_frame` at `window.rs:2742` is not required) and repaints only when the view is notified. The bar fallback (60 `div`s) is not needed for API reasons, but remains possible.

---

## 5. Summary

| Metric | Available on this host? | Approach | New dependency/feature |
|---|---|---|---|
| host.cpu_model | Yes (V) | Registry `ProcessorNameString`, static | none (local extern) or `Win32_System_Registry` |
| host.os_version | Yes (V) | `RtlGetVersion` + registry `DisplayVersion`/`UBR`, static; not `ProductName` | none / registry as above |
| host.boot_time_unix_ms | Yes (V) | now − `GetTickCount64`, rounded to seconds | none |
| disk_activity read/write B/s | Yes (V) | PDH `PhysicalDisk(_Total)` on the existing query, every tick | none |
| CpuV1.frequency_mhz | Yes (V) | PDH `Processor Frequency` × `% Processor Performance`, every tick | none |
| MemoryV1.commit_used/limit | Yes (V) | `GlobalMemoryStatusEx` page-file fields (or `GetPerformanceInfo`), every tick | none |
| GpuAdapterV1.name | Yes (V) | D3DKMT driver description joined by LUID, static | `Wdk_Graphics_Direct3D` |
| PowerV1.full_charge_basis_points | Yes (V; 10000 here) | SetupDi + battery IOCTL INFORMATION, 30 s | `Win32_Devices_DeviceAndDriverInstallation` (or local extern) |
| PowerV1.cycle_count | Firmware reports 0 here, so `null`/`not_reported` (V) | same IOCTL | same |
| PowerV1.rate_milliwatts | Yes (V; 0 on AC idle) | battery IOCTL STATUS, 30 s | same |
| NetworkInterfaceV1.kind | Yes (V) | `MIB_IF_ROW2.Type` (+ `HardwareInterface`), every tick | none |

**Version strategy:** stay on `schema_version: 1` with additive fields, as in commit `9755a22` (gpu/thermal). Update the core model (with serde defaults for the new groups), both closed decoders (CLI test and Tauri `contract.ts`), all byte-identical fixtures and `types.gen.ts` in one change. No V2.

**Elevation:** nothing above needs elevation, a new driver, a service, WMI or process spawning. All of it was verified non-elevated on this host. WMI `root/wmi` battery classes also worked non-elevated, but `BatteryStaticData` failed with "Generic failure", so WMI was not used.

---

## 6. Open questions for the user (real decisions only)

1. **How to report the reason for a missing field inside an existing group.** `Option` + `null` carries no reason, yet PRD R1 says every item has a value or a reason. Options:
   - (a) Extend the closed `UnsupportedCode` list with the new capabilities (for example `cpu_frequency`, `battery_health`, `battery_cycle_count`, `battery_rate`, `gpu_name`) and list them when null. This needs a decoder change in both places.
   - (b) Wrap each new field in its own `AvailabilityV1<T>`. This is heavier JSON, but it gives a per-field reason code.
   - (c) Mark the parent group `partial` with a reason code. This flips the envelope `outcome` to `partial` (exit 5) on every desktop without a battery, because cpu/memory/power are in `supported_group_is_degraded`. It is likely unwanted.
2. **`physical_disk_activity` in `unsupported_capabilities`.** Should it become conditional on `disk_activity` having a value, the way gpu/thermal work? That changes the meaning of an existing V1 list (core `tests.rs:40,150`, `cli_contract.rs:837`, `contract.ts:1668`). Should `host` and `disk_activity` also stay out of the outcome check, as gpu/thermal do?
3. **New windows-sys features.** `Wdk_Graphics_Direct3D` (GPU names) and `Win32_Devices_DeviceAndDriverInstallation` (battery enumeration), plus optionally `Win32_System_Registry`. Is adding them acceptable, or should the repo's local `extern "system"` pattern be used instead? The existing PDH scenario text records "Only the windows-sys feature `Win32_System_Performance` is added".
4. **`NetworkKind` for virtual adapters.** Hyper-V vEthernet, vSwitch, Bluetooth PAN and WAN miniports all report Ethernet type 6. Should they map to `other` using `HardwareInterface`, or should the closed enum gain a `virtual` value?
5. **Battery semantics.** Should `full_charge_basis_points` above 10000 (full > designed) be clamped or kept as is? Should a cycle count of 0 always mean `not_reported`? Is a battery rate up to 30 s old (power cadence) acceptable for the live tile, or should the STATUS IOCTL (about 0.1 ms) run every tick?
6. **`os_version` format.** For example, `"10.0.26300.9974 (26H2)"`, or a product label ("Windows 11") derived from build ≥ 22000? Note that the registry `ProductName` is unreliable.

## Caveats / Not found

- Cost figures came from PowerShell/C# interop and are only orders of magnitude. The PDH marginal cost of four `_Total` counters was not measured (I).
- Only one host was probed (a laptop with a battery). The no-battery path relies on the existing `GetSystemPowerStatus` flag 128 logic plus SetupDi returning no devices (I). ARM64, VMs and older Windows builds (whether `KMTQAITYPE_DRIVER_DESCRIPTION` exists before WDDM 2.x) were not probed (I).
- The PDH LUID `0x1DA3F`, seen once in a counter listing, had no D3DKMT match. It is assumed to be a transient or stale adapter instance (I).
