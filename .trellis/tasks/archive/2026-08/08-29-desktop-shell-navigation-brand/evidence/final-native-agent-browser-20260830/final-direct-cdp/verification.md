# Final direct-CDP debug evidence

Date: 2026-08-30

Decision: **FAIL at the first native route-fault focus assertion; stopped
without retry or product repair.** The canonical alert and unchanged route were
directly observed, but focus moved to `BODY` instead of remaining on the
Language opener. Dismiss/retry and the separate release taskbar run were not
attempted after this first evidence failure.

## Identity and target

| Fact | Direct value |
| --- | --- |
| Debug executable | `target/debug/devsweep-desktop.exe` |
| Bytes | 14,758,912 |
| SHA-256 before/after launch/final | `5677E0FE2DF24B6A454A2CEA378B38985EF6B0760DA12BC7D19E1384929C07D9` |
| Main PID / HWND | 44844 / `0x2210C1C` |
| Fault env | exact `DEVSWEEP_TASK_NATIVE_FAULT=route_cancel_once` |
| Store root | task-owned `final-direct-cdp/localappdata` (initial/final empty) |
| CDP | loopback `127.0.0.1:9341` |
| CDP owner | PID 10840, direct WebView child of PID 44844 |
| Unique target id | `3BA717010E836E9BE432412D9081ADE5` |
| Target title / URL | `devsweep` / `http://127.0.0.1:4180/` |

No Chromium or `about:blank` instance was created. The run selected the unique
target from `http://127.0.0.1:9341/json/list` and used only .NET
`ClientWebSocket` CDP `Runtime`, `Page`, and `Log` commands.

## First route-fault observation

The initial native state was `#/clean`, locale `en`, no alert, no Settings
select, and exactly one Language button. CDP then focused and clicked that real
button.

| Clause | State | Direct value |
| --- | --- | --- |
| One-shot fault invoked | PASS | Runtime evaluation returned `clicked=true`, active text `Language` at dispatch |
| Canonical localized alert | PASS | `Could not change destination. The current page remains active.` with `Dismiss` action |
| Route/hash unchanged | PASS | `#/clean` |
| Failed route not installed | PASS | no Settings select rendered |
| Window unhandled/error hooks | PASS at this boundary | both arrays empty |
| Focus intent not incorrectly changed | **FAIL** | post-fault `document.activeElement` was `BODY`, whose text was the whole shell, rather than the Language opener |
| Dismiss clears alert | **UNVERIFIED** | not run after the first failed assertion |
| Second click reaches Settings / one-shot clears | **UNVERIFIED** | not run after the first failed assertion |
| CDP exception/console audit through recovery | **UNVERIFIED** | recovery boundary was not reached; the recorded Vite favicon 404 is unrelated network log noise |
| HWND-bound alert screenshot | **UNVERIFIED** | assertion failed before capture; no screenshot was created |

The failure is recorded at `debug-direct-cdp.ps1:341` in
`verification.json`. Exact request/response traffic is preserved in
`cdp-messages.jsonl`; request 6 performs the Language action and response 7
contains the canonical alert, unchanged route, absent Settings control, empty
error arrays, and `activeTag: BODY`.

## Shutdown and residue

- `CloseMainWindow=true`; main PID exited after 32 ms.
- Five WebView descendants were still observable at the immediate main-exit
  sample, then all were gone at the script's final 750 ms sample.
- Final DevSweep main/WebView processes: 0.
- Final CDP 9341 and Vite 4180 listeners: 0.
- Final task Vite/node processes: 0.
- Final agent-browser processes/listeners: 0; agent-browser was not used.
- Final task-owned debug store/lock/temp files: 0.
- No forced termination was required.

The Vite PTY was ended with Ctrl+C after the only app run; its signal exit was
1 and is not an application-gate failure. `final-residue.json` is the final
zero-residue audit.

## Command outcomes

| Command/action | Exit/result |
| --- | --- |
| PowerShell parser check for `debug-direct-cdp.ps1` | 0; no parse errors |
| Vite `npm.cmd run dev` | ready on 127.0.0.1:4180; later Ctrl+C exit 1 |
| `pwsh -NoProfile -File debug-direct-cdp.ps1 ... -CdpPort 9341` | 1 at native focus assertion |
| Final read-only process/port/store/hash audit | 0; application residue zero |

This is a directly observed final-source native defect, not an automation
substitute and not an inferred failure. Evidence-only authorization forbids a
product repair in this round. Overall task state remains awaiting independent
verification and not ready for archive.
