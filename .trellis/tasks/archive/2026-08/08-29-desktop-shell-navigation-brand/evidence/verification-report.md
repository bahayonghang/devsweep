# Verification report - desktop shell, navigation, and brand

Date: 2026-08-30

Decision: **implementer native recapture PASS; awaiting independent
verification**. This report records implementer-produced evidence only; it does
not claim an independent `trellis-check` PASS. The invalid-runtime-hash repair
automated gates remain green and the unchanged-debug evidence remains preserved.
Run1 is rejected for its missing raw `02`; run2 is rejected for the PowerShell
5.1 parser failure; run3 failed the first-close owned-WebView gate. A new
evidence-only harness under
`evidence/native-cdp-close-recapture-20260830/` recaptured the remaining native
clauses against frozen `92C3919D…` without rebuilding: exact 39-byte V1 store,
no-flag zh restart, Settings Back/opener focus, empty two-run error arrays,
Explorer `Shell_TrayWnd` HWND-bound taskbar with three captures, and graceful
close with main/owned WebView/listener/lock/temp = 0. User-waived scaling,
native High Contrast, and native Reduced Motion observations remain explicitly
`WAIVED/UNVERIFIED` and are not represented as PASS.

## Requirements and acceptance trace

| Contract | Result | Implementation and evidence |
| --- | --- | --- |
| R1 / AC1 | Implementer verified; independent pending | `AppShell` registers the closed five-mode identity, renders only available modes/supporting destinations, normalizes both initial and runtime-invalid hashes, uses replace-not-push typed history for runtime fallback, restores actual Back/popstate routes, cancels and joins before activation, rejects stale/post-unmount route completions, and restores the real activating navigation control or deterministic deep-link fallback. Focused tests cover successful runtime normalization, cancel failure restoring the old canonical URL/focus/error, history length, registered/absent support routes, Settings opener/Back, keyboard activation, and stale/unmount behavior. TUI exposes only the registered Clean mode and a real language-settings action. |
| R2 | Implementer verified; independent pending | The shell uses the generated stable master icon, native Tauri chrome, the approved gray-green token system, `Segoe UI Variable` with `Segoe UI`/system fallback, restrained CSS-native structure, and no glass/glow/traffic-light/planet artwork. Desktop style and shell tests cover these contracts. |
| R3 / AC4 | Automated implementation and native recapture verified; independent pending | Core exclusively owns the closed `PresentationLanguageTag`, exact V1 document/path, full-transaction cross-process lock, same-directory atomic replacement, and fail-closed byte preservation. Tauri and TUI map tags exhaustively without a core-to-CLI dependency. Desktop has explicit loading/ready/unavailable states and closed save semantics. Recapture wrote exact 39-byte Chinese V1 bytes with lock/temp zero and consumed them on a no-flag restart. |
| R4 / AC2 | Implementer verified after static-audit correction; independent pending | Desktop `OperationCoordinator.start` first cancels/joins the old owner, then installs the new active lease, and only then invokes the service closure. Closed/refused and cancel-failure paths never invoke it; normalized result settlement and exactly-once completion prevent unhandled/double completion. Six coordinator tests cover call order, rapid serialization, cancellation failure, start rejection, stale completion, and close. TUI replacement Scan is queued, cancels active work, waits for a real worker terminal and `JoinHandle::join`, then starts exactly one replacement; stale progress/results remain rejected. |
| R5 | Automated/debug implementation verified; final-release Back/taskbar recapture verified; independent pending | Desktop/TUI tests cover deterministic routing including runtime-invalid hash normalization, keyboard navigation, activating-control focus restoration, canonical localized route errors, collision-free available accelerators, canonical IEC units, full accessible user data, and non-truncation of authority/action copy. The unchanged-debug direct-CDP evidence remains preserved. Recapture proved Settings Back/opener focus and Explorer taskbar binding against frozen `92C3919D…`. Scaling/High Contrast/Reduced Motion remain waived below. |
| R6 / AC5 | Implementer verified; independent pending | The three owned desktop spec files were changed first and their scoped diff-check passed before product UI edits. They retain Clean observation/selection/digest/confirmation invariants and define the five-mode, palette, bounded-motif, motion, accessibility, and mode-local-state constraints. |
| R7 / AC6 | Automated/header implementation verified; final-release taskbar recapture verified; independent pending | React shell uses `src/assets/devsweep-icon-master.png`; Tauri continues using the icon child's generated native outputs. The image remains decorative next to one accessible product name, and the exact-owned product-path audit found no `.trellis` runtime/reference string. A broader scan found only four pre-existing, untouched fixture-data paths in `desktop/src/api/fixtures/scan-report.real.json`; none is referenced by this task's runtime code. Recapture bound the unique Explorer taskbar button to the restart HWND/PID/path/hash with three same-timestamp captures. |
| AC7 | Implementer gates green; independent pending | Current focused TUI/Desktop tests, desktop web gate, Tauri build, format/diff/task validation, and repository `just ci` evidence are listed below. These do not substitute for the required independent check. |

