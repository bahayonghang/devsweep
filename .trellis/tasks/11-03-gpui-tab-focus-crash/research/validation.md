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

## Follow-up: full-suite failures (commit `1031091`)

Linux host only. Windows test run is NOT_RUN.

- Failures 1 and 2 (CRLF needles): confirmed. All `desktop-gpui/src` files
  were converted to CRLF on Linux. Under that conversion, HEAD `27c91c3`
  failed exactly
  `clean_review_keys_are_bound_on_the_note_and_startup_keeps_note_focus` and
  `every_clickable_surface_is_a_tab_stop_and_enter_clicks_the_focused_one`
  (147 passed, 2 failed). Source-contract tests now read sources through
  `source_lf`, which folds CRLF to LF. The same CRLF conversion then passes
  149 of 149.
- Failure 3 (`confirm_refuses_a_stale_snapshot_or_a_changed_file_before_any_move`):
  cause inferred from source, not reproduced on Windows. The live identity in
  `analysis/saved.rs` is the file id plus millisecond mtime. File size is
  not part of it. Windows stamps writes from a coarse clock tick. The test
  rewrote the file in place right after the preview, so the rewrite could
  keep both values. The digest then matched and the executor went on to a
  real Recycle Bin move of the temp file. That may also explain the
  `STATUS_ACCESS_VIOLATION` (a shell recycle call from a test thread), but
  this is unconfirmed. The test now moves the mtime forward explicitly. If a
  `devsweep-gpui-analyze-confirm-changed-*` `keep.bin` sits in the Recycle Bin
  from the earlier run, it is that test file.
- Product note, not changed here: the same-tick in-place rewrite gap also
  applies to the CLI Analyze confirm path. Hardening the core identity is
  separate backend work.

Gates on Linux: `cargo test --features linux-preview` 149 passed, 10 ignored.
`cargo fmt --all -- --check` is clean. Windows-target clippy
(`--all-targets -D warnings`) passes. A full suite run on Windows is still
required before this task can close.

User report, 2026-11-03: after `1031091`, `f96feee` and `1e55815`, the full
GPUI suite passed on the user's Windows host. The run log is not in this
directory. The task status was left unchanged for the owner to close.
