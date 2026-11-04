# Research: Planning consistency review

- Query: Review the 16-child CLI-first GPUI plan after adding AI-tool cleanup and latest-compatible dependencies; identify substantive safety, ownership, acceptance-order and source-scope contradictions.
- Scope: internal planning review, with existing source research treated as attributed evidence rather than independently reverified external facts.
- Date: 2026-10-15.
- State: all findings refer to the reviewed planning snapshot; the coordinating session may repair them concurrently. No implementation approval, builds, installations, task starts or product changes occurred in this review.

## Findings

### F1 — P1: C15 completion requires its own downstream UI

**Evidence:** `../10-15-gpui-ai-tool-cleanup/prd.md:31` requires CLI/GPUI report and digest parity for C15-AC5. `../10-15-gpui-clean-workbench/prd.md:5,21` makes C11 depend on C15 and gives C11 ownership of the AI review UI. Parent `implement.md:44–45` says integrated UI starts only after its CLI/core dependencies have accepted evidence.

**Effect:** The metadata DAG is acyclic, but acceptance is circular: C15 cannot finish without C11, while C11 cannot start before C15 finishes. C15's design also lists the GPUI AI review path as ownership, weakening the intended boundary.

**Minimal repair:** Make C15-AC5 require CLI/direct-service parity and reusable parity fixtures. Put actual CLI/GPUI AI parity in C11-AC1 and the parent C14 ledger. C15 hands off domain contracts and fixture identities; C11 alone implements the GPUI AI review. Do not solve this by weakening native acceptance or starting UI before the core contract is accepted.

### F2 — P1: C16 native-job acceptance conflicts with C09 ownership

**Evidence:** `../10-15-gpui-latest-dependencies/prd.md:26` assigns final GPUI pin/build acceptance to C09, but line 27 requires core/CLI/native toolchain jobs to run for C16 completion. C09 depends on completed C16 and C01. `research/dependency-policy.md:25–26` intends C16 to perform the initial candidate audit and C09 the first final native build.

**Effect:** If “native toolchain jobs” includes the new GPUI application, C16 requires a not-yet-created downstream deliverable. If it means existing Tauri jobs, the acceptance text does not say so.

**Minimal repair:** Scope C16 acceptance to selected shared dependency builds/tests, declared-MSRV jobs, retained legacy gates, and an auditable GPUI candidate/toolchain decision. New GPUI native build and platform acceptance remain explicitly NOT_RUN until C09. C09 updates the ledger with the final compatible pin. Avoid creating a reverse C16 -> C09 dependency or requiring C16 to claim a native PASS prematurely.

### F3 — P2: AI tool uninstall has an undeclared provider prerequisite

**Evidence:** `../10-15-gpui-ai-tool-cleanup/design.md:7` says tool uninstall needs accepted C04/C05 provider contracts, while its PRD line 5 and parent task map list only C01/C03. C15-R6 includes a Software-provider route. Existing providers do not imply support for every Claude Code/Codex/Cursor installation method.

**Effect:** C15 can be scheduled before its documented uninstall provider exists, or appear complete with an unreviewed path that deletes an AI tool's bin/config directories.

**Minimal repair:** Choose one explicit contract: (a) C15 produces a typed installed-provider identity and inspect/handoff result only, with executable integration accepted in downstream C11/C12 after C04/C05; or (b) add the actual minimal provider dependency to C15. Do not add C05 reflexively if its update/startup/leftover functionality is not needed. Any unsupported install method must stay an explained gap, not a cache rule or arbitrary package-manager command.

### F4 — P2: AI restore is an acceptance promise without a CLI operation contract

**Evidence:** C15-R3 (`../10-15-gpui-ai-tool-cleanup/prd.md:17`) mentions recovery inspection and restore planning; C15-AC3 at line 29 requires actual conditional restore/read-back. The additive CLI table in `research/cli-contract-proposal.md:15–24` has no AI family or recovery route. Its startup row already establishes a useful pattern: restore-plan uses the existing preview/execute path.

**Effect:** A child could implement only recovery listing/planning yet claim the recovery acceptance, or place the actual restore operation in GPUI and break CLI-first ownership.