## Final convergence implementation

- Desktop supporting destinations are typed (`Protection`, `Rules`, `History`,
  and shell-owned Settings/language); unavailable registrations are absent.
- Desktop navigation uses deterministic initialization, `pushState` for user
  navigation, `popstate`/Back restoration, cancel-and-join, stale request
  rejection, and focus restoration.
- The real TUI `ShellComposition` is passed into the single `App` reducer rather
  than loaded and discarded. `p` is a visible English/Chinese settings action;
  selection emits `SavePresentationLanguage`; runtime owns the store call and
  produces typed result events; failure stays visible without changing locale.
- Stale presentation results are rejected by request id, and all existing Clean
  target/selection/digest/confirmation/worker state is preserved.
- The independent review found that Chinese Clean advertised `[Q]` while the
  real reducer reserved `q` for Quit. The corrected catalogue leaves the Clean
  label and `root_modes` visibility group intact but sets its accelerator to
  null. Every other Chinese field and every English/root-mode accelerator is
  unchanged. Central and consumer validators accept an absent mnemonic while
  still requiring every present mnemonic to have a group and remain unique.
- Persisted-language final-source PTY trace:
  `evidence/tui-restart-final/verification.md`. The corrective final-source
  Chinese Quit trace is `evidence/accelerator-repair-pty/verification.md`; its
  debug binary SHA-256 is
  `A4F8510574AC204AEEE9604D5394551791FA0F7F089044D9C9934B45E5878768`.

## Independent finding correction

The prior claim that every locale-specific root mode had a collision-free
accelerator was invalid: Chinese Clean exposed `[Q]`, but the real input reducer
always treated `q` as Quit. Independent `trellis-check` rejected that state.
Following the user's exact decision, this task changed only
`messages.command.clean.accelerator` from `"Q"` to `null` in the Chinese
catalogue. Recursive JSON and raw-byte comparison found exactly that one
catalogue difference; the corrected catalogue SHA-256 is
`F7ED1FDCF88AEBA6EDD03FE502758E51A12B84CE450D89CECC475C31243F8D32`.

The 120-column direct ConPTY trace showed the visible `清理` label with no
`[Q]`, `[q] Quit` in the footer, `q` opening the active-job Quit guard, and `c`
performing cancel-and-wait before a graceful exit 0. The isolated explicit-CLI
run wrote no persisted settings and left zero process/listener/lock residue.

### Strict Desktop store correction

The earlier report overstated Desktop fail-closed behavior. Independent check
proved that Desktop rendered an OS/English-derived shell before load completion,
fell back after a corrupt/unknown/newer/load failure, and changed the visible
locale before save success. Those claims are retracted. Under the user's strict
gate decision, Desktop now renders no `AppShell` until a successful load; load
failure exposes one accessible bilingual recovery surface with no claimed
locale. Save keeps the prior locale and select value until the returned closed
tag exactly matches the request. Rejection or mismatch preserves the prior
locale and reports recovery copy. Late load/save settlement is ignored after
cleanup/unmount. Focused evidence and repair history are recorded in
`evidence/strict-store-verification.md`.

