"""Offline deterministic derivative of the pinned public-domain Natural Earth map.

Usage: python desktop/scripts/prepare-earth-assets.py SOURCE_ZIP
Requires Pillow 12.3.0 only for this developer conversion, never at runtime.
"""
import hashlib
import io
import math
from pathlib import Path
import sys
from zipfile import ZipFile

from PIL import Image

source = Path(sys.argv[1])
expected = "8e30c15d49bc73223a36eaeffc07fd49998e658e373a5498f276f5d26bfd1d0a"
if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
    raise SystemExit("Source does not match the recorded Natural Earth archive")
Image.MAX_IMAGE_PIXELS = 16200 * 8100
with ZipFile(source) as archive:
    raster = Image.open(io.BytesIO(archive.read("NE1_LR_LC_SR_W.tif")))
    if raster.size != (16200, 8100) or raster.mode != "RGB":
        raise SystemExit("Unexpected source raster")
    texture = raster.resize((1024, 512), Image.Resampling.LANCZOS)

output = Path(__file__).resolve().parents[1] / "public" / "earth"
output.mkdir(parents=True, exist_ok=True)
texture.save(output / "natural-earth.png", optimize=True)

# Same equirectangular projection/20-degree pose and fixed lighting as the
# renderer. Bilinear source sampling preserves coastlines; nothing is invented.
diameter = 512
radius = diameter / 2
still = Image.new("RGBA", (diameter, diameter))
pixels = still.load()
source_pixels = texture.load()
light = (-0.55, 0.5, 0.67)
length = math.sqrt(sum(value * value for value in light))
light = tuple(value / length for value in light)
for y in range(diameter):
    ny = (radius - y - 0.5) / radius
    row = max(0, min(511, (0.5 - math.asin(ny) / math.pi) * 512 - 0.5))
    top = int(row)
    bottom = min(511, top + 1)
    fy = row - top
    for x in range(diameter):
        nx = (x + 0.5 - radius) / radius
        distance = math.hypot(nx, ny)
        coverage = min(1, max(0, (1 - distance) * radius + 0.5))
        if not coverage:
            continue
        nz = math.sqrt(max(0, 1 - min(1, distance * distance)))
        column = ((math.atan2(nx, nz) / (2 * math.pi) + 0.5 + 20 / 360) * 1024 - 0.5) % 1024
        left = int(column)
        right = (left + 1) % 1024
        fx = column - left
        lambert = max(0, nx * light[0] + ny * light[1] + nz * light[2])
        shade = (0.42 + 0.58 * lambert) * (0.85 + 0.15 * math.sqrt(nz))
        colors = []
        for channel in range(3):
            a = source_pixels[left, top][channel] * (1 - fy) + source_pixels[left, bottom][channel] * fy
            b = source_pixels[right, top][channel] * (1 - fy) + source_pixels[right, bottom][channel] * fy
            colors.append(round((a * (1 - fx) + b * fx) * shade))
        pixels[x, y] = (*colors, round(coverage * 255))
still.save(output / "still.png", optimize=True)
for path in sorted(output.glob("*.png")):
    data = path.read_bytes()
    print(path.name, len(data), hashlib.sha256(data).hexdigest())
