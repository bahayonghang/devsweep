# Implementation: Custom Main Window Controls

## Entry Gate

- [x] The user approved the parent summary and implementation of both children.
- [x] Load this task's PRD, design, implement/check manifests, and parent research.
- [x] Start this child only after the review gate. Keep the parent as integration owner.

## Ordered Work

1. [x] Update only the affected native-chrome and adapter-scope spec clauses. Preserve the capsule, accessibility, domain, and close contracts.
2. [x] Extend lifecycle.ts with the typed window-control adapter and fake bridge. Add action/state/listener tests at the adapter seam.
3. [x] Add the titlebar and original SVG controls. Connect maximize state, pending/error states, drag exclusion, keyboard labels, and close request behavior.
4. [x] Set the main window decoration flag and exact capabilities in the same working change. Update the existing capability assertion.
5. [x] Extend existing App/lifecycle tests for repeated close, active drain, wait-only completion, minimize preservation, and effect replay. Do not refactor the coordinator.
6. [ ] Verify the focused tests, then run the required desktop gate and Rust gate. Record actual outcomes; do not claim native behavior from mocks.
7. [ ] Record the native window scenarios from W-AC2 and the changed titlebar's scale/accessibility evidence. Link any unavailable row and its reason.
8. [ ] Hand off semantic titlebar tokens and evidence to the settings child and parent. Review the combined titlebar under the selected theme and font values during parent integration.

## Commands

From desktop, with the repository Node toolchain:

- mise exec node@22 -- npm run lint
- mise exec node@22 -- npm run typecheck
- mise exec node@22 -- npm run test
- mise exec node@22 -- npm run build

From the repository root after the Tauri capability/config changes:

- just ci

Run focused Vitest cases while editing. Run each final gate once for the final change set; repeat only after relevant fixes. No product build or test is required for creating these planning documents.

## Review And Rollback

The implementation/check sub-agents own their assigned product scope. The main session coordinates spec changes and review. Parent and child work must not overwrite concurrent changes in the other desktop tasks.

The rollback unit is titlebar + native adapter + decoration setting + permissions. Restore native decorations with that unit if native acceptance fails. Do not change the old performance thresholds or operator evidence.

## Context Loading Note

The full backend quality guide exceeds the configured 32 KiB per-file injection limit. The manifests load the backend index and scoped research instead. Before affected Rust edits, read the relevant quality-guide sections directly in bounded chunks. Do not treat a truncated injected document as the full contract.