### Final static-audit convergence correction

The previous report still overstated several implementation details after the
strict-store and focus repairs. Those claims are retracted until a fresh
independent check evaluates this final diff. The user authorized one combined
repair round with these bounded corrections:

- The English and Simplified Chinese canonical JSON catalogues now own the
  exact 19-key `shell.v1.*` namespace. TUI and Desktop project shell/settings,
  persistence, and route-error copy from those catalogues rather than private
  bilingual tables. Rust and TypeScript closed-schema tests lock key, form,
  placeholder, count, accelerator/group, and truncation parity. Excluding the
  additive namespace, deterministic legacy-message hashes remain
  `2ae2cd066c151192b86e52284faa2dc692649239a2c691af6c2fe27f3e85baa2`
  (English) and
  `1988b6f0370a9cd38ece4859f861c54b861cb8e5b52754cc4f81d91a162d4485`
  (Chinese, including the already-approved Clean accelerator `null`).
- Desktop heavy service calls now live inside a coordinator-owned start
  closure. Cancellation/join/refusal completes before service invocation;
  result rejection is normalized and each lease completes at most once.
- TUI no longer starts a parallel replacement Scan. The reducer queues one
  replacement, requests cancellation, ignores stale progress/results, and the
  runtime joins the registered worker before dispatching the terminal event
  that may start the replacement.
- Clean presentation delegates byte formatting to the canonical IEC helper;
  focused boundaries cover `1023 B`, `1.0 KiB`, `1.0 MiB`, `1.0 GiB`, and
  `1.0 TiB`, including real dry-run consumer assertions.
- Route cancellation failure is caught and rendered as a locale-aware
  canonical accessible alert with dismiss recovery. It does not install the
  failed route/history/focus intent or produce an unhandled promise.
- The shell typography contract now uses `Segoe UI Variable` first, then
  `Segoe UI` and system fallbacks.

All results above are implementer evidence and remain **awaiting independent
verification**.

## Final commands and logs

| Command | Exit/result | Complete log/evidence |
| --- | --- | --- |
| `rtk cargo test -p devsweep-cli tui` | 0; 72 passed | `evidence/logs/tui-complete-integration-focused-final.log` |
| Desktop focused AppShell/App tests | 0; 20 passed | `evidence/logs/desktop-navigation-lint-repair.log` |
| `rtk just desktop-web-check` | 0; lint/typecheck, 66 tests, build | `evidence/logs/desktop-web-check-final-after-lint-repair.log` |
| `rtk just desktop-build` | 0; release exe and one NSIS bundle | `evidence/logs/desktop-build-final-exit-confirmation.log` |
| `rtk cargo fmt --all -- --check` | 0 | `evidence/logs/final-diff-format-task-validate.log` |
| `rtk git diff --check` | 0 | `evidence/logs/final-diff-format-task-validate.log` |
| `python ./.trellis/scripts/task.py validate <task>` | 0 | `evidence/logs/final-diff-format-task-validate.log` |
| `rtk just ci` | 0; fmt/update-offline/check/workspace tests/clippy `-D warnings` | `evidence/logs/just-ci-final-convergence.log` |
| Final exact-owned `.trellis` reference, manifest, artifact, process/listener audit | 0; no owned reference/dependency diff/residue | `evidence/logs/final-scope-residue-artifact-audit.log` |

Corrective independent-finding gates:

