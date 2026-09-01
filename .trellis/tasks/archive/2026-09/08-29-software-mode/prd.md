# Deliver Windows Software mode

## Goal

Own safe Windows application inventory, preview, current-user MSIX-only uninstall authorization, audit, CLI/TUI/Desktop surfaces, and native evidence; every MSI remains inventory/manual in V1.

## Requirements

- R1: Deliver a separate Windows Software domain from tagged inventory through
  saved selection plan, preview digest, current-user MSIX-only execution, post-
  state requery, audit, CLI/TUI/Desktop, and native no-UAC evidence.
- R2: Registry-only vendor entries and every MSI context (current-user managed,
  current-user unmanaged, and machine) are visible manual/unselectable records.
  Vendor uninstall strings, display icons, fuzzy names, publishers, and guessed
  leftover paths never become authority or serialized plan data.
- R3: Complete inventory/plan before execution/audit, then presentation/native.
  The umbrella owns contract handoffs, dependency gates, cross-child fixtures,
  and rollback only.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3): Recursive children pass hostile inventory, identity/fingerprint,
      stale preview, all-MSI/manual and protected refusal, MSIX dispatch,
      cancellation, reboot, the closed five-terminal outcome/restart recovery,
      partial-source evidence, audit, and no-UAC criteria.
- [ ] AC2 (R1, R3): The same fixture yields matching CLI/TUI/Desktop identities, selection
      eligibility, reason codes, and locale-invariant machine documents.
- [ ] AC3 (R1, R2): No plan/audit/log contains ARP uninstall command strings or guessed
      leftovers; no Software path constructs CleanupPlan or requests elevation.

## Out of Scope

- Any MSI uninstall, machine-scope uninstall, other-user packages, vendor command execution,
  heuristic leftovers, app repair/update/startup management, or administrator mode.
