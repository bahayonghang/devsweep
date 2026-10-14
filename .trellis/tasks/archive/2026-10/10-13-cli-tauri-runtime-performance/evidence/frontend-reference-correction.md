# Frontend reference B: search-scope correction

2026-10-13. The coordinator approved this functional correction before freezing
the frontend runtime reference. It is not a runtime optimization or an A-to-C
parity claim.

`CleanWorkbench` previously passed a search-filtered replacement report to
`ReviewPage`. Selection events still reached the full-report reducer, but the
review capacity and select-all indicator used the filtered plan. Hiding a
selected row therefore hid its capacity while it remained selected for cleanup.

The workbench now passes its complete state and a separate display query.
Review filters group rows only. Selected capacity, selected count, select-all
eligibility, coverage and the eventual dry-run request use the full state. No
matches has separate English/Chinese copy and does not claim an empty scan.
Search is display-only and does not modify a current plan or digest.

Verification under Node 22.23.2:

- `CleanWorkbench.test.tsx`, `ReviewPage.test.tsx`, `large-results.test.ts`:
  31 tests pass. The new workbench regression hides an already selected row,
  checks its capacity remains included, selects all from the narrowed display,
  searches to zero matches, and verifies dry-run receives the unchanged full
  plan plus both eligible selected IDs.
- Full `tsc --noEmit`: pass.
- Scoped ESLint for changed component/state tests and components: pass.
- Earlier large-results plus existing app-state tests: 13 pass. The 10k report
  preserves protected/skipped/inspect-only exclusions, last-row selection and
  totals. Twenty fresh 1k previews replace rather than accumulate observations
  and stay read-only after cancellation.

No production rendering, paging, decoder, reducer or transport optimization
has been made at this checkpoint. Native gates remain with integration.
