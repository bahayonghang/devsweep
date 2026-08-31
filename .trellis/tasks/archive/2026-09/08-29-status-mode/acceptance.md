# Status Mode Parent Acceptance Review

Date: 2026-08-31
Reviewer role: coordination-parent acceptance reviewer (umbrella `08-29-status-mode`, review-only; no sub-agents spawned, no product files touched, no commits, parent not `task.py start`ed).
Review basis: `HEAD = 25b83d839d3e926aae64ef074d8ad556d393c91f` (archive commit of the last leaf). Archived child evidence under `.trellis/tasks/archive/2026-08/` was traced clause by clause; product state was independently grepped at HEAD.

## Reviewed leaf product commits and archived task paths

| Child | Product commit | Archive commit | Archived path |
| --- | --- | --- | --- |
| `08-29-status-collector-cli` | `70e43ce` (`70e43ce13984d9ef5b8123ed81873d09460897fa`, feat(core) Status collectors) | `edb42f4` (`edb42f4e02512fda5dbdccdb52b5fccdac85c673`) | `.trellis/tasks/archive/2026-08/08-29-status-collector-cli/` |
| `08-29-status-tui-desktop-native` | `fc3a69c` (`fc3a69c830c4168ec9b29ba274db6227ae4004a3`, feat(desktop) Status TUI/desktop) | `25b83d8` (`25b83d839d3e926aae64ef074d8ad556d393c91f`) | `.trellis/tasks/archive/2026-08/08-29-status-tui-desktop-native/` |

Ordering verified from `git log -8 --oneline`: HEAD is `25b83d8`; each product commit is immediately followed by its planning evidence commit then its independent `chore(task): archive` commit, in the mandated child order collector/CLI -> presentation/native (R3 ordering). Product times: `70e43ce` 16:02, `fc3a69c` 17:27. Both child `task.json` files carry `status: completed` and `parent: 08-29-status-mode`. No umbrella product-code commit exists; every product commit is child-tagged (`Agent-Task:` trailer). Archived status alone was not used — product state was verified at HEAD.

Independent HEAD greps and diffs (product, not archive markdown):

- Frozen CLI grammar: `git log 70e43ce^..HEAD -- crates/devsweep-cli/src/application/cli.rs` is empty. Child-1 fills `application/commands/status.rs` and `presentation/status.rs`; child-2 fills TUI/desktop. Dispatch wiring in `commands/mod.rs` / `presentation/mod.rs` is handler registration, not parser expansion.
- Core schema freeze: `git diff --name-only 70e43ce fc3a69c -- crates/devsweep-core` is only `crates/devsweep-core/src/status/mod.rs`. The sole change is `pub use sampler::{LiveControl, ...}` (presentation cancel/join handle). `StatusSnapshotV1`, `AvailabilityV1`, and `StatusEventV1` wire fields are unchanged. `StatusEventV1::Snapshot.data` was already `Box<StatusSnapshotV1>` in the collector product.
- Four live events at HEAD: `status_started`, `status_snapshot`, `tick_skipped`, `status_terminal`. Desktop `decodeStatusEvent` `oneOf` is that exact closed set; extra/renamed/omitted variants fail `exact()`.
- `AvailabilityV1` five states: `available` / `partial` / `unavailable` / `permission_denied` / `unsupported`. Missing hardware uses `battery_present: false` and `charge_basis_points: null`, never a synthesized zero charge (`PowerV1` comment + `decodePowerValue` reject).
- Process DTO: `ProcessV1` fields are only `pid`, `name`, `cpu_basis_points_of_one_logical_core`, `private_bytes`, `read_bytes_per_second`, `write_bytes_per_second`. Desktop `PROCESS_ROW_KEYS` is the same closed set. Status collectors contain no WMI / PowerShell / `nvidia-smi` / `typeperf` / `wmic` hits. Status module never constructs `CleanupPlan`.
- No Status persistence/service/tray: core module documents “never persists samples”; desktop `modes/status` has zero `localStorage` / `indexedDB` / persist hits. Isolated native `presentation-v1.json` is language only (`{"schema_version":1,"language":"zh-CN"}`), not sample history.
- Unsupported capabilities: closed six-code set `gpu_utilization`, `vram`, `thermal`, `fan`, `smart`, `physical_disk_activity`, each `state=unsupported` / `not_supported_v1`. Desktop decoder requires that exact ordered set.

