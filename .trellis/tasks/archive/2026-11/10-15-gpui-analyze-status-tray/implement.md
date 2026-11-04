# C13 Implementation and verification

Status: in_progress. Dependencies C07, C08, and C10 are archived. C13-AC1 is checked on release SHA-256 `6E827733E649AE9A13E1349AE0565C17F90115C9B5B006ABF66E3F5F1CF37A69`, 17511936 bytes. The same hash completed 20 create, minimize, restore, and `WM_CLOSE` cycles with `clean_exits=20`, `forced=0`, leftover 0. `23F7D614` and `5BFE59AF` cycles were not reused. OS crash release of the real execution state is NOT_RUN. A displayed PID is not termination authority. A missing sensor stays unavailable; this host's GPU and thermal probes were available, and processes was partial. Large snapshot rendering remains NOT_RUN. C13-AC3 and C13-AC4 stay open. Not archived.

Next prompt: `implement-prompt.md`. It does not rerun cycles or crash release. Software verification is the last prompt.

## Sequence

1. Verify approval of the parent final-review.md; platform scope is already confirmed.
2. Verify listed dependency acceptance and load curated manifests. Complete this
   child's technical decisions before task.py start or code dispatch.
3. Assign exact file ownership; implement only the requirements in this PRD.
4. Add meaningful successful and rejected identity/partial/failure/lifecycle cases
   appropriate to the boundary; do not add tests that only repeat implementation.
5. Run targeted checks below and required broader gates; repair relevant failures.
6. Record each C13-AC result with source/binary identity and evidence level.
7. Dispatch independent Trellis check review, update implemented-contract specs,
   and hand accepted contracts to dependents. Commit/archive need separate approval.

## Checks

- GPUI treemap/process/tray state tests
- Native lifecycle/emergency-exit/accessibility tests
- Parent Analyze/Status memory and percentile protocol

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
