# Route-failure focus repair verification

Date: 2026-08-30

Decision: **the focused product repair, all automated gates, the closed offline
debug audit, and the final third new-release native attempt pass. The original
debug command remains exit 1 because its pre-audit harness treated the sole
Vite `favicon.ico` 404 as a product error; the reproducible closed audit proves
that no runtime/console/window error occurred. New release locale persistence,
restart, Back/focus, and natural-close evidence pass. Taskbar appearance remains
UNVERIFIED because no taskbar UIA control could be directly bound to the final
PID/HWND. Overall state remains implementer-verified and awaiting independent
verification.**

## Root cause and mechanism

`AppShell.navigate` disables navigation controls while waiting for
`cancelAndJoin`. Chromium moves focus away from a focused button when that
button becomes disabled. The rejection branch previously rendered the
canonical route-error alert but did not restore the captured focus intent after
the control became enabled again.

The repair changes only:

- `desktop/src/app-shell/AppShell.tsx`
- `desktop/src/app-shell/AppShell.test.tsx`

The shell now retains one request-keyed failed focus intent. A layout effect
consumes it only after the matching request has rendered the error surface and
`transitioning` is false. It restores the original still-connected, visible,
enabled activator; otherwise it uses the existing content-heading fallback.
Starting a newer request clears the slot, request-sequence invalidation protects
stale settlement/unmount, and success/Back focus behavior is unchanged. No
timeout, global selector, coordinator change, dependency, manifest, or lockfile
change was introduced.

The focused regression uses real DOM `document.activeElement`: it transfers
focus to a focusable `BODY` while the Language button is disabled, rejects the
coordinator promise, then requires the enabled Language opener to own focus.
It dismisses the alert and retries the same action, requiring Settings and
`#/settings` on the second attempt. Existing keyboard, Back, stale request, and
App-level one-shot tests remain in the same focused run.

## Automated evidence

| Command | Exit/result | Evidence |
| --- | --- | --- |
| Initial focused test | 1; jsdom retained focus on a disabled button after `blur()`, unlike the observed WebView lifecycle | Test-harness diagnostic recorded here; the mistakenly relative generated log was removed from `desktop/.trellis` before continuing |
| `rtk npm test -- src/app-shell/AppShell.test.tsx src/App.test.tsx` | 0; 44/44 | `02-focused-appshell-app-repair.log` |
| `rtk npm run lint` | 0 | `03-lint.log` |
| `rtk npm run typecheck` | 0 | `04-typecheck.log` |
| `rtk just desktop-web-check` | 0; 96/96 plus Vite build | `05-desktop-web-check.log` |
| `rtk just desktop-test` | 0; 21 passed, 2 ignored fixtures | `06-desktop-test.log` |
| `rtk just desktop-build` | 0; release executable and NSIS bundle | `07-desktop-build.log` |
| `rtk git diff --check` | 0 | `08-git-diff-check.log` |
| task validation | 0; 7 implement and 3 check entries | `09-task-validate.log` |
| `rtk just ci` | 0; fmt/offline-update/check/workspace tests/clippy | `10-just-ci.log` |
| release seam and exact capability audit | 0; release seam search returned expected no-match 1; capability remained exactly listen/unlisten/destroy | `12-capability-seam-audit.log` |

Final rebuilt artifacts:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/devsweep-desktop.exe` | 9,827,328 | `6A54C2C752969045B247251A87FB6053271985511DEF1DC2D663088F6B530DC6` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,151,278 | `1BEF773BF50612D9A2E3B4B9E447F0EB5ED52D457652E2706BC67FE34346EF0B` |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `5677E0FE2DF24B6A454A2CEA378B38985EF6B0760DA12BC7D19E1384929C07D9` |

The debug executable hash is unchanged because this round changes only the
Vite-served TypeScript source; the direct executable plus current Vite source
is the same debug-native model used by the approved fault harness.

## Direct-CDP native evidence

The single authorized run used the unique real DevSweep WebView target at
`http://127.0.0.1:4180/`, loopback CDP port 9341, PID 41332/HWND `0xFA14D2`,
the fixed debug hash above, exact
`DEVSWEEP_TASK_NATIVE_FAULT=route_cancel_once`, and task-owned isolated
`LOCALAPPDATA` under `native-debug-direct-cdp/localappdata`.

