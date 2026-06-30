# Design - size/age ranking and freshness guard

## Summary

Add a small plan-ranking module that post-processes `CleanupPlan.targets` after scanners have produced safe `CleanTarget` values:

1. apply a 7-day freshness guard that can only turn `selected_by_default` from true to false;
2. append explainable evidence when that guard changes a target;
3. sort targets by descending `estimated_bytes` with deterministic tie-breakers;
4. expose a tested `size x age` score helper for the later sort-mode UI/CLI work.

This keeps scanner/provider code focused on discovery and keeps TUI/CLI consumers on one shared ordering contract.

## Boundaries

- New backend module: `src/ranking.rs`.
- Export from `src/lib.rs` as `pub mod ranking;` because both binary and TUI code need it.
- Call the post-processor at plan assembly boundaries:
  - `ProjectScanner::scan_roots` after `dedupe_targets`.
  - `GlobalProviderScanner::scan` / `scan_with_probe` before returning.
  - `run_staged_scan` after project/global plans are merged, so mixed-scope TUI output is globally size-ranked.
  - `run_scan` after project/global targets are merged, so `scan --json` matches TUI final order.

The processor must not run cleanup actions or inspect additional filesystem state. It only reads fields already stored in `CleanTarget`.

## API Shape

Suggested public surface:

```rust
pub const DEFAULT_FRESHNESS_FLOOR: Duration = Duration::from_secs(7 * 86_400);

pub fn rank_cleanup_plan(plan: &mut CleanupPlan);
pub fn target_score(target: &CleanTarget, now: SystemTime) -> f64;
```

Internal helpers:

```rust
fn apply_freshness_guard(target: &mut CleanTarget, now: SystemTime, floor: Duration);
fn target_age(target: &CleanTarget, now: SystemTime) -> Option<Duration>;
fn sort_targets(targets: &mut [CleanTarget], now: SystemTime);
```

`rank_cleanup_plan` should capture `SystemTime::now()` once and pass it to both guard and score calculations. Tests can use a `rank_cleanup_plan_at(plan, now, floor)` helper kept `pub(crate)` or private inside `#[cfg(test)]` if public API pressure is not needed.

## Sorting Contract

Current deliverable sorts by size, not score:

1. `estimated_bytes` descending.
2. `size x age` score descending as a secondary signal when sizes tie and mtimes are known.
3. `last_modified` older first when score is unavailable/equal.
4. `id.as_str()` ascending as the final deterministic tie-breaker.

The first key satisfies the user-visible "large targets first" requirement. The score helper is still tested now, but score does not displace a much larger recent target in this task.

## Freshness Guard

Default floor: 7 days, matching the putzen reference.

Rules:

- If `selected_by_default` is false, leave it false.
- If `last_modified` is missing, leave the existing default unchanged.
- If `now.duration_since(last_modified)` errors because the mtime is in the future, treat age as `Duration::ZERO`.
- If age is lower than the floor, set `selected_by_default = false`.
- Do not change `risk`, `action`, `reversible`, or `estimated_bytes`.

Explainability uses the existing `Evidence` enum to avoid changing JSON shape:

```rust
Evidence::RuleMatched {
    rule_id: "ranking.freshness_guard.7d".to_string(),
}
```

TUI Details already renders evidence lines, so no dedicated UI field is required for this child task.

## JSON Compatibility

No new fields are added and no enum variants are needed. JSON output changes only in:

- target order,
- `selected_by_default` for fresh targets that were previously selected,
- evidence list gaining a `rule_matched` item for targets changed by the guard.

This is compatible with `CLEANUP_PLAN_VERSION = 1`; no version bump is required.

## Interaction With Existing Code

- `dedupe_targets` must keep its current path-depth sort before dedupe so parent cleanup paths still win over nested children. Apply ranking after dedupe.
- `ScanSnapshot::targets()` currently chains project targets then global targets. Either rank the combined `Vec` there or rank immediately after rebuilding from the snapshot. Do not sort project/global independently in the final TUI list.
- `default_selected_ids` remains unchanged; it should consume the post-guard `selected_by_default` and keep the current-exe safety filter.
- `selected_cleanup_plan` may continue to mark manually selected TUI targets as `selected_by_default = true`; freshness guard is an auto-selection default, not a hard execution ban after explicit user selection.

## Tests

Backend:

- ranking sorts targets by descending `estimated_bytes`.
- equal sizes use deterministic tie-breakers.
- score is `size_MiB x age_days`, returns 0 for missing mtime, and handles future mtime.
- freshness guard deselects recent default-selected targets.
- freshness guard preserves stale selected targets and already-unselected targets.
- `ProjectScanner::scan_roots` emits ranked output after dedupe.
- `GlobalProviderScanner` emits ranked output with `FakeProbe` sizes.

TUI:

- scan progress merge uses globally ranked targets instead of project-then-global order.
- default selected ids exclude a recent target after the guard but still allow explicit manual selection.

CLI:

- a focused `run_scan` integration-style test is optional because CLI is not currently structured for direct stdout capture. Module tests around scanner/provider/TUI are sufficient unless implementation naturally extracts a testable plan assembly helper.

## Risks And Rollback

- Order changes affect snapshot-style consumers of `scan --json`. This is expected behavior, but tests should avoid relying on discovery order where order is not the behavior under test.
- Freshness is based on subtree newest mtime from `fs_size::estimate_tree`; this means a single recently written file keeps the whole cleanup target fresh. That is conservative and matches the safety goal.
- If sorting at all assembly points causes duplication, keep `ranking::rank_cleanup_plan` idempotent and cheap; repeated calls should only preserve sorted output and avoid duplicate freshness evidence.