| Command | Exit/result | Complete log/evidence |
| --- | --- | --- |
| `rtk cargo test -p devsweep-cli i18n` and focused TUI contract tests | 0; i18n 8 passed, TUI 73 passed | `evidence/logs/accelerator-repair-focused.log` |
| Desktop i18n/AppShell focused tests | 0; 2 files, 12 tests | `evidence/logs/accelerator-repair-desktop-focused.log` |
| Final Chinese ConPTY `q` Quit/cancel-and-wait trace | 0; no `[Q]`, graceful exit, zero residue | `evidence/accelerator-repair-pty/verification.md` |
| `rtk just desktop-web-check` | 0; lint/typecheck, 66 tests, build | `evidence/logs/accelerator-repair-desktop-web-check.log` |
| `rtk just desktop-build` | 0; release exe and one NSIS bundle | `evidence/logs/accelerator-repair-desktop-build.log` |
| `rtk git diff --check` and task validation | 0 | `evidence/logs/accelerator-repair-diff-task-validate.log` |
| First corrective `rtk just ci` | 1; rustfmt-only diagnostic | `evidence/logs/accelerator-repair-just-ci-full.log` |
| Corrective `rtk just ci` after exact rustfmt repair | 0; fmt/update-offline/check/workspace tests/clippy `-D warnings` | `evidence/logs/accelerator-repair-just-ci-format-repair-full.log` |

Strict Desktop store corrective gates:

| Command | Exit/result | Complete log/evidence |
| --- | --- | --- |
| Focused App/AppShell/i18n tests | 0; 3 files, 30 tests | `evidence/strict-store-verification.md` |
| First `rtk just desktop-web-check` | rejected despite recipe exit 0; lint stage failed on redundant effect state set | `evidence/logs/strict-store-desktop-web-check-first-failure.log` |
| Focused lint and typecheck after exact repair | 0 | `evidence/strict-store-verification.md` |
| Repeated `rtk just desktop-web-check` | 0; lint/typecheck, 72 tests, Vite build | `evidence/strict-store-verification.md` |
| `rtk just desktop-build` | 0; release exe and one NSIS bundle | `evidence/strict-store-verification.md` |
| `rtk git diff --check` and task validation | 0 | `evidence/strict-store-verification.md` |
| `rtk just ci` | 0; fmt/update-offline/check/workspace tests/clippy `-D warnings` | `evidence/logs/strict-store-just-ci-full.log` |

Final static-audit convergence gates:

| Command | Exit/result | Complete log/evidence |
| --- | --- | --- |
| Rust i18n/shell/app/runtime and full TUI focused tests | 0 final; 10/10, 4/4, 31/31, 11/11, 74/74 | `evidence/logs/final-static-convergence-focused.md`; preserved first-failure RTK logs named there |
| Desktop i18n/coordinator/AppShell/App/styles focused tests | 0 final; 49/49 | `evidence/logs/final-static-convergence-focused.md` |
| Desktop lint and typecheck | 0 / 0 | `evidence/logs/final-static-convergence-focused.md` |
| `rtk just desktop-web-check` | 0; lint/typecheck, 82 tests, Vite build | `evidence/logs/final-static-convergence-desktop-web-check.log` |
| `rtk just desktop-build` | 0; release exe and one NSIS bundle | `evidence/logs/final-static-convergence-desktop-build.log` |
| Catalogue legacy-hash audit | 0; hashes match the pre-addition baselines above | `evidence/logs/final-static-convergence-focused.md` |
| `rtk git diff --check` and task validation | 0 / 0; 7 implement and 3 check context entries valid | `evidence/logs/final-static-convergence-final-audit.log` |
| `rtk just ci` | 0; fmt/update-offline/check/workspace tests/clippy `-D warnings` | `evidence/logs/final-static-convergence-just-ci.log`; complete unfiltered output `evidence/logs/final-static-convergence-just-ci-full.log` |

Final lifecycle/ACL correction gates:

