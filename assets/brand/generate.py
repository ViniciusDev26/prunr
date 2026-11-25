#!/usr/bin/env python3
"""Generates the prunr brand SVGs in this directory.

The mark is a module graph: a root, two children and four leaves. One leaf is dead code, so
its edge is cut (the coral slash) and the leaf is left hollow. The wordmark is drawn as
monoline strokes, so it does not depend on any installed font.

Run: python3 assets/brand/generate.py
"""

from math import hypot
from pathlib import Path

OUT = Path(__file__).parent

PALETTES = {
    # leaf: live nodes, edge: live edges, dead: dead node and stub, cut: the slash, ink: text
    "light": dict(leaf="#1FA463", edge="#2F4A3B", dead="#A7B3AC", cut="#F05A4F", ink="#10231A"),
    "dark": dict(leaf="#3DDC84", edge="#9FB8A9", dead="#56665D", cut="#FF6B5E", ink="#ECF3EE"),
}
BACKGROUND = "#10231A"

ROOT = (64, 20)
LEFT, RIGHT = (34, 60), (94, 60)
LEAVES = [(16, 104), (52, 104), (110, 104)]
DEAD = (76, 104)
EDGE_W, NODE_R, LEAF_R = 8, 10.5, 9.5
DEAD_R, DEAD_W = 8, 5


def f(n):
    return f"{n:.1f}".rstrip("0").rstrip(".")


def point(a, b, dist):
    """The point `dist` units away from `a` towards `b`."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    length = hypot(dx, dy)
    return a[0] + dx / length * dist, a[1] + dy / length * dist


def line(a, b):
    return f"M{f(a[0])} {f(a[1])}L{f(b[0])} {f(b[1])}"


def mark(p):
    live = [(ROOT, LEFT), (ROOT, RIGHT), (LEFT, LEAVES[0]), (LEFT, LEAVES[1]), (RIGHT, LEAVES[2])]
    length = hypot(DEAD[0] - RIGHT[0], DEAD[1] - RIGHT[1])
    gap_start, gap_end = length * 0.34, length * 0.62
    stub_end = length - DEAD_R - DEAD_W / 2 - 1.5
    mid = point(RIGHT, DEAD, (gap_start + gap_end) / 2)
    ux, uy = (DEAD[0] - RIGHT[0]) / length, (DEAD[1] - RIGHT[1]) / length
    half = 10
    slash = ((mid[0] - uy * half, mid[1] + ux * half), (mid[0] + uy * half, mid[1] - ux * half))
    nodes = [(ROOT, NODE_R + 1), (LEFT, NODE_R), (RIGHT, NODE_R)] + [(l, LEAF_R) for l in LEAVES]
    return "\n".join(
        [
            '<g fill="none" stroke-linecap="round">',
            f'  <path d="{"".join(line(a, b) for a, b in live)}'
            f'{line(RIGHT, point(RIGHT, DEAD, gap_start))}" '
            f'stroke="{p["edge"]}" stroke-width="{EDGE_W}"/>',
            f'  <path d="{line(point(RIGHT, DEAD, gap_end), point(RIGHT, DEAD, stub_end))}" '
            f'stroke="{p["dead"]}" stroke-width="{EDGE_W}"/>',
            f'  <circle cx="{f(DEAD[0])}" cy="{f(DEAD[1])}" r="{DEAD_R}" '
            f'stroke="{p["dead"]}" stroke-width="{DEAD_W}"/>',
            f'  <path d="{line(*slash)}" stroke="{p["cut"]}" stroke-width="6"/>',
            "</g>",
            f'<g fill="{p["leaf"]}">',
            *(f'  <circle cx="{f(c[0])}" cy="{f(c[1])}" r="{f(r)}"/>' for c, r in nodes),
            "</g>",
        ]
    )


# Monoline lowercase "prunr": x-height 0..56, descender to 84, stroke 14.
WORDMARK_PATHS = [
    "M0 0V84",
    "M0 28a28 28 0 1 0 56 0a28 28 0 1 0-56 0",
    "M78 56V28A28 28 0 0 1 106 0h6",
    "M134 0v28a28 28 0 0 0 56 0M190 0v56",
    "M212 0v56M212 28a28 28 0 0 1 56 0v28",
    "M290 56V28A28 28 0 0 1 318 0h6",
]


def wordmark(color):
    return (
        f'<g fill="none" stroke="{color}" stroke-width="14" stroke-linecap="round" '
        f'stroke-linejoin="round">\n  <path d="{"".join(WORDMARK_PATHS)}"/>\n</g>'
    )


def svg(view_box, body, title="prunr"):
    indented = "\n".join("  " + l for l in body.splitlines())
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" role="img" '
        f'aria-label="{title}">\n  <title>{title}</title>\n{indented}\n</svg>\n'
    )


def group(transform, body):
    inner = "\n".join("  " + l for l in body.splitlines())
    return f'<g transform="{transform}">\n{inner}\n</g>'


def lockup(p):
    # Mark at 128px; the wordmark is centered on the mark's optical middle.
    return svg("0 0 500 128", mark(p) + "\n" + group("translate(164 34)", wordmark(p["ink"])))


def icon():
    background = f'<rect width="128" height="128" rx="28" fill="{BACKGROUND}"/>'
    return svg("0 0 128 128", background + "\n" + group("translate(9.6 11.7) scale(.85)",
                                                        mark(PALETTES["dark"])))


def social():
    # GitHub social preview, 1280x640. The tagline uses a system font: export it to PNG locally.
    p = PALETTES["dark"]
    body = "\n".join([
        f'<rect width="1280" height="640" fill="{BACKGROUND}"/>',
        group("translate(250 190) scale(1.56)", mark(p) + "\n" + group("translate(164 34)",
                                                                       wordmark(p["ink"]))),
        f'<text x="640" y="490" text-anchor="middle" fill="{p["edge"]}" '
        'font-family="JetBrains Mono, ui-monospace, monospace" font-size="34">'
        "dead code detector for Node.js, CommonJS first</text>",
    ])
    return svg("0 0 1280 640", body)


def main():
    files = {
        "mark.svg": svg("0 0 128 128", mark(PALETTES["light"])),
        "mark-dark.svg": svg("0 0 128 128", mark(PALETTES["dark"])),
        "logo.svg": lockup(PALETTES["light"]),
        "logo-dark.svg": lockup(PALETTES["dark"]),
        "wordmark.svg": svg("-7 -7 338 98", wordmark(PALETTES["light"]["ink"])),
        "wordmark-dark.svg": svg("-7 -7 338 98", wordmark(PALETTES["dark"]["ink"])),
        "icon.svg": icon(),
        "social-preview.svg": social(),
    }
    for name, content in files.items():
        (OUT / name).write_text(content)
        print(name)


if __name__ == "__main__":
    main()
