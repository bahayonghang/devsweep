# Verification Evidence - Bound Filesystem Sizing Concurrency

Recorded on 2026-08-30 (Asia/Shanghai). All commands ran from
`D:\Documents\Code\Rust\Exp\devsweep`. No command performed cleanup, install,
uninstall, publication, signing, release, or remote mutation.

## Scope and diff audit

- `rtk git status --short` (exit `0`) showed the pre-existing unrelated
  `.trellis/.gitignore`, `README.md`, and `justfile` edits plus the task-owned
  `crates/devsweep-core/src/filesystem/sizing.rs` checkpoint and untracked task
  documents. No unrelated path was edited or staged by this implementation.
- `rtk git diff -- crates/devsweep-core/src/filesystem/sizing.rs` (exit `0`)
  confirmed the product diff is restricted to the module-owned pool, explicit
  serial fallback, and sizing tests.
- `rg -n "reports_current_global_rayon_workers|build_global\\(|par_iter\\("
  crates/devsweep-core/src/filesystem/sizing.rs` (exit `0`) printed only the
  single `par_iter()` inside `pool.install(...)` at line 347. It printed no
  temporary probe and no `build_global()` occurrence.
- No Cargo manifest, lockfile, CLI, IPC, Serde, configuration, desktop API, or
  public model file is part of this task's product diff.

## Focused implementation proof

Command:

```powershell
rtk cargo test -p devsweep-core filesystem::sizing
```

Initial checkpoint run: exit `0`, `16 passed`, `156 filtered out`, two suites,
`0.15 s`. Final post-benchmark run: exit `0`, `16 passed`, `156 filtered out`,
two suites, `0.16 s`.

The passing tests include:

- synchronized exact two-worker peak plus direct pool thread count;
- dedicated/forced-serial bytes, mtime, completeness, and warning-kind parity;
- a fresh local low entry budget for every root child;
- low max-depth incomplete evidence;
- pre-requested cancellation in both modes;
- synchronized mid-walk cancellation in both modes with join and `<250 ms`
  after gate release;
- existing Windows reparse/no-follow and fail-closed probe cases.

## Temporary global Rayon probe

Environment snapshot and required exact command:

```powershell
[Environment]::ProcessorCount
[Environment]::GetEnvironmentVariable('RAYON_NUM_THREADS','Process')
[Environment]::GetEnvironmentVariable('RAYON_NUM_THREADS','User')
[Environment]::GetEnvironmentVariable('RAYON_NUM_THREADS','Machine')
rtk cargo test -p devsweep-core filesystem::sizing::tests::reports_current_global_rayon_workers -- --exact --nocapture
```

Exit `0`. Output: logical CPUs `24`; all three `RAYON_NUM_THREADS` scopes
`<unset>`; RTK summary `1 passed`, `172 filtered out`.

Because compact RTK output suppresses captured stdout, the same test was also
run through the documented raw proxy:

```powershell
rtk proxy cargo test -p devsweep-core filesystem::sizing::tests::reports_current_global_rayon_workers -- --exact --nocapture
```

Exit `0`; raw output included `global_rayon_workers=24`, with `1 passed`,
`0 failed`. The temporary test was then removed via `apply_patch`; the final
search above proves it is absent.

## Immutable binary identity and benchmark input

PowerShell `Get-Item` plus `Get-FileHash -Algorithm SHA256` and each binary's
`scan --help` ran with overall exit `0` and established:

| Label | Absolute binary | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| A | `D:\Documents\Code\Rust\Exp\devsweep\target\devsweep-bench\unbounded\devsweep.exe` | 2673664 | `C89EB17F2CCFA98A7393E15A6AA338E92F25001FCD2459E90792C685669128FA` |
| B | `D:\Documents\Code\Rust\Exp\devsweep\target\devsweep-bench\two-workers\devsweep.exe` | 2686976 | `1D87A025C595F936F242F5EB91FF7DF6BD4F4CD890754CBED52EDFC82BD7463A` |

Both help outputs expose `scan [OPTIONS] [ROOT]...`, `--json`, and
`--projects`. The current staged CLI handler was not used.

Input preparation command:

```powershell
$root = (Resolve-Path -LiteralPath '.').Path
(Get-ChildItem -LiteralPath $root -Force -Recurse -ErrorAction Stop | Measure-Object).Count
Get-Process cargo,rustc,devsweep -ErrorAction SilentlyContinue
```

Exit `0`; root
`D:\Documents\Code\Rust\Exp\devsweep`; deterministic recursive entry count
`110393`; count duration `4308.151 ms`; heavy processes `none`.

## Repository-exclusive paired benchmark

Every timed invocation used exactly:

```powershell
& $binary scan $root --projects --json | Out-Null
```

The script set `LASTEXITCODE` to zero before each invocation, checked native
exit and pipeline success after it, and aborted on either failure. Overall
script exit: `0`. One run was discarded per binary before collecting the five
pairs. No build/test/package/icon work ran during this window.

| Stage | A unbounded (ms) | B two workers (ms) | Native exit |
| --- | ---: | ---: | ---: |
| Warm-up (discarded) | 31810.878 | 33945.057 | 0 / 0 |
| Pair 1, A then B | 35715.874 | 40222.620 | 0 / 0 |
| Pair 2, B then A | 38016.762 | 38281.145 | 0 / 0 |
| Pair 3, A then B | 36388.675 | 36281.985 | 0 / 0 |
| Pair 4, B then A | 36582.462 | 42232.281 | 0 / 0 |
| Pair 5, A then B | 38117.836 | 39999.052 | 0 / 0 |

- A sorted: `[35715.874, 36388.675, 36582.462, 38016.762, 38117.836]`;
  median (third) `36582.462 ms`.
- B sorted: `[36281.985, 38281.145, 39999.052, 40222.620, 42232.281]`;
  median (third) `39999.052 ms`.
- Threshold: `36582.462 * 1.20 = 43898.9544 ms`.
- Ratio: `39999.052 / 36582.462 = 1.093394`.
- Result: `39999.052 <= 43898.9544`, PASS. Candidate `2` was selected and the
  conditional four-worker candidate was correctly not measured.

Every invocation emitted the same non-fatal warning that Cargo metadata failed
for `ref/repo/putzen-rs` and no local target was available. Every invocation
still reported native exit `0` and pipeline success `True`; the warning was not
silently converted into success by the harness.

## Broader gates

```powershell
rtk cargo test -p devsweep-desktop scan
```

Exit `0`: `8 passed`, `12 filtered out`, two suites, `0.01 s`.

```powershell
rtk git diff --check
```

Exit `0`, no whitespace errors.

```powershell
rtk just ci
```

Exit `0`. It ran `cargo fmt --all -- --check`, offline dependency resolution
with zero packages changed, workspace locked all-target check, workspace locked
all-target tests, and workspace locked all-target Clippy with `-D warnings`.
The only displayed warning was Windows linker stdout while creating the desktop
DLL import library. Complete output is preserved in
`evidence/just-ci-2026-08-30.log`.

The independent `trellis-check` repeated `rtk just ci` after its AC-to-R and
real-diff audit; it also exited `0`. The complete 25,601-byte output is preserved
as `evidence/independent-check-just-ci-2026-08-30.log`, SHA-256
`7C0575EBB73CDB5F0D29F4C3770B53B0313F7D5DC80AA927629D724F4F53FE4E`.

## Verification boundary

This evidence verifies the Rust behavior, throughput threshold, task-focused
tests, desktop scan callers, whitespace gate, and full local CI. It does not
perform or claim any cleanup, install/uninstall, system modification, signing,
publication, release, or push.
