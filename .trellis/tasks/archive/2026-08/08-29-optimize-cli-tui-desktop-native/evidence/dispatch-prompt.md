Active task: .trellis/tasks/08-29-optimize-cli-tui-desktop-native

You are already the trellis-implement sub-agent. Implement directly. Do NOT spawn trellis-implement or trellis-check. Do NOT git commit, push, merge, or amend.

<!-- trellis-hook-injected -->
Hook injection did not fire; load context from the Active task path.

## Repo

D:\Documents\Code\Rust\Exp\devsweep — Windows PowerShell, branch `dev`.
HEAD includes archived core child `08-29-optimize-catalog-execution` (product `e36974e`, planning `19ad890`, archive `6861a65`). Do not reopen it. Do not reset.

This child owns Optimize **presentation** only: bilingual CLI renderer, TUI mode, Desktop IPC/workbench, fixtures, generated types, docs. Catalogue/authorizer/WOW64/URI/build query stay in core. Any id/URI/build/path change returns to the archived core child (do not patch core identity).

## Load first

1. This task's implement.jsonl then listed files. Catalogue design now lives at `.trellis/tasks/archive/2026-08/08-29-optimize-catalog-execution/design.md` (jsonl still has the pre-archive path).
2. This task `prd.md`, `design.md`, `implement.md`
3. `.trellis/spec/desktop-frontend/index.md` and `.trellis/spec/frontend/index.md` Pre-Development Checklists and the files they require
4. Shell-owned desktop spec (do not edit it): follow software-mode presentation child as the structural precedent (`crates/devsweep-cli/src/tui/modes/software/`, `desktop/src/modes/software/`, `desktop/src-tauri/src/software.rs`) without copying Software domain rules
5. Existing CLI renderer already committed: `crates/devsweep-cli/src/application/presentation/optimize.rs` and handler `commands/optimize.rs`. Extend the renderer if needed; do not rewrite core execution.

## Exact contracts

R1–R3 and AC1–AC4 in prd.md. Design: catalogue DTOs are the sole rendering source. Badges: Runs here / Opens Windows Settings / Guidance only. Eight ids only. Guidance has no run action. Settings buttons say open; terminal copy says launched, never completion. Only DNS says running/executing. Staged states: checking, ready, selected, previewing, preview-ready, confirming, running/launching, terminal/unknown. Sticky summary never calls Settings/guidance an optimization completion. Consume core typed build capability and refusal evidence including RtlGetVersion query failure; this task does not query OS build or add a fallback version helper.

Exact change list in design.md. Do not own parser grammar (`cli.rs`), catalogue, WOW64, URI decisions, or shell-owned desktop spec. Do not edit `commands/mod.rs` module root except if Optimize IPC registration on desktop `lib.rs` is required (that IS in the change list).

## Forbidden

- Do not modify `.trellis/.gitignore`, `README.md`, `justfile`, or other `.trellis/tasks/*` except this task directory.
- Do not `git add -f .trellis/`. Do not commit `target/` or `dist/`.
- No push/amend/publish/sign/release/credentials/paid services/user-global settings.
- No unapproved dependencies.
- Cleanup dry-run except the already-approved `ipconfig.exe /flushdns` and opening the three frozen Settings URIs. Do not change a Windows setting. Do not treat opened Settings as completed maintenance. Permanent delete disabled. Program/argv separate. No Docker cleanup. No UAC. No new catalogue ids.

## Required work

1. Implement the design change list. Generate Optimize DTOs via `desktop/scripts/generate-types.mjs` if that is the repo pattern. Add docs `docs/guide/optimize.md` and `docs/zh/guide/optimize.md`.
2. Mode-local TUI/Desktop flows with the staged state machine, shell coordinator cancel/join, operation-id stale rejection.
3. Focused gates — save raw logs with command + real exit code under this task's `evidence/`:
   - `rtk cargo test -p devsweep-cli optimize`
   - `rtk cargo test -p devsweep-desktop optimize`
   - `rtk cargo test -p devsweep-cli tui::modes::optimize`
   - `rtk just desktop-web-check`
   - `rtk just desktop-test` if applicable
   - `rtk just desktop-build`
   - `rtk git diff --check`
   Do not change protected `justfile`; if `just desktop-web-check` chains oddly, also run the underlying npm commands and record both.
4. Native evidence required by implement.md (cannot finish with completion-required UNVERIFIED):
   - Both languages, target widths/scales, keyboard/screen-reader, reduced motion
   - unsupported builds, policy failure, stale/cancel/timeout/unknown, audit history
   - exact DNS program/argv/process/integrity/no-UAC
   - **all three** literal Settings pages; record `launched`, never completion
   Save commands, exit codes, logs, hashes, PID/HWND, screenshots, residue samples.
   After identity-verified task-owned SystemSettings/WebView/desktop processes, close gracefully; force-kill only after path+PID match.
5. Write `evidence/verification-report.md` (implementer evidence only) and append implement.jsonl notes. Copy this dispatch into `evidence/dispatch-prompt.md`.

## Report

Implementation Complete format: files modified, summary, each command+exit+log path, UNVERIFIED ledger with completion-required yes/no. Do not commit.
