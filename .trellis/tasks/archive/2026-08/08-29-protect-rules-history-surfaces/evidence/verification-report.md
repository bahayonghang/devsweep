# Protection, Rules, and History — implementer verification report

Implementer evidence only. This is **not** an independent trellis-check PASS.

Task: `.trellis/tasks/08-29-protect-rules-history-surfaces`
Date: 2026-08-31
Host: Windows 11 build 26200, x64, Medium integrity (`S-1-16-8192`), `consent.exe` count 0. Isolated `LOCALAPPDATA` + `APPDATA`. Device metrics and forced-colors via WebView2 CDP only — no OS global scale or high-contrast change. No commit.

Release desktop: `target/release/devsweep-desktop.exe` sha256 `6884e108c56b1eabf690b401d7107f47ad969323373c347e5ffdd9cfab5a13fd`.
Concurrent CLI: `target/debug/devsweep.exe` sha256 `5ec8fca4f52e15c724836bb8f797f4faadc8a44b858b56a75da72ebe9d2da85b`.

Driver: `evidence/capture-native-support.mjs` → `evidence/native-desktop/capture-log.jsonl`.

## R / AC trace

| Clause | Result | Evidence |
| --- | --- | --- |
| R1 / AC1 protection lock, byte preservation, audit, deny-all | PASS | core-protection 15; native corrupt/newer bytes preserved; concurrent CLI PIDs 56388 + 78380 both committed; desktop shows keep-a and keep-b |
| R2 / AC2 inspect-only rules | PASS | cli-rules; rules-en-detail.png; AX 410 nodes, no execute control |
| R3 / AC3 three stores, mixed versions, redaction, no replay | PASS | core-history; history list shows `unknown_schema` + `redacted_forbidden_fields`; body/AX have no `C:/secret-keep-native`, `msiexec`, argv; no Run/Retry button |
| R4 / AC4 bilingual, a11y, native, `just ci`, no sixth mode | PASS | EN/ZH 390–1440; keyboard confirm dialog; forced-colors; `just-ci.log` exit 0; five-mode nav only |

## Focused and repository gates

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test --offline -p devsweep-core -- protection` | 0 (15) | `evidence/core-protection.log` |
| `rtk cargo test --offline -p devsweep-core -- history` | 0 (7) | `evidence/core-history.log` |
| `rtk cargo test --offline -p devsweep-cli -- protect` | 0 (4) | `evidence/cli-protect.log` |
| `rtk cargo test --offline -p devsweep-cli -- rules` | 0 (4) | `evidence/cli-rules.log` |
| `rtk cargo test --offline -p devsweep-cli -- history` | 0 (4) | `evidence/cli-history.log` |
| `rtk cargo test --offline -p devsweep-cli -- tui::support` | 0 (3) | `evidence/cli-tui-support.log` |
| `rtk npm --prefix desktop run test -- --run support` | 0 (6) | `evidence/npm-support.log` |
| `rtk just desktop-web-check` | 0 | `evidence/desktop-web-check.log` |
| `rtk just desktop-test` | 0 | `evidence/desktop-test.log` |
| `rtk just desktop-build` | 0 | `evidence/desktop-build.log` |
| `rtk node evidence/capture-native-support.mjs` | 0 | `evidence/native-capture.log` |
| `rtk just ci` | 0 (`ci complete`) | `evidence/just-ci.log` |
| `rtk git diff --check` | 0 | `evidence/git-diff-check.log` |
| focused clippy `-D warnings` (core/cli/desktop) | 0 | `evidence/clippy-focused.log` |

## Native GUI (isolated LOCALAPPDATA + APPDATA, release desktop)

Integrity Medium. `consent.exe` none. CDP ports 9571–9574. Host DPR 1.25; widths emulated to innerWidth 1440/1024/800/390.

| Check | PID | Artifact |
| --- | --- | --- |
| EN protection 1440/1024/800/390 | 47328 | `native-desktop/screenshots/protect-en-{1440,1024,800,390}.png` |
| EN rules + detail | 47328 | `rules-en-1440.png`, `rules-en-detail.png`, widths |
| EN history + no-replay detail | 47328 | `history-en-1440.png`, `history-en-detail-no-replay.png`, widths |
| Keyboard add confirm (Space on Add path) | 47328 | `protect-en-keyboard-confirm.png`; capture-log `keyboard_confirm_add` `open:true` title “Add this exact path…” buttons Cancel / Add path |
| Forced-colors | 47328 | `protect-en-high-contrast.png`, `history-en-high-contrast.png`; probe `forced:true` `contrast:true` |
| AX / SR labels | 47328 | `native-desktop/axtree/protection-en.json` (112), `rules-en.json` (410), `history-en.json` (172), `history-zh.json` (172) |
| ZH protection/rules/history 1440–390 | 47328 | `protect-zh-1440.png`, `protect-zh-CN-{1440,1024,800,390}.png`, `rules-zh-*.png`, `history-zh-*.png`; store `{"schema_version":1,"language":"zh-CN"}` |
| Corrupt store fail-closed, bytes preserved | 55088 | `protect-en-corrupt-fail-closed.png`; `stores/corrupt-after-ui.json` still `{not-json` |
| Newer-version store fail-closed, bytes preserved | 68948 | `protect-zh-newer-fail-closed.png`; `stores/newer-after-ui.json` still `{"version":2,"paths":["C:/future"]}` |
| Concurrent writers (two CLI processes) | 56388, 78380 | both exit 0 `committed`; store 2 paths; desktop PID 47568 `protect-en-concurrent-two-writers.png` |
| Mixed audit + redaction | 47328 | `snapshots/history-detail-en.txt`: `redacted_forbidden_fields`, `unknown_schema`; forbidden probe all false; no execute/replay control |
| History cannot run/retry | 47328 | screenshot `history-en-detail-no-replay.png`; buttons are operation ids only |

CDP JSONL: `native-desktop/capture-log.jsonl` (events `app_launched`, `cdp_target`, `width_probe`, `keyboard_confirm_add`, `high_contrast_probe`, `axtree_captured`, `corrupt_store`, `newer_store`, `concurrent_writers`, `done`).

## Fixes made during native/ci

- Protection add form reads `FormData` `name="path"` so keyboard/IME submit can confirm.
- CLI protect/history tests share one process env lock so parallel `just ci` cannot clobber `APPDATA`.
- `cli_contract` Chinese human-error case now uses a missing protect path (history list is wired; exit 6 + `添加保护要求路径已经存在`).

## Remaining

- Legacy `protection_list_get` / `protection_list_set` IPC remains for compatibility; supporting UI uses confirmed add/remove.
- AX `replayLike` on history-en matches the inspect-only sentence “cannot retry or replay”, not a Run control.

No blockers. Native bar is PASS with artifacts. `just ci` exit 0.