| Command | Exit/result | Complete log/evidence |
| --- | --- | --- |
| Rust capability and debug-fault focused tests | 0 | `evidence/logs/final-lifecycle-acl-focused-round1.log` |
| Lifecycle/App/AppShell/coordinator focused tests | 0; 54/54 | `evidence/logs/final-lifecycle-acl-ts-focused.log` |
| Desktop lint and typecheck | 0 / 0 | `evidence/logs/final-lifecycle-acl-lint-typecheck.log` |
| `rtk just desktop-web-check` | 0; 96/96 and Vite build | `evidence/logs/final-lifecycle-acl-desktop-web-check.log` |
| `rtk just desktop-build` | 0; release exe and one NSIS bundle | `evidence/logs/final-lifecycle-acl-desktop-build.log` |
| Exact capability/generated-schema/release-seam audit | 0 after correcting a PowerShell generic-call error | `evidence/logs/final-lifecycle-acl-schema-seam-release-hashes-round2.log`; first command-error log preserved |
| `rtk git diff --check` and task validation | 0 / 0 | `evidence/logs/final-lifecycle-acl-diff-validate.log` |
| First ACL `rtk just ci` | 1; rustfmt-only diagnostic | `evidence/logs/final-lifecycle-acl-just-ci.log` |
| Corrected ACL `rtk just ci` | 0; fmt/update-offline/check/workspace tests/clippy `-D warnings` | `evidence/logs/final-lifecycle-acl-just-ci-round2.log` |

Frozen final artifacts after the artifact-only rebind (superseding the older
`DEC46...D604` release and `029FFD...E8C4` NSIS evidence for final-source claims):

- `target/release/devsweep-desktop.exe`: 9,827,328 bytes, SHA-256
  `92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20`.
- `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe`: 3,152,372 bytes,
  SHA-256 `0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328`.
- `target/debug/devsweep-desktop.exe`: 14,758,912 bytes, SHA-256
  `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5`.

Native close recapture (no rebuild; PowerShell 7.6.5 wrapper exit 0, helper exit 0):

| Command / artifact | Exit/result | Evidence |
| --- | --- | --- |
| `pwsh.exe -NoProfile -ExecutionPolicy Bypass -File native-close-wrapper.ps1 ... -FirstPort 9401 -RestartPort 9402` | wrapper 0; helper 0; `outer-exit.helper_exit` 0 | `evidence/native-cdp-close-recapture-20260830/` |
| Transcript before launch | authentic; 63 lines / 8,020 bytes | `native-release/02-native-release-rebind.log` SHA-256 `CAE69BC936C590D534C78425DFC6DAE5CB6CB24F3CE28DA7750DB2810747A336` |
| CDP send/receive/event JSONL | 50 lines / 19,265 bytes | `native-release/cdp-messages.jsonl` SHA-256 `E455586A37FF428BCD984AC3A7B76E7E43FA6AE2B9B8E641E4E63676E87DF266` |
| Frozen hashes before and after | unchanged `92C3919D…` / `F5DFC77B…` / `0903D735…` | helper `ARTIFACT_GATE when=before_launch` and `after_run` |
| First close residue | main WaitForExit 41 ms; owned=0 at 818 ms; query 718.8 ms; no kill | PID 53684 HWND `0x30E1458` |
| Restart close residue | main WaitForExit 63 ms; owned=0 at 798 ms; query 720.7 ms; no kill | PID 47692 HWND `0x1C7A0904` |


## Native/manual evidence matrix

