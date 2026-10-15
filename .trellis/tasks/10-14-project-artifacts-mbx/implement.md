# B: execution plan

Implementation handoff: [safe partial delivery and verification](implementation-results.md).
Executable mbx cleanup remains unqualified and this task stays in progress.

## Gate and order

- [x] Read parent prd.md/design.md/task-map.md, this child's design and curated context.
- [x] Confirm latest planning approval and dependencies: A's scope/compatibility policy. Publish the ownership/overlap contract before C's final aggregation and D's final integration.
- [x] Reproduce relevant baseline and freeze fixtures/expected behavior before changing code (parent confirmed frozen baseline).
- [x] Step 1: Retain marker-first proof for Node build/test outputs, distinguish node_modules/venv dependencies, and use Cargo metadata for workspace/external target ownership.
- [ ] Step 2: Inventory mbx shared action storage separately from workspace target/private state; identify missing/unsupported/ambiguous evidence rather than reporting zero.
- [x] Step 3: Qualify supported public versioned mbx outputs before introducing an executable intent. Qualification found workspace/managed-root execution insufficient; no executable intent added.
- [ ] Step 4: Implement a closed, explicitly selected mbx workspace cleanup only after live footprint, executable/config identity, protection and busy/lease proof can be bound to preview/execute.
- [ ] Step 5: Deduplicate Rust/mbx aliases and nested/shared targets; never follow arbitrary links or run global gc as project cleanup.
- [x] Run focused verification and resolve failures before integration (scan 88, rules 26, footprints 11 pass; final integrated lint belongs to parent/F).
- [x] Update only affected executable spec contracts; record evidence and gaps.
- [ ] Pass independent review and hand findings to parent/F. Do not commit or archive without authorization.

## Verification

Before the executable-intent gate, record the pinned mbx version and full mutation set, including workspace link and metadata. Audit version-matched source/public contracts and test link/protection/digest drift, untouched shared objects, unrelated-workspace sentinels, busy partial results and post-command disposition. Mocks do not independently prove real command behavior; a disposable conformance run is a separately authorized fallback.

Isolated read-only mbx reporting fixtures and mocked owner-command execution; project/Cargo/dedupe/provider-authority tests; CLI/Tauri parity; just ci. No real user-cache cleanup or global gc.

## Review gates

Check every acceptance item in prd.md. Distinguish source-confirmed, fixture-tested, native-tested and NOT_RUN. Preserve dependency/ownership boundaries and the existing user modification.

## Rollback point

Disable/revert the new executable provider while preserving truthful inspection; never weaken generic no-follow or expand a command to make acceptance pass.

