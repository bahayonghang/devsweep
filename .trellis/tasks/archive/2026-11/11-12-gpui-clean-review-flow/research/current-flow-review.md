# Step 1 review: current Clean flow

Reviewed on 2026-11-15 against `dev` / `faf9dc6`. Documentation checkpoint only.
The load-bearing findings in `current-flow.md` are supported by the owning code
and existing test assertions. No product changes or task-status edits were made.

## Verdict and remaining decision

The conditional **missing core per-item results** gate is not triggered. Core
already supplies the data needed for live grouped completion details.

**AC6 needs a main/user decision before completion acceptance can be claimed.**
Literal equality between every live completion counter and durable History is
not achievable within the current scope for cancellation and audit failures.
The design's instruction to show existing data and report gaps authorizes
truthful fallbacks; it does not silently amend AC6's equality requirement.

Recommended explicit resolution: keep live run counters/reasons and the session
history entry derived from the same retained report; identify durable history
as recorded evidence, with unavailable/pending evidence stated separately.
Compare like-for-like succeeded Recycle Bin estimates where terminal persistence
completed normally. Do not promise reconstruction of unrecorded selected,
failed, or skipped targets after restart. This is a proposed acceptance
clarification, **not an approved change to AC6**. Unknown/at-least size wording
alone is routine and already authorized; it cannot resolve missing counts.

## Verified result and history contracts

- `crates/devsweep-core/src/execution/mod.rs:224` defines `TargetOutcome` with
  target ID, action, terminal status and `CapacityEstimate`. `OutcomeStatus`
  (`:206`) carries failed messages and skipped reasons; uncertain results are
  represented as failed outcomes, not a separate terminal Unknown variant.
  `record_outcome`/`record_remaining_skipped` (`:308`, `:329`) cover every
  deduplicated selected target. The aggregate estimate (`:299`) covers **all
  selected targets**, so it is not moved capacity.
- The retained `UntrustedPlan` supplies ecosystem/kind by target ID
  (`crates/devsweep-core/src/model/plan.rs:37`). GPUI retains that plan but
  currently drops detailed report outcomes in `on_execute_result`
  (`desktop-gpui/src/clean_workbench.rs:884`), keeping counts/digest/canceled
  only (`:920`). Retaining a GPUI result projection is sufficient; no core
  execution-model change is needed. Only succeeded `ActionKind::MoveToTrash`
  outcomes contribute to moved counts/estimated bytes. Command success does not.
- Current `clean_result_card` uses succeeded/selected counts with moved wording
  (`desktop-gpui/src/view/page_clean.rs:454`), without group reasons or capacity.
  `history_entry` uses the same aggregate counts (`clean_workbench.rs:933`), and
  Session pushes it on accepted completion (`shell.rs:3480`). Opening History
  reloads the fixed stores (`shell.rs:2685`, `:2691`) and replaces those entries
  with domain/operation/outcome lines (`:2726`). Matching the transient entry
  does not prove matching durable History.

### Why start records and helpers cannot close AC6

1. **Concrete cancellation case:** select two targets and cancel before the
   first action. `execution/mod.rs:667` records one skipped live outcome without
   an audit write; `:943` fills the remaining skip. The existing
   `cancel_flag_stops_later_targets_before_dispatch` test (`:2375`) asserts
   selected=2, attempted=2, skipped=2 and no runner call. `AuditJournal::open`
   merely creates in-memory state (`execution/audit.rs:666`); it emits no run
   header. There can therefore be no operation row at all, while the live page
   truthfully reports two skips. Cancellation after an earlier success similarly
   leaves only the started/completed targets in the journal.
2. **Audit terminal failure:** dispatch occurs at `execution/mod.rs:859`.
   Failure writing its terminal records a live failed outcome and Unknown
   progress, then halts (`:911`); later selected targets become live skips
   (`:943`). The existing test at `:2487` asserts one trash-runner call but zero
   successes and one failure (`:2539`). A missing terminal cannot reconstruct
   that live classification. Also, append writes before flush/sync
   (`execution/audit.rs:644`): a failure at the latter stages may leave a
   parseable succeeded line even though the live result is failed/uncertain.
   Treating every readable line as proof of acknowledged durable success would
   hide that distinction.
3. The complete Clean V1 envelope has only per-target execution transitions and
   protection mutations (`execution/audit.rs:102`), with hashed identity,
   action/confidence, duration and optional succeeded-trash estimated bytes
   (`:176`, `:366`). There is **no selected-set manifest, operation summary,
   full reason, path or category**. Started records describe only reached
   targets. History summaries only group existing records and collect codes
   (`history/mod.rs:655`); they do not synthesize omitted targets.
4. `history::clean_moved_totals` is cumulative across the fixed Clean store,
   not a per-run report (`history/mod.rs:301`). It sums succeeded-trash bytes
   only (`:312`). Its `lower_bound` tracks store read reasons and missing/partial
   sizes; it does **not** diagnose dangling Started records or unrecorded
   targets (`:305`). The test at `:874` explicitly includes Started/Failed
   records while expecting `lower_bound=false` (`:886`). A Started-only operation
   also is not automatically Unknown in `summaries_from` (`:684`). GPUI must not
   infer complete run evidence from these flags. `ExecutionReport` exposes no
   journal operation ID (`execution/mod.rs:249`), so a per-run history comparison
   also needs an explicit correlation strategy; the cumulative helper alone
   cannot provide it.

These facts support the research conclusion. Keep the above helper/durability
qualifications with it; no factual correction to `current-flow.md` was needed.
Changing core or adding a new persistence format would exceed this task, and
still cannot guarantee a durable record when storage itself fails.

