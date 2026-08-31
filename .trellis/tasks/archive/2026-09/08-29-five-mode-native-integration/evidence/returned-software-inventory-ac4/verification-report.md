# Verification — reopen five-mode AC4 (`software.p95_elapsed_le_5s`)

Working-tree copy of archived `08-29-software-inventory-plan`. Not archived. Not committed.

## Change

`--source all` now overlaps ARP, MSI, and MSIX **source** enumeration on named `std::thread` workers (`devsweep-software-arp`, `devsweep-software-msi`) while MSIX runs on the caller (existing joined MTA worker). Results merge through the same `assemble_inventory` path as before.

MSIX WinRT enumeration stays on the MTA thread. Installed-path size walks run **after** MTA join and queue on the existing `SIZE_WALK_POOL` (still **2** workers, ceiling unchanged). Walks remain bounded, cancelable, no-follow, exact `InstalledPath` only. No third Rayon pool. No new dependencies. `windows = 0.56.0` features unchanged.

Single-source `arp` / `msi` / `msix` inventory stays sequential. Source-worker panic maps that family to `partial` / `source_worker_panicked` without erasing siblings. Spawn failure falls back to in-process inventory for that family.

## Native elapsed (release `devsweep.exe`)

Command (warmup + five measured reps):

```text
software inventory --source all --format json --output <file>
```

Clock: process start to exit (`Stopwatch`). p95: nearest-rank `ceil(0.95 * n) - 1` (same as `tools/measure-resources.ps1`). For n=5 that is the maximum sample.

| Rep | elapsed_ms | exit |
| --- | --- | --- |
| warmup | 4033.795 | 0 |
| 1 | 4379.214 | 0 |
| 2 | 3941.491 | 0 |
| 3 | 4225.933 | 0 |
| 4 | 3935.930 | 0 |
| 5 | 4087.308 | 0 |

- **p95 = 4379.214 ms** (target ≤ 5000 ms) — **PASS**
- median = 4087.308 ms
- Binary: `target/release/devsweep.exe` (5,254,656 bytes)
- Raw: `native-timing.json`, `native-timing.log`

Prior independent five-mode-v1 miss on this host was p95 5205–6481 ms with every later measured rep above 5 s.

## Contracts (run 1 document)

`native-inventory-run1.json`: envelope `outcome=partial` (live ARP incomplete entries, unchanged). 1,293 entries (823 ARP, 291 MSI, 179 MSIX). 8 sources. All 291 MSI manual. 0 selectable non-MSIX. Every `last_used` is `unknown` / `no_supported_exact_source`. No `UninstallString`, `QuietUninstallString`, `DisplayIcon`, or `"path"` field. `msix_installed_path` remains the closed size `source_code` only.

## Tests

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test --locked -p devsweep-core -- software` | 0 (38 passed) | `core-software-tests.log` |
| `rtk cargo test --locked -p devsweep-cli -- software` | 0 (13 passed) | `cli-software-tests.log` |
| `rtk cargo test --locked -p devsweep-core -- estimate_trees_preserve_order` | 0 (1 passed) | `sizing-batch-test.log` |
| `rtk git diff --check` | 0 | `git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 | `cargo-fmt-check.log` |
| `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` | 0 | `release-build.log` |

`just ci` was not re-run (focused gates above were cheap and green).

## Source files

- `crates/devsweep-core/src/software/mod.rs` — overlap ARP/MSI with MSIX on `--source all`
- `crates/devsweep-core/src/software/msix.rs` — size walks after MTA, queued on existing pool
- `crates/devsweep-core/src/filesystem/sizing.rs` — `estimate_trees_with_budget_and_cancel` on `SIZE_WALK_POOL`
- `crates/devsweep-core/src/filesystem/mod.rs` — re-export
- `crates/devsweep-core/src/software/tests.rs` — panic-fallback isolation
