# Independent Trellis Check — PASS

The product defects found in rounds 1 and 2 remain fixed, the supplied final
desktop web log is green, and all independently runnable code gates pass. The
round-5 rerun closes the sole remaining evidence-integrity blocker: the reused
main profile is forced to English through the real settings control before the
English captures, the persisted store and rendered UI are audited, and r3-08 is
then switched back to zh-CN with its store audited separately. The refreshed
width/state/DPI PNGs, AX tree, raw event log, executable hash, and dimensions
are mutually consistent. See the round-5 section at the end for the current
verdict; earlier sections are retained as the historical record.

## Findings (fixed)

- File: `crates/devsweep-cli/src/tui/modes/analyze/mod.rs`, `crates/devsweep-cli/src/tui/render/analyze.rs`
  - Issue: The TUI always rendered the first rows of a page, so keyboard movement could leave the cursor off-screen; selection styling also depended on the viewport-local row index.
  - Fix: Scroll the bounded viewport around the actual page cursor, compare selection by node id, and add a regression test proving the cursor remains visible.
- File: `desktop/src/modes/analyze/state.ts`, `desktop/src/modes/analyze/AnalyzePage.tsx`, and tests
  - Issue: Drill-up, breadcrumb, and drill-down anchor restoration did not restore the canonical 200-row page, so the restored node could remain outside the rendered list.
  - Fix: Carry a computed page through directory navigation actions, derive it with `pageForNode`, and add reducer/component coverage with a 450-child parent.
- File: `desktop/src/modes/analyze/AnalyzePage.tsx`, `desktop/src/modes/analyze/AnalyzePage.test.tsx`
  - Issue: Keyboard drill-in lost focus to `BODY` (also recorded in `native-20260831/capture-log.jsonl:12`).
  - Fix: Restore focus to the canonical listbox after timed keyboard navigation and assert focus after Enter and Backspace.
- File: `desktop/src/modes/analyze/styles.css`, `desktop/src/modes/analyze/AnalyzePage.test.tsx`
  - Issue: Partial-evidence tile white text over `#9a7126` had a computed 4.408:1 contrast ratio, below 4.5:1; focus styling also used generic `:focus`.
  - Fix: Use `#8f671f` (computed 5.087:1), use `:focus-visible`, and add static regression checks for contrast, reduced motion, forced colors, and focus-visible.
- File: `crates/devsweep-cli/src/tui/render/analyze.rs`, `crates/devsweep-cli/src/tui/modes/analyze/mod.rs`, `resources/i18n/en.json`, `resources/i18n/zh-CN.json`
  - Issue: Several TUI labels were hard-coded English, permission/unsupported warnings were not rendered, and the root breadcrumb did not use the normalized root path.
  - Fix: Route controls, sort labels, empty/error/footer copy, and evidence warnings through the shared bilingual catalogue; render AccessDenied and unsupported/reparse warnings; show the normalized root path; add bilingual test-backend coverage. Both catalogues now have 39 Analyze keys with identical schemas.
- File: `desktop/src/modes/analyze/AnalyzePage.test.tsx`, `desktop/src/modes/analyze/state.test.ts`
  - Issue: Component coverage did not prove every required state in both languages, page-aware anchor restoration, focus restoration, or all CSS accessibility contracts.
  - Fix: Add table-driven English/zh-CN empty/loading/canceling/canceled/partial/error/complete coverage, no-cleanup assertions, labelled/link/focus checks, large-parent paging, and CSS contract tests.

## Findings (not fixed)

- File: `.trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/native-evidence.md:68`
  - Issue: AC3/implement.md requires native 100/125/150/200% scaling evidence, but the evidence says OS DPI variants were not separately captured.
  - Why not fixed: Capturing host OS DPI variants requires native GUI/environment authority unavailable to this checker.
- File: `desktop/vite.config.ts` / `desktop/node_modules/esbuild/lib/main.js:2268`
  - Issue: Independent Vitest, Vite, and Tauri gates fail before executing tests/build with `Error: spawn EPERM` (`errno: -4048`, `syscall: spawn`).
  - Why not fixed: This is a sandbox process-spawn restriction, not an in-repository defect; weakening or bypassing the required gates is not permitted.
- File: `.trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/render-budget-browser.json`, `.trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/native-20260831/capture-log.jsonl`
  - Issue: The recorded browser/native artifacts predate this checker's focus, navigation, contrast, bilingual-TUI, and test fixes. In particular the old keyboard log records `focused: "BODY"`; a post-fix native capture and post-fix browser commit sample are absent.
  - Why not fixed: Fresh browser production-build and native evidence depend on the blocked esbuild build and native GUI capture. The old layout/DOM samples remain internally valid, but they are not sufficient post-fix closeout evidence.

## Acceptance criteria

