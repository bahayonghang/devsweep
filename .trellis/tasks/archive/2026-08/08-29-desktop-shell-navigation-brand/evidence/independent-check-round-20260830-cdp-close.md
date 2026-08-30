# Independent Phase 2.2 check — CDP close recapture

Date: 2026-08-30

Result: **PASS**

This report is an independent `trellis-check` of in-progress task
`.trellis/tasks/08-29-desktop-shell-navigation-brand` after the evidence-only
native recapture. It does not reuse implementer command logs as this check's
gates. `just desktop-build` was not run. Frozen artifacts were hashed before
the first gate and after the last gate; they did not drift.

Scaling 100/125/150/200%, native High Contrast, and native Reduced Motion
remain the user-approved **WAIVED/UNVERIFIED** observations and are not
represented as PASS.

Protected dirty files `.trellis/.gitignore`, `README.md`, and `justfile` were
not edited and remain unstaged. Other `08-29-*` task directories were out of
scope. No git commit, push, amend, archive, or parent start.

## Independent command results

`rtk` compact wrappers were used first for the cargo filters (all exit 0).
Complete evidence logs are the unfiltered `cargo` / Node 22 `npm` / `just`
reruns below. Desktop npm gates used `mise` Node `22.23.2` as required by
`.trellis/spec/desktop-frontend/index.md` Quality Check and
`desktop/package.json` `engines.node`.

| Command | Exit | Result | Complete log |
| --- | ---: | --- | --- |
| `rtk cargo test -p devsweep-core presentation_settings` then unfiltered `cargo test -p devsweep-core presentation_settings` | 0 / 0 | 8 passed; 171 filtered | `evidence/independent-check-round-20260830-cdp-close-01-core-presentation-settings.log` |
| `rtk cargo test -p devsweep-cli i18n` then unfiltered `cargo test -p devsweep-cli i18n` | 0 / 0 | 10 passed; 98 filtered | `evidence/independent-check-round-20260830-cdp-close-02-cli-i18n.log` |
| `rtk cargo test -p devsweep-cli tui::shell` then unfiltered `cargo test -p devsweep-cli tui::shell` | 0 / 0 | 4 passed; 104 filtered | `evidence/independent-check-round-20260830-cdp-close-03-tui-shell.log` |
| `rtk cargo test -p devsweep-cli tui::app` then unfiltered `cargo test -p devsweep-cli tui::app` | 0 / 0 | 31 passed; 77 filtered | `evidence/independent-check-round-20260830-cdp-close-04-tui-app.log` |
| `rtk cargo test -p devsweep-cli tui::runtime` then unfiltered `cargo test -p devsweep-cli tui::runtime` | 0 / 0 | 11 passed; 97 filtered | `evidence/independent-check-round-20260830-cdp-close-05-tui-runtime.log` |
| `rtk cargo test -p devsweep-cli tui::render` then unfiltered `cargo test -p devsweep-cli tui::render` | 0 / 0 | 24 passed; 84 filtered | `evidence/independent-check-round-20260830-cdp-close-06-tui-render.log` |
| `rtk cargo test -p devsweep-desktop debug_native_fault` then unfiltered `cargo test -p devsweep-desktop debug_native_fault` | 0 / 0 | 1 passed; 22 filtered | `evidence/independent-check-round-20260830-cdp-close-07-debug-native-fault.log` |
| focused `npm test -- src/i18n/index.test.ts src/app-shell/operation-coordinator.test.ts src/app-shell/AppShell.test.tsx src/App.test.tsx src/styles.test.ts src/lifecycle.test.ts` (cwd `desktop`, Node 22) | 0 | 6 files, 67 passed | `evidence/independent-check-round-20260830-cdp-close-08-focused-npm-tests.log` |
| `npm run lint` (cwd `desktop`, Node 22) | 0 | eslint `.` no findings | `evidence/independent-check-round-20260830-cdp-close-09-npm-lint.log` |
| `npm run typecheck` (cwd `desktop`, Node 22) | 0 | `tsc --noEmit` no findings | `evidence/independent-check-round-20260830-cdp-close-10-npm-typecheck.log` |
| `just desktop-web-check` (Node 22 on PATH) | 0 | types:generate, lint, typecheck, 12 files / 100 tests, Vite build | `evidence/independent-check-round-20260830-cdp-close-11-desktop-web-check.log` |
| `just desktop-test` | 0 | 21 passed, 2 ignored fixtures | `evidence/independent-check-round-20260830-cdp-close-12-desktop-test.log` |
| `git diff --check` | 0 | no whitespace errors; autocrlf LF/CRLF warnings only | `evidence/independent-check-round-20260830-cdp-close-13-git-diff-check.log` |
| `python -X utf8 ./.trellis/scripts/task.py validate .trellis/tasks/08-29-desktop-shell-navigation-brand` | 0 | implement.jsonl 7 entries, check.jsonl 3 entries | `evidence/independent-check-round-20260830-cdp-close-14-task-validate.log` |
| `just ci` | 0 | fmt, offline lock, check, workspace tests, clippy `-D warnings`, `ci complete` | `evidence/independent-check-round-20260830-cdp-close-15-just-ci.log` |

