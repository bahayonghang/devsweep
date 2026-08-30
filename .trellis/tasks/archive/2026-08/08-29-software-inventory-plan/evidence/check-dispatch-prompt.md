Active task: .trellis/tasks/08-29-software-inventory-plan

You are the trellis-check sub-agent performing independent verification in repo D:\Documents\Code\Rust\Exp\devsweep (Windows, Git Bash shell, branch `dev`). Standing constraints: no sub-agents, no commit/push, protected dirty files (`.trellis/.gitignore`, `README.md`, `justfile`) untouched, no cap/contract weakening; self-fix only in-scope defects.

## Read first
`check.jsonl` + its listed files, then `prd.md`, `design.md`, `implement.md`. Spec: `.trellis/spec/backend/index.md` + referenced guides.

## What was implemented (verify each)
1. Approved dependency: `windows = "=0.56.0"` Windows-target only, five named features, single `windows` version in the core tree; size-growth gates recorded (binary +411,136 bytes < 5 MiB; zip +149,703 < 2 MiB). Audit `step1-*` and `step3-size-budget.json`.
2. Core Software V1 (`crates/devsweep-core/src/software/`): ARP 32/64 HKCU/HKLM allowlisted-value adapters, MSI user/machine identities (manual), MTA current-SID MSIX adapter (`FindPackagesByUserSecurityId`, joined worker, clean shutdown), ordered refusal matrix (all MSI/registry-only manual; only complete/healthy current-user MSIX selectable), closed size union (checked KiB; MSIX bounded no-follow InstalledPath measurement, path never serialized), last-used strictly `unknown`/`no_supported_exact_source`, dedup/fingerprint incl. refusal/evidence, untrusted selection, live revalidation, opaque preview token, digest.
3. CLI: `software.*` dispatch activation in `commands/mod.rs` (narrowly-justified seam mirroring analyze; parser `cli.rs` untouched — verify with git diff), frozen `software inventory`/`software plan` handlers in `commands/software/inventory.rs`, bilingual renderer, `software preview`/`software uninstall` still unavailable.
4. Evidence: hostile/parity/native records under `evidence/` (`step3-native-summary.json`, `step3-native-inventory.json`, `step3-native-plan-safety.log`, etc.).

## Verify
1. Re-run: `cargo test -p devsweep-core software`, `cargo test -p devsweep-cli software`, `cargo fmt --all -- --check`, `git diff --check`, `just ci`, `npm --prefix desktop run typecheck` (Vitest may EPERM in your sandbox; audit the recorded main-session logs).
2. Security/authority audit: no CleanupPlan conversion, no vendor command/icon/uninstall-string reads, no other-user enumeration, no UAC, no serialized installed paths, no network, program/argv separation preserved, plan payloads contain only the five frozen fields.
3. Contract audit: parser grammar untouched (git diff), DTO/fingerprint determinism, refusal matrix ordering exact, dedup exactness, fingerprint includes refusal/evidence.
4. Cross-check the recorded native evidence for authenticity (commands, exit codes, counts, SID handling, MTA shutdown).
5. Self-fix in-scope defects; STOP for contract/cap/dependency changes.

## Output
Update/append `evidence/independent-check-report.md` (child-verification section), save raw logs with an `independent-` prefix, append `check.jsonl` lines. Final message: overall PASS or FAIL per prd AC, commands + exit codes, fixes, blockers.
