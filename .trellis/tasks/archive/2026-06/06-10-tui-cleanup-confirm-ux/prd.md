# Improve TUI cleanup confirmation UX

## Goal

Make the TUI cleanup confirmation flow understandable, visually deliberate,
and Windows-path friendly without weakening cleanup safety. Users should be
able to see why irreversible command-backed cleanup asks for at most the fixed
word `confirm`, which paths and commands will be affected, and which footer
actions are currently valid.

## Confirmed Facts

- The screenshot shows `Confirm cleanup` over the Global view with two selected
  command-backed cleanup targets: a Rust `cargo clean --manifest-path ...`
  target and an npm `cache clean --force` target.
- The screenshot asks for `CLEAN 45.1 GiB` because `src/tui.rs::confirm_state`
  treats irreversible command-backed actions as a stronger confirmation class
  and builds the required phrase from the selected byte total.
- The product decision for this task is to lower confirmation friction: the
  strongest typed confirmation should be the fixed word `confirm`, not a
  size-dependent phrase such as `CLEAN 45.1 GiB`.
- `src/tui.rs::handle_confirm_key` only emits `Effect::StartClean` after the
  typed input exactly matches `ConfirmState.required_phrase`; invalid Enter
  keeps the modal open and stores visible inline feedback.
- Previous archived tasks already added command argv previews, inline
  confirmation feedback, and cleanup progress visibility. This task should not
  re-solve those finished behaviors.
- UI rendering currently formats paths with raw `Path::display()` in multiple
  places: target titles, details, scope labels, action summaries, evidence, and
  command `cwd` previews.
- `src/scanner.rs` canonicalizes project scan roots. On Windows, canonical or
  verbatim paths can surface as `\\?\D:\...`; because the TUI directly displays
  those strings, implementation-detail path prefixes leak into user-facing UI.
- The footer is a single plain text shortcut strip. It is not context-aware
  enough for confirmation mode and does not visually distinguish active keys
  from labels.
- `render_modal` is generic: fixed centered area, same focused border for every
  modal, and unstructured body text. The confirmation modal lacks a visual
  hierarchy for warning, command preview, required phrase, input, feedback, and
  final action hints.
- Project safety rules require render functions to stay pure, command-backed
  cleanup to keep program and argv separate, and permanent delete to remain
  disabled.

## Requirements

- Preserve display-only cleanup safety: TUI rendering must not scan, execute,
  move, delete, or write audit logs.
- Keep cleanup explicitly confirmed when typed confirmation is required, but cap
  the typed input at the single word `confirm`.
- Explain `confirm` in the modal as a low-friction safety gate for selected
  irreversible commands, not as a mysterious input chore.
- Use `confirm` as the only typed confirmation word. Do not keep a separate
  `clean` phrase for trash-backed cleanup if that flow still asks for typed
  input.
- Normalize Windows verbatim path prefixes for UI display, including
  `\\?\D:\...` and `\\?\UNC\server\share\...`, without changing the `PathBuf`
  values used by executor, scanner, audit, or plan serialization.
- Reuse one display-path helper across target titles, details, scope labels,
  action summaries, evidence summaries, command previews, and any path-like
  modal copy touched by this task.
- Replace the footer shortcut sentence with a compact context-aware key/action
  bar that renders key "pills" and labels with semantic styling.
- In confirmation mode, the footer should show only confirmation-relevant
  actions such as Enter, Esc, Backspace, and Ctrl-C; it should not keep
  advertising normal commands like scan/filter/help as if they are active.
- Improve the confirmation modal hierarchy: warning tone, selected summary,
  command preview section, required phrase, typed input, inline feedback, and
  explicit action hints should be visually distinct.
- Keep the palette restrained and aligned with the existing terminal theme:
  accent for focus, amber for caution, red/pink for destructive or invalid
  states, muted text for secondary hints.
- Add focused tests for path display normalization, context-aware footer
  rendering, and confirmation modal copy/styling behavior.
- Keep `just ci` as the final gate before implementation is reported complete.

## Acceptance Criteria

- [ ] A Windows verbatim project path such as
      `\\?\D:\Documents\Code\Rust\Exp\devsweep\target` renders in TUI text as
      `D:\Documents\Code\Rust\Exp\devsweep\target`.
- [ ] A Windows verbatim UNC path such as
      `\\?\UNC\server\share\cache` renders as `\\server\share\cache`.
- [ ] Path normalization is UI-only; selected cleanup plans and executor command
      requests still receive the original program, argv, cwd, and path values.
- [ ] Any typed cleanup confirmation requires typing exactly `confirm`, not
      `CLEAN <size>`, `clean`, or any size-dependent phrase.
- [ ] The confirmation modal explicitly states that typed confirmation is
      required because the selection includes irreversible command-backed
      cleanup.
- [ ] Command previews remain argv-oriented and do not become shell-composed
      strings.
- [ ] The footer renders styled key/action items instead of one flat shortcut
      sentence.
- [ ] The footer changes by mode: normal, filter, confirmation, running cleanup
      progress, and finished cleanup progress each expose only relevant actions.
- [ ] Confirmation modal rendering has tests proving the warning copy,
      required phrase, input row, feedback row, and action hints remain visible.
- [ ] Existing dry-run behavior, progress behavior, permanent-delete rejection,
      command argv separation, and audit reporting do not regress.
- [ ] `just ci` passes.

## Out of Scope

- Adding new cleanup providers or target discovery rules.
- Changing cleanup plan JSON serialization.
- Changing executor semantics, audit formats, or command execution behavior.
- Enabling permanent delete.
- Adding mouse support or true clickable buttons in the terminal UI.
- Replacing the whole TUI layout or splitting `src/tui.rs` unless required by
  reviewability.

## Product Decision

Resolved on 2026-06-10: lower friction. The strongest typed confirmation should
be the fixed word `confirm`. The implementation should not ask the user to type
`CLEAN <size>` or any other dynamic phrase.

## Notes

- This is a complex task because it touches safety copy, Windows display
  formatting, mode-specific footer behavior, and modal hierarchy. It therefore
  needs `prd.md`, `design.md`, and `implement.md` before `task.py start`.
