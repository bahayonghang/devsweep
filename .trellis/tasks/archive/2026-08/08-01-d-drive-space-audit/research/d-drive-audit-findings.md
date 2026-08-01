# D: Space Audit Findings

Status: read-only audit completed on 2026-08-01. No cleanup command, delete, move, uninstall, recycle-bin operation, or `devsweep clean` invocation occurred.

## Scope And Method

- Scope: the current `D:` volume only. Reparse points were skipped rather than followed.
- Capacity: 1,907.73 GiB total, 1,450.85 GiB used (76.05%), 456.88 GiB free.
- General inventory: a read-only file-length walk of D: roots and selected children. These are logical sizes, not an allocated-space accounting, so they are not added together to reproduce the volume total.
- devsweep evidence: `cargo run --quiet --bin devsweep -- scan --projects --json D:\Documents\Code`. This only generated `devsweep-code-scan-20260801-1516.json`; it did not run `clean` or pass `--execute`.

## Largest Observed Consumers

| Path | Logical size (GiB) | Classification |
| --- | ---: | --- |
| `D:\Documents` | 1,308.97 | Main source of use; mostly code workspaces |
| `D:\SteamLibrary` | 108.81 | Installed games and workshop content |
| `D:\BaiduNetdiskDownload` | 59.21 | User downloads, media, backups, and data |
| `D:\Music` | 35.27 | User media |
| `D:\$RECYCLE.BIN` | 27.52 | Recoverable deleted items until the bin is emptied |
| `D:\.pnpm-store` | 8.77 | Legacy-looking pnpm stores; current pnpm store is on C: |
| `D:\GreenSoftware` | 3.66 | Installed portable software |

`D:\Documents\LYH` is a reparse point and was not followed. `D:\System Volume Information` rejected access, so it is unmeasured rather than zero-sized. The largest code-workspace roots are `Github` (517.41 GiB), `Rust` (267.48 GiB), `CLI` (239.22 GiB), `Agents` (124.50 GiB), and `Tauri` (70.26 GiB).

## Devsweep Results

The valid devsweep scan found 428 project candidates totalling 665.34 GiB of estimated logical size and no duplicate paths.

| Candidate class | Targets | Estimated GiB | Evidence limit |
| --- | ---: | ---: | --- |
| Rust `target/` build artifacts | 33 | 643.49 | 13 total targets across all classes have incomplete size measurements |
| Node `node_modules` | 74 | 14.19 | Medium risk; a project can restore dependencies from its lockfile or registry |
| Python virtual environments | 13 | 7.63 | Medium risk; environment must be recreated before use |
| Python test/tool caches | 308 | 0.03 | Small total; not a meaningful first action |

32 Rust targets use devsweep's registry-owned `cargo clean --manifest-path ... --target-dir ...` action. The remaining Rust target was downgraded to a medium-risk, trash-backed candidate after its Cargo metadata was not usable.

## Approval Candidates

All rows below are candidates only. Nothing is selected for execution by this report. A fresh revalidation and an explicit user selection remain mandatory.

### Priority A: Long-Inactive, Complete Rust Build Artifacts

These 16 low-risk targets have complete size measurements, were all selected by devsweep's default selection policy, and have not been modified for at least 31 days. Their combined estimated size is 101.74 GiB. Cleanup would use `cargo clean`, which removes build outputs and forces a later rebuild; it does not remove source files, but it is not reversible as a byte-for-byte restore.

| Path | GiB | Last modified | Age (days) |
| --- | ---: | --- | ---: |
| `D:\Documents\Code\Tauri\GithubStarsManager\src-tauri\target` | 24.97 | 2026-06-04 | 57 |
| `D:\Documents\Code\Rust\Exp\rt_db\target` | 22.38 | 2025-07-02 | 395 |
| `D:\Documents\Code\Rust\Exp\muxy-rs\target` | 10.48 | 2026-05-23 | 69 |
| `D:\Documents\Code\Rust\Reference\lapce\target` | 9.59 | 2025-04-27 | 461 |
| `D:\Documents\Code\Agents\autoresearch_win\desktop_app\src-tauri\target` | 9.21 | 2026-05-02 | 90 |
| `D:\Documents\Code\Github\herdr\target` | 5.39 | 2026-05-22 | 70 |
| `D:\Documents\Code\CLI\llmtop\target` | 4.84 | 2026-06-05 | 56 |
| `D:\Documents\Code\Agents\skills-janitor\target` | 3.69 | 2026-05-16 | 77 |
| `D:\Documents\Code\Agents\autoresearch_win\target` | 3.28 | 2026-05-02 | 90 |
| Seven smaller complete Rust targets | 7.91 | 2025-04-28 to 2026-04-13 | 109 to 460 |

### Priority B: Rebuildable Dependency Artifacts

64 complete Node/Python candidates older than 30 days total 10.19 GiB. They are medium-risk and trash-backed by devsweep rather than default-selected. Examples include `Web\Reference\flowpilot\node_modules` (2.21 GiB, 255 days) and `Github\industryai_client\.venv` (1.56 GiB, 413 days). They should only be considered after confirming the owning projects are not needed immediately.

### Priority C: Likely Legacy pnpm Store

The current pnpm configuration points to `C:\Users\lyh\scoop\apps\pnpm\current\store`, not D:. A targeted scan of `.npmrc` files under `D:\Documents\Code` returned no positive reference to `D:\.pnpm-store`. The D: directories remain manual-review candidates because a project-specific or external configuration could still exist.

| Path | GiB | Last modified | Recommendation |
| --- | ---: | --- | --- |
| `D:\.pnpm-store\v10` | 6.92 | 2026-06-12 | Verify no live project uses it, then request explicit approval |
| `D:\.pnpm-store\v11` | 1.19 | 2026-06-02 | Same verification and approval gate |
| `D:\.pnpm-store\v3` | 0.66 | 2025-04-25 | Same verification and approval gate |

### Priority D: Recycle Bin

`D:\$RECYCLE.BIN` contains 27.52 GiB. Emptying it is irreversible for these items and is outside devsweep's project cleanup model. It is a reclaim opportunity only if the user expressly authorizes emptying the D: recycle bin.

## Not Recommended Without More Review

| Path or group | Size (GiB) | Reason |
| --- | ---: | --- |
| 13 devsweep targets with incomplete measurement | 422.76 | Size is not reliable enough for a reclaim promise; many are recently active |
| Complete devsweep candidates modified within 7 days | 88.85 | Likely active workspaces, including the current devsweep target |
| `D:\SteamLibrary\steamapps\common\Cyberpunk 2077` | 91.23 | Installed game; use Steam uninstall/move flow, not raw deletion |
| `D:\SteamLibrary\steamapps\workshop` | 11.68 | Workshop data; manage subscriptions through Steam |
| `D:\BaiduNetdiskDownload` | 59.21 | User content; the 19.47 GiB 4K video and 18.50 GiB anime folder need human choice |
| `D:\Music` | 35.27 | User media; no automated cleanup recommendation |

## Approval Contract

This task remains in Trellis planning. Before any cleanup, the user must name the exact candidate rows or paths to consider. The next step must rescan and revalidate those paths, show the exact command-backed or trash-backed action, and wait for a separate execution approval. No broad "clean all" action is authorized by this audit.
