# Current Clean flow and implementation boundaries

Inspected on 2026-11-15 against `dev` / `faf9dc6`. This is step 1 research;
no product changes, host scans, cleanup, or native acceptance were performed.
Line references below describe the baseline, before implementation.

## Decision and review findings

**Core already has sufficient per-target results for the live completion page.**
`ExecutionReport.outcomes` supplies a target ID, action, terminal status with
failure/skip detail, and confidence-preserving capacity. The retained scan plan
can join each result to its ecosystem and target kind. The conditional gate in
`implement.md` about *missing core per-item results* is therefore not triggered.
The current loss is in GPUI: it discards those outcomes when accepting a report.

**AC6 still needs an explicit acceptance interpretation.** The durable audit is
redacted and does not record every unstarted target after cancellation or an
audit halt. Its counts cannot always equal the full live report's failed/skipped
counts. No label can make those different data sets equal. GPUI-only changes can
truthfully present live run counts/reasons and separately qualified durable
history evidence; they cannot promise full durable reconstruction after restart.
If AC6 requires exact equality for every completion number in cancellation/audit
failure cases, that requirement cannot be met within the stated GPUI-only scope.
Main/user review should resolve this before claiming completion-page acceptance;
do not silently restrict acceptance to ordinary successful runs or change core.

Two design details also need care:

- PRD R5/AC5 explicitly authorizes selection edits invalidating the digest.
  Treat a preview as a frozen snapshot; any edit immediately clears it and
  returns to review. The design table's “selection locked” must not leave a stale
  digest displayed as valid. This is routine invariant enforcement.
- Core supports both `MoveToTrash` and command-backed actions. “Succeeded” is
  not synonymous with “moved to Recycle Bin.” Only successful trash outcomes
  contribute to that amount. The PRD's “Recycle Bin only” confirmation and
  “independently managed / review only” rows must not accidentally authorize a
  command under trash wording. The smallest safe GPUI interpretation is to keep
  non-trash intents visible as review-only, unchecked and outside the recycle
  selection; deciding whether command actions remain actionable needs review if
  preserving their current desktop selection is required.

## 1. Existing page, state and retention

| Source | Current behavior |
| --- | --- |
| `desktop-gpui/src/view/page_clean.rs:33` `clean_page` | Uses `page_bar`; landing status already says no scan occurred. Its optional figure is mapped bytes only when the disk map is active. `Found` directly opens the flat review; there is no distinct scanned-summary/review state. `sheet_frame(..., None)` has no fixed bottom action slot. |
| `desktop-gpui/src/view/page_clean.rs:454` `clean_result_card` | Completed title, check-circle, and `Moved {succeeded} of {selected} selected targets`; success color when counts match, warning otherwise. No bytes, failed/skipped reasons, or group details. |
| `desktop-gpui/src/view.rs:1762` `clean_outcome_line` | Maps `Unavailable`, `Empty`, `Partial`, `Failed`, `Canceled`, `StalePreview`, `ConfirmationRequired`, `Mixed`, `TrashReported` to localized generic sentences. The current success sentence is “The isolated trash move was reported.” / “已报告隔离的移出。” (`copy.rs:390`); mixed says some targets moved and some did not (`copy.rs:383`). |
| `desktop-gpui/src/view.rs:1568` `clean_phase_line` | Landing prompt, scanning found-count, found row-count, all-clean, cleaning, completed, failed. No result capacity breakdown. |

`CleanPhase` (`clean_workbench.rs:27`) is `Landing`, `Scanning`, `Found`,
`AllClean`, `Cleaning`, `Completed`, `Failed`. Review and preview are currently
substates of `Found`, distinguished by selection and `Option<CleanPreview>`.
The required six screenshots need distinguishable UI states; they do not by
themselves require replacing this enum.

`CleanWorkbench` (`clean_workbench.rs:334`) retains rows, ID index, sorted IDs,
accepted `UntrustedPlan`, plan epoch, selection, preview, notice, failure detail,
result counts, and `CleanExecutionSummary`. `on_execute_result` (`:884`) keeps
only selected/succeeded/failed/skipped, digest and canceled flag (`:284`), then
drops the detailed report. Rows and accepted plan remain available until reset or
rescan. Thus GPUI can retain/project `TargetOutcome` and join against that plan
without rescanning or changing core.

