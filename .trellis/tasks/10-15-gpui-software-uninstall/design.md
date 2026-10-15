# C04 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Do not promise arbitrary Win32 command support. Decode recognized uninstall formats into exact executable plus argv under a finite provider policy; unknown formats remain manual handoff. Scoop is an explicit external-provider exception to Rust implementation, without embedding PowerShell business logic or interpolated scripts.

## Ownership

- `crates/devsweep-core/src/software/model.rs`
- `crates/devsweep-core/src/software/plan.rs`
- `crates/devsweep-core/src/software/execution/`
- `crates/devsweep-core/src/software/audit.rs`
- `crates/devsweep-cli/src/application/commands/software/`

These paths identify responsibility, not permission to refactor every file.
One owner edits a shared file at a time. Workers are not alone in this repository
and must not revert other workers' changes. Main session owns cross-child design,
spec updates and integration decisions.

## Data flow and compatibility

CLI request -> domain observation/plan -> live validation -> explicitly confirmed execution where supported -> durable audit/read-back -> typed output. Discovery never mutates system state.

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
