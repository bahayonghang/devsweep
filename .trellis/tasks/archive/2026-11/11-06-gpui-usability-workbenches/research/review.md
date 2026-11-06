# Independent integration review — 2026-11-06

Reviewer: separate `trellis-check` agent. Scope: all four children under this
parent, including shared shell/input state and preference preservation.
Product fixes were made after the main agent released source ownership.
No commits, task-state changes, archive operations, or production cleanup were
performed by this reviewer.

## Findings fixed

1. **Shared search could overwrite another mode's query.**
   `view.rs::sync_inputs` restored only Software's query, while the one shared
   `TextInput` observer also reacted to style and label notifications. Switching
   from Software to Clean, Optimize, or Status could therefore publish the old
   field contents into the destination model. Added mode-owned query access and
   update routing in `Session`, restored it before presentation notifications,
   and made Analyze ignore search notifications. Identical notifications no
   longer rebuild a mode's query projection. The regression covers independent
   Clean/Software/Optimize/Status queries, reverse navigation, Settings return,
   and an Analyze notification without altering Clean.

2. **The new font filter exposed missing shared input operations.**
   Main-agent native evidence showed ordinary character typing filtered real
   fonts, but UIA SetValue and clipboard-driven typing did nothing. `TextInput`
   advertised a value but did not handle SetValue; its action set lacked paste
   and select-all. Added UIA SetValue, Ctrl+A, and Ctrl+V to the shared input.
   Replacement/paste use the existing UTF-16 buffer and notify the real input
   observer. Whole-value replacement ignores an old selection or IME mark;
   select-all clears composition before paste. Added Unicode/non-BMP/marked
   text regression tests. Native automation must be repeated on the new build;
   these buffer tests alone do not prove a platform interaction.

3. **A partial Settings save could corrupt retry intent.**
   The initial implementation snapshotted dirty flags correctly for one Save,
   but rebased untouched fields only after all desktop patches succeeded. If
   an early patch returned concurrent unrelated changes and a later patch
   failed, the next Save could treat those untouched stale draft fields as
   local edits. `Draft::commit_desktop` now rebases untouched fields after each
   successful patch and retains unsaved local edits. A deterministic regression
   models a successful font patch, concurrent theme/motion changes, a remaining
   scale edit, and retry. Updated the allowed paragraph of the desktop
   preferences spec to require this per-patch behavior.

## Reviewed contracts

- **Shell:** the footer and Note entity are removed; fallback focus applies
  only when no focus exists. Navigation/menu paths remain independent of a
  deleted input. Search and font filter have distinct focus identities.
- **Fonts/About:** enumeration runs through the background executor, catalogue
  caching coalesces repeated renders, and the complete searchable family set
  has bounded visible controls. Font identity lives in the draft and uses
  explicit typed Save; inherited root typography also reaches input shaping.
  Missing installed names retain their saved identity with system fallback.
  About uses native package metadata and the canonical project repository.
- **Clean:** entering the page, selecting scope, or choosing a folder does not
  dispatch a scan. Project starts explicitly; Global retains confirmation.
  Scope/root replacement clears dependent review state. Existing saved-plan,
  digest, and confirmation checks remain in the execution path. Review rows
  precede the secondary summary/map, with a viewport-relative bounded scroll
  region; no fabricated cleanup data was added to fill space.
- **Software:** first entry starts one read-only Uninstall inventory. Same-tab
  activation is a no-op; the single tab group controls segment transitions.
  Refresh and old-generation rejection remain explicit. Return from Settings
  or other modes retains the active segment/query. Escape applies to the
  active mode, and the worker's final synthetic-selection fix ensures Enter
  cannot reuse a landing-row identity as an installed-app identity. Host-backed
  review still does not gain uninstall/startup execution authority. Rows have
  bounded scrolling and existing detail/preview behavior.
- **Documentation:** code-map routing includes the font catalogue and native
  entry/view files; the preferences spec covers native draft/font/concurrency
  behavior. No new dependencies or public core contracts were introduced by
  the review fixes.

## Verification

- `just gpui-check`: **PASS**, format check and Clippy `--all-targets -- -D warnings`
  (also compiles/types all targets).
- `just gpui-test`: **PASS**, **178 passed, 0 failed**, including the four new
  review regressions and existing safety/state/font tests.
- `git diff --check`: **PASS**.
- Main agent reported canonical root `just ci` passed before these GPUI-only
  fixes. The reviewer did not rerun or claim ownership of that result.

## Remaining evidence boundary

No unresolved actionable source defect was identified in this review. Final
release build and native acceptance belong to the main agent. In particular,
UIA SetValue/paste must be repeated after these fixes; source/unit evidence is
not a substitute for the planned viewport, scale, locale, theme, IME, actual
font-rendering, and focus checks. Main-agent preliminary native results proved
real font enumeration, ordinary key filtering, inherited Cascadia Code text,
explicit Save persistence, About link navigation, and real Software inventory;
they used a build preceding this review and are not relabeled as final-build
evidence here. See the main agent's native QA record for final coverage and
any explicitly unverified matrix cells.
