# Current DevSweep Capability and Task Checkpoint

## Current Product Contracts

- CLI definitions: `crates/devsweep-cli/src/application/cli.rs:14-99`.
- CLI handlers and saved-plan boundary:
  `crates/devsweep-cli/src/application/commands.rs:17-149`.
- Safety model: `docs/guide/safety-model.md:3-49`.
- Desktop Tauri registration: `desktop/src-tauri/src/lib.rs:7-16`.
- Desktop workflow composition: `desktop/src/App.tsx:24-98`.
- Desktop state ownership: `desktop/src/state/app-state.ts`.
- Desktop presentation/safety checklist:
  `.trellis/spec/desktop-frontend/index.md:7-62`.

Current capability-backed user modes:

1. cleanup scan/review/dry-run/confirmation/execution;
2. read-only capacity inventory in core/CLI;
3. persistent protection lists in core/CLI/Tauri;
4. read-only rule catalogue in core/CLI.

Missing end-to-end contracts include Windows application inventory/uninstall,
system maintenance/optimization, installer discovery, system metrics/status,
audit-history query, desktop inventory IPC, and desktop rules IPC.

## Existing Task-Tree Conflict

The old parent excludes CLI and desktop redesign
(`../08-29-scan-resource-bounds-app-icon/prd.md:50-54,87-92`). The icon child
owns `App.tsx`, `styles.css`, and header behavior while also excluding redesign
(`../08-29-generated-app-icon-integration/prd.md:45-50,93-99`). These ownership
rules must be narrowed before a desktop-shell child can be implementation-ready.

The old parent is now a child of this umbrella so its history and generated icon
research remain intact.

## Paused Sizing Implementation

`08-29-bound-sizing-concurrency` remains `in_progress` with task metadata:

- `implementation_state = paused_for_parent_redesign`;
- `approval_state = invalidated_by_scope_change`;
- `verification_state = partial_unverified`.

Known evidence before the pause:

- a pre-change release baseline was preserved under ignored build output;
- a temporary test probe observed 24 logical CPUs, no `RAYON_NUM_THREADS`
  override, and a 24-worker global Rayon pool;
- the working tree contains a candidate module-owned two-worker sizing pool,
  serial fallback, and expanded tests in
  `crates/devsweep-core/src/filesystem/sizing.rs`;
- the isolated focused sizing suite was reported passing after the user added
  narrow antivirus exclusions;
- no accepted five-pair A/B sample set, median, ratio, or final throughput gate
  is recorded in the task, so the candidate is not complete or approved.

Do not discard, commit, archive, or resume this diff until the new parent fixes
its dependency order and receives fresh approval.

## Desktop Redesign Constraint

The current desktop guideline favors a quiet dense workbench and forbids large
decorative hero/illustration/card patterns
(`.trellis/spec/desktop-frontend/component-guidelines.md:3-24,53-60`). A new
visual direction must update that spec first rather than silently diverging in
CSS. Regardless of visual direction, the state, type, and safety contracts in
the rest of `.trellis/spec/desktop-frontend/` remain authoritative.
