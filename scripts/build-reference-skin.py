"""Build the original reference-inspired PNG sprite sheets and .jpskin archive."""
import json
import math
import struct
import zipfile
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/skins/hope-heart-realistic-source.png"
OUT = ROOT / "release/skins/hope-heart-realistic-1.0.0"
ARCHIVE = OUT.parent / f"{OUT.name}.jpskin"
CELL = (320, 224)


def master():
    image = Image.open(SOURCE).convert("RGBA")
    bounds = image.getchannel("A").point(lambda a: 255 if a > 80 else 0).getbbox()
    image = image.crop((bounds[0] - 12, bounds[1] - 12, bounds[2] + 12, bounds[3] + 12))
    image.thumbnail((288, 168), Image.Resampling.LANCZOS)
    return image


def sprite_frame(sprite, state, index):
    frame = Image.new("RGBA", CELL)
    if state != "idle":
        fx = Image.new("RGBA", CELL)
        draw = ImageDraw.Draw(fx)
        alpha = [105, 155, 195, 135][index]
        color = (132, 246, 100, alpha) if state == "locked" else (255, 196, 71, alpha)
        if state in ("locked", "target"):
            draw.ellipse((36, 30, 285, 197), outline=color, width=3)
            draw.arc((45, 38, 276, 189), 25 + index * 28, 110 + index * 28, fill=color, width=5)
        else:
            draw.ellipse((250, 74, 319, 151), fill=(255, 232, 136, alpha))
        frame.alpha_composite(fx.filter(ImageFilter.GaussianBlur(7)))
        frame.alpha_composite(fx)
    size = (round(sprite.width * (1 + [0, .012, .02, .012][index])),
            round(sprite.height * (1 + [0, .012, .02, .012][index])))
    body = sprite.resize(size, Image.Resampling.LANCZOS)
    frame.alpha_composite(body, ((CELL[0] - size[0]) // 2,
                                  (CELL[1] - size[1]) // 2 + [2, 0, -2, 0][index]))
    return frame


def save_sheet(sprite, state):
    sheet = Image.new("RGBA", (CELL[0] * 4, CELL[1]))
    for index in range(4):
        sheet.alpha_composite(sprite_frame(sprite, state, index), (CELL[0] * index, 0))
    sheet.save(OUT / f"{state}.png", optimize=True)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    sprite = master()
    for state in ("idle", "locked", "target", "hit"):
        save_sheet(sprite, state)
    preview = Image.new("RGBA", (640, 448))
    preview.alpha_composite(sprite_frame(sprite, "locked", 2).resize((640, 448), Image.Resampling.LANCZOS))
    preview.save(OUT / "preview.png", optimize=True)
    trail = Image.new("RGBA", (320, 64))
    draw = ImageDraw.Draw(trail)
    for x in range(10, 310, 20):
        r = 2 + x / 95
        draw.ellipse((x-r, 32-r, x+r, 32+r), fill=(255, 197, 83, round(20+x/2)))
    trail.save(OUT / "trail.png", optimize=True)
    manifest = {
        "schemaVersion": 1, "id": "hope-heart-realistic", "name": "希望之心·晶翼",
        "author": "Juno Pulsar Desktop contributors", "version": "1.0.0",
        "license": "CC-BY-4.0", "assetScale": 1.2,
        "preview": "preview.png", "trail": "trail.png",
        "animations": {s: {"file": f"{s}.png", "frames": 4, "fps": 10 if s == "idle" else 14}
                       for s in ("idle", "locked", "target", "hit")},
    }
    (OUT / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False), encoding="utf-8")
    names = ["manifest.json", "preview.png", "idle.png", "locked.png", "target.png", "hit.png", "trail.png"]
    decoded = 0
    for name in names[1:]:
        image = Image.open(OUT / name)
        assert image.mode == "RGBA" and image.width <= 4096 and image.height <= 4096
        decoded += image.width * image.height * 4
        data = (OUT / name).read_bytes()
        at = 8
        while at < len(data):
            length = struct.unpack(">I", data[at:at+4])[0]
            kind = data[at+4:at+8]
            assert kind in (b"IHDR", b"IDAT", b"IEND")
            at += 12 + length
        assert at == len(data)
    assert decoded < 64 * 1024 * 1024
    with zipfile.ZipFile(ARCHIVE, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as bundle:
        for name in names:
            bundle.write(OUT / name, name)
    assert ARCHIVE.stat().st_size < 16 * 1024 * 1024
    print(ARCHIVE)


if __name__ == "__main__":
    main()
