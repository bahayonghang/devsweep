# GPUI Clean review and completion

## 1. Scope / Trigger

Apply to `desktop-gpui` Clean scope dispatch, grouped review, preview authority,
and completion/history presentation. The desktop calls core in process. It does
not launch the CLI, add cleanup rules, change persisted plans/audit formats, or
turn scan observations into execution authority.

The recycle-only UI boundary and source-specific completion semantics were
explicitly approved on 2026-11-15. Core/CLI command execution remains available
under its own contracts; this desktop flow does not confirm commands.

## 2. Signatures

`clean_workbench.rs` owns these desktop projections:

```rust
pub enum CleanScope { Project, Global, All }
pub fn clean_scan_options(
    scope: CleanScope,
    confirmed_global: bool,
    roots: Vec<PathBuf>,
) -> Option<ScanOptions>;
```

`CleanRow` retains typed ecosystem/kind and recycle eligibility from the accepted
target intent. A display group, rule name, path or scanner recommendation must
never grant eligibility. `CleanWorkbench` owns the accepted plan, selection and
preview snapshot; view code only projects that state.

## 3. Contracts

- Keep requested scope distinct from effective coverage. Project and All need
  explicit roots; never supply an implicit current directory. Global requires
  fresh confirmation. Unconfirmed All scans projects only and explains the
  omitted global scope; confirmed All enables both flags for one scan. A rescan
  cannot silently reuse global consent.
- Command and InspectOnly targets remain visible as review-only. Exclude them
  from recommended recycle capacity and every selection path, including row,
  group, recommended selection, saved preview and execution. Preserve their
  actual intent; do not rewrite a command as a trash action.
- Cache group membership from accepted rows. Keep group subtotals based on the
  accepted set rather than the search window. Search, bounded paging and stable
  target IDs must preserve offscreen selection. Tri-state counts use currently
  eligible rows; protection and ignore rules still apply.
- Every selection edit invalidates the old preview immediately. Execution also
  checks the exact current selection, accepted plan epoch and live `sha256:`
  preview. Neither a group subtotal nor a scan progress snapshot is authority.
- Retain per-target `ExecutionReport` outcomes and join metadata from the same
  accepted plan by target ID. Only succeeded `ActionKind::MoveToTrash` contributes
  to moved count/capacity. The report's aggregate recoverable estimate includes
  all selected targets and is not moved capacity. Observed bytes never promise
  physical disk-space recovery; incomplete/unknown values remain qualified.
- Completion and the session history entry derive from the same report.
  Persisted History contains recorded evidence, not a complete manifest of all
  selected targets. Cancellation can produce skipped outcomes without any audit
  record; a terminal write/sync failure can disagree with readable journal bytes.
  Explain incomplete evidence rather than reconstructing missing rows or claiming
  full equality after restart. Compare normal persisted results only using the
  same run and evidence scope, never the cumulative store total as one run.
- Group checkboxes need a real Checkbox role, focus, stable accessible label/ID
  and `aria_toggled(Toggled::{False, True, Mixed})`. Do not attach selected-item
  semantics to these Toggle controls or make the focused node its own active
  descendant. Verify Mixed as native UIA Indeterminate, not only a painted dash.
- `DEVSWEEP_CLEAN_REVIEW_FIXTURE` is an optional explicit native review hook.
  Its closed value is `<en|zh>:<landing|scanning|scanned|review|previewed|complete|mixed|canceled|audit-failed>`.
  An absent variable launches normally; invalid/non-Unicode values fail closed
  rather than falling back to host startup. A fixture has no accepted cleanup
  plan, never creates a usable digest, and disables scans, execution and navigation
  into host-reading modes, History or AI surfaces. Synthetic results are labeled
  and never persisted as audit. The previewed fixture is not a valid live preview.

## 4. Validation & Error Matrix

| Input/state | Required result |
| --- | --- |
| Project/All without explicit roots | Refuse dispatch; request roots |
| Global without current confirmation | Refuse dispatch |
| All without global confirmation | Project-only options and visible omission |
| Command or InspectOnly selected by any route | Not admitted to recycle preview/execution |
| Selection changes after preview | Drop digest; disable confirmation; re-preview required |
| Old plan epoch or mismatched selected set | Refuse execution even if a button is invoked |
| Partial positive size / unknown size | Lower-bound / unknown wording; never exact zero |
| Cancellation or audit failure | Full live outcomes plus incomplete-history qualification |
| Started-only or parseable terminal journal line | Not proof of complete acknowledged run results |
| Invalid/non-Unicode fixture value | No normal host session fallback |
| Fixture mode/menu/AI navigation | Stay inside read-only Clean scene; no host job or History load |

## 5. Good / Base / Bad Cases

- Good: a two-row recycle group is partially selected; the header exposes Mixed,
  retains its full accepted subtotal, and toggling it invalidates the digest.
- Base: unscanned Clean shows scope and roots without a numeric zero. Scanned
  results separate found capacity from recommended recycle capacity.
- Bad: a successful `cargo clean` is labeled as a Recycle Bin move, a filtered
  map total is presented as the whole accepted plan, or two pre-dispatch skips
  are said to exist in durable History when no operation record was written.

## 6. Tests Required

Assert disjoint group totals, no empty groups, zero/incomplete estimates,
eligible-only tri-state and recommendation, and all review-only refusal routes.
Test All with/without confirmation and roots, fresh rescan consent, immediate
preview invalidation, stale/missing digest refusal, stable row/paging behavior,
and success/failure/skip projections. Use an isolated audit store to compare
normal persisted evidence with the corresponding live result; separately cover
cancellation and terminal audit failure without pretending durable parity.
Assert that map summaries retain incomplete/unknown confidence and label their
filtered scope separately from full review-group totals. Cover fixture navigation
and invalid environment input in addition to scan/preview/execution refusal.

Native acceptance covers six states in English and Chinese, fixed actions,
keyboard traversal, native Mixed and authorization controls. Use task-owned
fixtures/test runners. Redirecting APPDATA/LOCALAPPDATA alone does not isolate
every global provider. Read-only injected scenes must be identified separately
from real scans and real Recycle Bin execution, and must not create executable
authority. Run `just gpui` and `just ci`; preserve existing safety contracts.

## 7. Wrong vs Correct

```text
Wrong: moved_bytes = report.estimated_recoverable; success means recycled.
Correct: sum succeeded MoveToTrash outcomes, retaining capacity confidence.

Wrong: readable audit lines or cumulative clean_moved_totals prove this run.
Correct: show the complete live report and identify durable recorded evidence;
         qualify cancellation/audit gaps and compare only matching scopes.

Wrong: group header is a decorative square with aria_selected(partial).
Correct: focusable Checkbox + aria_toggled(Mixed), verified in native UIA.
```
