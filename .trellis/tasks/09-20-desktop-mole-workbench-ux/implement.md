# Implement — UX child

## Order

1. Read the desktop spec index and parent design; record current UI fingerprints.
2. Freeze tokens, copy/provenance rules, and responsive DOM contract.
3. Refine shell and shared surfaces, then Clean, Software/Optimize, Analyze,
   and Status in small mode-local changes.
4. Add reducer/render tests for state truthfulness and accessibility.
5. Run frontend gates and hand native/resource work to the later children.

## Validation

```powershell
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

Review EN/zh-CN at 390/800/1024/1440 CSS px, keyboard-only, reduced motion,
forced colors, and long path/application copy. Do not claim native scaling.

## Risk and rollback

Risky files are `desktop/src/styles.css`, `desktop/src/app-shell/AppShell.tsx`,
mode workbenches, shared glyph/format components, and their tests. Restore the
child's presentation files together if route/focus or authority tests fail.
