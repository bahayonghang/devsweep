# C09 Implementation and verification

Status: technical freeze recorded in design.md on 2026-10-17. C02 checks passed and `just ci` exited 0. Implementation is dispatched inside `desktop-gpui/` and this task directory. C09-AC4 launch evidence is recorded for SHA-256 `7C1A82CD42F6E071624DFAAD8B1E70AD21C822FCBE833FC6764D54B73230669D`. UIA focus and IME composition stay NOT_RUN.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C09-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- Release build of the approved GPUI spike using its pinned toolchain
- Native Windows UIA/IME/DPI/graphics/startup/close matrix
- Core/CLI MSRV gate and just ci

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
