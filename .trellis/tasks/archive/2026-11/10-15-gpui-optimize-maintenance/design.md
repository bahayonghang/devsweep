# C06 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Preserve operation-specific native executors. Never turn optimize into a generic
shell list. Catalogue v2 is frozen in the PRD from the Mole 0.14.0 REA note.
`defrag.exe /L`, `w32tm.exe /resync`, `w32tm.exe` stripchart, and the two
`dism.exe /Online /Cleanup-Image` forms are the execute argv. The observed
Mole host calls the DISM API and `DnsFlushResolverCache`; DevSweep keeps the
existing DNS executor and uses the public `dism.exe` argv so the CLI remains
the supported surface. Dead-startup removal goes through the C05 startup core
and writes the `.reg` backup first.

A required operation that cannot meet the privilege/rollback model remains an
explicit guidance row. Update cache, icon cache, search restart, winget repair,
and registry cleaning do not gain executors in this child. A Settings link is
not claimed as functional parity with actual repair.

## Ownership

- `crates/devsweep-core/src/optimize/`
- `crates/devsweep-cli/src/application/commands/optimize.rs`

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

The program, argv, class, and precheck table in `prd.md` is the catalogue.
Do not add ids. Do not change the eight v1 ids. `dns.flush` still previews.
One plan is one operation id. Image scan does not start image repair. Clock
probe does not start clock sync. Guidance and observe rows have no executor.
`optimize plan` on `maintenance.disk_health` and on a guidance id does not
launch a program. `optimize health` is read-only. Cooldown stays in the
existing optimize audit. Do not add `optimize.json`.

Program and argv stay separate. No shell string. No registry cleaner. Do not
run `defrag.exe`, `dism.exe`, `w32tm.exe`, or a shell kill on this machine.
Tests use a command double and injected prechecks: UAC decline, not SSD, low
battery, reboot pending, cooldown, missing program. A real UAC prompt is
NOT_RUN. C06-AC4 stays unchecked because this session does not run those
programs on a disposable Windows install. Fake-runner evidence does not close
AC4.

Do not edit `desktop/`. Do not commit. Cargo uses `--target-dir dist/ci-target`.
Do not export `CARGO_TARGET_DIR`. Do not set `HOME`, `CARGO_HOME`, or
`RUSTUP_HOME`. Do not download Rust 1.88.0.