## Per-requirement verdicts

| Clause | Verdict | Evidence |
| --- | --- | --- |
| R1 (bounded read-only snapshot and explicit live mode using documented Win32 APIs through the existing `windows-sys` baseline, plus CLI/TUI/Desktop and native resource evidence) | PASS | Collectors in `crates/devsweep-core/src/status/` (`system.rs` / `network.rs` / `process.rs` / `sampler.rs`) in `70e43ce`: `GetSystemTimes`, `GlobalMemoryStatusEx`, volume APIs, `GetIfTable2`/`FreeMibTable`, `GetSystemPowerStatus`, ToolHelp, `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`, `GetProcessTimes` / `GetProcessMemoryInfo` / `GetProcessIoCounters`. `Cargo.toml` adds `Win32_NetworkManagement_IpHelper` + documented `Win32_NetworkManagement_Ndis` cfg gate for `GetIfTable2`; Optimize flags kept. Presentation: `fc3a69c` adds `tui/modes/status/`, `desktop/src/modes/status/`, `desktop/src-tauri/src/status.rs`. Native CLI sha256 `3E6CD1BF…` and desktop sha256 `4D1D764E…` on host build 26200, Medium `S-1-16-8192`, `consent.exe` 0. |
| R2 (supported metrics CPU / physical memory / fixed-volume capacity / network interfaces / battery-power / bounded process CPU-memory-I/O; GPU/VRAM/thermal/fan/SMART/physical-disk are explicit unsupported/unavailable, never zero or synthesized health) | PASS | Native `snapshot-1.json`: CPU 5042 bp available, physical total 68 143 853 568 available, volumes/network/power available; processes `partial` with `process_limit` + `process_access_denied`; six unsupported capabilities never numeric zeros. Desktop `gpuZero=false`; charts use `null` gap points (`gapPoint` / `chartSegments`). Network `0 B/s` is an available idle rate, not missing→zero. |
| R3 (collector/CLI before presentation; umbrella owns schema parity, live lifecycle, resource gates, recursive evidence, and rollback only) | PASS | Commit/archive ordering above. Presentation did not add collectors, change Win32 mapping, or edit `cli.rs`. Core diff is the `LiveControl` re-export only. Umbrella owns this acceptance review; no umbrella-authored product change. Rollback remains “unregister presentation (`App.tsx` `status` route / TUI mode), discard ephemeral samples.” |
| R4 (numeric native resource gates on the recorded release-build host, including exact 25-sample post-exit with both bounds in every final-five sample) | PASS | Same host (Windows 11 build 26200, x64, 24 logical processors, Medium). CLI release `devsweep.exe` sha256 `3E6CD1BF89AC27850F596AEE331B178AFDE5D2106097E71B2EAE2BF6C5DC112F`: snapshot p95 611 ms; live private 4 362 240 ≤ 2 150 400 + 67 108 864; live threads 5 ≤ 5 + 4; live CPU median 0% / p95 7.802%; post-exit exactly 25 samples / 5 s, leftover 0, all 25 `present=false` (process absent; CPU 0 and threads 0 satisfy idle+0.5 pp and idle+1). Desktop release `devsweep-desktop.exe` sha256 `4d1d764ed3ffd3783b732aefadd3c109f6add6fc374a9efbc82b7c5eaf8abc1c` PID 1248: snapshot p95 820 ms; live private 6 844 416 ≤ 5 926 912 + 67 108 864; live threads 37 ≤ 36 + 4; live CPU median 0% / p95 7.781%; post-exit exactly 25 scheduled samples with **no padding**, none missing; final five all `present=true`, CPU 0 ≤ idle median 0 + 0.5 pp **and** threads 33 ≤ idle max 36 + 1. Child-2 `gates.post_exit_final_five_hold` and `post_exit_no_missing_sample` are true. |
| R5 (CLI task `StatusSnapshotV1` / `AvailabilityV1` / four live event shapes are the sole JSON/NDJSON/Tauri contract; integer units/time bases, truncation metadata, sequence, and terminal/broken-pipe lifecycle unchanged across children) | PASS | Shared types in `crates/devsweep-core/src/status/mod.rs`. Snapshot keys, process truncation metadata (`enumerated_count` / `returned_count` / `requested_limit` / `enumeration_ceiling` / `detail_budget_ms` / `truncated_by_limit` / `budget_exhausted`), integer bytes/basis-points, UTC `sampled_at_unix_ms` / `emitted_at_unix_ms`, and sequence are consumed by CLI JSON, TUI, and desktop `exact()` decoders. Four events frozen as above. Broken-pipe: internal terminal, no second write, cancel/join, exit 0 (unit + CLI contract + native recapture). Presentation clippy self-fix boxed presentation-owned enums only; Serde JSON unchanged. |

