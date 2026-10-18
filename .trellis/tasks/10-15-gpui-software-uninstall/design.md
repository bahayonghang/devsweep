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

## Technical freeze (2026-10-17)

Authority: this section and `prd.md`. Current-user MSIX removal stays the
existing adapter and the existing `software.jsonl` transitions. Do not add a
field to that journal. Audit lines still must not contain `UninstallString`,
`QuietUninstallString`, `cmd.exe`, `powershell`, `argv`, or `program`. Do not
route uninstall through `windows-action-helper`. Do not uninstall software
that is already installed on this machine. Do not delete an installation
directory as a substitute for uninstall. Leftovers stay C05. This child never
calls leftover scan or leftover execute.

### Inventory

Keep ARP, MSI, and current-user MSIX as separate source identities. The same
display name is not a merge. Two source rows merge only when the source-local
identity is equal. Each row may carry publisher, version, installation time,
an icon reference, and a qualified size. Missing size stays unavailable. It
is not zero. A failed source does not erase a sibling source.

### Executable providers

All three new providers run only against a temp fixture. The test supplies
the adapter. Production code may call the real tool only when the saved plan
names a fixture-resolved program that the live requery still matches. This
session does not install an MSI, a vendor app, or a Scoop app on the host.
Isolated `process_fixture` copies exercise msiexec, vendor, and Scoop argv.
Host-installed products were not used. Real current-user MSIX removal remains
NOT_RUN.

| Provider | Program and argv | Removed only when |
| --- | --- | --- |
| current-user MSIX | existing adapter, unchanged | existing requery says the package is absent |
| MSI | `msiexec.exe` and separate argv `/x`, `{PRODUCT-CODE}`, `/qn`, `/norestart` | fixture product code is absent on requery |
| vendor | absolute `program` inside the fixture plus a closed argv list | fixture requery says the app id is absent |
| Scoop | `scoop.exe` inside the fixture and separate argv `uninstall`, `<app>` | fixture requery says the app id is absent |

`msiexec.exe` is the literal file name. Do not prefix `cmd.exe`. A registry
uninstall string is not parsed by a shell. If it is not one of the rows
above, eligibility stays `registry_only_manual` or `msi_execution_not_supported_v1`
and no process starts. Vendor argv that contains `cmd`, `/c`, `powershell`,
`-command`, or `-enc` is manual and does not start. A missing tool does not
start and does not change the fixture.

Exit code 0, or a process that only opens a window, is not `Removed`.
`RebootRequired` is not `Removed`. `Failed`, `CanceledBeforeStart`,
`StillPresent`, and `UnknownAfterDispatch` do not create a leftover plan.
No automatic retry. Durable audit start stays the existing transition written
before dispatch. A crash after dispatch stays `UnknownAfterDispatch`.

### Tests

Extend the existing software execution tests. Do not add a live host install.
Cover identity collision, a shared vendor folder that is not deleted, a
malformed command that does not start, a missing tool, command drift between
plan and launch, and a reboot outcome that is not `Removed`. One test still
proves the current-user MSIX fixture path. Check an acceptance box only when
the saved log shows that case.

Cargo tests use `--target-dir dist/ci-target`. Do not export `CARGO_TARGET_DIR`.
Do not set `HOME`, `CARGO_HOME`, or `RUSTUP_HOME`. Do not download Rust 1.88.0.
Do not commit.
