# Implementation sequence and gates

Status: implementation approved on 2026-10-15; C16 is active. See implementation-approval.md.

## Task map

Parent/child links are metadata relationships; the dependency column is the
execution ordering. Each child has PRD, draft design, execution checklist and
curated implement/check manifests.

| Child | Deliverable | Depends on |
| --- | --- | --- |
| [C16](../10-15-gpui-latest-dependencies/prd.md) | Latest compatible dependencies and Rust toolchain | Parent planning approval |
| [C01](../10-15-gpui-cli-contracts/prd.md) | Shared services and CLI feature parity | C16 |
| [C02](../10-15-gpui-windows-action-boundary/prd.md) | Windows action authority and elevation boundary | C01 |
| [C03](../10-15-gpui-clean-coverage/prd.md) | Windows cleanup catalogue and CLI coverage | C01, C02 |
| [C15](../10-15-gpui-ai-tool-cleanup/prd.md) | AI tool cleanup and recovery through the CLI | C01, C03 |
| [C04](../10-15-gpui-software-uninstall/prd.md) | Native software inventory and uninstall providers | C01, C02 |
| [C05](../10-15-gpui-software-maintenance/prd.md) | Software updates startup and leftovers | C01, C02, C04 |
| [C06](../10-15-gpui-optimize-maintenance/prd.md) | Bounded Windows maintenance through the CLI | C01, C02 |
| [C07](../10-15-gpui-analyze-capabilities/prd.md) | Disk analysis explanations and safe actions | C01 |
| [C08](../10-15-gpui-status-capabilities/prd.md) | System status telemetry and process actions | C01, C02 |
| [C09](../10-15-gpui-native-feasibility/prd.md) | GPUI Windows feasibility and dependency pin | C16, C01 |
| [C10](../10-15-gpui-shell-assets/prd.md) | GPUI shell assets preferences and lifecycle | C01, C09 |
| [C11](../10-15-gpui-clean-workbench/prd.md) | Native cleanup workbench | C03, C10, C15 |
| [C12](../10-15-gpui-software-optimize-workbenches/prd.md) | Native software and optimization workbenches | C04, C05, C06, C10 |
| [C13](../10-15-gpui-analyze-status-tray/prd.md) | Native analysis status and tray surfaces | C07, C08, C10 |
| [C14](../10-15-gpui-acceptance-migration/prd.md) | Native acceptance performance and migration | C01, C02, C03, C04, C05, C06, C07, C08, C09, C10, C11, C12, C13, C15, C16 |

## Ordered milestones

1. Platform scope is confirmed and the PRD convergence pass is complete. Obtain
   approval of final-review.md and its linked feature matrix/performance budgets;
   record the approved revision before task.py start.
2. Start C16 only after approval to audit/adopt current shared dependencies and
   establish the toolchain policy, then C01. Make current desktop-only core features reachable
   through the CLI and prove legacy command compatibility before new UI work.
3. Complete C02 for new privileged/mutating capabilities. C07 can proceed after C01.
   Run C09 after C01 to retire toolkit risk early; it is a feasibility gate, not a
   reason to deliver decorative UI ahead of the functional work.
4. Deliver C03–C08 and C15 CLI/domain capabilities in dependency order. C04 precedes C05.
   Each concrete rule/provider/action catalogue is reviewed and frozen before its
   child starts. Concurrent agents must have disjoint ownership; overlapping
   software/service/CLI registry edits are serialized by the coordinator.
5. Complete C10 after C09. C11, C12 and C13 cannot start their integrated workflows
   until their listed CLI/core children have accepted contracts and required evidence.
6. C14 owns final equivalent native measurement, package, compatibility, rollback,
   evidence-ledger review and a separately proposed default switch/retirement.

## Per-child execution

- Refresh current status and child context; preserve unrelated local work.
- Resolve the child's finite technical unknowns and update the design before code.
- Populate exact source ownership for the implementation sub-agent. Shared files
  get one owner at a time; workers never revert other edits.
- After explicit planning approval, run task.py start for the selected child.
- Dispatch Trellis implement/check agents with the active child path and curated
  manifests. Main session coordinates architecture/spec/acceptance decisions.
- Add meaningful positive and refusal/identity-drift tests before widening authority.
- Run targeted checks, then just ci and any affected native/legacy gates.
- Record source/binary hashes and exact PASS/FAIL/NOT_RUN/UNVERIFIED evidence.
- Update specs only for implemented, reviewed contracts; proposed task docs do not
  redefine the current executable contract.

## Validation

Planning: task tree is acyclic, reciprocal parent links exist, dependency IDs resolve,
all 17 tasks remain planning, no placeholder PRDs or empty context manifests, all
curated paths exist, and task.py validate succeeds. Product source must be unchanged.

Implementation: root `just ci`; targeted core/CLI tests listed per child; explicitly
pinned GPUI format/check/test/clippy/build commands added by C09. The separate
GPUI native gate remains mandatory even if the root workspace does not include it.
Shared changes also retain supported Tauri wire/build checks using its required
Node/npm versions. Native matrix is in research/performance-protocol.md.

## Rollback points and completion

Keep the initial commit as context, not immutable evidence of future builds.
Retain an exact current Tauri baseline before candidate changes. Keep existing
persisted formats readable and old assets available until consumer migration passes.
Never mark implementation complete from a build, fixture-only tests or task archive
state. Do not commit, archive, install, sign, publish or retire Tauri without the
separate user authorization appropriate to that action.

Current action: implement/check C16, then advance to C01 only when its required
dependency evidence is accepted. The final summary is approved; do not recreate
this task tree or repeat platform/per-child approval within the agreed scope.
