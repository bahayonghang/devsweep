# C15 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Use provider documentation/source and current supported Windows layouts, never translate macOS paths mechanically. No broad ~/.claude or ~/.codex deletion. C03 owns ordinary cache registry integration; C15 owns sensitive AI-data classification and opt-in recovery. Keep session/worktree operations distinct from Clean cache rules. This child only emits typed installed-tool handoff identity; actual tool uninstall is C04/C05 work and native integration waits for C11/C12.

## Ownership

- `crates/devsweep-core/src/rules/ and scan/ AI adapters`
- `new narrowly scoped AI session/worktree recovery modules`
- `crates/devsweep-cli/src/application/commands/clean.rs and child modules`
- AI CLI parser additions after C01; C11 exclusively owns the native AI review UI.

These paths identify responsibility, not permission to refactor every file.
One owner edits a shared file at a time. Workers are not alone in this repository
and must not revert other workers' changes. Main session owns cross-child design,
spec updates and integration decisions.

## Data flow and compatibility

CLI request -> domain observation/plan -> live validation -> explicitly confirmed execution where supported -> durable audit/read-back -> typed output. Discovery never mutates system state.

Freeze a provider-specific transaction matrix before enabling sensitive operations:
unknown active ownership or unobservable index changes refuse execution; failure
before durable recovery preserves the original state; later failures retain original
files and provider metadata through the permitted recovery mechanism and report
partial/unknown outcomes without retry. Restore uses a saved restore intent through
the same preview/execute authority and refuses destination/index drift. A backup is
not an exception to the parent's permanent-deletion exclusion. Unsupported session
or worktree mechanisms remain explicitly inspect-only.

Preserve existing supported schemas, command semantics, refusal and cancellation.
Version incompatible shapes and consult the parent CLI proposal. Rust native views
call shared services, rather than parsing CLI text or cloning domain logic.
Follow the latest-compatible dependency policy; any older required pin or compiler
compatibility change is documented and tested.

## Risks and rollback

Identity drift, partial observations, late worker results and broadening authority
are relevant failure cases. GPUI-specific native accessibility/IME/graphics are
unproven until C09. Native uninstaller/helper/provider behavior cannot be inferred
from fixture success alone. Keep operation-specific close/cancel semantics.

Retain existing CLI/Tauri paths while this child is developed. Roll back isolated
adapter changes without changing unrelated work or historical evidence. Installation,
publishing and Tauri retirement are not implicit consequences of passing tests.

## Technical freeze before implementation

Refresh source evidence, freeze concrete provider/rule/action or native API choices,
and record exact schemas and refusal states needed for this child's requirements.
Do not turn an unresolved technical candidate into executable authority.
A material change to product scope, safety, platform or acceptance returns to review.
