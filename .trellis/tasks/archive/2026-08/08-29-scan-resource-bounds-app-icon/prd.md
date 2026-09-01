# Bound Scan Concurrency and Add App Icon

## Goal

Deliver two independently verifiable improvements: a bounded shared sizing
policy that reduces scan-time resource pressure without an unmeasured speed
loss, and a generated DevSweep icon master plus native package outputs that can
be consumed by the redesigned shell.

This parent owns the source requirement set, child mapping, cross-child gates,
and final integration review. It has no direct product-code implementation.

## Confirmed Background

- The scan slowdown maps to unbounded project-global Rayon use inside the shared
  filesystem-sizing boundary, not multiple desktop scan jobs.
- The icon foundation spans a generated source and Tauri package/window assets.
  React/TUI brand placement moved to the new desktop-shell child after the user
  replaced the earlier UI direction.
- The two deliverables have separate files, tests, rollback paths, and native
  evidence, so the repository's parent/child workflow applies.
- Every pre-existing dirty path outside this task tree's declared change lists
  is unrelated, must be preserved, and is not owned by this task tree.

## Source Requirements

- R1: Reduce scan resource pressure at the shared owner.

Limit filesystem-sizing concurrency for every project/global/rescan/inventory
caller through the shared core boundary. Preserve correctness, cancellation,
budgets, diagnostics, safety, and cleanup authority.

Owner: `08-29-bound-sizing-concurrency`.

- R2: Protect useful scan throughput.

Choose the smallest hard ceiling from the approved candidates using a stable
same-host release A/B protocol. Do not ship a candidate whose median exceeds
the current unbounded baseline by more than 20%.

Owner: `08-29-bound-sizing-concurrency`.

- R3: Create and integrate one generated DevSweep identity.

Use the selected small-size-first generated master as the stable product source,
regenerate the complete Tauri icon set, and require direct native 16/32px
evidence before handing the asset to the redesigned shell.

Owner: `08-29-generated-app-icon-integration`.

- R4: Preserve scope and evidence boundaries.

Do not add dependencies, user-facing concurrency configuration, cleanup
authority, or any shell/UI edit. Preserve unrelated dirt and distinguish
direct Windows/native evidence from macOS/Linux `UNVERIFIED` appearance.

Owner: both children plus parent integration review.

## Child Task Map

| Child | Deliverable | Status | Dependency |
| --- | --- | --- | --- |
| `08-29-bound-sizing-concurrency` | dedicated capped sizing pool, equivalent serial fallback, deterministic tests, timing evidence | in_progress — paused / partially unverified | no product dependency; revised-root approval gates resumption |
| `08-29-generated-app-icon-integration` | stable generated master, complete native icon set, 16/32px evidence, shell handoff contract | planning | none |

The sizing child already has a product diff in
`crates/devsweep-core/src/filesystem/sizing.rs` covering the dedicated pool,
forced-serial mode, root-child budgeting, cancellation, and focused tests. It is
paused because the parent scope changed; its R4 same-host throughput samples,
Measurement Record, full gates, and resulting ceiling acceptance remain
`UNVERIFIED`. After the latest planning summary is explicitly approved, resume
that existing checkpoint without resetting or redoing accepted code. The icon
child remains a new planning task that may start independently after approval.

## Cross-Child Acceptance Criteria

- [ ] AC1 (R1, R2): Every acceptance criterion in
      `08-29-bound-sizing-concurrency/prd.md` is satisfied with recorded test
      and measurement evidence.
- [ ] AC2 (R3): Every acceptance criterion in
      `08-29-generated-app-icon-integration/prd.md` is satisfied, including the
      native small-size stop/go gate.
- [ ] AC3 (R4): A final tree-wide diff review finds no
      dependency, contract, cleanup-authority, task-directory runtime reference,
      or change to any pre-existing dirty path outside this task tree's declared
      change lists.
- [ ] AC4 (R4): Focused child gates, `git diff --check`,
      relevant desktop build checks, and `just ci` pass on the combined tree.
- [ ] AC5 (R4): Both children are completed and archived before this
      parent is archived; the parent itself is never used as the implementation
      target.

## Out of Scope

- User-selectable performance modes or scan-concurrency settings.
- Changes to scan budgets, discovery, ranking, capacity, cleanup, or execution.
- Any React/TUI shell or brand placement; that belongs to
  `08-29-desktop-shell-navigation-brand` under the new parent.
- New dependencies or image-processing tooling.
- Publishing, pushing, or releasing without separate authorization.
