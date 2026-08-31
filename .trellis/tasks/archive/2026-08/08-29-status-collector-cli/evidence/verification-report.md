# Status collectors and CLI — implementer verification report

Implementer evidence only. This is **not** an independent trellis-check PASS.

Task: `.trellis/tasks/08-29-status-collector-cli`
Date: 2026-08-31
Host: Windows 11 专业版 build 26200, x64, Medium integrity (`S-1-16-8192`), `consent.exe` absent. No UAC, no signing, no new crates, no WMI/PowerShell/vendor collectors.

## Product delivered

One `StatusSampler` in `devsweep-core` plus frozen CLI `status snapshot` / `status live`.

- Snapshot waits 500 ms, live default 2 s clamped to 1–60 s, process refresh 4 s, volumes/power 30 s, one sample in flight, skipped ticks, cancel/join.
- Process DTO is name, PID, CPU, private bytes, I/O rates only. Enumeration ceiling 4096, 150 ms detail budget, default 15 / max 100 rows, truncation metadata set before sort/limit.
- Availability tags: available / partial / unavailable / permission-denied / unsupported. Missing hardware or access is never mapped to zero.
- GPU / VRAM / thermal / fan / SMART / physical-disk activity are static `unsupported`.
- JSON snapshot envelope plus four NDJSON events: `status_started`, `status_snapshot`, `tick_skipped`, `status_terminal`.
- Broken pipe: no second write, cancel/join, exit 0. Partial snapshot exit 5. Coordinator refusal is in-process (`status_busy`, exit 4).
- No persistence, service, tray, or `CleanupPlan`.

### windows-sys flags

Kept Optimize/analysis flags. Added:

| Feature | Proof of use |
| --- | --- |
| `Win32_NetworkManagement_IpHelper` | `GetIfTable2`, `FreeMibTable` in `status/network.rs` |
| `Win32_System_Diagnostics_ToolHelp` | `CreateToolhelp32Snapshot`, `Process32FirstW`, `Process32NextW` in `status/process.rs` |
| `Win32_System_Power` | `GetSystemPowerStatus` in `status/system.rs` |
| `Win32_NetworkManagement_Ndis` | `NET_LUID_LH` (windows-sys 0.59 gates `GetIfTable2` / `MIB_IF_ROW2` on Ndis) |

Already present at HEAD and used by Status: `Win32_System_ProcessStatus` (`GetProcessMemoryInfo`), `Win32_System_SystemInformation` (`GlobalMemoryStatusEx`), `Win32_System_Threading` (`GetSystemTimes`, `GetProcessTimes`), `Win32_Storage_FileSystem` (fixed volumes).

## R / AC trace

| Clause | Result | Evidence |
| --- | --- | --- |
| R1 / AC1 adapters, cadence, one sample, skip, cancel/join | PASS | Core `status::system` 6, `status::process` 6, `status::sampler` 6, all `status` 28 (`01`/`02`/`03`/`01b`). Native 500 ms window (~576–614 ms wall). |
| R2 supported metrics + static unsupported, never zero-fill | PASS | Native snapshot-1: cpu/memory/volumes/network/power `available`; processes `partial` with `process_limit` and `process_access_denied` (not empty available). Six unsupported capabilities. Fixtures in `status/tests.rs`. |
| R3 CLI JSON/NDJSON before presentation siblings | PASS | Frozen handlers only; `cli.rs` untouched. `04` CLI status 6; `04b` live pipe-close; `04c` wire fixtures. |
| R4 process privacy + native resource gates | PASS | DTO test omits path/cmdline/user/env/handles. Native: snapshot p95 586 ms ≤ 2 s; live private peak 3 997 696 ≤ idle 1 982 464 + 64 MiB; threads peak 5 ≤ idle 5 + 4; live CPU median 0% ≤ 5%, p95 7.654% ≤ 15% of one core; 25 post-exit samples, final five process-absent. |
| R5 frozen events, integer units, truncation, broken-pipe | PASS | Wire fixtures + native NDJSON `status_started`/`status_snapshot`. Truncation: enumerated ~653, returned 15, `truncated_by_limit` true. Broken-pipe exit 0. |
| AC4 minimal windows-sys + `just ci` | PASS | Cargo.toml as above. `rtk just ci` EXIT 0. |

