# Independent trellis-check — status-collector-cli

Task: `.trellis/tasks/08-29-status-collector-cli`
Checker: trellis-check sub-agent (direct verify + self-fix)
Date: 2026-08-31
Host: Windows 11 专业版 build 26200, x64, Medium integrity (`S-1-16-8192`), `consent.exe` count 0.
Binary: `target\release\devsweep.exe` PE `amd64 (0x8664)` sha256 `3E6CD1BF89AC27850F596AEE331B178AFDE5D2106097E71B2EAE2BF6C5DC112F`.

Implementer `evidence/verification-report.md` was treated as a clue only. All gates below were re-run. Native logs under `evidence/independent-native/` replace implementer native evidence for this verdict.

No git commit / push / merge / amend. Protected paths (`README.md`, `justfile`, `.trellis/.gitignore`, other `.trellis/tasks/*`) were not modified by this check. Optimize `windows-sys` flags were not reverted.

## Overall: **PASS**

Completion-required UNVERIFIED: **no**.

---

## Independent gates (command + exit)

| Log | Command | Exit |
| --- | --- | --- |
| `evidence/independent-core-status-system.log` | `rtk cargo test -p devsweep-core status::system --offline` | **0** (6 passed; nested `status::system::tests`) |
| `evidence/independent-core-status-process.log` | `rtk cargo test -p devsweep-core status::process --offline` | **0** (6 passed) |
| `evidence/independent-core-status-sampler.log` | `rtk cargo test -p devsweep-core status::sampler --offline` | **0** (6 passed) |
| `evidence/independent-core-status-all.log` | `rtk cargo test -p devsweep-core status --offline` | **0** (28 passed) |
| `evidence/independent-cli-status.log` | `rtk cargo test -p devsweep-cli status --offline` | **0** (6 passed) |
| `evidence/independent-cli-contract-json-ndjson.log` | `rtk cargo test -p devsweep-cli --test cli_contract json_and_ndjson --offline` | **0** (1 passed; live stdout close → exit 0) |
| `evidence/independent-cli-contract-status-wire.log` | `rtk cargo test -p devsweep-cli --test cli_contract status_wire --offline` | **0** (1 passed) |
| `evidence/independent-git-diff-check.log` | `rtk git diff --check` | **0** |
| `evidence/independent-fmt-check.log` | `rtk cargo fmt --all -- --check` | **0** |
| `evidence/independent-release-build.log` | `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` | **0** |
| `evidence/independent-just-ci.log` (+ full `independent-just-ci-full.log`) | `rtk just ci` | **0** (`ci complete`; clippy `-D warnings`; all workspace suites `test result: ok`) |
| `evidence/independent-native-harness.log` | `powershell -File evidence/independent-native/harness.ps1` | **0** |
| `evidence/independent-snapshot-partial-privacy.log` | snapshot JSON inspect + extra broken-pipe recapture | **0** |

Honest filter note: `status::system` / `process` / `sampler` do **not** execute `status/tests.rs` or `status/network.rs`. Those are covered by `status` (28 = system 6 + process 6 + sampler 6 + network 4 + `tests.rs` 4, plus two other `status`-substring matches in the crate). CLI `status` does not run `cli_contract`; those were run separately.

---

## Native ledger (independent recapture)

Harness: `evidence/independent-native/harness.ps1` (copy of implementer harness, outputs isolated). Summary: `evidence/independent-native/summary.json`. Protocol: warm-up + five snapshots + idle live + 60 s live at 200 ms process sampling + post-exit 25×200 ms + interval bounds + broken pipe + two-OS-process overlap.

| Gate | Parent limit | Independent result | Pass |
| --- | --- | --- | --- |
| Snapshot p95 | ≤ 2000 ms | 611 ms (586, 589, 591, 606, 611) | yes |
| Live private peak | ≤ idle + 64 MiB | 4 362 240 ≤ 2 150 400 + 67 108 864 | yes |
| Live threads peak | ≤ idle + 4 | 5 ≤ 5 + 4 | yes |
| Live CPU median | ≤ 5% of one core | 0% | yes |
| Live CPU p95 | ≤ 15% of one core | 7.802% | yes |
| Post-exit samples | exactly 25 / 5 s | 25 | yes |
| Final five hold | process absent (stronger than idle+1 thread / idle+0.5 pp) | all five absent; leftover 0 | yes |
| Live duration / sampling | 60 s @ 200 ms | 300 present / 300 | yes |
| NDJSON | started + snapshots until stop | `status_started` + 30 `status_snapshot`; `skipped_total` 0 | yes |
| Interval 1 / 60 | wire 1000 / 60000, exit 0 | yes | yes |
| Interval 0 / 61 | clap exit 2 | yes | yes |
| Broken pipe | exit 0, join, no leftover | harness + extra recapture | yes |
| Leftover release exe | 0 | 0 after all runs | yes |
| Two OS live processes | in-process permit only | both stayed alive | expected |

