# Research: Planning integration review

- Query: Review the parent and three child PRD/design/implementation plans, including the functionality coverage matrix, against source findings and the expanded CLI/Tauri scope.
- Scope: internal; planning validation only.
- Date: 2026-10-13

## Findings

The plans cover the material current defects: invisible desktop CWD scope, Go conversion failure, configured-cache path mismatch, nested Python ownership mismatch, provider parsing/diagnostics, CLI presentation, and desktop empty/command results. Ownership, platform boundaries, read-only benchmarking, whole-plan rejection, fake/disposable execution, and original safety protections are explicit. No omission requires another child task. Resolve the following integration details before implementation.

### R1 — Add a post-functional performance reference build (P1)

- Files: parent `design.md:32-36`; runtime child `prd.md:44-45`, `design.md:5`, `implement.md:12-19`.
- Issue: the schedule preserves only the original pre-change build, then repairs rules/coverage and optimizes. The runtime criterion requires semantically identical golden results at equal inputs. Original Go-containing fixtures fail report conversion, configured/nested fixtures fail validation, and partial statuses intentionally change. “Old versus added coverage” alone does not distinguish repaired behavior from an optimization.
- Action: retain three identities: original baseline; functionally corrected/stable-DTO build before runtime optimization; final optimized build. Compare optimization parity/timing against the second on repaired fixtures. Keep original-to-final measurements for unchanged healthy workloads and end-user improvement, explicitly identifying behavior changes. The functional handoff should publish the second binary plus golden observations/plans before overlapping runtime edits.
- Source proof: `crates/devsweep-core/src/plan/mod.rs:266-310` and `rules/registry.rs:54-68` reject current Go report conversion; the planned repair intentionally changes that result.

### R2 — Freeze the provider authority snapshot/digest contract (P1)

- Files: functional child `prd.md:36,47`; `design.md:22-24,40-42`; `implement.md:12`.
- Issue: config/tool identity drift rejection is required, but the design does not yet state which live facts enter the preview digest, how execute reacquires/comparatively checks them, or how CLI saved-plan and Tauri payload paths share that contract. Existing rule reconstruction uses names such as `npm`/`pnpm`; the digest hashes those names/argv/cwd, not the resolved executable identity or configuration snapshot.
- Action: add a mandatory pre-code contract decision describing rule-specific validated footprint, executable identity/version policy, configuration fingerprint, digest binding, live reacquisition, and compatibility behavior. Decide whether this is preview-time derived authority within v2 or requires a persisted authority-version change. Do not accept frontend-submitted identity/config fields as authority. Add same-named executable replacement and same-path config-content drift fixtures to F-AC7.
- Source proof: `crates/devsweep-core/src/rules/registry.rs:238-247,282-287`; `plan/digest.rs:78-105,134-150`; `desktop/src-tauri/src/clean.rs:34,74` independently validate at each call.

### R3 — Clarify exclusion semantics and CLI reachability (P2)

- Files: functional child `prd.md:31,43`; `design.md:32-34`; parent `prd.md:39`.
- Issue: desktop supports “remove/exclude”, but neither the planned effective-scope schema nor CLI syntax defines an exclusion. Removing an entire selected root is different from excluding descendants or applying a pattern; this affects candidate coverage, persistence, stale preview rules, and normalized CLI/Tauri parity.
- Action: explicitly define whether exclusion only removes a root or is a core-owned descendant exclusion list. For descendant exclusions, specify normalized literal-path versus pattern semantics, typed skipped coverage, persistence, and an explicit CLI argument (with parser/reference tests) or deliberate documented reachability boundary. Keep scan exclusion separate from durable cleanup protection.
- Source proof: current `ScanOptions` has only booleans/roots (`crates/devsweep-core/src/scan/mod.rs:159-169`); CLI scan has root/scope/rescan only (`crates/devsweep-cli/src/application/cli.rs:78-90`).

### R4 — Correct the shared saved-plan wording (P2, wording)

- File: parent `prd.md:44` (P-07).
- Issue: “existing saved-plan” reads as if Tauri already requires a disk-saved plan. CLI does; Tauri currently passes and validates a reviewed versioned `UntrustedPlan` payload directly. No child proposes a GUI persistence step.
- Action: say “CLI saved plan / Tauri reviewed versioned plan payload, both through current validation, preview digest, confirmation, live authorization and audit.” If disk persistence is intended for Tauri, specify it as new behavior with an owner; do not accidentally imply it already exists.
- Source proof: `crates/devsweep-cli/src/application/commands/clean.rs:209`; `desktop/src-tauri/src/clean.rs:24-40,59-76`.

## Files found / related specs

- Read parent `prd.md`, `design.md`, `implement.md`; the same three files under `10-13-desktop-planet-fidelity`, `10-13-cleanup-discovery-execution`, and `10-13-cli-tauri-runtime-performance`; and functional `coverage-matrix.md`.
- Related contracts: `.trellis/spec/backend/quality-guidelines.md` execution/plan/scanner boundaries; `.trellis/spec/desktop-frontend/state-management.md` completed-report authority and stale digest invalidation; current source findings in `research/current-cleanup-gaps.md`.
- External references: none needed for this integration review; asset licensing and provider documentation remain assigned in the plans.

## Caveats / Not Found

- Static planning review only; no code changes, test executions, machine scans, or cleanup.
- Metadata and context manifests were being finalized by the parent and were deliberately not reviewed here; no implementation/check manifests were loaded.
- Exact provider documentation and native performance are correctly scheduled evidence, not present-day pass claims. New provider rows remain proposed scope subject to planning approval and their recorded enablement gates.

## Resolution verification — 2026-10-13

All four planning findings are resolved in the revised authoritative plans; no remaining contradiction was found within this targeted verification.

- **R1 resolved:** parent `design.md:32-36`, runtime `prd.md:31,44-45` / `design.md:7,49` / `implement.md:12-13`, and functional `implement.md:19` consistently define original A, corrected reference B, optimized C, the functional handoff, and refreshing B after semantic changes. Broken original results are explicitly excluded as correctness baselines.
- **R2 resolved:** functional `design.md:30-36` defines the core-derived footprint/executable-content/capability/config binding, canonical preview digest, live reacquisition, bound program dispatch, and legacy evidence/rescan policy. Functional `prd.md:36,47` and `implement.md:12` require replacement/config-drift negatives and freezing serialization before adapter work. Parent `prd.md:44` reflects the binding.
- **R3 resolved:** parent `prd.md:40` and functional `prd.md:31,43` / `design.md:40-42` / `implement.md:14` limit this task to selected-root add/remove/deduplication, preserve parent-root coverage, introduce no descendant/glob exclusion grammar, and separate durable protection from read-only scope.
- **R4 resolved:** parent `prd.md:44` and functional `design.md:32` distinguish CLI disk-saved plans from Tauri reviewed in-memory versioned payloads while retaining common validation/confirmation/audit requirements.

This verifies planning text only, not implementation or safety/performance test results. Final child plans override earlier research proposals where scope was narrowed.
