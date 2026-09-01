Active task: .trellis/tasks/08-29-analyze-core-ipc

REOPEN RE-VERIFICATION. You are the trellis-check sub-agent for the reopened child in repo D:\Documents\Code\Rust\Exp\devsweep (Windows, Git Bash shell, branch `dev`). Standing constraints: no sub-agents, no commit/push, protected dirty files (`.trellis/.gitignore`, `README.md`, `justfile`) untouched, no cap/contract weakening. The parent `08-29-analyze-mode` acceptance review failed this child on four gaps; a trellis-implement dispatch claims all four are fixed. Independently verify each.

## What changed (implementer claims; verify against actual diff)
1. ACL-denied/unverified no-follow opens now return `AccessDenied` → incomplete evidence + access-denied warning + zero available bytes, never traversed (`crates/devsweep-core/src/analysis/walker.rs`).
2. Sparse-file coverage: fake 8 GiB logical sparse fixture + native `FSCTL_SET_SPARSE` test (`analysis/tests.rs`).
3. Native 6,000-file live-churn gate distinguishing deletion (unknown+churn) from growth (incomplete+churn) with lower-bound reconciliation, real ACL-denied branch; refreshed release 50k gate (51,007 nodes, 3 ms p95 cancel-to-join).
4. Parity gate: deterministic fixture generator (`evidence/reopen-generate-parity-fixture.ps1`), recorded CLI JSON (`desktop/src/api/fixtures/analyze/cli-parity.json`), TUI reducer parity test, desktop decoder/selector Vitest parity test (`desktop/src/modes/analyze/parity.test.ts`) — all surfaces agree on 7 nodes / 8,388,616 lower-bound bytes / 5 complete / 2 incomplete / 1 access-denied warning (snapshot sha256 d14a6c33…).

## Verify
1. Read the diff of `crates/devsweep-core/src/analysis/{walker.rs,tests.rs}`, `crates/devsweep-cli/src/tui/modes/analyze/mod.rs`, `desktop/src/modes/analyze/parity.test.ts`, and the new fixture. Confirm the denied classification change cannot follow/traverse denied paths and does not weaken the conservative reparse handling for non-denied probe failures; confirm no cleanup authority, no DTO change, no new dependency, no frozen-contract edit.
2. Re-run: `cargo test -p devsweep-core analysis` and with `--release -- --nocapture` (includes native gates), `cargo test -p devsweep-cli analyze`, `cargo fmt --all -- --check`, `git diff --check`, `just ci`, `npm --prefix desktop run typecheck`. Vitest/Vite may EPERM in your sandbox — if so, audit `evidence/reopen-desktop-test-main-session.log` (main session run: 19 files / 122 tests passed, exit 0).
3. Audit the parity artifacts: fixture generator determinism (hash), CLI JSON vs TUI vs desktop parity claims, snapshot hash.
4. Audit native evidence: `reopen-core-analysis-release.log` / refreshed `native-50k-status.json` (churn/denied/sparse assertions present), and any capture logs.
5. Self-fix real defects in scope; STOP for anything requiring contract/cap changes.

## Output
Update `evidence/independent-check-report.md` with a reopen-verification section and per-gap verdicts. Save raw logs with a `reopen-independent-` prefix. Append `check.jsonl` journal lines. Final message: overall PASS or FAIL for the four gaps, commands + exit codes, fixes applied, blockers.
