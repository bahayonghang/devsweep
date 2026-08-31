# Design - Status Delivery Gate

`status-collector-cli` owns the Win32 adapters, delta calculation, scheduler,
bounds, and machine output implementation but implements the CLI-owned
`AvailabilityV1`, `StatusSnapshotV1`, and four live event shapes verbatim.
`status-tui-desktop-native` consumes that frozen stream and owns only in-memory
presentation history. The umbrella
compares adapter fixtures and native resource traces, verifies coordinator
mutual exclusion, applies the exact snapshot/live/exit thresholds from R4, and
registers/rolls back Status atomically. A resource miss blocks the mode; it is not
reclassified as a qualitative regression judgment.