## Focused gates (exact commands)

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test -p devsweep-core status::system --offline` | 0 (6 passed) | `evidence/01-rtk-core-status-system.log` |
| `rtk cargo test -p devsweep-core status::process --offline` | 0 (6 passed) | `evidence/02-rtk-core-status-process.log` |
| `rtk cargo test -p devsweep-core status::sampler --offline` | 0 (6 passed) | `evidence/03-rtk-core-status-sampler.log` |
| `rtk cargo test -p devsweep-core status --offline` | 0 (28 passed; includes `network` + `tests.rs`) | `evidence/01b-rtk-core-status-all.log` |
| `rtk cargo test -p devsweep-cli status --offline` | 0 (6 passed) | `evidence/04-rtk-cli-status.log` |
| `rtk cargo test -p devsweep-cli --test cli_contract json_and_ndjson --offline` | 0 (1 passed; live stdout close → exit 0) | `evidence/04b-rtk-cli-contract-json-ndjson.log` |
| `rtk cargo test -p devsweep-cli --test cli_contract status_wire --offline` | 0 (1 passed) | `evidence/04c-rtk-cli-contract-status-wire.log` |
| `rtk git diff --check` | 0 | `evidence/05-rtk-git-diff-check.log` |
| `rtk cargo clippy -p devsweep-core -p devsweep-cli --all-targets --offline -- -D warnings` | 0 | session clippy after Box/arg-pack fixes |
| `rtk just ci` | 0 | `evidence/06-rtk-just-ci.log`, full copy `evidence/06b-rtk-just-ci-full.log` |

`status::system` / `process` / `sampler` match the nested `mod tests` in those files (6 tests each). `tests.rs` and `network.rs` are covered by the honest `status` filter (28).

## Native x64 release

Binary: `target\release\devsweep.exe` PE `amd64 (0x8664)` sha256 `3E6CD1BF89AC27850F596AEE331B178AFDE5D2106097E71B2EAE2BF6C5DC112F`.

Harness: `evidence/native/harness.ps1`, log `evidence/native/harness.log`, numbers `evidence/native/summary.json`.

- Warm-up snapshot exit 5, 593 ms.
- Five snapshots exit 5 (partial), latencies 585, 586, 577, 576, 578 ms, p95 **586 ms**.
- CPU/memory/volumes/network/power available. Battery **present** (charge 9800 bp, AC online). Processes partial: enumerated 653–657, returned 15, `truncated_by_limit`, reason_codes `process_limit` + `process_access_denied`.
- Sample window 500 ms, 24 logical processors, six unsupported capabilities.
- Idle (live `--interval 60` after first sample): private median 1 982 464, threads median 5, CPU median 0%.
- 60 s live `--interval 2`: 300 × 200 ms samples, all present; raw user/kernel/cpu ms in `summary.json` `live60.raw_samples`. CPU-time delta % of one core: median **0**, p95 **7.654**. Private peak 3 997 696. Threads peak 5. NDJSON: `status_started` + 30 `status_snapshot`, `skipped_total` **0**.
- Post-exit: exactly 25 samples / 5 s, leftover same-exe count 0, final five process-absent.
- Interval 1 s → wire `interval_ms` 1000 exit 0; 60 s → 60000 exit 0; `--interval 0` and `61` clap exit 2.
- Broken pipe (close stdout after `status_started`): exit 0.
- Two overlapping OS live processes both stay alive (permit is in-process). After all runs, leftover release `devsweep.exe` count 0.

All recorded native resource gates passed.

## Remaining UNVERIFIED (with blockers)

| Clause | Completion-required? | Blocker |
| --- | --- | --- |
| Native sleep/resume sample-gap | No — unit tests own `is_sleep_sized_gap` (actual > max(4×expected, 10 s)) | Putting the host to sleep is not part of this child. |
| Native missing battery (`battery_present: false`, null charge) | No — `power_from_status` fixture | This host reports `battery_present: true`. |
| Native interface/process churn as a forced topology change | No — network/process pair-delta fixtures | Unplugging adapters or killing arbitrary processes is not required when fixtures exist. |
| CLI in-process coordinator refusal | No — `coordinator_refuses_a_second_live_session` + `status_busy` mapping | `LIVE_PERMIT` is process-local. Two OS processes both run, recorded as such. |
| 60 s live stop via console Ctrl+C | No — pipe-close exit 0 + unit cancel/join | Resource run used `Stop-Process` after 300 samples; join is proven on the broken-pipe path. |

Do not treat this file as trellis-check.
