"""Render Markdown docs as branded SVG images, so a GitHub README looks like the product's website.

Sources live in <src>/*.md (edit those). Each ## section becomes one SVG drawn in the brand from
readme.brand.json (colours, embedded fonts, logo). The published .md files are rewritten as a column of
images; links stay clickable under each image and code blocks stay copyable in a fold.

Usage:
  python render_docs.py                      # uses ./readme.brand.json
  python render_docs.py --config path.json   # paths in the config are relative to the config file
  python render_docs.py --check              # self-check, no fonts needed

Needs: pip install fonttools (and brotli when a font file is .woff2).
Adapted from Netra's docs/render.py (MIT, (c) 2026 rofiperlungoding, github.com/rofiperlungoding/climatechain-id).
"""
import base64
import html
import io
import json
import os
import re
import sys
from pathlib import Path

# ---------- config ----------
DEFAULT = {
    "src": "docs/src", "out": "docs/assets/md", "docs": {"README.md": "README.md"},
    "width": 1280, "padding": 56, "radius": 24,
    "colors": {"ink": "#121212", "paper": "#fbfaf7", "muted": "#55524c", "line": "#e6e2da", "accent": "#e8590c",
               "accent_text": "#c2410c", "card": "#ffffff", "row_alt": "#faf8f4", "code_bg": "#121212",
               "code_text": "#fbfaf7", "code_comment": "#a8a39a"},
    "fonts": {"sans": {"family": "Helvetica"}, "mono": {"family": "ui-monospace"}},
    "metrics": {"sans": 0.53, "sans_bold": 0.555, "mono": 0.6},
    "logo": None,  # {"svg": "<shapes in a 28x28 box, may use {ink} {accent} {paper}>", "wordmark": "name"}
}
CFG, ROOT, C = DEFAULT, Path.cwd(), DEFAULT["colors"]
W = PAD = 0
SANS = MONO = ""


def load(path):
    global CFG, ROOT, C, W, PAD, SANS, MONO
    cfg = json.loads(Path(path).read_text(encoding="utf-8-sig")) if path else {}
    CFG = {**DEFAULT, **cfg, "colors": {**DEFAULT["colors"], **cfg.get("colors", {})},
           "metrics": {**DEFAULT["metrics"], **cfg.get("metrics", {})}, "fonts": {**DEFAULT["fonts"], **cfg.get("fonts", {})}}
    ROOT = Path(path).resolve().parent if path else Path.cwd()
    C, W, PAD = CFG["colors"], CFG["width"], CFG["padding"]
    SANS = f'{CFG["fonts"]["sans"]["family"]}, Helvetica, Arial, sans-serif'
    MONO = f'{CFG["fonts"]["mono"]["family"]}, ui-monospace, monospace'


# ---------- fonts ----------
def font(file, chars):
    """The brand font cut down to the characters this image uses (keeps each SVG small). Returns (base64, format)."""
    from fontTools.subset import Options, Subsetter
    from fontTools.ttLib import TTFont
    try:
        import brotli  # noqa: F401  woff2 needs it, plain woff only needs zlib
        flavor = "woff2"
    except ImportError:
        if file.lower().endswith(".woff2"):
            sys.exit(f"{file} is woff2: run `pip install brotli`, or point the config at a .ttf/.otf/.woff file.")
        flavor = "woff"
    f = TTFont(ROOT / file, recalcTimestamp=False)  # same input -> same bytes, so a re-render only touches changed images
    sub = Subsetter(Options(flavor=flavor, layout_features=["*"]))
    sub.populate(text=chars + " ")
    sub.subset(f)
    buf = io.BytesIO()
    f.flavor = flavor
    f.save(buf)
    return base64.b64encode(buf.getvalue()).decode(), flavor


def fonts(chars):
    """@font-face blocks for every configured font that has a file; families without a file fall back to system fonts."""
    out = []
    for f in CFG["fonts"].values():
        if not f.get("file"):
            continue
        data, flavor = font(f["file"], chars)
        weight = f';font-weight:{f["weight"]}' if f.get("weight") else ""
        out.append(f'@font-face{{font-family:"{f["family"]}"{weight};src:url(data:font/{flavor};base64,{data}) format("{flavor}")}}')
    # No ligatures: JetBrains Mono would draw "--release" as one long dash.
    return f"<style>{''.join(out)}text{{font-variant-ligatures:none;font-feature-settings:'calt' 0,'liga' 0}}</style>" if out else ""


