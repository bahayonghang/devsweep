# CLI-first GPUI desktop rewrite

Status: **in progress — final implementation plan approved on 2026-10-15**.
Amended 2026-10-16 from a static REA inspection of installed Mole 0.14.0.
The amendment freezes Clean/Software/Optimize/Analyze/Status screen states and
the optimize CLI catalogue. Evidence is
[mole-014-rea.md](research/mole-014-rea.md). Approval of the 2026-10-15 summary
still stands for the unchanged safety, platform, and migration decisions.
Created at the user's explicit request on 2026-10-15. Baseline checkout: `dev`,
`25437d102ab7355198b3ef2647c2cbced740b3b2`; initially clean, no active tasks.
The older Tauri tasks are archived; this task does not reopen or relabel them.

## Goal and user value

Deliver a responsive Rust/GPUI desktop for DevSweep's five maintenance domains,
with a complete, scriptable CLI underneath. Users must be able to understand
what is consuming resources, review a concrete action, and obtain a truthful
result. Reuse proven core logic and existing DevSweep assets.

The request fixes the implementation language/framework, CLI priority, reference
product and task-tree deliverable. It does not establish that the old slowdown is
caused solely by WebView; backend discovery and rendering are measured separately.

## Confirmed facts

- Current Rust core has Clean, Software, Optimize, Analyze and Status. CLI exposes
  all five plus History, but several existing core/Tauri features lack CLI routes.
- CLI Software execution is current-user MSIX only; winget updates are check-only.
  Startup and leftover primitives exist. Optimize mixes DNS execution, Settings
  handoff and guidance. Analyze has native reveal/recycle APIs absent from CLI.
- Current Clean separates global and project scans. Original beaver branding,
  local licensed Earth assets, preferences, history and safety logic are reusable.
- User screenshots establish five-mode navigation, Software's three subviews,
  analysis root choice, and a status dashboard/process list. They do not prove
  undisplayed execution behavior or authorize running actions on this computer.
- Official reference findings and current code anchors are in
  [feature matrix](research/mole-feature-matrix.md) and
  [repository audit](research/repository-audit.md).

## Requirements

| ID | Required outcome | Implementation owners |
| --- | --- | --- |
| R01 | CLI first: all domain operations needed by GPUI have supported CLI routes, stable human/JSON/NDJSON contracts and shared Rust behavior before the UI adapter ships. Pure presentation actions need no artificial CLI command. | C01, C03–C08, C15 |
| R02 | One authoritative Rust core: renderers never invent file/package/process identity, reconstruct cleanup commands, or bypass live validation. | C01, C02, C10 |
| R03 | Clean retains developer-cache capabilities and independent global/project flows, adds a finite evidence-backed Windows catalogue, explains recoverability/cost and protects personal data. | C03, C11 |
| R04 | Software supports qualified inventory, supported native uninstall providers, separate residual-data review, explicit winget updates and reversible startup controls. | C04, C05, C12 |
| R05 | Optimize is a closed CLI catalogue: existing eight rows stay valid, and the 2026-10-16 amendment adds only the preflighted execute, read-only, and guidance rows in C06. One plan is one operation. Image scan does not start image repair. | C02, C06, C12 |
| R06 | Analyze provides bounded disk/folder inspection, treemap/list drill-down, storage explanations, lock-owner evidence and separate reviewed recycle actions. | C07, C13 |
| R07 | Status provides honest live telemetry, explainable health/process details and carefully separated process actions; tray sessions follow native lifecycle rules. | C08, C13 |
| R08 | New desktop uses GPUI and Rust application/domain logic with no React, WebView, Node or Tauri runtime. OS APIs and explicitly supported installed providers remain valid dependencies. | C09, C10 |
| R09 | Reuse original DevSweep identity and licensed assets. GPUI navigation is the centered five-mode capsule from the Clean screenshot. Clean landing shows the licensed Earth still, one original line, and one primary action that chooses global or project scope before scanning. A continuous planet animation is not required. | C10–C13 |
| R10 | Preserve EN/zh-CN, font/scale/theme settings, keyboard use, screen-reader semantics, high contrast/reduced motion and report retention across navigation. | C09–C13 |
| R11 | Prove performance and cancellation/exit/resource behavior on native release builds, with equal scopes and complete reports. | C09, C14 |
| R12 | Keep a working rollback during migration and an explicit later decision on making GPUI default/retiring Tauri. | C10, C14 |
| R13 | Add explicit Claude Code/Codex/Cursor AI cleanup with separately reviewed caches, old versions, sessions, worktrees and tool removal; use current release logs as well as screenshots. | C15,C03,C11 |
| R14 | Prefer latest compatible stable libraries across shared code and GPUI; evaluate latest upstream GPUI when needed, pin reproducibly and document toolchain/older-version exceptions. | C16,C09,C14 |

### Scope of feature parity

