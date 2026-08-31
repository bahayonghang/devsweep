# Independent Trellis Check Report — reopen five-mode AC4

Working-tree copy of archived `08-29-software-inventory-plan`. Original archive left in place. Not committed. Not `task.py archive`.

## Overall result: **PASS**

Independent recapture of `software.p95_elapsed_le_5s` is **4253.265 ms** (target ≤ 5000 ms). Overlapping ARP/MSI/MSIX source enumeration does not change identity, fingerprint assembly, eligibility, or privacy contracts. Sizing remains on the existing two-worker pool. No product bug found; no Software-crate fix applied.

## Child verification

- AC1: PASS. ARP still reads only allowlisted identity/display/classification/`EstimatedSize` values. `UninstallString`, `QuietUninstallString`, and `DisplayIcon` are absent from `arp.rs` (allowlist test) and from the independent native document.
- AC2: PASS. Tagged ARP hive/view/subkey, MSI product-code/context, and current-user MSIX full-name identities are unchanged. All 291 MSI entries are manual. Only current-user MSIX is selectable (`eligible_current_user_msix`, 86 of 179). Eight source partitions remain; one failed source still cannot erase siblings.
- AC3: PASS. Preview/revalidation path was not part of this reopen diff. Exact-identity merge, ordered refusal, and fail-closed plan tests still pass under `cargo test -p devsweep-core -- software`.
- AC4: PASS. Native `--source all` warmup + five new-PID reps, isolated `LOCALAPPDATA`, nearest-rank p95 ≤ 5000 ms. Preview/plan payloads were not expanded. Focused fmt/diff/software tests pass.
- AC5: PASS. Every native `last_used` is exactly `{state: unknown, reason_code: no_supported_exact_source}`. Size codes on the wire are only `arp_estimated_size_kib` and `msix_installed_path` (closed set; MSI sizes on this host are `unknown`/not reported). No `"path":` key and no `installed_path` property on entries.

## Reopen contracts

| Contract | Result | Evidence |
| --- | --- | --- |
| Overlap does not change identity/fingerprint/eligibility assembly | PASS | `--source all` still merges into `assemble_inventory` (sort/dedup sources, `BTreeMap` by tagged identity, id-sorted entries, fingerprint over version/timestamp/sources/entries). Single-source `arp`/`msi`/`msix` stays sequential. |
| Privacy: no vendor command/icon/installed path on the wire | PASS | Independent run1: no `UninstallString` / `QuietUninstallString` / `DisplayIcon` / `"path":`. MSIX `InstalledPath` is walked after MTA join and never serialized. |
| Sizing worker ceiling remains 2 | PASS | `SIZE_WALK_WORKERS = 2`; `estimate_trees_with_budget_and_cancel` queues on `SIZE_WALK_POOL` only. No third Rayon pool. |
| No new dependencies | PASS | `crates/devsweep-core/Cargo.toml` still Windows-only `windows = "=0.56.0"` with the five approved features. `Cargo.toml` / `Cargo.lock` are not in the reopen diff. |
| Last-used V1 closed | PASS | Assembly still assigns `SoftwareLastUsedEvidence::default()`; 1,293/1,293 native entries match `unknown/no_supported_exact_source`. |
| MSI inventory/manual | PASS | `ordered_eligibility` still refuses every `SoftwareIdentity::Msi` as `msi_execution_not_supported_v1` (or an earlier manual row). Native `msi_non_manual = 0`. |
| Current-user MSIX only | PASS | Adapter still calls `FindPackagesByUserSecurityId` only. Native `msix_non_current_user = 0`; selectable source is exclusively `msix`. |

Worker panic still maps that family to `partial` / `source_worker_panicked` with empty observations; spawn failure falls back in-process. Covered by `source_worker_panic_fallbacks_keep_sibling_sources_intact`.

## Native elapsed (independent recapture)

Command (warmup + five measured reps, new process each time):

```text
software inventory --source all --format json --output <file>
```

Isolated `LOCALAPPDATA` = `evidence/reopen-five-mode-ac4/independent-localappdata` (empty after runs; inventory wrote no audit/home files). Clock: `Stopwatch` from `Process.Start` to exit. p95: nearest-rank `ceil(0.95 * n) - 1` (same as `tools/measure-resources.ps1`). For n=5 that is the maximum sample.

| Rep | elapsed_ms | exit | pid |
| --- | --- | --- | --- |
| warmup | 3881.941 | 0 | 35088 |
| 1 | 3820.682 | 0 | 61740 |
| 2 | 4030.408 | 0 | 28920 |
| 3 | 4253.265 | 0 | 82528 |
| 4 | 4073.429 | 0 | 70848 |
| 5 | 3917.308 | 0 | 82884 |

- **p95 = 4253.265 ms** (target ≤ 5000 ms) — **PASS**
- median = 4030.408 ms
- Unique PIDs: 6/6 (new processes; none reused)
- Binary: `target/release/devsweep.exe` (5,254,656 bytes, sha256 `efbed4371a85a52a73ee324cd8cf790641d0bb6cd66895ef8e431702e03abbc0`)
- Raw: `independent-native-timing.json`, `independent-native-timing.log`

Prior independent five-mode-v1 miss on this host was p95 5205–6481 ms with every later measured rep above 5 s. Implementer reopen recapture was p95 4379.214 ms. This independent recapture is in the same band and under the cap.

## Native document (run 1)

`independent-native-inventory-run1.json`: envelope `outcome=partial`, `command=software.inventory`. 1,293 entries (823 ARP, 291 MSI, 179 MSIX). 8 sources (3 ARP partial, 1 ARP available, 3 MSI available, 1 MSIX current-user available). Size states: 707 available, 1 partial, 585 unknown. Snapshot: `independent-native-contract-snapshot.json`.

## Tests and lint

| Command | Exit | Log |
| --- | --- | --- |
| `rtk cargo test --locked -p devsweep-core -- software` | 0 (38 passed) | `independent-core-software.log` |
| `rtk cargo test --locked -p devsweep-cli -- software` | 0 (13 passed) | `independent-cli-software.log` |
| `rtk git diff --check` | 0 | `independent-git-diff-check.log` |
| `rtk cargo fmt --all -- --check` | 0 | `independent-cargo-fmt-check.log` |
| `rtk cargo build --locked --release -p devsweep-cli --bin devsweep` | 0 (already up to date, 0.23s) | `independent-release-build.log` |

`just ci` was not re-run. The reopen gate list above is the required check surface.

## Findings

None. No Software-crate self-fix. No new failure signature.

## Spec sync

Not required. Overlap is scheduling around the existing `assemble_inventory` / eligibility / last-used / MTA contracts already in `design.md`. Sizing still uses the shared two-worker no-follow walker.

## Source files reviewed

- `crates/devsweep-core/src/software/mod.rs` — overlap ARP/MSI with MSIX on `--source all`
- `crates/devsweep-core/src/software/msix.rs` — size walks after MTA join, queued on existing pool
- `crates/devsweep-core/src/filesystem/sizing.rs` — `estimate_trees_with_budget_and_cancel` on `SIZE_WALK_POOL`
- `crates/devsweep-core/src/filesystem/mod.rs` — re-export
- `crates/devsweep-core/src/software/tests.rs` — panic-fallback isolation
- `crates/devsweep-core/Cargo.toml` — unchanged `windows = "=0.56.0"` features