- **AC1 — FAIL:** Source/test audit confirms deterministic bounded layout coverage for zero/equal/extreme/tiny/Other/unknown/resize/parity behavior, and the raw pure-layout evidence passes. However, the post-fix desktop Vitest suite could not execute because esbuild failed to spawn. Rust Analyze tests pass. Evidence: `independent-check-cargo-test-tui-analyze.log`, `independent-check-render-budget-verify.log`, `independent-check-desktop-web-check.log`.
- **AC2 — FAIL:** The implementation now has a canonical 200-row list, labelled linked rectangles, logical focus restoration, keyboard drill-down/up, compliant partial-tile contrast, reduced-motion/forced-colors rules, and no cleanup action. The added post-fix component assertions could not execute, and the only native keyboard artifact is the pre-fix `BODY` focus record. Evidence: `independent-check-desktop-web-check.log`, `native-20260831/capture-log.jsonl`, source tests in `desktop/src/modes/analyze/AnalyzePage.test.tsx`.
- **AC3 — FAIL:** Existing evidence covers both languages, required states, and 390/800/1024/1440 widths. Recomputed nearest-rank values pass: layout p95 is 1.9589 ms, commit p95 is 6.69999998807907 ms, 513 rectangles, and 759 DOM elements after five warm-ups and 30 samples, with Node/browser versions recorded. The required native OS scaling matrix is explicitly absent and the browser/native evidence predates the fixes. Evidence: `independent-check-render-budget-verify.log`, `native-evidence.md:68-70`.
- **AC4 — FAIL:** Focused Rust Analyze gates, `just ci`, formatting, lint, type-check, and `git diff --check` pass. Independent `just desktop-web-check` and `just desktop-build` both exit 1 because esbuild cannot spawn; post-fix native evidence is absent. Evidence: all `independent-check-*.log` files.

## Cross-layer and safety audit

- Core snapshot -> CLI JSON/human -> Tauri operation events/result -> strict desktop decoder -> reducer/selectors/treemap -> canonical list/linked SVG DOM is traced and remains data-only.
- Reducers reject mismatched operation ids and non-monotonic sequences. Both TUI and desktop request cancel/join on mode leave before releasing ephemeral state.
- No Analyze output converts to `CleanupPlan`; no select/delete/cleanup affordance or cleanup authority exists in the new Analyze presentation.
- No frozen CLI parser, frozen `ModeId` variant set, or generated snapshot DTO schema was changed. Mode registration/wiring is the only shell touch. No Cargo/npm dependency was added.
- Selectors/layout have no filesystem/network access, and the presentation uses no canvas/WebGL.
- The PathSafety observation is acceptable: native `Unverified -> Reparse` is a conservative unfollowed leaf, while the fake filesystem covers `AccessDenied` distinctly. Both states are distinguishable, so this does not violate R3.
- The protected pre-existing dirty files `.trellis/.gitignore`, `README.md`, and `justfile` were not modified, staged, or reverted by this checker. No other task directory was touched.

## Verification

| Command | Exit | Result / evidence |
| --- | ---: | --- |
| `cargo test -p devsweep-cli tui::modes::analyze` | 0 | 8 passed; `independent-check-cargo-test-tui-analyze.log` |
| `cargo test -p devsweep-core analysis` | 0 | 11 passed; `independent-check-cargo-test-core-analysis.log` |
| `just desktop-web-check` | 1 | types generation, lint, and type-check passed; Vitest and Vite build blocked by `spawn EPERM`; `independent-check-desktop-web-check.log` |
| `just desktop-build` | 1 | Tauri beforeBuild Vite step blocked by `spawn EPERM`; `independent-check-desktop-build.log` |
| `git diff --check` | 0 | Pass with line-ending warnings only; `independent-check-git-diff-check.log` |
| `cargo fmt --all -- --check` | 0 | Pass; `independent-check-supplemental-gates.log` |
| `npm --prefix desktop run lint` | 0 | Pass; `independent-check-supplemental-gates.log` |
| `npm --prefix desktop run typecheck` | 0 | Pass; `independent-check-supplemental-gates.log` |
| `just ci` | 0 | Canonical Rust format/check/tests/clippy gate passed; `independent-check-supplemental-gates.log` |

Overall verdict: **FAIL** until the desktop gates run successfully in an environment that permits esbuild child processes and fresh post-fix browser/native evidence includes the OS DPI scaling matrix and keyboard-focus verification.

---

## Verification round 2 — FAIL (2026-08-31)

Round 2 confirms that the round-1 product fixes remain intact. The post-fix
browser sample is authentic and within every frozen render cap, the post-fix
release binary hash matches the native log, and the keyboard probe now restores
focus to the canonical `.analyze-listbox`. Closeout still fails for two
independent reasons: the supplied desktop web-check log contains a real failed
test despite being described as exit 0, and the new native capture is renderer
emulation rather than the required Windows OS DPI matrix and omits several
claimed matrix cases.

### Findings (fixed in round 2)

- File: `desktop/src/modes/analyze/AnalyzePage.test.tsx`
  - Issue: The post-fix `desktop-web-check-post-check-fix.log` shows that
    `styles.css?raw` resolved to an empty string under Vitest, so the new CSS
    accessibility regression test failed. The remaining 120 desktop tests
    passed, including all five behavior/state `AnalyzePage` tests.
  - Fix: Read the checked-in stylesheet with `readFileSync(new
    URL("./styles.css", import.meta.url), "utf8")` so the regression asserts
    the real CSS instead of Vitest's disabled CSS stub. Desktop lint and
    type-check pass, and a direct stylesheet-contract probe passes. A full
    Vitest rerun remains blocked by this checker's `spawn EPERM` sandbox.
