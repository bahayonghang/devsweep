# CI And Verifiable Windows Release Archive

## Status And Boundaries

This is a planning-only child of `07-28-audit-remediation`. It remains in
`planning` until the parent start gate is satisfied. The task owns repository
automation and release artifacts, not product cleanup behavior.

In scope at implementation time:

- `.github/workflows/ci.yml` and any narrowly scoped CI policy files.
- `deny.toml`, an optional checked-in gitleaks allowlist only when a reviewed
  false positive needs one, and `docs/ci.md` for the required-check inventory.
- `justfile`, a release-archive smoke script, and the README release section.

Out of scope:

- `src/` behavior, release distribution channels, `cargo publish`, SBOM,
  signing, provenance attestations, and automated branch-protection mutation.
- LICENSE, repository metadata, and the choice of `package.rust-version`;
  `07-28-license-baseline` owns those inputs.
- New production dependencies. CI-only tools are installed or invoked only in
  CI and must be version-pinned.

## Fixed Inputs

- D4 is MIT. The archive must include the root `LICENSE` and `README.md`.
- The release task is last in the parent order. Required checks are documented
  before they are configured remotely, and only after the preceding P1 suites
  exist and have produced stable check names.
- The current public contract is a Windows single-binary archive. Linux and
  macOS validate build and tests in CI; they do not claim a release archive.
- `edition = "2024"` makes `1.85.0` the expected lower-bound candidate. The
  exact MSRV remains the `rust-version` value committed by license-baseline;
  this task consumes that value rather than creating a competing source.

## CI Contract

The workflow has top-level `permissions: contents: read` and a top-level
concurrency group scoped by workflow and ref, with `cancel-in-progress: true`.
Every job has an explicit timeout. It must never use `pull_request_target` or
write credentials.

The stable quality matrix runs on `windows-latest`, `ubuntu-latest`, and
`macos-latest`. Each row runs the same local gate classes:

```text
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

`cargo fmt` intentionally has no `--locked`: Cargo's formatter does not
support that option and does not resolve the dependency graph. All dependency
resolving Cargo commands, including the release build and `cargo package`, use
`--locked`.

One MSRV job runs the exact `package.rust-version` toolchain on a supported
host and executes at least locked check and test. The stable matrix remains the
cross-platform compatibility signal. The workflow must fail if the hard-coded
MSRV row and Cargo metadata diverge; the implementation may use a small
workflow-local comparison step, but must not duplicate the value as an
unvalidated second source of truth.

Security gates are separate stable jobs so each failure is legible:

- `cargo audit` checks RustSec advisories against the committed lockfile.
- `cargo deny check` enforces advisory, ban, license, and source policy from
  `deny.toml`. The initial allowlist is derived from the current lockfile
  closure; future licenses or sources fail until explicitly reviewed.
- Gitleaks scans the checkout without exposing secrets. A false-positive
  exception must be narrow, explained, and checked into its policy file.

All GitHub Actions use immutable 40-character SHA references with a human
readable version comment. Planning verification recorded these candidate pins:

```text
actions/checkout v4.2.2: 11bd71901bbe5b1630ceea73d27597364c9af683
dtolnay/rust-toolchain stable snapshot: 4cda84d5c5c54efe2404f9d843567869ab1699d4
gitleaks/gitleaks-action v2: ff98106e4c7b2bc287b24eaf42907196329070c7
```

Before implementation, re-resolve each advertised upstream reference and
record any intentional pin update in review. `cargo-audit` and `cargo-deny`
are installed with explicit versions and `--locked`; they are not Cargo product
dependencies.

## Release Archive Contract

`just release-archive` remains a Windows-native operation. Its target triple
is either an explicit installed Windows target or the host triple parsed from
`rustc -vV`; it is never a fixed `x86_64-pc-windows-msvc` label. Unsupported
hosts, missing targets, or a non-Windows target must fail before staging an
archive.

The recipe builds with:

```text
cargo build --locked --release --target <actual-target-triple>
```

It stages exactly these files in a temporary directory:

```text
devsweep.exe
README.md
LICENSE
```

The published local output is `dist/devsweep-<actual-target-triple>.zip` with
a sibling `dist/devsweep-<actual-target-triple>.zip.sha256`. The checksum is
computed over the final zip bytes. A pre-existing archive is replaced only
after a new staged archive and checksum exist; `dist/` remains generated and
ignored.

The host-target smoke test expands the zip into a temporary directory, verifies
the SHA-256 sidecar, and runs the bundled executable with `--version`,
`scan --json`, and `clean` without `--execute`. The last command proves the
archive's dry-run path and must not create an audit log or execute cleanup.
Cross-architecture Windows archives are named correctly but are not executed
on an incompatible host; native CI covers the host smoke path.

## Required Checks And Rollout

`docs/ci.md` records stable job display names, their purpose, and the future
branch-protection inventory: three stable quality rows, MSRV, audit, deny,
gitleaks, and Windows archive smoke. It also lists the P1 safety regression
suite names once their owning tasks deliver them.

Remote required-check configuration is deliberately manual: a maintainer with
repository administration permission first observes green workflow runs on the
default branch, then applies the documented inventory. It must not be changed
by CI. This prevents an unavailable or renamed check from blocking every merge.

Rollback order is: remove new remote protection requirements first if they
block recovery, revert the workflow/configuration change, then revert the
recipe or documentation change. Release archives are reproducible generated
outputs and are never rollback inputs or committed source.

## Failure Boundaries

- A stale lockfile, advisory, denied license/source, secret finding, tool
  installation failure, or smoke failure fails its own CI job.
- A missing `rust-version` handoff blocks this task rather than guessing an
  MSRV.
- An unsupported archive target fails before deleting or replacing the existing
  output.
- macOS is not evidence for Windows Trash or archive runtime behavior; it only
  provides the specified compile and unit-test signal.
