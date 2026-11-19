"""Pre-render the blob PNGs for candidate (3) of the material gallery.

Each file is one blob colour with a radial alpha ramp: `ALPHA` at the centre,
falling linearly to 0 at 70% of the radius (the CSS
`radial-gradient(<colour> / 0.7, transparent 70%)` of the parent spec).
The colour channels are constant, so the edge has no dark fringe.

Run from this directory: `python make_blob_png.py`. Needs Pillow.
"""

import math
from pathlib import Path

from PIL import Image

SIZE = 256
ALPHA = 0.70
EDGE = 0.70
BLOBS = {
    "light": ["CDEEE2", "DCD8F6", "D8EBF6"],
    "dark": ["12302A", "2A2550", "142536"],
}


def blob(colour: str) -> Image.Image:
    red, green, blue = (int(colour[i : i + 2], 16) for i in (0, 2, 4))
    image = Image.new("RGBA", (SIZE, SIZE))
    pixels = image.load()
    half = SIZE / 2
    for y in range(SIZE):
        for x in range(SIZE):
            radius = math.hypot(x + 0.5 - half, y + 0.5 - half) / half
            alpha = ALPHA * max(0.0, 1.0 - radius / EDGE)
            pixels[x, y] = (red, green, blue, round(alpha * 255))
    return image


for theme, colours in BLOBS.items():
    for index, colour in enumerate(colours, start=1):
        path = Path(__file__).with_name(f"blob-{theme}-{index}.png")
        blob(colour).save(path, optimize=True)
        print(path.name, path.stat().st_size)
