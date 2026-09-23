# Implement — Software parity

## Order

1. Confirm the capsule shell child is complete. Read backend spec index,
   `quality-guidelines.md`, and `docs/safety-capability-matrix.md`.
2. Capture real `winget upgrade --disable-interactivity` output on this host
   (EN and, if available, zh-CN) as redacted fixtures.
3. `updates.rs` + parser tests → Tauri command → desktop Updates view.
4. `startup.rs` + tests (temporary test key) → commands → Startup view.
5. `leftovers.rs` + plan/digest + precondition tests → commands → uninstall
   flow leftover step.
6. Software stage with counts; both locales.
7. Spec and capability matrix updates.
8. Run gates.

## Validation

```powershell
cargo test -p devsweep-core --locked software
cargo test -p devsweep-desktop --locked
mise exec node@22 -- npm --prefix desktop run types:generate -- --check
cd desktop; mise exec node@22 -- npm run lint; mise exec node@22 -- npm run typecheck; mise exec node@22 -- npm run test; mise exec node@22 -- npm run build
just ci
```

Manual: toggle one current-user startup entry off and on in `just tdev`, then
confirm Task Manager shows the same state.

## Risk and rollback

Risks: winget text format drift (mitigated by `unavailable` on any mismatch)
and registry write mistakes (mitigated by re-read verification, current-user
scope, and audit). Revert module by module.
