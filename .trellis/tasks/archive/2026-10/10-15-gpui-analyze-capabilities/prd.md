# C07 Disk analysis explanations and safe actions

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md).

## Goal

Complete CLI analysis navigation exports, storage explanations, lock ownership and reviewed recycle operations.

## Requirements

C07-R1. Expose drive/root discovery, bounded snapshot/drill-down/top-items views and machine exports; preserve no-follow traversal and partial coverage.

C07-R2. Add explainable classifications for hibernation/paging, WSL/Docker virtual disks, AI models, backups, games and cloud placeholders without inferring cleanup eligibility from size.

C07-R3. Expose existing reveal and separate preview/execute Recycle Bin actions through CLI using snapshot/node identities and live revalidation.

C07-R4. Report supported file-lock owner evidence without auto-terminating processes; reconcile logical versus allocated size and excluded/unreadable content where available.

## Acceptance criteria

- [x] C07-AC1: Permission, junction cycle, deep/large tree, sparse/hardlinked data, Unicode and cancellation fixtures preserve bounded memory and truthful totals.
- [x] C07-AC2: Root and node identity changes refuse recycle; protected/system/shared locations stay read-only and scan code creates no action authority. Node replacement remains tested. `root_identity_change_refuses_preview_and_keeps_the_replacement` exited 0 and returned `ChangedSinceSnapshot(0)`. `shared_public_file_is_refused_as_profile_root` exited 0 and returned `ProfileRoot` for `users/Public/shared.txt` without calling the trash runner.
- [x] C07-AC3: Large-item classification has exact evidence and unknown fallback; a WSL disk or model is never labeled disposable because of its size.
- [x] C07-AC4: CLI and shared service reports agree; render-ready projections remain separate from discovery and execution.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
