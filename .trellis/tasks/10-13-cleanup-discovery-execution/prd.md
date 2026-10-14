# Cleanup discovery coverage and executable outcomes

## Goal

Make supported folders discoverable and safely actionable in both CLI and Tauri, with explicit scope, complete per-provider accounting, usable review, and truthful outcomes.

## Ownership and dependencies

- Parent: `10-13-desktop-cleanup-completeness`. Own core scan/rule/plan/eligibility contracts, CLI Clean presentation, Tauri Clean IPC, React scope/review/result state, relevant tests and documentation.
- UI child owns Planet, not Clean controls. Runtime child owns progress scheduling, sizing throughput, output backpressure and measurements. Agree changed event/coverage DTOs before either changes overlapping files.
- Sequence: capture runtime baseline first; repair current contracts (F-01) and explicit roots/coverage (F-02/03), then add providers (F-04) and finish front-end parity. Runtime optimization integrates after shared contracts stabilize. Earth work is independent.

## Source-confirmed problems

| Priority | Evidence | Requirement |
| --- | --- | --- |
| P0 | Go discovery emits `go.mod_cache.clean` but action-to-intent and trusted reconstruction omit it; report conversion fails (`scan/global/mod.rs:271`; `rules/registry.rs:54,229`; `plan/mod.rs:266,310`) | F-01 |
| P0 | Configured Gradle/NuGet/Go roots disagree with default-only registry and live safety (`scan/global/mod.rs:532`; `rules/registry.rs:159,353`; `execution/safety/mod.rs:780`) | F-01 |
| P0 | Desktop roots are `["."]` with no editor (`CleanWorkbench.tsx:49`; `ScanPage.tsx:96`) | F-02 |
| P1 | Provider parser rejects space-containing paths and names containing `warning`/`error` (`scan/global/mod.rs:502`) | F-01,03 |
| P1 | Nested Python caches inherit a root but violate exact reconstructed path (`scan/project/mod.rs:252,480`; `rules/registry.rs:130-143`) | F-01 |
| P1 | Global provider API loses diagnostics (`scan/mod.rs:88,394`; `scan/global/probe.rs:49,88`); first Python executable can mask a working alternative (`global/mod.rs:156`) | F-03 |
| P1 | CLI scan/preview/execute human output omits review facts and partial JSON says succeeded (`application/commands/clean.rs:232-334`) | F-05 |
| P1 | Desktop zero-target state loses diagnostics; command-only cleanup can show only 0 B moved (`CleanWorkbench.tsx:217,254,258`; `modes/clean/result.ts:10`) | F-05 |

Rust paths above are under `crates/devsweep-core/src/` except the CLI path under `crates/devsweep-cli/src/`; TSX paths are under `desktop/src/modes/clean/` or `desktop/src/pages/` as listed in the full research. These are static failure conditions, not reproduced incidents on the user's machine. See `../10-13-desktop-cleanup-completeness/research/current-cleanup-gaps.md` for full anchors and existing test gaps.

## Requirements

- F-01: Every scanner-emittable rule round-trips through real report conversion, persisted v2 intent, trusted reconstruction and preview. Register Go's bounded owner-command contract, unify trusted configured-root resolution, preserve valid spaces/Unicode, and support nested Python caches with explicit per-rule ownership. Unknown/tampered rules still fail closed. Do not weaken whole-plan validation to hide broken candidates.
- F-02: Desktop shows resolved project roots before scanning, supports adding/removing/deduplicating selected roots and remembers approved selections. Removing a root removes that selection only: it does not exclude descendants still covered by another selected parent. No new path-pattern or negative-scope grammar is introduced. Suggest existing conventional project containers and exact agent-worktree containers without scanning all drives in the background. First use requires a meaningful visible choice; no hidden CWD default. CLI retains repeated `--root` semantics and reports its effective roots; desktop preferences do not silently change CLI defaults. Empty project scope is a visible validation state.
- F-03: Add typed root/provider coverage and complete/partial health. Healthy roots/providers continue after independent unreadable/missing/failed ones where safe. Absent tool, unsupported platform/version, empty result, protected, busy, unresolved ownership, timeout, truncation, permission failure, canceled and budget-limited are distinguishable. Never report missing coverage as zero cleanable bytes. Avoid logging secrets or entire environment/config contents.
- F-04: Deliver the finite required waves in `coverage-matrix.md`: existing providers fixed, additional evidence-backed project outputs/tool caches, and bounded Windows current-user browser/editor/temp providers. Each rule needs discovery/config precedence, ownership, risk/rebuild cost, size semantics, selection policy, live validator, exact footprint and an executor or a visible inspect-only reason. Broad names alone never authorize deletion.
- F-05: CLI human output displays selectable ids, paths, rule/category, size confidence, action/risk, coverage and next steps; preview shows selected footprints and irreversibility; results show target-level success/skip/failure/unknown. Tauri keeps diagnostics reachable for empty/partial reports and distinguishes owner commands from Recycle Bin results. Use existing bilingual catalogues. JSON outcomes/warnings and exit classes follow shared semantics, including partial and skipped-only cases.
- F-06: Provide Tauri reviewed-size rescan using core's existing larger-budget helper; a changed report replaces observation state and invalidates selection/digest as needed. Unsupported large folders can open the existing Analyze/reveal flow, without turning analysis observations into cleanup authority.
- F-07: Add read-only eligibility information for likely live refusals where available, then recheck all authority at execute. Bind the validated provider footprint, resolved executable/interpreter identity, supported capability/version and relevant nonsecret configuration fingerprint into the core preview digest as specified in `design.md`. Config/tool/path/owner/protection drift invalidates or refuses the action. Preserve explicit selection, live preview digest, confirmation, audit and program/argv separation. Owner commands remain explicitly irreversible and unselected by default; direct permanent deletion stays disabled.
- F-08: Retain existing cross-platform Rust rules; new Windows app/temp providers declare unsupported status elsewhere. Same effective roots, scope and provider context yield the same core facts across CLI, TUI where exposed, and Tauri. Do not build a second scanner/executor or shell out to DevSweep from Tauri.

