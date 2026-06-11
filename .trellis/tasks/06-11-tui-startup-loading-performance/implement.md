# Optimize TUI startup loading performance - Implementation Plan

## Checklist

1. Load pre-development specs before editing.
   - Required: frontend quality, component, state, hook/event, type-safety, and
     backend quality/logging if scan/provider code is touched.
   - Verify: note the relevant constraints before code edits.

2. Add or adjust focused TUI tests for staged scan behavior.
   - Verify: a project partial result can populate `App.targets` before a
     global result or final scan completion.
   - Verify: final global results are merged without duplicates.
   - Verify: stale scan job updates cannot overwrite the active scan snapshot.

3. Introduce an internal staged scan worker event.
   - Keep the event private to `src/tui.rs`.
   - Include job id, phase/progress message, and optional partial plan or target
     slice as needed.
   - Verify: existing `startup_effects_request_an_initial_scan` still passes.

4. Split startup scan worker emission into phases.
   - Emit project progress.
   - Run `ProjectScanner::scan_roots`.
   - Emit project partial results immediately.
   - Emit global progress.
   - Run `GlobalProviderScanner::scan`.
   - Emit global partial/final results.
   - Verify: error handling still reports job failure if either phase fails.

5. Add deterministic scan snapshot merge logic in `App`.
   - Keep project and global staged targets separate for the active scan.
   - Rebuild `App.targets` in stable project-then-global order.
   - Recompute selected ids through `default_selected_ids`.
   - Verify: manual `s` scan starts a fresh staged snapshot.

6. Improve Jobs/Logs phase messages.
   - Make startup scan progress specific enough to explain long global work.
   - Verify: TUI tests assert representative progress text without brittle full
     frame snapshots.

7. Consider only small provider/scanner micro-optimizations if staged results
   leave a measurable problem.
   - Possible follow-up: avoid deep size estimation for inspect-only Cargo home
     on startup or make size estimation optional/deferred.
   - Guardrail: do not change cleanup target definitions without updating
     requirements and tests.

8. Run focused validation while iterating.
   - `cargo test tui::tests --all-targets`
   - Add narrower filters if needed during implementation.

9. Run the canonical gate.
   - `just ci`

10. Record final performance evidence.
    - Re-run the debug scan timing commands used during diagnosis.
    - If practical, manually open `just dev` / `cargo run -- tui` and verify the
      Dashboard receives project results before global scan completion.

## Validation Commands

```powershell
cargo test tui::tests --all-targets
cargo fmt --all -- --check
just ci
```

Optional diagnostic timing commands:

```powershell
$exe = Resolve-Path '.\target\debug\devsweep.exe'
$sw=[Diagnostics.Stopwatch]::StartNew(); & $exe scan --projects; $sw.Stop(); $sw.ElapsedMilliseconds
$sw=[Diagnostics.Stopwatch]::StartNew(); & $exe scan --global; $sw.Stop(); $sw.ElapsedMilliseconds
```

## Risk Points

- Do not remove startup scanning entirely; earlier accepted behavior requires
  automatic discovery.
- Do not let stale scan workers update the current target list after a newer
  manual scan starts.
- Do not duplicate targets when project and global phases arrive separately.
- Do not preserve selected ids for targets that disappeared from the current
  scan snapshot.
- Do not move filesystem/process work into render functions.
- Do not rely on exact timing thresholds for tests; use state/event behavior as
  the regression guard.

## Review Gate Before `task.py start`

- `prd.md`, `design.md`, and `implement.md` describe staged startup scanning
  as the intended implementation.
- No blocking product questions remain.
- The implementation scope is primarily `src/tui.rs`; scanner/provider changes
  are optional follow-ups only if evidence requires them.