- File: `.trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/native-evidence.md`
  - Issue: The index called the web gate passing and described renderer
    `deviceScaleFactor` emulation as OS DPI evidence; it also did not disclose
    mislabeled/missing width, locale, partial/unknown, and accessibility-tree
    captures.
  - Fix: Corrected the evidence index without changing any raw capture.

### Findings (not fixed in round 2)

- File: `justfile` / `desktop-web-check-post-check-fix.log`
  - Issue: The protected `desktop-web-check` recipe chains commands with
    semicolons, so a failed `npm test` can be masked when the later Vite build
    succeeds. The supplied log proves exactly that: Vitest reports 1 failed
    test and 120 passed tests, then the build succeeds. It is not a passing web
    gate even if the shell's final status was 0.
  - Why not fixed: The round-2 dispatch explicitly forbids touching `justfile`.
    The main session must make the recipe fail fast and rerun it after the test
    harness fix.
- File: `native-20260831-postfix/` and
  `capture-native-analyze-postfix.mjs:133-138`
  - Issue: The alleged 100/125/150/200% OS DPI matrix uses
    `Emulation.setDeviceMetricsOverride` at a fixed 1440x860 CSS viewport with
    only `deviceScaleFactor` changing. The PNG dimensions are respectively
    1440x860, 1800x1075, 2160x1290, and 2880x1720, which proves renderer bitmap
    scaling while the CSS layout remains 1440x860. This does not exercise
    Windows display scaling, WebView/window DPI negotiation, or breakpoint
    changes at a fixed physical display size.
  - Why not fixed: Authentic Windows 100/125/150/200% evidence requires a new
    native capture outside this sandbox. No cap, schema, or frozen contract was
    changed.
- File: `native-20260831-postfix/` capture matrix
  - Issue: The directory has no post-fix 390/800/1024 CSS-width screenshots.
    `postfix-01-analyze-complete-en-1440.png` and
    `postfix-07-analyze-complete-zh-1440.png` are actually 1000x750 because the
    driver never applied the claimed 1440 viewport before those captures.
    `postfix-08-analyze-canceling-en.png` is visibly and programmatically zh-CN.
    The warning probe records an **Available** unsupported reparse entry, not a
    partial or unknown evidence state. No accessibility-tree/screen-reader
    capture was recorded. The older pre-fix directory supplies 390/800/1024
    responsive images, but it does not make the claimed fresh post-fix matrix
    complete.
  - Why not fixed: These are missing native observations and cannot be
    reconstructed from screenshots or source review.
- File: desktop Vitest/Vite execution in the checker sandbox
  - Issue: `just desktop-web-check`, `just desktop-build`, and all attempted
    Vitest config-loader variants fail at child-process creation with
    `spawn EPERM` before a valid post-fix test run can complete.
  - Why not fixed: This is an execution-environment restriction. Lint,
    type-check, Rust tests, and `just ci` run normally.

### Round-2 raw evidence audit

- Pure layout: 30 raw samples, nearest rank `ceil(0.95 * 30) = 29`, recomputed
  p95 `1.9588999999999999 ms` (limit 50 ms), recorded value identical.
- React commit: 30 raw samples, rank 29, recomputed p95
  `7.699999988079071 ms` (limit 100 ms), recorded value identical. The raw
  browser console JSON exactly matches `render-budget-browser-post-fix.json`.
- Frozen caps: 513 rectangles (limit 513), 759 DOM elements (limit 900), five
  warm-ups plus 30 measured drill-ins. Pure/browser fixture hashes both equal
  `sha256:42685e0971165c2b59362cb44f251d4699e282bc647fd2f407d85559830ef37f`.
- Native authenticity checks: the current release executable SHA-256 equals the
  raw log's `f977af3d7f16a0643b3bd9cf78a3c1d1e74c8f93ba2081b3aa76e0f197762b47`;
  the log terminates with `done`; all 12 screenshot events have corresponding
  PNGs; the keyboard probe reports `activeClass: "analyze-listbox"` and two
  breadcrumbs; cancel-on-leave returns `empty: true`; the persisted store is
  exactly `{"schema_version":1,"language":"zh-CN"}`.
- Visual inspection confirms the canonical list focus ring, forced-colors
  rendering, bilingual complete state, unsupported warning presentation, and
  cleared post-cancel state. Static screenshots cannot by themselves prove
  reduced-motion behavior or screen-reader semantics.

### Acceptance criteria — round 2

- **AC1 — PASS:** The bounded layout/source contract remains intact. The
  post-fix main-session run passed the selector and treemap suites, including
  deterministic equal/resize, zero/unknown/extreme/tiny aggregation, exact
  `Other`, 10,000-child cap, pagination, and fixture-hash parity. The independently
  recomputed layout p95 and rectangle cap pass. The round-2 test-only CSS fix
  does not affect layout behavior.
- **AC2 — FAIL:** Product source and raw native evidence now show canonical
  list focus restoration, labelled rectangles, keyboard drill/down/up,
  contrast 5.087:1, forced-colors/reduced-motion rules, and no cleanup action.
  However, the CSS regression test has not completed after its harness fix, and
  the promised native screen-reader/accessibility-tree observation is absent.
