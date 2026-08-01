# Harden scan safety, diagnostics, and capacity reporting

## Goal

Make devsweep's D: drive scan trustworthy and reviewable on Windows: Cloud
Files reparse roots must never be traversed or authorized, scan health must be
visible to CLI and TUI users, displayed capacity must distinguish verified
totals from lower bounds, and high-volume result rows must remain usable.

## Confirmed Facts

- The 2026-08-01 full `D:\` project scan emitted 657 targets and displayed
  688.777 GiB, but 424.119 GiB belongs to 14 incomplete size walks.
- `D:\Documents\LYH` is a Cloud Files reparse root with tag `0x9000701a`.
  The scan descended into it and emitted 34 targets despite the documented
  reparse-point policy.
- The scan emitted 200 Cargo metadata warnings (186 invalid JSON and 14
  exit-101 failures), but JSON plan consumers cannot inspect them.
- `ProcessRunner` retains a bounded output tail, while `query_cargo_metadata`
  parses it without recognizing truncation. Repeated member-manifest probes
  make large workspaces disproportionately expensive.
- `python.__pycache__` accounts for 497 of 657 rows but approximately 50 MiB.
- Cleanup execution can reject a target before dispatch without a durable audit
  record or useful CLI failure detail. The `ccusage` retry reproduced this.
- The volume inventory identifies large non-project roots, but those must not
  become cleanup actions. An old D: pnpm store is a discovery question, not a
  deletion candidate.

## Requirements

### R1: Cloud Files and reparse-point safety

- On Windows, determine whether a path is a reparse point through a no-follow,
  path-level Win32 probe that can expose the reparse tag. Do not rely only on
  `MetadataExt::file_attributes()`.
- Apply this guard consistently to scanner descent, bounded size walks, target
  and marker validation, and ancestor revalidation before execution.
- A reparse point must contribute no descendant candidate or capacity value and
  must cause revalidation to deny execution. Preserve the existing conservative
  behavior on non-Windows platforms.

### R2: Scan health and execution denial observability

- Preserve scan completeness and structured diagnostics through the scanner,
  scan CLI JSON report, and TUI. A diagnostic must identify the affected path,
  stage, outcome, truncation state, and retained versus total process-output
  bytes when a process probe is involved.
- Keep the existing portable cleanup-plan JSON form usable by `clean --plan`.
  A richer scan report must not turn diagnostic data into executable authority.
- Record and report authorization denials before dispatch as durable terminal
  audit events. CLI execution output must identify each failed target and its
  denial reason while keeping JSON output clean.

### R3: Cargo metadata reliability and bounded reuse

- Treat truncated Cargo metadata output as an explicit diagnostic outcome,
  never as a generic invalid-JSON parse failure.
- Cache successful workspace resolutions during one scan by canonical workspace
  and proven member-manifest membership. Do not reuse a parent result for an
  unproven nested manifest; nested independent workspaces must still resolve
  independently.
- Retain a fresh Cargo metadata recheck immediately before every command-backed
  Rust cleanup. Scan caching must not weaken execution-time validation.

### R4: Truthful size reporting and review rescan

- Represent and render verified bytes separately from partial lower-bound bytes
  and preserve target sizing warnings for inspection.
- Keep the default bounded size walk. Permit a higher-budget size rescan only
  for an explicitly reviewed target, using the same reparse and cancellation
  guards as the normal scan.

### R5: High-volume cache presentation

- Group or collapse `__pycache__` entries by owning Python project in the TUI
  and scan-report presentation while retaining every exact target, selection,
  action, and revalidation semantic.
- Do not globally exclude `site-packages`, `.trellis`, or other paths solely to
  reduce row count.

### R6: Read-only capacity inventory and orphan pnpm inspection

- Add a read-only capacity inventory/report surface that uses the safety-aware
  traversal policy and clearly separates storage observations from cleanup
  targets.
- Add an inspect-only orphan-pnpm-store finding only when it can show the
  current pnpm configuration and relevant project references. It must not
  create a cleanup action or default selection.
- Steam, personal media, download folders, recycle-bin contents, and legacy
  stores remain inventory-only. The feature must never add raw deletion,
  recycle-bin emptying, Docker cleanup, or Cargo-home cleanup behavior.

## Acceptance Criteria

- [ ] A Cloud Files-style Windows reparse tag at a scan root, nested child,
  target, marker, or execution ancestor is skipped or denied, with focused
  regression tests for every boundary.
- [ ] CLI and TUI expose complete versus partial scan health, structured Cargo
  probe diagnostics, and explicit pre-dispatch denial results without mixing
  diagnostics into plan JSON stdout.
- [ ] Existing saved cleanup plans remain executable; diagnostic/report fields
  cannot influence plan validation or action reconstruction.
- [ ] Truncated metadata is classified distinctly; cache tests cover repeated
  workspace members and nested independent workspaces; command cleanup still
  makes a live metadata recheck.
- [ ] Verified totals, partial lower bounds, and sizing warnings are separately
  visible; targeted rescans remain bounded and path-safe.
- [ ] `__pycache__` grouping reduces review noise without changing target IDs,
  selected bytes, or executable actions.
- [ ] Inventory and orphan-pnpm outputs are read-only and cannot be selected by
  `clean --plan`.
- [ ] `just ci` passes, command/report contracts have focused tests, and no
  scanner or model layer performs cleanup side effects.

## Out of Scope

- Bypassing Cargo's `CACHEDIR.TAG` protection, fabricating marker files, raw
  deletion, stopping user processes, or emptying the recycle bin.
- Turning inventory findings into cleanup rules or expanding Cargo-home and
  Docker cleanup.
- Changing the user-owned contents currently observed in
  `quanergy_client_rs\\target\\scratch`.

## Planning Status

All repository-answerable facts have been researched. This is a complex task;
`design.md` and `implement.md` are required before activation. No product code
may be edited and `task.py start` may not run until the user approves the final
planning summary.
