"""Inspect packaged images and render a review sheet, without changing artwork."""
import hashlib
import json
from pathlib import Path
import sys
from PIL import Image, ImageDraw
import PIL

source, icons, output = (Path(value) for value in sys.argv[1:])
output.mkdir(parents=True, exist_ok=False)

def inspect(path):
    with Image.open(path) as opened:
        frame_sizes = (sorted(opened.ico.sizes()) if opened.format == "ICO" else
                       opened.info.get("sizes") if opened.format == "ICNS" else None)
        image = opened.convert("RGBA")
        alpha = image.getchannel("A")
        counts = alpha.histogram()
        return {
            "file": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "format": opened.format, "width": image.width, "height": image.height,
            "frames": frame_sizes, "alpha_extrema": alpha.getextrema(),
            "alpha_counts": {"zero": counts[0], "one_to_eight": sum(counts[1:9]),
                             "nine_to_249": sum(counts[9:250]),
                             "250_to_254": sum(counts[250:255]), "255": counts[255]},
            "bounds_alpha_gt_8": alpha.point(lambda value: 255 if value > 8 else 0).getbbox(),
            "bounds_alpha_gt_128": alpha.point(lambda value: 255 if value > 128 else 0).getbbox(),
            "outer_edge_max_alpha": max(alpha.crop(box).getextrema()[1] for box in
                [(0, 0, image.width, 1), (0, image.height - 1, image.width, image.height),
                 (0, 0, 1, image.height), (image.width - 1, 0, image.width, image.height)]),
        }

paths = [source, *sorted(path for path in icons.iterdir() if path.is_file())]
report = {"pillow": PIL.__version__, "images": [inspect(path) for path in paths],
          "method": "Read-only alpha/format inspection; actual ICO frames where available. Only 20px uses deterministic master resizing for the review sheet."}
(output / "inspection.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
master = Image.open(source).convert("RGBA")
ico = Image.open(icons / "icon.ico")
sizes = [16, 20, 24, 32, 48, 64, 128]
sheet = Image.new("RGBA", (len(sizes) * 150, 3 * 180))
draw = ImageDraw.Draw(sheet)
for row, color in enumerate(["#ffffff", "#111714", "#111426"]):
    draw.rectangle((0, row * 180, sheet.width, (row + 1) * 180), fill=color)
    for column, size in enumerate(sizes):
        icon = (ico.ico.getimage((size, size)).convert("RGBA") if (size, size) in ico.ico.sizes()
                else master.resize((size, size), Image.Resampling.LANCZOS))
        sheet.alpha_composite(icon, (column * 150 + (150 - size) // 2, row * 180 + 10))
        draw.text((column * 150 + 10, row * 180 + 150), f"{size}px {'ICO' if (size,size) in ico.ico.sizes() else 'master'}", fill="#64748b")
sheet.convert("RGB").save(output / "actual-frames-light-dark.png")
print(json.dumps({"images": len(report["images"]), "ico_sizes": sorted(ico.ico.sizes()), "output": str(output)}))