## Per-AC verdicts

| Clause | Verdict | Evidence |
| --- | --- | --- |
| AC1 (R1, R2, R3): recursive children prove interval/process bounds, one sample, skipped ticks, cancel/join/broken-pipe, availability semantics, locale-invariant snapshots, exact wire fields/types/units/time bases and truncation metadata, no sensitive process fields, and no background work/history | PASS | Child-1: snapshot window 500 ms; live clamp 1–60 s (native interval 1000/60000 exit 0, 0/61 clap exit 2); process 15/100/4096/150 ms; one in flight; skip without backfill; in-process `LIVE_PERMIT`; fake-time sampler tests (6) + status crate 28. Native 60 s live: 300/300 samples, `skipped_total` 0, `status_started` + 30 `status_snapshot`. Broken-pipe join 550.8 ms, leftover 0. Child-2: TUI 10 tests (opt-in live, 60-cap, `Released` drop); desktop reducer 60-cap + leave; `OperationCoordinator` `kind: "status"`; AppShell `cancelAndJoin` on route change; Tauri `StatusAlreadyRunning`. Native process keys name/pid/cpu/private/read/write only; `privacyHit=false`; AX 792 nodes, no cmdline. Isolated `LOCALAPPDATA` holds language, not charts. |
| AC2 (R1, R2, R5): CLI/TUI/Desktop match the same frozen snapshot/event bytes and clearly distinguish available/partial/unavailable/permission/unsupported in both languages; runtime decoders reject renamed, omitted, localized, or extra variants | PASS | Child-2 independent check: shared fixtures + `exact()`; `types:generate` 32 named fixtures; `contract.test.ts` rejects unknown fields/`gpu`; TUI leaked-`cmdline` fixture does not render; CLI `unknown-event.json` + `broken-pipe-terminal.json`. Desktop `decodeAvailability` requires the five-state union; `decodeStatusEvent` requires the four event names; missing battery cannot carry charge. Native EN/ZH widths 390–1440; capability note states unsupported are not shown as zero. Charts never interpolate missing as zeros (`null` gaps). |
| AC3 (R1, R3, R4): native resource evidence covers idle, snapshot, live, navigation exit, sleep/resume or interface/process churn where reproducible, and no overlap with heavy modes; exit evidence contains all 25 scheduled samples and the final-five continuous-hold evaluation from the integration protocol | PASS | Idle + five snapshots + 60 s live + navigation leave/restart/close (desktop capture-log) + post-exit 25×200 ms on both release binaries. Desktop final-five continuous-hold uses **both** CPU and thread bounds on every sample (five-mode protocol), not a padded absent-after-kill trace. CLI post-exit is process-absent for all 25 (stronger hold). Process-table churn observed on the desktop host (15 rows, changing first process). Sleep/resume host transition was not forced (sanctioned UNVERIFIED; covered by `is_sleep_sized_gap` fixtures). Live Status is mutually exclusive with Scan/Analyze/Software/Optimize via the shell coordinator; native resource runs did not overlap those modes. |

## Cross-child integration checks

