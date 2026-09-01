Active task: .trellis/tasks/08-29-protect-rules-history-surfaces

You are already the trellis-implement sub-agent. Implement directly. Do NOT spawn trellis-implement or trellis-check. Do NOT git commit, push, merge, or amend.

## Repo

D:\Documents\Code\Rust\Exp\devsweep — Windows PowerShell, branch `dev`.

Deps are archived/accepted: Clean workbench, Software execution audit, Optimize catalogue/audit, desktop shell. Status parent acceptance PASS at `682c5d2`. Do not reopen those children.

## Load

implement.jsonl + listed files, prd.md, design.md, implement.md, `.trellis/spec/backend/index.md`, `.trellis/spec/frontend/index.md`, `.trellis/spec/desktop-frontend/index.md`. Frozen CLI from `.trellis/tasks/archive/2026-08/08-29-cli-contract-localization/design.md`: `clean protect list|add|remove --path` with `--confirm` for mutations; `clean rules list|show --id`; `history list` / `history show --operation-id`. Do **not** edit `cli.rs`. Fill `application/commands/{protect,rules,history}.rs` and `presentation/{protect,rules,history}.rs` only.

## Contracts

R1 Protection: persistent exact-path allowlist/deny-to-clean; sidecar lock over load→validate→canonical compare→mutate→temp flush→atomic replace; two processes cannot lose updates; corrupt/unknown UTF-8/JSON/newer version preserve original bytes and return `protection_store_unavailable`; never auto-clear or delete; cannot expand cleanup scope; add/remove confirmation; mutation audit via Clean V1 writer with action/operation id/stable code/SHA-256 of canonical identity, **never raw path**; store not committed until audit terminal flushed; pre-mutation audit failure blocks mutation; post-replace audit failure fail-closed/unknown without rolling store back or repeating mutation.

R2 Rules: read-only projection of shipped registry; no edit/import/execute.

R3 History: read only the three fixed V1 Clean/Software/Optimize stores from shared app-data resolver; no directory walk, no `--audit-log`, no legacy default audit file; no replay; redact command/path/argv; protection-mutation records have no raw path.

R4 supporting CLI/TUI/Desktop under Clean/help/overflow — **no sixth primary nav mode**.

Existing `crates/devsweep-core/src/execution/safety/protections.rs` currently parses and may fail without byte preservation — upgrade in place to the sidecar-lock fail-closed contract.

## Forbidden

`.trellis/.gitignore`, `README.md`, `justfile`, other task dirs. No `git add -f .trellis/`. No push/amend. No unapproved deps. No UAC. No fuzzy protection. No audit replay. Do not create additional product modes. Permanent delete remains disabled.

## Required

1. Implement design file-level change list including `docs/reference/{protection,rules,history}.md`.
2. Focused gates with logs under this task `evidence/` (command + real exit):
   - `rtk cargo test -p devsweep-core -- protection`
   - `rtk cargo test -p devsweep-core -- history`
   - `rtk cargo test -p devsweep-cli -- protect`
   - `rtk cargo test -p devsweep-cli -- rules`
   - `rtk cargo test -p devsweep-cli -- history`
   - `rtk just desktop-web-check` (or npm support tests + lint/typecheck/build as implement.md)
   - `rtk just desktop-test` if Tauri support commands added
   - `rtk git diff --check`
3. Native GUI bar (completion-required): EN/ZH, keyboard, SR/AX, high contrast, narrow/wide, concurrent protection writers, corrupt/newer stores, mixed audit versions, redaction inspection, proof no history row can execute/replay.
4. `evidence/verification-report.md`, implement.jsonl notes, `evidence/dispatch-prompt.md`.

Report Implementation Complete. Do not commit.
