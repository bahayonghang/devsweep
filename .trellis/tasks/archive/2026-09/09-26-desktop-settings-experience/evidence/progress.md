# Sequential Implementation Progress

Updated: 2026-09-28 UTC (2026-09-27 America/Chicago). User approval dated 2026-09-26 covers the reviewed four-child plan and the two named production dependencies.

| Stage | Implementation | Independent review | Evidence |
| --- | --- | --- | --- |
| Navigation | Complete | PASS | ../09-26-desktop-settings-navigation/evidence/implementation-report.md and check-report.md |
| Controls | Complete, including native composition repair | PASS | Controls implementation/check reports and ime-repair-report.md / ime-check-report.md |
| Host fonts | Complete | PASS | 09-26-desktop-settings-host-fonts/evidence/implementation-report.md and check-report.md |
| Palettes | Complete | Code review PASS; full CI blocked | Active task remains 09-26-desktop-settings-palettes; check-report.md and dialog-repair-report.md |
| Parent integration | Targeted checks complete; canonical CI blocked | Independent integration report | integration-report.md, integration-check-report.md, native-final/, native/, integrated-source-final.json |

## Current integration result

Final frontend gate passes 51 files / 387 Vitest tests and 5 Node tests, generated types, ESLint, TypeScript, and build. Desktop Rust tests pass 83 with 2 intentional fixture ignores. Corrected Clippy passes. The final release executable and NSIS installer build passes; no installer was run. The production source changed only in the Combobox keyboard handler after the pre-IME integration snapshot.

The final native binary passes 248-family catalogue discovery, first/middle/last font selection, V1/V2 byte-preservation checks, failed-save recovery, and restart. Navigation passes in both locales. Matrix results: 44 page cases, 156 popup cases, and 28 media cases. Four final-binary Settings screenshots were captured and visually checked. The prior binary has actual HUD/dialog evidence with unchanged palette, CSS, HUD, and Rust source hashes.

Native composition testing reproduced a saved-font change during composing Enter. The reviewed repair preserves the saved font and browser IME action. Final renderer-composition evidence shows true composing key events, identical before/during/after file hashes, and Chinese alias filtering from 249 options to 12 after explicit CDP text commit. Real Windows candidate-window interaction remains unverified; the user requested archival without supplying a manual result.

Canonical just ci remains BLOCKED. Two runs could not execute a missing core test binary. An earlier direct core run ended with STATUS_ACCESS_VIOLATION after 203 successful tests, including all 14 preference tests. A real-toolchain no-run build in a new ordinary directory exited 0 but did not retain its reported executable. Read-only diagnostic logs have not established the cause. No security setting was changed. See core-gate-diagnostic.md.

The user subsequently requested commits and archival after receiving the validation limits. The feature commit is 5e73a4fb2e329c305d400efbb7670684f998d709. See closure-decision.md. The archive metadata retains the blocked core gate and unverified manual checks. Unrelated tasks remain unchanged.

## Earlier stage evidence

The following paragraphs record intermediate results. Current integration outcomes are listed above and in integration-report.md.

Navigation evidence: 70 focused tests, lint, typecheck, production build, and 8 bilingual viewport cases pass. The implementation preserves five operational ModeIds and adds Settings as the sixth primary navigation destination. No production dependency was added in navigation. Its report records the pre-Base-UI production asset baseline.

The navigation browser screenshot request timed out. Final native control/theme screenshots and main/HUD rendering remain parent integration work. No pixel-level acceptance is claimed from the navigation geometry evidence alone.

Controls evidence: 110 implementation tests, lint, typecheck and build pass. Independent review fixed first-character loss when typing into a closed font picker and cleared-query restoration, and added two regressions. Its final 27 focused tests, lint and typecheck pass. The host-font implementation later identified the Select test race: synchronous getByRole inspected the DOM before portal synchronization. The tests now await findByRole; no Select production change was needed. The final focused frontend run at the host-font stage passed all 116 selected tests across 11 files; it was not the full repository frontend gate.

Base UI 1.8.0 adds 157,563 bytes (54.44 build-reported gzip kB) to the shared state chunk relative to the navigation baseline. Vite emits a size warning; the production build succeeds. No unrelated chunk-splitting change is included.

Host-font implementation evidence: 13 core preference tests, 83 desktop tests (2 intentionally ignored), 116 frontend tests, lint, typecheck, scoped formatting, and diff checks pass. The V2 store preserves strict read-only V1 migration and locked first-save behavior. DirectWrite exposes local font families through a main-window-only command. A 2,000-family jsdom measurement confirms list reachability and one enumeration call; it is not native latency evidence. Actual Windows enumeration, main/HUD rendering, and restart remain parent integration checks.

Host-font independent review: PASS. The reviewer corrected one outdated component-guideline paragraph about closed local font presets and found no unresolved code defect. Production code was unchanged during review; the scoped diff check passed. The palette child starts only after this result.

Palette recovery: the implementer transport returned HTTP 403 before its completion report. The user requested continuation. The existing palette edits were retained, and the same implementer received a continuation for only the remaining work. No palette review or final-gate success is inferred from that interruption.

Palette focused validation: 14 core preference tests, lint and typecheck pass. An intermediate full Vitest run exposed two related asynchronous test assumptions: App language tests queried portal options synchronously; the choice typeahead test sent keys before close/reopen focus settled. The implementer replaced those queries with awaited role lookup and added explicit listbox/focus condition waits while preserving the save-count assertions. The six affected test files then passed 96/96. Original failure output and final JSON are in the palette child's evidence directory. Final integrated gates remain pending.

At the earlier stage checkpoint, no commit, push, installer execution, or archive had occurred. The subsequent user-directed closure is recorded above. Existing operation-performance and native-acceptance tasks retain their separate scope and evidence.
