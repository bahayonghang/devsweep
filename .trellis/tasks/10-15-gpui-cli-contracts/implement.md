# C01 Implementation and verification

Status: implemented on 2026-10-17, not accepted. C16-AC3 hosted matrix and local Rust 1.88.0 remain FAIL or NOT_RUN.

Local gates on rustc 1.99.0 (`b940084d7`, 2026-09-28), `--target-dir dist/ci-target`:

- `cargo test --locked -p devsweep-cli --test cli_contract`: exit 0, 13 passed.
- `cargo test --locked -p devsweep-cli --test five_mode_contract`: exit 0, 8 passed.
- `just ci` devsweep-core lib: 491 passed, 3 ignored. The first `just ci` in the refusal pass exited 1 with 488 passed, 3 failed, and 3 ignored. `process::tests::endless_stdout_is_capped_with_truncation_mark`, `endless_stderr_is_capped_with_truncation_mark`, and `hung_child_and_grandchild_are_terminated` failed under load. The rerun log says 491 passed and 3 ignored. `process/mod.rs` was not changed.
- `just ci`: exit 0. Full log: `C:\Users\lyh\AppData\Local\Temp\grok-goal-8f19d0d75913\implementer\c01-just-ci.txt`. Named exits: `C:\Users\lyh\AppData\Local\Temp\grok-goal-8f19d0d75913\implementer\c01-checks.txt`.

C01-AC1 has evidence from the clap manifest test, the frozen parser test, `cli_contract`, and `five_mode_contract` inside that `just ci` run. C01-AC2 compares three successful previews and three stale refusals between `CARGO_BIN_EXE_devsweep` and the core service on the same fixture. The refusal codes are `startup_entry_changed`, `stale_candidate`, and `changed_since_snapshot`, and each CLI process exits 3. C01-AC3 is checked: startup and leftover missing-confirm tests, Analyze replacement and protection drift, startup partial list, and output-collision tests passed before a side effect. `analyze_trash_execute_without_confirm_leaves_the_fixture` and `analyze_trash_execute_closed_stdout_does_not_move_the_fixture` cover Analyze strict execute.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C01-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- `cargo test --locked -p devsweep-cli --test cli_contract`
- `cargo test --locked -p devsweep-cli --test five_mode_contract`
- `cargo test --locked -p devsweep-core`
- `just ci`

Use [the parent protocol](../10-15-gpui-desktop-rewrite/research/performance-protocol.md)
for applicable native measurements. Freeze approved budgets and exact scopes before
candidate work. A failed result never authorizes a relaxed threshold.

## Evidence and stop conditions

- Map every requirement to the acceptance checklist in prd.md.
- Use isolated roots, test registry state, disposable apps and owned processes.
- No real user-cache cleanup, unrelated app uninstall or host-wide repair for tests.
- Keep failures and NOT_RUN/UNVERIFIED platform/hardware results explicit.
- Failed required dependency evidence blocks downstream integration.
- Preserve current rollback behavior until C14 accepts migration.
