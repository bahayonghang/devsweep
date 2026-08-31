# Independent check — 08-29-five-mode-native-integration

Independent `trellis-check` against `prd.md`, `design.md`, and `implement.md`.
Implementer round-6 (`raw-20260831T205206Z` plus a later Chrome commit JSON in
`raw-20260831T212913Z`) was a clue only. This check re-ran the **full**
`tools/measure-resources.ps1 -Protocol five-mode-v1` after the script already
included `tools/run-analyze-react-commit.mjs`. No commit, push, amend, sign, or
publish. Frozen thresholds 200%, 5 s, 1.20, 1 s, 100 ms, and 512 MiB were not
changed. Analyze walker/pool was not patched. Round-4 `p95cpu=0` after a 600 s
idle pad is invalid and is not treated as PASS. Edge `:9223` is not the
react-commit path; sampler glue was not patched.

**Overall: PASS**

**Independent check: PASS**

PASS is this recapture of frozen `five-mode-v1` (protocol exit 0). Analyze
walk-window CPU p95 **190.16%** (≤200 and not 0). React-commit p95 **6 ms** from
the same full run (5 warm-ups + 30 measured, Chrome free port **2052**). Every
frozen numeric gate on this recapture passed. Completion-required UNVERIFIED
rows: none.

## Independent recapture identity

| Item | Value |
| --- | --- |
| Protocol | `tools/measure-resources.ps1 -Protocol five-mode-v1` |
| OutDir / isolated LOCALAPPDATA | `evidence/resources/independent-check/` (`localappdata`, `desktop-idle-localappdata`, `desktop-localappdata`; profiles deleted then recreated before this run) |
| Raw | `evidence/resources/independent-check/raw-20260831T214110Z/` |
| Gates / summary / host | `evidence/resources/independent-check/gates.json`, `summary.json`, `host-manifest.json` |
| Log | `evidence/logs/independent-measure-resources.log` |
| UTC | start `2026-08-31T21:41:09Z`, end `2026-08-31T22:05:52Z` (~1483 s) |
| Exit | **0** (`five-mode-v1 PASS`; `SAMPLER_EXIT=0`) |
| Commit | `1cda3f2ffa4b537cf246fad1a25dde8153efba3a` (working tree also includes uncommitted five-mode glue) |
| CLI SHA-256 | `46387a57944641fe4dde99353a69fe0b3c6d0627777cb044cc19129d92d0d43b` (independent `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` after deleting the exe; hash unchanged vs implementer) |
| Desktop SHA-256 | `dd2f3bcb9de30a4e90aecd39aca8b7342a461d198627b1a0712e1cf7c3517182` (independent `rtk just desktop-build` after deleting the exe; **not** implementer `2bd36354…`) |
| Host | Intel Core Ultra 9 275HX, 24 logical cores, Windows 11 build 26200, Balanced, rustc 1.98.0, Node v26.7.0, locale zh-CN, Medium `S-1-16-8192`, Defender services disabled (environment note, not a correctness pass) |
| New PIDs | idle desktop **34840** (no remote-debug); Status live CLI **61336 / 68272 / 14004 / 57660 / 47068**; Analyze CDP desktop **56736** on port 9588; react-commit HeadlessChrome on port **2052** |

Implementer round-6 artifacts under `evidence/resources/raw-20260831T205206Z/`
and the later commit-only `raw-20260831T212913Z/` were not overwritten. Prior
independent raw `raw-20260831T193506Z/` (AC4 FAIL, walk-window CPU 203.39%)
remains.

## Sampler window confirmation (this recapture)

Inspected `tools/measure-resources.ps1` and `tools/run-analyze-react-commit.mjs`
before the run. CPU% is `delta TotalProcessorTime / Stopwatch wall_ms * 100`.
`[DateTime]::UtcNow` is used only for artifact timestamps, not CPU wall.
`$MaxMs = 600000` is a loop ceiling; the Analyze walker loop **breaks** on
`partial|complete|canceled|error`. No Get-Date inflation and no 600 s
post-terminal pad were found.

React-commit in this script is `node tools/run-analyze-react-commit.mjs` against
product Vite `benchmark:analyze` on `127.0.0.1:4181`, Chrome first, free port,
**refusing 9223**. Chrome exists at
`C:\Program Files\Google\Chrome\Application\chrome.exe`. The script does **not**
call Edge `:9223` as the only path, so glue was **not** patched.

Raw Analyze jsonl (`analyze-250k-desktop-1` … `-5`):

