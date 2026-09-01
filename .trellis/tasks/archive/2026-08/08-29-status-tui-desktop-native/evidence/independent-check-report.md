# Independent trellis-check — status-tui-desktop-native

Task: `.trellis/tasks/08-29-status-tui-desktop-native`
Checker: independent trellis-check (direct verify + one product self-fix)
Date: 2026-08-31
Host: Windows 11 build 26200, x64, 24 logical processors, 64 GiB RAM, Medium integrity (`S-1-16-8192`), `consent.exe` count 0. Isolated `LOCALAPPDATA`. OS global scale was not changed. No UAC.

Implementer `evidence/verification-report.md` was treated as a clue only. All listed gates were re-run. Native GUI and resource samples live under `evidence/independent-native/` with **new PIDs** (not implementer 27688).

No git commit / push / merge / amend. Protected paths (`.trellis/.gitignore`, `README.md`, `justfile`, other task directories) were not modified. Optimize/Status collector `windows-sys` flags were not reverted.

## Overall: **PASS**

Completion-required UNVERIFIED: **no**.

Independent check **PASS**.

---

## Self-fix this child owns

Round-1 `rtk just ci` failed clippy `-D warnings` (`clippy::large_enum_variant`) on two Status presentation enums:

- `crates/devsweep-cli/src/tui/modes/status/mod.rs` `StatusAction::SnapshotFinished.snapshot`
- `desktop/src-tauri/src/status.rs` `DesktopStatusSnapshotResult::Completed.snapshot`

Fix: box `StatusSnapshotV1` in those variants (same pattern as `WorkerEvent::StatusSnapshotFinished` / live `StatusEventV1::Snapshot`). Serde JSON is unchanged. Re-ran the failed gate: `independent-just-ci-round2.log` **EXIT 0**. Same signature did not recur.

---

## R / AC ledger (child `prd.md` / `design.md` / `implement.md`)

| Clause | Result | Independent evidence |
| --- | --- | --- |
| **R1** snapshot first; explicit live; ≤60 in-memory points; drop on leave | **PASS** | TUI reducer tests (`independent-tui-status-after-clippy-fix.log`, 10 passed). Desktop reducer/workbench (`independent-desktop-web-check.log`, 157 tests including `status/state.test.ts` 60-cap + `released`). Native: snapshot `data-status=ready` with Start live before any live; Space on focused Start live → `status=live`; leave `#/clean` drops mode; restart new snapshot id. |
| **R2** availability labels; no GPU/thermal/fan zero cards; frozen V1 integers; process truncation metadata | **PASS** | Fixtures/decoders reject unknown fields/`gpu` (`contract.test.ts`). Native capability note EN/ZH; `gpuZero=false`; AX “Truncated by limit: 15 of 613 enumerated.” Network `0 B/s` is available idle rates, not missing→zero. Charts use `null` gap points + polyline breaks (`state.ts` `gapPoint` / `chartSegments`). |
| **R3** coordinator exclusivity; pause/leave/close cancel+join; skip without backlog; stale ids ignored | **PASS** | Desktop `kind: "status"` on `OperationCoordinator`; AppShell `cancelAndJoin` on route change; Tauri `StatusAlreadyRunning`. TUI `request_mode` cancel-requests Status then `Released` after join. Sequence mismatch → decode fail; other `operation_id` ignored. |
| **R4 / parent Status R4** bilingual a11y + native resource | **PASS** | Native ledger below. Resource round 2 on current release sha256 `4D1D764E…` PID **1248**. |
| **AC1** fake-stream coverage listed in PRD | **PASS** | TUI `tests.rs` (snapshot, live opt-in, interval, skip/stale, partial/no-battery, churn/truncation/budget, sequence/terminal, leave/close, 60-cap, drop). Desktop `state.test.ts` / `StatusWorkbench.test.tsx` / `parity.test.ts` / `contract.test.ts` (unknown event, leaked `cmdline`, extra `gpu`). CLI `unknown-event.json` + `broken-pipe-terminal.json`. |
| **AC2** CLI/TUI/Desktop fixture parity; charts never interpolate missing as zeros; generated decoders | **PASS** | Shared fixtures + `exact()` decoders. `types:generate` in `desktop-web-check` (32 named fixtures). Chart text alternatives `gap` / `缺口`. |
| **AC3** EN/ZH, keyboard sort/focus, SR names, widths/scales, reduced motion, high contrast | **PASS** | Native ledger. Keyboard Name sort changed first row and `aria-pressed`. AX 792 nodes: Refresh snapshot / Stop live / truncation; no cmdline. Widths 390/800/1024/1440 innerWidth matched. DPR 1 / 1.25 / 1.5 / 2 via `--force-device-scale-factor` only. |
| **AC4** native resource + frontend/TUI tests + desktop build + `just ci` | **PASS** | Command table. Resource round 2 all gates true. |

