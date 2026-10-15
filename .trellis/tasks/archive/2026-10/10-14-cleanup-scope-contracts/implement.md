# A: execution plan

## Gate and order

- [x] Read parent prd.md/design.md/task-map.md, this child's design and curated context.
- [x] Confirm latest planning approval and dependencies: user approved implementation; parent released frozen-baseline gate.
- [x] Reproduce relevant baseline and freeze fixtures/expected behavior before changing code.
- [x] Step 1: Global-only discovery works with no roots; project-only discovery requires explicit validated roots. Keep CLI --scope projects|global|all and its existing default behavior.
- [x] Step 2: Reuse uv and the current global provider catalogue. Model availability, capability, coverage and sizing separately; do not add an unnecessary new uv implementation or swap clean for prune.
- [x] Step 3: Set project selected_by_default=false through the shared policy. Keep explicit CLI selection and global provider risk rules intact; update TUI/native consumption tests.
- [x] Step 4: Keep saved ScanReport V1 and CleanupPlan V2 compatibility. Advisory/overview fields must not be silently added to deny_unknown_fields documents.
- [x] Step 5: Never add direct deletion or shell command assembly; use current saved plan, live preview digest and confirmation.
- [ ] Run focused verification and resolve failures before integration.
- [x] Update only affected executable spec contracts; record evidence and gaps in result.md.
- [ ] Pass independent review and hand findings to parent/F. Do not commit or archive without authorization.

## Verification

Extend scan/global fixtures, ranking, CLI contract and provider_authority suites; run just ci; exercise generated-type parity if any wire schema changes.

Implementation handoff is in [result.md](./result.md). A's focused tests pass;
two unchanged-source tests failed in broader runs and require parent/F triage.
The parent owns integrated `just ci` and independent review; these are not
claimed complete here.

## Review gates

Check every acceptance item in prd.md. Distinguish source-confirmed, fixture-tested, native-tested and NOT_RUN. Preserve dependency/ownership boundaries and the existing user modification.

## Rollback point

Revert this bounded contract/policy change without rewriting saved observations, plans, or user settings.

