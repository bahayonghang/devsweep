# C: execution plan

## Gate and order

- [x] Read parent prd.md/design.md/task-map.md, this child's design and curated context.
- [x] Confirm latest planning approval and dependencies: A's observation/compatibility contract; B's owner/footprint contract before mbx acceptance. Non-mbx pure fixture work can begin once A is stable.
- [x] Reproduce relevant baseline and freeze fixtures/expected behavior before changing code.
- [x] Step 1: Default T=5,368,709,120 bytes, inclusive. Aggregate owner-attributed build/test/tool-cache footprints only; show dependencies/venv separately, outside this subtotal.
- [x] Step 2: Exclude global/shared mbx storage; count overlapping aliases once; distinguish exact values, known lower bounds and unknown.
- [x] Step 3: Warn when a complete value or partial lower bound reaches T. A partial value below T or unknown cannot claim below threshold.
- [x] Step 4: Persist a positive whole-GiB setting with overflow-safe byte conversion, default-on-missing without write, and preserve corrupt/future/unknown data.
- [x] Step 5: Evaluate completed user-requested reports; setting changes can re-evaluate a snapshot without I/O. No timers, background scans, OS notifications, selection changes or cleanup.
- [x] Run focused verification and resolve failures before integration.
- [x] Update only affected executable spec contracts; record evidence and gaps.
- [ ] Pass independent review and hand findings to parent/F. Do not commit or archive without authorization.

## Verification

Pure boundary/property-style aggregation tests, persisted-setting compatibility/error tests, closed DTO decoder/wire parity; just ci and relevant generated-types checks.

## Review gates

Check every acceptance item in prd.md. Distinguish source-confirmed, fixture-tested, native-tested and NOT_RUN. Preserve dependency/ownership boundaries and the existing user modification.

## Rollback point

Remove advisory presentation/commands without altering reports, plans, root stores or cached artifacts; preserve unknown settings bytes.

