# C10 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

GPUI navigation is the centered five-mode capsule in the parent design, on a dark field, with the beaver menu and a light selected pill. Clean landing paints the licensed Earth still once. Do not port React/CSS/WebGL, the reference poem, or a recurring planet animation. Move neutral assets only with audited consumers; the existing Node icon generator is not a new-app runtime dependency.

## Ownership

- `desktop-gpui/src/app/ and components/ (proposed)`
- `desktop-gpui asset manifest (proposed)`
- `crates/devsweep-core/src/desktop_preferences.rs only if required`
- `new GPUI-specific .trellis/spec layer during implementation`

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

Extend `desktop-gpui/` only. Do not edit `desktop/` Tauri code. Do not add
`desktop-gpui` to the root workspace. Keep the Zed pin already in
`desktop-gpui/Cargo.toml`. The shell is one window. The capsule order is
Clean, Software, Optimize, Analyze, Status. Copy the existing DevSweep beaver
assets. Do not copy Mole poems, planets, or runner sprites. The licensed Earth
still is a Clean asset for C11, not a continuous animation here.

Preferences: read the existing store. Write only on an explicit save action.
Do not migrate on launch. Scan roots and the protection list stay separate
stores. A test uses a temp preference file, not this user's store.

Blocking domain work is not on the render thread. A canceled or replaced
operation id cannot apply a late result. Idle, minimized, and hidden states
do not schedule decorative frames.

C10-AC3 stays unchecked until a real UI Automation, text-scale, high-contrast,
reduced-motion, and Chinese-text sample is saved. A unit test of focus order
does not close AC3. Do not commit. Do not download Rust 1.88.0. `just ci`
does not build this package. The child gate is
`cargo test --manifest-path desktop-gpui/Cargo.toml --locked` and a locked
release build, plus `just ci` still exiting 0. Do not export `CARGO_TARGET_DIR`.
Do not set `HOME`, `CARGO_HOME`, or `RUSTUP_HOME`.
