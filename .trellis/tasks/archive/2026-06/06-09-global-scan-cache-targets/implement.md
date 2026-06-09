# global scan cache targets

## Plan

1. Add a TUI bootstrap hook that emits one initial scan effect on startup.
2. Keep the existing scan worker and `ScanFinished` state replacement path.
3. Add a focused test for the startup scan request.
4. Run the repo validation gate.

## Files

- `src/tui.rs`

## Validation

- `cargo fmt --all -- --check`
- `cargo test --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- `just ci`

## Rollback

- If the startup scan creates noisy duplicate jobs, revert only the bootstrap
  hook and keep the existing manual `s` path intact.
- If the initial scan changes selection or filtering semantics, back out the
  startup hook and re-evaluate state initialization.
