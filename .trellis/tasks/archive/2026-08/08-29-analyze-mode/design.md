# Design - Analyze Delivery Gate

`analyze-core-ipc` owns traversal, snapshot schema, CLI serializer, cancellation,
resource bounds, and Tauri commands/events. `analyze-tui-desktop-treemap` consumes
only the frozen DTO and owns navigation/rendering. No UI child reads the
filesystem directly. The umbrella accepts the mode only after fixture parity,
native resource evidence, and explicit proof that no DTO converts to a cleanup
selection or plan. The umbrella rejects any deviation from the 250,000-node,
256-MiB accounted-memory, two-worker, 512+Other-tile, or 200-row-page contract;
budget changes return to planning. Rollback disables the Analyze registration as
one unit.
