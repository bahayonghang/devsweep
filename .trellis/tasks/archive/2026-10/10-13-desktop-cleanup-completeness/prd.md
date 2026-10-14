# CLI and Tauri cleanup completeness, Earth quality, and performance

## Goal

Make DevSweep find and explain supported cleanup candidates across explicitly chosen locations, complete the safe cleanup workflow consistently in CLI and Tauri, and remain responsive with bounded foreground/background cost. Replace Clean's artificial globe with an offline real-Earth visual.

The original planning deliverable was approved for implementation on 2026-10-13.
Current implementation and acceptance are tracked in
[implementation-progress.md](implementation-progress.md) and
[acceptance-status.md](acceptance-status.md). This task does not authorize
cleaning the user's machine.

## Source requirements and accepted decisions

| ID | User requirement / decision | Owner |
| --- | --- | --- |
| SRC-01 | Improve the unrealistic Earth and rendering performance in the supplied screenshot | UI child |
| SRC-02 | Investigate incomplete folder discovery and cleanup using `ref/Mole` and mole.fit; create UI and functional subtasks | Functional child; shared research |
| SRC-03 | User selected real Earth with locally bundled redistributable texture on 2026-10-13 | UI child; targeted prospective spec update |
| SRC-04 | User extended scope to both CLI and Tauri and frontend/backend performance | Functional parity and runtime child |

## Confirmed background

- P1 scope gap: desktop starts with `roots: ["."]` and exposes only projects/global toggles, so launch location controls project coverage (`desktop/src/modes/clean/CleanWorkbench.tsx:49`; `desktop/src/pages/ScanPage.tsx:96-99`). The CLI already has explicit-root controls; its defaults need not change to match the desktop UX.
- P1 functional inconsistencies: provider path parsing rejects spaces, custom cache locations do not consistently round-trip through trusted rules, and a Go discovery rule lacks matching scan-action intent conversion (`crates/devsweep-core/src/scan/global/mod.rs:502-557,271-305`; `crates/devsweep-core/src/rules/registry.rs:54-68,159-170,353-364`). The functional research records additional nested Python and reporting gaps.
- P1 appearance defect: the globe's continents are scalar-noise colour bands and its 256px source is scaled into a larger DPR-based output (`desktop/src/stage/planet-renderer.ts:116-142,156-201`; `Planet.tsx:59-80`). No current Planet timings were supplied or measured.
- Code-confirmed performance risks include per-target cumulative plan cloning/ranking and preview conversion before IPC throttling (`crates/devsweep-core/src/scan/mod.rs:338-347,394-403`; `desktop/src-tauri/src/scan.rs:147-180`). The runtime research distinguishes risks from measured historical evidence.
- Mole's local macOS CLI and its native Mac/Windows web pages are different evidence sources. The reusable lessons are wider explicit coverage, owner-aware cache rules, meaningful review, and visible exclusions. Native implementation and performance cannot be inferred from the CLI or screenshots.

## Task map

| Child | Deliverable | Boundary |
| --- | --- | --- |
| [10-13-desktop-planet-fidelity](../10-13-desktop-planet-fidelity/prd.md) | Real Earth, offline assets, bounded renderer, visual/native evidence | Stage/Planet and its lifecycle; preserve other mode identities |
| [10-13-cleanup-discovery-execution](../10-13-cleanup-discovery-execution/prd.md) | Shared rule correctness, visible scan roots/coverage, scoped provider expansion, CLI/Tauri review and outcomes | Core contracts and both operational adapters; no deletion-authority shortcuts |
| [10-13-cli-tauri-runtime-performance](../10-13-cli-tauri-runtime-performance/prd.md) | Release baselines, measured scanner/output/IPC/render/lifecycle fixes, foreground/background evidence | Shared runtime and observers; preserve result coverage and operation-specific cancellation |

The parent owns source requirements, scope boundaries, decisions, cross-child contracts and final integration. It is not the product implementation target. Child ordering and shared-file ownership are explicit in `design.md` and each `implement.md`.

## Requirements

