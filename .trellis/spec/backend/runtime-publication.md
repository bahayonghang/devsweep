# Scan publication and bounded sizing

## 1. Scope / Trigger

Apply when changing `scan::Sweeper`, CLI scan observation, desktop scan progress
consumption or the shared no-follow size walker. These optimizations change work
and publication cadence, not discovery coverage, final ranking or cleanup
authority.

## 2. Signatures

- `Sweeper::with_progress_policy(ScanProgressPolicy)` selects `TerminalOnly` or
  the default `Interactive` policy.
- `validate_scan_preview_plan(&CleanupPlan) -> Result<()>` validates internal
  preview identities without constructing display targets.
- `scan_preview_from_plan(&CleanupPlan) -> Result<ScanPreviewSnapshot>` validates
  and projects the authority-free display DTO.
- Shared sizing retains `SizeEstimate`,
  `estimate_tree_with_budget_and_cancel(path, entry_budget, cancel)` and its
  reparse-probe variant. Root child batches are an internal implementation
  bound, not a smaller scan budget.

## 3. Contracts

- One-shot CLI scans select `TerminalOnly`: no intermediate ranked/cloned plans
  or observation events are constructed for their no-op consumer. Discovery,
  health, deduplication, final ordering and selection facts remain identical.
- Interactive publication sends the first discovery in each phase and then at
  a 100 ms cadence, with phase/cancellation boundary flushes. Provider callbacks
  still observe every completed target and cancellation; the publisher decides
  whether a cumulative snapshot is needed. It never drops final candidates.
- Every published snapshot is cumulative, deduplicated and ranked by `Sweeper`.
  Consumers replace observations and never merge/rank them independently.
- Rank/dedupe the cloned observation, never the shared accumulator during
  discovery publication. The freshness guard can irreversibly clear default
  selection; observer cadence must not change final selection when a target
  crosses that time boundary. Phase/final ranking remains shared by policies.
- Directory sizing checks cancellation while enumerating names. Nested names
  are visited immediately; root names are processed in batches of at most 64
  through the existing two-worker pool. No scan-wide cache or extra pool is
  introduced.
- Keep each package root and fan-out root child independently stealable within
  that pool. A blocked item must not trap ready siblings inside its worker's
  serial iterator fold. This does not parallelize deeper walks or change their
  local budgets; indexed result order remains stable.
- Preserve a fresh local budget for each fan-out root child, including the last
  singleton batch. A root with exactly one child keeps its existing reviewed
  budget. Depth, no-follow, partial-size and warning semantics remain unchanged.
- Cancellation observed as the root iterator ends must still visit its pending
  batch through the cancel-aware walkers so the result is marked incomplete;
  never skip the pending names and return a complete lower byte count.
- Project traversal already streams directory entries. Keep per-entry cancel
  checks for file-only directories instead of introducing a redundant buffering
  rewrite.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Terminal-only versus interactive policy, equal input | Equal final report; terminal policy publishes no snapshots |
| Freshness boundary crossed after an observation | Observation ranking cannot change later final selection |
| Burst of targets within one interval | Bounded published snapshots; final/phase snapshot includes all targets |
| Invalid preview identity | Fail the scan; never hide malformed input |
| Cancellation while enumerating only files | Stop requesting entries promptly; report partial/canceled sizing |
| Root fan-out exceeds one batch | Same child coverage and budgets; bounded retained names |
| One root child blocks while another worker is free | Ready siblings can run; release and join the blocked worker before any test failure |
| Cancellation on root iterator exhaustion | Pending singleton/partial batch yields incomplete sizing |
| Missing metadata, denied directory or reparse uncertainty | Preserve typed warning and conservative size confidence |

## 5. Good / Base / Bad Cases

- Good: a 10k-target terminal report constructs no unused cumulative snapshots.
- Base: an interactive scan flushes a small completed phase immediately even if
  less than 100 ms elapsed since the last discovery.
- Bad: reducing discovery budgets to make a benchmark faster, or using partial
  progress as a saved cleanup plan.
- Bad: an unbounded channel is called bounded because its upstream publisher is
  time-throttled.

## 6. Tests Required

- Policy equality, first/periodic/boundary publication and deterministic
  constructed-work counts; cancellation must keep partials non-authoritative.
- Wide file-only enumeration with deterministic maximum retained-name counts,
  cancellation during iteration, chunk boundaries, singleton reviewed budget,
  existing worker ceiling and no-follow/permission/size-confidence regressions.
- A bounded scheduling gate proves ready sibling work advances while one child
  is blocked. Always release and join before asserting, including on timeout.
- Paired immutable reference/candidate final reports at equal fixture coverage;
  preserve all timing/resource samples and failed prechecks separately.

## 7. Wrong vs Correct

Wrong: a consumer merges or re-ranks partial observations itself, or saves a
partial progress snapshot as a cleanup plan.

Correct: replace the previous observation with each cumulative snapshot from
`Sweeper`, validate preview identities with `validate_scan_preview_plan`, and
take selectable cleanup facts only from the matching completed report.
