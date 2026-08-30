# Native CDP close recapture verification

Date: 2026-08-30

Decision: **implementer recapture PASS; awaiting independent verification**.
This evidence-only round did not rebuild, did not change product/spec files, and
does not claim an independent `trellis-check` PASS.

Harness: PowerShell 7.6.5 at `C:\Program Files\PowerShell\7\pwsh.exe`.
Wrapper invoked the helper with no pipeline and persisted `outer-exit.json`
from the immediate `$LASTEXITCODE`. Transcript started before any DevSweep
launch. Frozen hashes were gated before launch and after the run.

## Command

```
"C:\Program Files\PowerShell\7\pwsh.exe" -NoProfile -ExecutionPolicy Bypass -File "D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-desktop-shell-navigation-brand\evidence\native-cdp-close-recapture-20260830\native-close-wrapper.ps1" -RepositoryRoot "D:\Documents\Code\Rust\Exp\devsweep" -RunRoot "D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-desktop-shell-navigation-brand\evidence\native-cdp-close-recapture-20260830" -PwshPath "C:\Program Files\PowerShell\7\pwsh.exe" -FirstPort 9401 -RestartPort 9402
```

| Record | Value |
| --- | --- |
| Wrapper PID | 18904 |
| Wrapper / helper exit | 0 / 0 |
| Helper transcript | `native-release/02-native-release-rebind.log` 8,020 bytes SHA-256 `CAE69BC936C590D534C78425DFC6DAE5CB6CB24F3CE28DA7750DB2810747A336` |
| CDP JSONL | 50 lines / 19,265 bytes SHA-256 `E455586A37FF428BCD984AC3A7B76E7E43FA6AE2B9B8E641E4E63676E87DF266` |
| `outer-exit.json` | helper_exit 0 |
| UIA raw | `native-release/shell-tray-wnd-uia-raw.json` SHA-256 `3E359FFF1046A95F76FD16F61B2B96114BCB8B8BCFB97F7B3296F0B7051E891C` |

Close order used for both runs: keep the CDP session through the last UI
action, `CloseMainWindow` while JS is alive, `WaitForExit`, then
`Target.detachFromTarget` if the socket is still open, dispose, then cheap
filtered owned-WebView poll starting at `CloseMainWindow`. `Page.close` /
`Browser.close` were not used.

## Frozen hashes before and after

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/devsweep-desktop.exe` | 9,827,328 | `92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20` |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,152,372 | `0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328` |

Before-launch and after-run gates matched. No drift.

## Native facts

First run PID 53684, HWND `0x30E1458`, port 9401, isolated `LOCALAPPDATA`.
English shell saved `zh-CN`. Exact V1 bytes
`{"schema_version":1,"language":"zh-CN"}` (39 bytes, SHA-256
`2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`), lock/temp 0.
`CloseMainWindow` with socket still Open; main `WaitForExit` 41 ms; first
residue sample at 818 ms: main 0, owned WebView 0, listener 0, lock/temp 0,
query 718.8 ms. Identity-gated termination: none.

No-flag restart PID 47692, HWND `0x1C7A0904`, port 9402, locale `zh-CN`,
`#/clean`. Settings Back restored `BUTTON` / `语言`. Taskbar:
Explorer `Shell_TrayWnd` `0x10162`, unique button bound to HWND `0x1C7A0904`
PID 47692 path/hash match, capture id `20260830-214341528`. Restart close:
main `WaitForExit` 63 ms; first residue sample at 798 ms: main 0, owned 0,
listener 0, lock/temp 0, query 720.7 ms. Identity-gated termination: none.

Two-run window unhandled/error arrays and CDP runtime/console/log errors were
empty. Final audit: main 0, owned WebView 0, ports 9401/9402 0, lock/temp 0.

## Screenshots

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `release-first-zh-CN.png` | 14,104 | `146437FC610C988C2DCF4056C1AD63E430FB76EA4666721312E94E55AA6BF3D3` |
| `release-restart-after-back.png` | 16,907 | `B5004269C8E8C80E808FF0250AAC98DAAB20BD558285CD63517B30A4453DE97F` |
| `release-taskbar-bound-desktop-20260830-214341528.png` | 1,890,694 | `19A5514DFEEEE67A4CCFDD1252EEDE0900CCBC6307E18A0FDB827799CDFD9DB8` |
| `release-taskbar-bound-crop-20260830-214341528.png` | 68,389 | `BA1244E6CA4715801625E1E0A406F63C70EDDA47DC3296A928512C7374AA2978` |
| `release-taskbar-bound-app-reference-20260830-214341528.png` | 31,224 | `231E5E776177CA78FB9DF56A4A043D9D497E9C930FD42CDBC7715B05CF0A4908` |

## Remaining UNVERIFIED

Scaling 100/125/150/200% and native High Contrast / Reduced Motion remain
`WAIVED/UNVERIFIED` per the user waiver. This report does not claim independent
PASS.
