# Verification report — 08-29-five-mode-native-integration

Glue-only leaf. No commit, push, sign, or release. Frozen thresholds were not
changed after viewing results. Analyze walker/pool was not patched.

Round-6 protocol artifacts plus react-commit recapture:
`evidence/resources/raw-20260831T205206Z/` (full five-mode-v1),
`evidence/resources/raw-20260831T212913Z/` (react-commit only),
`evidence/resources/summary.json`, `evidence/resources/gates.json`,
`evidence/resources/host-manifest.json`,
`evidence/logs/measure-resources-round6.log`.
Prior artifacts kept, including round-5 `raw-20260831T185925Z/` and
`round-5-20260831T185925Z/`, and invalid round-4 padded-CPU
`raw-20260831T173233Z/` / `round-4-20260831T173233Z/`.

## R / AC ledger

| ID | Verdict | Notes |
| --- | --- | --- |
| R1 | PASS | Five shipped roots and Tauri commands only. |
| R2 | PASS | Contract tests and native cancel/nav/restart recorded. |
| R3 | PASS | Standard-user Medium integrity; consent 0; native matrix recorded. |
| R4 | PASS | Frozen five-mode-v1 after react-commit recapture: all gates pass. Walk-window Analyze CPU p95 **187.19%**. React-commit p95 **5.70 ms**. |
| R5 | PASS | Docs describe shipped behavior. Packaging unsigned current-user NSIS only. |
| AC1 | PASS | Live recursive `task.py validate` exit 0 for this child and live parents. |
| AC2 | PASS | `just ci`, desktop web/test/build, types generate, tauri debug NSIS, `git diff --check` recorded exit 0. |
| AC3 | PASS | Native matrix exit 0. Sanctioned UNVERIFIED: sleep/resume, Software uninstall. |
| AC4 | PASS | Recapture `analyze.react_commit_p95_le_100ms` pass (p95=5.70, 30 samples). Round-6 other frozen gates remain pass, including `analyze.p95_cpu_le_200` p95cpu=187.186 (not 0). |

## Command + exit table

| Command | Exit | Log / artifact |
| --- | --- | --- |
| `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` | 0 | CLI SHA-256 `46387a57944641fe4dde99353a69fe0b3c6d0627777cb044cc19129d92d0d43b` |
| `rtk just desktop-build` | 0 | desktop SHA-256 `2bd3635482425a1df2de7a7b233a18275d26866b9294a1fa0e09c8bf1ff9db72` |
| `& .\tools\measure-resources.ps1 -Protocol five-mode-v1` (round 6) | **1** | Edge `:9223` CDP timeout on react-commit only. `raw-20260831T205206Z/` |
| `node tools/run-analyze-react-commit.mjs` (recapture) | 0 | `raw-20260831T212913Z/analyze-react-commit.json`; headless Chrome port 13428 |

Binaries are not `efbed437…` / `60a9d7ce…` / `6205b473…`. UI bench rebuilt; release desktop not rebuilt.

## Round-6 measurement windows

| Workload | Window |
| --- | --- |
| CPU% | Stopwatch `wall_ms`. Analyze CPU uses samples from start until `partial` or `complete` only. |
| Analyze 250k | Five desktop IPC reps. Terminal = `partial` **or** `complete` (250k stored-node cap). Median walk ~13.0 s, 66–67 samples. Not CLI JSON. Not 600 s. |
| Optimize list | After idle/status/software/clean, before Analyze. Catalogue of eight. |
| React-commit | Product Vite `benchmark:analyze` page. 5 warm-ups + 30 measured drill-ins. Collection is headless Chrome CDP on a **free** port (not Edge 9223). |
| Other | Idle desktop no-CDP; CLI Status snapshot/live/post-stop; live-repo Clean vs 39999.052; Software `--source all`. |

## Resource table (frozen five-mode-v1, round 6 + recapture)

Host: 24-core Windows build 26200, Balanced, commit `1cda3f2`.

| Gate | Result | Detail |
| --- | --- | --- |
| idle / Status snapshot / live / post-stop | PASS | idle maxThreads=35; snapshot p95=963 ms; live p95cpu=7.83 |
| clean.median_ratio_le_1_20 | PASS | ratio=0.381; median=15256.382; entries=127529 |
| analyze.accounted_le_256mib | PASS | 38.9 MiB |
| analyze.peak_private_le_512mib | PASS | 360878080 B (344.2 MiB) |
| analyze.p95_cpu_le_200 | PASS | **187.19%** walk-until-partial. Unchanged by recapture. |
| analyze.layout_p95_le_50ms | PASS | 1.51 ms |
| analyze.react_commit_p95_le_100ms | PASS | **5.70 ms**; status=pass; 30 samples; port 13428. Threshold still 100 ms. Round-6 Edge `:9223` timeout superseded. |
| analyze.cancel_ack_p95_le_500ms | PASS | 13 ms |
| software.p95_elapsed_le_5s | PASS | 3583.830 ms |
| optimize.list_preview.p95_le_1s | PASS | 698.726 ms |
| optimize.dns / Settings launches | PASS | DNS 908; storage 896; search 892; energy 1169 |

Round-4 `p95cpu=0` after 600 s of `partial` idle is not a pass and is not used for AC4.

## Domain return

None. Analyze walk-window CPU and react-commit both meet frozen gates. Glue did not patch walker/pool.
