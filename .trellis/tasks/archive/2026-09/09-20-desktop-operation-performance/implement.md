# Implementation plan

## Preconditions

1. Confirm the shared typed-service child passed its contract review.
2. Read the parent design, archived five-mode performance protocol, desktop
   state/types specs, and backend quality/error specs.
3. Freeze the current build/fixture/host identifiers and keep native-only gaps
   separate from local measurements.
4. Build the Clean baseline release CLI binary from the parent base commit
   (the HEAD recorded when the first child started) in a separate git
   worktree with the same toolchain; preserve it and record its commit and
   SHA-256 (design, Clean baseline step 1).

## Ordered work

1. Run the baseline protocol for all workloads and retain raw artifacts.
2. Reproduce any threshold or lifecycle failure with focused traces and tests.
3. Apply the smallest fix at the verified coordinator/channel/render seam.
4. Extend `tools/measure-resources.ps1`: replace the Clean block at
   `:1381-1404` with the paired baseline/candidate runs, manifest fields,
   and the three real gates plus `clean.baseline_comparable`; add the
   desktop Status stop experiment with `status.desktop.*` gates beside the
   CLI-exit `status.live.*` gates; add request/ack/join latency capture for
   each row of the operation table. Add a sampler self-test that feeds an
   absent PID and a non-comparable manifest and expects `fail`.
5. Re-run focused tests, then repeat the full release protocol, the paired
   Clean comparison, and the desktop stop experiment.
6. Write the cause/fix/evidence record with the operation table's observed
   values and one status per row (`pass`, `fail`, `skipped` with reason,
   `UNVERIFIED` with reason and owner); hand native-only rows to the
   acceptance child without converting any measured `fail`.

## Validation

- `mise exec node@22 -- npm --prefix desktop run test -- --run src/state src/modes/analyze`
- `mise exec node@22 -- npm --prefix desktop run lint`
- `mise exec node@22 -- npm --prefix desktop run typecheck`
- `just ci`
- `& .\tools\measure-resources.ps1 -Protocol five-mode-v1` with the actual
  current protocol arguments, baseline binary path, and candidate binary
  path recorded in the artifact.
- Inspect `gates.json`: no gate named `*_recorded`, no `pass = $true` with
  `comparable = $false`, and every `status.desktop.*` sample carries the same
  live PID with `present = true`.

## Risk and rollback

The main risk is optimizing a fixture path while native lifecycle remains
unverified. Keep raw evidence and classify it. Roll back only the smallest
performance edit when semantic, cancellation, or resource evidence regresses.
