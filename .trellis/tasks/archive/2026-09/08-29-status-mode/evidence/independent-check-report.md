# Independent trellis-check — status-mode (coordination parent)

Task: `.trellis/tasks/08-29-status-mode`
Checker: independent trellis-check (coordination parent; no `task.py start`, no archive, no product edits, no commits)
Date: 2026-08-31
Review basis: `HEAD = 25b83d839d3e926aae64ef074d8ad556d393c91f`

Parent `acceptance.md` was treated as a claim set, not as proof. Product state was grepped at HEAD. Archived child independent reports, native summaries, and gate logs were re-read. `just ci` was not re-run (umbrella owns coordination, not a product gate recapture).

## Overall: **PASS**

Parent `acceptance.md` overall: **PASS**.
Completion-required UNVERIFIED: **no**.

Independent check **PASS**. Parent stays active. Not archived.

Self-fixes: none. Parent acceptance is factually consistent with HEAD product and archived child evidence.

---

## Commit and archive audit

| Child | Product | Archive | Archived path | Independent check |
| --- | --- | --- | --- | --- |
| `08-29-status-collector-cli` | `70e43ce13984d9ef5b8123ed81873d09460897fa` (`feat(core)` Status collectors, 16:02:23) | `edb42f4e02512fda5dbdccdb52b5fccdac85c673` | `.trellis/tasks/archive/2026-08/08-29-status-collector-cli/` | Overall **PASS**; completion-required UNVERIFIED **no** |
| `08-29-status-tui-desktop-native` | `fc3a69c830c4168ec9b29ba274db6227ae4004a3` (`feat(desktop)` Status TUI/desktop, 17:27:59) | `25b83d839d3e926aae64ef074d8ad556d393c91f` | `.trellis/tasks/archive/2026-08/08-29-status-tui-desktop-native/` | Overall **PASS**; completion-required UNVERIFIED **no** |

`git log` ancestry is linear and matches R3 order:

`70e43ce` (collector product) → `74f9538` (planning evidence) → `edb42f4` (archive collector) → `fc3a69c` (presentation product) → `4946d6b` (planning evidence) → `25b83d8` (archive presentation = HEAD).

Both archived `task.json` files are `status: completed` with `parent: 08-29-status-mode`. Product commits carry `Agent-Task:` trailers. No umbrella-authored product commit exists.

Dirt (not a defect): an unarchived leftover copy remains at `.trellis/tasks/08-29-status-tui-desktop-native/`. Canonical evidence is the archive path above. Parent `task.json` stays `planning` because this umbrella was never `task.py start`ed, matching `implement.md`.

---

## HEAD product greps (independent)

### No WMI / PowerShell / vendor collectors

Zero matches for `WMI` / `wmi` / `PowerShell` / `powershell` / `nvidia-smi` / `typeperf` / `wmic` / `Get-CimInstance` in:

- `crates/devsweep-core/src/status/`
- `crates/devsweep-cli` status handlers
- `desktop/src-tauri/src/status.rs`

Collectors call documented Win32 APIs: `GetSystemTimes`, `GlobalMemoryStatusEx`, volume APIs, `GetIfTable2`/`FreeMibTable`, `GetSystemPowerStatus`, ToolHelp, `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`, `GetProcessTimes` / `GetProcessMemoryInfo` / `GetProcessIoCounters`. `Cargo.toml` includes `Win32_NetworkManagement_IpHelper` plus documented `Win32_NetworkManagement_Ndis` cfg gate.

### No persistence / service / tray

- Core module docs: “never persists samples”.
- `LIVE_PERMIT` is an in-process `AtomicBool` in `sampler.rs`.
- `desktop/src/modes/status` has zero `localStorage` / `indexedDB` / persist hits.
- Isolated native `presentation-v1.json` is `{"schema_version":1,"language":"zh-CN"}` only.
- Zero `CleanupPlan` construction in the status module (CLI tests only assert the string is absent).

### Missing → never synthesized zero

- `AvailabilityV1` is the five-state union `available` / `partial` / `unavailable` / `permission_denied` / `unsupported`.
- `PowerV1` comment + `decodePowerValue`: missing battery cannot carry charge (`battery_present: false` ⇒ `charge_basis_points`/`remaining_seconds` must be null).
- Native `snapshot-1.json`: CPU `available` 5042 bp; physical total `68143853568`; processes `partial` with `process_limit` + `process_access_denied`; six capabilities `unsupported`/`not_supported_v1`.
- Desktop charts insert `null` gap points (`gapPoint` / `chartSegments`). Native `gpuZero=false`. Network `0 B/s` appears on `state=available` idle interfaces, not as missing hardware.

