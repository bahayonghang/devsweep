# Final artifact native rebind verification

Date: 2026-08-30

Decision: **REJECTED / SUPERSEDED evidence record**. The later evidence audit
found that the `02-native-release-rebind.log` claimed below does not exist.
Those native assertions may not be used as final-source command evidence and
must not be reconstructed from `verification.json`. The record is preserved
unchanged below as historical diagnosis; the authentic run2 record lives under
`../final-artifact-rebind-92c-run2-20260830/` and failed before completing native
acceptance.

## Frozen artifacts

The following values were independently recomputed before launch and after both
native runs. They are recomputed once more in the final report audit.

| Artifact | Bytes | SHA-256 | Execution boundary |
| --- | ---: | --- | --- |
| `target/release/devsweep-desktop.exe` | 9,827,328 | `92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20` | Executed only for this rebind |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5` | Not executed; prior native fault evidence remains bound |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,152,372 | `0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328` | Not installed or executed |

Evidence: `01-preflight-hashes.log`, `03-post-run-hashes.log`.

## Release locale, persistence, navigation, and close

Historical claim only (rejected because the cited raw log is missing):
`02-native-release-rebind.log` exits 0. Both runs used the same frozen release,
a new isolated task-owned `LOCALAPPDATA`, and unique loopback CDP ports 9371 and
9372.

- First run PID 77024 selected the unique real
  `http://tauri.localhost/#/clean` WebView target and began in English.
- The real Language settings control changed to `zh-CN`; the shell rendered
  canonical Chinese `清理`/`语言` copy.
- The presentation store is exactly
  `{"schema_version":1,"language":"zh-CN"}`: 39 bytes, SHA-256
  `2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`.
  Lock/temp residue was zero.
- First `CloseMainWindow` completed in 44 ms; the first 250 ms external residue
  sample was zero for main, descendants, owned WebView, listener, lock/temp,
  and Vite.
- No-flag restart PID 40720 began in `zh-CN`. Settings followed by real
  `history.back()` restored `#/clean` and active `BUTTON/语言`; store bytes stayed
  exact and unchanged.
- Runtime exceptions, console errors, Log errors, window errors, and unhandled
  rejections were zero.
- Second natural close completed in 57 ms; the first 250 ms residue sample and
  final main/WebView/listener/lock/temp audits were all zero. No forced
  termination occurred.

## Explorer taskbar direct binding

The second run performed the taskbar evidence before natural close:

- `FindWindow("Shell_TrayWnd")` returned `0x10162`. Its UIA subtree contained
  exactly one `ControlType.Button` named `devsweep - 1 running window`.
- AutomationId `Window: 0x1971166`; rectangle `1498,1380,133,60`; supported
  patterns contained only `ScrollItemPatternIdentifiers.Pattern`.
- `ShowWindow(SW_MINIMIZE)` produced `IsIconic=true` and foreground HWND
  `0x900F7E`, not the application.
- UIA exposed no clickable point. The helper used the exact rectangle center
  `(1564.5,1410)` and one `SetCursorPos + SendInput` left-button down/up pair at
  `2026-08-30T20:20:16.0934505+08:00`.
- The first activation sample showed foreground HWND `0x1971166`, PID 40720,
  and `IsIconic=false`. `GetWindowThreadProcessId` mapped that HWND to the same
  launch PID; its exact path and SHA-256 matched the frozen release.
- Capture id `20260830-202016251` binds the full desktop, Explorer taskbar crop,
  and HWND PrintWindow reference. Visual inspection confirms the original green
  DevSweep taskbar icon and `devsweep` label.

| Capture | Bytes | SHA-256 |
| --- | ---: | --- |
| Full desktop | 7,842,178 | `B8DB65E5992921A56390DFE5DD8ED822CF8648B18D8711A3E3A2210BA32579E1` |
| Taskbar crop | 69,529 | `F3ACD40C36D98E07827AEE7B2DD5B9782806D05CED5699C7BF59BCB1ECC4C86E` |
| HWND reference | 31,474 | `0AE77938F6EE96CBF454B9720984E69B86A535DE9F4718F42B75B732E9EAD1B6` |

Full machine-readable evidence is in `native-release/verification.json`.

## Supersession and retained boundaries

The prior `DEC46...D604` release/`029FFD...E8C4` NSIS evidence remains preserved
as historical diagnosis but is superseded for final-source claims by this
`92C391...B4E20` release rebind. The unchanged `F5DFC7...65F5` debug binary and
its existing one-shot evidence remain current; its original helper exit 2 and
the successful narrow offline audit remain reported exactly as recorded.

Native scaling, High Contrast, and Reduced Motion remain the user-approved
non-completion-required `WAIVED/UNVERIFIED` items. No independent PASS is
claimed.
