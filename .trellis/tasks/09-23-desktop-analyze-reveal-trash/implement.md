# Implement — Analyze reveal and trash

## Order

1. Confirm the capsule shell child is complete. Read backend and desktop
   specs sections that define Analyze read-only behaviour and the Clean plan
   and execution contracts.
2. Update specs and the capability matrix first.
3. Core `analysis/actions.rs` with `node_path`, preview/refusals, reveal;
   tests for each refusal class and nested selection.
4. Snapshot retention in `AnalyzeCoordinator`; three Tauri commands; command
   inventory test; regenerate types; decoders; bridge; fixture bridge.
5. Desktop context menu, trash preview, confirmation, moved-node projection;
   keyboard tests; both locales.
6. Analyze stage on `Stage`.
7. Run gates.

## Validation

```powershell
cargo test -p devsweep-core --locked analysis
cargo test -p devsweep-desktop --locked
mise exec node@22 -- npm --prefix desktop run types:generate -- --check
cd desktop; mise exec node@22 -- npm run lint; mise exec node@22 -- npm run typecheck; mise exec node@22 -- npm run test; mise exec node@22 -- npm run build
just ci
```

Manual: analyze a temporary directory, reveal one file, move one file to the
Recycle Bin, and restore it from the Recycle Bin.

## Risk and rollback

Main risk: a path that should be refused reaches the plan. Tests for each
refusal class gate the child. Revert the trash commands alone if needed.
