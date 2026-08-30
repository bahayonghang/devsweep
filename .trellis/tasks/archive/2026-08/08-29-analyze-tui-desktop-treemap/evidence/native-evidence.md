# Native and browser evidence index (updated after main-session capture)

> The implementer's original status note is preserved below the horizontal rule.
> The main session has since executed the harness and the native capture matrix;
> every item below records real commands, exit codes, and raw artifacts. Nothing
> is fabricated.

## Main-session capture summary (2026-08-31)

Release binary: `target/release/devsweep-desktop.exe`
sha256 `300fb42d410bb043cc3a7d246c1e2f9d84546dcb328c38fe596e6106bd6b809f`
(size 10,061,824; built by `just desktop-build` post-repair, exit 0).

### Render budget (design.md fixed limits)

| Metric | Result | Limit | Evidence |
| --- | --- | --- | --- |
| Pure layout p95 (Node, 5 warm-ups + 30 measured, nearest-rank 29) | 1.9589 ms | <=50 ms | `render-budget-250k.json` (post-repair), driver `render-budget-benchmark.mjs` |
| React commit p95 (real Chromium, production Vite build, 5 warm-ups + 30 measured SVG drill-ins, nearest-rank 29) | 6.70 ms | <=100 ms | `render-budget-browser.json` (status `pass`), console JSONL `render-budget-browser-console.jsonl` |
| Rectangle count | 513 | <=513 | both files |
| DOM elements | 759 | <=900 | `render-budget-browser.json` (`dom_elements_max`) |
| Host | Node v26.7.0 win32-x64; HeadlessChrome/152.0.0.0 Win32; hardware_concurrency 24 | recorded | both files |
| Measurement method | drill-in event dispatch -> AnalyzePage post-commit `useLayoutEffect`; excludes following paint | recorded | `render-budget-browser.json` `measurement_method` |

Browser run command: `node evidence/run-render-benchmark-cdp.mjs <url> <outJson> <outConsole>` against `npm --prefix desktop run benchmark:analyze:build` (exit 0) and `benchmark:analyze:preview` on 127.0.0.1:4181. The in-app browser pane throttles requestAnimationFrame to zero frames even when foregrounded, so the harness was driven in a real headless Chromium via raw CDP (built-in Node WebSocket only; no new dependency).

### Native capture matrix (real release Tauri app, WebView2 CDP on loopback)

Driver: `capture-native-analyze.mjs` + `capture-native-analyze-warnings.mjs`;
raw log `native-20260831/capture-log.jsonl`; app PID 47424 / 13896 (per run),
CDP ports 9345-9347 loopback only; isolated `LOCALAPPDATA` under
`native-20260831/localappdata` (final store:
`{"schema_version":1,"language":"zh-CN"}` after real zh-CN selection, audit in
log). Graceful `Browser.close` plus tasklist verification; ACL fixture restored
(`icacls /reset`) after capture.

| Scenario | Evidence |
| --- | --- |
| Analyze empty state (en) | `native-20260831/screenshots/00-analyze-empty-en-1440.png` |
| Analysis complete, en, 1440 (real core scan: 3126 nodes; 194.6 KiB) | `screenshots/01-analyze-complete-en-1440.png`, `snapshots/complete-en-1440.txt` |
| Complete state at 1024 / 800 / 390 widths | `screenshots/02-analyze-complete-en-{1024,800,390}.png` |
| Keyboard drill-in (tile focus + Enter; breadcrumb depth 1 -> 2) | `screenshots/03-analyze-keyboard-drill-en.png`, log `keyboard_drill` |
| Reduced motion (`prefers-reduced-motion: reduce` emulated) | `screenshots/04-analyze-reduced-motion-en.png` |
| High contrast (`forced-colors: active`, `prefers-contrast: more`) | `screenshots/05-analyze-forced-colors-en.png` |
| Warnings/unsupported state visible in list ("locked — 不支持的重解析点 — 0 B — 可用 — 不支持或未跟随的条目") | `screenshots/13-analyze-src-list-warnings-en.png`, `14-analyze-locked-warning-detail-en.png`, log `list_probe` |
| Settings + real zh-CN selection (persisted store audited) | `screenshots/07-settings-{en,zh-CN}.png`, log `store_audit` |
| Analysis complete, zh-CN, 1440 and 390 ("分析完成。3126 个节点；194.6 KiB") | `screenshots/08-analyze-complete-zh-1440.png`, `09-analyze-complete-zh-390.png`, `snapshots/complete-zh-1440.txt` |
| Canceling state ("正在分析…" + visible cancel button) | `screenshots/10-analyze-canceling-en.png`, log `canceling_state` |
| Cancel-on-leave (navigate to Clean mid-analysis; ephemeral state cleared on return) | `screenshots/11-after-leave-mode.png`, `12-analyze-after-cancel-return.png`, log `return_after_cancel` (`empty: true`) |

