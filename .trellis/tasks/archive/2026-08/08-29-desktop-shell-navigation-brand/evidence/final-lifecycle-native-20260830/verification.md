# Final lifecycle and native evidence

> Historical report: the later focus-repair build and native result are recorded
> at `../focus-failure-repair-20260830/verification.md`. Its rebuilt release and
> NSIS hashes supersede the hashes below for final-source claims.

Date: 2026-08-30

Decision: **implementer automated gates pass; release locale/restart/Back
evidence now passes, but final direct native evidence exposes a debug
route-fault focus failure; awaiting independent verification and not ready for
archive**. The
bounded third product-evidence round remains preserved below. A later
user-authorized evidence-only agent-browser round is recorded at
`../final-native-agent-browser-20260830/verification.md`.

## Fixed artifacts

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/devsweep-desktop.exe` | 9,826,816 | `C75D08506B97984635EB3799E8BD3D9D5732A5D40A328BBA4B3FEA2D6FD66742` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,150,041 | `662FD425FF9B3701BA974769EF7C403FC385237F69DAB0E8F568EEC1B4347CAF` |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `E16F09D393A81291D7D05F1D655648B16592A456786EAA431E0478CF5B727091` |

The production web bundle and release executable contain none of
`debug_native_fault_mode`, `DEVSWEEP_TASK_NATIVE_FAULT`, or
`route_cancel_once`.

## ACL lifecycle correction

The installed Tauri `onCloseRequested` implementation awaits the registered
frontend handler and then calls `window.destroy` when the event was not
prevented. The generated desktop ACL maps exact permission
`core:window:allow-destroy` to the single command `destroy`. The final
capability contains exactly:

1. `core:event:allow-listen`
2. `core:event:allow-unlisten`
3. `core:window:allow-destroy`

No other `core:window:*` authority was added. The frontend does not call
destroy itself or prevent the native close; its awaited handler returns one
shared coordinator cancel/join drain. The Rust static permission test and
generated-schema audit both pass.

## Native evidence matrix

| Final-source clause | State | Direct evidence/boundary |
| --- | --- | --- |
| Release launch, English Clean shell, native title bar, title/header icon | PASS | `release-attempt3.log`; `release-attempt3/screenshots/01` through `07`; fixed release hash above |
| Real keyboard focus: primary Clean, Language opener, Settings activation | PASS | `05-release-first-tab-focus.png`, `06-release-language-focus.png`, `07-release-settings-en.png` visibly show the focus sequence and Settings route |
| Diagnostic 390/800/1024/1440 window requests at current DPI 120 | PASS as diagnostic only | `01` through `04` record requested CSS width, DPI, physical size, image bytes, and hashes; they do not prove waived exact native dimensions/scaling |
| English to Chinese save and exact 39-byte V1 document | PASS | User-authorized evidence-only round: agent-browser real native WebView select, `data-locale=zh-CN`, canonical Chinese shell, exact 39-byte document with SHA-256 `2CD9EEDF...D6B43D2899`; `../final-native-agent-browser-20260830/verification.md` |
| No-flag Chinese restart and byte-stable store | PASS | Same isolated `LOCALAPPDATA`, fixed C75D release, no-flag restart at `#/clean` with `data-locale=zh-CN`; document hash unchanged |
| Settings Back and opener-focus restoration after Chinese save | PASS | Real CDP History Back restored `#/clean`; `document.activeElement` was the Chinese Language opener; HWND-bound native screenshot and DOM snapshot saved |
| Natural last-window close and process/runtime cleanup | PASS | Original ACL close proof plus both evidence-only release sessions: main and all DevSweep WebView children gone, CDP/listeners/session/store lock/temp residue zero |
| Debug-only canonical route-error and stable route | PASS | Final .NET direct-CDP run on unique real WebView target: canonical English alert, `#/clean` unchanged, Settings absent, empty window unhandled/error arrays |
| Debug fault focus intent | **FAIL, completion-required** | Post-fault `document.activeElement` was `BODY`, not the Language opener; direct evidence under `../final-native-agent-browser-20260830/final-direct-cdp/` |
| Debug dismiss/retry, one-shot recovery, complete console audit, native alert screenshot | **UNVERIFIED, completion-required** | The single run stopped immediately at the first focus failure; no retry/product repair was permitted |
| Final-hash taskbar icon | **UNVERIFIED** | A screen taskbar button titled `devsweep` could not be uniquely bound to the Tauri HWND because another host/repository window shared the title. The executable's associated icon is correct, but that is not direct taskbar evidence |
| 100/125/150/200% native scaling and exact dimensions | **WAIVED/UNVERIFIED** | User waiver; system scaling was never changed |
| Native High Contrast appearance | **WAIVED/UNVERIFIED** | User waiver; forced-colors CSS/test remains automated evidence only |
| Native Reduced Motion appearance | **WAIVED/UNVERIFIED** | User waiver; reduced-motion CSS/test remains automated evidence only |

