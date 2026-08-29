# Implementation Plan: continuous desktop scan previews

## Preconditions

- Keep the task in `planning` until the user explicitly approves the latest
  planning summary in a subsequent message; only then run `task.py start`.
- Run `trellis-before-dev` and reload backend, desktop-frontend, frontend/TUI, and
  cross-layer specs before the first code edit.
- Record and preserve the existing unrelated `README.md` and `justfile` changes.
- Do not add a production dependency, alter cleanup authority, or change report/
  plan versions without new approval and a planning rollback.

## Ordered Implementation

### 1. Establish the focused baseline

- Re-read `CONTEXT.md`, ADR-0001, this task's PRD/design/research, and the current
  scan/TUI/desktop files because line numbers may drift.
- Run the existing focused core scan, TUI staged-progress, Tauri scan, desktop API,
  reducer, and App tests. Record passed/failed/skipped evidence in task research;
  do not treat the planning-time structural validator as product proof.
- Confirm generated fixtures/types are initially in sync.

**Gate:** baseline failures are classified before feature edits; unrelated failures
are reported and preserved rather than repaired silently.

### 2. Extend core continuous progress without changing final output

- Add a core-owned purpose-built `ScanPreviewSnapshot`/`ScanPreviewTarget`
  observation DTO. It may reuse canonical validation helpers but must omit plan
  version, cleanup intent, default selection, and trusted `CleanAction` fields.
- Introduce private project/global semantic checkpoint callbacks so fully
  constructed targets can reach `Sweeper` before phase completion.
- Keep `Sweeper` as the only cumulative merge/dedupe/order owner; emit replacement
  snapshots, never consumer-owned deltas.
- Add a latest-preserving bounded emission policy with immediate first preview and
  forced phase/cancel/terminal flush.
- Thread cancellation through ordinary project target sizing, and return an
  explicit completed-versus-canceled terminal kind instead of inferring cancel
  from partial health after project or global work.

**Focused checks:** core unit tests for in-phase snapshot sequence, cumulative
dedupe/order, safe serialization, emission cap/flush, projection failure, and
project/global cancellation; existing final report/ranking tests unchanged.

**Rollback point:** if continuous checkpoints require scanner duplication or change
final target semantics, revert this slice and return to planning rather than
shipping phase-checkpoint behavior under the approved requirement.

### 3. Preserve CLI/TUI parity

- Keep non-interactive CLI output/final JSON untouched and add regression evidence
  if changed progress plumbing reaches command tests.
- Forward richer cumulative snapshots through the existing TUI worker/job protocol.
- While a TUI scan job is active, render staged targets as preview-only and block
  selection, dry-run, and cleanup confirmation; promote selection defaults only
  from `ScanFinished`.
- Verify latest-job guards, replacement-not-append behavior, cancellation, and
  final snapshot parity under multiple same-phase events.

**Focused checks:** TUI app/runtime/render tests covering multiple project-phase
updates and an older job racing a newer scan; CLI scan fixtures/output tests.

### 4. Introduce the safe command-scoped Tauri stream

- Add an opaque frontend-provided scan id to `scan_start` and store/forward it only
  as correlation metadata.
- Create the desktop-specific progress DTO with monotonic sequence, phase, message,
  and optional safe preview; deliver it over a per-command Tauri `Channel` and stop
  serializing core `ScanProgress` through the application-global event bus.
- Integrate bounded/latest-preserving forwarding and force terminal flush before the
  final command result becomes observable.
- Return a correlated explicit completed/canceled terminal kind; keep partial-health
  completion distinct. Preserve single-flight reservation, id-scoped idempotent
  cancel, coordinator release, and structured projection/scan errors.

**Focused checks:** Tauri tests for id/sequence monotonicity, command/channel
correlation, safe non-plan serialized shape, no program/argv/cwd/direct action/
intent/default-selection fields, coalescing/flush, explicit cancellation,
concurrent scan rejection, stale cancel ids, and projection error handling.

### 5. Regenerate and harden the TypeScript IPC boundary

- Add controlled fixtures for zero, project-only, mixed project/global, partial
  capacity, long-path, and inspect-only previews.
- Update `contract-variants.json` and the generator reference graph for
  `DesktopScanProgress`/`ScanPreview`; regenerate `types.gen.ts` through
  `npm run types:generate` only.
