"""Buat gambar installer: art/banner.bmp (493x58) dan art/dialog.bmp (493x312).

Jalankan dari mana saja:  python native/installer/make-art.py
Butuh Pillow dan font Segoe UI (bawaan Windows). Hasilnya di-commit, jadi
build-msi.ps1 tidak butuh Python.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
ICON = HERE.parent.parent / "assets" / "icon.png"
OUT = HERE / "art"
FONTS = Path(r"C:\Windows\Fonts")
INDIGO, VIOLET = (79, 70, 229), (124, 58, 237)


def font(name, size):
    return ImageFont.truetype(str(FONTS / name), size)


def gradient(w, h):
    """Gradien diagonal indigo -> violet."""
    img = Image.new("RGB", (w, h))
    px = img.load()
    for y in range(h):
        for x in range(w):
            t = (x / w + y / h) / 2
            px[x, y] = tuple(round(a + (b - a) * t) for a, b in zip(INDIGO, VIOLET))
    return img


def icon(size):
    return Image.open(ICON).convert("RGBA").resize((size, size), Image.LANCZOS)


def dialog():
    # Panel kiri 164 px (area gambar WixUI), sisanya putih.
    img = Image.new("RGB", (493, 312), "white")
    img.paste(gradient(164, 312), (0, 0))
    d = ImageDraw.Draw(img)
    ic = icon(96)
    img.paste(ic, (34, 70), ic)
    d.text((82, 188), "PrayDF", font=font("segoeuib.ttf", 26), fill="white", anchor="mm")
    d.text((82, 216), "Fast, private\nPDF viewer", font=font("segoeui.ttf", 12),
           fill=(224, 222, 255), anchor="ma", align="center", spacing=3)
    return img


def banner():
    img = Image.new("RGB", (493, 58), "white")
    ic = icon(38)
    img.paste(ic, (493 - 38 - 14, 10), ic)
    ImageDraw.Draw(img).line([(0, 57), (493, 57)], fill=(226, 226, 238))
    return img


OUT.mkdir(exist_ok=True)
dialog().save(OUT / "dialog.bmp")
banner().save(OUT / "banner.bmp")
print("ok:", *sorted(p.name for p in OUT.iterdir()))
