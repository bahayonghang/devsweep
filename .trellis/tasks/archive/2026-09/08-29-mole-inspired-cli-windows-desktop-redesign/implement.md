# Implement - Five-Mode Programme Orchestration

This parent is a planning and integration gate, not a product-code target. Do
not run `task.py start` on it. No child may begin or resume from this revision
turn; first present the complete revised task-tree summary and obtain a later,
explicit implementation approval.

## 1. Planning and approval gate

1. Validate the root and all recursive child PRDs, designs, implementation
   plans, and manifests.
2. Confirm the independent planning review has no unresolved blocker and record
   any remaining native, external-document, or measurement evidence as
   `UNVERIFIED`.
3. Present one combined programme summary. Approval applies only to that exact
   revision and does not implicitly start coordination parents.

## 2. Dependency-ordered child delivery

1. Freeze `08-29-cli-contract-localization`; resume the existing paused sizing
   checkpoint only when separately authorized, and complete the icon foundation
   before shell brand integration.
2. Complete `08-29-desktop-shell-navigation-brand`, including its spec-first
   palette/motif/localization contract, before any mode presentation task.
3. Deliver Clean and Analyze; within Analyze, core/IPC precedes TUI/Desktop.
4. Deliver Software in order: inventory/plan, execution/audit, then
   CLI/TUI/Desktop/native presentation.
5. Deliver Optimize in order: catalogue/execution with the accepted OS-build
   mechanism, then CLI/TUI/Desktop/native presentation.
6. Deliver Status in order: collector/CLI, then TUI/Desktop/native presentation.
7. Deliver Protection/Rules/History only after the Clean, Software, and Optimize
   audit schemas and shell support registry are accepted.
8. Run `08-29-five-mode-native-integration` only after every preceding focused
   gate, schema handoff, generated binding, and explicit child approval passes.

Each product defect returns to its owning child. The integration child may edit
only shared glue/documentation declared in its plan and may not absorb domain
logic or weaken a failed threshold.

Lifecycle semantics are leaf-first. After an implementation leaf passes its
check and focused gate, commit only that leaf's product changes and immediately
archive it. Coordination parents perform their acceptance reviews from commit
history and `.trellis/tasks/archive/`; a dependency on a coordination parent
means that parent's task-owned `acceptance.md` records overall `PASS`, not that
the parent is already archived. The five required records are
`08-29-scan-resource-bounds-app-icon/acceptance.md`,
`08-29-analyze-mode/acceptance.md`, `08-29-software-mode/acceptance.md`,
`08-29-optimize-mode/acceptance.md`, and
`08-29-status-mode/acceptance.md`, all below `.trellis/tasks/` while active.
`08-29-five-mode-native-integration` completes after all preceding
implementation leaves and those parent acceptance gates pass, then the
remaining coordination parents and this root are archived by hierarchy.

## 3. Parent acceptance review

Before closeout, trace every root AC to child evidence and verify command removal,
domain-specific authority, bilingual catalogues, native accessibility/scaling,
cross-mode cancellation/join, resource thresholds, no-UAC behavior, provenance,
and preservation of every pre-existing out-of-scope dirty path. Automated tests
do not convert missing native or external evidence into PASS.

## 4. Rollback and closeout

- Stop the programme on an unresolved child blocker, ownership collision,
  schema mismatch, unapproved dependency/scope change, or failed safety/native
  gate. Roll back only through the owning child's documented boundary.
- Archive each accepted implementation leaf immediately after its product
  commit. After final integration passes, archive the remaining coordination
  parents by hierarchy, then this root.
- Do not push, publish, sign, package for release, or release without separate
  authorization.
