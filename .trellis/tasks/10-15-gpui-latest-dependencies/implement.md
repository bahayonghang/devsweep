# C16 Implementation and verification

Status: implementation approved and active; dependency audit/API adaptation are
implemented and independently reviewed. Local `just ci` on rustc 1.99.0 exited 0
on 2026-10-17 with `--target-dir dist/ci-target`. Hosted run 39001489521 on
`b0b5ce62` concluded failure, and that SHA does not contain the later working-tree
fixes. Declared MSRV 1.88.0 was not executed locally. C16-AC3 remains
NOT_YET_ACCEPTED, so C01 has not started.
See acceptance-status.md and research/toolchain-decision.md.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C16-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- `just ci`
- Explicit core/CLI declared-MSRV checks after policy freeze
- GPUI candidate/version inventory; final native build remains C09
- Dependency resolution/license/advisory evidence and compatibility tests

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
