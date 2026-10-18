# C02 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Select the smallest process/helper arrangement that meets authenticated handoff and privilege isolation during this child's detailed design. Do not add a general privileged command runner. New saved plans contain intents and evidence, never executable command strings. Review Windows vendor documentation for each concrete operation before enabling it.

## Ownership

- `crates/devsweep-core/src/process/`
- `crates/devsweep-core/src/execution/`
- `proposed Rust Windows action helper module or minimal executable`
- `new versioned action/backup contract docs`

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

Authority: this PRD, parent `design.md` "Authority boundary", and `research/mole-014-rea.md` only as a catalogue description. Mole's elevation UX is not evidence. No production Rust module currently requests elevation. `optimize/windows.rs` launches three literal Settings URIs and does not elevate. Do not turn that into a privileged command runner.

### Closed operation set

The helper accepts one closed typed set. C02 does not enable a new host catalogue action. The set used by tests is fixture-only and runs against isolated registry keys and files:

- `fixture.backup`
- `fixture.restore`
- `fixture.identity_check`

Each request carries operation id, operation enum, live preview digest, caller user SID, tool identity hash, and the exact target identity captured at plan time. It does not carry a program path, argv, a shell string, or a free-form registry path supplied by the caller. The core reconstructs the fixture target from the saved plan's isolated root and the typed operation.

Later children may add a reviewed variant only by extending this enum and its tests. A string command is never a variant.

### Process and privilege

The normal app stays a standard user. There is no persistent service and no arbitrary command endpoint. A privileged fixture action starts one short-lived helper process bound to that operation id and caller. The helper exits when the operation reaches a terminal record. Tests inject the helper binary and the elevation response. They do not show a UAC dialog on this machine and do not change host policy.

After handoff, the helper re-reads the target identity and the live digest. Tool replacement, path replacement, digest mismatch, forged payload, replay of a spent operation id, stale preview, or a different user SID fails before any write.

### Audit, backup, and outcomes

A durable start record is written before dispatch. Cancellation before that write is cancellation. Disconnect, timeout, crash, or a missing terminal record after dispatch is unknown. None of these retry. UAC decline and a missing consent prompt are not success.

Registry and file mutation in the fixture requires a versioned backup of the exact bytes, then a read-back of the current identity. Restore runs only when that identity still matches the backup subject. A changed subject refuses restore. There is no generic undo.

No permanent delete, Cargo-home mutation, Docker cleanup, or shell command string. Do not run `clean execute` or `optimize run` on this machine.
