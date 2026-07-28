# Design: Bounded Walkers and Explicit Ranking

## Traversal contract

Discovery and size estimation use iterative walkers with an explicit budget:
maximum depth, entries visited, per-root deadline, mount/volume boundary policy,
and a cancellation observation point. The walker checks budget/cancellation at
bounded intervals and returns a structured partial diagnostic when any limit is
reached. It never silently reports a partial traversal as complete.

Default limits are internal policy constants, documented with the 10k-entry p95
target and measured in CI/benchmarks. They are not new user-facing CLI flags in
this child. A no-op cancellation observer is used until
`07-28-true-cancellation` supplies the shared token interface. `07-28-scan-
reliability` owns how partial diagnostics and estimates reach the plan/TUI.

## Discovery rules and pruning

The walker always skips `.git`, `.hg`, and `.svn` metadata subtrees. It no
longer prunes an arbitrary directory merely because its basename is `cache`.
Descent stops only after a directory has been positively recognized as a
cleanup footprint owned by a rule, such as `target`, `node_modules`, or an
approved cache rule; recognition retains the target then prevents scanning its
opaque contents. Project markers inside a normal `cache/` directory remain
discoverable.

At an actual filesystem/volume boundary, discovery and sizing stop descent and
emit a partial diagnostic rather than escaping the authorized root's device.
Platform implementations use a stable device/volume identifier where available.
Link/reparse protections remain in force before any boundary decision.

## Iterative size estimation

`SizeEstimate` from scan-reliability is accumulated by a stack-based walker.
It reports known logical bytes, latest modification time, completeness, and
warnings. The implementation does not recursively submit each child to Rayon.
Rayon may parallelize independent top-level cleanup-target estimates after
discovery; each target has its own budget and diagnostic accumulator.

Logical byte totals intentionally remain file-length totals. Hardlinks may be
counted more than once and sparse files may exceed allocated disk blocks; both
are explicit estimate warnings rather than hidden inaccurate claims. A future
allocated-byte mode is out of scope.

## Ranking semantics (D3)

Plans sort by descending known logical bytes. A named
`freshness_tiebreaker` derived from size and age participates only when bytes
are equal, followed by last-modified and stable target id. The old
`target_score` name is removed because it falsely implies a primary score.
Freshness protection remains separate: recent, incomplete, or unknown targets
do not become default-selected merely through ranking.

## Compatibility and rollback

Budget and mount-boundary events surface as partial diagnostics through the
scan-reliability contract. Existing size-first order is preserved, except for
the clarified helper name. Rollback may restore the old walker only with its
partial-result tests retained; it must not retain a partial JSON/TUI shape that
claims a complete estimate.
