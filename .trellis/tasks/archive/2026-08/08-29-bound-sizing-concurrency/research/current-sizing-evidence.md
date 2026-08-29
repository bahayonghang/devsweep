# Current Sizing Evidence

## Verified hot path

- `crates/devsweep-core/src/filesystem/sizing.rs:224-236` is the only production
  Rayon fan-out in the sizing module. Root children use `par_iter()` on the
  global pool; nested walks remain sequential.
- The branch creates a fresh local budget per child at lines 229-231. The
  sequential nested branch at lines 238-255 shares `remaining` and cannot be
  reused as an equivalent root fallback without changing behavior.
- `serial_estimate_tree` at lines 637-639 calls `estimate_tree` and is currently
  not serial.

## Verified callers

- Project discovery: `crates/devsweep-core/src/scan/project/mod.rs:634-639`.
- Targeted rescan: `crates/devsweep-core/src/scan/project/mod.rs:726-727`.
- Global provider sizing: `crates/devsweep-core/src/scan/global/probe.rs:130-135`.
- Inventory: `crates/devsweep-core/src/inventory/mod.rs:167-172`.

All converge on the shared sizing boundary, so no CLI/TUI/Tauri contract change
is needed.

## Test gaps resolved by this plan

- Current pre-cancel regression: `sizing.rs:383-410`.
- Missing before this task: deterministic mid-walk cancellation, explicit
  low-entry-budget coverage, explicit low-max-depth coverage, a real serial
  reference, and observable peak-worker proof.

The implementation design uses the existing injectable `PathReparseProbe`
boundary for deterministic mid-walk coordination and a private execution-mode
seam for forced serial fallback.

