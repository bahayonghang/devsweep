# Research: Bounded planning package review

- Query: Review the parent planning package and six child PRD/design/implementation plans for contradictions, scope fidelity, dependency order, missing planned tests, and unsafe mbx assumptions.
- Scope: internal, read-only review; only this findings file written.
- Date: 2026-10-14

## Findings

The package is coherent about independent scopes, one heavy-operation coordinator, scope/report/digest isolation, explicit project roots, project opt-in selection, and the user-confirmed configurable 5 GiB scan-triggered warning. It honestly leaves mbx executable qualification open. All seven task.json files currently say `planning`, with correct parent/child links. No implementation test result is required or claimed in this review.

### 1. P2 — Baseline ownership is scheduled after the changes it must precede

- `task-map.md:12` and child F `prd.md:10` make F depend on A/B/C/D/E being implemented and checked.
- F `design.md:5` requires a current baseline **before product changes**, while F `implement.md:6` first requires those implementations and only then `:7` asks to reproduce baseline.
- Parent `implement.md:28` mentions pre-implementation baselining, but its ordered work starts A immediately at `:14` and identifies no owner/artifact handoff for that preflight.
- **Impact:** following the task order can lose the genuine before geometry/performance/native evidence, or tempt a late baseline from a changed tree.
- **Correction:** add an explicit parent-owned or F-preflight step before A/E writes product files, recording the required baseline manifest and source/binary identity. Keep F's final acceptance dependency on all children. Clarify that the preflight is read-only evidence preparation and does not mark F complete or require tests during this planning turn.

### 2. P2 — New palette application is ambiguous about preserving other modes

- Parent `prd.md:31` and `design.md:57` preserve other modes; child D `prd.md:19` repeats that boundary.
- `design-system/DESIGN.md:66` directs implementation to use the new light/dark role sets and `:68` maps them into all existing themes. D `design.md:14` owns `styles/preferences role mapping` without specifying selector scope.
- Existing appearance roles are shared across modes. Applying the green/charcoal canvas, text, border, and control palette to the shared root would visually change Software/Optimize/Analyze/Status and Settings as well as Clean.
- **Impact:** D can satisfy the literal palette instructions while violating the explicit Clean-only redesign boundary.
- **Correction:** state whether new roles are Clean-local aliases/overrides beneath the Clean root, with inherited saved theme/font/scale, and limit shared shell changes to the reviewed brand/control fixes. Add a before/after check that unrelated mode palette and geometry remain unchanged, apart from the intentional shared logo. If global palette replacement is intended, that is a scope decision to make explicit instead.

### 3. P2 — mbx qualification must enumerate command side effects, including the workspace link

- The project's own `research/scan-contracts.md:80` records that `mbx clean <WORKSPACE>` removes the managed target **link** in addition to target/private state.
- Parent `design.md:35` checks the link as ownership evidence, but `:36` lists the cleanup footprint as managed target/private state. Child B `prd.md:17` has a general live-footprint gate; its `:25` and `implement.md:19` specify simulated/mocked execution tests. Neither makes the link-removal effect and complete command mutation set an explicit preview/confirmation/test acceptance item.
- **Impact:** a provider can bind the correct destination yet preview only bytes, leaving a workspace-link mutation undisclosed; a mocked executor cannot itself prove the real mbx command stays inside the assumed footprint.
- **Correction:** the qualification deliverable should list every documented/reviewed side effect of the pinned mbx version (link entry, managed target, private state, and any metadata) and which paths are protected/digest-bound. Require preview copy for structural/link removal and fixtures for a changed/protected link, untouched shared store, unrelated-workspace sentinels, busy partial outcomes, and post-command disposition. State how actual tool behavior is corroborated beyond the mock: version-matched source/public-contract audit, or a separately authorized fully isolated disposable-workspace conformance test. Do not run user-cache cleanup or relax the no-follow/qualification gate. This is a missing planned proof detail, not a finding that executable mbx integration is currently unsafe or implemented.

## Checked without findings

