# trellis-implement dispatch — 08-29-optimize-catalog-execution

Active task: .trellis/tasks/08-29-optimize-catalog-execution
Source: session:cursor-goal-mole-redesign-20260831
Date: 2026-08-31
Role: trellis-implement (Cursor Task `implementer`; you ARE this agent)

Do not spawn trellis-implement or trellis-check. Do not commit or push.

## Repo and role

- Repo: D:\Documents\Code\Rust\Exp\devsweep (Windows, PowerShell, branch `dev`)
- HEAD at dispatch: 56597a4 (ancestor of baseline d79041fe). Do not reset.
- This child owns the closed Optimize V1 catalogue, authorizer, Windows adapters, execution, durable audit, and frozen CLI `optimize list|plan|preview|run` handler.
- Revised task docs are the requirement source of truth. Do not edit review reports. Do not reopen archived tasks.

## Context to read first (in this order)

1. `.trellis/tasks/08-29-optimize-catalog-execution/implement.jsonl` then each listed file
2. `.trellis/tasks/08-29-optimize-catalog-execution/prd.md`
3. `.trellis/tasks/08-29-optimize-catalog-execution/design.md`
4. `.trellis/tasks/08-29-optimize-catalog-execution/implement.md`
5. `.trellis/spec/backend/index.md` and its Pre-Development Checklist files that apply (quality, directory, error, logging, database/persistence)
6. `.trellis/spec/guides/index.md`
7. `.trellis/tasks/08-29-mole-inspired-cli-windows-desktop-redesign/research/windows-optimize-boundary.md`
8. Current uncommitted product files under `crates/devsweep-core/src/optimize/`, `crates/devsweep-cli/src/application/commands/optimize.rs`, presentation/i18n wiring, and `crates/devsweep-core/Cargo.toml` / `lib.rs`

## Existing WIP (do not throw away)

Substantial uncommitted implementation already exists and prior focused/native logs live under the task `evidence/` directory. Treat that as starting point: close remaining contract gaps; do not rewrite a working closed catalogue unless evidence proves it violates prd/design/implement.

Known prior notes (verify, do not assume PASS):
- Eight-id V1 catalogue, MaintenancePlanV1, digest, lock, audit, RtlGetVersion, WOW64/Sysnative, DNS via bounded runner, Settings ShellExecuteExW launched-only, CLI handler wired.
- Native Settings confirmed-run previously ended `launched=0 failed=1 exit=6` (policy/launch failure). implement.md still requires native records of exact Settings page launch AND policy/launch failure. Diagnose whether this is adapter/lifecycle (fix product) vs host policy on unsigned bins (capture exact SE_ERR/HRESULT, PID/HWND, no UAC, do not sign, do not claim launched). If a successful `launched` handoff is possible without signing/elevation/new deps, capture it. If objectively blocked, record that as native policy-failure evidence with commands/exit codes; do not invent success.
- x86 Sysnative arm already has a log; keep or recapture if the binary/path identity is stale.
- Focused filters: design.md uses a single `tests.rs`, so `optimize::tests::catalogue` (etc.) is the honest filter; verbatim `optimize::catalogue` may match zero tests. Record the exact command that actually ran.

## Exact contracts (must all be true)

R1–R6 and AC1–AC4 in prd.md. Design table of exactly eight ids. DNS algorithm IsWow64Process2 → Sysnative/GetSystemDirectoryW, no PATH/env/SysWOW64/canonicalization. Settings only three literal URIs; ShellExecuteExW verb open, SEE_MASK_FLAG_NO_UI; success is `launched` never completion. Guidance never in dispatch enum. No CleanupPlan. No UAC. Cancel before dispatch = canceled; after = observed or unknown_after_dispatch. Durable locked V1 journal; no localized/system output as identity.

Cargo.toml: keep existing Win32_System_Threading and add exactly `Wdk_System_SystemServices`, `Win32_System_SystemInformation`, `Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging`. Do not add unapproved dependencies.

CLI: fill frozen `optimize.rs` handler. Minimal `commands/mod.rs` dispatch branch `optimize.` is allowed wiring (do not restructure the module root; do not edit `cli.rs` parser grammar). Presentation/i18n keys for optimize.v1.* with en/zh-CN parity are in scope. The `cli_contract.rs` / staged-command test swap from optimize→status is required because optimize is now wired; do not revert it to unwired-optimize.

## Forbidden

- Do not modify, stage, or revert: `.trellis/.gitignore`, `README.md`, `justfile`, other `.trellis/tasks/*` except this task directory.
- Do not `git add -f .trellis/`. Do not commit `target/` or `dist/`.
- Do not push, amend, publish, sign, create a release, touch credentials/production/paid services, or change user-global settings.
- Do not install dependencies not approved by this task's docs.
- Cleanup remains dry-run except this task's separately approved `ipconfig.exe /flushdns` and opening the three frozen Settings URIs. Do not change a Windows setting. Do not treat an opened Settings page as completed maintenance. Permanent delete disabled. Scanner/model only create plans. Program and argv stay separate. Cargo home inspect-only. No Docker cleanup.
- Do not expand beyond this child's scope (no TUI/Desktop Optimize UI; that is the sibling `08-29-optimize-cli-tui-desktop-native`).

## Required work this turn

1. Close remaining product gaps vs prd/design/implement. Prefer smallest contract-correct change.
2. Run focused gates from implement.md (use `rtk` when it exists at C:\Users\lyh\.cargo\bin\rtk.exe). Save full raw logs with command line and real exit code under `.trellis/tasks/08-29-optimize-catalog-execution/evidence/`.
3. Native x64 (and x86-on-x64 if still buildable) evidence.
4. Append implement.jsonl notes. Write `evidence/verification-report.md`.
5. Copy this dispatch into `evidence/dispatch-prompt.md` if not already complete.

## Report back

Use the Implementation Complete format. Do not commit.
