# Host Fonts Independent Check

Date: 2026-09-26. Reviewer: dispatched `trellis-check`.
Result: **PASS for the host-fonts implementation checkpoint.**

No blocking production-code defect was found in the reviewed paths. The
palette child can proceed. Parent integration still owns native Windows
acceptance and the final full gates. This checkpoint does not declare the
native acceptance criteria or the parent release complete.

## Findings (fixed)

- File: `.trellis/spec/desktop-frontend/component-guidelines.md:190`.
- Issue: The Visual System section still described closed local font presets.
  That statement conflicted with the new host catalogue and V2 contract.
- Fix: Describe the System option, installed host families, and the closed V2
  text-scale choices. No production code or existing test was changed.

## Findings (not fixed)

No unresolved code or design finding. The following evidence remains with the
parent under the dispatch contract:

- F-AC1: Actual Windows catalogue count, native first/middle/last selections,
  and available per-user families. Fixture enumeration does not prove OS data.
- F-AC2/F-AC3/F-AC6: Native keyboard/IME behavior, real main/HUD font rendering,
  fallback rendering, and persistence across an isolated process restart.
- F-AC7: A non-Windows build was not run on this Windows host. The source has
  an explicit unsupported-platform branch and Windows-only dependency.
- Final `just ci`, full desktop web gate, production build, and native surface
  evidence run after the palette child. No task status or acceptance checkbox
  was changed by this reviewer.

## Source review

| Contract | Source evidence and conclusion |
| --- | --- |
| Strict V2 and V1 mapping | `crates/devsweep-core/src/desktop_preferences/mod.rs` and `legacy.rs`: closed documents, tagged font shapes, scalar/control/trim validation, separate legacy theme decoder, all three preset mappings, unchanged unrelated fields. |
| Read authority and migration | `store.rs:136` reads V2 first and only reads V1 on NotFound. Invalid, future, or unreadable present documents fail. Read does not create files. `store.rs:170` takes the existing transaction lock before the authoritative reread and patch. |
| Atomicity and preservation | Same-directory temporary write/flush/sync precedes atomic replacement. The shared V1 mutex name remains unchanged. Migration and later updates preserve V1 and language bytes. Tests cover concurrent first saves, injected replacement failure, read-only V2, resets, and the failure matrix. |
| Commit ordering | `desktop/src-tauri/src/desktop_preferences.rs` orders get/update/publication through one coordinator and rejects exhausted safe sequences before persistence. Failed operations publish nothing. Renderer `preferences/store.ts` retains the same-bridge sequence and rejects disposed or older snapshots. HUD reloads committed settings on show. |
| Native discovery | `desktop/src-tauri/src/fonts.rs:67` creates and consumes DirectWrite interfaces inside the blocking worker. `GetSystemFontCollection(false, ..., true)` excludes downloadable fonts and checks for updates. The command returns owned names only. Family-level enumeration prefers en-US, merges case-insensitive duplicates and aliases, and has no count cap. |
| Window and sampling boundaries | `lib.rs` registers `desktop_fonts_list` but the HUD allowlist remains language/preferences reads. The allowlist test iterates every shipped command. Discovery has no sampling, shell, registry, path, byte, or download side effect. |
| Catalogue lifetime | `preferences/fonts.ts` caches by bridge, loads through Settings, coalesces requests, retains the previous list after refresh failure, and rejects stale/disposed responses. The settle/dispose/remount regression covers the corrected same-turn race. |
| Selection and CSS | `FontPicker.tsx` keeps System available, retains missing saved names, searches aliases, and resolves explicit selections against returned canonical families. `SettingsCombobox.tsx` preserves the first typed character and cleared query. `appearance.ts:7` quotes and escapes one validated family; main/HUD consume the same custom property with existing fallbacks and scale. Technical monospace remains separate. |
| IPC and generated contract | Closed decoders, font/V2/patch fixtures, generated TypeScript, bridge registration, generator references, source-boundary registry, and Rust wire parity agree. Unicode whitespace and scalar handling match the Rust font boundary. |
| Dependency and specs | Direct `windows = =0.61.3` is Windows-target-only with DirectWrite enabled. The implementation records the existing lockfile package and compatible MSRV. Backend persistence and frontend state/type specs cover V2, migration, native ownership, and downgrade behavior. |

## Verification

The implementation stopped production edits before this check completed. The
reviewer changed only the specification clause and this report. Successful
implementation checks were reused as instructed; no unchanged full gate was
rerun.

| Check | Result and evidence |
| --- | --- |
| Lint | PASS, exit 0 in implementation evidence: `mise exec node@22 -- npm run lint`. |
| TypeCheck | PASS, exit 0 in implementation evidence: `mise exec node@22 -- npm run typecheck`. |
| Core tests | PASS, 13 desktop preference tests in implementation evidence. Source review confirms the migration, preservation, concurrency, validation, and rollback cases. |
| Desktop tests | PASS, 83 tests and 2 expected ignored process fixtures in implementation evidence; includes command gate, event ordering, wire parity, and native catalogue transformation. |
| Frontend tests | PASS, 116/116 across 11 files. Reviewer read `focused-tests.json` and the final log: success=true, failed=0. Coverage includes fonts, preferences, controls, API, locales, and styles. |
| Generation | PASS, V2/font generation completed with exit 0 in implementation evidence; generated DTOs were compared with Rust and fixture ownership. |
| Review diff check | PASS, scoped `git diff --check` after the specification correction. |

The 2,000-family test exposes 2,001 options including System. The recorded
Windows x64 / Node 22.23.2 / Vitest-jsdom sample measured 2089.10 ms for opening
and rendering and 92.45 / 28.21 / 16.94 ms for the three queries. The tests prove
first/middle/last and alias reachability and one native-bridge request while
typing. The measurements are diagnostic and do not establish native latency.

Reviewed evidence: `implementation-report.md`, `focused-tests.json`,
`focused-tests.log`, and `font-interaction-measurement.json` in this directory.
No fonts, OS settings, user preference files, cleanup actions, task status,
commits, or archives were changed.
