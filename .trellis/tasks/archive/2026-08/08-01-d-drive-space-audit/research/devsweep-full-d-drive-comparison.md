# Full D: Project Scan Comparison And Rule Assessment

Status: read-only research, 2026-08-01. This report does not authorize a
cleanup plan, `devsweep clean`, `--execute`, deletion, trashing, moving, or
any other filesystem mutation.

## Scope And Evidence

The complete D: project-rule scan used:

```powershell
cargo run --quiet --bin devsweep -- scan --projects --json D:\
```

`--projects` is intentional. It keeps the audit constrained to D: roots and
does not probe global providers whose configured homes can be on another
volume. The raw plan is the complete 657-row result:

- `devsweep-d-drive-project-scan-20260801.json`
- SHA-256: `C1A32CD95DD1EF6AE59C3801ADFCA78F710C664ECAEA37E8915785E69751D1E2`
- `devsweep-d-drive-project-scan-20260801.stderr.log` retains the scanner
  warnings that the plan JSON cannot represent.

All byte values below are logical file-length estimates. They are not an
allocated-space measurement and must not be summed with the filesystem
inventory to reproduce D: free space. A target with `size_complete=false` is
only a lower-bound observation, not a reclaim estimate.

## Complete Project-Rule Result

| Rule | Targets | Logical GiB | Complete targets / GiB | Partial targets / GiB | Default-selected |
| --- | ---: | ---: | ---: | ---: | ---: |
| `rust.target` | 34 | 643.688 | 27 / 226.137 | 7 / 417.551 | 22 |
| `python.venv_dot` | 23 | 30.323 | 20 / 28.172 | 3 / 2.152 | 0 |
| `node.node_modules` | 81 | 14.705 | 77 / 10.290 | 4 / 4.416 | 0 |
| `python.venv` | 1 | 0.010 | 1 / 0.010 | 0 / 0.000 | 0 |
| `python.__pycache__` | 497 | 0.049 | 497 / 0.049 | 0 / 0.000 | 479 |
| `python.pytest_cache` | 10 | 0.001 | 10 / 0.001 | 0 / 0.000 | 8 |
| `python.ruff_cache` | 11 | less than 0.001 | 11 / less than 0.001 | 0 / 0.000 | 9 |
| **Total** | **657** | **688.777** | **643 / 264.658** | **14 / 424.119** | **518** |

The plan has 33 `rust.target` command-backed targets (637.944 GiB) and one
medium-risk, trash-backed fallback target at
`D:\Documents\Code\Rust\Reference\zed\target` (5.744 GiB). The fallback
is not selected by default.

The largest partial-size targets are Rust build directories under
`llmusage` (126.984 GiB), `ccr-ui` (70.920 GiB), `ccr` (64.659 GiB),
`PromptHub` (57.210 GiB), `zotero-cli` (42.639 GiB),
`skills-manage-windows` (32.682 GiB), and `quanergy_client_rs` (22.458 GiB).
They account for 417.551 GiB of the partial total. They are intentionally not
recommended for any capacity claim.

## Comparison With Earlier Scan And D: Inventory