`CleanPreview` (`:301`) holds the selected saved-plan snapshot, selected IDs,
digest, selected-set snapshot and plan epoch **in memory**. These are not saved
as a desktop plan file. `preview_selection` (`:799`) validates and asks core for
the digest; `begin_execute` (`:844`) requires matching epoch and selection.
`execute_clean_plan` (`:1084`) revalidates the snapshot and supplies the core
executor `execute: true` and the expected digest. No `devsweep.exe` is spawned.
The existing meaning of “saved plan” is this retained snapshot; adding new
persistence is unnecessary for this task.

Scope/reset/new scan clear authority (`:399`, `:545`, `:551`, `:628`, `:652`).
Protection updates and ignoring a row clear preview (`:461`, `:488`). Ordinary
toggle/select-recommended/clear currently change selection without immediately
clearing the stored preview (`:700`, `:719`, `:726`); `preview_matches_current`
(`:527`) and `begin_execute` still refuse stale authority. R5 requires clearing
the stale preview immediately as well.

Session history is also separate from durable history. `history_entry` (`:933`)
formats the retained counts/digest; accepted completion pushes it to the
20-entry in-memory `ShellState.history` (`shell.rs:3480`, `:972`). Opening History
reloads the fixed audit stores and replaces these rows with domain, operation ID
and outcome code (`shell.rs:2691`, `:2701`); it does not preserve full live details
across restart or currently display durable numeric subtotals.

## 2. Exact core result and history contracts

There is no `ExecutionRecord` type in the inspected Clean execution/history
modules. The relevant public types are:

| Type/source in `crates/devsweep-core/src/` | Fields / variants |
| --- | --- |
| `execution/mod.rs:249` `ExecutionReport` | `dry_run`, `selected`, `attempted`, `succeeded`, `failed`, `skipped`, `failures: Vec<ActionFailure>`, `outcomes: Vec<TargetOutcome>`, `notes`, `estimated_recoverable: ScanTotals`, `confirmation_digest`, `audit_log: Option<PathBuf>`. |
| `execution/mod.rs:224` `TargetOutcome` | `target_id`, `action: ActionKind`, `status: OutcomeStatus`, `estimated_recoverable: CapacityEstimate`. |
| `execution/mod.rs:142` `ActionKind` | `Command { irreversible }`, `MoveToTrash`, `InspectOnly`, `PermanentDelete`. |
| `execution/mod.rs:206` `OutcomeStatus` | `Succeeded`, `Failed { message }`, `Skipped { reason }`. There is no separate terminal `Unknown` variant: uncertain action/audit results are failures with truthful diagnostics. |
| `execution/mod.rs:172` `CapacityEstimate` | `Verified { bytes }`, `Partial { lower_bound_bytes }`, `Unknown`. Complete zero is verified; incomplete zero is unknown (`:188`). |
| `execution/mod.rs:351` `ActionFailure` | `target_id`, sanitized `message`; compatibility projection of failed outcomes only. |
| `execution/mod.rs:238` `ExecutionNote` | `DuplicateSelectionRemoved { target_id }`. |
| `execution/mod.rs:469` / `:482` | Progress status includes `Succeeded`, `Failed`, `Skipped`, `Unknown`; progress has completed/total, target ID, status and message. GPUI's execute adapter currently ignores this callback (`clean_workbench.rs:1118`). |

`record_outcome` / `record_remaining_skipped` (`execution/mod.rs:308`, `:329`)
ensure one terminal outcome per deduplicated selected target, in plan order.
`selected == attempted == outcomes.len() == succeeded + failed + skipped`
(`:340`); attempted includes targets never dispatched. `estimated_recoverable`
is the capacity of **all selected targets**, not successful moves (`:299`).
Never use that aggregate as completed/moved bytes.

Join `outcome.target_id` to `UntrustedTarget.id` in the retained plan. Its
`ecosystem`, `kind`, `rule_id`, `scope`, `path`, `estimated_bytes`,
`size_complete`, `sizing_warnings`, risk, reversible flag and intent remain
available (`model/plan.rs:37`). The current `CleanRow` projection drops the
typed ecosystem/kind/intent except `inspect_only`; it retains provider prefix,
rule ID, path/scope, byte confidence and default-selection hint
(`clean_workbench.rs:119`, `:161`). Group from original metadata or extend only
the GPUI projection; do not invent core categories from totals.

