# Implement Analyze core, contracts, and IPC

## Goal

Own bounded read-only traversal, evidence-bearing size DTOs, cancellation, CLI serialization, and Tauri IPC for Analyze.

## Requirements

- R1: Add a separate read-only Analyze domain that accepts an exact local root,
  records volume/root identity, never follows reparse points, and aggregates file
  and directory lower-bound sizes into an immutable versioned snapshot.
- R2: Bound traversal to a dedicated two-worker pool, one active job, cooperative
  cancellation, at most 256 progress nodes or 100 ms per batch, 250,000 stored
  nodes, 10,000 warnings, and 256 MiB (268,435,456 bytes) of accounted owned
  memory. Do not use the global Rayon pool. Hitting a node/memory/warning bound
  stops scheduling, joins workers, and returns `partial_budget` lower-bound
  evidence.
- R3: Each node carries stable snapshot-local id, parent id, kind, name, size,
  immediate/recursive counts, evidence state, warnings, and optional timestamps;
  it carries no cleanup target/action. Churn, access denial, I/O error, cycle,
  and reparse are distinguishable.
- R4: Implement `analyze scan` human/JSON output through the owned
  `application/presentation/analyze.rs` renderer and typed Tauri start/cancel/
  progress/complete IPC. Paths stay on input/audit boundaries and are escaped;
  progress and late completion are operation-id scoped.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3): Fixtures cover wide/deep trees, sparse files, hard links policy,
      reparse loops, inaccessible/deleted/growing files, cancellation, node and
      memory ceilings, worker ceiling, and pool construction fallback.
- [ ] AC2 (R2, R3): Complete snapshots reconcile parent/child totals; partial snapshots
      remain lower bounds with stable warning classes and no fabricated totals.
- [ ] AC3 (R3, R4): CLI JSON and IPC fixtures match the same versioned DTO, remain locale-
      invariant, bilingual human snapshots use the owned renderer, and no output
      contains an executable or CleanupPlan conversion path.
- [ ] AC4 (R1, R2, R4): Focused tests, resource measurements, cancellation latency, `just ci`,
      and Windows native root/permission evidence pass.

## Out of Scope

- Treemap/UI, duplicate-file analysis, content hashing, cleanup recommendations,
  network shares, reparse traversal, persistent history, or administrator access.