- Frozen CLI grammar untouched: `git log 70e43ce^..HEAD -- crates/devsweep-cli/src/application/cli.rs` is empty. Child-1 wiring of `commands/mod.rs` / `presentation/mod.rs` is required reachability, not grammar expansion.
- Schema DTO stability: `StatusSnapshotV1`, `AvailabilityV1`, `ProcessV1`, truncation metadata, six unsupported codes, and the four live events are unchanged across children. Presentation added no collector files and did not query OS APIs for metrics. Desktop integer units (basis points, bytes, unix ms) match core.
- No missing→zero: native CPU/memory are non-zero available values; processes are `partial` with reason codes rather than empty-available zeros; unsupported capabilities stay `unsupported`; power missing-battery fixture uses `null` charge; live charts insert `null` gap points on `tick_skipped`.
- No WMI/PowerShell/vendor collectors: zero hits in `crates/devsweep-core/src/status`, `crates/devsweep-cli` status handlers, and `desktop/src-tauri/src/status.rs`.
- No persistence/service/tray: no Status sample store; live is explicit user start; leave/close cancel+join and drop the 60-point buffer (`StatusAction::Released` / desktop `released`).
- No sensitive process fields: native snapshot items and desktop decoder `exact(PROCESS_ROW_KEYS)`; TUI/desktop tests reject leaked `cmdline` / `user` / `path`.
- Collector before presentation: product times 16:02 then 17:27; presentation resource R4 was re-measured on the post-clippy boxed binary (`4D1D764E…`), not the pre-fix GUI hash.
- `LIVE_PERMIT` is in-process only. Two overlapping OS `status live` processes both stayed alive (child-1 native). Shell exclusivity is the presentation coordinator, not an OS-wide lock. This matches child-1 design and is not a parent defect.
- Registration: Status is registered in the desktop shell (`App.tsx` adds the `status` route) and TUI `modes/mod.rs` only after collector freeze. Rollback remains unregister presentation and discard ephemeral samples.
- Child-1 snapshot `outcome=partial` / CLI exit 5 is truthful availability (process truncation + access denied at Medium), not a resource or schema failure.

## Commands, exit codes, and log paths (independent, per archived evidence)

Logs are under the archived child paths above. `just ci` was not re-run; every clause traces to an archived independent log.

Child 1 (`08-29-status-collector-cli/evidence/`):

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test -p devsweep-core status::system --offline` | 0 (6 passed) | `independent-core-status-system.log` |
| `rtk cargo test -p devsweep-core status::process --offline` | 0 (6 passed) | `independent-core-status-process.log` |
| `rtk cargo test -p devsweep-core status::sampler --offline` | 0 (6 passed) | `independent-core-status-sampler.log` |
| `rtk cargo test -p devsweep-core status --offline` | 0 (28 passed) | `independent-core-status-all.log` |
| `rtk cargo test -p devsweep-cli status --offline` | 0 (6 passed) | `independent-cli-status.log` |
| `rtk cargo test -p devsweep-cli --test cli_contract json_and_ndjson --offline` | 0 (1 passed; live stdout close → exit 0) | `independent-cli-contract-json-ndjson.log` |
| `rtk cargo test -p devsweep-cli --test cli_contract status_wire --offline` | 0 (1 passed) | `independent-cli-contract-status-wire.log` |
| `rtk git diff --check` | 0 | `independent-git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 | `independent-fmt-check.log` |
| `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` | 0 | `independent-release-build.log` |
| `rtk just ci` | 0 (`ci complete`) | `independent-just-ci.log` / `independent-just-ci-full.log` |
| Native harness (warmup, five snapshots, idle, 60 s live, post-exit 25, intervals, broken pipe, two-OS overlap) | 0 | `independent-native-harness.log`; summary `independent-native/summary.json` |
| Snapshot JSON inspect + extra broken-pipe recapture | 0 | `independent-snapshot-partial-privacy.log` |