- **AC3 — FAIL:** Both-language state behavior tests in the supplied run pass,
  and the render budget is comfortably inside every frozen cap. The required
  native scaling evidence is still absent: renderer `deviceScaleFactor`
  emulation is not Windows OS DPI scaling. The fresh capture also lacks
  390/800/1024 widths and partial/unknown state evidence, and several filenames
  misstate their actual viewport or locale.
- **AC4 — FAIL:** Focused Rust Analyze tests, the canonical Rust `just ci` gate,
  lint, type-check, direct stylesheet probe, and `git diff --check` pass. The
  supplied desktop release build is credible and its binary hash matches the
  native run. The web gate is not passing because its raw log contains one
  failed test; the checker cannot rerun Vitest/Vite/Tauri because of `spawn
  EPERM`; and the native matrix is incomplete.

### Verification command ledger — round 2

| Command | Exit | Result |
| --- | ---: | --- |
| `git status --short --branch` | 0 | Dirty scope inventoried; protected files preserved. |
| `git diff --name-only HEAD` | 0 | Change classes inventoried. |
| `python ./.trellis/scripts/get_context.py --mode packages` | 0 | Single-repo layers: backend, desktop-frontend, frontend. |
| Task/spec/source/evidence `Get-Content`, `Get-ChildItem`, and `rg` audit commands | 0 | All requested artifacts and relevant code paths read. One manifest-size probe also returned exit 0 but emitted a non-terminating diagnostic for stale path `crates/devsweep-cli/src/presentation/analyze.rs`; the real file is `application/presentation/analyze.rs`. |
| Local PNG inspection with `view_image` and `System.Drawing` dimension probe | 0 | All post-fix screenshots opened; dimensions listed above. |
| PowerShell nearest-rank recomputation over both raw arrays | 0 | Layout p95 1.9589 ms; commit p95 7.70 ms; rank 29 each. |
| PowerShell raw-console/capture-log/hash consistency audit | 0 | Browser JSON exact match; executable hash, focus, cancel return, store, and terminal log event verified. |
| `just desktop-web-check` | 1 | Checker sandbox: Vitest and Vite cannot start esbuild (`spawn EPERM`). |
| `npm --prefix desktop run lint` | 0 | Pass after round-2 test fix. |
| `npm --prefix desktop run typecheck` | 0 | Pass after round-2 test fix. |
| `node node_modules/vitest/vitest.mjs run src/modes/analyze/AnalyzePage.test.tsx --configLoader runner` | 1 | `spawn EPERM` during Vite real-path/config resolution. |
| `node node_modules/vitest/vitest.mjs run src/modes/analyze/AnalyzePage.test.tsx --configLoader native` | 1 | `spawn EPERM` starting the default fork pool; no tests ran. |
| `node node_modules/vitest/vitest.mjs run src/modes/analyze/AnalyzePage.test.tsx --configLoader native --pool threads --poolOptions.threads.singleThread` | 1 | `spawn EPERM` in Vite Windows real-path resolution; no tests ran. |
| `node -e "...stylesheet contract tokens..."` | 0 | Real CSS contains contrast, focus-visible, reduced-motion, and forced-colors contracts. |
| `cargo test -p devsweep-cli tui::modes::analyze` | 0 | 8 passed. |
| `cargo test -p devsweep-core analysis` | 0 | 11 passed. |
| `just ci` | 0 | Format/check/workspace tests/clippy pass; 363 tests passed, 2 ignored. |
| `git diff --check` | 0 | Pass; line-ending warnings only. |
| `just desktop-build` | 1 | Checker sandbox: Tauri before-build Vite step blocked by `spawn EPERM`. |

Supporting exploratory commands (`npm exec`/Vitest help and all targeted
read-only file/search probes) exited 0. No commit, push, dependency, cap,
schema, frozen CLI/DTO contract, `justfile`, protected dirty file, or other task
directory was modified.

Overall round-2 verdict: **FAIL**. Required closeout actions are (1) make the
desktop web-check recipe fail fast and rerun it after the test harness fix, and
(2) capture an authentic Windows 100/125/150/200% DPI matrix plus the missing
post-fix widths, correct locale labels, partial/unknown evidence, and a native
accessibility-tree/screen-reader observation.

---

## Verification round 3 — FAIL (2026-08-31)

Round 3 closes the round-2 product, desktop-test, scaling, locale, focus, and
accessibility-tree blockers. One independently proven evidence-integrity gap
remains: the current post-fix capture set does not contain a true 1440 CSS-pixel
render, although two files carry a `-1440` suffix. No product-code defect was
found in this round.

### Findings (fixed in round 3)

- File: `evidence/capture-native-analyze-round3.mjs`
  - Issue: `runAnalyzeAndCapture` wrote the file labelled `1440` before calling
    `Emulation.setDeviceMetricsOverride`; the default host window is 800x600
    CSS pixels at the observed 125% factor. The driver also truncated its JSONL
    log before fixture setup, so a sandbox spawn failure could erase the prior
    log before a new capture existed.
  - Fix: The width matrix now explicitly requests 1440/1024/800/390 in English
    and 1440/390 in zh-CN, unoverridden captures use the honest
    `native-window` suffix, and log reset occurs only after ACL fixture setup
    succeeds. `node --check` exits 0. The existing raw capture log was restored
    byte-for-byte from the copy read at the start of this round after the
    recapture attempt failed before app launch; it parses as the same 54 JSONL
    events and still ends in `done`.
