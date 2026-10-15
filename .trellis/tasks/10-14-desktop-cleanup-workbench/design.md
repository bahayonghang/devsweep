# D: technical design

## Design and evidence

- Source evidence and precise anchors are in parent research/home-ui.md. Keep React/domain authority in Clean, not shell navigation.
- Two report slots require scope + report revision + scan/operation ID fencing, not just a second results array.
- Use existing TargetTable pagination/lazy evidence and full-report selection logic; project grouping must also cap mounted group count.
- Supersede Clean-specific stage-first/hidden-title/mandatory-Earth spec rules. Do not rewrite global Stage/Planet behavior or reset appearance preferences.
- CSS: existing plain styles/custom properties, 8px controls, 44px hit targets, no external font/network asset, responsive 2:3 sections then stack.
- New color/control roles are scoped under the Clean workbench root and inherit the selected appearance settings; do not replace shared root appearance roles. Before/after captures must show unchanged palette and geometry for other modes/Settings, except the shared animal logo.
- D owns semantic glyph/locale updates; E owns raster master/native assets. Coordinate the one shared shell image consumer.

## Owned boundary

CleanWorkbench/useScanScope/ScanRoots/pages/state/selectors/coordinator seam, styles/preferences role mapping, i18n resources, semantic glyphs, desktop DESIGN.md and corresponding specs/tests.

## Compatibility

Preserve the parent design's saved-report/plan, appearance and explicit-root contracts. New wire fields require generated Rust/TypeScript/decoder parity and old/new fixtures. No UI-derived cleanup authority.

## Dependencies and handoff

A; B/C frozen contracts before integrated behavior; E's reviewed mark before final visual acceptance. Token/layout work can use reviewed fixtures earlier.

## Rollback

Revert bounded layout/controller changes while preserving root/appearance stores. Never recover by clearing settings or disabling digest checks.

