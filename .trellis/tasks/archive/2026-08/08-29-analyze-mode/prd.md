# Deliver Analyze mode

## Goal

Own the read-only disk-analysis domain, IPC, CLI/TUI/Desktop presentation, truthful size evidence, and treemap interaction.

## Requirements

- R1: Deliver Analyze as a read-only vertical slice with an evidence-bearing
  bounded filesystem snapshot, versioned CLI/IPC DTOs, TUI hierarchy, and desktop
  hierarchy plus treemap. It must never create CleanupPlan or offer deletion.
- R2: Reparse points are not followed; inaccessible, changed, canceled, budget-
  limited, and unsupported entries remain explicit partial/unknown evidence.
- R3: The core/IPC child completes before the presentation child. The umbrella
  owns dependency, schema handoff, recursive evidence, and rollback only; it does
  not duplicate implementation ownership.
- R4: Accept only the frozen Analyze budgets: 250,000 nodes, 256 MiB accounted
  owned memory, two workers, bounded progress, 512 treemap tiles plus one `Other`,
  200 list rows per page, and the fixture/latency/cancellation gates in the child
  designs. Exceeding a core bound returns `partial_budget` lower-bound evidence.

## Acceptance Criteria

- [ ] AC1 (R1, R3): Both children pass their PRDs, designs, manifests, focused gates, and
      independent review; schemas and evidence semantics match exactly.
- [ ] AC2 (R1, R2): CLI, TUI, and desktop report equal totals/evidence for the same frozen
      fixture and locale-invariant machine output.
- [ ] AC3 (R1, R2, R4): Native Windows cancellation, reparse, permission-denied, live-file
      churn, large-tree resource, hierarchy, treemap, keyboard, and scaling
      evidence passes without any cleanup authority.

## Out of Scope

- Duplicate scanner implementation, execution actions, storage monitoring,
  privileged access, historical disk snapshots, or cross-host validation claims.
