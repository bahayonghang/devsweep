# Implement — capsule shell

## Order

1. Update `.trellis/spec/desktop-frontend/{component-guidelines,index,state-management}.md`
   and `docs/provenance.md` per design.
2. Add `desktop/src/stage/` (`Planet`, palettes, `Stage`, `StageResult`,
   `DetailView`) with unit tests.
3. Rewrite `AppShell` markup to capsule + brand menu; keep routing and focus
   logic. Update i18n keys (EN + zh-CN) for brand menu and back control.
4. Move each mode's first screen onto `Stage` with existing state/actions.
   Remove `PageHeaderSlot` usage, then the portal, `SweepBody`, sidebar CSS.
5. Rewrite shell tests; add responsive, reduced-motion, forced-colours tests.
6. Run gates.

## Validation

```powershell
cd desktop
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

Manual: `just tdev`, check five modes and brand menu at 1080x720 and 900x600.

## Risk and rollback

Risky files: `app-shell/AppShell.tsx`, `styles.css`, each mode workbench
entry. Revert the child's presentation files together if routing, focus, or
Clean authority tests fail.
