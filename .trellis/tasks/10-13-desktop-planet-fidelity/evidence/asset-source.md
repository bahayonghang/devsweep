# Earth source acquisition

Inspected and downloaded on 2026-10-13. The application's Earth is a geographic
cartographic raster, not a photograph or live satellite observation.

- Provider: Natural Earth; primary authors Tom Patterson and Nathaniel Vaughn Kelso.
- Catalogue: https://www.naturalearthdata.com/downloads/10m-raster-data/10m-natural-earth-1/
- Selected variant: Natural Earth I with Shaded Relief and Water, medium resolution.
- Source: https://naturalearth.s3.amazonaws.com/10m_raster/NE1_LR_LC_SR_W.zip
- Archive SHA-256: `8e30c15d49bc73223a36eaeffc07fd49998e658e373a5498f276f5d26bfd1d0a`.
- TIFF SHA-256: `539906dc5f2cc83be551aeb2a4e7d2e168a6fdd573207a4477c69e923ff95dbb`.
- Raster: `NE1_LR_LC_SR_W.tif`, RGB, 16200 x 8100, equirectangular.
- Version evidence: the catalogue advertises 3.2.0, but the downloaded archive's
  `NE1_LR_LC_SR_W.VERSION.txt` says 2.0.0. The exact archive/hash identifies the
  input; do not describe it as a verified 3.2.0 archive.
- Terms: https://www.naturalearthdata.com/about/terms-of-use/ states Natural Earth
  raster/vector data are public domain and may be modified and redistributed.

The first land-cover-only candidate had white water and was rejected. The
selected water/relief map was inspected after a Lanczos downsample to 1024 x 512;
it retains geographic continents, islands, lakes and polar regions. No land or
coastlines are synthesized. No Mole source or asset is used.

Original archives/TIFF and candidate PNGs are in this task's ignored
`evidence/dist/` directory, outside the application payload.

## Shipped derivatives

Reproduction: `python desktop/scripts/prepare-earth-assets.py
.trellis/tasks/10-13-desktop-planet-fidelity/evidence/dist/NE1_LR_LC_SR_W.zip`
(one command, Pillow 12.3.0). The script refuses an archive with a different hash.
No production dependency is added. The primary is a Lanczos 1024x512 RGB
downsample with no color/geographic edits. The still is a 512x512 RGBA sphere
with a 20-degree longitude pose and fixed lighting, sampled from that map.

| Packaged path | Bytes | SHA-256 |
| --- | ---: | --- |
| `desktop/public/earth/natural-earth.png` | 658001 | `de1ded3b671bb14c3a518dbf4f2b932eb2fb0987621969df88a4d7e8036f8a60` |
| `desktop/public/earth/still.png` | 250789 | `1826f0bd821a838717f4cd8756d3fa50c85ada38395f8a8bfaa638e0620cfe18` |

Total: 908790 bytes, below the 1 MiB payload budget. Both files are present in
the production benchmark build's `earth/` directory. Native embedded packaging
and offline application launch remain integration checks, not inferred passes.
Distribution notice: `desktop/public/THIRD_PARTY_NOTICES.txt`.
