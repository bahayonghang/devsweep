# Five-mode native validation

Native evidence for the integrated five-mode Windows product. Device scale uses
WebView2 `--force-device-scale-factor` only. The host OS global scale, power
plan, and Defender exclusions are not changed by the agent. Antivirus
observations are environment notes, not correctness passes.

## Environment

See `evidence/resources/host-manifest.json` for CPU, cores, RAM, arch, Windows
build, power plan, toolchains, commit, binary SHA-256, Defender, locale, and
fixture digests. Standard user, Medium integrity, no UAC.

Recorded host (current protocol run `raw-20260831T205206Z` after `1cda3f2`,
react-commit recapture `raw-20260831T212913Z`; prior runs kept, including
round-5 `raw-20260831T185925Z` and invalid padded-CPU
`raw-20260831T173233Z`): Intel Core Ultra 9 275HX, 24 logical cores, 64 GiB
RAM, AMD64, Windows 11 build 26200, Balanced power plan, rustc 1.98.0, Node
v26.7.0, commit `1cda3f2`, CLI SHA-256
`46387a57944641fe4dde99353a69fe0b3c6d0627777cb044cc19129d92d0d43b`,
desktop SHA-256
`2bd3635482425a1df2de7a7b233a18275d26866b9294a1fa0e09c8bf1ff9db72`, Defender
services disabled on this host, locale `zh-CN`, Medium integrity.

## Scenario matrix

| Scenario | Method | Status | Artifact |
| --- | --- | --- | --- |
| English / Chinese | Settings language then each primary mode and support route | PASS | `evidence/native/screenshots/en-*-1440.png`, `zh-*-1440.png` |
| Keyboard / screen reader | Tab from Status; CDP AX tree 81 nodes | PASS | `evidence/native/axtree/`, `capture-log.jsonl` `keyboard_*` |
| Reduced motion / high contrast | CDP `prefers-reduced-motion` and `forced-colors` | PASS | `en-optimize-reduced-motion.png`, `en-optimize-high-contrast.png` |
| Widths 390 / 800 / 1024 / 1440 | `Emulation.setDeviceMetricsOverride` | PASS | `en-status-{390,800,1024}.png`, `*-1440.png` |
| Device scale 100 / 125 / 150 / 200 | `--force-device-scale-factor` | PASS | `dpi-{100,125,150,200}-en-status.png`; DPR 1 / 1.25 / 1.5 / 2 |
| Cancellation / navigation / exit | Leave live Status to Clean; analyze cancel join; Browser.close | PASS | `capture-log.jsonl` `leave_live_to_clean`, `analyze_cancel` |
| App restart | Isolated `LOCALAPPDATA` relaunch; zh-CN store restored | PASS | `restart-status.png`, `restart_relaunch` |
| Taskbar / window icon | Generated ICO copied and hashed | PASS | `evidence/native/icon/icon.ico` |
| Capability / unsupported states | Status unsupported note; Software MSI manual; Optimize catalogue | PASS | `en-status-capability.png`, `en-software-capability.png` |
| Sleep / resume | Host transition | UNVERIFIED | sanctioned; not a substitute for other rows |
| Software inventory | Live standard-user inventory (1293 entries) | PASS | `en-software-inventory.png` |
| Software uninstall | Exact disposable current-user MSIX only | UNVERIFIED | no confirmed identity; no install/sign/MSI execute |
| Clean Recycle Bin fixture | Task-owned trash target only | PASS | `evidence/native/clean/recycle-note.json` |
| DNS flush | `optimize run` `dns.flush` | PASS | `evidence/resources/gates.json` `optimize.dns.p95_le_10s` |
| Settings URI launches | Frozen catalogue URIs; launched ≠ completion; clock is DevSweep launch return | PASS (storage 896 ms, search 892 ms, energy 1169 ms) | `evidence/resources/summary.json` |

Statuses are filled from the recorded native capture and resource protocol.
Completion-required UNVERIFIED rows are only the sanctioned Software uninstall
and non-reproducible sleep/resume cases.

## Resource protocol

`tools/measure-resources.ps1 -Protocol five-mode-v1` recorded idle (no
remote-debug desktop), CLI Status snapshot/live, live-repo Clean, Software
`--source all`, Optimize list of eight before Analyze, and five desktop IPC
Analyze reps on `analysis-250k-v1` until `partial` or `complete` (~13 s
walk). CPU% uses Stopwatch `wall_ms` on walk samples only. Round-4
`p95cpu=0` after a 600 s idle pad is invalid and is not AC4. React-commit
collection is the product Vite `benchmark:analyze` page (5 warm-ups + 30
measured) via headless Chrome CDP on a free port (`tools/run-analyze-react-commit.mjs`),
not Edge `:9223`.

Recorded result: **PASS**. See
`.trellis/tasks/08-29-five-mode-native-integration/evidence/resources/gates.json`,
`raw-20260831T205206Z/`, and recapture `raw-20260831T212913Z/`. Walk-window
Analyze CPU p95 **187.19%** (not 0). Peak private **344.2 MiB**. Optimize
list p95 **698.726 ms**. Software p95 **3584 ms**. Clean ratio **0.381**.
React-commit p95 **5.70 ms** (30 samples, status=pass, threshold 100 ms).
Thresholds are not weakened. Analyze walker/pool was not patched.
