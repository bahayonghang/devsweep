# C05 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Prefer a documented structured winget boundary when available; otherwise preserve the bounded parser and exact-match refusal rules. Treat source agreements/interactive installer ownership as explicit provider states. Generic scheduler/service editing is outside this child.

## Ownership

- `crates/devsweep-core/src/software/updates.rs`
- `crates/devsweep-core/src/software/startup.rs`
- `crates/devsweep-core/src/software/leftovers.rs`
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

## Technical freeze (2026-10-17)

Authority: this section, `prd.md`, and the C01 startup/leftover/update contracts
already in the tree. Do not run winget, startup changes, or leftover moves
against this machine. Tests use temp homes, an offline registry fixture, and a
command double. Program and argv stay separate. No shell string. No permanent
delete. No registry cleaner that matches a vendor name prefix.

### Updates

`software updates check` stays read-only and does not download. New mutation
leaves, if not already present, are `plan`, `preview`, and `execute` for one
explicit package id, source, and version. Ambiguous id, unsupported source,
injected network failure, and a version that changed since the plan are
refusals before a process starts. Restart-required is not recorded as updated.
The test double is not the installed winget. Argv may include
`--disable-interactivity`. Do not add `--accept-source-agreements`.

### Startup

Keep `StartupTogglePlanV1`. HKLM stays view-only. `already_in_state` does not
write. Add backup and restore on an offline fixture hive, not the host HKCU or
HKLM. Restore writes only when the live value still matches the backed-up
post-change identity. An external change refuses and leaves the fixture bytes
unchanged. Entries classified `security`, `network`, or `device_management`
are not toggled. The class comes from the fixture record, not from a host scan.

### Leftovers

Leftover execute runs only when the saved uninstall outcome for that software
id is `Removed`. `Failed`, cancel, `StillPresent`, `RebootRequired`, and
`UnknownAfterDispatch` leave every leftover path unchanged and create no
leftover plan execution. Personal-data rows start unselected. A shared folder
is protected. File mutation is move-to-trash through the existing trash runner.
Registry residue is one closed fixture key. Write the exact bytes to a backup
file before the change. Restore refuses if those bytes changed. Do not add a
second uninstaller.

Cargo tests use `--target-dir dist/ci-target`. Do not export `CARGO_TARGET_DIR`.
Do not set `HOME`, `CARGO_HOME`, or `RUSTUP_HOME`. Do not download Rust 1.88.0.
Do not commit. Check an acceptance box only when the saved log shows that case.
Desktop parity in C05-AC4 is the CLI and the shared service. GPUI parity stays
C12.