Warm-up and all five snapshots: **exit 5**, `outcome=partial`, `sample_window_ms=500`, 24 logical processors, six static unsupported capabilities.

---

## Skeptical items

### 1. `Win32_NetworkManagement_Ndis` vs design table — **PASS** (documented; strictly required)

windows-sys **0.59.0** crate source (`C:\Users\lyh\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\windows-sys-0.59.0\src\Windows\Win32\NetworkManagement\IpHelper\mod.rs`):

```
#[cfg(feature = "Win32_NetworkManagement_Ndis")]
windows_targets::link!(... fn GetIfTable2(table : *mut *mut MIB_IF_TABLE2) ...);
```

`MIB_IF_ROW2` / `MIB_IF_TABLE2` are the same cfg (lines 1655–1711) because `InterfaceLuid: super::Ndis::NET_LUID_LH`. Enabling only `Win32_NetworkManagement_IpHelper` does **not** compile `GetIfTable2`. Ndis is a typed-binding compile gate, not an unused umbrella and not extra NDIS driver APIs.

Self-fix this child owns: `design.md` / `implement.md` now list Ndis with that citation; `Cargo.toml` comment records the cfg. HEAD Optimize `Win32_System_ProcessStatus` / `Win32_System_SystemInformation` were **already present** and were kept (not re-added, not reverted).

### 2. Snapshot exit 5 (`partial`) — **PASS** (truthful availability; never missing→zero)

Independent `snapshot-1.json`:

- Envelope `outcome=partial`, warnings `process_limit,process_access_denied`.
- CPU / memory / volumes / network / power: `available` with real values (CPU 5042 bp, physical total 68 143 853 568, not a synthesized zero).
- Processes: `state=partial`, enumerated 676, returned 15, `truncated_by_limit=true`, `budget_exhausted=false`, reason codes **`process_limit` and `process_access_denied`**.
- Six capabilities remain `unsupported` / `not_supported_v1`, never numeric zeros.

`process_access_denied` is set only when `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` (or a follow-up query) fails with `ERROR_ACCESS_DENIED` at Medium integrity. Truncation beyond the requested 15 is a separate truthful `process_limit`. Empty rows are **not** published as `available` with zeros (`finalize_processes` with `access_denied` is Partial). Fixtures cover missing battery (`charge_basis_points: null`), zero elapsed, counter reset, and sleep-sized gaps as unavailable/partial rather than zero.

### 3. Process DTO privacy + locale-invariant JSON — **PASS**

Native process item keys only: `pid`, `name`, `cpu_basis_points_of_one_logical_core`, `private_bytes`, `read_bytes_per_second`, `write_bytes_per_second`. Independent scan: no `"path"`, `"cmdline"`, `"command_line"`, `"user"`, `"environment"`, `"handles"`, `"history"`, `CleanupPlan`. Envelope keys are ASCII snake_case (`schema_version`, `command`, `outcome`, `data`, `warnings`, `error`). CLI `--language` is rejected for JSON (exit 2). OS-provided adapter aliases may contain host-locale text; that is Windows Alias data, not catalogue localization.

### 4. No WMI / PowerShell / vendor; no persistence / service / tray; `cli.rs` untouched — **PASS**

Status collectors call documented Win32 APIs only (`GetSystemTimes`, `GlobalMemoryStatusEx`, volume APIs, `GetIfTable2`/`FreeMibTable`, `GetSystemPowerStatus`, ToolHelp, `OpenProcess` limited, `GetProcessTimes` / `GetProcessMemoryInfo` / `GetProcessIoCounters`). No Status persistence. `git diff -- crates/devsweep-cli/src/application/cli.rs` is empty. Dispatch wiring in `commands/mod.rs` (`status::run`) and `pub(super) mod status` in `presentation/mod.rs` is handler registration, not parser edits.

### 5. Broken pipe — **PASS** (independent recapture)

- Unit: `broken_pipe_cancels_and_joins_without_another_write` (writer.writes stays 1; no terminal write after close).
- CLI contract: live stdout drop → exit 0.
- Native harness: close after `status_started` → exit 0.
- Extra recapture (`independent-snapshot-partial-privacy.log`): first event `status_started`, **exit 0**, join **550.8 ms** after close (in-flight 500 ms window), leftover 0, stderr length 0. CLI path on `broken_pipe` cancels, joins, returns `Ok(())` and does not call `write_producer_error_terminal`.