### Process DTO has no cmdline / path

`ProcessV1` fields at HEAD: `pid`, `name`, `cpu_basis_points_of_one_logical_core`, `private_bytes`, `read_bytes_per_second`, `write_bytes_per_second`. Desktop `PROCESS_ROW_KEYS` is that exact closed set. Native snapshot items use only those keys. `cmdline` / `path` / `user` appear in collector tests as **forbidden** strings, and TUI tests inject leaked `cmdline` to prove it does not render.

### Frozen DTOs across children

- `git log 70e43ce^..HEAD -- crates/devsweep-cli/src/application/cli.rs` is empty.
- `git diff --name-only 70e43ce fc3a69c -- crates/devsweep-core` is only `crates/devsweep-core/src/status/mod.rs`.
- The sole core change is `pub use sampler::{LiveControl, ...}`. `StatusSnapshotV1`, `AvailabilityV1`, and `StatusEventV1` wire fields are unchanged. `StatusEventV1::Snapshot.data` was already `Box<StatusSnapshotV1>` in the collector product.
- Four live events: `status_started`, `status_snapshot`, `tick_skipped`, `status_terminal`. Desktop `decodeStatusEvent` `oneOf` is that closed set.
- Unsupported codes: exact ordered set `gpu_utilization`, `vram`, `thermal`, `fan`, `smart`, `physical_disk_activity`.
- Presentation clippy self-fix boxed presentation-owned enums only (`StatusAction::SnapshotFinished`, `DesktopStatusSnapshotResult::Completed`); Serde JSON unchanged.

### R4 numbers in independent resource summaries

Child-1 CLI `evidence/independent-native/summary.json` (sha256 `3E6CD1BF89AC27850F596AEE331B178AFDE5D2106097E71B2EAE2BF6C5DC112F`, build 26200, Medium `S-1-16-8192`, consent 0):

| Gate | Limit | Measured | Result |
| --- | --- | --- | --- |
| Snapshot p95 | ≤ 2000 ms | 611 ms | pass (`snapshot_p95_le_2000ms`) |
| Live private | ≤ idle + 64 MiB | 4 362 240 ≤ 2 150 400 + 67 108 864 | pass |
| Live threads | ≤ idle + 4 | 5 ≤ 5 + 4 | pass |
| Live CPU median / p95 | ≤ 5% / 15% | 0% / 7.802% | pass |
| Post-exit | exactly 25 / 5 s | 25 samples, leftover 0, all 25 `present=false` | pass (`post_exit_25_samples`, `post_exit_final_five_hold`) |
| Live 60 s | 300 present | 300/300, `skipped_total` 0, 31 events = `status_started` + 30 `status_snapshot` | pass |

Child-2 desktop `evidence/independent-native/resource/summary.json` (sha256 `4d1d764ed3ffd3783b732aefadd3c109f6add6fc374a9efbc82b7c5eaf8abc1c`, PID 1248, `"pass": true`):

| Gate | Limit | Measured | Result |
| --- | --- | --- | --- |
| Snapshot p95 | ≤ 2000 ms | 820 ms | `snapshot_p95_le_2000ms` |
| Live private | ≤ idle max + 64 MiB | 6 844 416 ≤ 5 926 912 + 67 108 864 | true |
| Live threads | ≤ idle max + 4 | 37 ≤ 36 + 4 | true |
| Live CPU median / p95 | ≤ 5% / 15% | 0% / 7.781% | true |
| Post-exit samples | exactly 25, none missing | 25 | `post_exit_25_samples`, `post_exit_no_missing_sample` |
| Final five hold | CPU ≤ idle median + 0.5 pp **and** threads ≤ idle max + 1 | all five `present=true`, CPU 0 ≤ 0.5, threads 33 ≤ 37 | `post_exit_final_five_hold` |

These match parent `acceptance.md` R4. Desktop post-exit is the five-mode continuous-hold protocol (process still present, both bounds on every final-five sample), not a padded absent-after-kill trace. CLI post-exit is process-absent for all 25 (stronger hold).

---

## Parent R / AC vs acceptance and children

