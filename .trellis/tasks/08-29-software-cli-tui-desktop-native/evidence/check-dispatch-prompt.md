Active task: .trellis/tasks/08-29-software-cli-tui-desktop-native

You are the trellis-check sub-agent performing independent verification in repo D:\Documents\Code\Rust\Exp\devsweep (Windows, Git Bash shell, branch `dev`). Standing constraints: no sub-agents, no commit/push, protected dirty files untouched, no real package removal, no cap/contract weakening; self-fix only in-scope defects.

## Read first
`check.jsonl` + listed files, then `prd.md`, `design.md`, `implement.md`. Specs: `.trellis/spec/backend/index.md`, `.trellis/spec/frontend/index.md`, `.trellis/spec/desktop-frontend/index.md` (+ guides, including the render-budget/native evidence protocol).

## What was implemented (verify each)
1. Tauri Software IPC (inventory/preview/uninstall/audit/cancel) with operation-id scoping, single-flight, stale-result rejection, audit recovery; strict TS decoders + generated fixtures/types (`desktop/src-tauri/src/software.rs`, `desktop/src/api/`).
2. TUI Software mode (`crates/devsweep-cli/src/tui/modes/software/` + render + shell registration): full state machine, exact-ID rows, disabled manual entries, sticky summary, size/last-used evidence, second confirmation, closed terminals, stale rejection, cancel/join.
3. Desktop SoftwareWorkbench (`desktop/src/modes/software/` + App registration): same machine over typed IPC, accessible list, bilingual, responsive, reduced-motion/forced-colors.
4. Bilingual CLI human renderer updates; parser grammar untouched (verify git diff on `application/cli.rs` is empty for grammar changes).
5. Parity fixtures: 13 refusal reasons, identity kinds, duplicate/long names, partial sources, strict last-used unknown, five terminals, restart audit recovery; `partial` outcome rejected in both Rust and TS.
6. Native evidence: implementer's standard-user inventory/preview records + the main session's capture set in `evidence/native-20260831/` (bilingual + widths + DPI 100-200% + 44,045-node AX tree + real 1,293-entry inventory; see `native-evidence.md` final section). Real-uninstall scenarios remain task-doc-sanctioned UNVERIFIED.

## Verify
1. Re-run: `cargo test -p devsweep-cli software`, `cargo test -p devsweep-cli tui::modes::software`, `cargo test -p devsweep-desktop software`, `cargo fmt --all -- --check`, `git diff --check`, `just ci`, `npm --prefix desktop run typecheck`. Full `npm test`/web-check may EPERM in your sandbox — audit `evidence/desktop-web-check-final.log` (main session: 22 files / 130 tests passed, exit 0) and `desktop-build-main.log` (exit 0).
2. Cross-surface parity: same fixture through CLI JSON, TUI reducer, desktop decoder/selectors — identical identities/refusals/evidence; machine documents locale-invariant.
3. Authority audit: no irreversibility wording violations, no cleanup authority outside the frozen software flow, no `partial` execution acceptance, plan payloads frozen fields, stale events rejected, cancel/join on navigation.
4. Audit the native evidence for authenticity (logs, probes, hashes) and coverage of implement.md step 3 minus the sanctioned UNVERIFIED set; adjudicate the `${arpDisplayName}` registry-data note in `native-evidence.md` (raw third-party registry value surfaced as data — defect or faithful behavior under "paths/display strings are data"?).
5. Self-fix in-scope defects; STOP for contract/cap/core changes (core defects belong to the owning child).

## Output
Update `evidence/independent-check-report.md`, save `independent-` logs, append `check.jsonl`. Final message: overall PASS or FAIL per prd AC (noting the sanctioned UNVERIFIED set), commands + exit codes, fixes, blockers.