Preflight and final frozen hashes:
`evidence/independent-check-round-20260830-cdp-close-00-preflight-hashes.log`,
`evidence/independent-check-round-20260830-cdp-close-16-final-hashes.log`.

Non-blocking observation: `devsweep-desktop` test compiles emit MSVC linker
stdout `正在创建库 ...dll.lib` under `#[warn(linker_messages)]`. Clippy
`-D warnings` still finished clean (`independent-check-round-20260830-cdp-close-15-just-ci.log:399-400`).
This does not change `target/debug/devsweep-desktop.exe`.

## Frozen hash table

Independent SHA-256 before the first gate and after `just ci`. No drift.
`just desktop-build` was not run.

| Artifact | Bytes | SHA-256 | First | Last |
| --- | ---: | --- | --- | --- |
| `target/release/devsweep-desktop.exe` | 9,827,328 | `92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20` | match | match |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5` | match | match |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,152,372 | `0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328` | match | match |

mtime of the three artifacts remained `2026-08-30 20:05:38` / `19:42:32` /
`20:05:37` local.

## Independently hashed recapture artifacts

| Artifact | Bytes | Independent SHA-256 |
| --- | ---: | --- |
| `native-release/localappdata/DevSweep/settings/presentation-v1.json` | 39 | `2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899` |
| `native-release/cdp-messages.jsonl` | 19,265 | `E455586A37FF428BCD984AC3A7B76E7E43FA6AE2B9B8E641E4E63676E87DF266` |
| `native-release/02-native-release-rebind.log` | 8,020 | `CAE69BC936C590D534C78425DFC6DAE5CB6CB24F3CE28DA7750DB2810747A336` |
| `outer-exit.json` |  | `5EE23C5CC0B85FC9EEA3598F8A81AE3B20521F3A7C609DA1B56257EC0718BB63` |
| `native-release/shell-tray-wnd-uia-raw.json` |  | `3E359FFF1046A95F76FD16F61B2B96114BCB8B8BCFB97F7B3296F0B7051E891C` |
| `native-release/verification.json` |  | `5B85206BA8E50BA7509575506EC9B8607380BB2622714266A12273999E05B6F9` |
| `screenshots/release-first-zh-CN.png` | 14,104 | `146437FC610C988C2DCF4056C1AD63E430FB76EA4666721312E94E55AA6BF3D3` |
| `screenshots/release-restart-after-back.png` | 16,907 | `B5004269C8E8C80E808FF0250AAC98DAAB20BD558285CD63517B30A4453DE97F` |
| `screenshots/release-taskbar-bound-desktop-20260830-214341528.png` | 1,890,694 | `19A5514DFEEEE67A4CCFDD1252EEDE0900CCBC6307E18A0FDB827799CDFD9DB8` |
| `screenshots/release-taskbar-bound-crop-20260830-214341528.png` | 68,389 | `BA1244E6CA4715801625E1E0A406F63C70EDDA47DC3296A928512C7374AA2978` |
| `screenshots/release-taskbar-bound-app-reference-20260830-214341528.png` | 31,224 | `231E5E776177CA78FB9DF56A4A043D9D497E9C930FD42CDBC7715B05CF0A4908` |

Exact V1 UTF-8 bytes:
`{"schema_version":1,"language":"zh-CN"}`
(`native-release/localappdata/DevSweep/settings/presentation-v1.json:1`).

## Native clause table

PASS run root:
`evidence/native-cdp-close-recapture-20260830/`
(not `attempt1` / `attempt2` / `attempt3`).

| Clause | Result | Independent evidence |
| --- | --- | --- |
| helper_exit 0 | PASS | `outer-exit.json:3` `"helper_exit": 0`; transcript `02-native-release-rebind.log:59` `HELPER_EXIT_CODE=0` |
| CloseMainWindow while socket Open, then WaitForExit, then detach/dispose | PASS | first: `02-native-release-rebind.log:33-37` `socket=Open`, `WAIT_FOR_EXIT ... elapsed_ms=41`, `CDP_Target.detachFromTarget`; `verification.json:183-191` `socket_state_before=Open`, `page_close_used=false`, `browser_close_used=false`, `close_order=CloseMainWindow-WaitForExit-Target.detachFromTarget-dispose`. Restart: `02-native-release-rebind.log:47-51`; `verification.json:799-807` |
| First owned webview 0 without identity-kill | PASS | `02-native-release-rebind.log:38` `RESIDUE i=1 after_ms=818 main=0 owned=0`; `verification.json:205` `owned_webview_count=0`; `verification.json:217` `identity_gated_termination=[]` |
| Restart owned webview 0 without identity-kill | PASS | `02-native-release-rebind.log:52` `RESIDUE i=1 after_ms=798 main=0 owned=0`; `verification.json:821` `owned_webview_count=0`; `verification.json:833` `identity_gated_termination=[]` |
| Store 39 bytes SHA `2CD9EEDF…` | PASS | independent hash of remaining file; `verification.json:146-159`; `02-native-release-rebind.log:32` |
| Taskbar bound to restart HWND | PASS | restart HWND `0x1C7A0904` (`verification.json:225-226` value 477759748); `verification.json:717-758` `expected_hwnd=0x1C7A0904`, `shell_tray_wnd=0x10162`, `exact_match_count=1`, bound foreground HWND/PID/path/hash `0x1C7A0904` / 47692 / frozen release; three captures `20260830-214341528` |
| CDP jsonl SHA `E455586A…` | PASS | independent hash of `native-release/cdp-messages.jsonl`; `outer-exit.json:34`; `02-native-release-rebind.log:58` `cdp_lines=50 cdp_sha256=E455586A…` |
| Two-run error arrays empty | PASS | `verification.json:836-843` all empty; `02-native-release-rebind.log:54` `ERROR_AUDIT_PASS runtime=0 console=0 log=0` |
| No-flag zh restart + Settings Back/opener | PASS | restart locale `zh-CN` `#/clean` (`verification.json:248-250`); after Back `activeTag=BUTTON` `activeText=语言` (`verification.json:276-280`); `02-native-release-rebind.log:45` |
| Attempt1/2/3 are failed method history, not PASS | PASS (excluded) | see below |