**Minimal repair:** Add a proposed AI namespace and recovery flow with C15 parser ownership after C01. Explicitly state whether restore-plan is an AI plan intent executed through the same live preview/digest/confirmation path. Restore must revalidate destination, bundle integrity, provider version/index state and external changes; it is a mutation, not read-only recovery inspection. Parser names may remain draft until C15 freeze, but a real no-GUI restore path must be part of the approved requirement.

### F5 — P2: Sensitive AI recovery needs a concrete refusal and crash-consistency gate

**Evidence:** C15-R4 allows supported provider behavior or an atomically consistent recovery bundle (`../10-15-gpui-ai-tool-cleanup/prd.md:19`); AC1 lists locked stores and active/unknown sessions, but AC3 only states successful recovery/read-back and backup failure. Parent `design.md:52` forbids introducing permanent file deletion, and its AI section preserves inspect-only capability gaps.

**Effect:** “Recovery bundle” is not by itself a complete transaction contract across transcript files, provider indexes and Git registration. An archive existing on disk cannot prove the active provider is quiescent, the archive is durable, the moved data is complete, or a crashed operation is safe to retry. The inspected wording leaves the required outcomes of these failures implicit.

**Minimal repair before executable C15 rules:** Add a provider-specific acceptance matrix covering unknown active ownership -> inspect-only/refusal; inability to lock or detect index changes -> refusal; failure before recovery durability -> no mutation; crash/partial failure at each bundle/index/move/registration stage -> truthful recoverable or unknown outcome without automatic redispatch; restore after external edits -> refusal. Require original files and provider metadata to remain recoverable under the parent's allowed mechanism. A backup must not become an undocumented exception to the permanent-deletion exclusion. Standard Git worktree removal is not automatically approved by age or clean tracked status.

This is a bounded technical freeze requirement, not a reason to claim that safe session/worktree mutation has already been proven on Windows. Preserve required-vs-inspect-only capability rows so the parent can distinguish a supported workflow from a deferred gap.

### F6 — P2: Keep-awake domain ownership is conditional in CLI, mandatory in UI

**Evidence:** `../10-15-gpui-analyze-status-tray/prd.md:19` requires Keep Screen On sessions. The CLI proposal (`research/cli-contract-proposal.md:24`) adds keep-awake only if C13 implements the shared capability. C08's requirements do not own a keep-awake session contract. Parent R01 requires domain operations to have supported CLI behavior before the adapter ships.

**Effect:** A non-presentation OS power request can first appear inside a GPUI feature task, or the required UI capability can ship while CLI remains conditional. The screen-cover view itself can remain presentation-only; keep-awake has a shared OS lifecycle.

**Minimal repair:** Put the bounded lease/session token, owner identity, explicit confirmation, stop/release and CLI behavior into C08, then let C13 consume it. Alternatively define an explicit first C13 core/CLI milestone that must be accepted before its native adapter starts. Choose one owner and make the CLI proposal's conditionality match the feature requirement. Keep screen-cover escape and no-permanent-input-lock acceptance native and separate.

## Consistent Decisions Worth Preserving

- The latest-compatible policy supersedes the earlier recommendation to hold Rust 1.88 at all costs. Parent `prd.md:83` was observed updated during this review to require the declared MSRV to be tested and changes documented. C09/C16 do not claim that a new compiler was already selected or validated. Do not reopen this as a conflict with the superseded research recommendation.
- `research/dependency-policy.md` distinguishes latest stable registry packages from latest required upstream GPUI work, requires an immutable coherent revision, and keeps old Tauri baseline pins as explained exceptions. This reconciles freshness and reproducibility without promising that everything can use the same newest version.
- Parent `design.md:16–19` places shared Rust services beneath CLI and GPUI, without a CLI text-parsing transport, Tauri runtime, or web UI in the new application. Native OS APIs and installed providers remain allowed. The standalone Rust tray library considered earlier is not equivalent to embedding a Tauri application.
- Parent permanent-deletion/Cargo-home/Docker exclusions remain explicit. AI caches, sessions, worktrees, models, credentials, memory and tool uninstall are distinct data/authority classes. C15 explicitly requires Windows/version/root research before executable rules.
- `research/release-log-delta.md` separates Mac desktop, Mac CLI, Windows desktop and Windows CLI; it marks the X fetch as unverified and does not infer Windows provider APIs from Mac release notes. `research/gpui-windows-feasibility.md` likewise distinguishes published 0.2.2 from moving main despite their same version string.
- Native accessibility/IME/performance and installation evidence remain separate from fixtures/builds. At the reviewed snapshot Q1 and final-summary approval remained pending; the subsequent user answer confirms Windows x64 first and retained cross-platform CLI support. Only final-summary approval remains pending. No implementation is authorized by this review.

