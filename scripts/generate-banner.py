"""Rebuild the self-contained vector banner using the project's Geist font.

Authoring dependency only: fonttools[woff]. No font files or raster images are
embedded in the SVG; the lettering is converted to vector paths.
"""

from pathlib import Path
from xml.etree import ElementTree

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
FONT = ROOT / "node_modules/@fontsource-variable/geist/files/geist-latin-wght-normal.woff2"


def lettering(text, x, baseline, size, weight, color, tracking=0):
    font = TTFont(FONT)
    glyphs = font.getGlyphSet(location={"wght": weight})
    cmap = font.getBestCmap()
    scale = size / font["head"].unitsPerEm
    paths = []
    for character in text:
        glyph = glyphs[cmap[ord(character)]]
        pen = SVGPathPen(glyphs, ntos=lambda n: f"{n:.3f}".rstrip("0").rstrip("."))
        glyph.draw(TransformPen(pen, (scale, 0, 0, -scale, x, baseline)))
        commands = pen.getCommands()
        if commands:
            paths.append(f'<path d="{commands}"/>')
        x += glyph.width * scale + tracking
    font.close()
    return f'<g fill="{color}">' + "".join(paths) + "</g>"


def main():
    icon = ElementTree.parse(ROOT / "app-icon.svg").getroot()
    mark = icon.find("{http://www.w3.org/2000/svg}path").attrib["d"]
    title = lettering("Winget Updater", 292, 230, 88, 700, "#F3F5F0", -1.3)
    tagline = lettering("Your software. Up to date.", 296, 302, 40, 420, "#C9F46A", -0.4)
    sources = lettering("winget / Scoop / Chocolatey / npm / pip", 78, 414, 20, 450, "#ADB6A5", 0.1)
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="480" viewBox="0 0 1600 480" role="img" aria-labelledby="title description">
  <title id="title">Winget Updater — Your software. Up to date.</title>
  <desc id="description">Lime WU monogram with an upward arrow on graphite. Updates for winget, Scoop, Chocolatey, npm, and pip.</desc>
  <defs><clipPath id="canvas"><rect width="1600" height="480" rx="24"/></clipPath></defs>
  <g clip-path="url(#canvas)">
    <path fill="#181B19" d="M0 0H1600V480H0Z"/>
    <path fill="#252E22" d="M1044 480H1240L1600 120V0H1524Z"/>
    <path fill="#303D26" d="M1250 480H1346L1600 226V130Z"/>
    <path fill="#C9F46A" d="M1394 94H1508L1600 2V0H1488Z"/>
    <path fill="#C9F46A" d="M1192 480H1306L1526 260H1412Z"/>
    <path fill="#52663B" d="M1146 348H1220L1272 296H1198Z"/>
    <path d="{mark}" transform="translate(48 112) scale(.85)" fill="#C9F46A"/>
    {title}
    {tagline}
    {sources}
  </g>
</svg>
'''
    (ROOT / "docs/branding/readme-banner.svg").write_text(svg, encoding="utf-8")


if __name__ == "__main__":
    main()