| Rep | samples = walk_samples | first status | last status | last wall_ms | elapsed_ms | cpu p95 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 72 | loading | partial | 14210.377 | 14245.328 | **190.16%** |
| 2 | 71 | loading | partial | 14009.526 | 14043.500 | 185.42% |
| 3 | 70 | loading | partial | 13807.085 | 13841.020 | 188.78% |
| 4 | 69 | loading | partial | 13609.784 | 13640.422 | 187.11% |
| 5 | 71 | loading | partial | 14018.946 | 14052.234 | 186.89% |

Nearest-rank p95 of those five run p95s is **190.165%**. This is a
walk-until-partial window (~13.6–14.2 s, 69–72 samples at 200 ms). It is **not**
a 600 s idle tail and is **not** round-4 `p95cpu=0`.

Status snapshot and live used CLI `status snapshot` / `status live` with
`--process-limit 15` and no WebView2 `--remote-debugging-port`. Idle used a
separate no-debug desktop. Relative Status gates are not CDP-bloated desktop.

## R / AC ledger

| ID | Verdict | Notes |
| --- | --- | --- |
| R1 | PASS | Five shipped modes in `registry.ts` (`clean`, `software`, `optimize`, `analyze`, `status`) plus protection/rules/history. `five_mode_contract.rs` rejects `tui`/`scan`/`inventory`/`protect`/`rules` (`shipped_modes_are_reachable_and_removed_roots_stay_unknown`, 7 tests). Tauri `generate_handler` test asserts no compatibility alias. No feature flag or placeholder adapter. |
| R2 | PASS | `five_mode_contract.rs` covers coordinator, digest mismatch, locale vs machine JSON, cancel/join on live Status, partial/unavailable, mixed audit versions (inside `just ci`). Native capture-log records live→Clean leave, analyze cancel, restart. |
| R3 | PASS | Independent host integrity Medium; `consent` 0 during recapture. Native EN/ZH, keyboard Tab, reduced motion, high contrast, 390/800/1024/1440, device scale 100/125/150/200 via `--force-device-scale-factor` (OS scale not changed). Antivirus is environment-only. |
| R4 | PASS | Independent five-mode-v1 exit 0. Frozen Analyze walk-window CPU p95 190.16% ≤ 200% and not 0. React-commit p95 6 ms ≤ 100 ms from the same full script. Thresholds not weakened. Round-4 padded CPU was not scored. |
| R5 | PASS | README five-mode section is a separate hunk from the pre-existing `just tinstall` dirt. `docs/guide/cli-migration.md`, `docs/safety-capability-matrix.md`, and `docs/validation/five-mode-native.md` describe shipped capabilities. Delivery remains local. |
| AC1 | PASS | Live `task.py validate 08-29-five-mode-native-integration` exit 0 (warnings only). |
| AC2 | PASS | Independently re-ran `rtk just ci`, `rtk just desktop-web-check`, `rtk git diff --check` — all exit 0. `just ci` includes workspace tests (`five_mode_contract` 7 passed; desktop 39 passed, 2 ignored) and clippy `-D warnings`; log ends `ci complete`. Desktop-web-check: 30 files, 172 tests, vite `built in 612ms`. Independent release CLI rebuild exit 0; independent `just desktop-build` exit 0 (unsigned NSIS). |
| AC3 | PASS | Implementer native matrix artifacts are sufficient; spot-checked 35 screenshots, `matrix.json`, `capture-log.jsonl`, `integrity.txt` (Medium; Administrators deny-only). Sanctioned UNVERIFIED only: sleep/resume, Software uninstall. Observation (not an AC3 coverage miss): `en-software-inventory.png` / capture-log still show leaked `${{arpDisplayName}}`; not patched here. |
| AC4 | PASS | Exact protocol recaptured with new PIDs, isolated LOCALAPPDATA, independently rebuilt desktop, and the Chrome commit harness inside the same script. Every frozen numeric gate passed. Walk-window Analyze CPU is 190.16%, not 0. |
| AC5 | PASS | Fingerprinted README tinstall hunk preserved next to `## Five-mode product`. Unrelated `justfile` dirt preserved. No push/sign/publish. |

## Command + exit table (this check)