- File: `evidence/native-evidence.md`
  - Issue: The round-3 index did not disclose that current 1440-width evidence
    is absent and retained a misleading `-1440` suffix for native-window DPI
    captures.
  - Fix: The index now distinguishes valid CSS-width evidence from the native
    scaling series and records the blocked recapture honestly.

### Findings (not fixed in round 3)

- File: `evidence/native-20260831-round3/screenshots/r3-01-analyze-complete-en-1440.png`,
  `r3-08-analyze-complete-zh-1440.png`
  - Issue: These are not 1440 CSS-pixel renders. The English file is byte-for-byte
    identical to the 125% native-window screenshot (SHA-256
    `f0fd88aadd485759d130940508130ca395f31bd5386ad4c2452f1f99230041f1`),
    whose adjacent `dpi_probe` records DPR 1.25 and `inner=[800,600]`. The driver
    confirms the screenshot preceded every width override. The zh-CN file was
    captured at the same unoverridden window state. Valid current width evidence
    exists at 1024/800/390 CSS pixels in English and 390 in zh-CN.
  - Why not fixed: Running the corrected driver exits 1 at fixture setup with
    `spawnSync cmd.exe EPERM`; the real app never launches. Missing native
    observations cannot be reconstructed or relabelled as 1440 evidence.
- File: checker execution environment
  - Issue: Independent `just desktop-web-check` exits 1 when Vitest and Vite
    attempt to start esbuild (`spawn EPERM`).
  - Why not fixed: This is the same sandbox restriction as round 2. It is not a
    code blocker because `desktop-web-check-final.log` shows the corrected suite
    genuinely passing 18/18 files and 121/121 tests, followed by a successful
    production Vite build; this checker independently reran lint and type-check.

### Round-3 evidence audit

- Pure layout: 30 samples, nearest rank 29, recomputed p95
  `1.9588999999999999 ms` (limit 50 ms), 513 rectangles (limit 513).
- Post-fix React commit: 30 samples, nearest rank 29, recomputed p95
  `7.699999988079071 ms` (limit 100 ms), 513 rectangles, and 759 DOM elements
  (limit 900). The pure/browser fixture hashes both equal
  `sha256:42685e0971165c2b59362cb44f251d4699e282bc647fd2f407d85559830ef37f`;
  browser console data matches the JSON result.
- Native authenticity: the current release executable hash equals the three raw
  log records (`f977af3d7f16a0643b3bd9cf78a3c1d1e74c8f93ba2081b3aa76e0f197762b47`);
  the 54-line JSONL parses and ends with `done`; every unique screenshot event
  has a PNG; focus is `.analyze-listbox`; cancel-on-leave returns `empty: true`;
  the persisted store is exactly `{"schema_version":1,"language":"zh-CN"}`.
- Scaling adjudication: **accepted for implement.md step 3**. Four separate real
  Tauri/WebView2 launches use app-scoped `--force-device-scale-factor` values,
  not page-level device-metrics emulation. Probes record DPR 1/1.25/1.5/2 and
  native-window CSS viewports 1000x750, 800x600, 667x500, and 500x375 while PNG
  physical dimensions stay approximately 1000x750. This directly exercises the
  app/WebView scaling pipeline without changing the protected OS display-scale
  setting; it is not evidence that the OS setting itself was changed.
- Accessibility and interaction: the real-app AX tree has 123 nodes, including
  one listbox, six options, nine buttons, two tabs, headings, navigation, and a
  grouped treemap. Native logs/screenshots prove English canceling, zh-CN
  completion and persistence, forced-colors rendering, list focus restoration,
  and cleared state after cancel-on-leave. Reduced-motion behavior is backed by
  the passing stylesheet contract test; a static image alone would not prove
  absence of animation.
- Partial/unknown/unsupported adjudication is unchanged from round 1: the native
  host conservatively reports the denied path as an available, unfollowed
  unsupported reparse leaf; the fake adapter separately proves
  `access_denied`, unknown, and partial lower-bound states. This respects the
  frozen no-follow design and does not collapse those typed states.

### Acceptance criteria — round 3

- **AC1 — PASS:** Deterministic zero/equal/extreme/tiny/resize behavior, exact
  `Other` reconciliation, unknown-node exclusion, 10,000-child cap, 200-row
  paging, snapshot parity, fixture hash, layout p95, and rectangle cap all pass.
- **AC2 — PASS:** The corrected desktop suite passes the canonical list,
  labelled rectangles, logical focus order/restoration, keyboard drill/down/up,
  no-cleanup surface, contrast, reduced-motion, and forced-colors contracts.
  Native focus and the 123-node AX tree independently support the keyboard and
  screen-reader alternatives.
- **AC3 — FAIL:** Bilingual state coverage, correct native locale captures,
  render budgets, responsive 1024/800/390 English evidence, 390 zh-CN evidence,
  and the accepted 100/125/150/200% app/WebView scaling matrix pass. The required
  current 1440 CSS-pixel width observation is absent and the files labelled
  `1440` are demonstrably the default 800x600 CSS-pixel window.
