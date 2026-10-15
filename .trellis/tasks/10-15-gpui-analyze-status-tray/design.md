# C13 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Keep the treemap projection pure and bounded. Retain CLI lossless stream semantics while native view projections may refresh from the most recent already-delivered sample. Keep-screen controls consume C08's accepted shared service and CLI lease contract; screen-cover UI is desktop presentation.

## Ownership

- `desktop-gpui/src/modes/analyze/ (proposed)`
- `desktop-gpui/src/modes/status/ (proposed)`
- `desktop-gpui/src/tray/ (proposed)`
- Native session controls over C08 contracts; no duplicate OS request service.

These paths identify responsibility, not permission to refactor every file.
One owner edits a shared file at a time. Workers are not alone in this repository
and must not revert other workers' changes. Main session owns cross-child design,
spec updates and integration decisions.

## Data flow and compatibility

Native input -> typed view action -> shared domain service -> bounded worker results -> accepted report/projection -> native rendering. UI owns no cleanup authority.

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