- P-01 / SRC-01,03: Deliver recognizable real geography and a responsive offline globe with measured rendering budgets, fallback, visibility suspension, reduced motion, and native scaling evidence.
- P-02 / SRC-02,04: For identical effective scan options and fixtures, CLI and Tauri produce semantically equivalent targets, coverage diagnostics, risk/selection decisions and executable plans. Fix current round-trip failures before catalogue expansion.
- P-03 / SRC-02: Make scope explicit before scanning through selected-root add/remove/deduplication, without new exclusion-pattern syntax. Every selected root/provider must be evaluated or receive a typed skipped/partial/error reason. Empty, absent, unavailable, protected, busy, unsupported and incomplete are different outcomes. Existing cleanup protection is distinct from scan scope. Do not label a partial scan as whole-machine clean.
- P-04 / SRC-02,04: Expand marker-backed project artifacts, configured developer-tool caches, and a bounded set of Windows current-user browser/editor/temp caches. The functional coverage matrix is the deliverable contract; a category cannot be counted supported without a validated executor or visible inspect-only reason.
- P-05 / SRC-02,04: CLI human output and Tauri review expose paths, size confidence, category/rule, rebuild cost, execution capability, skips/errors and next action. JSON/exit status preserve truthful partial results. Recycle Bin moves and command cleanup have distinct capacity claims.
- P-06 / SRC-04: Measure and reduce actual redundant backend work, output/IPC backlog and UI stalls. CLI one-shot work exits cleanly; TUI input stays responsive; desktop hidden/idle producers follow an explicit lifecycle matrix. Performance improvements must preserve the selected roots, rule outcomes, safety checks, and partial/unknown semantics.
- P-07: Keep scans and previews read-only. Execute only reviewed selected targets through the CLI saved plan or Tauri reviewed versioned in-memory plan payload, both using core validation, live SHA-256 preview digest, explicit confirmation, live authorization and durable audit. Provider footprint/tool/configuration bindings participate in preview and live revalidation. Preserve Cargo-home inspect-only and no permanent-delete/Docker rules.

## Acceptance Criteria

- [ ] P-AC1 (P-01): UI child visual, offline, lifecycle and benchmark criteria pass on the documented native build; source and notices are recorded.
- [ ] P-AC2 (P-02,P-03): A golden fixture set covers multiple roots outside CWD, spaces/Unicode, nested projects/caches, custom provider roots, unavailable tools, permission errors, reparse points, protected paths and cancellation/budget limits. CLI and Tauri agree semantically for identical effective inputs.
- [ ] P-AC3 (P-04,P-07): Every required rule/provider in the functional matrix has scan-to-plan-to-validation-to-preview coverage and an explicit execution/inspect-only outcome. Positive execution verification uses isolated disposable fixtures or fake runners; negative tests prove excluded data remains unchanged.
- [ ] P-AC4 (P-05): Human and machine results accurately distinguish complete/partial/failed/canceled, usable versus inspect-only candidates, and moved-to-Recycle-Bin versus owner-command results. No inflated 'freed' claim or silent whole-report loss from a malformed provider.
- [ ] P-AC5 (P-06): Current release CLI and native Tauri raw measurements meet frozen runtime budgets. Baseline/candidate use the same host, fixture and effective coverage. Frontend interaction, backend acknowledgment/join and process-tree resources are reported separately.
- [ ] P-AC6 (P-07): Selection/protection/scope changes invalidate stale previews; no frontend action reconstruction, scanner mutation, unrelated process termination or destructive benchmark side effect appears.
- [ ] P-AC7: Root `just ci`, relevant desktop quality/wire-generation checks and native acceptance are recorded. Existing historical failures are tracked as inherited evidence to remeasure, not treated as current passes or automatically attributed to Planet.
- [x] P-AC8 (planning deliverable): Parent and all three children contain PRD/design/implementation plans, valid real context manifests, correct tree links, and research provenance. The reviewed plan was approved and all four tasks entered implementation. See [planning validation](research/planning-validation.md).

## Out of scope

Permanent delete/emptying the Recycle Bin; blanket drive/AppData/cache-directory removal; Docker cleanup; Cargo-home mutation; privilege escalation/system-store cleanup; software-uninstaller expansion; AI session/memory/skill/model or whole-worktree deletion; cloud/VM/backups; unrelated five-mode features; Mole source/assets or a new 3D dependency. Windows-native behavior is the desktop acceptance target; existing Rust cross-platform behavior must not regress.

## Risks and deliberately scheduled technical decisions

- Wider discovery must not promote authored or costly personal state to disposable data. New providers are independently gated with retained-file sentinels and live owner/path checks.
- Absolute scan timing depends on storage, fixture and tool behavior. Baseline capture and renderer comparison are implementation work; no fresh performance result is claimed by this plan.
- Historical performance scripts include side effects. The runtime child must first provide an isolated owned-process/read-only measurement path before invoking them.
- Natural Earth is a verified redistributable candidate; selecting/downsampling its exact visual asset and choosing the lowest passing Canvas source size are scheduled technical steps. A package addition or materially different deletion policy returns to planning.

## Artifact status

The user approved this final plan on 2026-10-13 and requested implementation in the recorded Trellis order. The accepted Earth direction, bounded provider expansion, CLI/Tauri scope and performance acceptance are assigned to the three children. Implementation progress and explicit ownership are recorded in [implementation-progress.md](implementation-progress.md); open acceptance criteria still require evidence.
