# Unify the Tauri and CLI typed service path

## Goal

Make the desktop and CLI surfaces reuse one typed `devsweep-core` service path
without launching an external CLI process, while preserving the safety and
authority boundaries already documented for DevSweep.

## Dependencies and ownership

- Parent task: `09-20-desktop-mole-tauri-performance`.
- This child is the first implementation step and gates the performance child.
- Own the core/service seam, Tauri adapters, CLI wiring, parity fixtures, and
  related contract tests. Do not own React workbench styling or native manual
  acceptance.

## Requirements

### R1. Shared typed service

Clean scan/plan/preview/execute, Analyze, Software, Optimize, and Status must
reach the same typed service/core owner from both CLI and Tauri entry points.
Tauri remains an adapter over `devsweep-core`; it must not import the CLI crate
or spawn `devsweep` as a child process.
Protection/Rules/History retain their corresponding typed core owners.
Presentation settings and window lifecycle remain separate typed non-domain
adapters, as defined by the parent R2; single ownership does not require one
physical IPC file (TPR-04).

### R2. Stable wire and lifecycle contracts

Preserve command names, closed Serde unions, locale-neutral fields, error
codes, event sequencing, operation identity, cancellation/join semantics,
plan/digest authority, and audit metadata. Any intentional schema change must
be versioned in the existing contract path and covered by fixtures.
Distinguish streamed operations from result-only commands. Preserve cooperative
cancel where supported and joined normal completion for Clean dry-run/execute;
the performance child's operation table defines the timing predicates (TPR-06).

### R3. Safety authority

Keep program and argv separate, prohibit shell composition and arbitrary
executable execution, keep permanent delete disabled, and prevent inventory or
preview data from becoming execution authority without a validated explicit
plan, matching digest, explicit confirmation, and existing core policy. CLI
uses a saved plan file and `--confirm`; desktop uses its completed in-memory
plan and second-confirmation dialog. Do not add plan persistence to desktop
solely to make transport representations identical.

### R4. Parity evidence

Add or update deterministic fixtures for stale operation IDs, stale plan
digests, inspect-only paths, cancellation, partial results, unknown values,
unavailable metrics, and successful/failed command completion. The fixtures
must exercise the shared service contract rather than a second mock protocol.

## Acceptance Criteria

- [ ] AC1 (R1): No production Tauri source launches a `devsweep`/CLI child
      process or imports the CLI crate; the adapter uses the shared core owner.
      Domain IPC stays in DesktopBridge; existing typed settings/lifecycle
      exceptions remain separately tested, with no component-level IPC.
- [ ] AC2 (R2, R4): CLI and Tauri parity fixtures agree on core request, event,
      result, error, cancellation and audit semantics; transport-specific
      operation IDs/envelopes are correlated without requiring identical bytes.
- [ ] AC3 (R3): CLI Clean execute requires the saved plan, live `sha256:`
      preview digest and `--confirm`; desktop requires its validated plan,
      matching current digest and second confirmation. Inventory/preview alone
      cannot authorize either path.
- [ ] AC4 (R2, R4): Cooperative cancellation produces a joined terminal result;
      wait-only Clean actions finish before replacement/close. Stale events or
      terminal results cannot replace the active operation. Timing is checked
      against the performance design's operation table, not a global timeout.
- [ ] AC5 (R4): Focused contract tests, read-only generated-type checks, and the canonical
      `just ci` gate pass, with any unavailable native evidence recorded rather
      than inferred.

## Out of scope

- External CLI subprocesses, stdout/NDJSON loopback, binary discovery, and
  process-tree cancellation.
- React workbench layout, visual tokens, and manual Windows acceptance.
- New cleanup capabilities, permanent deletion, Docker cleanup, or provider
  integrations.
