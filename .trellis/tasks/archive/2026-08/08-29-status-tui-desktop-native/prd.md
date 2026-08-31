# Build Status TUI, desktop, and native evidence

## Goal

Own Status cards/process table, bilingual unavailable states, opt-in live lifecycle, IPC wiring, accessibility, and native resource/evidence validation.

## Requirements

- R1: Render snapshot first and start live only by explicit user action. Desktop
  uses compact metric cards, truthful sparklines, and a sortable process table;
  TUI shows equivalent values and availability. Keep at most 60 in-memory points
  and drop them on mode exit.
- R2: Clearly label sampled time, live interval, stale/partial/unavailable/
  permission/unsupported. Unsupported GPU/thermal/fan fields are omitted or
  explanatory—not zero-valued cards. Consume the frozen Status V1 integer
  bytes/basis-points/UTC-monotonic fields and process truncation metadata
  directly; no local wire shape or synthetic health score.
- R3: Integrate shell coordination: live Status is mutually exclusive with Scan,
  Analyze, Software, and Optimize; pause/leave/close cancels and joins. Skip ticks
  without visual backlog or concurrent requests; stale event ids are ignored.
- R4: Both languages, keyboard/screen-reader, reduced motion, high contrast,
  narrow layouts, scaling, and privacy-safe process display must pass native gates.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3): Fake-stream tests cover initial snapshot, live opt-in, interval change,
      skip/stale events, partial groups, no battery, unsupported metrics, process
      churn/truncation/budget flags, exact sequence/terminal handling,
      pause/leave/close, broken-pipe fixture parity, 60-point cap, and no
      persisted history.
- [ ] AC2 (R1, R2): CLI/TUI/Desktop values and availability match frozen fixtures; charts
      never interpolate missing samples as real zeros and expose tabular/text
      alternatives. Generated decoders preserve/reject the exact frozen
      primitive fields, units, time bases, unions, and event variants.
- [ ] AC3 (R4): English/Chinese long labels, keyboard process sorting, focus, screen-
      reader names, target widths/scales, reduced motion, and high contrast pass.
- [ ] AC4 (R3, R4): Native idle/snapshot/live/exit resource traces, process privacy review,
      frontend/TUI tests, desktop build, and relevant `just ci` gates pass.

## Out of Scope

- Persistent charts, alerts, health score, hidden background refresh, task killing,
  process path/user/cmdline, or unsupported hardware values.
