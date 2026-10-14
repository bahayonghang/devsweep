# Design Provenance

## Scope

This note records how DevSweep relates to other public cleanup tools,
especially Mole (GPL-3.0). It is not legal advice.

## Position

DevSweep is an independent implementation under the MIT License.

Public tools such as Mole informed the product problem: identify high-value
developer caches and build artifacts, make irreversible work explicit, and
prefer inspectable plans to silent mutation. Those are shared concerns, not
shared implementation material.

## What was not copied

DevSweep did not copy or lightly rewrite any of the following from Mole or
another third-party cleanup tool:

- source code;
- fixtures or golden files;
- rule tables or provider command catalogs;
- user-facing copy or UI text;
- license text or copyright notices.

Audit-remediation work under `.trellis/tasks/` is derived from DevSweep's own
audit reports and invariant descriptions.

## PureMac

DevSweep reviewed PureMac (`https://github.com/momenbasel/PureMac`) as an
MIT-licensed reference for sidebar workbench layout topology and interaction
ideas. DevSweep did not copy PureMac source, strings, screenshots, SF Symbols,
or hex values.

## Mole desktop layout

On 2026-09-23 DevSweep reviewed the public Mole desktop product page
(`https://mole.fit`) as a layout reference. The capsule navigation shell took
three topology ideas from that page: one top-centered pill navigation bar, one
centered planet visual per mode, and one large primary number with a single
action below it.

DevSweep did not copy Mole code, images, textures, planet renders, copy,
taglines, labels, or colour values. In that September version, the planets
were drawn at runtime by DevSweep code (`desktop/src/stage/Planet.tsx`) from seeded value noise and
palettes chosen for this project (`desktop/src/stage/planet-palettes.ts`).
No photograph, NASA image, or texture file backed that version. All interface copy is
original DevSweep text in English and Simplified Chinese.

## Geographic Earth direction

On 2026-10-13 the user approved replacing Clean's invented noise geography
with an offline geographic Earth texture. This changes only the Clean planet;
other mode planets retain DevSweep's procedural sources. The renderer and
interface remain independently implemented. Mole imagery and textures are
not a source for this work.

Clean's source is [Natural Earth I with Shaded Relief and Water](https://www.naturalearthdata.com/downloads/10m-raster-data/10m-natural-earth-1/),
downloaded as `NE1_LR_LC_SR_W.zip` from Natural Earth's own distribution.
[Natural Earth's terms](https://www.naturalearthdata.com/about/terms-of-use/)
place its raster/vector map data in the public domain and permit modification
and redistribution. This is a geographic cartographic map, not a photograph
or live satellite observation.

The catalogue advertises version 3.2.0, while this exact archive's embedded
version file says 2.0.0. The packaged
third-party notices in `desktop/public/THIRD_PARTY_NOTICES.txt` preserve that
discrepancy and pin the archive, source TIFF and derivative SHA-256 values.
The offline conversion script, `desktop/scripts/prepare-earth-assets.py`,
downsamples the 16200 x 8100 RGB map to a 1024 x 512 texture and creates a
local still-pose fallback. Only these small derivatives ship with the app;
original source archives remain build evidence and are not application assets.

## Future review

If a future change introduces material derived from a GPL-licensed work, stop
and reevaluate the licensing obligations before merging. Independent
reimplementation of ideas and safety requirements does not by itself change the
MIT license baseline.
