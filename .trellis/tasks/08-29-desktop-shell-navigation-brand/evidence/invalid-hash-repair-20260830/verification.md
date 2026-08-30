# Invalid runtime hash repair verification

Date: 2026-08-30

Decision: **implementer verified; awaiting independent verification**. No
independent PASS is claimed. The runtime invalid-hash product defect is fixed
and all automated gates pass. Final-source release locale, persistence,
Back/focus, error, shutdown, and direct taskbar evidence passes.

Supersession: the later artifact-only rebind under
`../final-artifact-rebind-92c-20260830/` supersedes this report's
`DEC46...D604` release and `029FFD...E8C4` NSIS native claims. This report
continues to own the product repair, automated gates, and unchanged
`F5DFC7...65F5` debug evidence.

## Product repair

Changed product/test files:

- `desktop/src/app-shell/AppShell.tsx`
- `desktop/src/app-shell/AppShell.test.tsx`

Runtime `hashchange`/`popstate` parse failure now selects the first available
canonical route and submits it through the same request-keyed
`cancelAndJoin -> commit` lifecycle. The commit uses `replaceState`, including
the closed typed route state, so normalization adds no history entry. A
same-route fallback is not treated as a no-op: `focusCommitRequest` causes the
existing post-composition layout effect to restore the deterministic route
control. A cancellation failure leaves the old route rendered, replaces the
invalid URL with the old canonical route/state, displays canonical localized
route-error copy, and restores focus. Request sequencing prevents stale or
post-unmount settlement from committing.

Focused tests exercise real `window.history`, hashes, `document.activeElement`,
coordinator settlement order, replace-not-push history length, failure restore,
stale requests, and unmount. Initial invalid deep-link behavior and unavailable
route absence remain covered.

## Automated evidence

| Command | Exit/result | Log |
| --- | --- | --- |
| AppShell focused | 0; 15/15 | `01-focused-appshell.log` |
| AppShell + App | 0; 48/48 | `02-focused-appshell-app.log` |
| Desktop lint / typecheck | 0 / 0 | `03-lint.log`, `04-typecheck.log` |
| Core presentation settings | 0; 8 pass | `05-core-presentation-settings.log` |
| CLI i18n / TUI shell/app/runtime/render | 0; 10/4/31/11/24 pass | `06-cli-i18n.log` through `10-tui-render.log` |
| Desktop debug-fault Rust test | 0; 1 pass | `11-desktop-debug-native-fault.log` |
| Desktop focused aggregate | 0; 67/67 | `12-desktop-focused-aggregate.log` |
| `rtk just desktop-web-check` | 0; 100/100 plus build | `13-desktop-web-check.log` |
| `rtk just desktop-test` | 0; 21 pass, 2 ignored | `14-desktop-test.log` |
| `rtk just desktop-build` | 0; release + NSIS | `15-desktop-build.log` |
| `rtk git diff --check` / task validate | 0 / 0 | `16-git-diff-check.log`, `17-task-validate.log` |
| `rtk just ci` | 0 | `18-just-ci.log` |
| Debug cargo build | 0 | `19-debug-build.log` |
| Artifact/seam/capability audit | 0 | `20-artifact-seam-capability-audit.log` |

No manifest, lock, or dependency file changed. The accidental first log path
under `desktop/.trellis` was migrated into this task evidence. Only empty,
git-invisible directories remain because the execution policy rejected the
recursive directory removal.

## Final artifacts

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/devsweep-desktop.exe` | 9,827,328 | `DEC46FB5FC07338C84A16AAFF5A6412EFFC4EE9F5FCA6381C3ADDAEAAD54D604` |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,150,790 | `029FFD2BB8EF563ADE98E290B0C2D33DAC05D8D67F0967C9B83D5F457B9CE8C4` |

All hashes are exactly 64 hexadecimal characters. Release binary search found
no debug fault command or opt-in string. The capability remains exactly
`core:event:allow-listen`, `core:event:allow-unlisten`, and
`core:window:allow-destroy`.

## Debug native one-shot

The fixed debug hash was used with isolated task-owned `LOCALAPPDATA`, Vite on
loopback 4180, CDP on loopback 9361, and exact
`DEVSWEEP_TASK_NATIVE_FAULT=route_cancel_once`.

- First Language navigation displayed the exact canonical alert, retained
  `#/clean`, kept Settings absent, and restored `BUTTON/Language` focus.
