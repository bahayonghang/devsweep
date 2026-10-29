# C11 Implementation and verification

Status: in_progress. Dependencies C03, C10, and C15 are archived. On 2026-10-26 the isolated keyboard review reached preview and execute, and a later release binary completed the F10, Down three times, Enter history path against an isolated store. Native AI review scanned only an explicit temp home. The 10k-row search, select, and scroll upper bounds were within 100 ms. The 16.7 ms frame budget was NOT_PASS at nearest-rank p95 45.960 ms. Cancel acknowledgement was within 100 ms on 6 samples; the 100-sample count is unmet. UIA FocusPattern is not exposed and is not PASS. C11-AC1 through C11-AC4 stay unchecked. Not archived. See `evidence/c11-r10-history.txt`, `c11-r10-ai.txt`, `c11-r10-interact.txt`, `c11-r10-scroll.txt`, and `c11-r10-select-frames-log.txt`.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C11-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- GPUI Clean state and digest-invalidation tests
- Disk map projection tests in `desktop-gpui/src/clean_map.rs`: category split, nested build output, area, Other byte conservation, zero-byte exclusion
- `found_scan_summary_names_the_disk_map` and `disk_map_summary_does_not_replace_the_outcome_line`
- Native isolated scope/parity/error/keyboard flow
- Parent performance protocol Clean workload

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