# ---------- inline markdown -> runs ----------
INLINE = re.compile(r"(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\)|\*[^*\s][^*]*\*|_[^_\s][^_]*_)")


def runs(text, links):
    """Split inline markdown into (text, style) runs; style in {"", "b", "i", "code", "link"}. Collect links."""
    out = []
    text = re.sub(r"<[^>]+>", "", text)  # stray inline HTML
    for part in INLINE.split(text):
        if not part:
            continue
        if part.startswith("**"):
            out.append((part[2:-2], "b"))
        elif part.startswith("`"):
            out.append((part[1:-1], "code"))
        elif part.startswith("["):
            t, u = re.match(r"\[([^\]]+)\]\(([^)]+)\)", part).groups()
            links.append((re.sub(r"[*`_]", "", t), u))
            out.append((re.sub(r"[*`]", "", t), "link"))
        elif part[0] in "*_" and len(part) > 2:
            out.append((part[1:-1], "i"))
        else:
            out.append((html.unescape(part), ""))
    return out


def tw(s, size, style=""):
    """Approximate text width in px. Tune metrics.sans / sans_bold / mono in the config if lines run long or short."""
    m = CFG["metrics"]
    if style == "code":
        return len(s) * size * m["mono"]
    k = m["sans_bold"] if style == "b" else m["sans"]
    narrow = sum(1 for c in s if c in "iljtf.,:;'|!()[] ")
    wide = sum(1 for c in s if c in "mwMW@%")
    return (len(s) - narrow * 0.45 + wide * 0.45) * size * k


def wrap(rs, width, size):
    """Word-wrap runs into lines of runs."""
    lines, cur, x = [], [], 0.0
    for text, st in rs:
        for word in re.split(r"(\s+)", text):
            if not word:
                continue
            w = tw(word, size, st)
            if word.isspace():
                if cur:
                    cur.append((" ", st)); x += tw(" ", size, st)
                continue
            if x + w > width and cur:
                while cur and cur[-1][0] == " ":
                    cur.pop()
                lines.append(cur); cur, x = [], 0.0
            cur.append((word, st)); x += w
    if cur:
        lines.append(cur)
    return lines or [[("", "")]]


def tspans(line, size):
    """SVG tspans for one wrapped line."""
    out = []
    for text, st in line:
        t = html.escape(text, quote=False)
        if st == "b":
            out.append(f'<tspan font-weight="700">{t}</tspan>')
        elif st == "i":
            out.append(f'<tspan font-style="italic">{t}</tspan>')
        elif st == "code":
            out.append(f'<tspan font-family="{MONO}" font-size="{size * .9:.1f}" fill="{C["accent_text"]}">{t}</tspan>')
        elif st == "link":
            out.append(f'<tspan fill="{C["accent_text"]}" text-decoration="underline">{t}</tspan>')
        else:
            out.append(t)
    return "".join(out)