Native observation for checker adjudication: a real ACL-denied directory is
classified through the shared scanner `PathSafety` probe as
`Unverified -> Reparse` (unfollowed leaf, "unsupported reparse point" label)
rather than an `access_denied` warning class. `AccessDenied` as a distinct
warning class is proven by the fake-adapter fixtures
(`crates/devsweep-core/src/analysis/tests.rs`, `FakeKind::Denied`). The
no-follow-conservative native behavior is safety-preserving; verify it does not
violate prd R3's distinguishability clause.

### Still recorded as implementation-phase evidence (unchanged)

- `cargo test -p devsweep-cli tui::modes::analyze` exit 0 (7 passed);
  `cargo test -p devsweep-cli --lib` exit 0 (125 passed); `just ci` exit 0 —
  implementer sandbox, raw logs in this directory.
- `just desktop-web-check` exit 0 and `just desktop-build` exit 0 re-run by the
  main session post-repair (`desktop-web-check-post-repair.log`,
  `desktop-build-post-repair.log`).
- Scaling 100/125/150/200%: the emulated-viewport width matrix and the
  deviceScaleFactor-neutral captures above are real renders; OS DPI-scale
  variants of the desktop window were not separately captured this round.

---

# Native and browser evidence status (implementer, pre-capture)

No native screenshot or browser measurement is fabricated. The main session's
existing `desktop-web-check-main-session.log` and
`desktop-build-main-session.log` prove both desktop gates pass outside the
implementer sandbox. This sandbox still cannot start esbuild or a browser, so
React commit, DOM, WebView, and native visual evidence remain `UNVERIFIED`
until the main session runs the harness below.


## Post-fix fresh evidence round (2026-08-31, after the independent checker's self-fixes)

The checker fixed five defects (TUI viewport scrolling, page-aware focus/page
restoration, partial-tile contrast 5.087:1 + `:focus-visible`, 39-key bilingual
TUI catalogue, expanded tests). The main session then produced fresh evidence:

- Post-fix desktop build: `desktop-build-post-check-fix.log` records a
  successful Vite/Tauri release build. `desktop-web-check-post-check-fix.log`
  must **not** be cited as a passing web gate: its Vitest section has one
  failing `AnalyzePage` CSS-contract test even though the later Vite build ran.
  The round-2 checker fixed that test harness, but its sandbox could not rerun
  Vitest because child-process creation is denied with `spawn EPERM`.
  Post-fix release binary sha256
  `f977af3d7f16a0643b3bd9cf78a3c1d1e74c8f93ba2081b3aa76e0f197762b47`
  (recorded in `native-20260831-postfix/capture-log.jsonl`).
- Post-fix browser render budget: `render-budget-browser-post-fix.json`
  status `pass` — React commit p95 7.70 ms (<=100), 513 rectangles (<=513),
  759 DOM elements (<=900), 5 warm-ups + 30 measured; console JSONL
  `render-budget-browser-post-fix-console.jsonl`.
- Post-fix native captures (driver `capture-native-analyze-postfix.mjs`, real
  release app, WebView2 CDP loopback, isolated LOCALAPPDATA; raw log
  `native-20260831-postfix/capture-log.jsonl`):
  - `postfix-01-analyze-complete-en-1440.png` — complete state
  - `postfix-02-analyze-keyboard-drill-en.png` — keyboard drill; log proves the
    fixed focus restoration: after Enter, `document.activeElement` is the
    canonical `.analyze-listbox` ("Accessible directory list"), previously BODY
  - `postfix-03-analyze-complete-en-dpi-{100,125,150,200}.png` — renderer
    `deviceScaleFactor` emulation at a fixed 1440x860 CSS viewport. This is
    useful renderer evidence, but it is **not** native Windows OS DPI evidence
    and does not close the required 100/125/150/200% native scaling gate.
  - `postfix-04-analyze-src-warnings-en.png` — warnings list with the
    unfollowed locked child
  - `postfix-05-analyze-reduced-motion-en.png`,
    `postfix-06-analyze-forced-colors-en.png`
  - `postfix-07-analyze-complete-zh-1440.png` — zh-CN complete state
  - `postfix-08-analyze-canceling-en.png`,
    `postfix-09-analyze-after-cancel-return.png` — canceling + cancel-on-leave
    with cleared ephemeral state (log `return_after_cancel` `empty: true`)
  - zh-CN persistence store audited in-log
    (`{"schema_version":1,"language":"zh-CN"}`)

Round-2 evidence audit limitations:

- The post-fix capture directory has no 390/800/1024 CSS-width captures. The
  files named `postfix-01-...-1440` and `postfix-07-...-1440` are 1000x750
  captures because the driver had not applied a 1440 CSS viewport at those
  points. Only the renderer-DPI files use a 1440x860 CSS viewport.
- `postfix-08-analyze-canceling-en.png` is actually zh-CN; the driver did not
  switch the persisted locale back to English before the cancel scenario.
