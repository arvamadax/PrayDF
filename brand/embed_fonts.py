"""Embed the brand fonts into hand-made SVG diagrams (banner, architecture, flow) so GitHub shows them in the brand type.
GitHub renders SVGs as <img>, which cannot load web fonts; an embedded @font-face can. Idempotent.

Usage: python embed_fonts.py [--config readme.brand.json] docs/assets/*.svg
Uses the "fonts" entries of the config (whole font files; run render_docs.py for per-image subsets).
Adapted from Netra's brand/embed_font.py (MIT, (c) 2026 rofiperlungoding).
"""
import base64
import json
import re
import sys
from pathlib import Path

args = sys.argv[1:]
cfg_path = Path(args[args.index("--config") + 1]) if "--config" in args else Path("readme.brand.json")
files = [a for a in args if a.endswith(".svg")]
cfg = json.loads(cfg_path.read_text(encoding="utf-8"))
root = cfg_path.resolve().parent

faces = []
for f in cfg.get("fonts", {}).values():
    if not f.get("file"):
        continue
    fmt = Path(f["file"]).suffix.lstrip(".").lower().replace("ttf", "truetype").replace("otf", "opentype")
    data = base64.b64encode((root / f["file"]).read_bytes()).decode()
    weight = f';font-weight:{f["weight"]}' if f.get("weight") else ""
    faces.append(f'@font-face{{font-family:"{f["family"]}"{weight};src:url(data:font/{fmt};base64,{data}) format("{fmt}")}}')
block = f'<style id="brand-font">{"".join(faces)}</style>'

for p in map(Path, files):
    s = p.read_text(encoding="utf-8")
    s = re.sub(r'<style id="brand-font">.*?</style>', "", s, flags=re.S)
    s = re.sub(r"(<svg[^>]*>)", lambda m: m.group(1) + block, s, count=1)
    assert s.count('id="brand-font"') == 1, p
    p.write_text(s, encoding="utf-8")
    print(p.name, f"{len(s) // 1024} KB")
