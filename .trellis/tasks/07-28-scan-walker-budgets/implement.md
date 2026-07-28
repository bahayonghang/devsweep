# Implementation Plan: Bounded Walkers and Explicit Ranking

## Preconditions

- Consume `ScanOutcome`, `SizeEstimate`, and partial diagnostic contracts from
  `07-28-scan-reliability`.
- Consume the cancellation observer interface from `07-28-true-cancellation`
  or its approved no-op adapter.
- Keep plan serde changes with `07-28-plan-validation` and do not alter
  default selection semantics outside incomplete/unknown safety handling.

## Ordered work

1. Add deterministic fail-red fixtures for a normal `cache/` project, VCS
   metadata, budget exhaustion, depth/entry limits, and a top-level large fanout.
2. Define traversal budget, diagnostic, device/volume boundary, and cancellation
   observation abstractions with test-controlled clocks/counters.
3. Replace recursive discovery descent with an iterative stack, then apply VCS
   skips and rule-aware cleanup-footprint pruning.
4. Replace recursive Rayon size traversal with an iterative per-target walker;
   schedule only independent top-level target estimates in parallel.
5. Emit partial completeness/warnings on every budget or boundary stop and
   preserve known lower bounds through the scan-reliability pipeline.
6. Rename `target_score` to `freshness_tiebreaker`, lock D3 ordering tests, and
   update user-facing/docs wording where the old score implied primary rank.
7. Add benchmark fixtures for 10k entries and record p95 on supported hosts;
   investigate regressions but do not hide them by weakening diagnostics.
8. Run scanner/fs-size/ranking/sweep/TUI focused tests, three-platform evidence,
   then `just ci`.

## Verification matrix

| Scenario | Required evidence |
| --- | --- |
| ordinary `cache/` directory | nested Cargo/Node project is discovered normally |
| VCS subtree | traversal counter proves `.git/.hg/.svn` entries are not descended |
| cleanup footprint | recognized target is emitted once and its opaque contents are not walked |
| budget/deadline/cancel | result is partial with diagnostic and known lower bound/unknown state, never complete |
| mount/volume boundary | walker stops with diagnostic and never counts another filesystem/volume |
| deep/fanout fixture | 10k-entry fixture completes within recorded budget/p95 target without recursive Rayon explosion |
| rank semantics | byte size is primary; freshness tiebreaker only applies on equal size; id stabilizes final ties |
| estimate caveat | hardlink/sparse warnings remain visible without claiming allocated bytes |

Run dynamic volume/mount tests where the CI host can create the fixture. If it
cannot, record the unsupported environment and keep the platform evidence as a
no-go for that boundary assertion. Run `cargo test --all-targets`, benchmark
recording, and `just ci`.

## Review and rollback points

- Review every stop path to ensure it emits a diagnostic/completeness change.
- Do not add a global basename prune for `cache` or other generic names.
- Do not reintroduce nested Rayon task creation through helper recursion.
- Verify the renamed tiebreaker is not described or exposed as a primary score.
