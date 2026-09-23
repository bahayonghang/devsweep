# Implement — Clean parity

## Order

1. Confirm `09-23-desktop-mole-capsule-shell` is complete.
2. Add `impact.ts` + tests; switch `ReviewPage` grouping to it.
3. Add skip/restore/protect reducer actions + tests; wire controls in
   `TargetTable` / `ReviewPage`.
4. Move Clean first screen, found screen, and report to `Stage` /
   `StageResult` / `DetailView`.
5. Core: `estimated_bytes` evidence field, history aggregate, tests.
6. Tauri command, generated types, decoder, bridge, fixture bridge, tests.
7. Result stage shows run total and cumulative total.
8. Run gates.

## Validation

```powershell
cargo test -p devsweep-core --locked
cargo test -p devsweep-desktop --locked
mise exec node@22 -- npm --prefix desktop run types:generate -- --check
cd desktop; mise exec node@22 -- npm run lint; mise exec node@22 -- npm run typecheck; mise exec node@22 -- npm run test; mise exec node@22 -- npm run build
just ci
```

## Risk and rollback

Risk: selection/digest invalidation regressions and audit schema drift.
Revert core R5 separately from the frontend if needed.
