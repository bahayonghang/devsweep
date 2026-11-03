# Validation

## Native Windows debug runtime

Entry point: `RUST_BACKTRACE=1; just gpui-dev`, using the existing locked
dependencies and mbx-managed debug target. Accessibility enabled via UIA.

- Before fix: click Software from Clean -> exact reported a11y assertion,
  `Interactivity::prepaint` -> `A11y::set_active_descendant`, exit 1.
- After fix: click Software, Optimize, Analyze, Status, then Clean. Every
  destination rendered and UIA reported the corresponding tab as focused.
- Repeat click on selected Clean -> remains focused/rendered, no panic.
- Tab from Clean -> Software focus, Clean panel remains until Enter.
- Enter -> Software panel, Software focus retained.
- Shift+Tab -> Clean focus, Software panel remains until Enter.
- Enter -> Clean panel, Clean focus retained.
- Alt+F4 -> test window closes, `just gpui-dev` exits 0 with no panic.

Screenshots and UIA observations were inspected through the computer-use tool.
No scan, cleanup, uninstall, optimization action, or live sampling was started.

## Source sweep

The only application-owned `aria_active_descendant()` use was the main mode
tab builder. Shared segments (`ui::segment`) already use real focus plus
`aria_selected` without an active-descendant override.

## Automated checks

- `mode_tabs_use_real_focus_without_active_descendant_override`: failed before
  the fix and passed after it. This is a source-contract guard; the native
  runtime checks above provide the behavioral evidence.
- Cargo formatting: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.
- Independent Trellis check review: no actionable findings in the fix or spec.

The full GPUI test suite did **not** pass. It printed failures for:

1. `view::tests::clean_review_keys_are_bound_on_the_note_and_startup_keeps_note_focus`
2. `view::tests::every_clickable_surface_is_a_tab_stop_and_enter_clicks_the_focused_one`
3. `analyze_window::tests::confirm_refuses_a_stale_snapshot_or_a_changed_file_before_any_move`

The first two contain unchanged LF-only source needles. A probe against HEAD
with CRLF, matching `git ls-files --eol` for this checkout, confirmed those
needles fail while the LF HEAD text matches. The Analyze cause is unconfirmed.
The harness exited with `STATUS_ACCESS_VIOLATION` (0xc0000005) before a final
summary or detailed assertion messages. Follow-up runs encountered a missing
hashed test executable after compilation. Do not infer a full-suite pass or
attribute the Analyze/harness problems to a confirmed cause from this run.
No unrelated test or tooling repairs were applied.

The reported navigation crash is fixed and natively verified. The broader test
gate remains incomplete, so the Trellis task remains unarchived. Changes are
uncommitted.