| Command | Exit | Log |
| --- | --- | --- |
| `python -X utf8 ./.trellis/scripts/task.py validate 08-29-five-mode-native-integration` | 0 | `evidence/logs/independent-task-validate.log` |
| `rtk just ci` | 0 | `evidence/logs/independent-just-ci.log` (rtk tee; `ci complete`) and `independent-just-ci.rtk.log` |
| `rtk just desktop-web-check` | 0 | `evidence/logs/independent-desktop-web-check.log` / `.rtk.log` (172 tests, vite build) |
| `rtk git diff --check` | 0 | `evidence/logs/independent-git-diff-check.log` |
| `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` | 0 | `evidence/logs/independent-release-cli.log` |
| `rtk just desktop-build` | 0 | `evidence/logs/independent-desktop-build.log` |
| `& .\tools\measure-resources.ps1 -Protocol five-mode-v1 -OutDir independent-check` | **0** | `evidence/logs/independent-measure-resources.log` |
| Native spot-check (no recapture; OS display unchanged) | n/a | `evidence/native/matrix.json`, 35 screenshots, `capture-log.jsonl` |

## Resource table (independent five-mode-v1, this recapture)

CPU% = delta TotalProcessorTime / Stopwatch wall × 100. Nearest-rank p95.
Sample 200 ms. One unrecorded warm-up + five measured reps. Analyze CPU uses
**loading-until-partial/complete only**. React-commit is the product Vite
harness driven by headless Chrome on a free port in this same run.

| Gate | Result | Detail |
| --- | --- | --- |
| idle.median_cpu_le_1 | PASS | medianCpu=0; all five idle reps `present=300` (PID 34840 stayed alive) |
| idle.max_private_le_192mib | PASS | 7,102,464 B |
| idle.max_threads_le_40 | PASS | maxThreads=33 |
| status.snapshot.p95_le_2s | PASS | 808.206 ms (CLI; no CDP desktop). Raw: 716.852, 716.226, 808.206, 734.471, 718.217 |
| status.snapshot.private_le_idle_plus_64mib | PASS | peak 1,933,312 |
| status.snapshot.threads_le_idle_plus_4 | PASS | threads=4 idle=33 |
| status.live.median_cpu_le_5 | PASS | 0 (CLI `status live`; no CDP desktop) |
| status.live.p95_cpu_le_15 | PASS | 7.82% |
| status.live.private_le_idle_plus_64mib | PASS | peak 4,485,120 |
| status.live.threads_le_idle_plus_4 | PASS | threads=5 |
| status.live.poststop_25_samples | PASS | exactly 25 each measured live rep |
| status.live.poststop_final_five_hold | PASS | final five consecutive samples meet idle+0.5 pp CPU and idle+1 threads |
| clean.median_ratio_le_1_20 | PASS | ratio=**0.410**; median=16391.791; baseline=39999.052; entries=127797; live root `D:\Documents\Code\Rust\Exp\devsweep` |
| clean.threads_current_recorded | PASS | max_threads=8 |
| clean.peak_private_recorded | PASS | peak_private=5,738,496 |
| analyze.accounted_le_256mib | PASS | 38,906,299 B (CLI `--output`; not used for 512/200) |
| analyze.peak_private_le_512mib | PASS | 320,765,952 B (~305.9 MiB) |
| analyze.p95_cpu_le_200 | PASS | **190.16%** walk-until-partial. Not rounded down. Analyze domain. Not 0. |
| analyze.layout_p95_le_50ms | PASS | 1.4555 ms |
| analyze.react_commit_p95_le_100ms | PASS | **6 ms**, status=pass, warmups=5, measured=30, samples=30, HeadlessChrome port 2052 |
| analyze.cancel_ack_p95_le_500ms | PASS | p95=28 ms, all joined |
| software.p95_elapsed_le_5s | PASS | **3717.981 ms** on rebuilt CLI `46387a57…`. Raw: 3491.510, 3515.270, 3717.981, 3453.298, 3717.782 |
| software private/threads / no UAC | PASS | peak 14,028,800; threads 10; consent still 0 |
| optimize.list_preview.p95_le_1s | PASS | 841.356 ms; catalogue_count=8. Raw: 841.356, 707.334, 721.767, 788.902, 765.528 |
| optimize.list_preview private/threads | PASS numerically | peak_private=0 / threads=0 because `Wait-ProcessSamples` stops on the first absent `Get-Process` (process already exited). Elapsed clock is still process-create. Do not treat 0 as a measured walker private-bytes sample. |
| optimize.dns.p95_le_10s | PASS | 901.666 ms |
| optimize.settings.storage_recommendations.launch_p95_le_2s | PASS | 822.481 ms; clock=`devsweep_launch_return` |
| optimize.settings.search.launch_p95_le_2s | PASS | 914.136 ms |
| optimize.settings.energy_recommendations.launch_p95_le_2s | PASS | 1166.106 ms |