- Extend the strict runtime decoder and bridge signature for scan id/sequence and
  command-scoped channel delivery plus closed-world preview validation.
- Prove authority-bearing, unknown, unsafe-integer, stale-shape, and malformed
  payloads fail closed.

**Focused checks:** type-generation determinism, contract decoder tests, bridge
command/channel argument/lifecycle tests, and fixture bridge compilation.

### 6. Implement reducer-owned preview lifecycle

- Add separate active scan, stopped preview, and retained completed-report state,
  with active scan id, last sequence, and distinct completed/canceled/failed actions.
- Apply only matching monotonic progress; replace cumulative preview snapshots.
- Preserve preview on cancel request, explicit cancellation, and scan failure, but keep it outside `scan`,
  `selectedIds`, dry-run, confirmation, and execution selectors.
- Keep the previous completed report during a rescan as an unmixed fallback. On
  matching completed success, atomically clear preview identity, replace that report,
  and derive selection from final executable targets. Ignore late progress or
  terminal results from older ids.

**Focused checks:** reducer transition table for start, multiple progress snapshots,
duplicate/out-of-order/stale events, explicit cancel, failure retention, rescan
fallback preservation, empty final, partial-health final, non-empty final, and
existing dry-run/digest gates.

### 7. Build the active-scan workbench UI

- Reshape `ScanPage` into stable scope/action controls plus the requested-phase
  progress rail and labelled indeterminate progress element.
- Add active/stopped preview presentation with `Discovered so far`, scope groups,
  ecosystem counts, exact capacity confidence, risk/evidence, and current backend
  copy; do not expose cleanup intent or default-selection semantics.
- Refactor `TargetTable` with a discriminated preview/review prop contract so
  selection controls cannot exist in preview mode; preserve the existing dense row,
  evidence, risk, inspect-only, and action semantics.
- Give starting, cancel-requested, failed/canceled preview, final empty, partial
  final, and complete final states distinct copy.
- Update CSS within the incumbent workbench visual system, including 800×600,
  390×844, long paths, contained table overflow, focus-visible, and reduced motion.

**Focused checks:** App/component tests for all states, category counts, missing
preview selection/dry-run controls, progress/aria semantics, keyboard operation, and
narrow layout DOM invariants.

### 8. Update controlled fixture and durable documentation

- Extend `fixtureBridge` to emit correlated multi-snapshot project/global progress,
  stale/out-of-order negative cases, explicit cancellation with retained preview,
  partial-health completion, failed-rescan fallback, and final replacement.
- Update English and Chinese scan/desktop documentation only where user-visible
  behavior or developer verification commands changed.
- Update `.trellis/spec/desktop-frontend/` and TUI/backend specs if implementation
  establishes a durable continuous-preview contract beyond existing phase snapshots.
- Run the Impeccable mechanical detector once over changed UI targets after the UI
  is complete; address applicable findings in one bounded batch.

### 9. Verification and review

Run the smallest gates first, then the full tree:

```powershell
# desktop frontend
mise exec node@22 -- npm --prefix desktop run types:generate
mise exec node@22 -- npm --prefix desktop run lint
mise exec node@22 -- npm --prefix desktop run typecheck
mise exec node@22 -- npm --prefix desktop run test
mise exec node@22 -- npm --prefix desktop run build

# focused Rust packages/tests chosen from the final diff
cargo test -p devsweep-core
cargo test -p devsweep-cli --lib
cargo test -p devsweep-desktop --lib

# repository gate
git diff --check
just ci
```

- Use the controlled fixture for one combined visual pass at 800×600 and 390×844,
  covering starting, live rows, cancel requested, stopped preview, empty final, and
  result final. Fix defects in one batch and confirm once.
- Separately record native Tauri IPC/window evidence when available. Do not promote
  browser fixture evidence to native proof.
- Review the complete diff for authority leakage, duplicate projection ownership,
  event-order races, selection/digest regression, event volume, unrelated dirt, and
  generated-file drift.

## Completion Gate

- Every PRD acceptance criterion has concrete automated or explicitly manual
  evidence.
- No preview path reaches dry-run/execution authority; final report semantics and
  CLI output remain compatible.
- Focused and full gates pass, or failures/skips are reported without overstating
  completion.
- Commit/archive/journal steps require the user's later implementation and closeout
  authority; do not push unless separately requested.
