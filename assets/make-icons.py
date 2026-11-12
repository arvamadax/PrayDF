# Buat ikon dari assets/icon.svg: native/praydf.ico dan extension/icons/*.png.
# PyMuPDF tidak merender gradien SVG, jadi bentuk dirender olehnya dan latar gradien diisi Pillow.
# Jalankan dari root repo: python assets/make-icons.py
import pathlib

import fitz
from PIL import Image

root = pathlib.Path(__file__).resolve().parent.parent
svg = (root / "assets" / "icon.svg").read_text(encoding="utf-8")
N = 1024


def render(text):
    pix = fitz.open(stream=text.encode(), filetype="svg")[0].get_pixmap(matrix=fitz.Matrix(N / 256, N / 256), alpha=True)
    return Image.frombytes("RGBA", (pix.width, pix.height), pix.samples)


mask = render(svg.replace('fill="url(#bg)"', 'fill="#ffffff"')).getchannel("A")  # kotak sudut membulat
shapes = render(svg.replace('fill="url(#bg)"', 'fill="none"'))  # halaman + huruf P

# Gradien diagonal #3b82f6 -> #7c3aed
a, b = (0x3B, 0x82, 0xF6), (0x7C, 0x3A, 0xED)
v = Image.linear_gradient("L")  # atas -> bawah
grad = Image.blend(v, v.transpose(Image.Transpose.ROTATE_90), 0.5).resize((N, N))
bg = Image.composite(Image.new("RGB", (N, N), b), Image.new("RGB", (N, N), a), grad).convert("RGBA")
bg.putalpha(mask)
icon = Image.alpha_composite(bg, shapes)

(root / "extension" / "icons").mkdir(exist_ok=True)
for s in (16, 32, 48, 128):
    icon.resize((s, s), Image.LANCZOS).save(root / "extension" / "icons" / f"icon{s}.png")
icon.resize((256, 256), Image.LANCZOS).save(root / "native" / "praydf.ico", sizes=[(s, s) for s in (16, 20, 24, 32, 40, 48, 64, 128, 256)])
icon.resize((512, 512), Image.LANCZOS).save(root / "assets" / "icon.png")
print("ok")
