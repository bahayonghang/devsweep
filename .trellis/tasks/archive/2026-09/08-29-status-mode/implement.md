# Implement - Status Umbrella Orchestration

This umbrella owns schema, lifecycle, resource, and rollback coordination only.
Do not start it as a product implementation task. Child work requires a later
explicit approval of the revised task tree.

## Child order and handoff

1. Complete `08-29-status-collector-cli`: freeze availability DTOs, delta
   calculation, snapshot/live cadence, process/privacy bounds and truncation
   metadata, skipped-tick/sequence/terminal and cancel/join behavior, and
   locale-invariant JSON/NDJSON matching the CLI V1 schema byte-for-shape.
2. Review generated fixtures/decoders and record the exact stream/schema handoff
   before `08-29-status-tui-desktop-native` starts.
3. Complete TUI/Desktop presentation against the frozen stream only, with
   explicit live start, bounded in-memory charts, privacy-safe process fields,
   unavailable states, shell mutual exclusion, and cancel-on-leave.

## Umbrella acceptance and rollback

- Perform this review only after both implementation leaves are archived.
  Inspect their product commits plus moved task artifacts under
  `.trellis/tasks/archive/`; do not use archived status alone as acceptance.
- Compare CLI/TUI/Desktop fixture values, primitive types/units/time bases,
  truncation metadata, event sequences/terminals, and availability states, then apply the
  exact R4 snapshot/live/post-exit thresholds on the recorded release-build host.
- Stop on synthesized zeros, background/history persistence, overlapping
  samples/heavy work, sensitive process fields, resource-threshold failure, or
  missing cancel/join evidence; return defects to the owning child.
- Register or roll back Status atomically and discard only ephemeral samples.
  Keep missing native evidence `UNVERIFIED`; archive leaf children before this
  umbrella.
- Persist the review at `.trellis/tasks/08-29-status-mode/acceptance.md`. Record
  reviewed leaf product commit SHAs and archived task paths, every umbrella AC
  clause and its evidence, every command/exit code/log path, all manual/native
  `UNVERIFIED`, and exactly one overall `PASS` or `FAIL`. Overall `PASS` requires
  no completion-required `UNVERIFIED`. Keep this umbrella active after PASS;
  archive it only after `08-29-five-mode-native-integration` passes.