## Attempt boundary

- Attempt 1 used release `2BBFBBD5...F41C418`. Locale navigation and an exact
  Chinese V1 document were captured, but native close left PID 50416 alive.
  That pre-repair run is superseded for final-source PASS claims.
- Attempt 2 used release `E30AC97F...F41C418`. Foreground/DPI automation did
  not complete the locale action, and native close still left PID 85444 alive.
  Static/runtime diagnosis then proved the missing `allow-destroy` ACL.
- Attempt 3 used the fixed release `C75D0850...FD66742`. It captured the final
  English shell, window diagnostics, keyboard focus, and Settings route. The
  native select action did not create the store, so the script stopped. A
  cleanup-only, identity-gated `CloseMainWindow` then proved the repaired
  natural exit and zero process/child/listener/lock/temp residue.
- A later user-authorized evidence-only round used named agent-browser sessions
  and loopback-only CDP ports against the same C75D release. It directly closed
  locale save, exact V1 bytes, no-flag restart, real History Back, opener-focus,
  and two natural-close residue checks. A subsequent user-authorized fixed
  5677 debug run changed diagnosis to direct .NET CDP and reached the real
  fault: canonical alert and unchanged route passed, but focus moved to BODY.
  The run stopped before dismiss/retry. Debug/Vite/CDP/agent-browser residue is
  zero. See
  `../final-native-agent-browser-20260830/verification.md`.

Identity-gated forced termination was used only for previously failed residual
PIDs 50416 and 85444 under the user's explicit/default authorization. PID
59044 exited naturally; it was not terminated. There is no final DevSweep
process or listener residue.

## Automated gates

| Gate | Result | Log |
| --- | --- | --- |
| Rust capability and debug-fault focused tests | PASS | `../logs/final-lifecycle-acl-focused-round1.log` |
| Lifecycle/App/AppShell/coordinator focused tests | PASS, 54/54 | `../logs/final-lifecycle-acl-ts-focused.log` |
| Desktop lint/typecheck | PASS | `../logs/final-lifecycle-acl-lint-typecheck.log` |
| `rtk just desktop-web-check` | PASS, 96/96 and Vite build | `../logs/final-lifecycle-acl-desktop-web-check.log` |
| `rtk just desktop-build` | PASS | `../logs/final-lifecycle-acl-desktop-build.log` |
| Capability/schema/release-seam audit | PASS on corrected audit | `../logs/final-lifecycle-acl-schema-seam-release-hashes-round2.log`; the first log preserves a PowerShell `SequenceEqual` invocation error |
| `git diff --check` and task validation | PASS | `../logs/final-lifecycle-acl-diff-validate.log` |
| `rtk just ci` | PASS on round 2 | `../logs/final-lifecycle-acl-just-ci-round2.log`; round 1 preserves the rustfmt-only failure |

Automated tests and the new release evidence do not substitute for the missing
debug route focus and recovery clauses. The final fixed-hash direct-CDP run
exposed a real native focus failure and stopped before dismiss/retry, so the
task remains incomplete.
