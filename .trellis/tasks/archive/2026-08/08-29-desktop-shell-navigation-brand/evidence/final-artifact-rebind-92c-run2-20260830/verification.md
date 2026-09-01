# Final artifact native rebind run2 verification

Date: 2026-08-30

Decision: **REJECTED / incomplete native evidence record; awaiting a new user
decision**. This was the single authorized run2. It failed during the first CDP
handshake and was not retried. No product, test, specification, manifest, lock,
dependency, or frozen artifact was changed.

## Frozen artifact preflight

All three frozen artifacts matched before launch and after the failed run:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/devsweep-desktop.exe` | 9,827,328 | `92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20` |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,152,372 | `0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328` |

The exact wrapper invocation is preserved in `00-wrapper-command.txt`.
`run2-wrapper.ps1` is 7,099 bytes with SHA-256
`6A6636434BEC51DDA511C0BFD875621D92E47B7BBE8AFD279096871B85B17F56`;
`release-rebind-run2.ps1` is 37,461 bytes with SHA-256
`50DB70FD6F9596F2CFAAAA6F7CE462CBEE2F696D89ECEADDAC6C88A60EE38F16`.

## Authentic transcript and failure

The transcript capture chain existed before the release launch. The raw
`02-native-release-rebind.log` is 45 lines / 4,558 bytes with SHA-256
`FAE0E9A8F4EBFAEB1F3BF057F14EBEBB40662A56BD2D75E0999948A73AA2C2C1`.
It contains the PowerShell host header, exact wrapper/helper commands, wrapper
and helper hashes, all preflight artifact values, process/listener preflight,
helper checkpoints, postflight artifacts/residue, and the failure record. It
was produced by `Start-Transcript`; it was not reconstructed from
`verification.json`.

The child helper exited 1 and the tool-observed outer wrapper process exited 1.
The transcript's internal `EXIT_CODE=99` is itself an evidence-helper defect:
the wrapper threw while `$wrapperExit` still held its initialization sentinel.
It must not be read as the real OS exit code and was not rewritten.

The first `Runtime.enable` request was sent, after which Windows PowerShell 5.1
rejected the inherited `ConvertFrom-Json -Depth 100` call because that parameter
is unavailable in 5.1. The exact failure, path, line, and stack are preserved in
`native-release/verification.json`. This is an evidence-harness compatibility
failure, not a DevSweep product assertion.

The independent raw `native-release/cdp-messages.jsonl` therefore contains only
one real send record, 1 line / 156 bytes, SHA-256
`BA694D1119970AB4DCE3628B9AA07C75604BD8BED74758E27EA01A370DB4B22C`.
There was no successful receive, locale action, second launch, Back/focus action,
UIA enumeration, taskbar activation, or screenshot. No such result is inferred
from the earlier rejected run.

## Graceful residual close

Because `Start-ReleaseRun` failed before returning its run object, the helper
could not invoke its normal close path. A separate cleanup-only script performed
a fresh read-only identity gate for PID 16920: exact release path/hash, start
time `2026-08-30T20:32:40.8018019+08:00`, non-zero HWND, title `devsweep`, and
matching command line. It called `CloseMainWindow` once; the request returned
true and the first recorded post-close sample showed main 0, owned WebView 0,
and listener 0. No forced termination occurred. Evidence:
`05-residual-graceful-close.log` (26 lines / 1,429 bytes, SHA-256
`7B78714431C9A9B1D0A321954634E8DD744C420B1D1D4948EFAD9EBFB867D0F6`).

Final read-only residue is main 0, owned WebView 0, ports 9381/9382 listeners 0,
and task-owned settings lock/temp 0.

## Evidence outcome

| Clause | State | Reason |
| --- | --- | --- |
| Authentic run2 command transcript | PASS for existence/authenticity only | Raw 02 exists and is hashed; the run itself failed |
| English to Chinese, exact V1 store | **UNVERIFIED** | Failure occurred before the first CDP response |
| No-flag Chinese restart and Back/opener focus | **UNVERIFIED** | Restart was never launched |
| Runtime/console/window error audit | **UNVERIFIED** | CDP domains were not enabled successfully |
| Explorer taskbar direct binding and captures | **UNVERIFIED** | UIA stage was never reached; raw UIA JSON and images do not exist |
| Natural close / final zero residue | PASS for cleanup-only close | Exact task-owned PID closed naturally; this does not rescue the failed native evidence run |
| Scaling / High Contrast / Reduced Motion | **WAIVED/UNVERIFIED** | Unchanged user waiver |

The previous `final-artifact-rebind-92c-20260830` record is rejected/superseded
because its claimed `02-native-release-rebind.log` does not exist. This run2 is
the only final-source command record, and it is incomplete. The task cannot claim
completion-required final-release native evidence or independent PASS from it.
