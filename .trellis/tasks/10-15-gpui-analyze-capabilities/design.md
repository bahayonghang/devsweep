# C07 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Extend existing AnalyzeSnapshot and actions rather than introducing a second filesystem walker. Version additional explanations and capability fields. Persisted snapshot replay never grants recycle authority on its own.

## Ownership

- `crates/devsweep-core/src/analysis/`
- `crates/devsweep-core/src/filesystem/`
- `crates/devsweep-cli/src/application/commands/analyze.rs`

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

C01 already exposes `analyze reveal`, `trash-plan`, `trash-preview`, and
`trash-execute`. This child adds read-only `analyze roots`, `analyze list`,
and `analyze explain`. Do not change the Clean V1 digest. Scan and explain
create no cleanup action. Reveal stays `explorer.exe` with argv `/select,`
and the path, through the existing process runner.

`analyze roots` lists volumes the existing snapshot source can see. A missing
root is an error for that root, not a scan of the profile. `analyze list`
returns bounded children of one snapshot node id. `analyze explain` returns
one closed class: `hibernation`, `page_file`, `wsl_disk`, `docker_disk`,
`ai_model`, `backup`, `game`, `cloud_placeholder`, or `unknown`. Size never
selects a class and never makes a class disposable. Unknown is the fallback.
A class is not a cleanup plan.

Recycle preview and execute keep the C01 refusals, including a live identity
mismatch. Protected, system, and shared locations stay read-only. File-lock
evidence is a report. It does not terminate a process. Tests use temp trees.
Do not analyze or trash this machine's user profile. Do not edit `desktop/`.
Do not commit. Cargo uses `--target-dir dist/ci-target`. Do not export
`CARGO_TARGET_DIR`. Do not set `HOME`, `CARGO_HOME`, or `RUSTUP_HOME`. Do not
download Rust 1.88.0. Check a box only when the saved log shows that case.
GPUI parity stays C13.