# ---------- blocks ----------
class Canvas:
    def __init__(self):
        self.y, self.parts = PAD, []

    def text_block(self, rs, size=22, lh=1.6, x=None, width=None, color=None, weight=400, gap=14):
        x, width, color = PAD if x is None else x, W - 2 * PAD if width is None else width, color or C["ink"]
        for line in wrap(rs, width, size):
            self.y += size * lh * (0.78 if not self.parts else 1)
            self.parts.append(f'<text x="{x}" y="{self.y:.0f}" font-size="{size}" font-weight="{weight}" fill="{color}">{tspans(line, size)}</text>')
        self.y += gap

    def heading(self, text, level, links):
        if level == 1:
            logo = CFG.get("logo")
            if logo:
                mark = logo.get("svg", "").replace("{ink}", C["ink"]).replace("{accent}", C["accent"]).replace("{paper}", C["paper"])
                word = logo.get("wordmark", "")
                wx = 36 if mark else 0
                self.parts.append(f'<g transform="translate({PAD} {self.y})">{mark}'
                                  + (f'<text x="{wx}" y="21" font-size="20" font-weight="600" letter-spacing="-.8" fill="{C["ink"]}">{html.escape(word)}</text>' if word else "")
                                  + "</g>")
                self.y += 46
            self.text_block(runs(text, links), size=54, lh=1.15, weight=600, gap=12)
        elif level == 2:
            self.text_block(runs(text, links), size=40, lh=1.2, weight=600, gap=20)
        else:
            self.y += 6
            self.text_block(runs(text, links), size=27, lh=1.3, weight=700, gap=12)

    def bullets(self, items, links, ordered=False):
        for n, it in enumerate(items, 1):
            y0 = self.y
            mark = f'<text x="{PAD}" y="{self.y + 33:.0f}" font-size="21" font-weight="700" fill="{C["accent_text"]}">{n}.</text>' if ordered else \
                f'<circle cx="{PAD + 6}" cy="{self.y + 26:.0f}" r="4.5" fill="{C["accent"]}"/>'
            self.parts.append(mark)
            start = len(self.parts)
            self.text_block(runs(it, links), size=22, x=PAD + 32, width=W - 2 * PAD - 32, gap=6)
            if start == len(self.parts):
                self.y = y0
        self.y += 10

    def table(self, rows, links):
        head, body = rows[0], rows[1:]
        n = len(head)
        cells = [[runs(c, links) for c in r] for r in [head] + body]
        x0, size, lh, pad = PAD, 19, 1.5, 16
        avail = W - 2 * PAD
        # each column at least as wide as its longest unbreakable word; the rest shared by text length
        minw = [max(tw(w, size, st if ri else "b") for ri in range(len(cells)) for t, st in cells[ri][c] for w in t.split() or [""]) + 2 * pad + 4
                for c in range(n)]
        lens = [min(max(sum(len(t) for t, _ in cells[r][c]) for r in range(len(cells))), 60) or 1 for c in range(n)]
        spare = max(avail - sum(minw), 0)
        cw = [m + spare * l / sum(lens) for m, l in zip(minw, lens)]
        cw = [w * avail / sum(cw) for w in cw]
        top = self.y
        out = []
        for ri, r in enumerate(cells):
            wrapped = [wrap(r[c], cw[c] - 2 * pad, size) for c in range(n)]
            h = max(len(w) for w in wrapped) * size * lh + 2 * pad - 4
            fill = C["ink"] if ri == 0 else (C["card"] if ri % 2 else C["row_alt"])
            out.append(f'<rect x="{x0}" y="{self.y:.0f}" width="{avail}" height="{h:.0f}" fill="{fill}"/>')
            x = x0
            for c in range(n):
                yy = self.y + pad + size
                for line in wrapped[c]:
                    color = C["paper"] if ri == 0 else C["ink"]
                    weight = 700 if ri == 0 or c == 0 else 400
                    out.append(f'<text x="{x + pad:.0f}" y="{yy:.0f}" font-size="{size}" font-weight="{weight}" fill="{color}">{tspans(line, size)}</text>')
                    yy += size * lh
                x += cw[c]
            self.y += h
        self.parts.append(f'<clipPath id="t{top:.0f}"><rect x="{x0}" y="{top:.0f}" width="{avail}" height="{self.y - top:.0f}" rx="14"/></clipPath>'
                          f'<g clip-path="url(#t{top:.0f})">{"".join(out)}</g>'
                          f'<rect x="{x0}" y="{top:.0f}" width="{avail}" height="{self.y - top:.0f}" rx="14" fill="none" stroke="{C["line"]}"/>')
        self.y += 22

    def code(self, lines):
        size, lh = 18, 1.6
        top = self.y
        maxc = int((W - 2 * PAD - 56) / (size * 0.64))
        shown = []
        for l in lines:
            m = re.match(r"^(.*?\S)(\s+#.*)$", l)
            if m and len(l) > maxc:  # long line: its comment goes on its own line above
                shown.append(re.match(r"^\s*", l).group(0) + m.group(2).strip())
                l = m.group(1)
            while len(l) > maxc:  # still long: break at the last space
                cut = l.rfind(" ", 0, maxc)
                cut = cut if cut > maxc // 2 else maxc
                shown.append(l[:cut]); l = "    " + l[cut:].lstrip()
            shown.append(l)
        h = len(shown) * size * lh + 36
        self.parts.append(f'<rect x="{PAD}" y="{top:.0f}" width="{W - 2 * PAD}" height="{h:.0f}" rx="14" fill="{C["code_bg"]}"/>')
        y = top + 18 + size
        for l in shown:
            m = re.match(r"^(.*?)(\s+#.*)?$", l)
            cmd, comment = m.group(1), m.group(2) or ""
            if cmd.lstrip().startswith("#"):
                cmd, comment = "", cmd
            self.parts.append(f'<text x="{PAD + 24}" y="{y:.0f}" font-family="{MONO}" font-size="{size}" fill="{C["code_text"]}" xml:space="preserve">'
                              f'{html.escape(cmd, quote=False)}<tspan fill="{C["code_comment"]}">{html.escape(comment, quote=False)}</tspan></text>')
            y += size * lh
        self.y += h + 22

    def svg(self, label, embed=True):
        h = self.y + PAD - 22
        r = CFG["radius"]
        chars = "".join(sorted(set(html.unescape("".join(re.findall(r">([^<]+)<", "".join(self.parts)))))))
        return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {h:.0f}" width="{W}" height="{h:.0f}" role="img" aria-label="{html.escape(label)}">'
                + (fonts(chars) if embed else "")
                + f'<rect width="{W}" height="{h:.0f}" rx="{r}" fill="{C["paper"]}"/><rect x=".5" y=".5" width="{W - 1}" height="{h - 1:.0f}" rx="{r}" fill="none" stroke="{C["line"]}"/>'
                + f'<g font-family="{SANS}">' + "".join(self.parts) + "</g></svg>")