The user confirmed on 2026-10-15: **Windows x64 first, preserving existing
cross-platform CLI support**. Native macOS/Linux/Windows ARM64 desktop delivery
belongs to later milestones. Shared changes must retain supported CLI behavior
and CI coverage on Windows, Linux and macOS. New Windows-only operations report
explicit capability gaps elsewhere; they never break existing CLI functionality.

The five primary workflows and their meaningful Windows operations are the parity
target. Mole-specific commercial licensing, pricing, branding and exact application
catalogue counts are not requirements. A supported/guidance/deferred/refused matrix
must make differences visible; disabled buttons cannot stand in for completed work.

New actions must satisfy existing repository contracts. Permanent file deletion,
Cargo-home cleanup and Docker cleanup remain excluded. System-managed cleanup that
cannot satisfy that boundary stays inspect/Settings guidance with a documented gap.
Large AI models, conversations, browser profiles, mail/chat databases, virtual disks,
passwords and credentials never become automatically selected caches.

## Acceptance criteria

| ID | Observable acceptance | Covers |
| --- | --- | --- |
| P-AC01 | Every approved feature-matrix row has a concrete owner, CLI/core proof, GPUI proof where applicable, and final PASS or explicit gap. A required unresolved row prevents completion. | R01–R07 |
| P-AC02 | Identical isolated inputs yield equivalent CLI/GPUI reports, identities, selections, live preview digests, refusal codes and outcome semantics. Golden legacy contracts still pass. | R01, R02 |
| P-AC03 | No mutation occurs before its required saved plan/live preview/confirmation. Stale identity/protection/tool state refuses execution. Audit, partial/unknown and read-back are preserved. | R02–R07 |
| P-AC04 | All five native workflows cover loading/empty/partial/error/cancel/review/result states; navigation and Settings retain relevant state. Large-list/treemap operations are usable by keyboard and UIA. | R03–R10 |
| P-AC05 | Packaged GPUI works offline from an unrelated working directory and uses original licensed assets with correct native window/taskbar/tray icons; no WebView/Node runtime. | R08, R09 |
| P-AC06 | EN/zh-CN, Chinese IME, font sizes, observed Windows DPI, theme/high contrast, reduced motion, accessibility and graphics capability are recorded against the native candidate. Missing platform evidence stays NOT_RUN. | R10 |
| P-AC07 | The proposed performance budgets in research/performance-protocol.md are frozen before candidate work and pass on equivalent current baseline/candidate workloads. No work or findings are removed to improve time. | R11 |
| P-AC08 | Read-only cancellation joins cleanly, dispatched mutations use operation-specific wait/cancel policy, stale events cannot revive state, and main/tray/decoration workers settle on close. | R02, R11 |
| P-AC09 | just ci, existing Windows/Linux/macOS CLI compatibility gates, pinned GPUI checks and Windows x64 native package/migration checks pass. Declared core/CLI MSRV is tested; any change is documented. Old and new preferences survive explicit-save migration and rollback tests. | R01, R08, R12 |
| P-AC10 | Final review records evidence identity and every FAIL/NOT_RUN/UNVERIFIED; promotion/retirement remains separately reviewable and historic failures are preserved. | R11, R12 |
| P-AC11 | AI provider fixtures prove active/personal-data protection, explicit session/worktree recovery, bounded inventory and CLI/GPUI parity; unknown provider versions remain refused. | R13 |
| P-AC12 | Dependency ledger records latest/selected versions or immutable revisions, reasons for exceptions and tested toolchain compatibility. No stale-version claim or silent MSRV change. | R14 |

## Out of scope

- Rewriting proven scanners/executors solely because the desktop toolkit changes.
- Copying Mole product assets, closed-source internals, commercial flows or slogans.
- Arbitrary shell scripts, registry cleaning, debloating, disabling security/services,
  kernel sensor drivers, automatic process termination or persistent admin service.
- Automatic deletion of models/sessions/offline content, permanent file deletion,
  Cargo-home mutation, Docker cleanup or live user-machine cleanup as validation.
- A blanket promise of hundreds of app rules or complete telemetry on every GPU.
- Native macOS/Linux/Windows ARM64 desktop delivery in the first release; existing CLI support on those applicable platforms is retained.
- Removing Tauri, publishing, signing, installing, committing or archiving in this planning turn.

## Technical risks and implementation gates

Technical risks: GPUI dependency churn, Windows accessibility/IME/graphics evidence,
deliberate toolchain/MSRV migration, native uninstall identity, elevation, hardware sensor
availability and backend latency. C09 explicitly resolves framework feasibility;
C02 resolves privileged action mechanics. Child designs must freeze concrete provider
and catalogue contracts before implementation; any material scope change returns to review.

The product-scope decision is resolved and the PRD convergence pass is complete.
No blocking product question remains. The user approved [the final planning summary](final-review.md)
and linked performance budgets; see [the approval record](implementation-approval.md).
No product or native acceptance is claimed by approval of these documents.
