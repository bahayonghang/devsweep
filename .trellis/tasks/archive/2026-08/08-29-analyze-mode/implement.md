# Implement - Analyze Umbrella Orchestration

This umbrella owns dependency and acceptance coordination only. Do not start it
as a product implementation task. A later explicit approval of the revised tree
is required before either child begins.

## Child order and handoff

1. Complete `08-29-analyze-core-ipc`: freeze the bounded snapshot DTO, evidence
   semantics, CLI serializer, Tauri operation lifecycle, resource accounting,
   nearest-rank cancellation gate, and proof that no DTO creates CleanupPlan.
2. Review the generated fixtures/decoders and record the exact accepted schema
   handoff before `08-29-analyze-tui-desktop-treemap` starts.
3. Complete the presentation child against that immutable DTO only; it must not
   read the filesystem or change core budgets. Apply its nearest-rank 30-sample
   layout/commit gates.

## Umbrella acceptance and rollback

- Perform this review only after both implementation leaves are archived.
  Inspect their product commits plus moved task artifacts under
  `.trellis/tasks/archive/`; do not use archived status alone as acceptance.
- Compare CLI/TUI/Desktop totals, lower-bound evidence, warnings, and locale-
  invariant machine fixtures; review native reparse, permission, churn,
  cancellation, resource, hierarchy, keyboard, and scaling evidence.
- Stop on schema drift, hidden unknown capacity, any cleanup authority, or a
  budget/latency miss. Return the defect to the owning child.
- Register or roll back Analyze atomically. Preserve raw evidence and mark any
  missing native result `UNVERIFIED`; archive children before this umbrella.
- Persist the review at `.trellis/tasks/08-29-analyze-mode/acceptance.md`. Record
  reviewed leaf product commit SHAs and archived task paths, every umbrella AC
  clause and its evidence, every command/exit code/log path, all manual/native
  `UNVERIFIED`, and exactly one overall `PASS` or `FAIL`. Overall `PASS` requires
  no completion-required `UNVERIFIED`. Keep this umbrella active after PASS;
  archive it only after `08-29-five-mode-native-integration` passes.