| Observation | State | Evidence boundary |
| --- | --- | --- |
| Final bare-TUI visible settings action, exact V1 write, persisted restart, explicit session-only override, Chinese Clean without `[Q]`, exclusive `q` Quit flow, graceful cancel-and-wait exit, zero residue | PASS | Direct final-source ConPTY traces and process transcripts under `evidence/tui-restart-final/` and `evidence/accelerator-repair-pty/` |
| Final-hash Windows Tauri launch, English shell, Language focus, Settings activation | PASS | Recapture PowerShell 7 transcript/CDP chain against `92C391...B4E20`; first PID 53684 HWND `0x30E1458` |
| English to Chinese save and exact V1 bytes | PASS | Recapture canonical Chinese shell and exact 39-byte store SHA-256 `2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`, lock/temp zero |
| No-flag Chinese restart and Settings Back/focus | PASS | Restart PID 47692 HWND `0x1C7A0904` locale `zh-CN`; Back restored `BUTTON` / `语言` |
| Two-run window/CDP error arrays | PASS | Recapture `error_audit` unhandled/window/runtime/console/log all empty |
| Debug canonical route alert and unchanged route | PASS | New .NET direct-CDP run selected the unique native WebView target, observed exact copy, retained `#/clean`, kept Settings absent, and recorded empty window unhandled/error arrays |
| Debug route-fault focus | PASS | After the real one-shot failure `document.activeElement` was the `BUTTON` with exact text `Language`; `evidence/focus-failure-repair-20260830/verification.md` |
| Debug dismiss/retry, one-shot recovery, native alert screenshot | PASS | Alert cleared; retry entered `#/settings`; two HWND-bound captures and exact DOM records are preserved |
| Closed debug error audit | PASS interaction/final residue; original helper command exit 2 | Reproducible offline audit: Runtime exception/console error/window unhandled/window error all zero; sole Log error exact known Vite favicon 404. Main exited in 46 ms; helper's final 750 ms audit was zero after an over-eager immediate descendant snapshot. |
| Natural last-window close and zero process/child/listener/lock/temp residue | PASS | First main WaitForExit 41 ms, owned=0 at 818 ms query 718.8 ms; restart WaitForExit 63 ms, owned=0 at 798 ms query 720.7 ms; no identity-gated kill; final main/webview/ports/lock/temp 0 |
| Header/title/taskbar original icon presence | PASS | HWND-bound Chinese header plus Explorer `Shell_TrayWnd` `0x10162` taskbar button bound to HWND `0x1C7A0904` PID 47692 with three captures `20260830-214341528` |
| 390/800/1024/1440 responsive behavior | PASS automated; native current-DPI captures diagnostic only | Final-hash screenshots record the four requested widths at DPI 120; exact native dimensions remain waived |
| 100/125/150/200% native scaling and exact native dimensions | **WAIVED/UNVERIFIED** | User waiver; 125% DPI-virtualized captures remain diagnostic only |
| Native Windows High Contrast appearance | **WAIVED/UNVERIFIED** | User waiver; forced-colors CSS/test remains PASS only as automation |
| Native Windows Reduced Motion appearance | **WAIVED/UNVERIFIED** | User waiver; reduced-motion CSS/test remains PASS only as automation |

The latest native recapture is
`evidence/native-cdp-close-recapture-20260830/verification.md`. It supersedes
run3 for final-source locale/restart/Back/close/taskbar claims against frozen
`92C3919D…`. The focus-repair report remains
`evidence/focus-failure-repair-20260830/verification.md` for debug route-error
focus/dismiss/recovery. Run1/run2 remain rejected historical diagnoses. Run3
remains a failed first-close residue record and is not run4. Scaling/High
Contrast/Reduced Motion retain their user-approved `WAIVED/UNVERIFIED` state.
The task is awaiting independent verification; this recapture is not an
independent `trellis-check` PASS.

## Repair history

- The original full-gate Windows lock failure was reproduced with stage/path
  context at `probe_lock_after_access_denied` (`os code 5`). The evidence-backed
  fix closes the lock handle before sidecar removal while retaining the lock for
  the whole transaction. Focused stress and the one user-approved extra `just
  ci` passed; original failure logs remain preserved.
- The first accessibility scan found five structural/contrast issues; scoped
  semantic/CSS repairs produced zero violations while preserving prior logs.
- Final convergence focused tests exposed cross-test history leakage and one
  post-locale route assumption; both were repaired with isolated history setup
  and a real route action, then 20/20 focused tests passed.
- The first final web gate was green but emitted one exhaustive-deps warning for
  initial hash normalization. Replacing captured render values with immutable
  initial refs removed the warning; lint and the full web gate then passed with
  no warning.
- A preliminary PTY trace predated the final visible `[p] Language` footer.
  Rather than overstate that evidence, it was retained and the complete
  three-process trace was repeated with the final-source binary under
  `evidence/tui-restart-final/`.
- Independent review then caught the Chinese `[Q]`/Quit conflict. The user
  authorized exactly one catalogue-field correction in this active shell task.
  Central/desktop validators and real reducer/render tests now enforce the
  absent Chinese Clean mnemonic while keeping `q` exclusive to Quit. The first
  corrective full gate exposed only rustfmt wrapping; that exact formatting
  repair was applied and the repeated full gate passed.
