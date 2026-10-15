# C05 Software updates startup and leftovers

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C02](../10-15-gpui-windows-action-boundary/prd.md), [C04](../10-15-gpui-software-uninstall/prd.md).

## Goal

Complete CLI-first package updates, reversible startup controls and ownership-based leftover review.

## Requirements

C05-R1. Upgrade the existing read-only winget check to explicitly selected package/version/source update plans; keep check separate from network download/installation.

C05-R2. Expose and strengthen startup enable/disable with live identity, protection for security/network/device-management entries, backup/read-back and supported restore.

C05-R3. Extend leftover discovery to removed apps only where exact ownership evidence exists; personal data starts unselected and shared folders remain protected.

C05-R4. Handle registry residue only for a finite approved key set with backup and conditional restore. Never add a registry cleaner based on names or vendor prefixes.

## Acceptance criteria

- [ ] C05-AC1: Ambiguous package match, unsupported source, network failure, changed offered version and restart-required outcomes are explicit.
- [ ] C05-AC2: Startup restore refuses to overwrite externally changed values; mutation tests run under isolated test registry roots.
- [ ] C05-AC3: Failure/cancel of uninstall retains all leftovers; files use Recycle Bin and registry modifications have verifiable backups and audit entries.
- [ ] C05-AC4: CLI human/JSON and service/desktop parity covers updates, startup and leftovers without unsupported bulk selection.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
