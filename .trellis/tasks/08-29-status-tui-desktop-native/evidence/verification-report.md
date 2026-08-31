# Status TUI/Desktop/native — implementer verification report

Implementer evidence only. This is **not** an independent trellis-check PASS.

Task: `.trellis/tasks/08-29-status-tui-desktop-native`
Date: 2026-08-31
Host: Windows 11 build 26200, x64, Medium integrity (`S-1-16-8192`), `consent.exe` count 0. No UAC, no OS global scale change, no new production dependencies, no `cli.rs` / justfile / README / collector edits.

This child owns Status **presentation** only. Collectors stay in archived `08-29-status-collector-cli`.

## What shipped

- TUI Status mode: snapshot-on-enter, explicit live, interval steps, 60-point gapped charts, privacy-safe process table, cancel/leave drop.
- Desktop Tauri adapter (`status_snapshot` / `status_live_start` / `status_cancel`) plus closed-world generated types and `exact()` decoders.
- Desktop workbench registered in `App.tsx` (there is no `desktop/src/app-shell/registry.ts`; shell already lists `status` in `MODE_IDS`).
- Native validation matrix: `docs/validation/status-native.md`.

## R / AC trace

| Clause | Result | Evidence |
| --- | --- | --- |
| R1 / AC1 snapshot first, explicit live, 60-point cap, drop on leave | PASS | TUI reducer tests (`02-rtk-cli-tui-status.log`, 10 passed). Desktop reducer + workbench (`04e-npm-test.log`). Native keyboard Space on Start live then Stop live; leave `#/clean` then restart `#/status`. |
| R2 / AC2 frozen V1, gaps not zeros, no GPU zero cards, privacy | PASS | Fixtures/decoders reject unknown fields and `gpu`. Native body/AX have no `cmdline` / `gpu 0`. Capability note rendered EN/ZH. Charts use `gap` / `缺口` text alternatives. |
| R3 / AC3 coordinator exclusivity, cancel-and-join | PASS | `kind: "status"` on shell coordinator; AppShell `cancelAndJoin` on route change; Tauri `StatusAlreadyRunning`; pause/leave/close cancel+join in TUI jobs and desktop `status_cancel`. |
| R4 / AC4 bilingual a11y + native resource | PASS | Native EN/ZH 390–1440, DPI 100–200 via `--force-device-scale-factor`, reduced-motion + forced-colors, 792-node AX tree. Resource gate all true (`native-desktop/resource/summary.json`). |

## Focused gates

| Command | Exit | Log |
| --- | --- | --- |
| `cargo test --locked -p devsweep-cli -- status` | 0 (15 lib + 1 cli_contract) | `evidence/01-rtk-cli-status.log` |
| `cargo test --locked -p devsweep-cli -- tui::modes::status` | 0 (10 passed) | `evidence/02-rtk-cli-tui-status.log` |
| `git diff --check` | 0 | `evidence/03-rtk-git-diff-check.log` |
| `just desktop-web-check` | 0 | `evidence/04-rtk-just-desktop-web-check.log` |
| `npm run types:generate` | 0 | `evidence/04b-npm-types-generate.log` |
| `npm run lint` | 0 | `evidence/04c-npm-lint.log` |
| `npm run typecheck` | 0 | `evidence/04d-npm-typecheck.log` |
| `npm test` | 0 (157 passed) | `evidence/04e-npm-test.log` |
| `npm run build` | 0 | `evidence/04f-npm-build.log` |
| `just desktop-test` | 0 | `evidence/05-rtk-just-desktop-test.log` |
| `just desktop-build` | 0 | `evidence/06-rtk-just-desktop-build.log` |
| `node evidence/capture-native-status.mjs` | 0 | `evidence/08-native-capture.log` |
| `node evidence/resource-gate.mjs` | 0 | `evidence/09-resource-gate.log` |

`just desktop-web-check` is one semicolon-chained recipe; underlying npm commands were also recorded separately. `justfile` was not modified.

## Native GUI (isolated `LOCALAPPDATA`, release `devsweep-desktop.exe`)

Binary sha256 `9B4EBB3F8E95326C5663065F6150979A63D94C9CC4C575B3A4FEF9CC16FA0185`. Integrity Medium. `consent.exe` none. Device scale via WebView2 `--force-device-scale-factor` only.

| Check | Artifact |
| --- | --- |
| EN widths 1440/1024/800/390 | `native-desktop/screenshots/status-snapshot-en-{width}.png` (innerWidth matched) |
| ZH widths 1440/1024/800/390 | `status-snapshot-zh-*.png`; store `{"schema_version":1,"language":"zh-CN"}` |
| Keyboard Start live (Space) | capture-log `keyboard_focus_start_live` + `live_started_keyboard` status=`live` |
| Screen reader | `native-desktop/axtree/status-en-full-axtree.json` (792 nodes; Stop live / Refresh snapshot / capability note; no cmdline) |
| Reduced motion | `status-reduced-motion-en.png` |
| High contrast | `status-high-contrast-en.png`; probe `forced:true` |
| Explicit live / interval 5 s | `status-live-started-en.png`, `status-interval-change-en.png` interval=`5000` |
| Process churn / truncation | 15 process rows, truncated-by-limit on host; no path/cmdline/user |
| Leave / restart / close | capture-log events `leave`, `restart`, `close` |
| DPI 100/125/150/200 | `status-dpi-{percent}-en.png`; dpr 1 / 1.25 / 1.5 / 2 |

## Resource gate (same release binary, PID 27688)

Raw samples: `native-desktop/resource/{idle,live-60s,post-exit}.json`.

| Gate | Measured | Limit | Pass |
| --- | --- | --- | --- |
| Snapshot p95 | 820 ms (5 refresh: 812–820) | <= 2000 ms | yes |
| Live private peak | 7 278 592 | idle median 6 172 672 + 64 MiB | yes |
| Live threads peak | 37 | idle median 36 + 4 | yes |
| Live CPU median | 0% of one core | <= 5% | yes |
| Live CPU p95 | 7.776% of one core | <= 15% | yes |
| Post-exit | 25 samples @ 200 ms; final five `present=false` | 25 samples; both caps in final five; no missing sample | yes |

Live started only from the explicit Start live control. No background live.

## Remaining UNVERIFIED

| Clause | Completion-required? | Notes |
| --- | --- | --- |
| Human Narrator listen-through | No | AX tree + labelled controls + `aria-live` match Software/Optimize presentation evidence. |
| OS-global display scale 125/150/200 | No | Forbidden to change user display settings. Device scale was emulated on WebView2. |