| Evidence | Targets | Logical GiB | Complete targets / GiB | Interpretation |
| --- | ---: | ---: | ---: | --- |
| Earlier Code-only project scan (`D:\Documents\Code`) | 428 | 665.337 | 415 / 242.575 | Baseline project-cleanup view |
| Complete D: project scan (`D:\`) | 657 | 688.777 | 643 / 264.658 | Full D: project-rule coverage |
| D: scan outside `Documents\Code`, calculated from the same full scan | 229 | 23.252 | 228 / 22.083 | Additional roots found by full coverage |
| `D:\Documents\LYH` subset | 34 | 8.026 | 34 / 8.026 | Must be excluded; it is a Cloud Files reparse root |
| Other non-Code D: subset | 195 | 15.226 | 194 / 14.057 | Valid scan observations, still subject to normal review |

All 428 earlier Code-only target IDs also occur in the full scan. The two scans
were taken at different times, so the full scan's Code subset is 0.188 GiB
larger; that is measurement drift, not additional coverage. The raw-plan total
difference is 23.440 GiB, while the same-time non-Code contribution is 23.252
GiB.

The broad filesystem inventory answers a different question: why D: is full.
It recorded 1,450.85 GiB used on a 1,907.73 GiB volume. Its largest logical
roots were `Documents` (1,308.97 GiB), `SteamLibrary` (108.81 GiB),
`BaiduNetdiskDownload` (59.21 GiB), `Music` (35.27 GiB),
`$RECYCLE.BIN` (27.52 GiB), and `D:\.pnpm-store` (8.77 GiB). Steam, personal
downloads/media, the recycle bin, and the legacy-looking pnpm store are not
project cleanup artifacts and must not be converted into raw deletion rules.

## Scan Reliability Findings

### P0: Cloud Files reparse root was traversed

`fsutil reparsepoint query D:\Documents\LYH` reports tag `0x9000701a`. The
directory has `0x80430` attributes when enumerated from its parent but
`0x80030` when queried directly; the missing `0x400` bit is exactly
`FILE_ATTRIBUTE_REPARSE_POINT`. devsweep currently determines Windows reparse
status from `MetadataExt::file_attributes()` in `src/fs_size.rs`, and all
scanner, sizing, and live-revalidation guards use that helper.

The full scan nevertheless emitted 34 paths below this root: four
`node_modules`, two `.venv` directories, one Ruff cache, and 27
`__pycache__` directories. That is an observed violation of the README's
reparse-point skip claim, not merely a theoretical edge case. No action was
executed, but these targets must never be approved from this plan.

Required correction before any implementation task uses this scan:

1. Add a Windows path-level reparse probe that opens the path without following
   it and checks a reparse tag through a Win32 API, rather than relying only on
   direct `Metadata` attributes.
2. Apply the same guard to scanner descent, size walking, target validation,
   marker validation, and ancestor revalidation. Updating scan discovery alone
   leaves an execution-time bypass.
3. Add regression coverage for the Cloud Files tag path (or an injectable
   platform attribute probe) at root, child, and ancestor positions. A normal
   directory symlink test does not cover this provider behavior.

### P1: Cargo metadata warnings are silent in the JSON contract

The D: scan emitted 200 warnings: 186 `cargo metadata returned invalid JSON`
messages and 14 `cargo metadata` exit-101 failures. `ProcessRunner` keeps only
the last 1 MiB of each stream but records truncation metadata. The Cargo query
parses captured stdout without testing the truncation flag. This is a strong
explanation for the Zed workspace invalid-JSON warnings, but should be proven
by a focused regression test before changing behavior.

`ProjectScanner` has `ScanOutcome.diagnostics` and `ScanCompleteness`, yet
`Sweeper` converts it to a `CleanupPlan`, and `main.rs` serializes only that
plan. Cargo metadata failures are tracing warnings rather than structured scan
diagnostics. A JSON consumer therefore cannot tell that the scan had 200
metadata probes fail, even though individual targets expose `size_complete`.

Required improvements:

1. Serialize scan-level completeness and structured diagnostics in CLI/TUI
   output, including path, stage, probe status, truncation, and captured versus
   total output bytes. Preserve clean JSON on stdout and diagnostics separately
   from human stderr rendering.
2. Treat truncated metadata output as an explicit probe outcome, never as a
   generic JSON parse failure. Do not make a medium-risk fallback target unless
   its provenance is represented in the plan.
3. Cache successful Cargo workspace resolution per canonical workspace during a
   scan, while retaining the existing live Cargo revalidation immediately
   before a command-backed cleanup. Avoid blindly sharing a parent workspace
   result across a nested independent workspace.

### P1: Partial sizes dominate the apparent reclaim total

The default tree estimator has a 50,000-entry budget and a depth limit of 64.
It intentionally returns a lower bound with `size_complete=false` on budget or
read failures. The behavior is safe, but 424.119 GiB (61.58 percent of the
displayed total) comes from fourteen partial targets. Presenting 688.777 GiB
as a single reclaimable total invites the wrong interpretation.

The plan format and UI should show separate verified and partial totals, label
partial values as lower bounds, retain sizing warnings, and offer a targeted
higher-budget rescan only after a user selects a target for review. The default
scan should remain bounded.

### P2: `__pycache__` dominates rows but not space

`python.__pycache__` accounts for 497 of 657 rows (75.65 percent) but only
about 50 MiB. Thirty-six of those rows are under `site-packages` and 29 are
under `.trellis`. This is primarily a plan-review and TUI usability issue, not
a reason to broaden deletion.

Prefer presentation-level grouping by owning Python project, plus a collapsed
default view or small-item threshold. Keep each exact path available for
inspection and preserve the existing action/revalidation semantics. Do not
globally exclude `site-packages` or administrative directories without a
separate safety decision; they may occur in legitimate project layouts.

### P2: Capacity inventory is a separate product surface

The project rules correctly omit the dominant non-project roots. The pnpm
provider also only discovers the currently configured `pnpm store path`, so it
will not find an old D: store after pnpm has moved to C:. Add a read-only
capacity-inventory/report mode and, separately, an inspect-only orphan-store
finding that proves the current configuration and project references before it
is surfaced. Neither feature should create a cleanup action or default
selection for Steam, media, downloads, recycle-bin contents, or an old store.

## Recommendation

The rules need optimization, but the first changes should increase safety and
truthfulness rather than expand what devsweep can delete:

1. Block on the P0 Cloud Files reparse-point fix and regression tests.
2. Add structured scan health plus separate verified/partial capacity totals.
3. Make Cargo workspace discovery bounded and workspace-aware, then verify it
   against a large real workspace and nested independent workspaces.
4. Reduce `__pycache__` presentation noise without changing target semantics.
5. Add inventory-only reporting for storage that deliberately remains outside
   cleanup rules.

No product source code was changed as part of this audit. Any later
implementation requires a new reviewed plan and explicit approval to activate
the task.
