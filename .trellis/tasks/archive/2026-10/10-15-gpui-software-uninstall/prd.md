# C04 Native software inventory and uninstall providers

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C02](../10-15-gpui-windows-action-boundary/prd.md).

## Goal

Extend MSIX-only execution to verified MSI, supported vendor uninstallers and Scoop with durable requery outcomes.

## Requirements

C04-R1. Extend inventory with exact provider identities, versions, publisher, installation time, icon reference and qualified size; deduplicate source aliases without merging unrelated apps.

C04-R2. Preserve current-user MSIX behavior; add Windows Installer product-identity removal, supported native vendor uninstallers and bounded Scoop provider support.

C04-R3. Use vendor uninstall flow and explicit user review; never replace uninstall with installation-directory deletion or pass registry command strings through a shell.

C04-R4. Requery installed state and capture vendor UI pending, reboot-needed, partial, canceled and unknown outcomes. Leftovers are a separate review after proven uninstall success.

## Acceptance criteria

- [x] C04-AC1: MSIX/MSI/ARP/Scoop fixtures cover identity collisions, shared vendor folders, malformed/untrusted commands, missing tool, command drift and reboot transitions.
- [x] C04-AC2: Vendor UI opening or exit code alone cannot report uninstalled; failed/canceled/unknown removal does not trigger leftovers.
- [x] C04-AC3: Isolated test applications exercise each executable provider in a disposable Windows environment; unsupported native evidence stays NOT_RUN. Host-installed MSI, vendor, and Scoop products were not used, and real MSIX removal remains NOT_RUN. Evidence: `disposable_provider_executables_run_isolated_msi_vendor_and_scoop` exited 0 (`C:\Users\lyh\AppData\Local\Temp\devsweep-c04-providers.txt`).

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