- A later independent AC4 check rejected Desktop's pre-load OS/English fallback
  and optimistic locale switch. The strict repair adds an explicit store gate,
  bilingual unavailable surface, matching-response commit point, controlled
  pending selection, recovery copy, and generation/in-flight cleanup guards.
  The first focused run exposed legacy fixtures that relied on the production
  bridge; injecting an explicit successful fake store made their dependency
  truthful. The first web gate then exposed one redundant effect state-set lint
  error; removing only that redundant assignment produced green lint,
  typecheck, 72-test web gate, release build, diff/task validation, and full CI.
- Final native close attempts proved the frontend drain was correct but Tauri's
  post-drain `window.destroy` IPC lacked its exact ACL permission. The user
  authorized only `core:window:allow-destroy`; a static test locks the exact
  three-permission capability, and generated-schema audit proves it maps only
  to `destroy`. The final-hash release then exited naturally with every owned
  child gone and zero listener/lock/temp residue.
- The sole remaining native round reached Settings through real keyboard focus
  but the native select action did not create the Chinese V1 document. The run
  stopped immediately without another locale input or debug harness launch.
  Locale save/restart/Back, debug route-error, and final-hash taskbar evidence
  remain explicitly `UNVERIFIED` rather than being inferred from tests or old
  captures.
- The later user-authorized evidence-only agent-browser round closed the locale
  save, exact 39-byte V1, no-flag Chinese restart, real History Back,
  opener-focus, and natural-close clauses against the unchanged C75D release.
  It used named sessions and loopback-only CDP. A taskbar button could not be
  uniquely bound to the Tauri HWND, so taskbar appearance remains
  `UNVERIFIED`. Starting `tauri dev` first relinked the debug binary away from
  E16F. After the user fixed the resulting 5677 hash and agent-browser attempts
  were abandoned, the final run used `/json/list` plus .NET
  `ClientWebSocket` directly against the unique native target. It proved the
  canonical alert and unchanged route but found focus on `BODY`, then stopped
  before dismiss/retry. This historical finding triggered the later scoped
  focus repair. The debug app and Vite closed naturally, leaving no
  DevSweep/WebView/CDP/Vite/agent-browser/store residue. Full evidence and
  command diagnostics are under
  `evidence/final-native-agent-browser-20260830/`.
- The final focus repair retained the matching activator intent across native
  disabled-control focus loss and restored it only after the canonical error
  surface and enabled controls committed. Focused/broad gates passed. The sole
  new direct-CDP run proved focus, dismiss, retry, screenshots, and empty window
  unhandled/error hooks. A closed offline audit classified the sole Log error as
  the exact known Vite `/favicon.ico` 404 without changing the original exit 1.
  Release attempt 1 then exposed a Settings selector mistake; attempt 2 exposed
  a fixed-one-second evidence sampling mistake. The explicitly approved third
  method sampled every 250 ms for up to five seconds and passed both closes,
  locale save/restart, Back/focus, exact store, and error audits. Taskbar direct
  binding remained unavailable. Full evidence is under
  `evidence/focus-failure-repair-20260830/`.
- The later independent check found that a runtime-invalid `hashchange` or
  `popstate` left the previous route rendered beside an invalid URL. The repair
  routes parsing failure to the first available canonical route through the
  existing request-keyed cancel/join lifecycle, uses replace-not-push typed
  history, restores the old canonical URL/focus/error on cancellation failure,
  and rejects stale/unmounted settlement. All focused/broad gates pass. Final
  debug/release native evidence passes against `F5DFC7...65F5` and
  `DEC46F...D604`. The combined release helper's taskbar activation initially
  stopped on an unavailable Legacy UIA type; the explicitly authorized
  taskbar-only run changed method to rectangle-center SendInput and directly
  bound the unique Explorer button to HWND `0x6D412C2`, PID 67692, exact path,
  and final hash, with same-timestamp screenshots and zero residue. Full
  evidence is under `evidence/invalid-hash-repair-20260830/`.
