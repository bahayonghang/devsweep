# Fix duplicate cleanup targets

## Goal

Global cleanup targets should not show duplicate rows for the same cache
directory in the TUI. When multiple provider commands refer to the same
physical cache path, the plan should expose one user-facing cleanup choice so
the list and summary sizes are understandable.

## Requirements

- Explain why the duplicate rows appear in the current implementation.
- Fix duplicate user-facing targets for identical global provider cache paths.
- Keep scanner/provider layers non-mutating: discovery may run inspect commands
  only and must not execute cleanup commands.
- Keep command-backed cleanup actions represented as separated program and argv
  fields.
- Preserve accurate size summaries so one cache directory is not counted more
  than once.
- Do not change execution semantics for permanent delete or Cargo home
  inspect-only behavior.

## Acceptance Criteria

- [x] A regression test proves npm's single cache path is represented as one
      target instead of two duplicate rows.
- [x] Existing provider tests still cover official command argv shapes for npm,
      pip, pnpm, Yarn, and Cargo home.
- [x] TUI selected/global/project byte summaries count duplicate paths once.
- [x] `just ci` passes.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
