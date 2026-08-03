# Technical Design

## Boundaries

The implementation keeps three distinct data flows:

1. Scanner: read-only discovery produces a typed `ScanOutcome` with a cleanup
   plan, completeness, diagnostics, and size observations. It never invokes an
   executor or trash API.
2. Reporting: CLI and TUI project the scan outcome into human or report JSON
   views. Diagnostics are observational and are not accepted as action inputs.
3. Execution: plan validation reconstructs trusted actions from rule and
   intent. `SafetyPolicy` rechecks live paths and Cargo metadata immediately
   before dispatch; `Executor` durably records every terminal outcome.

## Reparse Probe

Introduce one platform-aware path probe owned by the filesystem safety module.
On Windows it opens the path with no-follow/reparse semantics and obtains the
reparse tag from Win32. The result distinguishes a normal path, a known
reparse tag, an unsupported query, and an I/O failure. Conservative callers
will not descend or authorize a path when the result is reparse or cannot be
verified. Existing Unix symlink behavior remains unchanged.

All current uses of `is_unsafe_link(&Metadata)` become path-aware checks where
the caller has a path: scanner directory descent, `fs_size` child visits,
marker validation, target validation, and ancestor checks. Metadata-only code
may remain as a fast prefilter but cannot be the final Windows decision.

## Scan Report Contract

Keep `UntrustedPlan` as the portable input accepted by `clean --plan`. Add a
separate versioned `ScanReport` for scan JSON and TUI worker results:

```text
ScanReport
  plan: UntrustedPlan
  health: ScanHealth
    completeness: complete | partial
    diagnostics: [ScanDiagnostic]
    totals: verified_bytes, partial_lower_bound_bytes, unknown_target_count
```

`scan --json` emits the report document; `clean --plan` accepts either the
embedded plan exported from the report or a legacy plan-only document. The
decoder must reject arbitrary executable fields and treat report diagnostics as
non-authoritative. The CLI retains stdout exclusively for JSON in JSON mode and
renders a concise health summary to stderr or normal text mode.

`ScanDiagnostic` gains a typed stage and process-probe detail rather than a
preformatted warning string. For bounded process output, record status,
stdout/stderr truncation, retained bytes, and total bytes. Sanitized human text
remains a presentation projection.

## Cargo Workspace Cache

Create a scan-lifetime `CargoWorkspaceCache` at the project scanner boundary.
The first successful metadata resolution records the canonical workspace root,
target directory, and every member manifest path returned by Cargo. A later
manifest reuses that result only when it is in the recorded member set. An
unseen nested manifest is probed independently and can establish a second
workspace entry. Cache both success and typed non-success diagnostics only for
the exact manifest, with bounded entry count and no cross-scan persistence.

`query_cargo_metadata` returns a typed result that checks output truncation
before JSON parsing. `SafetyPolicy::recheck_cargo_target_scope` deliberately
does not use this cache: it performs a fresh metadata query directly before a
Cargo command receives authority.

## Size And TUI Projections

Extend scan observations rather than inventing separate UI-only numbers. A
target keeps its existing `estimated_bytes` and `size_complete` compatibility
fields, plus typed sizing warnings where available. Aggregate helpers return:

- verified total: only complete target observations;
- partial lower bound: sum of incomplete observed bytes;
- unknown count: incomplete observations without a usable lower bound.

The normal estimator retains its depth and entry budgets. A target-specific
rescan accepts an explicit target ID/path selected from the current plan and a
bounded larger budget. It reuses the same no-follow probe and cancellation
path, then replaces only that target's observation in the report state.

The TUI owns a derived display grouping of `python.__pycache__` rows by
nearest detected Python project marker. Grouping affects navigation and visual
collapse only; `TargetId` selection and the underlying `CleanupPlan` remain
unchanged.

## Inventory And Pnpm Finding

Add an inventory command/report model independent of cleanup plans. It reports
top-level capacity observations, scan health, and explicit classification such
as `inventory_only` or `inspect_only`. It has no `CleanupIntent`, does not feed
the executor, and is never decoded by `clean --plan`.

The orphan-pnpm inspection compares `pnpm store path` with candidate stores and
searches configured/project references. A result is surfaced only with its
evidence and remains inspect-only even if it appears unused.

## Compatibility And Rollback

- Legacy plan-only JSON remains supported by the clean decoder and retains the
  existing declarative action reconstruction path.
- New report fields are never consulted for execution authorization.
- The changes add no persistent cache. Disabling report/inventory use leaves
  the existing plan and cleanup workflow intact.
- If a new Win32 probe cannot verify a path, the safe rollback is to skip or
  deny that path, not to fall back to metadata-only traversal.
