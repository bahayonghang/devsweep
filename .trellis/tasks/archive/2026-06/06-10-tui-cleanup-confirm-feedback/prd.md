# Fix TUI cleanup confirmation feedback

## Goal

Make the TUI cleanup confirmation flow visibly responsive and unambiguous. If
the user presses Enter before typing the required confirmation phrase, the
modal should explain why cleanup did not start. If confirmation is accepted,
the cleanup progress modal should be visible immediately and remain observable
even when the cleanup finishes quickly.

## Confirmed Facts

- The confirmation modal already requires exact input matching the displayed
  phrase before emitting `Effect::StartClean`.
- The screenshot shows an irreversible command-backed cleanup confirmation with
  `Type: CLEAN 43.4 GiB` and an empty `Input:` field.
- On a mismatch, the current code writes `Confirmation phrase did not match` to
  logs and leaves the modal unchanged, so the modal itself appears inert.
- The clean worker sends an initial `CleanProgress` event before running the
  executor, then forwards per-target executor progress.
- The event loop drains all queued worker events before rendering. A very fast
  cleanup can therefore process `CleanProgress` and `CleanFinished` in one
  drain cycle, clearing the progress state before it is rendered.
- Existing project safety rules keep command-backed cleanup explicit and keep
  permanent delete disabled.

## Requirements

- Preserve the existing irreversible-command confirmation phrase requirement.
- Show inline confirmation feedback when Enter is pressed with missing or
  incorrect confirmation input.
- Make confirmation instructions explicit enough that "Enter" is not read as
  unconditional execution.
- Show cleanup progress immediately after a valid confirmation, without waiting
  for worker scheduling.
- Keep completed cleanup progress visible long enough to be observable when the
  cleanup finishes quickly.
- Preserve existing cleanup execution behavior, command argv separation, and
  cleanup safety boundaries.
- Add focused regression tests for invalid confirmation feedback, immediate
  accepted-confirmation progress, and fast progress/final event ordering.

## Acceptance Criteria

- [x] Pressing Enter with empty or incorrect confirmation input keeps the
      confirmation modal open and shows inline guidance in that modal.
- [x] Valid confirmation input starts cleanup and immediately renders a cleanup
      progress modal with `0 / total` progress before worker events arrive.
- [x] If `CleanProgress` and `CleanFinished` are handled before the next draw,
      the cleanup result remains visible in the progress modal instead of
      disappearing.
- [x] Existing command preview, dry-run, confirmation phrase, and audit result
      behavior remain unchanged.
- [x] Focused TUI tests cover the new behavior.
- [x] `just ci` passes.

## Out of Scope

- Removing the confirmation phrase for irreversible command-backed cleanup.
- Adding new cleanup providers or changing scanner target selection.
- Changing cleanup plan serialization or CLI cleanup behavior.
- Enabling permanent delete.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