## Files Found and Patterns

| Files | Purpose / observed pattern |
| --- | --- |
| Parent `prd.md`, `design.md`, `implement.md` | CLI-first authority boundary, migration and dependency order. |
| C15 `prd.md`, `design.md`, `implement.md` | Sensitive AI-data classes and recovery; F1/F3/F4/F5 corrections are localized here. |
| C16 `prd.md`, `design.md`, `implement.md` | Fresh dependency selection and compiler policy; F2 narrows its completion boundary. |
| C09, C11, C13 PRDs/designs | Native feasibility and actual GUI/OS integration consumers. |
| C01–C08 PRDs, C14 PRD/design | Existing CLI, mutation/provider and final acceptance owners. |
| `research/cli-contract-proposal.md` | Additive grammar; restore-plan through preview/execute is a reusable pattern. |
| `research/dependency-policy.md` | Current authority for the revised MSRV/freshness decision. |
| `research/release-log-delta.md`, `research/performance-protocol.md` | Source/platform qualifications and evidence requirements. |

## Related Specs and External References

- Existing `AGENTS.md` safety exclusions and `.trellis/spec/backend/quality-guidelines.md` remain the executable contracts until implemented changes are reviewed; planning prose does not amend them.
- `.trellis/spec/backend/bounded-delivery.md` and `desktop-runtime.md` motivate bounded workers and separate native lifecycle evidence. The new toolkit does not remove those obligations.
- Existing primary-source URLs and versions are recorded in `research/gpui-windows-feasibility.md` and `research/release-log-delta.md`. This pass checks their stated scope/provenance distinctions; it does not perform a new release-version or external-source audit.

## Caveats / Not Found

- No mechanical task/link/manifest validation was duplicated; the coordinating session owns it. The first two findings concern acceptance dependencies that a DAG validator alone will not catch.
- A line reference can drift while the coordinator edits planning files. Requirement IDs identify the intended clauses.
- Detailed AI provider paths, transaction mechanisms, MSRV, exact GPUI pin and supported install methods remain technical decisions for their assigned children. None is reported as a completed capability here.
- No product code, specs, task metadata or other research files were changed by this review.

## Coordinator resolution — 2026-10-15

All six findings were repaired in the planning artifacts after the reviewed snapshot:

- F1: C15-AC5 is CLI/direct-service parity; C11-AC1 owns actual AI native parity.
  C15 no longer owns native UI files. The dependency direction stays C15 -> C11.
- F2: C16-AC3 covers existing core/CLI/legacy gates and candidate audit. C09 owns
  the first new GPUI native build and final pin; no reverse acceptance dependency.
- F3: C15-R6 and design emit typed install identity/handoff only; C04/C05 own actual
  uninstall providers and downstream integration must wait for those capabilities.
- F4: The CLI proposal defines `clean ai restore-plan` -> live `preview` -> explicitly
  confirmed `execute`; C15-R3 owns a real no-GUI restore path, with bundle/index and
  destination revalidation. C15 owns its parser additions after C01 conventions.
- F5: C15-R4/AC3 and design require refusal for unknown active ownership or changing
  indexes and failure injection at durability/index/move/registration stages. No
  retry or permanent-deletion exception is introduced by having a backup.
- F6: C08-R5/AC5 owns the finite keep-awake lease and CLI; C13 only consumes that
  accepted service. Screen cover remains presentation-only. Its native escape
  and lifecycle checks are unchanged.

These are document repairs, not implementation acceptance. Product criteria remain
NOT_RUN. Mechanical validation is recorded in the parent's planning-validation files.
