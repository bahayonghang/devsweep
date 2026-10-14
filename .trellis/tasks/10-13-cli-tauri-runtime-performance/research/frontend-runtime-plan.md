# Clean frontend runtime inspection

2026-10-13. This inspection follows the stable desktop functional checkpoint in
`../../10-13-cleanup-discovery-execution/evidence/desktop-functional.md`.
Production changes wait for the coordinator's immutable corrected frontend B.
The inspected mechanisms are not timing results.

## Observed work

- `TargetTable.tsx` builds every row and every evidence list on each render,
  including children of closed native details. It filters the full review
  target list twice for a header that grouped Review hides.
- `ReviewPage.tsx` filters, groups and sorts the complete report on selection
  changes even though plan, skipped IDs and target sizes stay unchanged. It
  mounts skipped rows while their parent disclosure is closed.
- `ScanPreviewPage.tsx` filters the whole cumulative observation into project
  and global arrays for every progress message, including messages with an
  unchanged preview. It uses the same all-row table.
- The selection reducer builds a complete executable-ID set for a single row
  toggle, then copies the existing selected set. Target eligibility remains a
  reducer check; rendering optimizations must not remove it.
- The reducer already retains a constant number of cumulative snapshots. A
  message without a preview can retain the last preview beside the new progress
  envelope. This is O(1) snapshots, not O(1) bytes. Native transport backlog needs
  separate evidence and is not solved by reducing table DOM.
- `CleanWorkbench.tsx` already owns a full-report path/ID search. Its current
  filtered-state projection also narrows Review's selected-capacity summary,
  while the real reducer's select-all still acts on the full report. Do not add
  a second per-table search. Separate display filtering from retained authority
  in a coordinated correction and record the intentional correctness change.

## Measurement and edit sequence

1. Freeze the production component/decoder/reducer B artifact. Record fixture
   bytes/hash and separate JSON parse, closed decode, reducer, commit and visible
   latency. Preview replay must include fresh decoded target objects as the
   real boundary does; shared references alone can exaggerate memoization gains.
2. Mount evidence only while expanded, retaining native disclosure keyboard
   behavior. Derive group membership/order/totals only when their inputs change.
   Keep selection totals driven by the complete retained report.
3. Repeat the same 1k/10k component workloads. If all-row work still violates
   the 100 ms visible p95, page the displayed rows with native labelled controls,
   an announced range/count, and stable selection across pages. Existing search
   covers every retained row; display scope never limits select-all or cleanup
   scope. Preview may need an equivalent local display-only query for reachability.
4. Only optimize reducer/selector passes if their separate measurements justify
   it. Keep all current stale-preview, inspect-only, protected and skipped
   authority tests. Do not cache trusted eligibility outside its current state.

## Regression matrix

- Closed evidence and skipped groups have no hidden evidence/row descendants;
  keyboard disclosure exposes the complete evidence and warning text.
- Selection, skip, restore, protect, locale changes and report replacement keep
  current behavior. Group order and capacity confidence remain identical.
- If pagination is required: first/middle/last target reachability, full-string
  search, clipboard-accessible long paths, keyboard controls, query/page clamp
  on new reports, and full-plan select-all with inspect-only/protected/skipped
  exclusions. Interim previews stay nonselectable on every page.
- One-row selection invalidates the dry-run digest synchronously and cannot
  select an absent or inspect-only target, including targets outside the shown
  page. A replacement snapshot does not promote preview authority.
- Paired production samples and deterministic retained-target/selected-ID/DOM
  counts demonstrate the effect; Vitest timing is not performance acceptance.

## Ownership

Frontend runtime worker owns TargetTable, ReviewPage, selectors and app-state
after the B gate, plus focused regression tests. ScanPreviewPage and any
catalogue/CSS changes require the coordinator's explicit file handoff. The
coordinator owns the production benchmark harness until its handoff. Native
runtime, IPC backpressure and Rust execution remain with the runtime worker.
