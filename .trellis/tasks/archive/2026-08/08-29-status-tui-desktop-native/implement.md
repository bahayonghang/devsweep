# Implementation Plan - Status TUI, Desktop, and Native Evidence

Implementation remains gated on explicit approval and on completion of the shell
and `status-collector-cli` contracts.

## Steps

1. Re-read the accepted frontend and desktop-frontend specifications, including
   the shell-owned component and state-management conventions.
2. Add TUI presentation against frozen Status fixtures; do not add collectors or
   persistence in this child.
3. Add the Tauri adapter and desktop mode-local store, charts/text alternatives,
   sortable process table, availability/truncation states, exact sequence/
   terminal validation, and 60-point cap. Generate from the shared DTO and
   reject local aliases or widened union variants.
4. Register Status only after focused Rust, TUI, web, lifecycle, and generated-
   binding checks pass.
5. Run native standard-user scenarios and retain raw resource samples using the
   final integration protocol.

## Verification

- `rtk cargo test -p devsweep-cli status`
- `rtk npm --prefix desktop run test -- --run status`
- `rtk npm --prefix desktop run lint`
- `rtk npm --prefix desktop run typecheck`
- `rtk npm --prefix desktop run build`
- `rtk npm --prefix desktop run types:generate`
- `rtk npm --prefix desktop run test -- --run src/api/types-generation.test.ts`
- `rtk just ci`
- Manual native gate: English and Chinese; keyboard-only and screen reader;
  reduced motion/high contrast; 390/800/1024/1440 widths; 100/125/150/200%
  scale; explicit live start, interval change, churn, leave/close, and restart.
- Resource gate: apply `status-mode` R4 and the final integration sampling
  protocol to idle, snapshot, 60-second live, and the exact 25-sample post-exit
  trace; require both thresholds in every final-five sample and fail on a
  missing sample.

## Stop and rollback points

- Stop before registration if fixtures disagree with CLI output, missing values
  are rendered as zero, or any stream survives leave/close.
- Roll back mode registration, Tauri exports, and presentation files as one unit.
  No Status history exists to migrate; delete only ephemeral in-memory samples.