Parent Status R4 post-exit was evaluated as **CPU ≤ idle median + 0.5 pp** and **threads ≤ idle max + 1** on every final-five sample (five-mode protocol), not the looser idle+64 MiB / idle+4 hold the implementer harness used.

---

## Command + exit table

| Log | Command | Exit |
| --- | --- | --- |
| `evidence/independent-cli-status.log` | `rtk cargo test --locked -p devsweep-cli -- status` | **0** (16 passed) |
| `evidence/independent-tui-status.log` | `rtk cargo test --locked -p devsweep-cli -- tui::modes::status` | **0** (10 passed) |
| `evidence/independent-desktop-web-check.log` | `rtk just desktop-web-check` | **0** (types:generate, lint, typecheck, 157 tests, vite build) |
| `evidence/independent-desktop-test.log` | `rtk just desktop-test` | **0** (34 passed, 2 ignored) |
| `evidence/independent-desktop-build.log` | `rtk just desktop-build` | **0** (pre-fix release) |
| `evidence/independent-git-diff-check.log` | `rtk git diff --check` | **0** |
| `evidence/independent-fmt-check.log` | `rtk cargo fmt --all -- --check` | **0** |
| `evidence/independent-native-capture.log` | `rtk node evidence/independent-native/capture-native-status.mjs` | **0** |
| `evidence/independent-resource-gate.log` | `rtk node evidence/independent-native/resource-gate.mjs` | **0** (pre-fix binary) |
| `evidence/independent-just-ci.log` | `rtk just ci` | **1** (clippy `large_enum_variant`; tests had already started) |
| `evidence/independent-cli-status-after-clippy-fix.log` | `rtk cargo test --locked -p devsweep-cli -- status` | **0** (16 passed) |
| `evidence/independent-tui-status-after-clippy-fix.log` | `rtk cargo test --locked -p devsweep-cli -- tui::modes::status` | **0** (10 passed) |
| `evidence/independent-desktop-test-after-clippy-fix.log` | `rtk just desktop-test` | **0** |
| `evidence/independent-fmt-after-clippy-fix.log` | `rtk cargo fmt --all -- --check` | **0** |
| `evidence/independent-just-ci-round2.log` | `rtk just ci` | **0** |
| `evidence/independent-git-diff-check-after-fix.log` | `rtk git diff --check` | **0** |
| `evidence/independent-desktop-build-after-fix.log` | `rtk just desktop-build` | **0** (current release sha256 `4D1D764E…`) |
| `evidence/independent-resource-gate-round2.log` | `rtk node evidence/independent-native/resource-gate.mjs` | **0** (current binary, PID 1248) |

---

## Native GUI ledger (isolated `LOCALAPPDATA`)

Harness: `evidence/independent-native/capture-native-status.mjs`. Log: `evidence/independent-native/capture-log.jsonl`. CDP ports 19671 / 19780+ (not implementer 9471/9571).

GUI capture used release sha256 `F9211E8F75EE8A718A7FDF64E825B7B6F003BBD13431EB598B0A9451BD17400A` (pre-box). After the clippy layout fix the rebuilt exe is `4D1D764ED3FFD3783B732AEFADD3C109F6ADD6FC374A9EFBC82B7C5EAF8ABC1C`. JSON/UI/copy did not change; resource R4 was re-measured on the current hash.

PIDs: **56536** (main EN/ZH), **74748** (DPI 100), **68852** (125), **70048** (150), **5748** (200). Leftover `devsweep-desktop` before launch: none. Implementer PID 27688 was not reused.

