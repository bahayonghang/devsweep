# Improve TUI cleanup confirmation and progress

## Goal

Make the TUI cleanup flow explicit before execution and observable while it is
running. Users should be able to see which cleanup commands will run, how to
confirm or cancel the modal from inside the modal itself, and how far execution
has progressed after confirmation.

## Confirmed Facts

- The TUI already opens a `Confirm cleanup` modal for selected targets.
- The confirmation state currently tracks target count, estimated bytes,
  irreversible command presence, required phrase, and input text.
- The current modal shows the required phrase and input, but it does not list
  command-backed cleanup argv values or key hints inside the modal body.
- The clean worker currently emits one cleanup progress message before calling
  `Executor::run_plan`, then emits a final result. It does not emit per-target
  cleanup progress.
- The executor already iterates selected targets and records audit records per
  target, which is the natural boundary for progress updates.
- Project safety rules require command-backed cleanup to keep program and argv
  separate and permanent delete to remain disabled.

## Requirements

- Show explicit cleanup commands in the TUI confirmation flow for
  command-backed targets.
- Display confirmation key hints in the confirmation modal body, including the
  key that submits a valid confirmation and the key that cancels.
- Show cleanup execution progress after confirmation, including completed and
  total selected target counts.
- Keep cleanup execution behavior unchanged except for progress observation.
  Do not add new cleanup providers, change scanner selection rules, or enable
  permanent delete.
- Preserve the safety model: command preview is display-only; execution must
  continue to pass program and argv separately to the command runner.
- Add or update TUI tests for confirmation copy, command preview, key hints, and
  cleanup progress rendering/state.
- Keep the canonical gate green with `just ci`.

## Acceptance Criteria

- [x] Selecting command-backed cleanup targets opens a confirmation modal that
      shows the explicit command argv preview for those targets.
- [x] The confirmation modal shows inline key hints for confirm and cancel
      actions without relying only on the footer key bar.
- [x] Starting cleanup creates visible progress state that advances per selected
      target and shows `completed / total` progress.
- [x] Cleanup completion still reports the existing audit-log path and
      success/failure counts.
- [x] Dry-run defaults, irreversible-command confirmation phrase behavior, and
      permanent-delete rejection remain unchanged.
- [x] Relevant unit/render tests cover the new UI behavior.
- [x] `just ci` passes before the task is reported complete.

## Out of Scope

- Adding new cleanup actions or providers.
- Changing risk classification, target selection, or scanner discovery.
- Executing cleanup from scanner or model layers.
- Docker cleanup.
- Enabling permanent delete.

## Product Decision

- Runtime cleanup progress should be visible in both places: a transient
  cleanup-progress modal after confirmation and the existing Jobs/Logs panel.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
