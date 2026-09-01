# Integrate Generated DevSweep App Icon

Parent: `08-29-scan-resource-bounds-app-icon`

## Goal

Promote the generated small-size-first DevSweep icon into a stable product
master, regenerate the Tauri native icon set, and directly verify that the
identity remains distinguishable at 16px and 32px before shell integration.

## Confirmed Background

- The selected generated asset is
  `research/assets/devsweep-icon-small-master.png`: 1254x1254 PNG, opaque RGB,
  full-bleed emerald background, with two broad cache slabs, a sweep crescent,
  and one sparkle. Its provenance is recorded in
  `research/icon-generation.md`.
- React/TUI brand placement is intentionally owned by sibling
  `08-29-desktop-shell-navigation-brand`; this child must not edit shell files.
- The local Tauri CLI supports `tauri icon INPUT --output OUTPUT` and currently
  owns 16 generated files under `desktop/src-tauri/icons/`; five are referenced
  by `desktop/src-tauri/tauri.conf.json:29-35`.
- Earlier generated transparency attempts contained simulated checkerboard or
  a dark backdrop. The selected asset is intentionally opaque and must not be
  represented as alpha-transparent.

## Requirements

- R1: Stable product-owned master.

Copy the selected task asset into a stable product path under
`desktop/src/assets/`. Runtime and packaging code must not reference
`.trellis/tasks/` or the Codex generated-images directory.

- R2: Reproducible complete Tauri icon set.

Run the repository's already-installed local Tauri CLI against the product
master and regenerate the complete existing output set under
`desktop/src-tauri/icons/`. Do not add a package or image-processing dependency.
All 16 outputs enumerated in `design.md` must exist and decode. The 14 PNGs must
match their documented pixel dimensions; ICO and ICNS must match their
documented embedded frame inventories.

- R3: Stable downstream handoff.

Document the stable master path, native output inventory, source hash, opaque
background, small-size criteria, and accessible-name recommendation for the
shell child. Do not edit `App.tsx`, CSS, TUI, or frontend tests here.

- R4: Hard small-size legibility gate.

Inspect the generated `32x32.png` at 1:1 and the running Windows window/taskbar
icon at native size (including the 16px-class title-bar presentation). A passing
icon must show two separate cache slabs, a recognizable single sweep crescent,
an unclipped silhouette, and no checkerboard/halo.

If either 16px or 32px evidence fails, stop and return to planning for a newly
generated simplified source. Do not compensate with CSS enlargement, accept a
blurred result, or silently ship the higher-detail earlier concept.

- R5: Packaging and cross-platform evidence boundaries.

Verify the Windows desktop build/package and native window/taskbar icon on the
available host. Verify macOS/Linux files decode, but keep their native visual
appearance `UNVERIFIED` unless checked on those hosts.

## Acceptance Criteria

- [ ] AC1 (R1): A stable product master exists under
      `desktop/src/assets/`, matches the selected task asset by SHA-256, and no
      runtime/config reference points into `.trellis/` or `$CODEX_HOME`.
- [ ] AC2 (R2): The documented local Tauri command exits
      successfully and produces the exact 16-file output inventory listed in
      `design.md`; all 16 files decode, all 14 PNGs match their dimensions, and
      ICO/ICNS match their embedded frame inventories.
- [ ] AC3 (R3): The handoff records the stable product path, SHA-256, opaque
      background, exact native inventory, small-size evidence, and decorative
      accessible-name recommendation; no React/TUI/CSS/test file changes.
- [ ] AC4 (R4): Recorded 1:1 evidence at 32px and native
      Windows 16px-class presentation shows two distinct slabs, one sweep,
      unclipped edges, and no fake transparency/halo; otherwise the child
      returns to planning.
- [ ] AC5 (R3, R4): The shell child can import the stable master without a
      `.trellis` or generated-images runtime reference; responsive UI appearance
      remains that child's acceptance responsibility.
- [ ] AC6 (R5): `just desktop-web-check` and
      `just desktop-build` pass; Windows window/taskbar/package evidence is
      recorded; macOS/Linux native appearance is labeled accurately.
- [ ] AC7 (R2, R5): `git diff --check` and relevant `just ci` gates pass, and
      every pre-existing dirty path outside this child's declared change list
      remains preserved and excluded.

## Out of Scope

- Editing desktop/TUI layout, typography, header, accessible name, or scan flow.
- Adding an icon editor, image library, runtime asset loader, or dependency.
- Claiming transparency for the opaque source.
- Scan-concurrency work, which belongs to sibling child
  `08-29-bound-sizing-concurrency`.

## Risks and Deferred Items

- Generated vector-like art may still degrade during platform resampling. The
  native 16/32 evidence gate is authoritative; source-resolution appearance is
  not sufficient.
- macOS/Linux native visual checks remain `UNVERIFIED` from this Windows task.
