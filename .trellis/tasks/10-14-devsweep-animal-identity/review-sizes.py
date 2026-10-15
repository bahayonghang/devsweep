from pathlib import Path
from PIL import Image, ImageDraw

assets = Path(__file__).parent.parent / '10-14-cleanup-home-redesign' / 'assets'
im = Image.open(assets / 'beaver-compact-v2.png')
out = Image.new('RGB', (780, 200), '#ffffff')
draw = ImageDraw.Draw(out)
draw.rectangle((0, 100, 780, 200), fill='#121827')
for index, size in enumerate([16, 20, 24, 32, 48, 64]):
    stamp = im.resize((size, size), Image.Resampling.LANCZOS)
    for top in [0, 100]:
        out.paste(stamp, (16 + index * 120, top + 16), stamp)
        draw.text((16 + index * 120, top + 82), str(size), fill='#64748b')
out.save(assets / 'compact-v2-size-review.png')
print(im.size, im.getchannel('A').getbbox())