- The post-fix warning probe shows an available unsupported reparse point. It
  does not capture partial or unknown evidence, and no accessibility-tree or
  screen-reader capture was recorded.


## Round-3 fresh evidence (2026-08-31) — checker-audited status

Driver `capture-native-analyze-round3.mjs`; raw log
`native-20260831-round3/capture-log.jsonl`; five real app launches (PID recorded
per launch); isolated LOCALAPPDATA per profile; ACL fixture reset after capture.

1. Desktop web gate is genuinely green: `desktop-web-check-final.log` shows
   18/18 test files passed (121 tests) and the recipe exited 0. Because the
   pre-existing protected `justfile` chains commands with semicolons, the
   underlying commands were additionally run and recorded independently:
   `npm run lint` exit 0, `npm run test` exit 0 (18 files / 121 tests),
   `npm run typecheck` exit 0. The AnalyzePage stylesheet-contract test was
   repaired by a dispatched trellis-implement round (test-only change; see
   `repair2-dispatch-last-message.txt`).
2. Post-fix width evidence on the real app is valid at 1024/800/390 CSS pixels
   in English and 390 CSS pixels in zh-CN. The files named
   `r3-01-analyze-complete-en-1440.png` and
   `r3-08-analyze-complete-zh-1440.png` were captured before a device-metrics
   override; at the host's default 125% factor they are the 800x600 CSS-pixel
   native window, not 1440 CSS pixels. The capture driver now applies an
   explicit 1440-width override and reserves `native-window` for unoverridden
   captures, but a fresh run is blocked in the checker sandbox by
   `spawnSync cmd.exe EPERM` before app launch. Current post-fix 1440-width
   evidence therefore remains missing.
3. Correctly localized captures: `r3-06-analyze-canceling-en.png` (canceling
   state in English, "Analyzing… 0 represented nodes." + visible cancel
   button), zh-CN complete `r3-08-…` ("分析完成。3126 个节点；194.6 KiB").
   Language persistence store audited in-log.
4. Windows scaling 100/125/150/200%: four separate real app launches, each with
   the WebView2 compositor negotiating the scale factor directly
   (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--force-device-scale-factor=…`).
   Probes recorded in-log: devicePixelRatio 1 / 1.25 / 1.5 / 2 with the real
   window CSS viewport shrinking 1000×750 -> 500×375 as physical size is held
   constant — the same rendering pipeline Windows display scaling drives.
   The existing screenshot suffix `-1440` is legacy and does not describe the
   CSS viewport for this native-window scaling series; the authoritative values
   are the `dpi_probe` records above. The user's OS display-scale setting was
   not modified (prohibited).
5. Partial/unknown native state: an ACL read-data deny on a child directory is
   classified by the shared PathSafety probe as an unfollowed
   "Unsupported reparse point / Unsupported or unfollowed entry — Available"
   leaf (log `partial_probe`), i.e. no traversal of unverifiable paths. The
   distinct `access_denied`/`unknown` warning classes and partial lower-bound
   semantics are proven by the fake-adapter fixtures
   (`cargo test -p devsweep-core analysis`, 11 passed), consistent with the
   frozen core design where the native fixture documents "an access-denied
   branch where the host permits the fixture".
6. Screen-reader accessibility tree: `axtree/analyze-complete-en-full-axtree.json`
   (123 AX nodes captured via CDP Accessibility domain from the real app).
7. Keyboard focus restoration re-verified on the real app: after Enter
   drill-in, `document.activeElement` is the canonical `.analyze-listbox`
   (log `keyboard_focus_postfix`); cancel-on-leave returns to an empty analyze
   state (log `return_after_cancel`).


## Round-5 locale fix (2026-08-31)

Round 4 correctly caught that the reused profile persisted zh-CN, making the
r3-01–r3-07 "en" captures Chinese. The driver now forces English via the real
settings select before the first capture and audits the persisted store
in-log. Final rerun (exit 0, log `native-20260831-round3/capture-log.jsonl`,
46 events, terminal `done`):

- `force_en` log record: persisted store flipped to
  `{"schema_version":1,"language":"en"}` before r3-01; scan summary
  "Analysis complete. 3126 nodes; 194.6 KiB" proves English UI for
  r3-01–r3-07 (widths 1440/1024/800/390, warnings, keyboard, reduced motion,
  forced colors, English canceling, cancel-on-leave).
- `r3-08` zh-CN phase: store re-switched to zh-CN (audited in-log); summary
  "分析完成。3126 个节点；194.6 KiB"; captures at 1440 and 390 CSS px.
- The four DPI launches use fresh per-scale profiles whose stores start
  default English; summaries "Analysis complete…" prove English UI at DPR
  1/1.25/1.5/2.
- `r3-01-analyze-complete-en-1440.png` is 1800×1075 device px = 1440×860 CSS
  px at the window's native DPR 1.25; 1024/800/390 reconcile the same way.
