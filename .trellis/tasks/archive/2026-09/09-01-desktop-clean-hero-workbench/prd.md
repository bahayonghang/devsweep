# Rebuild Clean hero workbench

## Goal

Replace the Clean table-first empty state with immersive first screens:
visible scope + Scan on a sweep-body home, a calm scanning state, grouped
review from existing `kind`/`scope`, and a truthful result hero. Depends
on `09-01-desktop-immersive-spec-shell` tokens and canvas.

## Requirements

- R1: Idle home is not `ReviewPage.tsx:15` light empty heading. Show sweep
  body, ready copy, Projects and Global caches checkboxes, and Scan.
- R2: Scanning uses one working state with backend phase/message and
  indeterminate progress. No percentage.
- R3: Completed Scan Report groups Cleanup Targets by existing `kind` then
  `scope`. Inspect Only stays unselectable. Footer shows Estimated
  Recoverable and Preview. Dry-run, digest invalidation, and second
  confirmation stay in the current reducer.
- R4: Reported outcome uses a large number and existing trash copy. Return
  goes back to review. No "space freed". No 4K-minute metaphor. No
  cumulative freed total.
- R5: Additive `clean.v1.*` keys only when no existing key fits. Existing
  forms stay byte-for-byte.

## Acceptance Criteria

- [x] AC1 (R1, R2): Home and scanning screenshots/tests in EN and zh-CN
      show visible scope and no light empty heading.
- [x] AC2 (R3): Grouped review still blocks Inspect Only, invalidates
      dry-run on selection change, and requires digest + confirm.
- [x] AC3 (R4): Result copy matches existing trash language. Capacity
      confidence remains verified / partial / unknown.
- [x] AC4: Desktop tests for ScanPage/ReviewPage/Clean reducer pass.
      Lint/typecheck/build pass.

## Out of Scope

- New scan providers, Docker cleanup, permanent delete.
- Shell capsule (owned by spec-shell child).

## Dependency

Wait for spec-shell tokens/canvas on the branch before visual
implementation. Reducer work may be prepared against current tests.
