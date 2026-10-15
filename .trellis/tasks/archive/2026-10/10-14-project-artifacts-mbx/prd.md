# B: Project artifacts and mbx ownership

## Goal

Identify Node/Rust and mbx-managed project artifacts with trustworthy ownership and on-demand cleanup, without confusing shared cache storage with project-owned bytes.

## Parent and dependencies

Parent: [cleanup-home-redesign](../10-14-cleanup-home-redesign/prd.md).
A's scope/compatibility policy. Publish the ownership/overlap contract before C's final aggregation and D's final integration.

## Requirements

B1. Retain marker-first proof for Node build/test outputs, distinguish node_modules/venv dependencies, and use Cargo metadata for workspace/external target ownership.
B2. Inventory mbx shared action storage separately from workspace target/private state; identify missing/unsupported/ambiguous evidence rather than reporting zero.
B3. Qualify supported public versioned mbx outputs and exact workspace/managed-root association before introducing an executable intent.
B4. Implement a closed, explicitly selected mbx workspace cleanup only after live footprint, executable/config identity, protection and busy/lease proof can be bound to preview/execute.
B5. Deduplicate Rust/mbx aliases and nested/shared targets; never follow arbitrary links or run global gc as project cleanup.

## Acceptance criteria

- [x] Fixtures cover Node proof success/failure, tracked/sensitive content, Cargo workspace members, external targets, metadata fallback and safe no-follow behavior.
- [ ] Same managed target is counted once; shared store is never counted for every workspace; other projects remain outside selected scope.
- [ ] mbx absent, unsupported version, malformed/truncated data, external roots, stale config/link/tool identity and busy state are visible and fail closed.
- [ ] A supported workspace cleanup has real scan -> plan -> preview -> simulated execution sentinel tests including cross-workspace protections.
- [ ] Qualification enumerates all pinned-version mutations, including the workspace target link and metadata. Preview discloses link removal; protection/digest tests cover changed or protected links, untouched shared storage, other-workspace sentinels, busy partial outcomes and post-command disposition. A version-matched source/public-contract audit corroborates the command footprint beyond mocks; any disposable conformance execution needs separate authorization.
- [x] If no stable ownership/footprint contract can be proven, executable mbx acceptance stays open and the parent is not declared delivered; inspect-only UI is a partial result.

## Scope boundaries

Implementation authorized on 2026-10-14. Preserve the pre-existing desktop/src-tauri/Cargo.toml edit. No actual user-cache cleanup, unrelated modes, commits, archive or publication. Parent scope exclusions apply.

## Decision status

User product decisions and selected design are in the parent PRD. The mbx executable authority gate is a technical proof obligation, not permission to silently ship inspection as full cleanup.

