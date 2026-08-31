# Implementation Plan - Final Five-Mode Integration

Implementation remains gated on explicit approval and every named dependency.
For a coordination-only named dependency, the gate is its recorded recursive
acceptance PASS; the coordination task may remain unarchived until final
integration completes. All preceding implementation leaves must already be
archived. The paused sizing child is not resumed by this plan.

## Steps

1. Verify all dependency ACs, generated bindings, versioned schemas, focused
   gates, explicit implementation approvals, archived implementation leaves,
   and the overall `PASS` result in each of these active coordination records
   before registering any mode:
   `.trellis/tasks/08-29-scan-resource-bounds-app-icon/acceptance.md`,
   `.trellis/tasks/08-29-analyze-mode/acceptance.md`,
   `.trellis/tasks/08-29-software-mode/acceptance.md`,
   `.trellis/tasks/08-29-optimize-mode/acceptance.md`, and
   `.trellis/tasks/08-29-status-mode/acceptance.md`. Each record must identify
   the reviewed product commits and archived leaf paths, map every parent AC to
   evidence, and retain commands, exit codes, logs, and native/manual status.
   Capture the existing user-owned `README.md` hunks with context fingerprints;
   keep later task documentation hunks separate and re-audit ownership before
   staging or commit.
2. Integrate shared CLI/TUI/Tauri/desktop glue only; return domain failures to
   the owning child instead of adding integration-layer workarounds.
3. Add cross-mode coordinator, digest, locale, cancellation, restart, stale
   event, partial-state, and audit-version fixtures.
4. Add the deterministic sampler and run automated checks before native tests.
5. Run the native and resource matrices on the frozen host/build/fixtures,
   retain raw artifacts, and mark each scenario PASS, FAIL, or UNVERIFIED. Use
   only the approved bounded native-evidence protocol; display scaling is
   user-operated, and Software requires the separately confirmed exact
   disposable current-user MSIX identity before uninstall.
6. Update bilingual documentation and local packaging evidence. Stop before
   signing, pushing, publishing, or releasing.

## Verification

- Recursive `task.py validate` and plan precheck for the root and all children.
- `rtk just ci`
- `rtk npm --prefix desktop run test -- --run`
- `rtk npm --prefix desktop run lint`
- `rtk npm --prefix desktop run typecheck`
- `rtk npm --prefix desktop run build`
- `rtk npm --prefix desktop run tauri build -- --debug`
- `rtk npm --prefix desktop run types:generate`
- `rtk npm --prefix desktop run test -- --run src/api/types-generation.test.ts`
- `& .\tools\measure-resources.ps1 -Protocol five-mode-v1`
- `rtk git diff --check`
- Manual native gate: both languages; standard user/no UAC; keyboard and screen
  reader; reduced motion/high contrast; 390/800/1024/1440 widths; 100/125/150/
  200% user-operated scaling; cancellation/navigation/exit; sleep/resume where
  reproducible; restart; taskbar/window icon; all capability and unsupported
  states. The agent does not change display settings.

## Stop and rollback points

- Stop on an unresolved child blocker, missing approval, schema mismatch,
  generated drift, elevation/UAC, resource-threshold miss, orphan process/thread,
  or incomplete native evidence. Do not weaken a threshold to obtain a pass.
- Roll back shared registration and packaging metadata to the last accepted local
  release. Preserve versioned plans, audit records, raw evidence, unrelated dirt,
  the recorded pre-existing `README.md` hunks, and unknown/newer schemas; never
  publish from this task.
