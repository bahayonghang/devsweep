# Implementation Plan: Desktop Window Controls And Settings

## Current Phase

Both approved children have product implementations. The 2026-09-26 continuation fixed a stale-preference reload defect and completed independent code review and text browser checks. The core unit-test executable failure and native acceptance rows remain open. See evidence/integration-review-20260926.md.

## Entry Gate

- [x] User approved the latest parent summary and implementation of both children.
- [x] Parent/child planning artifacts match the approved scope.
- [x] Both child context manifests passed planning validation.
- [x] Started the window-control child; the parent remains the integration owner.

## Ordered Execution

1. [x] Implement 09-25-desktop-custom-window-controls. Product integration and focused tests are recorded in the child's research/implementation-evidence.md; native acceptance remains open below.
2. [x] Check the window child. Independent code review and focused tests passed; research/check-evidence.md records the remaining native acceptance rows.
3. [x] Implement 09-25-desktop-settings-preferences. Store, IPC, Settings, shared appearance, and runtime consumers are complete; the child's backend/frontend reports record the implementation.
4. [ ] Check the settings child. Independent product review fixed the reload sequence defect and found no other product issue. Frontend/Tauri/Clippy checks passed; the full Rust gate and native rows remain incomplete.
5. [ ] Run the combined scenarios below on the final build. Reuse valid child evidence; rerun only checks affected by subsequent integration edits.
6. [x] Map W-AC1-W-AC6 and S-AC1-S-AC11 to parent AC1-AC8. The evidence/integration-review-20260926.md matrix identifies implementation/test coverage and native rows still open.
7. [ ] Hand any relevant new measurements to existing performance/native owners without changing their scope, thresholds, or prior evidence. Finish the task only when the approved acceptance is met.

## Combined Scenarios

| Scenario | Required evidence | Parent acceptance |
| --- | --- | --- |
| Theme/font change with mode results retained | All affected surfaces including titlebar/HUD update; Cleanup Plan/digest/result identities remain intact | AC3, AC6 |
| System theme changes and HUD creation/reopen | Main and HUD agree with the committed choice; late snapshots cannot revert appearance | AC3, AC5 |
| Active work then Settings or custom close | Existing route drain or close drain reaches join/completion; no duplicate controller or orphan sampler | AC1, AC2, AC6 |
| Status default, active interval change, HUD next-show interval | Actual command/sampler inputs and ordering match the matrix; hidden HUD remains stopped | AC4, AC6 |
| Failed write, group reset, restart, old locale reader | Correct persisted values and errors; protected invalid bytes; unchanged language/TUI contract | AC5, AC8 |
| Both locales, text-scale boundary, forced colors, native window controls | Readable actions, focus, non-overlap, native drag/resize/restore/close | AC1, AC7 |

## Required Implementation Gates

The 2026-09-26 continuation uses no image features or screenshot verification,
as requested by the user. The text-browser report records completed DOM,
computed-style, accessibility, and keyboard checks. Native acceptance remains
open where the available tools cannot perform the required actions.

Use the child implementation plans for focused commands. Final desktop gates are Node 22 lint, typecheck, tests, and build; regenerate changed wire contracts through types:generate. Run just ci for the affected Rust/Tauri contracts. Do not claim those gates passed during planning.

No need to rerun pre-existing failures merely to confirm their recorded state. The older default-workload performance findings remain open. Any future performance comparison records the actual preference values and preserves original thresholds.

## Planning Validation

Validate only artifacts created by this request: parent-child links, planning status, required documents, real JSONL context entries, referenced paths, and requirement/acceptance coverage. No product test, installer, native action, or unrelated cleanup is part of this planning pass.

## Context Loading Note

The full backend quality guide exceeds the configured 32 KiB per-file injection limit. The manifests load the backend index and scoped research instead. Before affected Rust edits, read the relevant quality-guide sections directly in bounded chunks. Do not treat a truncated injected document as the full contract.
