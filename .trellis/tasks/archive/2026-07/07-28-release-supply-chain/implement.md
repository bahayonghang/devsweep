# Implementation Plan

## Preconditions

- [ ] `07-28-license-baseline` has landed `LICENSE`, repository/readme
  metadata, and `package.rust-version`; record the exact value used by the
  MSRV CI row.
- [ ] The preceding P1 safety suites have stable CI job names. Do not enable
  remote required checks before that evidence exists.
- [ ] Re-check the three Action pin commits against their upstream repositories
  and record the reviewed SHAs in the workflow comments.
- [ ] Confirm the current dependency closure with `cargo deny check`; create a
  closed `deny.toml` policy from the observed license/source set rather than a
  broad permissive exception.

## Slice 1: CI Policy And Security Gates

1. Add top-level minimal permissions, concurrency, and explicit timeouts to
   `.github/workflows/ci.yml`.
2. Replace tag or branch Action references with reviewed immutable SHAs.
3. Expand the stable quality matrix to Windows, Ubuntu, and macOS. Keep format,
   locked check, locked test, and locked clippy commands equivalent to `just
   ci`; do not add unsupported `--locked` to `cargo fmt`.
4. Add a fixed-version MSRV job that compares its toolchain value to
   `Cargo.toml` metadata before running locked check and test.
5. Add independent audit, deny, and gitleaks jobs. Pin CI-only tool versions;
   add `deny.toml` and a gitleaks policy file only when each has a concrete
   reviewed policy need.

Rollback point: revert the workflow/config files together. Do not alter remote
branch protection in this slice.

## Slice 2: Reproducible Windows Archive

1. Refactor `just release-archive` to discover or accept the actual Windows
   target triple, run a locked release build, and reject unsupported targets
   before staging output.
2. Stage the binary, README, and LICENSE in a temporary directory; generate a
   target-named zip and SHA-256 sidecar only after staging succeeds.
3. Add a Windows PowerShell smoke script that validates zip contents and hash,
   expands into a temporary directory, and invokes `--version`, `scan --json`,
   and default `clean` without `--execute`.
4. Update README release instructions to describe the target-derived filename
   and checksum. Keep `dist/` ignored and out of commits.

Rollback point: restore the former recipe and documentation only after any
generated `dist/` files have been removed or left ignored; no source data is
deleted by the recipe.

## Slice 3: Required-Check Handoff

1. Add `docs/ci.md` with exact stable check names, command purpose, and the
   prerequisite P1 regression-suite names.
2. Run the full workflow on the default branch and record run URLs/check names
   in the task check record.
3. Ask a repository administrator to apply the documented branch-protection
   requirements only after all named checks are green. This is an external
   manual action, not a workflow mutation.

Rollback point: if a required check flakes or is renamed, remove it from remote
protection before reverting CI so the default branch remains mergeable.

## Validation Matrix

| Case | Evidence |
| --- | --- |
| Local quality parity | `just ci` and the four CI quality commands pass; dependency-resolving commands use `--locked`. |
| Lock drift | In a disposable worktree, intentionally perturb the lockfile and prove a locked Cargo check fails; discard the worktree afterward. |
| Stable matrix | Windows, Ubuntu, and macOS quality rows are green. |
| MSRV | Exact Cargo `rust-version` row runs locked check and test; a deliberately mismatched value fails the comparison step. |
| Security tools | Audit, deny, and gitleaks jobs pass independently; any exception has a checked-in rationale. |
| Archive identity | Windows archive filename contains the actual target triple, includes binary/README/LICENSE, and its sidecar verifies. |
| Archive smoke | Extracted native archive passes `--version`, `scan --json`, and default dry-run `clean` without an audit log or cleanup side effect. |
| Generated output | `git status --short --ignored` shows `dist/` ignored and no archive is staged. |
| Remote gate | After P1 evidence and green runs, an administrator verifies every documented required check is configured. |

Run `just ci`, the Windows archive smoke script, `cargo audit`, `cargo deny
check`, and the task validation command before reporting the child complete.

## Start Review Gate

Do not run `task.py start` until all of the following are true:

- [ ] `design.md`, this plan, and both curated context manifests pass review.
- [ ] License-baseline has supplied the exact `rust-version` input.
- [ ] The maintainer accepts the closed dependency-license/source policy and
  the reviewed Action pin snapshot.
- [ ] The parent has confirmed that P1 suites and their required-check names
  are ready for the release handoff.
- [ ] The administrator-owned branch-protection step is acknowledged as a
  post-green external gate, not silently assumed complete.