- **AC4 — FAIL:** Focused Rust gates, final desktop web log/build, current release
  binary, `just ci`, formatting, lint, type-check, and whitespace checks pass.
  The criterion also requires complete native visual evidence, so the missing
  true 1440-width capture prevents final closeout. The checker's independent
  desktop recipe failure is sandbox-only and is not counted as a product-gate
  failure.

### Verification command ledger — round 3

| Command | Exit | Result |
| --- | ---: | --- |
| `git status --short --branch` | 0 | Dirty scope inventoried; protected files preserved. |
| `python ./.trellis/scripts/get_context.py --mode packages` | 0 | Spec layers: backend, desktop-frontend, frontend. |
| `cargo test -p devsweep-cli tui::modes::analyze` | 0 | 8 passed. |
| `cargo test -p devsweep-core analysis` | 0 | 11 passed. |
| `npm --prefix desktop run lint` | 0 | Pass. |
| `npm --prefix desktop run typecheck` | 0 | Pass. |
| `git diff --check` | 0 | Pass. |
| `cargo fmt --all -- --check` | 0 | Pass. |
| `just ci` | 0 | 363 passed, 2 ignored; format/check/clippy passed. |
| `just desktop-web-check` | 1 | Checker sandbox: Vitest/Vite esbuild spawn is denied with `EPERM`. |
| Supplied `desktop-web-check-final.log` audit | 0 | 18/18 files, 121/121 tests, and production Vite build pass. |
| Raw p95/hash/console recomputation | 0 | Layout 1.9589 ms; commit 7.70 ms; caps and hashes pass. |
| PNG/log/executable/store/AX-tree consistency audit | 0 | Hashes, dimensions, probes, events, store, focus, cancel, and 123 AX nodes verified. |
| `node --check evidence/capture-native-analyze-round3.mjs` | 0 | Corrected capture driver parses. |
| Corrected native recapture | 1 | `spawnSync cmd.exe EPERM` before app launch; no new screenshot produced. |

No commit, push, dependency, cap, snapshot schema, frozen CLI/DTO contract,
`justfile`, protected dirty file, product source file, or other task directory was
modified in round 3. Overall verdict: **FAIL** until a fresh current 1440
CSS-pixel native capture is recorded with the corrected driver and audited.

---

## Verification round 4 — FAIL (2026-08-31)

Round 4 confirms that the corrected driver produced genuine current-width
captures from the release app and closed the round-3 1440 CSS-pixel geometry
gap. It also exposes a new evidence-integrity blocker: the reused main-session
profile was already `zh-CN`, and the driver does not force English before the
scenarios whose filenames and evidence index claim `en`. Product files and the
previously accepted code/gate conclusions are unchanged.

### Findings (fixed in round 4)

- File: `evidence/native-20260831-round3/screenshots/r3-01-analyze-complete-en-1440.png`,
  `r3-08-analyze-complete-zh-1440.png`
  - Issue: Round 3 had no current 1440 CSS-pixel native observation.
  - Fix: The corrected driver explicitly requested a 1440x860 CSS viewport.
    Both PNGs are 1800x1075 device pixels, exactly 1440x860 at the observed
    effective DPR 1.25. The 1024/800/390 captures are respectively
    1280/1000/488 device pixels wide and 1075 high, matching the same DPR (390
    multiplies to 487.5 and rounds to 488).

### Findings (not fixed in round 4)

- File: `evidence/capture-native-analyze-round3.mjs`,
  `evidence/native-20260831-round3/capture-log.jsonl`, and main-session
  screenshots labelled `en`
  - Issue: The driver reuses `native-20260831-round3/localappdata` and never
    selects English before `r3-01` through `r3-07`. The fresh log's `r3-01`
    summary is `分析完成…`, the canceling status is `正在分析…`, the AX-tree names
    are Chinese, and visual inspection confirms Chinese controls/content in
    `r3-01-analyze-complete-en-{1440,1024,800,390}.png` and
    `r3-02` through `r3-06`. The later `store_audit` remains
    `{"schema_version":1,"language":"zh-CN"}`. These artifacts are therefore
    genuinely rendered but incorrectly labelled and cannot substantiate the
    fresh English target-width/state claims carried forward from round 3.
  - Why not fixed: Correcting the evidence requires forcing English or using a
    genuinely fresh profile and then rerunning the native capture outside this
    checker sandbox. Relabelling the PNGs would not create the missing English
    observations. The round-4 dispatch authorizes only the audit/report update.
- File: `evidence/native-evidence.md:152-165`
  - Issue: The index still describes the overwritten `r3-01` width files and
    `r3-06` canceling capture as English and calls the AX tree English; the fresh
    raw log, AX tree, and visible screenshots contradict those statements.
  - Why not fixed: The round-4 scope explicitly requests the independent report
    update; this section records the correction without expanding into another
    evidence-index rewrite.

### Round-4 evidence audit

- Raw log: 45 parseable JSONL events, beginning with `exe_hash` and ending with
  one terminal `done`. Five unique app launches have five matching closes. Six
  scan summaries precede their captures; all 16 logged screenshot events have
  corresponding PNGs; four DPI probes, `store_audit`, focus, AX-tree, cancel,
  and return-after-cancel events are present in order.
