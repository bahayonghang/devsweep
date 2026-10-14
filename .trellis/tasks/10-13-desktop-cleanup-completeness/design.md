# Programme design and child integration

## Architecture retained

```text
CLI command / TUI                  React + Tauri adapter
           \                      /
            shared typed devsweep-core services
             | discovery + coverage (read-only)
             | validated plan + explicit selected ids
             | preview digest + confirmation
             | live safety + durable audit
             + trusted command runner / Recycle Bin

React Stage -> local Earth renderer (presentation only)
Observers -> bounded progress + measured UI/output lifecycle
```

No second cleaner, shell-composed command adapter, GUI-owned action catalogue, or background whole-disk scan is introduced. Wider discovery and better presentation are independent from execution authority. The functional child fixes candidate-to-plan consistency through the current core trust funnel.

## Responsibility boundaries

| Owner | Files / decisions | Coordination |
| --- | --- | --- |
| UI child | `desktop/src/stage/`, Earth assets/notices, renderer benchmark, targeted visual/provenance spec update | Functional child owns scope controls/content; runtime child owns integrated measurement |
| Functional child | Core rule catalogue/resolvers/coverage models/plan tests; CLI Clean output; Tauri Clean commands; React roots/review/results; generated wire/locale docs | Freezes DTOs and golden fixtures before shared runtime edits |
| Runtime child | Safe measurement tooling; scan/sizing publication cost; CLI sinks/TUI fairness; Tauri projection/backpressure; React large-result render cost; Status/HUD measured fixes | Explicit handoff for overlapping scan, CLI output, Tauri scan, TargetTable and CleanWorkbench files |
| Parent | Accepted decisions, source-to-child mapping, cross-layer spec consistency, integration evidence and review | No duplicate product implementation under the parent |

Tree links are not dependencies. The schedule below is authoritative:

1. Runtime captures original baseline A and fixture/build identity before any product change.
2. Functional existing-rule repair and coverage/scope contract freeze precede runtime changes to shared scan/event models. Functional handoff preserves corrected reference B and golden results before runtime optimization; refresh B whenever functional semantics change.
3. UI asset/render work is independent after baseline preservation and targeted spec update.
4. Functional provider waves use shared performance instrumentation. Runtime optimizations land against stable DTOs with explicit file handoffs; no simultaneous edits to those shared modules.
5. Parent integration waits for all three child deliverables. Compare optimized candidate C against B at equal repaired/new coverage, and against A for previously healthy unchanged workloads. Publish added-coverage costs separately; do not compare failed original results as if they were equivalent work.

## Shared contracts to settle before code

- Effective scan scope: desktop visible persisted roots versus unchanged CLI explicit/default-root semantics; no invisible process CWD in the desktop.
- Coverage: a selected root/provider always has a typed outcome, including empty, absent, unavailable, refused or partial. Safety exclusions and failures are not cleanable capacity.
- Rule resolution: scanner, persisted intent reconstruction and execution share validated provider configuration/footprint policy. Unknown caller data never becomes authority.
- Capacity: observed bytes, selected/action estimate, moved-to-Recycle-Bin bytes and command outcomes are distinct. A full package store is not automatically reclaimable by prune.
- Progress: interactive observations are cumulative, bounded in retention, and contain no action authority. Terminal-only CLI avoids unused snapshots. Final/phase events and join are preserved.
- Lifecycle: decorative renderers suspend when hidden; hidden HUD joins; explicitly requested operations continue on minimize; cleanup preview/execute remain wait-only. There is no universal cancel-to-join timeout.
- Earth: user-approved real geographic local texture with documented provenance and small payload; Canvas2D-first benchmark, no new dependency or Mole asset.

## Compatibility and specification work

Respect current CLI grammar/envelope/exit classes, plan v2 trust boundary, create-new outputs, bilingual catalogue ownership, generated desktop types and closed decoders. New observation/coverage DTOs need paired Rust/TypeScript fixture migration and older-file compatibility tests. Do not upgrade old observations into new authority.

The UI child/main coordinator revises only current procedural-only Clean clauses, not archived history or the repository's managed Trellis block. Functional changes update relevant backend scan/coverage/intent and frontend state/type contracts. Runtime changes to progress publication, bounded channels or output sync require narrow matching spec updates before integration. Other protections remain unchanged.

## Reference use and independent design

`research/mole-reference-and-assets.md` pins the local Mole revision and official Mac/Windows pages. Transfer concepts: explicit category coverage, rebuild cost, review/skip/protect, active-owner checks and visible results. Implement independent Rust and original DevSweep presentation. No macOS-specific destructive command, asset or catalogue is imported verbatim.

Mole's displayed native rendering does not prove a suitable Tauri renderer, and its category counts do not prove our coverage. Validate the bounded matrix from actual DevSweep fixtures. Natural Earth is a verified candidate source; exact asset processing and native appearance remain implementation work.

## Evidence and completion

Research reports classify code-confirmed mechanisms, historical measurements and untested hypotheses separately. There has been no current native benchmark, full-machine scan or cleanup in this planning turn. Required implementation evidence includes real round-trip tests, retained-data sentinel tests, CLI/Tauri semantic parity, production screenshots, release process-tree/phase profiles, slow-consumer/cancel/join scenarios and full quality gates.

Parent completion requires all required child acceptance rows, including native performance, to pass or receive an explicit reviewed scope decision. An unavailable native check is tracked with an owner; it is not counted as a pass. A measured failure remains failed regardless of historical task archival.

## Rollback and delivery

Keep current-rule fixes independent from provider additions and performance changes. Each new provider can be disabled/reverted without loosening validation. Each renderer/runtime change can be reverted without removing root/coverage fixes. No user cache migration, automatic cleanup or irreversible system operation is required to deploy these application changes.
