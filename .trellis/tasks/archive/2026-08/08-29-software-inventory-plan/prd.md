# Implement Software inventory and preview plans

## Goal

Own tagged ARP/MSI/MSIX inventory identities, immutable fingerprints, untrusted selections, preview digests, protected/manual states, and hostile-fixture tests.

## Requirements

- R1: Enumerate ARP entries from HKCU/HKLM and both registry views, authoritative
  Windows Installer products in current-user/machine contexts, and current-user
  MSIX packages. Each source returns available/partial/permission/unsupported and
  does not erase other sources on failure.
- R2: Use tagged identities: registry hive/view/subkey, MSI product code plus
  context, or MSIX package full name. Deduplicate only by an authoritative exact
  identity relation; never by display name, publisher, install path, or icon.
- R3: Treat all registry strings as untrusted display observations. Never parse,
  log, serialize, or execute `UninstallString`, `QuietUninstallString`, or
  `DisplayIcon`. Unknown related data remains unknown; no leftover paths.
- R4: Mark only a healthy, non-framework/resource/dependency/stub,
  non-conflicting current-user MSIX main package selectable. Every MSI context
  and every registry-only record is inventory/manual in V1. `NoRemove`, hidden,
  system/update, protected, unsupported, unhealthy, conflicting, and
  source-incomplete evidence maps to the deterministic refusal matrix in
  `design.md`; presentations never infer eligibility.
- R5: Build a versioned immutable inventory fingerprint, untrusted exact-id
  selection plan, live revalidation, and preview digest containing only opaque
  validated strategy tokens.
- R6: Use the selected Windows-only direct dependency
  `windows = =0.56.0` with features `ApplicationModel`, `Foundation`,
  `Foundation_Collections`, `Management_Deployment`, and
  `Win32_System_WinRT`. It is already locked transitively, is Microsoft
  `MIT OR Apache-2.0`, declares Rust 1.62, and has a 10,807,828-byte crate
  archive. A dedicated MTA thread initializes WinRT, enumerates the exact current
  process SID through PackageManager, and returns source-partial evidence on API
  failure. No PowerShell/winget/raw generated binding fallback is permitted.
- R7: Implement typed optional size and last-used evidence. ARP/MSI
  `EstimatedSize` is a labelled reported estimate converted from KiB to bytes
  with checked arithmetic; current-user MSIX size uses a bounded, cancelable,
  no-follow walk of the exact API-returned installed path and may be a truthful
  lower bound. Last-used implements the required field/column as the closed V1
  value `unknown/no_supported_exact_source`; adding an available timestamp needs
  a later schema revision backed by a documented exact-identity provider.
  `InstallDate`, MSIX
  `InstalledDate`, UserAssist, event logs, telemetry, and fuzzy executable/name
  matching are not last-used evidence.

## Acceptance Criteria

- [ ] AC1 (R1, R2, R3): Hostile registry fixtures with quoted commands, metacharacters,
      environment variables, malformed GUIDs, duplicate names, huge strings,
      invalid UTF-16, and denied keys cannot enter plans/logs/argv.
- [ ] AC2 (R1, R2, R4, R6): 32/64 and user/machine/MSIX fixtures produce stable tagged identities,
      the exact ordered eligibility/refusal matrix, deterministic fingerprints,
      and source-partial evidence without fuzzy merging; every MSI is manual and
      only qualifying current-user MSIX is selectable.
- [ ] AC3 (R4, R5): Removed/changed/reinstalled packages, inventory expiry, selection
      changes, protected entries, and digest mismatch fail closed.
- [ ] AC4 (R3, R4, R5, R6): Preview plans contain no command line or guessed path, machine scope is
      unselectable, schema fixtures are locale-invariant, and focused/native
      inventory plus `just ci` gates pass.
- [ ] AC5 (R2, R7): Wire fixtures distinguish reported estimate, measured,
      measured lower-bound, and unknown size evidence in integer bytes, and
      every V1 last-used field is exactly
      `unknown/no_supported_exact_source`. No install/update
      date or local heuristic appears as last-used, and installed paths never
      leave the core adapter.

## Out of Scope

- Execution, UI, Win32_Product WMI, winget as authority, other-user packages,
  heuristic leftovers, registry command adapters, Windows App SDK, or any
  dependency/version/feature outside the exact selected projection above.
- UserAssist/event-log/telemetry usage inference, background launch monitoring,
  and relabelling install/update timestamps as last-used.