### Failed method history (not PASS)

| Attempt | helper_exit | Why it is not the PASS run |
| --- | ---: | --- |
| `attempt1-nettcp-missing/` | 1 | `wrapper-stdout.log:14` `Get-NetTCPConnection` missing; `outer-exit.json:3` |
| `attempt2-target-close-hung-main/` | 1 | `02-native-release-rebind.log:28` `HELPER_FAILURE first release close left residue`; folder records Target.close hung-main method; `outer-exit.json:3` |
| `attempt3-page-close-before-window/` | 1 | `wrapper-stdout.log:22` `CDP_Page.close` before window; residue `main=1` through `wrapper-stdout.log:28-33`; `IDENTITY_GATED_STOP count=1` at line 34; `outer-exit.json:3` |

The PASS run used CloseMainWindow-while-JS-alive, not Page.close / Browser.close
(`verification.json:184-186` and `:800-802`).

## File:line findings

Product/spec review against the desktop-frontend Quality Check and this child's
PRD/design/implement did not produce a blocking defect. Cited facts:

1. Native close order on the PASS run is CloseMainWindow while CDP socket Open,
   WaitForExit, detach, dispose
   (`verification.json:183-191`, `02-native-release-rebind.log:33-37` and
   `:47-51`). Attempt3's Page.close-before-window left `main=1`
   (`attempt3-page-close-before-window/wrapper-stdout.log:22-33`) and is not
   this PASS.
2. First and restart residue samples are owned webview 0 with empty
   `identity_gated_termination`
   (`verification.json:205,217` and `:821,833`;
   `02-native-release-rebind.log:38,52`).
3. Persisted V1 document is exact 39-byte `zh-CN` JSON
   (`presentation-v1.json:1`; `verification.json:146-153`).
4. Restart taskbar bind is unique Explorer `Shell_TrayWnd` `0x10162` to HWND
   `0x1C7A0904` PID 47692 frozen path/hash
   (`verification.json:717-758`).
5. Automated AC7 gates in this round all exited 0. Focused Desktop tests 67/67;
   desktop-web-check 100/100 plus Vite build
   (`independent-check-round-20260830-cdp-close-08-focused-npm-tests.log:20-21`,
   `independent-check-round-20260830-cdp-close-11-desktop-web-check.log:28-42`).
6. `git diff --check` exit 0; only autocrlf warnings
   (`independent-check-round-20260830-cdp-close-13-git-diff-check.log:8-37`).
7. Task context validation 7 implement + 3 check entries
   (`independent-check-round-20260830-cdp-close-14-task-validate.log:11-14`).
8. `just ci` reached `ci complete` then clippy `-D warnings` finished
   (`independent-check-round-20260830-cdp-close-15-just-ci.log:375-400`).

## Remaining WAIVED/UNVERIFIED

| Observation | State |
| --- | --- |
| Native 100/125/150/200% scaling and exact native dimensions | **WAIVED/UNVERIFIED** |
| Native Windows High Contrast appearance | **WAIVED/UNVERIFIED** |
| Native Windows Reduced Motion appearance | **WAIVED/UNVERIFIED** |

Automated forced-colors / reduced-motion CSS tests remain implementation
coverage only and are not native PASS.
