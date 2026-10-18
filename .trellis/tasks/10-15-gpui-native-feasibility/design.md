# C09 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Run after the first CLI parity milestone to retire framework risk early; do not start feature UI before its CLI acceptance. Use C16 latest-dependency policy and select a shared or isolated native workspace from actual compatibility evidence, not a hard-coded old MSRV constraint.

## Ownership

- `proposed desktop-gpui/Cargo.toml`
- `proposed desktop-gpui/src/`
- `Cargo.lock only as required`
- `native feasibility evidence and future GPUI spec`

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

Authority: this PRD, `research/gpui-windows-feasibility.md`, and C16 `research/gpui-latest-checkpoint.md`. Those notes are not a successful native build.

### Pin

Do not use floating `main`. Do not mix crates.io `gpui` 0.2.2 with upstream `gpui_platform` or `gpui_windows`. The published 0.2.2 archive is the registry alternative, not the Windows accessibility candidate.

Start from immutable upstream revision `3e145061d861ed32bb6a08dad7b03f2b5e455ad8`, with `gpui`, `gpui_platform`, and `gpui_windows` from that same tree. Before writing the manifest, refresh the upstream default-branch SHA once. If the refresh returns a newer full commit SHA, pin that SHA instead and record both SHAs in this task directory. If the refresh fails, keep `3e145061d861ed32bb6a08dad7b03f2b5e455ad8` and record the refresh as NOT_RUN. The manifest dependency is `rev = "<40-hex>"` plus a locked checksum. No branch name and no `*` version.

### Package location

Create a standalone package at `desktop-gpui/` with its own `Cargo.lock`. Do not add it to the root workspace. Do not edit the root `Cargo.toml`, `justfile`, `.github/workflows/ci.yml`, or `desktop/`. The Tauri app stays the rollback. The GPUI process must not link a Tauri, WebView, or Node runtime.

### Spike behavior

One native window. Packaged offline text and one existing DevSweep icon copied into `desktop-gpui/` without modifying the source asset. One focusable text control and one keyboard-reachable action. No Mole assets. No five-mode product shell; that belongs to C10.

### Evidence

C09-AC2 and C09-AC4 stay unchecked unless a real Windows run records UIA labels/roles/focus, Chinese IME composition, keyboard-only operation, a screenshot, the release binary hash, and PID/creation identity plus shutdown. A unit test or a failed launcher is NOT_RUN, not PASS. `just ci` still has to exit 0 for the existing workspace. Local Rust 1.88.0 is not installed and must not be downloaded. Do not claim that the declared MSRV ran.
