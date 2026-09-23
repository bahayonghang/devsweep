# Implementation plan

## Preconditions

1. Read the backend and desktop specs plus the parent design and current-state
   research; verify the current `ref/Mole` hash is still research-only.
2. Record the current CLI/core/Tauri ownership matrix and existing contract
   fixture locations before editing, including the explicit settings/lifecycle
   exceptions and result-only versus streamed operation map in design.md.
3. Keep the task in planning until the parent plan is explicitly approved.

## Ordered work

1. Identify the existing `devsweep-core` service entry points used by CLI and
   Tauri; extract only a missing typed seam needed for parity.
2. Align Tauri adapters and CLI calls with that seam without importing the CLI
   crate or adding a child process.
3. Align request/event/result/error fixtures for stale IDs, stale digests,
   cancellation, partial/unavailable values, and audit data. Distinguish
   cancel-request ACK from joined terminal completion and cover wait-only Clean
   actions; do not retrofit an artificial universal cancel timeout.
4. Add focused contract tests at the trusted service/adapter boundary and
   regenerate/review desktop types only if the wire contract changed. Final
   verification uses the existing read-only `types:generate -- --check`.
5. Run the parent review gate and hand the accepted seam to the performance
   child.

## Validation

- `cargo test --workspace --locked`
- `mise exec node@22 -- npm --prefix desktop run types:generate -- --check`
- `mise exec node@22 -- npm --prefix desktop run typecheck`
- Focused desktop bridge/Tauri contract tests, then `just ci`.
- Search production Tauri for CLI imports/launches, shell strings and authority
  bypasses; search React imports against the three declared typed adapters.
  Test-only process fixtures and DEV lifecycle injection are not production
  authority and must not be counted as a forbidden CLI subprocess.

## Risk and rollback

Risk is contract drift between CLI, core, Tauri, and generated frontend types.
Use fixture-first changes and keep the existing command names and coordinator
identity. Roll back only the adapter/service edits if the focused gate fails;
preserve the planning evidence and do not add a subprocess fallback.
