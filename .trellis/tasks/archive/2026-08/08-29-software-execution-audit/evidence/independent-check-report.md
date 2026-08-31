# Independent check report

Overall: **PASS** against the task PRD and approved implementation boundary.

The real-uninstall native scenario set remains explicitly **UNVERIFIED** because the repository has no disposable current-user MSIX fixture and the user did not confirm a concrete package for removal. This is the exact no-fixture behavior required by `implement.md`; it is not treated as fabricated evidence or a failed automated gate.

## Findings (fixed)

- File: `crates/devsweep-core/src/software/execution/mod.rs`, `crates/devsweep-core/src/software/execution/tests.rs`, `crates/devsweep-cli/src/application/commands/software/execution.rs`
- Issue: `SoftwareExecutionRequest` publicly accepted a caller-constructed `SoftwareInventoryV1`, so an external caller could forge current-user/eligibility facts despite the opaque `ValidatedSoftwareAction`. The core executor also did not enforce explicit confirmation independently of the CLI.
- Fix: removed caller-supplied inventory from the public request, added a private executor-owned live current-user MSIX provider acquired behind single-flight after pending recovery, required `confirmed: true` at the core boundary, and added no-adapter tests for missing confirmation plus MSI/registry/machine/protected/malformed authority.

- File: `crates/devsweep-core/src/software/execution/audit.rs`, `crates/devsweep-core/src/software/execution/mod.rs`
- Issue: journal replay allowed a terminal directly after `dispatch_started` and did not prove that terminal adapter/requery fields matched earlier records. A crafted/corrupt journal could therefore skip required requery evidence or claim an unobserved terminal state.
- Fix: tightened the transition graph, made post-dispatch terminal require prior requery, accumulated conflict-preserving requery state, and required terminal adapter outcome, reboot evidence, and installed state to equal accumulated evidence. Added replay regression tests.

- File: `crates/devsweep-core/src/software/execution/tests.rs`
- Issue: terminal-table and single-flight proof was representative rather than exhaustive/direct.
- Fix: expanded the valid adapter/installed-state matrix to all 16 combinations and added a concurrent process-mutex test proving maximum active adapter count is one.

- File: `crates/devsweep-cli/src/application/output.rs`, `crates/devsweep-cli/src/application/mod.rs`, `crates/devsweep-cli/src/application/commands/software/execution.rs`
- Issue: a failed/canceled Software report was emitted and then returned as a normal `ApplicationError`, causing machine output to attempt a second terminal envelope while still needing a nonzero exit.
- Fix: added an internal already-emitted error path so the single Software report remains the only machine document while exit 6 or 130 is preserved; added regression coverage.

- File: `.trellis/spec/backend/quality-guidelines.md`, `.trellis/spec/backend/database-guidelines.md`
- Issue: long-term backend guidance described Clean execution/audit but not the new Software live-authority, recovery, terminal, and persistence contracts.
- Fix: documented the executable Software boundary, private live provider, explicit confirmation, exact terminal table, journal/recovery invariants, tests, and sanctioned native `UNVERIFIED` boundary.

## Findings (not fixed)

None.

## Verification

- Lint: pass. `cargo fmt --all -- --check`, `git diff --check`, and workspace clippy with `-D warnings` all exited 0.
- TypeCheck: pass. Workspace `cargo check --locked --all-targets` and `npm --prefix desktop run typecheck` exited 0.
- Tests: pass. Focused core Software tests: 35 passed. Focused CLI Software tests: 9 passed. Two compile-fail authority doctests passed. `just ci`: pass with 412 tests passed and 2 intentional desktop process-fixture entrypoints ignored.
- Parser/dependencies: pass. Frozen `application/cli.rs` diff is empty; Cargo manifests and `Cargo.lock` are unchanged.
- Native evidence: pass for the required non-uninstall evidence and honesty boundary. Real uninstall, reboot, post-dispatch cancellation/timeout, crash/restart, requery denial, and remaining-present scenarios are documented `UNVERIFIED` pending a user-confirmed disposable MSIX target.

Evidence logs: `independent-focused-gates.log`, `independent-full-gates.log`, and `independent-static-audit.log`.