For a live completed report every selected target can be joined and has a
terminal fact; failed/skipped outcomes carry the executor's reason. A run that
fails before producing a report has only the adapter failure class
(`clean_workbench.rs:1090`, `:1124`); it cannot supply invented per-item results.
Audit terminal persistence failure after a side effect becomes a failed/unknown
report outcome (`execution/mod.rs:911`); do not count it as a confirmed move just
because its message mentions an action result.

### Durable evidence and exact parity limits

`CleanAuditRecordV1::ExecutionTransition` (`execution/audit.rs:102`) has
`schema_version`, `domain`, `operation_id`, `timestamp_epoch_ms`, `transition`,
`outcome_code`, optional `error_code`, and `ExecutionEvidence`. Outcome codes
are started/succeeded/failed/skipped/unknown/canceled (`:131`). Error codes are
authorization_denied, safety_skip, self_clean_skip, command_failed, trash_failed,
audit_blocked, audit_terminal_failed, duplicate_fingerprint, canceled (`:143`).

`ExecutionEvidence` (`:176`) contains `target_identity_sha256`, `action_kind`,
`irreversible`, `capacity_class`, `duration_ms`, optional `estimated_bytes`.
Identity is SHA-256 of the target ID (`:385`); it is not the raw path, rule or
category. Full failure/skip strings are not persisted. Estimated bytes are written
only for **succeeded MoveToTrash**, from verified or partial capacity; unknown
and older records omit them (`:366`). Full history details wrap these same
redacted records (`history/mod.rs:93`, `:124`); list summaries hold operation ID,
timestamps, outcome/stable codes and partial/unknown/unsupported flags (`:78`).
History cannot recreate categories/reasons from this alone.

`history::clean_moved_totals` (`history/mod.rs:301`, `:305`) sums only succeeded
trash record bytes and returns `known_bytes`, `unknown_records`, `lower_bound`.
Partial sizes, missing sizes, corrupt/unsupported lines qualify the result.
All capacities are observed estimates of data moved, **not physical free-space
measurements or promises**. Use “at least” for partial/unknown contributions;
unknown zero is not a verified zero. Successful commands contribute neither
Recycle Bin bytes nor Recycle Bin counts.

| Case | What can match durable history exactly | What remains live-only / unavailable |
| --- | --- | --- |
| Ordinary run with durable terminal writes | Count corresponding terminal statuses in `history show` records; succeeded-trash known-byte sum and missing/partial qualification match the same report outcomes. | Category/path and full textual reasons need the retained plan/report; History list is only a code summary. |
| Cancellation before a target starts | Already durable succeeded-trash bytes/statuses still match their corresponding live outcomes. | `execution/mod.rs:667` marks the first canceled target skipped in the report without a terminal audit write; `:943` fills remaining skipped likewise. Report selected/skipped totals cannot be reconstructed from durable records. |
| Audit failure before dispatch or after a side effect | Only terminal records actually persisted remain durable evidence; preserve unknown/incomplete qualification. | `:911` reports failed/unknown after a side effect when terminal persistence fails, then `:943` marks unstarted remainder skipped. Durable failed/skipped/selected counts can differ; physical outcome must not be inferred. |
| Restart / only old history available | Succeeded trash totals remain a qualified recorded lower bound, including unknown older records. | Original group/category, full reasons, total selected and unstarted skipped records may be unavailable. |

These are evidence limitations, not a reason to modify the execution model.
Truthful separate labels can describe both sources, but AC6's blanket “numbers
agree with history” needs the review interpretation described above.

## 3. Scope and scan dispatch

`CleanScope` currently has Project/Global only (`clean_workbench.rs:60`).
`Session::clean_start_selected_scope` (`shell.rs:2846`) directly requests project
scan or sets `ConfirmGlobal`. `clean_confirm_global_scan` (`:2860`) consumes that
prompt once and scans Global with no project roots. Rescanning Global asks again
(`:3316`). Folder selection is currently Project-only (`:2871`). Project scan
uses saved explicit roots, prompts when empty, and never silently chooses `.`
(`:3334`). `start_clean_scan` (`:3355`) builds:

- Project: `include_projects=true`, `include_global=false`, explicit roots.
- Global: `include_projects=false`, `include_global=true`, empty roots.

