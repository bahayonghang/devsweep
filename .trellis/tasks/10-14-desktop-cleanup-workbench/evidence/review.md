# Independent D cross-layer review

Date: 2026-10-14. Reviewer: `backend_contract_check` (dispatched `trellis-check`).

Status: **source and focused regression checks PASS; D visual/native/performance acceptance remains separate**. This is not parent/F completion.

## Findings (fixed)

1. `desktop/src/modes/clean/CleanWorkbench.tsx`: a cancellation request suppressed an authoritative `completed` scan result through `!owned.canceled`, so a backend-completed scan could appear canceled. Removed that condition from the completed-terminal branch. The production bridge still checks returned scan identity, and the session/reducer still check scope, revision, scan identity and progress sequence. A cancellation request alone cannot promote partial observations. Updated the scope-session integration case to use the Rust-generated global report and expect completion. It failed before the production change and passed afterward.
2. `desktop/src/modes/clean/scope-session.test.tsx`: the touched cancellation test's typed mock parameters were unused. Replaced them with a typed `vi.fn<DesktopBridge["scanStart"]>` callback; this preserves the recorded request signature without unused variables. Final scoped ESLint passes.
3. `desktop/src/modes/clean/session-state.test.ts`: added a regression for a stale dry-run response arriving while a newer request has the same selection. It verifies the revision fence, acceptance of the current response, exit from confirmation on route hide while retaining the preview, and preview/selection invalidation on root or protection changes. This covers a case where matching selection alone is insufficient.
4. Final-gate follow-up, `desktop/src/stage/Stage.test.tsx`: the full desktop gate exposed two legacy EN/zh-CN Clean first-screen assertions for the old Scan/Earth Stage. D had already replaced these with named scope regions when the reviewer reopened the file. Preserved that concurrent correction and strengthened it with an explicitly empty roots store, waiting for preferences to finish loading before asserting global scanning enabled and project scanning disabled, plus no mounted planet. All other mode Stage tests remain intact. No product source changed in this follow-up.

## Reviewed boundaries

- The stable `CleanSessionProvider` lives above routed content. It retains independent global/project reports and domain state; the shell does not own plans or digests.
- Global scanning submits exactly `include_projects:false`, `include_global:true`, `roots:[]`, regardless of root-store failure. Project scanning requires validated explicit roots and submits the opposite flags. One scope's state update cannot overwrite the other slot.
- The immediate request ref prevents same-render duplicate heavy requests. Scans, previews and execution all use the shared operation coordinator. Normal shell navigation awaits cancel-and-join before route replacement; component cleanup and revision advancement fence late callbacks. A completed execute result can settle before normal route unmount.
- Root edits clear only project authority. Protection changes invalidate both plans while retaining explicitly stale observations; they do not mark all targets protected. A fresh scan is required to regain actionable authority.
- Preview and execute capture the selected scope's existing plan and exact selected IDs. Execute forwards the returned preview digest after second confirmation. No merged plan or frontend-derived cleanup action is constructed. Existing reducer tests retain selection/digest/stale-confirmation checks.
- Warning projections use only a completed project report and the core advisory bridge. Report/threshold responses are fenced, warning settings do not change selection or trigger scans, and failed persistence retains a visibly session-only threshold across routes.
- Project occupancy renders the core's deduplicated artifact/dependency confidence. Command completion is not represented as bytes moved to trash. Shared/unknown ownership remains an observation, not executable or selectable authority.
- Completed review slices 100 targets before grouping, bounding both rows and groups even with 10,000 distinct owners. Search and selection use the full report. Owner advisories are paged by 10; coverage and diagnostics are lazy and paged by 100. These are functional DOM-bound checks, not production timing evidence.
- New palette values are scoped to `.clean-workbench`; other saved themes inherit their appearance roles. `appearance.css` already scales the root font through `--text-scale`, so rem-based Clean typography must not multiply the scale again. The source retains reduced-motion/forced-colors handling. Unchanged-mode geometry and actual rendered theme/font/scale acceptance require the visual evidence below.
- The shared `formatTotals`/`formatCapacity` additions preserve English defaults for unchanged callers. Analyze benchmark changes only supply the three required unused bridge methods. No new runtime UI/icon dependency was added by this review.
- D synchronized the Stage/Earth/heading specifications, including the reviewer-noted generic hidden-h1 and Clean-below-Stage examples. The final component/index/state-management text now describes the visible Clean heading, separate detail/back flow, retained slots, authority fences and bounded groups; other modes retain Stage.

## Findings (not fixed) and evidence boundaries

- Native Windows/WebView2 visual acceptance, native icon/taskbar/tray/installer checks, and F integration/performance gates are **NOT_RUN by this reviewer**. Browser fixtures and jsdom viewport tests cannot substitute for them.
- The final screenshot/geometry matrix, actual Settings maximum text-scale/font measurement, keyboard/forced-colors/reduced-motion captures and unchanged-mode comparisons remain owned by D/parent. This report does not mark them passed from source inspection.
- Upstream B remains limited: mbx public location discovery does not establish project/private/shared occupancy or qualified expected-state execution. The backend review preserves those outstanding criteria. Installed uv 0.13 is outside the currently qualified 0.4–0.11 command range; this review did not widen qualification.

## Verification

All commands used exact Node `C:/Users/lyh/AppData/Local/mise/installs/node/22.23.2/node.exe`, with `desktop` as the working directory.

- Lint: **PASS**, `node_modules/eslint/bin/eslint.js src/modes/clean src/pages/ReviewPage.tsx src/pages/ReviewPage.test.tsx`. Reran the three reviewer-touched files after the final test addition; PASS. An initial invocation named the wrong ScanCoverage path and did not run; the corrected scope includes `src/modes/clean/ScanCoverage.tsx`.
- TypeCheck: **PASS**, `node_modules/typescript/bin/tsc --noEmit -p tsconfig.json`; rerun after final reviewer edits.
- Tests: **PASS**, `node_modules/vitest/vitest.mjs run src/modes/clean src/pages/ReviewPage.test.tsx src/state/large-results.test.ts src/state/app-state.test.ts src/app-shell/operation-coordinator.test.ts` — 15 files, 81 tests before the extra revision-fence case. The final `scope-session` 5 and `session-state` 5 cases passed after their respective edits (82 unique cases covered across this focused set; reruns are not counted as additional cases).
- Final-gate follow-up: Stage tests **15/15 PASS** with `node_modules/vitest/vitest.mjs run src/stage/Stage.test.tsx`; scoped Stage ESLint **PASS** and full `tsc --noEmit -p tsconfig.json` **PASS**. Parent's earlier full-gate result was 468 passed / 2 failed; the parent must rerun the full gate after this test-only repair.
- `git diff --check`: **PASS**, including the reviewer evidence document; final parent gate remains authoritative for the shared working tree.
- The parent reports final normal-host `just ci` PASS, 788 passed / 5 ignored, after resolving the external mbx target environment issue. This reviewer made no Rust changes in the D pass and did not rerun or replace that canonical gate.
- Preserved pre-existing `desktop/src-tauri/Cargo.toml`: SHA-256 `b7d6f6a30067850f5c4b1f3afde6450667262eb57309d230ca05fe7a0dd84915` unchanged.

No product user-cache scan or cleanup, actual cleanup execution, commit, journal, task archive, install or publication was performed by this review.
