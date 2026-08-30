# Implement - Analyze Core, CLI, and IPC

Start only after later approval and completion of the frozen CLI contract. Do
not resume or modify the paused sizing child.

## 1. Add the bounded read-only domain

1. Add compact node/warning/snapshot DTOs and deterministic owned-byte
   accounting before traversal.
2. Implement iterative no-follow traversal with exactly two owned workers,
   serial pool-construction fallback, one job, cancellation, and bounded progress.
3. Enforce node, warning, and byte caps before allocation; on the first exceeded
   cap, stop scheduling, join, and return lower-bound `partial_budget`.

Focused validation:

```powershell
rtk cargo test -p devsweep-core analysis
```

Rollback point: remove `analysis/` and its `lib.rs` export together; do not touch
cleanup sizing or reinterpret a partial snapshot.

## 2. Add exact CLI and Tauri adapters

1. Implement only `analyze scan --root ...` from the frozen parser plus the
   bilingual `application/presentation/analyze.rs` renderer; do not edit either
   CLI-owned module root.
2. Add operation-id/sequence-scoped Tauri start/cancel/progress/complete commands
   and regenerate typed fixtures.
3. Prove English/Chinese human snapshots consume the owned renderer, paths are
   data, no Analyze DTO converts to CleanupPlan, and machine output is locale invariant.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli analyze
rtk cargo test -p devsweep-desktop analyze
rtk just desktop-web-check
```

Rollback point: unregister CLI/Tauri Analyze adapters and remove generated
Analyze fixtures without changing the domain schema used by recorded evidence.

## 3. Resource and native gates

1. Run `analysis-250k-v1` through the fake filesystem adapter and
   `analysis-native-50k-v1` on Windows one warm-up plus five measured runs.
2. Record nodes, accounted bytes, warnings, progress queue depth, wall time,
   process private bytes, worker/thread counts, and cancellation latency.
3. Require deterministic cap behavior, no more than two workers, and p95 cancel
   request-to-join <= 500 ms on the native fixture.

```powershell
rtk cargo test -p devsweep-core analysis --release
rtk git diff --check
rtk just ci
```

Manual evidence covers reparse leaves, permission denial, live-file churn,
cancel, exact-root identity, and standard-user operation. Stop and return to
planning for any budget increase, network-share support, new dependency, or CLI
shape change.
