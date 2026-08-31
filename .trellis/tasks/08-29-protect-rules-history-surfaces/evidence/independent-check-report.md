# Protection, Rules, and History — independent check report

**Overall: PASS**

Independent trellis-check against revised `prd.md`, `design.md`, and `implement.md`.
Implementer `evidence/verification-report.md` was treated as a clue only.
Gates were re-run. Native GUI was recaptured with new PIDs and isolated
`LOCALAPPDATA`/`APPDATA`. No commit. No OS global scale or high-contrast change.

Date: 2026-08-31. Host: Windows 11 build 26200, x64, Medium integrity (`S-1-16-8192`),
`consent.exe` count 0.

## Self-fix (product bug, round 1 of 3)

`protection_list_get` / `protection_list_set` still existed. `protection_list_set`
called `UserProtectionList::replace()`, which takes the sidecar lock but **does not**
require confirmation and **does not** emit Clean V1 mutation audit (SHA-256 identity).
The supporting UI uses `protection_add` / `protection_remove`, but the registered
Tauri command remained a bypass.

Fix in this child only (`desktop/src-tauri/src/commands.rs`):

- `protection_list_get` remains a fail-closed read (`UserProtectionList::load` mapped
  through `CommandError::protection`).
- `protection_list_set` stays registered and returns `protection_confirmation_required`
  without writing the store. Mutations must use `protection_add` / `protection_remove`
  with explicit confirmation (lock + Clean V1 audit).
- Regression: `protection_list_set_cannot_bypass_confirm_lock_or_audit`.

Did not edit `cli.rs`, `justfile`, `README.md`, `.trellis/.gitignore`, other task
directories, or Optimize/Status `windows-sys` flags.

## R / AC trace

| Clause | Result | Independent evidence |
| --- | --- | --- |
| R1 / AC1 lock, byte preservation, audit, deny-to-clean | PASS | core-protection 15; corrupt `{not-json` and newer `{"version":2,...}` bytes preserved after UI; concurrent CLI PIDs 66648 + 51696 both `committed`; audit JSONL has `identity_sha256` only (no `keep-a`/`keep-b` path) |
| R2 / AC2 inspect-only rules | PASS | cli-rules 4; `rules-en-detail.png`; AX 410 nodes, `replayLike` empty |
| R3 / AC3 three V1 stores, mixed versions, redaction, no replay | PASS | core-history 7; history list shows `unknown_schema` + `redacted_forbidden_fields`; body/AX probes `secretKeep`/`msiexec`/`argv`/`runRetry` all false; no Run/Retry control; operation-id buttons only |
| R4 / AC4 bilingual, a11y, native, `just ci`, no sixth mode | PASS | EN/ZH 390–1440; keyboard confirm dialog; CDP forced-colors `forced:true`; `just ci` `ci complete`; primary segment remains Clean/Software/Optimize/Analyze/Status |

## Product checks

| Check | Result |
| --- | --- |
| No sixth primary nav mode | PASS. Desktop `MODE_IDS` and TUI `ModeId::ALL.len() == 5`. Native buttons: five mode tabs plus supporting Protection/Rules/History |
| Corrupt/unknown protection bytes preserved, never auto-cleared | PASS. After UI: `stores/corrupt-after-ui.json` is `{not-json`; `stores/newer-after-ui.json` is `{"version":2,"paths":["C:/future"]}` |
| History opens only three fixed V1 stores; no `--audit-log` discovery | PASS. `history_store_paths()` uses only `clean_audit_v1_path` / `software_audit_v1_path` / `optimize_audit_v1_path`. Core test `history_reader_opens_only_the_three_fixed_v1_stores` |
| Protection mutations audit SHA-256 identity, never raw path | PASS. Concurrent Clean V1 journal: `evidence.identity_sha256` only |
| `protection_list_get`/`protection_list_set` must not bypass confirm/lock/audit | PASS after self-fix. Get is fail-closed read. Set refuses mutation with `protection_confirmation_required` |
| Frozen CLI spellings only | PASS. Did not edit `cli.rs`. Handlers are `clean protect list\|add\|remove`, `clean rules list\|show`, `history list\|show`. Clap still requires `--confirm` on protect add/remove |

