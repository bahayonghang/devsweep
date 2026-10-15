# E: Original animal identity and icon family

## Goal

Turn the generated original beaver direction into a recognizable, consistent DevSweep logo and correctly packaged small-size assets.

## Parent and dependencies

Parent: [cleanup-home-redesign](../10-14-cleanup-home-redesign/prd.md).
Parent visual direction. Independent of A/B/C. Coordinate asset paths/consumer contract with D.

## Requirements

E1. Use built-in image generation for an original small-animal identity. The parent contains an initial transparent beaver concept and exact prompt; treat it as a proposal.
E2. Refine a compact head/silhouette version that reads at 16/20/24/32px; retain a fuller mark for larger surfaces.
E3. Preserve alpha and clean edges, constrain palette and safe area, and verify both dark/light uses. Do not copy Mole assets.
E4. Update actual webview, Tauri bundle, window/taskbar/tray and installer icon consumers with reproducible conversion and an asset manifest.
E5. Document original/master/derived hashes, generation prompt/tool, dimensions and provenance. Keep ordinary semantic control icons in the consistent vector system coordinated with D.

## Acceptance criteria

- [x] The full and compact marks are recognizably the same original character; small-size review detects no disappearing face/teeth or muddy tail pattern.
- [x] Transparency, alpha edges, crop/safe area and light/dark preview are inspected; square container renders are deliberate exports. Residual alpha 1 edge pixels and 253–254 interiors are recorded in evidence/inspection.json.
- [x] All configured PNG/ICO/ICNS and applicable Windows package sizes are reproducibly derived and consumed; no stale mark remains in enumerated consumers.
- [ ] Actual native taskbar/tray and webview screenshots pass; duplicate screen-reader naming is avoided.
- [x] Image generation success, concept review and production/native acceptance are recorded as separate statuses; native final acceptance is NOT_RUN.

## Scope boundaries

Implementation authorized on 2026-10-14. Preserve the pre-existing desktop/src-tauri/Cargo.toml edit. No actual user-cache cleanup, unrelated modes, commits, archive or publication. Parent scope exclusions apply.

## Decision status

User product decisions and selected design are in the parent PRD. No additional product question blocks this plan; implementation depends on the stated upstream contracts.