Child 2 (`08-29-status-tui-desktop-native/evidence/`):

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test --locked -p devsweep-cli -- status` | 0 (16 passed) | `independent-cli-status.log` then `independent-cli-status-after-clippy-fix.log` |
| `rtk cargo test --locked -p devsweep-cli -- tui::modes::status` | 0 (10 passed) | `independent-tui-status.log` then `independent-tui-status-after-clippy-fix.log` |
| `rtk just desktop-web-check` | 0 (types:generate, lint, typecheck, 157 tests, vite build) | `independent-desktop-web-check.log` |
| `rtk just desktop-test` | 0 (34 passed, 2 ignored) | `independent-desktop-test.log` then `independent-desktop-test-after-clippy-fix.log` |
| `rtk just desktop-build` | 0 | `independent-desktop-build.log` then `independent-desktop-build-after-fix.log` |
| `rtk git diff --check` | 0 | `independent-git-diff-check.log` then `independent-git-diff-check-after-fix.log` |
| `rtk cargo fmt --all -- --check` | 0 | `independent-fmt-check.log` then `independent-fmt-after-clippy-fix.log` |
| `rtk node evidence/independent-native/capture-native-status.mjs` | 0 | `independent-native-capture.log` |
| `rtk node evidence/independent-native/resource-gate.mjs` | 0 (pre-fix binary) | `independent-resource-gate.log` |
| `rtk just ci` round 1 | 1 (`clippy::large_enum_variant` on presentation-owned snapshot variants) | `independent-just-ci.log` |
| `rtk just ci` round 2 | 0 | `independent-just-ci-round2.log` |
| Resource gate round 2 (current release sha256 `4D1D764E…`, PID 1248) | 0; `pass: true` | `independent-resource-gate-round2.log`; `independent-native/resource/summary.json` |

Child-2 round-1 `just ci` failed in-scope (`clippy::large_enum_variant` on TUI `StatusAction::SnapshotFinished` and desktop `DesktopStatusSnapshotResult::Completed`) and was repaired by boxing `StatusSnapshotV1` in those presentation enums before archive. Round 2 is the gate used here. JSON/UI/copy did not change; R4 was re-measured on the current hash.

## Manual/native UNVERIFIED ledger (all sanctioned, none completion-required here)

1. Native sleep/resume host gap — this umbrella forbids faking timers or forcing a machine sleep. Covered by `is_sleep_sized_gap` unit tests (actual > max(4×expected, 10 s)) and process-table churn observed on the desktop host. **Completion-required: no.**
2. Native missing battery (`battery_present: false`) — this host reports battery present (9800 bp). Covered by `power_from_status` fixture and desktop `decodePowerValue`. **Completion-required: no.**
3. Forced interface/process topology change — covered by network/process pair-delta fixtures; desktop native already saw first-process churn. **Completion-required: no.**
4. OS-wide live single-flight — permit is in-process; two OS CLI processes both run (recorded). Presentation exclusivity is the shell coordinator. **Completion-required: no.**
5. Console Ctrl+C stop of 60 s live — pipe-close exit 0 + unit cancel/join; resource run used `Stop-Process` after 300 samples. **Completion-required: no.**
6. Human Narrator listen-through — AX tree (792 nodes) + labelled Refresh/Stop/truncation is the spec-protocol substitute. **Completion-required: no.**
7. OS-global display scale 125/150/200 — forbidden to change user display settings. Device-scale WebView2 emulation (DPR 1 / 1.25 / 1.5 / 2) used instead. **Completion-required: no.**

Nothing unsanctioned was silently dropped. Per `implement.md`, overall PASS requires no completion-required UNVERIFIED: none of the above is completion-required for this umbrella.

## Overall verdict

**PASS**

No defect requires returning work to a child. Schema parity, live lifecycle, exact R4 (including 25-sample post-exit with both CPU and thread bounds on every final-five sample on the recorded desktop release binary), and recursive evidence are satisfied. The sanctioned UNVERIFIED boundary is consistently documented across both children; per the umbrella's own gate this does not block PASS. This umbrella stays active after PASS; archive only after `08-29-five-mode-native-integration` passes.

Reviewer note (context, not a defect): the working tree contains post-archive local dirt from other in-progress work and this still-untracked umbrella directory. None of these were touched by this review.
