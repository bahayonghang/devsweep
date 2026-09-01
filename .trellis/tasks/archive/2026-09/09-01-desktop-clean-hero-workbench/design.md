# Design - Clean hero workbench

Follow parent `design.md` section 4.

Owned files under `desktop/src/modes/clean/`, `desktop/src/pages/ScanPage.tsx`,
`ReviewPage.tsx`, `ScanPreviewPage.tsx`, `ExecutePage.tsx`, related CSS,
and Clean tests. Do not change `desktop/src/state/app-state.ts` authority
transitions unless a test proves a presentation-only helper is required;
prefer mapping existing phases to new views.

Grouping is a presentation fold over `target.kind` and `target.scope.type`.
Do not invent a new DTO.
