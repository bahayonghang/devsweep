# Design — capsule shell and planet stage

## Shell composition

`AppShell` stays the owner of routing, focus restoration, accelerators, and
the `ShellRouteCoordinator` handshake. Only its markup changes:

```
<div class="app-shell">
  <nav class="capsule" role="tablist">      brand button + 5 mode tabs
  <BrandMenu>                                role=menu popover, anchored to brand button
  <main class="stage-host">                  active mode or supporting page
```

- `ShellRoute` keeps `mode:*`, `support:*`, and `settings`. Language and Help
  move from sidebar rows to brand-menu items; Language opens the existing
  settings route, Help opens the existing help behaviour.
- `PageHeaderSlot` is removed. Mode chips that rendered into it move into the
  mode stage (secondary line) or detail card header. Remove the portal and its
  context only after no mode imports it.
- `SweepBody` and the sidebar glyph tiles are removed when unused.

## Stage components (`desktop/src/stage/`)

- `Planet.tsx` — `<canvas>` sized by CSS; device-pixel-ratio aware. Drawing:
  for each pixel inside the disc, compute the sphere normal, rotate the
  longitude by the current angle, sample a seeded value-noise field (small
  in-file implementation, no dependency) to choose between palette bands, then
  apply Lambert lighting from a fixed light direction plus a rim darkening
  term. Render to an offscreen `ImageData` at a capped resolution (max 256 px
  diameter source) and scale with `drawImage`, so one frame costs a bounded
  amount of work. Frame loop uses `requestAnimationFrame`, throttled to 30 fps,
  stopped by `document.hidden`, `active=false`, or unmount.
- `planet-palettes.ts` — five original palettes and seeds keyed by `ModeId`.
  Values are chosen for this project; none are sampled from Mole screenshots.
- `Stage.tsx` — centered stack: `hero`, `title`, `meta`, `primary`,
  `secondary` slots.
- `StageResult.tsx` — number + unit + meta line + action, built on `Stage`.
- `DetailView.tsx` — full-width card region with a back control labelled with
  original copy (EN "Back to overview", zh-CN "返回概览").

Modes adopt `Stage` for their first screen in this child with their existing
state and actions only. Behaviour additions stay in the parity children.

## Tokens

Replace sidebar tokens with `--capsule-bg`, `--capsule-border`,
`--capsule-active-bg`, `--capsule-active-text`, `--stage-canvas`. Keep
semantic `--danger/--warning/--ok`. CSS still avoids `linear-gradient`,
`radial-gradient`, and `backdrop-filter`; shading lives only in the canvas.

## Spec and provenance updates (first step)

- `component-guidelines.md`: replace the sidebar contract with the capsule
  contract; replace the sweep-body/planet prohibition with the procedural
  planet rules in PRD R4 (original, seeded, non-informational, aria-hidden,
  stops when hidden, forced-colours outline). Keep the prohibition on
  photographs, NASA imagery, Mole assets/geometry, and "freed" copy.
- `index.md` and `state-management.md`: update sidebar references.
- `docs/provenance.md`: add a Mole desktop section: layout topology (capsule +
  planet stage + big number) informed by the public mole.fit product page on
  2026-09-23; no Mole code, image, texture, copy, or colour value copied.

## Tests

- Shell tests: rewrite sidebar queries to capsule/brand-menu queries; keep all
  route/back/focus/accelerator assertions.
- `Planet.test.tsx`: mock canvas context and `requestAnimationFrame`; assert
  start, stop on hidden/route/unmount, single frame under reduced motion.
- Stage snapshot-free render tests per mode in both locales.

## Rollback

All changes are presentation files. Revert the child as one unit if route,
focus, or mode authority tests regress.
