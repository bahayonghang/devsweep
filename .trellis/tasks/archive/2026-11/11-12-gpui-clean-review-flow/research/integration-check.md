# Independent Clean integration check, 2026-11-20

Reviewed the uncommitted integration of `264e7ac` and `324e72b` in
`D:\ds-integrate-1120`. No additional Clean product changes were needed.

## Product paths checked

- Four files matched `324e72b` under `git diff --quiet`: `view/page_clean.rs`,
  `clean_workbench.rs`, `clean_map.rs`, and `ui.rs`. Inspected the merged
  `view.rs` and `shell.rs` separately, including focused-control activation,
  Enter at the Clean root, both Tab directions and row paging (48 rows).
- The normal action path reaches `CleanWorkbench::begin_execute` only after
  preview. The snapshot binds the selected IDs, plan epoch and live core
  digest. Selection changes discard the preview. Both saved-plan projection
  and the background worker reject intents outside the recycle set. Command
  and InspectOnly entries cannot be executed through this path. The worker
  validates the plan and uses the same core executor as the CLI.
- Unconfirmed All scopes project scanning to the explicit roots. Global
  confirmation is consumed before dispatch. Rescan does not replay consent.
  Full-group summaries keep their original scope across search and paging.
- Tab and Shift+Tab use the GPUI focus order. The page and list scroll handles
  reveal the focused control. The group checkbox exposes its native role,
  stable ID and Toggle state, including Mixed. Pressed buttons change fill.
  The footer remains outside the scrolling content. Native UIA values and
  keyboard results belong in the acceptance record.
- The page computes distinct landing, scanning, found, review, previewed and
  completion summaries. Read-only preview scenes identify themselves and
  cannot generate a digest. Real completion groups retain the execution report,
  count successful Recycle Bin outcomes and qualify incomplete audit evidence.
  Durable comparisons require a single isolated run and its corresponding
  audit entries. A screenshot of a synthetic completion proves presentation only.

## Python gate

Initial isolated run failed one of 23 tests. Git returned 1 for
`.trellis/hooks.local.json`. In the original checkout, a separate read-only
probe yielded 128 for `target/debug/devsweep.exe` because `target` is a
symlink. Both workspace target symlinks were ignored there only by
`.git/info/exclude`. The existing directory-only `target/` pattern does
not cover the link itself.

Added repository exclusions for the two workspace target entries and the
local hook file. The test probes each workspace entry itself, since Git
cannot address descendants across a symlink. An ordinary nested Cargo
output still exercises the directory rule. Source-path assertions now
require Git's documented unmatched exit status and reject command errors.

The full Python suite passed (23 tests, 0 failures). The two policy tests
also passed in three fresh fixture layouts with global exclusions disabled:
absent targets, directory targets, and real directory symlinks. The
temporary repository was removed. The previously observed skill-distribution
directory rename error did not reproduce in this run.

## Verification boundary

The four Clean files passed rustfmt. The first whole-GPUI formatting attempt
during this check overlapped the host-actions agent adding a test module and
stopped on its then-missing file. The parent session owns integrated Rust
gates and native acceptance. No screenshot, UIA state or real fixture move
was credited by this source review.
