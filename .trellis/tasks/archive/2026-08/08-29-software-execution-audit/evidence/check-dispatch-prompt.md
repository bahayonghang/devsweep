Active task: .trellis/tasks/08-29-software-execution-audit

You are the trellis-check sub-agent performing independent verification in repo D:\Documents\Code\Rust\Exp\devsweep (Windows, Git Bash shell, branch `dev`). Standing constraints: no sub-agents, no commit/push, protected dirty files untouched, no real package removal, no cap/contract weakening; self-fix only in-scope defects.

## Read first
`check.jsonl` + listed files, then `prd.md`, `design.md`, `implement.md`. Spec: `.trellis/spec/backend/index.md` + guides.

## What was implemented (verify each)
1. Opaque non-deserializable `ValidatedSoftwareAction`; constructors only from live revalidation + exact eligible current-user MSIX identity; type-level/runtime proof that MSI/registry-only/machine/other-user/protected/stale/malformed identities cannot reach an adapter.
2. Locked versioned redacted JSONL audit: durable pre-side-effect `dispatch_started`, monotonic transitions, stable codes, `flush`/`sync_all`, exactly one terminal; fail-closed identity-only startup recovery with NO redispatch; digest/lock failures before any adapter call.
3. MSIX adapter: inventory MTA worker reuse, `RemovePackageAsync` with exact `PackageFullName`, process+audit-lock single flight, 120 s monitor, pre-dispatch cancel semantics, one post-dispatch OS cancel + 5 s grace + 0/2/10 s requeries, closed terminal set (removed/reboot_required/still_present/failed/unknown_after_dispatch), never reversible/`partial`; crash/evidence-conflict/adapter-unfinished/absent+reboot/present+failure/success fixtures with no-second-removal recovery proof.
4. CLI: frozen `software preview`/`software uninstall` wired in the owned handler; hostile-plan and locale-invariant coverage; `software inventory`/`plan` still pass.
5. Native non-uninstall evidence under `evidence/` (medium-integrity run, no UAC, stale-authority exit 3, audit durability/redaction/locking/unknown-version refusal). Real-uninstall scenarios are documented UNVERIFIED per implement.md's own no-disposable-fixture rule — verify that documentation is honest and complete, NOT that they ran.

## Verify
Re-run `cargo test -p devsweep-core software`, `cargo test -p devsweep-cli software`, `cargo fmt --all -- --check`, `git diff --check`, `just ci`, `npm --prefix desktop run typecheck`. Audit: no reversibility/partial language in code+docs; audit redaction actually redacts; recovery cannot redispatch; single-flight enforcement; terminal-table exhaustiveness; no MSI/machine/other-user path to the adapter; parser grammar untouched (git diff); no new dependency. Self-fix in-scope defects; STOP for contract/cap changes.

## Output
Update `evidence/independent-check-report.md`, save `independent-` logs, append `check.jsonl`. Final message: overall PASS or FAIL per prd AC (noting the documented UNVERIFIED real-uninstall set as task-doc-sanctioned), commands + exit codes, fixes, blockers.
