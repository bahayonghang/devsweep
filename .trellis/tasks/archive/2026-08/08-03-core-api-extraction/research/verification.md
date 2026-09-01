# Core API Extraction Verification

Date: 2026-08-03
Hosts: Windows, stable `rustc 1.97.0` and MSRV `rustc 1.88.0`; Ubuntu 24.04
under WSL, stable `rustc 1.95.0`

## Deterministic JSON Equivalence

The fixture generator is `tests/fixtures/make_scan_fixture.ps1`. It creates
Rust, Node, and Python projects, fixes every file and directory timestamp to
`2024-01-01T00:00:00Z`, and gives the Rust project its own empty `[workspace]`
so enclosing workspace discovery cannot affect Cargo metadata. Recursive
replacement is accepted only for the dedicated
`devsweep-core-api-extraction-fixture` leaf under the repository or system
temporary directory. Existing path components are rejected if any is a
reparse point, so recursive replacement cannot traverse a junction or symlink.

The pre-move source was reconstructed from clean commit
`4917126e822084d7174708fc5776aef6447709bb` rather than from the modified working
tree:

```powershell
$ErrorActionPreference = 'Stop'
$baselineCommit = '4917126e822084d7174708fc5776aef6447709bb'
$comparisonRoot = Join-Path ([System.IO.Path]::GetTempPath()) `
  ("devsweep-core-api-extraction-comparison-" + [guid]::NewGuid().ToString())
$source = Join-Path $comparisonRoot 'baseline-source'
$archive = Join-Path $comparisonRoot 'baseline.tar'
$fixture = Join-Path ([System.IO.Path]::GetTempPath()) `
  'devsweep-core-api-extraction-fixture'
$baselineRaw = Join-Path $comparisonRoot 'baseline.raw.json'
$afterRaw = Join-Path $comparisonRoot 'after.raw.json'
$baselineNormalized = `
  '.trellis\tasks\08-03-core-api-extraction\research\baseline.normalized.json'
$afterNormalized = `
  '.trellis\tasks\08-03-core-api-extraction\research\after.normalized.json'

New-Item -ItemType Directory -Path $source -Force | Out-Null
.\tests\fixtures\make_scan_fixture.ps1 -OutputPath $fixture

git archive --format=tar --output $archive $baselineCommit
if ($LASTEXITCODE -ne 0) { throw 'git archive failed' }
tar -xf $archive -C $source
if ($LASTEXITCODE -ne 0) { throw 'baseline extraction failed' }

cargo run --locked --manifest-path "$source\Cargo.toml" --bin devsweep -- `
  scan --json --projects $fixture | Set-Content -Encoding utf8 $baselineRaw
if ($LASTEXITCODE -ne 0) { throw 'baseline scan failed' }
jq -S . $baselineRaw | Set-Content -Encoding utf8 $baselineNormalized
if ($LASTEXITCODE -ne 0) { throw 'baseline jq normalization failed' }

cargo run --locked -p devsweep-cli --bin devsweep -- `
  scan --json --projects $fixture | Set-Content -Encoding utf8 $afterRaw
if ($LASTEXITCODE -ne 0) { throw 'workspace scan failed' }
jq -S . $afterRaw | Set-Content -Encoding utf8 $afterNormalized
if ($LASTEXITCODE -ne 0) { throw 'workspace jq normalization failed' }

git diff --no-index --exit-code -- $baselineNormalized $afterNormalized
```

Result: exit code 0 and no diff. No fields were removed or rewritten. The same
fixture path and fixed timestamps were used for both scans, so health,
diagnostics, intent, risk, selection, evidence, paths, and modification times
were compared directly.

## Local Quality Gates

| Gate | Result |
| --- | --- |
| `just ci` | PASS: format, lock sync, workspace check, 226 tests, Clippy `-D warnings` |
| `cargo test --workspace --locked --all-targets` | PASS: 70 CLI/TUI + 155 core + 1 external API test |
| `cargo test -p devsweep-core --test public_api` | PASS: external-crate `use devsweep_core::...` proof |
| Clean-snapshot CLI help diff | PASS: root plus `tui`, `scan`, `inventory`, `clean`, `protect`, and `rules` are byte-equivalent |
| Clean-snapshot CLI version diff | PASS: both report `devsweep 0.2.0` |
| `$env:RUSTDOCFLAGS = '-D missing-docs'; cargo doc -p devsweep-core --no-deps` | PASS |
| `cargo tree -p devsweep-core` forbidden dependency search | PASS: no `clap`, `ratatui`, or `crossterm` |
| `rustup run 1.88.0 cargo check --workspace --locked --all-targets` | PASS |
| `rustup run 1.88.0 cargo test --workspace --locked --all-targets` | PASS: 226 tests |
| Windows clean target: `cargo test --target-dir target/core-api-clean-check --workspace --locked --all-targets` | PASS: 226 tests; proves no cached `process_fixture` dependency |
| Ubuntu 24.04 WSL: workspace test + Clippy in `/tmp/devsweep-linux-target-clean` | PASS: 226 tests; Unix process-group tree termination ran dynamically |
| macOS cross-check: `cargo check --workspace --locked --all-targets --target x86_64-apple-darwin` | PASS: all workspace and test targets compile for macOS |
| `just release-archive` + `just release-smoke` | PASS: Windows archive, version, scan, and empty-plan dry run |

`cargo metadata --format-version 1 --no-deps` reports the two workspace
packages `devsweep-core` and `devsweep-cli`. The CLI package retains the
`devsweep` library and binary target names and the `process_fixture` binary.

## Cross-Platform Evidence Boundary

The workflow still has the Windows, Ubuntu, and macOS stable matrix plus the
Windows and Ubuntu MSRV matrix, and all Rust commands now use
`--workspace --locked --all-targets`. The documented local-equivalent path is:

- Windows stable and MSRV checks/tests, including a clean target directory;
- Ubuntu 24.04 stable checks/tests/Clippy with the Unix process-group behavior
  exercised dynamically;
- macOS `--all-targets` cross-compilation, with the shared Unix process-group
  implementation dynamically covered on Ubuntu.

This satisfies the PRD's local-equivalent option without pushing. Hosted GitHub
Actions themselves were not run, so this evidence does not claim that the
remote jobs have completed successfully.
