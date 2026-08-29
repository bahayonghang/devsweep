# Bound Filesystem Sizing Concurrency

Parent: `08-29-scan-resource-bounds-app-icon`

## Goal

Lower DevSweep's scan-time CPU and disk pressure with a project-owned hard
ceiling on filesystem-sizing workers while keeping same-host release scan time
within an explicit tolerance of the current unbounded Rayon implementation.

## Confirmed Background

- At the pre-change baseline commit
  `70b17b8c20df179ed071af5ae5c42d117774e5a7`, the only production Rayon fan-out
  in sizing is the root-child `par_iter()` at
  `crates/devsweep-core/src/filesystem/sizing.rs:224-236`; nested recursion is
  sequential. These are historical baseline anchors, not current working-tree
  line claims.
- Project scanning, targeted rescans, global providers, and inventory all call
  this shared sizing boundary. Desktop scan orchestration is already
  single-flight, so a desktop-only semaphore would not address the hot path.
- At that same pre-change commit, the root-parallel branch gives each child a
  fresh local copy of the root budget (`sizing.rs:229-231`), while the nested
  serial branch shares one `remaining` counter (`sizing.rs:238-255`). A correct
  fallback must preserve the baseline per-child-local budget semantics.
- At that same pre-change commit, `serial_estimate_tree` calls the parallel entry
  again (`sizing.rs:637-639`) and is not a serial reference.
- The pre-change test suite covers pre-requested cancellation but has no
  deterministic mid-walk cancellation, low-entry-budget, or low-max-depth
  regression.
- The current paused checkpoint now contains the two-worker cached dedicated
  pool/fallback at live `sizing.rs:65-87`, root dispatch and per-child budget at
  `:274-370`, synchronized pool-ceiling coverage at `:498-525`, and the real
  forced-serial helper at `:875-907`. These anchors describe unaccepted
  in-progress code only; the throughput Measurement Record remains `TBD` and no
  performance AC is thereby verified.

## Requirements

- R1: Dedicated hard-capped sizing pool.

All production parallel root sizing must use one lazy, module-owned dedicated
Rayon pool. Do not configure or use Rayon's global pool for filesystem sizing.
The selected worker ceiling must be a named constant, start with candidate `2`,
and never exceed `4` in this task.

- R2: Semantically equivalent serial fallback.

If dedicated-pool construction is unavailable, production sizing must run an
explicit serial root-child fan-out that:

- creates the same fresh local budget for every root child as the current
  parallel branch;
- checks cancellation before/between children;
- merges the same `SizeEstimate` fields and warnings; and
- never calls the global Rayon pool or panics.

This explicit serial fallback is the only production path allowed to bypass the
dedicated pool.

- R3: Deterministic safety, budget, and cancellation proof.

Add focused tests for both dedicated-pool and forced-serial modes. Tests must
cover exact result parity, a low entry budget, a low max depth, reparse/no-follow
safety, pre-requested cancellation, and cancellation requested after worker
execution has definitely begun.

Mid-walk cancellation must be synchronized through an existing injectable
probe/test seam, not timing luck. Both modes must terminate without deadlock,
return `complete = false`, and include a `Canceled` warning.

- R4: Stable same-host throughput protocol.

Before changing sizing, preserve the unbounded release binary under ignored
`target/devsweep-bench/`. Record the absolute project root, entry count, release
command, logical CPUs, `RAYON_NUM_THREADS` environment state, and observed
global Rayon worker count.

Use exactly:

```powershell
<binary> scan <absolute-root> --projects --json | Out-Null
```

Discard one warm-up per binary, then collect five paired observations with
alternating order (`A/B`, `B/A`, `A/B`, `B/A`, `A/B`). The median is the third
value after sorting each five-sample set. Select the smallest candidate from
`{2, 4}` whose median is at most `baseline median * 1.20`. If neither passes,
return to planning with evidence.

- R5: Internal-only policy and unchanged authority.

Do not add a user setting, environment variable, CLI flag, IPC field,
dependency, or persisted plan/report field. Preserve scan ordering, capacity
semantics, incomplete diagnostics, cleanup authority, and single-flight
behavior.

## Acceptance Criteria

- [ ] AC1 (R1): A synchronized wide-root probe drives and
      observes exactly the selected number of active sizing workers without
      ever exceeding it, and a direct pool assertion reports the same ceiling.
- [ ] AC2 (R2): Forced serial fallback and dedicated-pool
      mode produce equivalent bytes, mtime, completeness, and warning kinds on
      complete and budget-limited fixtures while preserving a fresh local root
      budget per child.
- [ ] AC3 (R2): Repository search and focused tests prove
      no filesystem-sizing path calls the global Rayon pool; only the explicit
      serial fallback bypasses the dedicated pool.
- [ ] AC4 (R3): Pre-requested and synchronized mid-walk
      cancellation pass in both modes with no deadlock, `complete = false`, a
      `Canceled` warning, and return within 250 ms after the blocking probe is
      released.
- [ ] AC5 (R3): Low entry-budget and low max-depth fixtures
      return incomplete estimates with `EntryBudgetExhausted` and
      `MaxDepthReached` evidence respectively; existing reparse tests remain
      green.
- [ ] AC6 (R4): The task records all five paired samples,
      medians, formula, worker counts, selected ceiling, and ratio; the selected
      candidate satisfies `bounded_median <= baseline_median * 1.20`.
- [ ] AC7 (R5): No CLI/IPC/Serde/config contract changes
      appear in the diff, and all project/global/rescan/inventory callers still
      converge on shared sizing.
- [ ] AC8 (R3, R5):
      `cargo test -p devsweep-core filesystem::sizing`,
      `cargo test -p devsweep-desktop scan`, `git diff --check`, and `just ci`
      pass.
- [ ] AC9 (R5): Every pre-existing dirty path outside this child's declared
      change list is preserved and excluded from this child.

## Out of Scope

- Changing entry-budget semantics, depth limits, scan ordering, target
  discovery, ranking, cleanup rules, or capacity reporting.
- A user-selectable performance mode or adaptive CPU-count-based ceiling.
- Replacing Rayon, adding a benchmark framework, or adding dependencies.
- Icon or desktop branding work, which belongs to sibling child
  `08-29-generated-app-icon-integration`.

## Risks and Deferred Items

- The timing result is valid for the recorded host/tree/cache protocol; it is
  not a universal hardware guarantee.
- Actual whole-system CPU/disk percentages remain `UNVERIFIED` unless measured
  separately. The hard worker ceiling and same-host timing are the required
  evidence for this child.