- Warning semantics consistently use inclusive `5,368,709,120` bytes, adjustable settings, explicit user scans, partial-above warning and partial-below/unknown incomplete assessment. The build/test/tool subtotal with dependencies/venv separately displayed is clearly identified as a parent product interpretation, rather than silently treating dependency capacity as zero.
- Threshold changes re-evaluate observations without scanning or altering cleanup selection/digest; there is no scheduler or OS notification creep.
- Saved report/plan closed schemas are recognized; new read-only advisory DTOs avoid silently expanding persisted authority formats.
- A precedes B; C can do non-mbx fixture work after A but waits for B's ownership contract for final integration; D can prepare layout but waits for B/C/E integration; E is independent. The only ordering correction found is the baseline preflight above.
- mbx absence/unsupported/malformed/ambiguous cases, stale identities, shared storage, cross-workspace protection, generic Cargo reparse rejection, and prohibited gc/cache-remove/--under shortcuts are present. Unproven executable cleanup blocks full parent completion.
- Native sizes are consistent with the supplied current configuration: minimum 900x600, default 1080x720. Small fixture widths and DPR emulation are not misrepresented as actual native minimum/OS-scale evidence.
- Original beaver concept generation, compact refinement, asset packaging and native acceptance are separate. No copied Mole asset or new runtime UI library is required.

## Files reviewed / Related specs

- Parent `prd.md`, `design.md`, `implement.md`, `task-map.md`, and `design-system/DESIGN.md`.
- `prd.md`, `design.md`, and `implement.md` in each of `10-14-cleanup-scope-contracts`, `10-14-project-artifacts-mbx`, `10-14-project-capacity-warnings`, `10-14-desktop-cleanup-workbench`, `10-14-devsweep-animal-identity`, and `10-14-cleanup-home-acceptance`.
- Parent/child `task.json` metadata, relevant mbx facts in parent `research/scan-contracts.md`, and the prior source trace `research/home-ui.md`.
- Relevant previously loaded contracts: `.trellis/spec/desktop-frontend/component-guidelines.md` (shared themes, shell and visual evidence), `state-management.md` (scope/lifecycle/digests), `type-safety.md` (closed wire contracts), and `.trellis/spec/backend/desktop-preferences.md` (retained appearance values).

## External References / Versions

No new external lookup. mbx 1.24.0 and its link-removal behavior above are attributed to the task's persisted source/help research, not a fresh tool execution by this reviewer.

## Caveats / Not Found

No product code was changed or executed. Current native behavior, performance, actual mbx side effects, generated icon production quality, and implementation checks remain future acceptance work. Context manifests were being populated concurrently and were intentionally not evaluated; no implement/check manifests were loaded. Only planning consistency and the explicit metadata above were reviewed.

## Resolution verification — 2026-10-14

Verified only the three requested planning fixes; the original findings above remain historical.

| Finding | Status | Verified resolution |
| --- | --- | --- |
| 1. Baseline scheduling | RESOLVED in plan | Parent `implement.md:14` adds a parent-owned read-only step 0 before any product writes, with baseline artifact location and identity requirements; `task-map.md:14` establishes its handoff. F `design.md:5` and `implement.md:7` now consume that pre-edit manifest instead of recreating a late baseline. |
| 2. Palette scope | RESOLVED in plan | Parent `design.md:57`, design-system `DESIGN.md:66` and `:68`, and D `design.md:10` restrict new roles to the Clean root while retaining saved preferences and shared appearance roles. D `prd.md:31` requires other-mode/Settings before-and-after palette/geometry comparisons, except the intentional logo replacement. |
| 3. mbx complete mutation proof | RESOLVED in plan | Parent `design.md:36`, B `design.md:9`, and B `prd.md:26` enumerate link, destination, private-state and metadata mutations, bind protections/digest identities, disclose link removal, add relevant sentinel/partial-outcome tests, and require version-matched source/public-contract corroboration beyond mocks. Any fallback isolated conformance execution is separately authorized and excludes user cache storage. |

No unresolved finding remains from this bounded review. These statuses certify planning corrections only; they do not establish implementation, mbx qualification, baseline capture, or test completion. No broader review or product execution was performed.