# ---------- document ----------
def sections(md):
    """Yield ("sec", [lines]) per ## section and ("img", line) for diagrams; a section's diagrams follow its text."""
    cur, imgs = [], []
    for line in md.splitlines():
        if line.strip().startswith('<p align="center"><img'):
            if any(l.strip() for l in cur):
                imgs.append(line)
            else:
                yield "img", line
            continue
        if line.startswith("## ") and any(l.strip() for l in cur):
            yield "sec", cur
            for im in imgs:
                yield "img", im
            cur, imgs = [], []
        if line.strip() == "---":
            continue
        cur.append(line)
    if any(l.strip() for l in cur):
        yield "sec", cur
    for im in imgs:
        yield "img", im


def render_section(lines, embed=True):
    c, links, code_blocks, title, plain = Canvas(), [], [], "", []
    i = 0
    while i < len(lines):
        line = lines[i]
        s = line.strip()
        if not s or s.startswith("<p align=\"center\"><a") or s.startswith("<!--"):
            if s.startswith("<p align=\"center\"><a"):  # nav line of links: keep them as links under the image
                links.extend((re.sub(r"<[^>]+>", "", t), u) for u, t in re.findall(r'<a href="([^"]+)">(.*?)</a>', s))
            i += 1
            continue
        if s.startswith("```"):
            j = i + 1
            block = []
            while j < len(lines) and not lines[j].strip().startswith("```"):
                block.append(lines[j]); j += 1
            c.code(block); code_blocks.append((s[3:].strip() or "bash", block)); i = j + 1
            continue
        m = re.match(r"^(#{1,3}) (.*)", s)
        if m:
            title = title or re.sub(r"[*`]", "", m.group(2))
            c.heading(m.group(2), len(m.group(1)), links); i += 1
            continue
        if s.startswith("|"):
            rows = []
            while i < len(lines) and lines[i].strip().startswith("|"):
                r = [x.strip().replace("\\|", "|") for x in re.split(r"(?<!\\)\|", lines[i].strip().strip("|"))]
                if not all(re.fullmatch(r":?-{3,}:?", x) for x in r):
                    rows.append(r)
                i += 1
            c.table(rows, links); plain.append(" ".join(" ".join(r) for r in rows[:3]))
            continue
        if re.match(r"^(\d+\.|[-*]) ", s):
            ordered = bool(re.match(r"^\d+\.", s))
            items = []
            while i < len(lines) and (re.match(r"^\s*(\d+\.|[-*]) ", lines[i]) or (lines[i].startswith("   ") and lines[i].strip())):
                t = lines[i].strip()
                if re.match(r"^(\d+\.|[-*]) ", t):
                    items.append(re.sub(r"^(\d+\.|[-*]) ", "", t))
                else:
                    items[-1] += " " + t
                i += 1
            c.bullets(items, links, ordered); plain.extend(items)
            continue
        if s.startswith(">"):
            c.text_block(runs(s.lstrip("> "), links), size=22, color=C["muted"]); i += 1
            continue
        para = [s]
        i += 1
        while i < len(lines) and lines[i].strip() and not re.match(r"^(#|\||```|\d+\. |[-*] |<p )", lines[i].strip()):
            para.append(lines[i].strip()); i += 1
        c.text_block(runs(" ".join(para), links)); plain.append(" ".join(para))
    if not c.parts:
        return "", title, "", links, code_blocks
    alt = (title + ": " if title and plain else title) + re.sub(r"[*`]", "", re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", " ".join(plain)))
    alt = "".join(ch for ch in alt if ch >= " ")
    return c.svg(alt[:300], embed), title, alt[:600], links, code_blocks


def build(src, dst, script_rel, embed=True):
    """Render one source document. Returns (published markdown, {svg name: svg})."""
    md = (ROOT / CFG["src"] / src).read_text(encoding="utf-8-sig")
    dst_path = ROOT / dst
    rel = os.path.relpath(ROOT / CFG["out"], dst_path.parent).replace("\\", "/")
    stem = re.sub(r"[^a-z0-9]+", "-", Path(src).stem.lower())
    out, n, svgs = [f"<!-- Generated by {script_rel} from {CFG['src']}/{src}. Edit the source, then run: python {script_rel} -->", ""], 0, {}
    for kind, val in sections(md):
        if kind == "img":
            out += [val, ""]
            continue
        svg, title, alt, links, code = render_section(val, embed)
        if not svg:  # a section that holds only links: keep them as text
            out += [f'<p align="center">{" · ".join(f"<a href=\"{u}\">{html.escape(t)}</a>" for t, u in links)}</p>', ""]
            continue
        n += 1
        name = f"{stem}-{n:02d}.svg"
        svgs[name] = svg
        anchor = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")
        out.append(f'<a id="{anchor}"></a>' if anchor else "")
        out.append(f'<p align="center"><img src="{rel}/{name}" alt="{html.escape(alt, quote=True)}" width="100%"></p>')
        seen, ls = set(), []
        for t, u in links:
            if u not in seen:
                seen.add(u); ls.append(f'<a href="{u}">{html.escape(t)}</a>')
        if ls:
            out.append(f'<p align="center"><sub>{" · ".join(ls)}</sub></p>')
        for lang, block in code:
            out += ["<details><summary>Copy the commands</summary>", "", f"```{lang}", *block, "```", "", "</details>"]
        out.append("")
    return "\n".join(out), svgs


def main(argv):
    if "--check" in argv:
        load(None)
        assert tw("abc", 10) > 0 and len(wrap([("one two three four", "")], 40, 10)) > 1
        links = []
        assert runs("see **bold** and [docs](d.md)", links)[1] == ("bold", "b") and links == [("docs", "d.md")]
        md = '# T\n\n<p align="center"><a href="https://x.dev"><b>Open</b></a> · <a href="d.md">Docs</a></p>\n\n## Run\n\n```bash\nnpm test   # all\n```\n'
        secs = list(sections(md))
        _, _, _, nav_links, _ = render_section(secs[0][1], embed=False)
        assert nav_links == [("Open", "https://x.dev"), ("Docs", "d.md")], nav_links  # (text, url), no swap heuristics
        svg, title, _, _, code = render_section(secs[1][1], embed=False)
        assert title == "Run" and code == [("bash", ["npm test   # all"])] and svg.startswith("<svg")
        print("ok")
        return
    i = argv.index("--config") + 1 if "--config" in argv else None
    cfg = argv[i] if i else "readme.brand.json"
    if not Path(cfg).exists():
        sys.exit(f"{cfg} not found: copy readme.brand.example.json from the skill and fill in the brand.")
    load(cfg)
    script = Path(__file__).resolve()
    script_rel = os.path.relpath(script, ROOT).replace("\\", "/") if ROOT in script.parents else "render_docs.py"
    out_dir = ROOT / CFG["out"]
    # render everything in memory first: a crash (missing font, missing brotli) leaves the old images in place
    pages, images = {}, {}
    for s, d in CFG["docs"].items():
        pages[d], svgs = build(s, d, script_rel)
        images.update(svgs)
        print(d, len(svgs), "sections")
    out_dir.mkdir(parents=True, exist_ok=True)
    for old in out_dir.glob("*.svg"):
        if old.name not in images:
            old.unlink()
    for name, svg in images.items():
        (out_dir / name).write_text(svg, encoding="utf-8")
    for d, md in pages.items():
        (ROOT / d).write_text(md, encoding="utf-8")
    kb = sum(len(s) for s in images.values()) // 1024
    print(f"{len(images)} images, {kb} KB in {CFG['out']}")


if __name__ == "__main__":
    main(sys.argv[1:])