The worker calls `scan_clean_rows` ->
`Sweeper::full_scan_report_run_with_cancel` in process (`clean_workbench.rs:1010`).
Only discovered target count is forwarded for progress; full rows are installed
on the accepted final report (`:1041`). A scanning group/byte display cannot claim
that unretained progress data already exists.

Core `ScanOptions` independently supports both flags (`scan/mod.rs:206`);
Sweeper independently traverses projects (`:456`) and globals (`:504`). CLI
`CleanScope::{Projects,Global,All}` defaults to All (`application/cli.rs:86`),
and `run_scan` enables both for All (`application/commands/clean.rs:51`). No core
or CLI change is needed. GPUI must explicitly carry All through confirmed and
unconfirmed dispatch: unconfirmed All may scan explicit projects only and state
why globals were omitted; explicit confirmation enables one scan with both flags.
Never infer global authorization from a previous project scan or scope tab.

## 4. Existing classification, selection, caching and accessibility

There is no reusable Clean `recycle` grouping model; Analyze's recycle workflow
is separate (`shell.rs:2135`). Clean's useful projection is `clean_map.rs`:

- `category_of` (`:159`) distinguishes build/dependencies, tool caches, agent
  history, AI models, Docker observation, inspect-only tooling, other. This is a
  visual classification, never cleanup authority. `group_identity` (`:471`)
  groups by parent ownership path, falling back to global provider prefix;
  it is not already an ecosystem/kind review grouping.
- `review_band` (`:181`) is Recommended iff selectable and default-selected,
  NeedsReview iff selectable but not default-selected, otherwise NotSelectable.
  `summarize` (`:201`) uses disjoint bands, includes zero-byte rows in counts,
  sums estimated bytes saturatingly, and counts incomplete rows.
- Current `row_selectable` (`clean_workbench.rs:449`) excludes inspect-only,
  protected and ignored; `recommended_ids` (`:589`) additionally requires
  `selected_by_default`. A completed scan initially clears selection (`:652`);
  selecting recommendations is explicit (`:719`). Command intent is currently
  selectable, so enforcing the PRD's recycle-only boundary requires explicit
  GPUI classification rather than equating `!inspect_only` with trash authority.
- `map_inputs` (`view.rs:1602`) uses **filtered** IDs, so its sums change with
  search. Group selection/subtotals must explicitly distinguish full accepted
  scope from filtered display; search must not silently change selected targets.
  Current selected bytes sum selected row estimates (`page_clean.rs:1008`).
- Rows/index/sorted ID list are cached and rebuilt on row-set/sort changes
  (`clean_workbench.rs:738`, `:755`). Search/window IDs have a separate bounded
  model in `shell.rs`. Rendering currently recomputes map inputs/summaries/layout
  (`page_clean.rs:90`, `:493`, `:542`); review grouping should be cached by accepted
  rows, with selection/protection/ignore changes updating eligibility/tri-state
  projections and scope/rescan replacing the grouping. Locale labels should not
  require reconstructing authority or performing I/O.
- The review is flat (`page_clean.rs:817`), with 48-row bounded window/pager and
  stable target IDs. Preserve those IDs, search, offscreen selection and existing
  row/page keyboard operations when adding headers. Use `ui::display_path` /
  `display_path_text`, already used for visible row subtitle (`:922`); raw
  `row_text` currently formats the path directly (`clean_workbench.rs:209`).

Pinned native API is available without dependency changes:

- `desktop-gpui/Cargo.lock` pins GPUI/Zed revision
  `322cd509dcccaee1f6bc44725d270598ae8171ad`, AccessKit **0.24.1**, and Windows
  adapter **0.34.0**.
- In the matching read-only Cargo checkout,
  `crates/gpui/src/elements/div.rs:1406` exposes
  `.aria_toggled(gpui::Toggled::{False, True, Mixed})`; `:3549` writes it to the
  AccessKit node. `crates/gpui/src/gpui.rs:99` re-exports `Role` and `Toggled`.
  AccessKit `src/lib.rs:486` defines all three values; Windows adapter
  `src/node.rs:575` requires toggled state without selection-item semantics and
  `:583` maps Mixed to `ToggleState_Indeterminate`.
