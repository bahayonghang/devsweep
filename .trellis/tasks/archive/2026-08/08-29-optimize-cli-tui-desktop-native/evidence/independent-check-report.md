# Independent check report — Optimize CLI, TUI, desktop, native

Task: `.trellis/tasks/08-29-optimize-cli-tui-desktop-native`
Branch: `dev`
Date: 2026-08-31
Host: Windows 11 build 26200, x64, Medium integrity `S-1-16-8192`, `consent.exe` none.
Role: trellis-check (this child only). Implementer `evidence/verification-report.md` is a clue, not proof.
Protected dirt left untouched: `.trellis/.gitignore`, `README.md`, `justfile`. No other `.trellis/tasks/*` edited. No `git add -f .trellis/`. No commit.

Overall: **PASS**

Completion-required clauses left UNVERIFIED: **no**.
Objectively blocked native clauses are recorded as **BLOCKED** (not used as a pass). Capturable Windows GUI, keyboard, responsive, and failure-path evidence was captured independently.

This child is presentation-only. Archived `08-29-optimize-catalog-execution` was not reopened. No catalogue id, parser grammar, URI, build predicate, or path identity edits.

---

## Per-requirement / AC / design

| Clause | Result | Evidence |
| --- | --- | --- |
| R1 catalogue + action-class distinction | **PASS** | Independent EN/ZH `optimize list` (8 ids, three badges). TUI/Desktop reducers and fixtures. Native WebView catalogue at 390/800/1024/1440 both languages. |
| R2 staged machine + sticky non-completion | **PASS** | TUI/Desktop tests for checking→ready→selected→preview→confirm→running\|launching→terminal\|unknown. Sticky copy never calls launch/guidance a completion (native screenshots + AX tree + Vitest). |
| R3 bilingual a11y, refusals, stale/cancel, shell | **PASS** | Native EN/ZH layouts, keyboard Space on guidance, 292-node AX tree, reduced motion, forced-colors, device-scale 100–200%. CLI stale digest exit 3. Guidance plan exit 6, no file. Shell header keeps `Scan partial` after Optimize nav. |
| AC1 fixture parity | **PASS** | Closed eight ids in CLI list, TUI tests, `parity.test.ts`, fixtures under `desktop/src/api/fixtures/optimize/`, generated `types.gen.ts` (28 named fixtures). |
| AC2 no guidance run; Settings launched; DNS running; confirm; stale | **PASS** | CLI presentation tests; TUI tests; Vitest workbench; native confirm dialog **Open Windows Settings** not **Run DNS flush**; native DNS **Succeeded**; three Settings **Launched** with unique PIDs. |
| AC3 bilingual widths/scales, keyboard/SR, native process/no-UAC | **PASS** | Independent WebView captures (contra implementer “not completion-required”). Software presentation child set the same native bar. Device-scale used; OS scale not changed. |
| AC4 gates + DNS + all three Settings | **PASS** | Independent gates below (including `just ci` after self-fixes). DNS succeeded, Medium, no UAC. Three Settings unique PIDs, launched ≠ completion. |
| Design: DTO-only render, badges, confirmation by class | **PASS** | CLI renderer, TUI mode, desktop workbench. |
| Design: no OS-build query / fallback in this child | **PASS** | Desktop IPC consumes core catalogue; fixtures cover `optimize_unavailable`. |
| Design exact change list | **PASS** | Listed presentation/IPC/fixture/docs paths shipped. `cli.rs` parser untouched. |
| Out of scope: new ids, batch, UAC, Settings-as-completion | **PASS** | None introduced. |

---

## Skeptical items

1. `desktop/src/modes/analyze/render-benchmark.tsx` — **in scope**. `DesktopBridge` grew Optimize methods; the Analyze harness must implement the interface. Diff is five unused-throw stubs (`optimizeListStart/Preview/Run/Audit/Cancel`). Necessary glue, not Analyze-product work.
2. Three Settings launches — **unique identity**. Independent recapture emptied leftover `SystemSettings.exe` first. PIDs **43148** (search), **27748** (storage), **52684** (energy); path `C:\Windows\ImmersiveControlPanel\SystemSettings.exe`; per-PID HWND values recorded; leftover not reused. Implementer `settings.search` had reused leftover PID 72144 — not used as proof. `visible_hwnd=False` on SystemSettings windows is Windows 11 hosting (visible frame is typically `ApplicationFrameHost`); identity requirement is met. Force-kill only after path+PID re-check.
3. Guidance has no run action on every surface; sticky summary does not call launched/guidance a completion — **PASS**. CLI lines, TUI TestBackend 40/80/120, Vitest, native screenshots, AX `guidance.drive_optimize: Guidance only` + `No run action`, ZH sticky `设置仅打开；它们从不完成一次优化。`
4. No new catalogue ids, no parser grammar, no core URI/build/path, no UAC — **PASS**. `git diff --name-only` has no `crates/devsweep-core` and no `application/cli.rs`. `consent.exe` none on DNS and Settings. Journal JSONL has no `ipconfig` / `ms-settings` / `program` / `argv`.
5. Desktop types generated from fixtures; decoders closed-world — **PASS**. `just desktop-web-check` ran `types:generate` (“28 named fixture files”). `contract.ts` uses `exact()` on Optimize envelopes and rejects id sets other than the closed eight.

