# Safety hardening and release implementation plan

## Ordered Checklist

1. [x] Safety tests
   - Add scanner tests for symlinked cleanup directories.
   - Add Windows-only junction/reparse-point coverage when the platform can create it.
   - Verify scanner still finds marker-backed real directories.

2. [x] Executor partial-failure reporting
   - Add or tighten tests for locked-file style trash failures using the existing `TrashRunner` seam.
   - Confirm reports include failed target IDs, continue later targets, and write failure audit rows with `partial = true`.

3. [x] TUI smoke coverage
   - Cover dashboard render, details overlay, confirm modal, and jobs/logs with `TestBackend` or app update tests.
   - Keep render path side-effect free.

4. [x] CI
   - Add GitHub Actions workflow for format, check, tests, and clippy.
   - Include Windows plus a non-Windows runner.

5. [x] Release packaging
   - Add a repo-local archive recipe for the release binary.
   - Keep package-manager distribution out of scope.

6. [x] README
   - Document MVP safety behavior, command-backed/trash-backed boundaries, inspect-only Cargo home, disabled permanent delete, Docker deferred, validation, and release archive.

## Validation Commands

Run after implementation:

```powershell
just ci
just release-archive
```

If `just` is unavailable, run the equivalent commands:

```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Risky Files

- `src/scanner.rs`: platform-specific filesystem semantics can accidentally hide valid targets or follow unsafe paths.
- `src/executor.rs`: failure handling must continue without losing audit data.
- `src/tui.rs`: tests should not require a live terminal.
- `.github/workflows/ci.yml`: keep workflow commands aligned with `justfile`.
- `justfile`: release archive commands must work on Windows PowerShell.
- `README.md`: do not document Docker or permanent delete as implemented features.

## Completion Criteria

- [x] Safety and TUI tests pass locally.
- [x] CI workflow exists and uses the agreed validation commands.
- [x] Release archive can be produced locally.
- [x] README explicitly documents Docker as deferred.
- [x] Task acceptance criteria in `prd.md` are all satisfied or updated with documented platform limitations.
