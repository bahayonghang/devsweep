# C10 Implementation and verification

Status: 2026-10-19 `just gpui-test` exited 0 on rustc 1.99.0. This is not an MSRV 1.88.0 pass. C10-AC1, C10-AC2, and C10-AC4 stay checked. C10-AC3 stays unchecked. Narrator utterance and the DevSweep frame interval stay NOT_RUN. They are not PASS. Dependencies C01 and C09 are archived. This child is not archived.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C10-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- GPUI state/action tests on the approved pinned version
- Native shell/settings/locale/accessibility matrix
- `just ci plus explicit GPUI native gate`

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