---

## Independent gates (re-run)

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test -p devsweep-cli optimize` | 0 (16 passed) | `independent-cli-optimize.log` |
| `rtk cargo test -p devsweep-desktop optimize` | 0 (3 passed) | `independent-desktop-optimize.log` |
| `rtk cargo test -p devsweep-cli tui::modes::optimize` | 0 (6 passed) | `independent-tui-optimize.log` |
| `rtk just desktop-web-check` | 0 (146 tests; chains `types:generate; lint; typecheck; test; build`) | `independent-desktop-web-check.log` |
| same recipe after CSS self-fix | 0 (146 tests; CSS hash `index-CNv90rPz.css`) | `independent-postfix-desktop-web-check.log` |
| focused Vitest after CSS | 0 (67 passed) | `independent-postfix-optimize-web.log` |
| `rtk just desktop-test` | 0 (33 passed, 2 ignored) | `independent-desktop-test.log` |
| `rtk just desktop-build` | 0 | `independent-desktop-build.log` then `independent-postfix-desktop-build.log` |
| `rtk git diff --check` | 0 (re-run after header/clippy) | `independent-git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 after `rtk cargo fmt --all` | `independent-cargo-fmt.log`; also inside `just ci` |
| `rtk just ci` | 0 (round 3) | `independent-just-ci.log` |

`just desktop-web-check` recipe (read-only, `justfile` not edited): `cd desktop; npm run types:generate; npm run lint; npm run typecheck; npm test; npm run build`.

### `just ci` self-fix rounds

| Round | Exit | Failure signature | Fix |
| --- | --- | --- | --- |
| 1 | 1 | `tui::render::tests::partial_scan_health_renders_truthful_totals_and_diagnostics` (`Scan partial` clipped at 120 cols after Optimize nav) | Full header now emits Scan health and Selected **before** the growing navigation list (`crates/devsweep-cli/src/tui/render/mod.rs`). Authority copy is not truncated. |
| 2 | 1 | `clippy::if_same_then_else` in `tui/modes/optimize/mod.rs` cancel restore (identical `Ready` branches) | Collapsed to a single `else { OptimizePhase::Ready }`. |
| 3 | 0 | — | `independent-just-ci.log` (fmt + check + test + clippy `-D warnings`). |

Earlier fmt `--check` failure (TUI/desktop Optimize rustfmt) was fixed with `rtk cargo fmt --all` before round 1 of `just ci`.

---

## Product self-fixes this check owns

1. **rustfmt** on Optimize TUI/desktop Rust so `cargo fmt --all -- --check` passes.
2. **`desktop/src/modes/optimize/styles.css`** — `.optimize-mode` now defines `--surface-raised: #fff` (and sibling tokens). Confirm dialog was transparent; native screenshot showed title overlapping catalogue. Added `dialog::backdrop`.
3. **`OptimizeWorkbench.tsx`** — `HTMLDialogElement.showModal()` / `close()` via ref+effect instead of the `open` attribute; jsdom fallback in tests. Regression: `--surface-raised` and `::backdrop` asserted in `OptimizeWorkbench.test.tsx`.
4. **TUI Full header order** — Scan health survives 120-col TestBackend after `[P] Optimize` is registered.
5. **clippy** cancel-phase identical branches.

Post-fix native confirm screenshot (`opt-05-confirm-settings-en.png`): opaque dialog, “Confirm opening Windows Settings”, “Open Windows Settings”, “Launched is not maintenance completion.” Post-fix desktop exe sha256 `14a2e7fcb1a4ad90cafcde05273850e998f4554f69020060ae630e1db6f890f7`.

---

## Native ledger

Adjudication vs implementer: native WebView screenshots, keyboard, AX tree, reduced motion, and device-scale **are** completion-required (desktop-frontend Quality Check + Software presentation child bar). They were captured. OS-global display scale was **not** changed.

