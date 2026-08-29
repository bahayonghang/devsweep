# Design - Project-Owned Bounded Sizing Pool

## 1. Change List

| Path | Planned ownership |
| --- | --- |
| `crates/devsweep-core/src/filesystem/sizing.rs` | dedicated pool, root fan-out modes, fallback, tests |
| `crates/devsweep-core/Cargo.toml` | no dependency change expected; inspect only |
| `Cargo.lock` | no change expected |
| this task's `implement.md` | measurement evidence only |

No CLI, Tauri, React, model, Serde, or configuration file is expected to change.

## 2. Production Execution Design

### 2.1 Lazy dedicated pool

Own one cached pool initialization result inside `filesystem::sizing`. Build the
pool with `ThreadPoolBuilder::new().num_threads(SIZE_WALK_WORKERS)` and a stable
thread-name prefix. Never call `build_global()` and never fall back to
`children.par_iter()` outside `pool.install(...)`.

Cache failure as well as success so every estimate does not retry pool creation.
The production selector returns either `Dedicated(&ThreadPool)` or
`SerialFallback`.

### 2.2 Root-child semantic owner

Extract the current depth-zero branch into one helper whose input includes the
execution mode. Each child always receives:

```text
local_remaining = min(root_remaining_at_fanout, DEFAULT_SIZE_ENTRY_BUDGET)
```

The dedicated mode maps that child operation through the project pool. The
serial fallback performs the same child operation in deterministic sequence,
checking cancellation before each child. It must not reuse the nested serial
loop because that loop shares one `remaining` counter and is not equivalent to
the existing root-parallel behavior.

Nested (`depth > 0`) recursion remains the current sequential shared-budget
walk. `SizeEstimate::merge` remains the aggregation owner.

### 2.3 Test seam

Keep the execution-mode selector module-private. Tests can explicitly request a
real dedicated test pool or forced serial fallback without manufacturing an OS
thread-creation failure. Production still exercises the cached pool result.

## 3. Deterministic Test Design

### 3.1 Pool ceiling

Use a bounded barrier/probe around at least `SIZE_WALK_WORKERS * 2` independent
root children. Hold workers until exactly `SIZE_WALK_WORKERS` have entered (or a
bounded timeout fails the test), track current and peak workers atomically, then
release them. Assert `peak == SIZE_WALK_WORKERS`, never greater, plus a direct
pool-size assertion. This cannot pass through accidental serial execution.

### 3.2 Mid-walk cancellation

Implement a test-only `PathReparseProbe` wrapper with a condition variable or
equivalent two-phase gate:

1. the worker signals that at least one child reached the probe;
2. the test thread requests cancellation;
3. the gate releases the worker(s);
4. the scan joins and assertions inspect the estimate.

Run the same test through dedicated and forced-serial modes. Measure from gate
release to join and require `< 250 ms`, `complete = false`, and at least one
`SizingWarningKind::Canceled`. Bound waits so a regression fails rather than
hanging the suite.

### 3.3 Budget and depth

Use private configurable limits to run:

- a low-entry-budget wide-root fixture in both modes; and
- a low-max-depth nested fixture.

Compare stable fields and warning-kind multisets rather than warning text order,
because parallel reduction need not preserve message ordering.

## 4. Throughput Measurement Design

Build the pre-change release binary and copy it under ignored
`target/devsweep-bench/unbounded/`. Build candidate binaries under sibling
ignored paths so A/B runs use immutable executables.

Use the exact projects-only command from `prd.md`, one discarded warm-up per
binary, and five alternating pairs. Keep the root and shell output sink fixed.
Record samples in milliseconds and compute each median as the third sorted
sample. Before changing production sizing, add a temporary `#[cfg(test)]` probe
named `reports_current_global_rayon_workers` that prints
`rayon::current_num_threads()`. Run it with:

```powershell
rtk cargo test -p devsweep-core filesystem::sizing::tests::reports_current_global_rayon_workers -- --exact --nocapture
```

Record the output together with the `RAYON_NUM_THREADS` environment state, then
remove the temporary probe before the final diff. The preserved release binary
is unaffected by this test-only probe.

The warm-up and five-pair window is exclusive: no sibling build, test, package,
icon generation, or other CPU/disk-heavy task may run until the last candidate
sample is recorded.

Candidate decision:

```text
try 2 workers
  if median_2 <= median_unbounded * 1.20: select 2
  else try 4 workers
    if median_4 <= median_unbounded * 1.20: select 4
    else: return to planning
```

## 5. Compatibility and Rollback

- No public or serialized contract changes.
- Pool initialization failure reduces performance but preserves truthful scan
  results and cancellation through the semantically equivalent serial root path.
- Rollback restores the current root-level global Rayon branch; no data
  migration exists.
- The child does not authorize changes to the current budget model itself.