- Executable: the logged SHA-256 and current
  `target/release/devsweep-desktop.exe` SHA-256 both equal
  `f977af3d7f16a0643b3bd9cf78a3c1d1e74c8f93ba2081b3aa76e0f197762b47`.
- Width matrix: 1440/1024/800/390 CSS widths map to
  1800/1280/1000/488 device pixels at effective DPR 1.25; all are 1075 device
  pixels high for 860 CSS pixels. The screenshots visibly contain the real
  Analyze toolbar, completion summary, breadcrumbs, treemap, and canonical
  accessible directory list.
- Scaling matrix: the probes remain DPR 1/1.25/1.5/2 with CSS viewports
  1000x750, 800x600, 667x500, and 500x375. Native-window PNGs are 1000x750,
  1000x750, 1001x750, and 1000x750; the single extra pixel at 150% is the
  expected rounding of 667 x 1.5 = 1000.5. These four captures are visibly
  English and preserve the round-3 accepted app/WebView scaling conclusion.
- Accessibility/lifecycle: the fresh AX tree contains 124 nodes, focus restores
  to `.analyze-listbox`, and cancel-on-leave returns `empty: true`. These facts
  preserve AC2 and the lifecycle portion of AC4, independent of the locale
  labelling defect.
- Four old `r3-dpi-*-...-1440.png` files remain in the directory but are not
  referenced by the fresh log; the authoritative fresh DPI artifacts use the
  `native-window` suffix as designed.

### Acceptance criteria — round 4

- **AC1 — PASS:** The deterministic bounded layout, exact `Other`, unknown-node
  exclusion, paging, snapshot parity, fixture hash, p95, and rectangle-cap
  conclusions are unchanged; no product or benchmark artifact changed.
- **AC2 — PASS:** The corrected desktop suite, keyboard focus restoration,
  accessible list/linked treemap, contrast, reduced-motion/forced-colors rules,
  no-cleanup surface, and fresh 124-node real-app AX tree remain sufficient.
- **AC3 — FAIL:** The render budget, true 1440/1024/800/390 geometry, zh-CN
  width captures, and 100/125/150/200% app/WebView scaling matrix pass. The
  files intended to prove fresh English target-width and state coverage were
  overwritten with Chinese renders, so the round-3 bilingual native-evidence
  conclusion no longer holds.
- **AC4 — FAIL:** All previously accepted Rust/desktop gates, release binary,
  executable hash, focus/cancel lifecycle, and native rendering remain valid.
  AC4 still cannot close because its required native visual evidence is
  internally mislabelled and lacks the fresh English target-width/state half.

### Verification command ledger — round 4

| Audit | Result |
| --- | --- |
| PNG `System.Drawing` dimensions and SHA-256 inventory | 20 PNGs inspected; all 16 fresh logged captures exist; width/DPR arithmetic above confirmed. |
| JSONL parse/order/count audit | 45 events; 5 launches/5 closes; 6 summaries; 16 screenshots; 4 DPI probes; terminal `done`. |
| Release executable SHA-256 | Matches the logged hash exactly. |
| `view_image` inspection | Real Analyze UI confirmed; `r3-01`/state captures labelled English are visibly Chinese, while DPI launches are English. |
| AX-tree parse | 124 nodes; names and labels are Chinese in the overwritten file named `analyze-complete-en-full-axtree.json`. |
| Cargo/npm/build gates | Not rerun by design; no product files changed since the accepted round-3 executions. |

No product, dependency, cap, schema, frozen contract, gate, or other task file
was modified in round 4. The only edit is this independent report. Overall
verdict: **FAIL**. Required closeout action: force the main native session to
English (or use a truly clean profile), rerun the corrected driver, and audit
that the files labelled `en` visibly and programmatically contain English while
retaining the now-correct 1440 CSS-pixel dimensions.

---

## Verification round 5 — PASS (2026-08-31)

Round 5 confirms that the corrected driver and final native rerun satisfy the
round-4 closeout action. The English-labelled main-session captures are now
English, the zh-CN captures remain Chinese, the true 1440/1024/800/390 geometry
and 100/125/150/200% app/WebView scaling evidence remain valid, and no product
source changed after the round-4 audit.

### Findings (fixed in round 5)

- File: `evidence/capture-native-analyze-round3.mjs`,
  `evidence/native-20260831-round3/capture-log.jsonl`, refreshed PNGs and AX tree
  - Issue: Round 4 found that the reused main profile remained zh-CN, so files
    labelled `en` visibly and programmatically contained Chinese.
  - Fix: Before the first Analyze capture, the driver now uses the real settings
    select and change event to request English, waits for persistence, and logs
    the resulting store as `{"schema_version":1,"language":"en"}`. It later
    switches through the same settings control to zh-CN and logs
    `{"schema_version":1,"language":"zh-CN"}` before r3-08. The final run
    refreshes all logged screenshots and the AX tree.
- File: `evidence/independent-check-report.md`
  - Issue: The current verdict and AC3/AC4 still reflected the round-4 locale
    mismatch.
  - Fix: Record this audit and update the current verdict and all acceptance
    criteria to PASS while retaining the earlier rounds as historical evidence.