Completion-required native items: **none UNVERIFIED**.

## Focused and repository gates (real exits)

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test --locked -p devsweep-core -- protection` | 0 (15) | `independent-core-protection.log` |
| `rtk cargo test --locked -p devsweep-core -- history` | 0 (7) | `independent-core-history.log` |
| `rtk cargo test --locked -p devsweep-cli -- protect` | 0 (4) | `independent-cli-protect.log` |
| `rtk cargo test --locked -p devsweep-cli -- rules` | 0 (4) | `independent-cli-rules.log` |
| `rtk cargo test --locked -p devsweep-cli -- history` | 0 (4) | `independent-cli-history.log` |
| `rtk just desktop-web-check` | 0 (170 tests) | `independent-desktop-web-check.log` |
| `rtk just desktop-test` | 0 (38; includes bypass regression) | `independent-desktop-test.log` |
| `rtk git diff --check` | 0 | `independent-git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 | `independent-fmt-check.log` |
| `rtk just ci` | 0 (`ci complete`) | `independent-just-ci.log` |
| `rtk just desktop-build` (needed for native exe) | 0 | `independent-desktop-build.log` |
| `rtk node evidence/independent-capture-native-support.mjs` | 0 | `independent-native-capture.log` + `independent-native-desktop/capture-log.jsonl` |

## Native GUI (isolated profiles, production exe)

| Check | PID | Artifact |
| --- | --- | --- |
| EN protection 1440/1024/800/390 | 3404 | `independent-native-desktop/screenshots/protect-en-{1440,1024,800,390}.png` |
| EN rules + detail | 3404 | `rules-en-1440.png`, `rules-en-detail.png`, widths |
| EN history + no-replay detail | 3404 | `history-en-1440.png`, `history-en-detail-no-replay.png`, widths |
| Keyboard add confirm (Space on Add path) | 3404 | `protect-en-keyboard-confirm.png`; `keyboard_confirm_add` `open:true` title “Add this exact path…” buttons Cancel / Add path |
| Forced-colors | 3404 | `protect-en-high-contrast.png`, `history-en-high-contrast.png`; probe `forced:true` `contrast:true` |
| AX / SR labels | 3404 | `axtree/protection-en.json` (112), `rules-en.json` (410), `history-en.json` (172), `history-zh.json` (172) |
| ZH protection/rules/history 1440–390 | 3404 | `protect-zh-1440.png`, `protect-zh-CN-*`, `rules-zh-*`, `history-zh-*`; store `{"schema_version":1,"language":"zh-CN"}` |
| Corrupt store fail-closed, bytes preserved | 83188 | `protect-en-corrupt-fail-closed.png`; `stores/corrupt-after-ui.json` still `{not-json` |
| Newer-version store fail-closed, bytes preserved | 22344 | `protect-zh-newer-fail-closed.png`; `stores/newer-after-ui.json` still `{"version":2,"paths":["C:/future"]}` |
| Concurrent writers (two CLI processes) | 66648, 51696 | both exit 0 `committed`; store 2 paths; desktop PID 49136 `protect-en-concurrent-two-writers.png` |
| Mixed audit + redaction | 3404 | `snapshots/history-detail-en.txt`: `redacted_forbidden_fields`, `unknown_schema`; forbidden probe all false |
| History cannot run/retry | 3404 | `history-en-detail-no-replay.png`; buttons are operation ids / five-mode / supporting dests only |

AX `replayLike` on history-en matches the inspect-only sentence “cannot retry or replay”, not a Run control.

## Notes

- First native attempt used `cargo build --release -p devsweep-desktop`, which still bound WebView2 to Vite `http://127.0.0.1:4180` (`ERR_CONNECTION_REFUSED`). That is not a product defect. Recapture used `just desktop-build` (`tauri.localhost`).
- Display paths on the Protection list are user-facing data (design). History redaction applies to audit payloads, not to the protection allowlist table.
- No blockers. Independent check **PASS**.