No leftover `devsweep` / `devsweep-desktop` after the run. `consent` count 0.
Integrity Medium. `summary.pass=true`; `failures` empty.

## Versus implementer round-6 (clue only)

Implementer claimed AC4 PASS by combining round-6 protocol exit 1 (Edge `:9223`
timeout on react-commit) with a later Chrome commit JSON (p95 5.70 ms). That
collage is not this check's pass. Walk-window Analyze CPU 187.19%, Software
~3584 ms, Clean ratio 0.381, CLI Status/idle/Optimize list pass were clues.

Independent recapture on commit `1cda3f2` with **rebuilt** desktop
`dd2f3bcb…` and independently rebuilt CLI (hash still `46387a57…`), **full**
script including Chrome commit harness:

- Confirmed Analyze walk-window CPU **190.16%** ≤ 200% and not 0. Per-rep
  windows are ~13.6–14.2 s to `partial`, not 600 s.
- Confirmed react-commit 5+30 samples p95 **6 ms** ≤ 100 ms on Chrome port 2052
  in this same run. Not Edge `:9223`. Not a bolted-on later JSON.
- Confirmed Software p95 ≤ 5 s on the rebuilt CLI (**3717.981 ms**).
- Confirmed Clean live-root ratio vs 39999.052 (**0.410**, entries 127797).
- Confirmed Status snapshot/live are CLI, not CDP-bloated desktop; idle is
  no-debug desktop. Those gates PASS.
- Peak Analyze private **305.9 MiB** (still ≤ 512 MiB).
- Optimize list p95 **841.356 ms** (still ≤ 1 s).

## Harness notes (not patched)

These are glue observations. They were **not** used to weaken 200%, 5 s, 1.20,
1 s, 100 ms, or 512 MiB.

1. Analyze CPU wall is Stopwatch; jsonl last status is `partial`; sample counts
   equal walk_samples; last wall ~13.6–14.2 s. `$MaxMs = 600000` did not fire.
2. `Wait-ProcessSamples` still breaks on the first absent sample, so short CLI
   workloads (optimize list/DNS) record private/threads 0.
3. `Set-Gate analyze.p95_cpu_le_200` requires a present p95 (`$null -ne` and
   `<= 200`). This run had a real p95 below 200, so the pass is valid. A missing
   p95 or a 0 from an idle-padded window would fail.
4. Clean comparison uses the live repo root and archived median 39999.052 ms on
   the same 24-core / build 26200 host. Entry count is recorded (127797) and is
   not required to equal 110393.
5. Independent desktop SHA differs from implementer `2bd36354…` after
   `just desktop-build` (NSIS patch of the exe). CLI SHA matched after rebuild.
6. React-commit already used Chrome on a free port; Edge `:9223` was not the
   only path. Sampler glue was not edited.

Domain return: none. Analyze walk-window CPU and react-commit both meet frozen
gates on this recapture.

## Native spot-check

| Scenario | Independent finding |
| --- | --- |
| EN/ZH 1440 all five modes + support | 35 screenshots present; Status EN/ZH copy and unsupported-capability note recorded in capture-log |
| Widths 390/800/1024 | Present; capture-log `width_probe` innerWidth matches |
| Device scale 100/125/150/200 | `dpi_probe` DPR 1 / 1.25 / 1.5 / 2; OS global scale not changed |
| Keyboard / AX | Tab Status→Protection in capture-log |
| Cancel / nav / restart | analyze_cancel joined; leave_live_to_clean; restart_relaunch recorded |
| Icon / integrity / no UAC | ICO hashed in capture-log; Medium; Administrators deny-only |
| Software inventory | 1293 entries / 86 selectable / 1207 manual in capture-log; leaked `${{arpDisplayName}}` on screenshot |
| Sleep / Software uninstall | UNVERIFIED, sanctioned, not completion-required |

## Remaining UNVERIFIED

| Item | Completion-required? | Reason |
| --- | --- | --- |
| Sleep / resume | No | Host sleep not reproduced; capture-log records the sanction |
| Software uninstall | No | No confirmed disposable current-user MSIX identity; no install/sign/MSI execute |

## Stop condition

Independent recapture of frozen `five-mode-v1` is Overall PASS. Do not weaken
thresholds. Do not patch Analyze walker from this check. Do not treat round-4
padded `p95cpu=0` as Complete. Do not treat implementer round-6 + a separate
commit JSON as this check's evidence. Do not commit from this check.
