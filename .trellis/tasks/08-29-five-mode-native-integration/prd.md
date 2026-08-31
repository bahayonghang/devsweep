# Integrate and validate the five-mode Windows product

## Goal

Integrate all five modes, run cross-mode safety and resource gates, validate bilingual native Windows behavior, update documentation, and prepare release evidence without publishing.

## Requirements

- R1: Integrate only child modes whose recursive acceptance criteria and focused
  gates pass. No feature flag, placeholder, hidden route, or stale command exposes
  an incomplete mode.
- R2: Run the full cross-mode operation-coordination matrix, plan/digest
  separation, cancellation/join, stale-event, crash/restart, mixed audit-version,
  locale switch, and unavailable-capability scenarios.
- R3: Validate the complete breaking CLI, TUI, and packaged Tauri application on
  Windows as a standard user with no UAC or privileged child. Record antivirus
  observations separately from correctness evidence.
- R4: Measure idle, snapshot, live Status, Clean scan, Analyze, Software
  inventory, and Optimize operation CPU/memory/thread behavior using the frozen
  sampling and threshold protocol in the design. No heavy modes overlap; scan
  sizing must retain its accepted throughput/resource envelope.
- R5: Complete English/Chinese docs, migration guide, safety/capability matrix,
  screenshots/evidence, provenance, packaging, and rollback notes. Do not push,
  publish, or release without separate authorization.

## Acceptance Criteria

- [ ] AC1 (R1, R2): Recursive task validation and independent plan/code audits have no
      unresolved blocker; all mode and foundation gates are linked to evidence.
- [ ] AC2 (R1, R2): `just ci`, desktop web/build/package gates, CLI/TUI integration,
      versioned fixture tests, `git diff --check`, and generated checks pass.
- [ ] AC3 (R2, R3, R5): Native Windows evidence covers both languages, keyboard-only use,
      100/125/150/200% scaling, 390/800/1024/1440 layouts, cancellation, sleep/
      resume where applicable, app restart, taskbar/window icon, and no UAC.
- [ ] AC4 (R2, R4): Cross-mode resource runs prove one heavy operation in flight, no
      orphan thread/process after navigation or exit, truthful partial/unknown
      output, and every numeric threshold in the frozen protocol passes.
- [ ] AC5 (R5): Repository docs describe exactly shipped capabilities and rejected
      actions; dirty unrelated files, including the recorded pre-existing
      `README.md` hunks, are preserved by hunk ownership and delivery remains local.

## Out of Scope

- New product scope, dependency additions, admin mode, publishing, signing,
  telemetry, background agent, or fixing unrelated repository dirt.
