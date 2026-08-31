# Optimize CLI/TUI/Desktop — implementer verification report

Implementer evidence only. This is **not** an independent trellis-check PASS.

Task: `.trellis/tasks/08-29-optimize-cli-tui-desktop-native`
Date: 2026-08-31
Host: Windows 11 build 26200, x64, Medium integrity (`S-1-16-8192`), `consent.exe` absent. No signing, no UAC, no new dependencies, no catalogue/URI/build/path changes.

This child owns Optimize **presentation** only. Catalogue identity, WOW64, URI allowlist, and `RtlGetVersion` stay in archived core `08-29-optimize-catalog-execution`.

## What shipped

- Bilingual CLI renderer over catalogue DTOs: badges **Runs here** / **Opens Windows Settings** / **Guidance only**.
- TUI mode with staged states checking → ready → selected → previewing → preview-ready → confirming → running|launching → terminal|unknown; stale operation-id rejection; shell cancel/join.
- Desktop IPC (`optimize.rs` + `lib.rs`), generated types, fixture-bridge, reducer/workbench.
- Docs: `docs/guide/optimize.md`, `docs/zh/guide/optimize.md`.
- Sticky summary never calls Settings launch or guidance an optimization completion. Guidance has no run action. Settings buttons say open; terminal copy says launched. Only DNS uses running.

## R / AC trace

| Clause | Result | Evidence |
| --- | --- | --- |
| R1 / AC1 eight ids, classes, refusals, digest, terminals | PASS | CLI list 8 rows (`native/01-list-en.log`, `native/01-list-zh.log`). Desktop fixtures/parity (`desktop/src/modes/optimize/parity.test.ts`). TUI closed-set reducer tests. Guidance plan exit 6, no file (`native/02-guidance-refusal.log`). |
| R2 / AC2 badges, no guidance run, Settings open/launched, DNS running, confirm, stale | PASS | CLI/TUI/Desktop tests. Native stale digest exit 3 then confirmed DNS succeeded / Settings launched. Workbench hides preview/run for guidance. Confirm dialog uses **Open Windows Settings** not **Run DNS flush**. |
| R3 / AC3 bilingual layout, keyboard/SR, reduced motion, widths, native process/no-UAC | PASS | TUI TestBackend 40/80/100/120 (`03-rtk-cli-tui-optimize.log`). Vitest EN/ZH workbench + radio `aria-label` + `role=status` + reduced-motion CSS. Native Medium integrity, `consent.exe` none. |
| AC4 gates + native DNS + **all three** Settings pages | PASS | Focused gates below. DNS succeeded. `settings.search`, `settings.storage_recommendations`, `settings.energy_recommendations` each **1 launched**, never completion. Journal redacted. |

## Focused gates

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test -p devsweep-cli optimize` | 0 (16 passed) | `evidence/01-rtk-cli-optimize.log` |
| `rtk cargo test -p devsweep-desktop optimize` | 0 (3 passed) | `evidence/02-rtk-desktop-optimize.log` |
| `rtk cargo test -p devsweep-cli tui::modes::optimize` | 0 (6 passed) | `evidence/03-rtk-cli-tui-optimize.log` |
| `rtk just desktop-web-check` | 0 | `evidence/04-rtk-just-desktop-web-check.log` |
| `npm run types:generate` (underlying) | 0 | `evidence/04b-npm-types-generate.log` |
| `npm run lint` | 0 | `evidence/04c-npm-lint.log` |
| `npm run typecheck` | 0 | `evidence/04d-npm-typecheck.log` |
| `npm test` | 0 (146 passed) | `evidence/04e-npm-test.log` |
| `npm run build` | 0 | `evidence/04f-npm-build.log` |
| `rtk just desktop-test` | 0 | `evidence/05-rtk-just-desktop-test.log` |
| `rtk just desktop-build` | 0 | `evidence/06-rtk-just-desktop-build.log` |
| `rtk git diff --check` | 0 | `evidence/07-rtk-git-diff-check.log` |

`just desktop-web-check` chains `types:generate; lint; typecheck; test; build` as one recipe. Underlying npm commands were also recorded separately. `justfile` was not modified.

## Native x64 (isolated `LOCALAPPDATA`)

Override: `evidence/native/lappdata`. Binary `target\debug\devsweep.exe` PE amd64 sha256 `3CB2058FB642AA6F09D720337B7866CB995F5BCFE3281A4BD787F699E2E2503A`. Integrity Medium. `consent.exe` none before/after every run.

| Operation | Exit | Outcome | Notes |
| --- | --- | --- | --- |
| `optimize list` EN | 0 | 8 rows, three badges | `native/01-list-en.log` |
| `optimize list` zh-CN | 0 | 在此运行 / 打开 Windows 设置 / 仅指引 | `native/01-list-zh.log` (UTF-8 recapture) |
| `optimize plan --operation guidance.drive_optimize` | 6 | never dispatched | no plan file |
| `dns.flush` stale digest | 3 | `invalid_optimize_authority` | then matching digest |
| `dns.flush` confirmed | 0 | **Succeeded** (not launched) | digest `sha256:3deb8ab1399acf3f832ac4886c2e1b646ef3d7a9f6c2035283c4a8961d926b1d` |
| `settings.search` confirmed | 0 | **Launched** | digest `sha256:7307cb96…`; SystemSettings `C:\Windows\ImmersiveControlPanel\SystemSettings.exe` |
| `settings.storage_recommendations` confirmed | 0 | **Launched** | digest `sha256:66f87dbc…`; new PID 74316, EnumWindows HWNDs recorded, identity-matched force-kill after CloseMainWindow failed |
| `settings.energy_recommendations` confirmed | 0 | **Launched** | digest `sha256:53ee66ae…`; new PID 25756, same close protocol |

Copy on every Settings run: “A launched Settings page is not maintenance completion.” No “optimization complete”.

Audit journal sha256 `D88F8EAC2FE69382ED2B6F9B604C519DF5A2A0772C40383121C944D678672963`, `REDACTED_OK` (no `ipconfig` / `ms-settings` / `program` / `argv`). Sample: `evidence/native/07-audit-journal.log`. DNS argv identity remains core-owned and is not reconstructed here.

## Remaining UNVERIFIED

| Clause | Completion-required? | Blocker |
| --- | --- | --- |
| Native live WMI sample of `ipconfig.exe` process tree | No | Child lifetime is shorter than WMI poll. Presentation redacts program/argv; core runner still uses separate program/argv. |
| Native unsupported-build Settings refusal | No | Host RtlGetVersion build 26200 ≥ 22624. This child must not guess a version or add a fallback helper. Fixture `optimize_unavailable` + decoder tests cover query failure copy. |
| Native CLI cancel-before / timeout → unknown | No | Frozen grammar has no cancel flag; real `flushdns` finishes inside 10 s. TUI/Desktop reducers and fixtures cover cancel/stale/unknown. |
| Native Narrator recording and OS display-scale change | No | Forbidden to change user-global display settings. Keyboard/SR covered by labelled radios, `aria-live`, focusable dialog; widths by TUI TestBackend + CSS 430/800 and shell 520/800/1024/1440. |
| Native desktop WebView screenshots at 390/800/1024/1440 CSS px | No | Workbench Vitest + production CSS. `desktop-build` produced `target\release\devsweep-desktop.exe` (exit 0). Opening extra Settings pages for screenshots would still not be completion. |