- Existing `ui::checkbox` (`ui.rs:873`) is decorative and boolean-only.
  `ui::list_row` (`:892`) exposes ListItem/`aria_selected`, not a checkbox.
  New group controls need a checkbox role, `aria_toggled`, stable accessibility
  ID/label, keyboard focus, and click/accessible action binding. Do not add
  `aria_selected` semantics to the group checkbox. `KeyboardFocus` (`:424`)
  provides Tab + Enter/Space reachability. Do not set an active descendant on
  the directly focused control (desktop-runtime spec). Actual UIA mixed-state
  reporting remains **native-unverified**.

## 5. Safe deterministic verification already available

| Existing hook/test | Safe coverage and limitation |
| --- | --- |
| `clean_workbench.rs:1153` `isolated_multi_cache_project` and `:1185` `project_scan` | Creates a marked temp Python project with pytest/mypy caches and venv; project-only scan, no global scan. Helpers enforce the `devsweep-gpui-c11-` prefix. These are `cfg(test)`, not a native launch fixture. |
| `clean_workbench.rs:958`, `shell.rs:2826` | Test-only accepted-row installation, including large sets; does not provide production/native six-state injection or a saved executable plan. |
| `clean_workbench.rs:1204`, `:1239`, `:1260`, `:1301` | Landing/progress, completed/partial/canceled/failed scan payloads and invalidation. |
| `clean_workbench.rs:1436`, `:1467`, `:1527` | CLI-equivalent digest, stale/missing preview refusal, prefixed digest worker contract. The live execution test uses task-owned temp targets; prefer fake runners for new failure/cancel combinations. |
| `clean_workbench.rs:1709` `test_execution_report` | Synthetic mixed success/failure/skip outcomes with fixture reasons; can extend to partial/unknown estimates and matched row metadata without real cleanup. |
| `shell.rs:5006`, `:5167`, `:5409` | Session keyboard selection/preview/stale refusal, search/sort/bounded windows, full 10,000-ID paging. These are model/text assertions, not native keyboard proof. |
| `execution/mod.rs:1558`, `:1622`, `:1879`, `:2375`, `:2425`, `:2487` | Existing fake-runner/audit tests establish continued target failure, self-clean skip, cancellation, blocked start and terminal audit failure semantics; use as evidence/fixtures, do not alter core behavior for this task. |
| `history/mod.rs:874`, `:936` | Succeeded-trash-only totals, unknown/partial/old sizes, corrupt-line lower bound. `view.rs:1778` test-only panel entries and `shell.rs:4247` History store projection support parity assertions. |

Native launch currently has no Clean-specific deterministic six-state/result
environment hook. `main.rs:53` launches the normal Session. Isolated process
`LOCALAPPDATA`/`APPDATA` can redirect preferences and the fixed audit stores
(`preferences.rs:46`); an explicit task-owned project root can exercise actual
scan/review/preview/recycle safely. This alone does not make failure, cancellation
timing, scanning screenshots or UIA mixed state deterministic. Other-page hooks
(`DEVSWEEP_PREVIEW_STATUS_FILE`, software fixtures, Linux-only window sizing)
must not be presented as Clean fixtures. Main should plan any narrowly scoped
GPUI test/review harness separately; no real host-cache scan or cleanup is needed.

Native acceptance must still observe all six states in English and Chinese,
keyboard-only traversal, disabled confirmation without a valid digest, stale
preview refusal, group mixed UIA state, mixed/canceled reasons and truthful
history qualification. Save screenshots under this task's `evidence/`; ignore
unreferenced images. A screenshot or text-projection test alone does not prove
the authorization chain or durable parity.

## Minimal implementation boundary

Expected product changes are limited to `desktop-gpui/`: `clean_workbench.rs`
for metadata/group/result projections and immediate digest invalidation;
`shell.rs` for All-scope confirmation/dispatch and state/keyboard integration;
`view/page_clean.rs` and its helpers in `view.rs` for state headers, grouped
bounded review, fixed footer and completion details; `copy.rs` for English and
Chinese text. A small checkbox extension in `ui.rs` is justified only if reused
by the new group headers. Keep `clean_map.rs` classification and existing tests
as the starting point; alter it only where consistent shared projection requires
it. A deterministic native hook, if needed, must remain explicit and task-scoped.
No new core/CLI model, cleanup rule, scanner authority, persistent format, Tauri
code, or child CLI process is required.

## Research validation

Sources and pinned APIs were inspected read-only. No heavy gates were run for
this document-only step. Product implementation remains at step 2, pending main's
decision on the AC6/history and command-versus-recycle interpretation above.
