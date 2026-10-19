# C03 Windows cleanup catalogue and CLI coverage

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C02](../10-15-gpui-windows-action-boundary/prd.md).

## Goal

Expand evidence-backed Windows cleanup coverage while preserving developer-cache ownership and retained-data safety.

## Requirements

C03-R1. Retain project artifacts, provider-managed caches, root preferences, project occupancy, warnings and the six existing Windows cache families.

C03-R2. Add a finite reviewed catalogue for shader/application caches, Windows temp/error-report/download-cache candidates and supported AI-tool caches; every exact rule needs ownership, age/use, regeneration-cost and retained-data evidence.

C03-R3. Support protection/ignore review, grouped totals, explicit personal-data exclusions, full-report filtering and separately selected privileged rows.

C03-R4. Keep Cargo home and Docker cleanup forbidden; local models, sessions, credentials, offline content and virtual disks are not ordinary cache. System-managed candidates without permitted execution remain explanation/Settings handoff with reason.

## Acceptance criteria

- [x] C03-AC1: Each proposed rule has positive, adjacent-personal-data negative, busy/unknown owner, reparse/hardlink/cloud and stale-preview cases through the production scan-to-plan path. Other AC1 clauses were already recorded in implement.md. `user_temp_foreign_sid_is_ownership_unknown` closes the user-temp unknown-owner gap. A real cloud-files reparse tag stays NOT_RUN. The Recycle Bin clause is now covered under C03-AC3 by a real measurement.
- [x] C03-AC2: Global and project scans remain independently useful; partial provider failure never erases healthy findings or reports unreadable as empty.
- [x] C03-AC3: No permanent file deletion fallback; estimated occupancy, moved-to-Recycle-Bin and verified freed bytes remain separate. Isolated rename evidence: `verified_freed_bytes_after_isolated_rename` exited 0 and measured the renamed file length after the source disappeared.

  **Real Recycle Bin clause now measured on 2026-10-18.** Through the production CLI flow — `clean scan --root <isolated fixture> --scope projects`, `clean plan --select`, `clean preview` (live digest `sha256:6bcee2244ad34fcc217ec2aa6ee41fb1b56a96304860b980fa03cbadbee6ad9f`), then `clean execute --preview-digest <digest> --confirm` — every step exited 0, the source path was gone afterwards, and the shell Recycle Bin item count rose from 201 to 202. That is a real move to the Recycle Bin, not a rename and not an in-place delete. The fixture was a Python project containing `.pytest_cache` with a 4098-byte payload inside a fresh temp directory, with `LOCALAPPDATA`/`APPDATA`/`XDG_CONFIG_HOME` pointed at an empty temp home. No real user cache, journal or protection list was read or written. Evidence: `evidence/c03-recycle-round3.md`.

  The separate "verified freed bytes" quantity is still not derived from this run: a Recycle Bin move does not free physical space until the bin is emptied, and the product deliberately reports moved bytes rather than claiming reclaimed space.
- [x] C03-AC4: A checked-in rule/coverage manifest states exact supported families and omissions; no unsupported claim of matching Mole's advertised rule count.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
