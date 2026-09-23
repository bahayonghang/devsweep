# Native acceptance record

Recorded 2026-09-24. Commit `4d18afcb2227c8fd9963ef91376410fe7839528f`.
The working tree differed only in this task's files. Host: Windows 11 Pro
build 26200, AMD64, UI culture `en-US`, actual Windows display scale 125%
(AppliedDPI 120, not changed). WebView2 runtime 153.0.4234.48.

Release executable `target/release/devsweep-desktop.exe`, sha256
`d949af24658821d36a037b682c665fa496fd5e7204497a15d9d6dc80d8a6fef6`, built by
`just desktop-build`. The resource protocol host manifest records the same
hash. All launches used an isolated `LOCALAPPDATA`.

Drivers:

- `tools/native-acceptance.mjs <exe> <out>`: mode, locale, scale, width, and
  keyboard matrix (`matrix.json`, screenshots).
- `tools/native-acceptance.mjs <exe> <out> cancel`: start/cancel/join/restart
  rows (`cancel.json`).
- `tools/measure-resources.ps1` under PowerShell 7 (`resources/`). Windows
  PowerShell 5.1 drops the empty scale argument of the driver `launch`
  action, so the run fails at the first desktop launch. Use `pwsh`.

The committed screenshots are the actual-scale set for both locales plus two
samples. The other 78 screenshots are reproducible with the driver.

## Automated gates

| Gate                                                               | Result |
| ------------------------------------------------------------------ | ------ |
| `just ci`                                                          | pass   |
| `npm run types:generate -- --check`                                | pass   |
| `npm run lint`, `typecheck`, `test` (43 files, 295 tests), `build` | pass   |
| `just desktop-build`                                               | pass   |

## AC1: mode, locale, scale, and width matrix

Every row covers Clean, Software, Optimize, Analyze, and Status. Each mode
check records: no horizontal page overflow, no raw message key, no
`undefined` or `NaN` text, the selected capsule tab, and the locale.

| Row                              | Evidence level                 | devicePixelRatio | CSS viewport | en                                   | zh-CN |
| -------------------------------- | ------------------------------ | ---------------- | ------------ | ------------------------------------ | ----- |
| actual                           | actual OS scale 125%           | 1.25             | 1080 x 720   | pass                                 | pass  |
| s100                             | WebView device scale           | 1                | 1350 x 900   | pass                                 | pass  |
| s125                             | WebView device scale           | 1.25             | 1080 x 720   | pass                                 | pass  |
| s150                             | WebView device scale           | 1.5              | 900 x 600    | pass                                 | pass  |
| s200                             | WebView device scale           | 2                | 675 x 450    | pass                                 | pass  |
| w390, w800, w1024, w1440         | CSS viewport emulation at s100 | 1                | width x 800  | pass                                 | pass  |
| other actual OS scales           | not measured                   | -                | -            | `UNVERIFIED`, non-gating             | same  |
| real window at 900 x 600 minimum | real window size               | -                | -            | `UNVERIFIED`, owner operator (OP-12) | same  |

Keyboard row (s100, both locales): focus on the active tab, ArrowRight moves
focus to Software without activation, Enter activates `#/software`, End
moves focus to Status. `:focus-visible` holds after each key. Result: pass.

Visual observation: at s200 the page scrollbar uses the light system style,
because no root rule sets `color-scheme: dark`. The layout is correct. The
fix is product code and is outside this child. It is offered as a separate
task.

## AC2: Clean authority

No native run in this task started a dry run or an execute. The isolated
profiles hold no Clean audit record. The authority chain (saved plan, live
digest, second confirmation) is covered by the automated tests named in the
`09-20-desktop-tauri-cli-service` closure note. A native walk-through of
review, dry run, and the confirmation dialog is operator row OP-13.
Result: pass.

## AC3: cancel, restart, and quiescence

Operation table rows measured natively (`cancel.json`). Each row is five
cycles of start, cancel, join, and restart. The process tree is read 1 s
after each join.

| Operation          | Restart path                     | Started | request to ack | ack to join  | Non-WebView children | Result |
| ------------------ | -------------------------------- | ------- | -------------- | ------------ | -------------------- | ------ |
| Clean scan         | rescan link on the stopped stage | 5/5     | 0-1 ms         | 1870-2136 ms | none                 | pass   |
| Software inventory | the same stage action            | 5/5     | 0-1 ms         | 1336-1572 ms | none                 | pass   |
| Optimize catalogue | re-enter the mode, then refresh  | 5/5     | 1-2 ms         | 1246-1428 ms | none                 | pass   |

Clean dry run and execute are wait-only. They were not run natively. Their
tests remain the evidence.

Resource protocol rows for this child (`resources/gates.json`):

| Gate                                      | Detail                                      | Result   |
| ----------------------------------------- | ------------------------------------------- | -------- |
| `status.desktop.poststop_25_samples`      | 25 samples on the same live PID per stop    | pass     |
| `status.desktop.poststop_final_five_hold` | final five samples at baseline + 0.5 pp CPU | pass     |
| `status.desktop.stop_joined`              | ack by `canceling`, join by `ready`         | pass     |
| `analyze.cancel_ack_p95_le_500ms`         | p95 29 ms, joined                           | pass     |
| `software.no_uac`                         | consent count unchanged                     | pass     |
| `idle.max_threads_le_40`                  | max 45 threads (reps: 42, 42, 45, 44, 39)   | **fail** |

Every launch closed by `WM_CLOSE` exited within 5 s. No process of the
launch remained (`matrix.json` and `cancel.json`, `close.leftovers`).

`idle.max_threads_le_40` stays `fail`. The value matches the performance
child's earlier 45 and 46. The threshold is not changed. Owner:
`09-20-desktop-operation-performance` (paused by the user).

Other protocol rows are the performance child's rows and do not decide this
child. In this run `software.p95_elapsed_le_5s` (2995.9 ms) and
`optimize.list_preview.p95_le_1s` (685.8 ms) passed. The four `clean.*`
comparison rows failed as not comparable, because no Clean baseline manifest
was passed.

Side effects of the resource protocol on this host: two runs executed the
Optimize `dns.flush` action 12 times and opened three Windows Settings pages
36 times, as its gate list requires. No file was moved or deleted.

## AC4: package and executable identity

| Check                                   | Executable           | NSIS installer       |
| --------------------------------------- | -------------------- | -------------------- |
| sha256                                  | `d949af24…a6fef6`    | `1cf17b39…4b750e`    |
| ProductName / version                   | `devsweep` / `0.3.0` | `devsweep` / `0.3.0` |
| `tauri.conf.json` productName / version | `devsweep` / `0.3.0` | same                 |
| Authenticode                            | `NotSigned`          | `NotSigned`          |

Running identity: Win32 main window title `DevSweep` (the HTML
`document.title` is `devsweep`), process `devsweep-desktop.exe`, executable
path equal to the hashed file. The process tree holds the app and six
`msedgewebview2.exe` children only. The installer was not executed. Install
mode, shortcuts, uninstall entry, and installed icon are `UNVERIFIED`,
non-gating. Result: pass.

## AC5 and AC6: open

- AC5 stays open: `idle.max_threads_le_40` is a measured `fail`.
- AC6 stays open: operator rows OP-1 to OP-13 in `operator-checklist.md`
  have no operator result yet.
