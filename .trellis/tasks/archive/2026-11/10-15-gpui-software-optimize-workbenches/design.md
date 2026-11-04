# C12 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Reuse the shell coordinator and review controls; keep native installer UI an external provider state. Rendering must never parse uninstall strings, invent package identity or reinterpret a guidance action as execution.

## Ownership

- `desktop-gpui/src/modes/software/ (proposed)`
- `desktop-gpui/src/modes/optimize/ (proposed)`

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

GPUI state only, on top of the C10 shell and the existing software and optimize
services. Software segments are Uninstall, Updates, and Startup. Leftovers are
a later review, not a fourth segment. Optimize states are Landing, Running,
Completed, and Failed. Optimize groups are startup, network and search, and
maintenance. One confirmed plan is one operation id. Image scan does not start
image repair. A guidance or observe row has no run button that launches a
program. A route change does not cancel a dispatched mutation. Do not uninstall
host software and do not run defrag, dism, or w32tm. C12-AC4 stays unchecked
without a saved screenshot and UIA sample. Do not edit `desktop/`. Do not commit.