### 6. `LIVE_PERMIT` is in-process only — **PASS** (no OS-wide single-flight claim)

`LIVE_PERMIT` is a process-local `AtomicBool`. `StatusError::CoordinatorRefused` display: “status live is already running **in this process**”. Unit test `coordinator_refuses_a_second_live_session` covers in-process refusal → CLI `status_busy` / exit 4. Independent native: two overlapping OS `status live` processes **both stayed alive**. This is **not** OS-wide single-flight. Implement.md coordinator refusal is the in-process permit + unit test, not a machine-wide lock.

---

## PRD R / AC

| Clause | Result | Evidence |
| --- | --- | --- |
| **R1** Win32 collectors | **PASS** | Adapters in `system.rs` / `network.rs` / `process.rs`; native CPU/memory/volumes/network/power available |
| **R2** availability; never missing→zero; static unsupported | **PASS** | Five-state union + fixtures; native partial processes; six unsupported capabilities |
| **R3** 500 ms snapshot; 2 s / 1–60 s live; 4 s process; 30 s volume/power; one in flight; skip; cancel/join | **PASS** | Constants + fake-time sampler tests; native 500 ms window (~586–611 ms wall); skipped_total 0 on this host |
| **R4** process privacy + truncation metadata | **PASS** | DTO + native keys; enumerated/returned/requested/ceiling/budget/`truncated_by_limit` |
| **R5** JSON/NDJSON; broken pipe; no service/WMI/vendor | **PASS** | Wire fixtures; native NDJSON; broken-pipe recapture; `cli.rs` untouched |
| **AC1** adapter fixtures | **PASS** | wrap/reset, zero elapsed, churn, access denial, missing battery, non-fixed volumes, sleep gap, 4096/150 ms, unsupported |
| **AC2** scheduler fake-time | **PASS** | clamps, cadence, one in flight, skip/no backfill, cancel, in-process coordinator, channel close |
| **AC3** JSON/NDJSON locale-invariant, privacy, terminals | **PASS** | core round-trip, CLI EN/ZH JSON, wire contract, broken-pipe join |
| **AC4** minimal documented windows-sys + `just ci` | **PASS** | design table now matches 0.59 cfg; `rtk just ci` EXIT 0 |

---

## Design

| Design clause | Result |
| --- | --- |
| One sampler; 500 ms then compose | **PASS** |
| Live one-in-flight; missed ticks skip; cached slow groups | **PASS** |
| Process ceiling 4096, budget 150 ms, limited query rights, handle/table Drop | **PASS** |
| Counter reset / churn → partial/unavailable, not negative | **PASS** |
| Shared V1 DTO / events; integer units; metadata before sort | **PASS** |
| windows-sys map (IpHelper + Ndis cfg + ToolHelp + Power; keep Optimize flags) | **PASS** after documenting Ndis |
| Broken pipe: internal reason, no second write, cancel/join, exit 0 | **PASS** |
| No Status persistence | **PASS** |
| Do not edit `cli.rs` parser | **PASS** |

---

## Backend Quality Check

- Non-mutating collectors: **PASS** (no delete/trash/cleanup dispatch).
- `just ci`: **PASS**.
- Forbidden WMI/PowerShell/vendor Status path: **PASS**.
- Resource numbers vs parent Status limits: **PASS** (ledger above).

---

## Remaining UNVERIFIED (not completion-required)

| Item | Completion-required? | Why not blocking |
| --- | --- | --- |
| Native sleep/resume gap | no | `is_sleep_sized_gap` unit tests (actual > max(4×expected, 10 s)) |
| Native missing battery (`battery_present: false`) | no | `power_from_status` fixture; this host reports battery present (9800 bp) |
| Forced interface/process topology change | no | network/process pair-delta fixtures |
| OS-wide live single-flight | no | permit is in-process; two OS processes both run (recorded) |
| Console Ctrl+C stop of 60 s live | no | pipe-close exit 0 + unit cancel/join; resource run used `Stop-Process` after 300 samples |

---

## Self-fixes this check applied

1. Documented `Win32_NetworkManagement_Ndis` in `design.md`, `implement.md`, and a `Cargo.toml` comment as the windows-sys 0.59 compile gate for `GetIfTable2` (crate source cited). No unused umbrella; Optimize features left intact.

No collector/CLI contract was weakened. No further product defect owned by this child required a code change (max 3-round loop unused).
