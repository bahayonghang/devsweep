# Backend Contract Routing For The Architecture Refactor

## Purpose

`.trellis/spec/backend/quality-guidelines.md` is larger than the Trellis context
injection limit. This routing note prevents silent loss of its later scenarios.
It is not a replacement for the spec: implementation and check agents must open
the full quality guideline and the scenario ranges assigned below before
editing or accepting a child.

## Universal Contracts

Read the full forbidden/required patterns and final review sections at
`.trellis/spec/backend/quality-guidelines.md:15` and
`.trellis/spec/backend/quality-guidelines.md:900`.

- Scanner, model, plan, rule, ranking, and inventory code cannot execute
  cleanup commands, move paths to trash, or delete files.
- Saved JSON carries observed facts and typed intent, never authoritative
  program/argv or an executable action supplied by the file.
- Command execution keeps program and argv separate and never uses a shell
  command string.
- `SafetyPolicy::authorize` remains the sole live pre-side-effect authorization
  funnel.
- Permanent delete remains disabled. Cargo home remains inspect-only. Docker
  cleanup is not introduced.
- Machine-readable JSON uses stdout; tracing/diagnostics use stderr.
- Incomplete/failed sizing is never converted to a trusted zero-byte result.
- `just ci` is the final local gate.

## Scenario Routing

### Child 1: Core Contracts And Rules

Read these complete scenarios:

- Declarative plan trust boundary: lines 215-313.
- Declarative rule catalogue: lines 418-531.
- Project scanner and JSON cleanup report: lines 696-801.
- Scan safety, review reports, and capacity inventory: lines 802-899.

Review plan opacity, canonical digesting, registry reconstruction, rule
ownership, scanner non-mutation, and report/diagnostic compatibility.

### Child 2: Backend Runtime And Discovery

Read these complete scenarios:

- Execution engine and audit log: lines 113-214.
- Declarative plan trust boundary: lines 215-313.
- Global cache providers: lines 314-417.
- Cleanup plan ranking and freshness guard: lines 602-695.
- Project scanner and JSON cleanup report: lines 696-801.
- Scan safety, review reports, and capacity inventory: lines 802-899.

Review explicit selection, authorization/audit ordering, cancellation, bounded
process execution, single-pass ranking, path safety, partial health, reviewed
rescan, and read-only inventory.

### Child 3: TUI Internals

The frontend specs are primary. Also read the plan trust, ranking, and scan
safety scenarios above whenever moving confirmation, staged scan, inventory, or
executor request integration.

Review frozen confirmation identity, shared ranking, inventory isolation, and
truthful cancellation completion.

### Child 4: Entrypoint And Final Integration

Read these complete scenarios and the full final review checklist:

- Foundation CLI and command entrypoints: lines 28-112.
- Execution engine and audit log: lines 113-214.
- CI and release archive: lines 532-601.
- Testing requirements and code review checklist: lines 900-end.

Review command compatibility, dry-run/JSON/error behavior, final integration,
generated-output exclusions, and every parent safety contract.

## Verification Rule

An implementation or check agent receiving this file must still use a
repository read command to open the assigned ranges from the full quality
guideline. This routing summary is not proof that moved code complies.

