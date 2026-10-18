# C11 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Use native virtualized rows and cached view projections over immutable accepted reports. Do not rebuild the entire report or serialize through the CLI on every interaction.

## Ownership

- `desktop-gpui/src/modes/clean/ (proposed)`
- `desktop-gpui shared table/review controls as needed`

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

## Technical freeze (2026-10-17)

GPUI calls the shared Rust scan, plan, preview, and execute services. It does
not parse CLI text and it does not start a scan from the landing render.
States, in order, are Landing, Scanning, Found, AllClean, Cleaning, Completed,
and Failed. Landing shows the licensed Earth still once and one button. That
button asks for global cache or project artifacts before any scan. First
launch does not scan the machine.

Selection, live preview digest, and explicit confirm stay the Clean contract.
A stale digest does not move files. Partial and inspect-only rows stay visible.
AI review is a Clean subview over the C15 classes. It does not add a trash
rule to generic clean scan. Tests compare GPUI state or the service digest to
the CLI digest on a temp fixture. C11-AC4 stays unchecked until a real keyboard
and UIA pass is saved. Do not edit `desktop/`. Do not commit. Do not run
`clean execute` on this machine.