| Clause | Parent acceptance | Independent verdict | Evidence |
| --- | --- | --- | --- |
| R1 | PASS | **PASS** | Win32 collectors in `70e43ce`; CLI/TUI/Desktop + native hashes above; `App.tsx` registers `status`; TUI `modes/mod.rs` + boxed snapshot; Tauri `status.rs` |
| R2 | PASS | **PASS** | Supported groups + static six unsupported; native snapshot-1; `gpuZero=false`; no missing→zero |
| R3 | PASS | **PASS** | Collector archived before presentation; umbrella review-only; core diff is `LiveControl` re-export; `cli.rs` frozen |
| R4 | PASS | **PASS** | Independent summaries above; both children `just ci` EXIT 0 on the gate used (child-2 round 2 after clippy box) |
| R5 | PASS | **PASS** | Frozen V1 snapshot/events/units/truncation metadata; four events; broken-pipe join evidence in child-1 |
| AC1 | PASS | **PASS** | Child-1: 500 ms window, 1–60 s clamp, 15/100/4096/150 ms, fake-time sampler, 28 core status tests, 6 CLI status tests, native 300/300. Child-2: TUI 10 tests, desktop 60-cap + `released`, coordinator `kind: "status"`, AppShell `cancelAndJoin`, `StatusAlreadyRunning`, AX 792 / `privacyHit=false` |
| AC2 | PASS | **PASS** | Shared fixtures + `exact()`; `types:generate` in desktop-web-check (157 tests); leaked-`cmdline` TUI fixture; EN/ZH capability notes |
| AC3 | PASS | **PASS** | Idle + five snapshots + 60 s live + desktop leave/restart/close + post-exit 25×200 ms on both release binaries; process-table churn (first process changed); sleep/resume sanctioned UNVERIFIED |

Parent `prd.md` / `design.md` / `implement.md` require schema parity, exact R4, recursive archived evidence, and overall PASS only with no completion-required UNVERIFIED. Acceptance records those clauses. This check confirms the records, not archived status alone.

---

## Cross-child integration

- Frozen CLI grammar: `cli.rs` untouched after `70e43ce^`. Child-1 wiring of `commands/mod.rs` (`status::run`) and `presentation/mod.rs` is handler registration.
- Schema stability: snapshot/event/process/unsupported DTOs unchanged across children.
- Shell exclusivity is the presentation coordinator (`kind: "status"`), not OS-wide. Child-1 native two-OS overlap (`a_alive`/`b_alive` both true) matches in-process `LIVE_PERMIT`. Not a parent defect.
- Registration after collector freeze; rollback remains unregister presentation and discard ephemeral samples.
- Child-1 snapshot `outcome=partial` / CLI exit 5 is truthful process truncation + access denied at Medium.

---

## Archived independent gates (not re-run)

Child 1 — all EXIT **0** except as noted in the child report:

| Log | Exit |
| --- | --- |
| `independent-core-status-all.log` | 0 (28 passed) |
| `independent-cli-status.log` | 0 (6 passed) |
| `independent-just-ci.log` | 0 |
| `independent-native/summary.json` gates | all true |

Child 2:

| Log | Exit |
| --- | --- |
| `independent-just-ci.log` | **1** (`clippy::large_enum_variant`; in-scope, repaired) |
| `independent-just-ci-round2.log` | **0** |
| `independent-cli-status-after-clippy-fix.log` | 0 (16 passed) |
| `independent-tui-status-after-clippy-fix.log` | 0 (10 passed) |
| `independent-desktop-web-check.log` | 0 (157 tests) |
| `independent-desktop-test-after-clippy-fix.log` | 0 (34 passed, 2 ignored) |
| `independent-native/resource/summary.json` | `"pass": true` |

---

## Manual/native UNVERIFIED (all sanctioned, none completion-required)

Matches parent acceptance ledger and both child independent reports:

1. Native sleep/resume host gap — covered by `is_sleep_sized_gap` fixtures + observed process-table churn.
2. Native missing battery — host reports battery present (9800 bp); fixtures cover `charge_basis_points: null`.
3. Forced interface/process topology change — pair-delta fixtures; desktop first-process churn recorded.
4. OS-wide live single-flight — in-process permit only; two OS CLI processes both ran.
5. Console Ctrl+C of 60 s live — pipe-close exit 0 + unit cancel/join.
6. Human Narrator listen-through — AX 792 + labelled controls.
7. OS-global display scale 125/150/200 — device-scale WebView2 emulation only.

Nothing unsanctioned was dropped. Per parent `implement.md`, overall PASS requires no completion-required UNVERIFIED: none of the above is completion-required.

---

## Parent acceptance review quality

`acceptance.md` traces every parent R/AC clause, lists both leaf product and archive SHAs, cites independent (not implementer-only) logs, records exact R4 numbers from the independent summaries, and states overall **PASS** with the UNVERIFIED boundary. No factual correction was required.

---

## Constraints honored

- Did not call `task.py start` or archive.
- Did not spawn trellis-implement or trellis-check.
- Did not git commit / push / amend.
- Did not edit product files or Optimize parent `acceptance.md`.
- Parent remains active.
