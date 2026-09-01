# WebView shutdown diagnosis — not run4

Date: 2026-08-30

Frozen artifacts unchanged:

| Artifact | SHA-256 |
| --- | --- |
| `target/release/devsweep-desktop.exe` | `92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20` |
| `target/debug/devsweep-desktop.exe` | `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | `0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328` |

## Cause

Run3 failed the five-second owned-WebView gate after a correct product close. It is an evidence-harness defect, not a product lifecycle defect.

1. Product close without CDP is clean. Arm `02-no-cdp-close.ps1` launched the frozen release with isolated `LOCALAPPDATA` and no `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`. `CloseMainWindow` succeeded; main exited in 72 ms; owned `webview-exe-name=devsweep-desktop.exe` count was 0 at 233 ms.
2. `--remote-debugging-port` without a CDP WebSocket is also clean. Arm `03-port-only-close.ps1` used port 9397 and `/json/list` only. Main exited in 58 ms; owned count was 0 at 2942 ms (first identity sample, CIM-bound).
3. Run3 held an open `ClientWebSocket` (`Runtime.enable` / `Page.enable` / `Log.enable`), then `CloseAsync(NormalClosure)` + `Dispose`, then `CloseMainWindow`. Main exited in 115 ms, but browser + crashpad + renderer with `--remote-debugging-port=9391` remained. Helper final audit ~20 s later still had browser PID 33352. A later read-only audit was zero. No kill.
4. Gate overhead: `Close-ReleaseRun` snapshots descendants with unfiltered `Get-CimInstance Win32_Process`, then polls that full table every 250 ms for 5 s. Idle timing: full CIM 857 ms / 742 processes; filtered WebView CIM 687 ms; `Get-Process` 25 ms. Run3 recorded only two residue samples 4.2 s apart. The five-second budget is consumed by WMI, not by waiting for teardown.

Owned WebView identity in run3 was real (`webview-exe-name=devsweep-desktop.exe`, parent 68028). The hold is the attached CDP session, not a DevSweep Job-Object leak. This host currently has ~25 unrelated `msedgewebview2.exe` processes; cheap `Get-Process` counts are not ownership.

## Required fix

Do not rebuild. Do not repeat the run3 helper. Do not change product/lifecycle/capability code.

Change the evidence mechanism:

1. Before `CloseMainWindow`, send CDP `Browser.close` or `Target.closeTarget` for the unique native page, wait for socket close, then dispose. Do not leave a live debugger session across natural close.
2. Poll owned WebViews with a name filter plus `webview-exe-name=devsweep-desktop.exe` / parent-pid identity. Record per-sample query milliseconds. Start the residue clock at `CloseMainWindow`.
3. Recapture the remaining native clauses against the frozen 92C3919D… release: no-flag restart, Settings Back/focus, two-run error arrays, Explorer taskbar binding, graceful close with owned WebView = 0, lock/temp/listener = 0.
4. Re-verify the three frozen hashes before and after. `just desktop-build` is forbidden unless a product file changes.

## Logs

- `01-cim-timing.ps1` stdout in session
- `02-no-cdp-close.log` / `02-no-cdp-close.json`
- `03-port-only-close.log` / `03-port-only-close.json`
