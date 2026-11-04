# C12 Implementation and verification

Status: in_progress. Dependencies C04, C05, and C10 are archived. C06 is not accepted. C12-AC3 and C12-AC4 are checked against release SHA-256 `6E827733E649AE9A13E1349AE0565C17F90115C9B5B006ABF66E3F5F1CF37A69`, 17511936 bytes, rustc 1.99.0 via mbx, not an MSRV 1.88.0 pass. Keyboard review used an isolated startup hive and a missing update provider. Host defrag, DISM, and clock-sync rows still say not run. They were not executed. C06-AC4 stays NOT_RUN. Do not uninstall real software or change real startup items. Not archived.

Next prompt: `implement-prompt.md`. This is the software verification prompt and runs last.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C12-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- GPUI software/optimize state/action tests
- Native disposable-provider UI flow and CLI parity
- Parent software/maintenance response and lifecycle measurements

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
