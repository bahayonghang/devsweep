# E: technical design

## Design and evidence

- Initial concept: parent assets/beaver-concept-v1.png; prompt: assets/logo-prompt.txt. Three nominal colors, broad paddle tail, warm face, sage accent.
- The initial concept has fine tail patterning and tonal variation; simplify for small-size production. Do not merely resize and claim tray suitability.
- Consumers: desktop/src/assets/devsweep-icon-master.png; AppShell image import; src-tauri/icons; tauri.conf.json bundle icon list; tray.rs uses default_window_icon.
- Use existing build-time packaging where available. Any image content edit uses imagegen; deterministic format/size export is a packaging step.
- Do not add mascot animations or replace warning/action semantics with cute raster images.

## Owned boundary

Task concept/provenance; desktop brand master and derived native icons; necessary shell asset import/test; asset conversion/manifest only if needed.

## Compatibility

Preserve the parent design's saved-report/plan, appearance and explicit-root contracts. New wire fields require generated Rust/TypeScript/decoder parity and old/new fixtures. No UI-derived cleanup authority.

## Dependencies and handoff

Parent visual direction. Independent of A/B/C. Coordinate asset paths/consumer contract with D.

## Rollback

Keep prior production assets until review succeeds and replace the complete asset family together; revert consumers/derived files as a unit.

