# Deliver bounded Windows Optimize mode

## Goal

Own the closed non-elevated maintenance catalogue, preview and execution policy, Settings handoffs, audit, CLI/TUI/Desktop surfaces, and native evidence.

## Requirements

- R1: Deliver a separate non-elevated Optimize domain with a closed versioned
  catalogue, plan/preview digest, authorizer, fixed execution/Settings handoff,
  durable audit, CLI/TUI/Desktop, and native no-UAC evidence.
- R2: Executable scope is only fixed DNS cache refresh. Storage recommendations,
  Search, and supported Energy pages are allowlisted Windows Settings handoffs;
  drive optimization, integrity/filesystem checks, and network reset are guidance
  only. Every other tweak/command is absent or explicitly rejected.
- R3: Complete catalogue/execution before presentation. The umbrella owns
  cross-child contract parity, recursive evidence, and rollback only.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3): Recursive children prove catalogue closure, preview/execute identity,
      stale/digest/refusal behavior, one operation, audit lock, cancellation,
      timeout/unknown, unsupported OS, fixed System32 process, and no UAC.
- [ ] AC2 (R1, R3): CLI/TUI/Desktop present identical operation ids, action classes,
      capability/refusal reasons, outcomes, and locale-invariant schemas.
- [ ] AC3 (R2): No path changes security/UAC/firewall/update/service/registry/power/
      pagefile/hibernation or accepts user program/argv/script input.

## Out of Scope

- Administrator helper, broad repair suite, remote management, automatic tuning,
  arbitrary commands, or treating Settings handoff as completed maintenance.