- Dismiss cleared the alert; the second Language navigation entered
  `#/settings`, proving the one-shot cleared.
- Runtime exceptions, console errors, unexpected Log errors, window errors, and
  unhandled rejections were all zero. The only Log error was the known Vite
  `favicon.ico` 404.
- Main exited naturally in 46 ms. The old helper returned exit 2 because it
  classified an immediate five-WebView descendant snapshot as residue; its own
  final 750 ms sample shows main/WebView/CDP/store all zero. The reproducible
  offline audit in `22-debug-offline-audit.log` exits 0. The original exit 2 is
  retained in `21-native-debug-direct-cdp.log` and is not rewritten as command
  success.

Evidence: `native-debug-direct-cdp/verification.json`, CDP JSONL, and two
HWND-bound screenshots.

## Release native evidence

`23-native-release-direct-cdp.log` exits 0 against the final release hash.

- Initial English shell changed through the real native WebView to `zh-CN` and
  canonical Chinese copy.
- Store bytes are exactly
  `{"schema_version":1,"language":"zh-CN"}` (39 bytes), SHA-256
  `2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`;
  lock/temp residue is zero.
- First natural close completed in 43 ms; the first 250 ms residue sample was
  zero for main, descendants, owned WebView, CDP listener, lock/temp, and Vite.
- No-flag restart loaded `zh-CN`. Real `history.back()` returned from Settings
  to `#/clean` and restored `BUTTON/语言` focus. Store bytes stayed unchanged.
- Second natural close completed in 45 ms with the first 250 ms sample all zero.
- Runtime/console/Log/window/unhandled error collections were empty.

Evidence: `native-release-direct-cdp/verification.json` and the two final-hash
HWND screenshots.

## Taskbar direct binding

The helper enumerated the Explorer `Shell_TrayWnd` subtree, not the application
window tree. It recorded 32 descendants with name, automation id, control type,
rectangle, and supported patterns. The unique matching candidate was:

- Name: `devsweep - 1 running window`
- AutomationId: `Window: 0x19113e8`
- ControlType: `ControlType.Button`
- BoundingRectangle: `1354,1380,133,60`
- Supported patterns: `ScrollItemPatternIdentifiers.Pattern` only

The first combined evidence helper then encountered an unavailable
`LegacyIAccessiblePattern` type before it could fall through to clickable-point
`SendInput`. Consequently it did not produce the required activation ->
foreground HWND -> exact PID -> executable path -> final SHA-256 binding or the
same-timestamp desktop/taskbar captures. That failed attempt remains preserved
as diagnosis and is not treated as PASS.

The separately authorized, one-time taskbar-only run then directly passed:

- Fixed release PID 67692, HWND `0x6D412C2`, exact path and final
  `DEC46F...D604` hash.
- Explorer `Shell_TrayWnd` contained exactly one matching
  `ControlType.Button`, Name `devsweep - 1 running window`, AutomationId
  `Window: 0x6d412c2`, rectangle `1458,1380,133,60`; its only supported pattern
  was `ScrollItemPatternIdentifiers.Pattern`.
- `ShowWindow(SW_MINIMIZE)` produced `IsIconic=true` and a different foreground
  HWND. UIA had no clickable point, so the helper used the exact button-rectangle
  center `(1524.5,1410)` and one `SetCursorPos + SendInput` down/up pair.
- Within the first 100 ms sample, foreground became exact HWND `0x6D412C2`,
  `IsIconic=false`; `GetWindowThreadProcessId` returned PID 67692, whose process
  path and SHA-256 matched the fixed release.
- Same capture id `20260830-195426806` binds the full desktop, taskbar crop, and
  HWND PrintWindow reference. The taskbar crop visibly shows the original green
  DevSweep icon and `devsweep` label.
- Natural close succeeded. The final bounded residue sample records main,
  descendants, owned WebView, CDP listener, and lock/temp all zero; no forced
  termination occurred.

Evidence: `25-taskbar-only-sendinput.log`,
`native-taskbar-sendinput/verification.json`, and its `screenshots/` directory.
Taskbar icon appearance is **PASS by direct native binding**. Scaling, native
High Contrast, and native Reduced Motion remain the user-approved
non-completion-required `WAIVED/UNVERIFIED` items.
