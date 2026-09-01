# Final artifact native rebind run3 verification

Date: 2026-08-30

Decision: **FAILED / final evidence attempt exhausted**. This was the third and
last authorized evidence-only attempt. It exited 1 at the first-run bounded
WebView residue gate and was not retried. No product, test, specification,
manifest, lock, dependency, or frozen artifact was modified.

## PowerShell 7 and prelaunch gates

The absolute executable was
`C:\Program Files\PowerShell\7\pwsh.exe`, version 7.6.5. The standalone
preflight and actual helper both recorded their real PID and command line.
Before any DevSweep launch, both verified:

- nested representative CDP JSON parsed and round-tripped with
  `ConvertFrom-Json -Depth 100`;
- `ClientWebSocket`, `Start-Transcript`, UIAutomation, and the SendInput /
  SetCursorPos P/Invoke declarations loaded;
- preflight/helper/wrapper PowerShell static parse had zero errors;
- DevSweep, owned WebView, and ports 9391/9392 were zero;
- the three frozen artifacts matched exactly.

Evidence: `01-toolchain-preflight.log`, 36 lines / 3,065 bytes, SHA-256
`9828DDAD92C7FE73C339A660A0EC0A727D8D03CE9B919647CAD80A385CACBF85`.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/devsweep-desktop.exe` | 9,827,328 | `92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20` |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,152,372 | `0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328` |

## Authentic command evidence

The helper started `native-release/02-native-release-rebind.log` before launch
and wrote the PRELAUNCH marker, exact PowerShell path/version/command line,
script hashes, and artifact hashes. It is 33 lines / 4,055 bytes, SHA-256
`C8C85959C9EC81093C35E030B407F7B56BC8FF0E1C4B4DD0221714A7C1665111`.
It ends with `HELPER_EXIT_CODE=1` before `Stop-Transcript`.

The wrapper invoked the helper directly without a pipeline, read the immediate
`$LASTEXITCODE`, wrote `outer-exit.json`, and exited with the same code. The
tool-observed wrapper exit, `outer-exit.helper_exit`, and helper transcript all
equal 1. `outer-exit.json` is 47 lines / 2,846 bytes, SHA-256
`09C8136091FA6E5A5C53884036BEE1D8E371C0F8799014C4DA85AEFE4B0D4787`.

Every actual CDP send, receive, and received event was appended in order to
`native-release/cdp-messages.jsonl`: 22 lines / 8,936 bytes, SHA-256
`D3D31C42354C66EFF322ABCF9D04CF7681B7827912330E4DF9015D7282200D78`.

## Native facts and failure

The first release run used the isolated task-owned `LOCALAPPDATA` and unique
loopback port 9391. It began in English, opened real Language settings, selected
`zh-CN`, rendered canonical Chinese, and wrote exact bytes
`{"schema_version":1,"language":"zh-CN"}`: 39 bytes, SHA-256
`2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`,
with lock/temp residue zero. The HWND-bound Chinese capture is 14,104 bytes,
SHA-256 `146437FC610C988C2DCF4056C1AD63E430FB76EA4666721312E94E55AA6BF3D3`.

PID 68028 accepted `CloseMainWindow`; the main process exited in 115 ms.
However, the external five-second gate still observed three owned WebView
descendants at its final sample. The helper therefore correctly raised
`first release close left residue`, exited 1, and did not start the no-flag
restart or the UIA/taskbar stage. One owned WebView was still present in the
helper's one-second final audit. A later read-only audit found main 0, owned
WebView 0, ports 9391/9392 zero, and lock/temp zero. No forced termination
occurred. Eventual cleanup does not convert the bounded close assertion to PASS.

No `shell-tray-wnd-uia-raw.json`, restart screenshot, taskbar captures, or
restart evidence exists. The helper's absence records are preserved in
`outer-exit.json`; none is inferred from run1/run2.

| Clause | State |
| --- | --- |
| Authentic transcript/CDP/outer exit chain | PASS |
| English to Chinese and exact V1 store | PASS for first run |
| First bounded natural-close residue gate | **FAIL** |
| No-flag restart and Back/opener focus | **UNVERIFIED** |
| Full error arrays across both runs | **UNVERIFIED** |
| Explorer taskbar direct binding / three captures | **UNVERIFIED** |
| Final eventual process/listener/lock/temp residue | PASS read-only, after the failed bounded gate |
| Scaling / High Contrast / Reduced Motion | **WAIVED/UNVERIFIED** |

Run1 remains rejected for its missing raw `02`; run2 remains rejected for its
PowerShell 5.1 JSON-parser failure. This run3 is the only final-source command
record, and it fails a completion-required native close gate. The task must stop
without run4.