## Command-backed targets and the recycle boundary

Current `CleanRow::selectable` checks only `!inspect_only`
(`clean_workbench.rs:144`); `row_selectable` adds protection/ignore checks
(`:449`), and recommendations add the scanner hint (`:589`). Thus command-backed
targets are selectable today. The registry reconstructs `cargo clean` for
`rust.target` (`crates/devsweep-core/src/rules/registry.rs:226`), and provider
actions such as npm/pip/pnpm/yarn/go also resolve to commands (`:273`).
`command_spec` marks them irreversible (`:537`). A path or ecosystem is not proof
of trash authority.

The minimum safe interpretation of the PRD's recycle-only goal and R4/R5
(`prd.md:7`, `:16`, `:17`) is to retain Command and Inspect Only findings visibly
in review-only groups, outside recommended recycle bytes and **all** recycle
selection paths (row, group, recommended, preview). Do not relabel an actual
Command target's core intent as Inspect Only or convert it to a trash action.
Use its typed classification for the GPUI eligibility projection.

This is supported by the existing task contract; keeping commands actionable
under a “move to Recycle Bin” confirmation would contradict it. However, R4's
“unchecked by default” alone does not mean nonselectable. Applying the combined
contract removes existing desktop command execution from this flow, including
ordinary Rust target cleaning. Main should explicitly state that compatibility
effect in the review decision. Preserving command execution would require a
separate, accurately worded command confirmation decision; it is not a local
checkbox fix. Core/CLI command support remains outside this UI change.

## Accessibility, scope and state

- The lockfile pins Zed/GPUI revision
  `322cd509dcccaee1f6bc44725d270598ae8171ad`, AccessKit 0.24.1 and Windows adapter
  0.34.0. Inspected read-only under the matching Cargo checkout: GPUI
  `crates/gpui/src/elements/div.rs:1406` accepts `aria_toggled`; `:3549` forwards
  it to the AccessKit node; `gpui.rs:99` re-exports `Toggled`. Installed
  `accesskit_windows-0.34.0/src/node.rs:575` enables Toggle only without
  selection-item semantics, and `:583` maps Mixed to UIA Indeterminate. No new
  dependency is required. Current `ui::checkbox` is a boolean decorative glyph
  (`desktop-gpui/src/ui.rs:873`); `list_row` exposes ListItem/selected (`:892`).
  New group checkboxes need a real checkbox role/toggled state, label, focus,
  click and accessible activation, without selected-item semantics. Tab focus
  comes from `KeyboardFocus` (`:424`); GPUI's Enter/Space click synthesis is in
  the same pinned `div.rs:3017`. Preserve the desktop-runtime rule against an
  active descendant pointing to the directly focused node. Native mixed-state
  reporting is still unverified.
- `CleanScope` is currently Project/Global (`clean_workbench.rs:60`). Session
  asks for Global confirmation and consumes it once (`shell.rs:2846`, `:2860`);
  folder selection is Project-only (`:2871`); explicit roots are required
  (`:3334`). The existing test at `:4489` proves scope changes do not themselves
  arm confirmation. Core independently supports both scan flags
  (`scan/mod.rs:206`, `:456`, `:504`), and CLI All enables both
  (`application/commands/clean.rs:51`). All must preserve explicit project roots,
  allow project-only dispatch while global is unconfirmed with an honest
  omission label, and enable both flags for a confirmed single scan. Carry the
  requested scope and effective scan separately so rescan cannot reuse global
  consent or silently present partial scope as complete All.
- Found currently covers scanned/review/preview states; preview validity is the
  selection/epoch snapshot (`clean_workbench.rs:527`, `:799`, `:844`). Separate
  visible states can be added within GPUI without changing core. Immediate
  preview invalidation on selection edits follows R5; the existing rejection
  test (`:1467`) currently checks invalidation only when execution is attempted.
  Scan progress retains only discovered count (`:1010`); showing discovered
  groups requires the core-owned read-only `ScanPreviewSnapshot` projection
  (quality-guidelines.md:1310), never partial cleanup authority or invented rows.

## Validation and fixture limits

Source review covered the actual owners, callers and existing tests above; tests
were **read, not executed**. The canonical `desktop-gpui/DESIGN.md`, task
artifacts, desktop-runtime, clean-advisories, scan-coverage and the relevant
execution/plan/ranking/scan sections of the real quality guidelines were read.
The saved hook output was read; its quality-guideline truncation was not treated
as the complete specification.

The temp project helpers (`clean_workbench.rs:1151`, `:1185`) support isolated
project-only scans and test-owned targets. They and accepted-row injections
(`:958`) are `cfg(test)`, not native launch fixtures. Normal startup launches
the regular Session (`desktop-gpui/src/main.rs:54`). Process-local APPDATA and
LOCALAPPDATA redirect preferences/protection/audit stores
(`preferences.rs:46`, `execution/audit.rs:303`); this alone does **not** isolate
every global provider or make cancellation/failure timing deterministic. Confirmed
All must be verified through isolated/fake dispatch evidence unless a narrowly
scoped fixture harness exists. Do not scan real host globals to fill the gap.

- Lint / format: not run; documentation-only checkpoint.
- Type-check / build: not run; documentation-only checkpoint.
- Tests: not run; existing assertions inspected only.
- Native screenshots, keyboard-only traversal, UIA and real fixture Recycle Bin
  acceptance: not run and not passed. These remain implementation acceptance.

No host scan, cleanup, uninstall, update, startup mutation, termination, native
automation, commit or push was performed. Stop here for the main/user review
checkpoint; product implementation and acceptance criteria remain unchanged.