- The first final artifact-only rebind record under
  `evidence/final-artifact-rebind-92c-20260830/` is rejected/superseded because
  its report claims a raw `02-native-release-rebind.log` that does not exist.
  The single authorized run2 under
  `evidence/final-artifact-rebind-92c-run2-20260830/` froze the same release,
  debug, and NSIS hashes and created an authentic transcript before launch, but
  failed during the first CDP receive because Windows PowerShell 5.1 does not
  support `ConvertFrom-Json -Depth`. It was not retried. The exact residual PID
  closed naturally with zero first-sample main/WebView/listener residue and no
  forced termination. Locale/store/restart/Back/UIA/taskbar remain unverified.
- The final run3 under
  `evidence/final-artifact-rebind-92c-run3-20260830/` used absolute PowerShell
  7.6.5 with successful nested-JSON, ClientWebSocket, UIAutomation, SendInput,
  static-parse, artifact, and empty-prelaunch gates. Its authentic transcript,
  ordered 22-line CDP JSONL, and matching helper/wrapper exit 1 are preserved.
  It proved first-run English-to-Chinese exact 39-byte persistence and an
  HWND-bound Chinese capture. The main process exited naturally in 115 ms, but
  the five-second gate still observed three owned WebViews; restart and
  UIA/taskbar were therefore not run. Later read-only residue was zero, without
  forced termination. This bounded failure is superseded for final-source
  close/restart/taskbar claims by
  `evidence/native-cdp-close-recapture-20260830/`.
- Evidence-only native recapture under
  `evidence/native-cdp-close-recapture-20260830/` used PowerShell 7.6.5, a
  no-pipeline wrapper, authentic transcript-before-launch, and CDP JSONL.
  Close order: `CloseMainWindow` while JS alive, `WaitForExit`, then
  `Target.detachFromTarget`/dispose, then filtered owned-WebView poll.
  Wrapper/helper exit 0. Frozen hashes unchanged before and after. First
  en→zh 39-byte V1, no-flag zh restart, Back/opener focus, empty two-run
  error arrays, Explorer taskbar HWND binding with three captures, and
  graceful close main/owned/listener/lock/temp = 0 with no identity-gated
  kill. Scaling/High Contrast/Reduced Motion remain `WAIVED/UNVERIFIED`.

## Exact product/spec files changed by this task

- `.trellis/spec/desktop-frontend/{component-guidelines.md,index.md,state-management.md}`
- `crates/devsweep-core/src/{lib.rs,presentation_settings.rs}`
- `crates/devsweep-cli/src/application/mod.rs`
- `crates/devsweep-cli/src/i18n/mod.rs`
- `crates/devsweep-cli/src/tui/mod.rs`
- `crates/devsweep-cli/src/tui/shell/mod.rs`
- `crates/devsweep-cli/src/tui/app/{events.rs,input.rs,mod.rs,tests.rs,worker.rs}`
- `crates/devsweep-cli/src/tui/app/jobs.rs`
- `crates/devsweep-cli/src/tui/runtime/{mod.rs,tests.rs}`
- `crates/devsweep-cli/src/tui/render/{mod.rs,overlays.rs,tests.rs}`
- `desktop/src-tauri/capabilities/default.json`
- `desktop/src-tauri/src/{commands.rs,lib.rs}`
- `desktop/src/{App.tsx,App.test.tsx,styles.css,styles.test.ts}`
- `desktop/src/{lifecycle.ts,lifecycle.test.ts}`
- `desktop/src/components/format.ts`
- `desktop/src/app-shell/{AppShell.tsx,AppShell.test.tsx,index.ts,operation-coordinator.test.ts}`
- `desktop/src/i18n/{index.ts,index.test.ts}`
- `desktop/src/state/operation-coordinator.ts`
- `resources/i18n/{en.json,zh-CN.json}`

Task-owned evidence/log/report artifacts are additional authorized changes.
No manifest/lock/dependency file changed. Startup dirt `.trellis/.gitignore`,
`README.md`, and `justfile` remains present and excluded from this task.