## Acceptance criteria

Current evidence is mapped in the [functional acceptance summary](evidence/acceptance-summary.md).
F-AC3 remains open for the combined GUI denied-root/healthy-project/global-provider
case; separate GUI project-only and CLI all-scope results do not close it.

- [x] F-AC1 (F-01): Production conversion/validation/preview tests cover every existing rule; Go plus unrelated candidates completes; custom Gradle/NuGet/Go paths work or receive precise refusal; nested Python cases match ownership contracts. Tests cannot bypass registry via `ValidatedPlan::from_cleanup_plan_for_test`. [Evidence](evidence/acceptance-summary.md#f-ac1-through-f-ac8).
- [x] F-AC2 (F-01,F-03): Parser/probe fixtures cover spaces, Unicode, CRLF, benign stderr, ambiguous stdout, paths named with warning/error substrings, nonzero exits, truncated output, timeout, missing tool and Python fallback. Invalid output never grants authority. [Evidence](evidence/acceptance-summary.md#f-ac1-through-f-ac8).
- [ ] F-AC3 (F-02,F-03): Launch GUI from an unrelated directory and scan two explicit roots including a different drive fixture; show both effective roots and exact outcomes. Overlaps dedupe; removing a selection preserves any coverage from remaining parent roots; invalid/empty selections explain recovery; one denied root does not erase healthy/global findings. Existing cleanup protection is presented separately from scan scope. [Verified parts and remaining case](evidence/acceptance-summary.md#exact-remaining-f-ac3-case).
- [x] F-AC4 (F-03,F-05): Zero targets after probe failure shows incomplete coverage; genuinely empty complete scan is distinct. CLI human/JSON and Tauri agree on normalized status/diagnostics; partial CLI uses existing partial exit class. All-skipped execution cannot imply files were cleaned. [Evidence](evidence/acceptance-summary.md#f-ac1-through-f-ac8).
- [x] F-AC5 (F-04,F-07): Every required matrix row has positive/negative rule fixtures and a real scan->v2->validate->preview roundtrip. Fake execution receives only approved actions. Retained-file sentinels prove source, secrets, profiles, offline storage, model/session data and protected/reparse targets survive. [Evidence](evidence/acceptance-summary.md#f-ac1-through-f-ac8).
- [x] F-AC6 (F-05,F-06): Mixed command/trash, command-only, all-inspect-only, all-recent, unknown-size and failed cleanup produce useful results. Recycle Bin bytes are not labeled freed; a prune command's entire observed store is not assumed reclaimable. Reviewed rescan and changed root/selection/protection invalidate stale previews. [Evidence](evidence/acceptance-summary.md#f-ac1-through-f-ac8).
- [x] F-AC7 (F-07,F-08): Tampered paths, changed provider config/tool identity (including same-path config-content changes and a replaced same-named executable), changed marker/owner state and stale digest refuse execution before side effects. Legacy reports missing required provider scope request rescan rather than acquiring new authority. CLI and Tauri share binding/revalidation services and generated wire parity; no new direct shell authority. [Evidence](evidence/acceptance-summary.md#f-ac1-through-f-ac8).
- [x] F-AC8: Root `just ci`, frontend lint/typecheck/test/build, generated type/fixture parity and bilingual CLI reference checks pass. Native Windows smoke evidence covers a packaged GUI environment and command shims using isolated fixtures, with no user-cache cleanup. [Evidence](evidence/acceptance-summary.md#f-ac1-through-f-ac8).

## Out of scope

Unrestricted entire-disk cleanup, arbitrary directory deletion, automatic app termination, privilege escalation, Windows Update/Installer/WinSxS stores, Recycle Bin emptying, Docker, Cargo-home cleanup, app uninstall expansion, AI histories/memory/models, whole-worktree removal, generic downloads/installers and personal/cloud/VM/backups. Inspect-only catalogue entries are not a blanket request to enable them.

## Artifact status

The user approved the parent and these provider waves on 2026-10-13. Current tool-specific argv/config/version documentation and native fixtures must be verified before each new provider is enabled. Missing evidence blocks that provider's completion and must be reported, not silently removed from the acceptance matrix. Product edits begin after original baseline A is preserved.