### Findings (not fixed in round 5)

None. No blocking product or evidence defect remains in the dispatched scope.

### Round-5 evidence audit

- Raw log: 46 parseable JSONL events, beginning with `exe_hash` and ending with
  one terminal `done`; five unique app launches have five matching closes; all
  16 logged screenshot events have corresponding PNGs; six scan summaries and
  four DPI probes are present.
- Main English session: `force_en` records the persisted store as exactly
  `{"schema_version":1,"language":"en"}`. Its immediate `selectValue` field
  is still `zh-CN`, so that field alone is not treated as proof; the persisted
  store after the wait, the subsequent `Analysis complete...` summary, visible
  English PNG content, and refreshed English AX tree independently establish
  the settled locale. The log has one main scan summary tagged r3-01 rather
  than separate summary events for r3-02 through r3-07; those state captures
  occur in the same app session before the only zh-CN switch, and every one is
  visibly English.
- Visible English content: the four r3-01 width PNGs show `Analyze`,
  `Analysis complete`, `Current directory treemap`, and `Accessible directory
  list`; r3-02 through r3-07 visibly show English warnings, keyboard/focus,
  reduced-motion, forced-colors, canceling, and cleared-return states. The four
  DPI PNGs visibly show English, and their four scan summaries each begin
  `Analysis complete...`.
- Visible zh-CN content: the audited zh-CN store precedes the r3-08 summary
  `分析完成...`; both r3-08 1440 and 390 PNGs visibly contain Chinese controls,
  summary, treemap, and accessible-list copy.
- Accessibility and lifecycle: the refreshed AX tree has 123 nodes, contains
  `Accessible directory list` and `Analysis complete`, and contains zero CJK
  codepoints. Focus restores to `.analyze-listbox`; cancel-on-leave returns
  `empty: true`.
- Geometry: r3-01 English and r3-08 zh-CN 1440 PNGs are 1800x1075 device
  pixels, exactly 1440x860 CSS pixels at DPR 1.25. English 1024/800/390 widths
  are 1280/1000/488 device pixels; zh-CN 390 is 488 device pixels. DPI probes
  remain DPR 1/1.25/1.5/2 with CSS viewports 1000x750, 800x600, 667x500, and
  500x375; PNGs remain 1000x750, 1000x750, 1001x750, and 1000x750.
- Authenticity and scope: the current release executable SHA-256 matches the
  raw log exactly
  (`f977af3d7f16a0643b3bd9cf78a3c1d1e74c8f93ba2081b3aa76e0f197762b47`).
  Product-source files in the audited Analyze/TUI/desktop/i18n scope all
  predate the round-4 report; later material writes are confined to the native
  evidence driver, capture artifacts/profile/AX tree, evidence index, and
  dispatch records. The accepted round-3/4 product, safety, benchmark, focus,
  lifecycle, and gate conclusions therefore remain unchanged.

### Acceptance criteria — round 5

- **AC1 — PASS:** Deterministic zero/equal/extreme/tiny/resize behavior, exact
  `Other` reconciliation, unknown-node exclusion, 10,000-child cap, 200-row
  paging, snapshot parity, fixture hash, layout p95, and rectangle cap remain
  accepted and unchanged.
- **AC2 — PASS:** The corrected desktop suite, canonical list/linked treemap,
  focus restoration, keyboard drill/down/up, no-cleanup surface, contrast,
  reduced-motion/forced-colors rules, and refreshed 123-node English AX tree
  remain sufficient.
- **AC3 — PASS:** Automated bilingual state coverage, final English and zh-CN
  native captures, true 1440/1024/800/390 responsive geometry, accepted
  100/125/150/200% app/WebView scaling, and the recorded render budget all
  pass. The round-4 English-labelling blocker is closed.
- **AC4 — PASS:** Previously accepted Rust/desktop gates, release build and
  executable hash, `just ci`, lint, type-check, tests, focus/cancel lifecycle,
  AX tree, and complete final native visual evidence all pass. No product-side
  change requires a gate rerun in round 5.

### Verification — round 5

| Audit | Result |
| --- | --- |
| JSONL parse/order/count | PASS — 46 events; 5 launches/5 closes; 6 summaries; 16 screenshots; 4 DPI probes; terminal `done`. |
| Store and locale audit | PASS — English store before r3-01; zh-CN store before r3-08; summaries and visible content agree. |
| `view_image` inspection | PASS — all 16 logged PNGs inspected at original detail; English/zh-CN labels agree with their scenarios. |
| PNG dimensions and file inventory | PASS — every logged PNG exists; target-width and DPR arithmetic reconcile. |
| AX-tree parse | PASS — 123 nodes; English labels present; zero CJK codepoints. |
| Release executable SHA-256 | PASS — current executable matches the logged hash exactly. |
| Product-scope change audit | PASS — no product source is newer than the round-4 report; only evidence/dispatch artifacts changed afterward. |
| Lint / type-check / tests / builds | PASS from the accepted round-3 final logs; not rerun by design because round 5 changed no product source. |

No product, dependency, cap, schema, frozen contract, benchmark, or other task
scope was modified by this checker. Overall verdict: **PASS** with **AC1 PASS,
AC2 PASS, AC3 PASS, and AC4 PASS**.
