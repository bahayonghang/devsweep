# Implement — Status parity

## Order

1. Confirm the capsule shell child is complete. Read backend status spec
   sections and desktop state-management lifecycle rules.
2. Core PDH probe, GPU/thermal aggregation, snapshot fields, tests.
3. Regenerate wire types; decoders; Status dashboard rows and stage.
4. Process sort/pin selectors and table controls; tests.
5. Tray + HUD window + `HudSampler` lifecycle; HUD Vite entry; tests.
6. Manual tray/HUD check in `just tdev`.
7. Run gates.

## Validation

```powershell
cargo test -p devsweep-core --locked status
cargo test -p devsweep-desktop --locked
mise exec node@22 -- npm --prefix desktop run types:generate -- --check
cd desktop; mise exec node@22 -- npm run lint; mise exec node@22 -- npm run typecheck; mise exec node@22 -- npm run test; mise exec node@22 -- npm run build
just ci
```

## Risk and rollback

Risks: PDH counter absence on some hosts (handled as `unavailable`) and a
HUD sampler that outlives the window (gated by the lifecycle test). Revert the
tray HUD separately from the core probes.
