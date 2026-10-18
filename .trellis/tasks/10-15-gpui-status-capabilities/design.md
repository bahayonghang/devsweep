# C08 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Keep sampling read-only; process actions live outside the sampler. GPU vendors may expose different capabilities. No kernel driver, admin telemetry service or download of hardware utilities is assumed.

Keep-awake is a separate bounded OS request service, accepted through the CLI before
C13 adds controls. Freeze finite lease limits, explicit confirmation, owner identity,
status/stop token semantics, request-handle cleanup on exit and capability reporting.
It changes no persistent power plan and shares no ownership with unrelated requests.

## Ownership

- `crates/devsweep-core/src/status/`
- `crates/devsweep-cli/src/application/commands/status.rs`
- `new narrowly scoped process-action contract`
- `new bounded keep-awake service and CLI session contract`

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

## Technical freeze (2026-10-17)

Keep the existing status snapshot and live NDJSON contracts. A missing GPU,
VRAM, disk-activity, thermal, battery, or fan reading is `unavailable` with
a reason code. It is not zero. Do not install a driver. Do not add an always-on
sampler. Health text names the evidence. It is not a single opaque score.

Process termination is a separate command from status display. The plan binds
PID and creation time. A mismatch refuses before any kill. The only native
target in tests is a process the test spawned. Critical, shared, and system
process names in a closed fixture list refuse. Do not terminate a process
the test did not create. Confirm is required. Data-loss text is part of the
preview. No implicit elevation.

`status keep-awake run`, `status`, and `stop` use an in-memory lease owned
by a session token. The lease has an explicit finite duration. Stop, expiry,
and owner-exit release only that token. The implementation must not call a
persistent power-setting API. Tests use a short lease and a fake clock or a
sub-second duration. Unsupported platforms return a capability gap and do
not change power settings. C13 owns the native control. This child is CLI
and the shared service.

Do not edit `desktop/`. Do not commit. Cargo uses `--target-dir dist/ci-target`.
Do not export `CARGO_TARGET_DIR`. Do not set `HOME`, `CARGO_HOME`, or
`RUSTUP_HOME`. Do not download Rust 1.88.0. Check a box only when the saved
log shows that case.