| Clause | State | Direct fact |
| --- | --- | --- |
| Canonical route-error alert | PASS | Exact `Could not change destination. The current page remains active.` with `Dismiss` |
| Failed route does not commit | PASS | Hash stayed `#/clean`; Settings remained absent |
| Failed-route focus restoration | PASS | `document.activeElement` was `BUTTON` with exact text `Language` |
| Dismiss and one-shot recovery | PASS | Alert cleared; second Language action entered `#/settings` with Settings present |
| Window unhandled/error hooks | PASS | Both injected arrays remained empty through recovery |
| HWND-bound native captures | PASS | `debug-route-error-native.png`, 19,098 bytes, SHA-256 `7CFF3D34A332CA24F75FE6C3C8C8FA464FD7B600F05947CED8D2AB8B4B8B86B0`; `debug-settings-after-retry-native.png`, 13,528 bytes, SHA-256 `DFAB9CA148C53332C8277E3848DF7A934339EA7C6AC9DAFCF00E3880F9F992DA` |
| Closed offline CDP audit | PASS; original native command remains exit 1 | `15-offline-cdp-audit.log`: Runtime exception 0, console error 0, four window snapshots with unhandled/error 0, and the sole Log error is exactly the known Vite `http://127.0.0.1:4180/favicon.ico` 404 |
| Natural close and final residue | PASS | `CloseMainWindow=true`, main exited in 40 ms; after the script's final bounded follow-up main/WebView/CDP/store files were zero |

Complete native records are
`native-debug-direct-cdp/verification.json`, `cdp-messages.jsonl`, and the two
screenshots. `13-native-debug-direct-cdp.exit.txt` records command exit 1.
After graceful Vite Ctrl+C, the separate final audit found ports 4180/9341,
DevSweep main processes, DevSweep WebView processes, Vite processes, and
store/lock/temp files all zero.

## Final new-release native evidence

The prior C75D release evidence remains historical and is superseded for
final-source claims by the rebuilt 6A54 release. Three bounded 6A54 evidence
attempts are preserved:

1. Attempt 1 wrote the exact store but the helper incorrectly looked for an
   `aria-selected=true` primary tab while Settings was active, so it reported a
   selector failure. The app closed naturally with final residue zero.
2. Attempt 2 passed the Chinese/store clauses but sampled close residue at one
   fixed second and did not preserve the transient sample before a later final
   zero check. This was classified as an evidence-harness failure, not PASS.
3. The explicitly approved final attempt changed the external evidence method
   to a maximum-five-second, 250 ms sampled residue gate. Both closes reached
   main/WebView/CDP/lock/temp/Vite zero in the first sample and the full command
   exited 0.

Attempt 3 used task-owned isolated `LOCALAPPDATA`, loopback CDP 9355/9356, and
the exact release hash. Complete evidence is under
`native-release-direct-cdp-attempt3/verification.json`.

| Clause | State | Direct fact |
| --- | --- | --- |
| Initial English release | PASS | PID 59396, unique `http://tauri.localhost/#/clean` WebView, canonical English shell |
| English -> Chinese save | PASS | `data-locale=zh-CN`, canonical `清理`/`语言`, exact 39-byte `{"schema_version":1,"language":"zh-CN"}` |
| Store integrity | PASS | SHA-256 `2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`; lock/temp zero |
| First natural close | PASS | `CloseMainWindow=true`, main exit 43 ms; first 250 ms sample main/descendant/WebView/CDP/lock/temp/Vite all zero |
| No-flag restart | PASS | PID 85476 reopened `#/clean` with `data-locale=zh-CN` and unchanged exact store |
| Settings and real Back | PASS | Settings reached `#/settings`; real `history.back()` returned `#/clean`; active element was `BUTTON` text `语言` |
| Second natural close | PASS | `CloseMainWindow=true`, main exit 39 ms; first 250 ms sample and final audit all zero |
| Release runtime/console/log errors | PASS | Runtime exception, console error, Log error, window unhandled, and window error counts all zero |
| HWND-bound release captures | PASS | `release-first-zh-CN.png`, 14,104 bytes, SHA-256 `146437FC610C988C2DCF4056C1AD63E430FB76EA4666721312E94E55AA6BF3D3`; `release-restart-after-back.png`, 16,907 bytes, SHA-256 `B5004269C8E8C80E808FF0250AAC98DAAB20BD558285CD63517B30A4453DE97F` |
| Taskbar icon direct binding | **UNVERIFIED** | UIA returned only this instance's `ControlType.Window` and WebView `ControlType.Pane`; neither supports InvokePattern, so no taskbar control could be activated and reverse-bound foreground HWND -> PID -> path -> 6A54 hash |

Scaling, native High Contrast, and native Reduced Motion remain the user's
non-completion-required `WAIVED/UNVERIFIED` items. No process was terminated in
the successful final release attempt.