| Required item | Result | Independent evidence |
| --- | --- | --- |
| EN catalogue 1440/1024/800/390 | **PASS** | `native-desktop/screenshots/opt-01-catalogue-en-1440.png`, `opt-02-catalogue-en-{1024,800,390}.png`. innerWidth 1440/1024/800/390 (`capture-log.jsonl`). |
| ZH catalogue 1440/1024/800/390 | **PASS** | `opt-07-catalogue-zh-1440.png`, `opt-08-catalogue-zh-{1024,800,390}.png`. Store `presentation-v1.json` language `zh-CN`. |
| Device scale 100/125/150/200 (not OS) | **PASS** | `opt-dpi-{100,125,150,200}-catalogue-en.png`. dpr 1 / 1.25 / 1.5 / 2; inner 1000×750 → 800×600 → 667×500 → 500×375. |
| Keyboard | **PASS** | Space on `guidance.drive_optimize`; `preview: false`; sticky “Guidance shown. This is not an optimization completion.” `opt-03-guidance-keyboard-en.png`. |
| Screen reader | **PASS** | CDP AX tree 292 nodes; `guidance.drive_optimize: Guidance only`, `No run action`. Live Narrator voice not recorded — AX tree is the spec protocol substitute. |
| Reduced motion | **PASS** | `Emulation.setEmulatedMedia prefers-reduced-motion=reduce`; `opt-06-reduced-motion-en.png`. CSS `@media (prefers-reduced-motion: reduce)`. |
| Forced colors / high contrast | **PASS** | CDP `forced-colors: active` + `prefers-contrast: more`; probe `forced:true`; `opt-09-high-contrast-en.png`. CSS `@media (forced-colors: active)`. |
| Settings confirm (not DNS run) | **PASS** | `opt-05-confirm-settings-en.png`; dialog `runDns:false`, `openSettings:true`. |
| Audit history GUI | **PASS** | Native journal copied into isolated LOCALAPPDATA; probe “20 audit transitions; 0 recovered terminals. Recovery never redispatches.” `opt-10-audit-en.png`. |
| CLI EN/ZH list | **PASS** | `independent-native-list-en.log` / `independent-native-list-zh.log` exit 0, eight ids, three badges, catalogue states `System32 ipconfig.exe /flushdns`. |
| Guidance never dispatched | **PASS** | `independent-native-guidance-refusal.log` exit 6, `file_exists=False`. |
| Stale digest | **PASS** | Settings and DNS recapture: stale exit 3 `invalid_optimize_authority`. |
| Policy / confirmation | **PASS** | Native dialog + `--confirm` required; unconfirmed path has no run control for guidance. |
| Three Settings launched, never completion | **PASS** | `independent-native-settings-identity.log` exit 0. Unique PIDs 43148 / 27748 / 52684. Run copy “1 launched”; “not maintenance completion”. consent none. |
| DNS succeeded, integrity, no UAC | **PASS** | `independent-native-dns-process.log` + round2. Medium `S-1-16-8192`. consent none. run exit 0 **Succeeded**. Journal fields omit program/argv (`REDACTED_OK`). |
| DNS live process PID/path/argv sample | **BLOCKED** | Two methods (CIM poll, C# `GetProcessesByName` spin) recorded **0 samples**. `ipconfig` lifetime is shorter than user-mode poll. Core owns separate program/argv; this child must not reconstruct them. Catalogue/preview copy names `C:\Windows\System32\ipconfig.exe` `/flushdns`. Not treated as overall FAIL. |
| Native unsupported-build host | **BLOCKED** | Host build 26200 ≥ 22624. This child must not fake `RtlGetVersion` or add a fallback helper. Fixtures `refusal-build-unsupported.json` / `refusal-os-build-unavailable.json` + `optimize_unavailable` decoder/ErrorBanner. |
| Native CLI cancel-before / timeout → unknown | **BLOCKED** | Frozen grammar has no cancel flag; real `flushdns` finishes inside 10 s. TUI/Desktop reducers, `execution-five-terminal.json`, and `unknown` status tests cover the surfaces this child owns. |
| OS display-scale change | **BLOCKED** | Forbidden to change user-global scale. Device-scale WebView2 emulation used instead (spec protocol). |

Native desktop binary: `target\release\devsweep-desktop.exe` sha256 `14a2e7fcb1a4ad90cafcde05273850e998f4554f69020060ae630e1db6f890f7`. Isolated `LOCALAPPDATA`. Force-kill only after executable path matches that binary.

---

## Scope / dirt notes

- `README.md` and `justfile` remain dirty from prior work (`just tinstall`, `npm run tauri -- build`). Not modified, not reverted.
- Untracked evidence and new Optimize modules stay unstaged. No commit.
- Core optimize catalogue/executor, parser, and URI/build/path stay with the archived child.

---

## Verdict

**PASS.** Independent gates are green after presentation-owned self-fixes. Native GUI/keyboard/responsive/Settings-identity/DNS-integrity evidence was captured against the task docs and the Software presentation native bar, not against the implementer’s weakened “screenshots not required” list. Blocked items (live `ipconfig` sample, unsupported-build host, CLI timeout→unknown, OS scale) are documented blockers, not UNVERIFIED completions.