| Check | Result | Artifact |
| --- | --- | --- |
| English widths 1440/1024/800/390 | PASS; innerWidth matched | `screenshots/status-snapshot-en-{width}.png` |
| Chinese widths 1440/1024/800/390 | PASS; store `{"schema_version":1,"language":"zh-CN"}` | `status-snapshot-zh-*.png`; `localappdata/DevSweep/settings/presentation-v1.json` |
| Keyboard process sort | PASS; Space on focused Name; `aria-pressed` Name true; first row changed | capture-log `keyboard_sort_name` |
| Keyboard Start live | PASS; Space on focused Start live → `status=live`, Stop live | `status-live-started-en.png` |
| Screen reader / AX | PASS; 792 nodes; Refresh/Stop; truncation; no cmdline | `axtree/status-en-full-axtree.json` |
| Reduced motion | PASS | `status-reduced-motion-en.png` |
| High contrast | PASS; `forced:true` | `status-high-contrast-en.png` |
| Explicit live / interval 5 s | PASS; interval=`5000`, still live | `status-interval-change-en.png` |
| Process privacy | PASS; keys name/pid/cpu/private/read/write only; `privacyHit=false` | body + AX; 15 rows; truncated-by-limit |
| Leave / restart / close | PASS | capture-log `leave`, `restart`, `close` |
| Device scale 100/125/150/200 | PASS; dpr 1 / 1.25 / 1.5 / 2 | `status-dpi-{percent}-en.png` |
| Charts missing≠zero | PASS | Unit tests + `null` gaps; native live start had empty chart (not zeros); capability note says unsupported are not shown as zero |
| No GPU zero cards | PASS | EN/ZH copy + `gpuZero=false` |

---

## Resource gate (current release binary, PID 1248)

Raw samples: `evidence/independent-native/resource/{idle,live-60s,post-exit,summary}.json`. Round-1 (pre-fix, PID 47716) retained as `summary-round1-pre-clippy-fix.json`.

Idle: 10 s @ 200 ms (50 present). Live started only from explicit Start live. 60 s live @ 200 ms (300/300 present). Post-exit: **immediate** 5.0 s window, 25 scheduled samples, **no padding**.

| Gate | Limit | Measured | Pass |
| --- | --- | --- | --- |
| Snapshot p95 | ≤ 2000 ms | 820 ms (820, 805, 805, 819, 813) | yes |
| Live private peak | ≤ idle max + 64 MiB | 6 844 416 ≤ 5 926 912 + 67 108 864 | yes |
| Live threads peak | ≤ idle max + 4 | 37 ≤ 36 + 4 | yes |
| Live CPU median | ≤ 5% of one core | 0% | yes |
| Live CPU p95 | ≤ 15% of one core | 7.781% | yes |
| Post-exit samples | exactly 25 / 5 s | 25 | yes |
| Final five hold | CPU ≤ idle median + 0.5 pp **and** threads ≤ idle max + 1 | CPU 0 ≤ 0.5; threads 33 ≤ 37; all five samples present | yes |
| Missing sample | fail if missing | none | yes |

Post-exit the process was still present (WebView2 shutdown) with **0% CPU** and **33 threads** (below idle 36). That meets parent R4 quiescence. It is stronger than a padded “all absent after wait+kill” trace. Force-kill ran **after** the 25-sample window, path-matched to this exe only.

---

## Remaining UNVERIFIED

| Clause | Completion-required? | Notes |
| --- | --- | --- |
| Human Narrator listen-through | No | AX tree + labelled controls + `aria-live` match the Software/Optimize presentation bar. |
| OS-global display scale 125/150/200 | No | Forbidden to change user display settings. Device scale was emulated on WebView2. |
| Sleep/resume host churn | No | Not required by this child’s PRD; process table churn (15 rows, changing first process) was observed on this host. |

---

## Notes that are not failures

- Design file list mentions `desktop/src/app-shell/registry.ts`; this repo registers Status in `App.tsx` against the existing shell `MODE_IDS`. No competing desktop spec was added.
- English settings file was unread immediately after `setLocale('en')` (race); Simplified Chinese store bytes were captured.
- Many NIC cards push sparkline figures below the default viewport; empty live charts at T+0 are empty, not zero-filled.
