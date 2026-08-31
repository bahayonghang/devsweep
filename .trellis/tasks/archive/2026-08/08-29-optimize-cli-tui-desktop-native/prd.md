# Build Optimize CLI, TUI, desktop, and native evidence

## Goal

Own Optimize presentation, bilingual explanations and refusals, IPC wiring, staged feedback, and native no-UAC/unsupported-OS evidence.

## Requirements

- R1: Render the frozen catalogue consistently in CLI/TUI/Desktop with action
  class, availability, reason, effects, risk, required confirmation, and outcome.
  Guidance and Settings handoffs must be visually distinct from real execution.
- R2: Use a staged mode state machine: checking, ready, selected, previewing,
  preview-ready, confirming, running/launching, and terminal/unknown. A sticky
  summary never calls Settings launch or guidance an optimization completion.
- R3: Provide bilingual keyboard/accessibility behavior, unsupported-OS and no-
  privilege explanations, stale/cancel/timeout/unknown handling, operation-id
  event rejection, and shell-coordinator integration.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3): CLI/TUI/Desktop fixture parity covers every catalogue entry/action
      class, unsupported/refusal state, preview/digest, and terminal outcome.
- [ ] AC2 (R1, R2): Tests prove guidance has no run action, Settings says open/launched,
      only DNS shows executing, confirmation is required, and stale events/plans
      cannot dispatch.
- [ ] AC3 (R3): English/Chinese layout, keyboard/screen-reader, focus, reduced motion,
      target widths/scales, and native process/no-UAC evidence pass.
- [ ] AC4 (R1, R2, R3): Frontend/TUI/CLI tests, desktop build, fixed DNS and Settings native
      scenarios, `git diff --check`, and relevant `just ci` gates pass.

## Out of Scope

- New catalogue entries, health scores, batch automation, privileged workflow,
  fake progress, or representing a Settings page as a completed change.
