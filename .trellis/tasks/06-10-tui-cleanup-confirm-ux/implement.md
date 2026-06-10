# Improve TUI cleanup confirmation UX - Implementation Plan

## Assumptions

- User chose the lower-friction confirmation model: typed cleanup confirmation
  should require at most the fixed word `confirm`, not `CLEAN <size>` or
  another mode-specific word.
- Treat "button" styling as terminal key/action pill styling. The TUI does not
  need mouse-clickable buttons.
- Keep implementation scoped to `src/tui.rs` and its existing tests unless a
  small split becomes clearly necessary.

## Steps

1. Add failing tests for path display normalization.
   - Verify: Windows verbatim drive path and UNC path render without the
     `\\?\` prefix.
   - Verify: normal paths remain unchanged.

2. Add failing tests for the lower-friction confirmation phrase.
   - Verify: command-backed cleanup uses `confirm` as
     `ConfirmState.required_phrase`.
   - Verify: trash-backed cleanup also uses `confirm` if it requires typed
     input.
   - Verify: no rendered confirmation modal asks for `CLEAN <size>`.
   - Verify: accepted confirmation still emits `Effect::StartClean` only after
     the exact required word is entered.

3. Add a display-only path formatting helper and route TUI path rendering
   through it.
   - Verify: target title, details path, scope label, action summary, evidence
     summary, and command `cwd` preview use normalized display text.
   - Guardrail: do not use the helper in selected plan construction or executor
     request construction.

4. Add failing tests for footer mode rendering.
   - Verify: normal mode shows compact key/action pairs.
   - Verify: confirmation mode shows Enter/Esc/Backspace/Ctrl-C and omits
     normal-only commands such as scan and filter.
   - Verify: finished progress mode shows close actions.

5. Implement footer action rendering.
   - Create small local helpers for key pill spans and action lists.
   - Keep the footer one row and context-aware.
   - Use semantic styling consistent with existing accent/amber/error colors.

6. Add failing tests for confirmation modal copy and structure.
   - Verify: irreversible command-backed modal explains why `confirm` is
     required.
   - Verify: required phrase, input row, command preview, feedback, and action
     hints remain visible.
   - Verify: command previews still use `argv:` phrasing and do not become
     shell-composed strings.

7. Polish confirmation modal rendering.
   - Specialize `render_confirm` with clearer section ordering.
   - Highlight the required phrase and input without changing the state model.
   - Keep generic `render_modal` for Help, Details, Dry-run, and Progress unless
     a small tone parameter is needed.

8. Regression pass.
   - Run focused tests first, for example `cargo test tui::tests`.
   - Run `just ci`.
   - Manually run or visually inspect `just dev` / `cargo run -- tui` if needed
     to confirm the footer and modal fit a representative terminal.

## Validation Commands

- `cargo test tui::tests`
- `just ci`
- Optional manual smoke: `just dev`

## Rollback Points

- If path normalization causes execution or audit test failures, rollback the
  helper call sites and keep normalization in render-only paths.
- If the footer clips too aggressively in normal mode, prefer fewer
  context-aware actions over returning to the full universal shortcut sentence.
- If modal copy becomes too verbose, keep the safety explanation but shorten
  command preview rows before changing confirmation semantics.

## Do Not Change

- Do not remove explicit confirmation entirely for irreversible command-backed
  cleanup; this task lowers it to `confirm`.
- Do not shell-compose commands for display or execution.
- Do not change scanner/provider/executor semantics.
- Do not enable permanent delete.
- Do not add new cleanup providers.
