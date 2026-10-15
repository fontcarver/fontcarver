#!/usr/bin/env python3
"""Extract real glyph outlines for the FontCarver website hero.

Development tool. It is not deployed and CI never runs it. It reads two
OFL-licensed fonts and records one Latin and one CJK glyph exactly as the font
stores it: every on-curve and off-curve point, the implied on-curve points a
TrueType contour leaves out, and whether each node is a corner or a smooth
node. Bitmap strikes are not recorded here: the page rasterises the outline it
is currently showing, so a dragged point changes the pixels too. It writes
`site/src/data/specimens.ts`, which the drawing component imports; the drawing
itself, static and live, is rendered from that data by
`site/src/components/GlyphDrawing.svelte`.

Run it from the repository root:

    python3 site/tools/extract-outlines.py \
        --latin site/public/assets/fonts/source-serif-4-variable.woff2 \
        --cjk /usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc \
        --cjk-font-number 2

Requires fontTools and brotli, which are development dependencies only.
"""

from __future__ import annotations

import argparse
import html
import json
import math
import pathlib

from fontTools.pens.recordingPen import RecordingPen
from fontTools.ttLib import TTFont

# Two tangents closer than this are the same direction, which is what makes a
# node smooth rather than a corner. The editor decides it the same way.
SMOOTH_TOLERANCE_DEGREES = 4.0

# One frame for both specimens, in font units with y already flipped for SVG,
# so switching specimen never changes the figure's size on the page. Both
# fonts have a 1000 unit em.
FRAME = (-250, -940, 1420, 1320)
GUIDE_LEFT = -210
GUIDE_RIGHT = 1130
LABEL_SIZE = 30

def angle_between(ax: float, ay: float, bx: float, by: float) -> float:
    """Angle in degrees between two vectors; 180 if either is degenerate."""
    la = math.hypot(ax, ay)
    lb = math.hypot(bx, by)
    if la < 1e-9 or lb < 1e-9:
        return 180.0
    cos = max(-1.0, min(1.0, (ax * bx + ay * by) / (la * lb)))
    return math.degrees(math.acos(cos))


class Outline:
    """A glyph as nodes plus drawing commands that index into them.

    Commands are the SVG ones the font's own segments map onto: `M`, `L`, `Q`,
    `C` and `Z`. Keeping the indices rather than a path string is what lets the
    page redraw the outline after someone drags a point.
    """

    def __init__(self) -> None:
        self.nodes: list[dict] = []
        self.commands: list[list] = []
        self.handles: list[list[int]] = []
        self.contour_start: int | None = None
        self.current: int | None = None

    def add(self, x: float, y: float, kind: str) -> int:
        self.nodes.append({"x": round(x, 1), "y": round(y, 1), "t": kind, "s": False})
        return len(self.nodes) - 1

    def move_to(self, point) -> None:
        index = self.add(point[0], point[1], "on")
        self.contour_start = index
        self.current = index
        self.commands.append(["M", index])

    def line_to(self, point) -> None:
        index = self.add(point[0], point[1], "on")
        self.commands.append(["L", index])
        self.current = index

    def cubic_to(self, c1, c2, end) -> None:
        i1 = self.add(c1[0], c1[1], "off")
        i2 = self.add(c2[0], c2[1], "off")
        i3 = self.add(end[0], end[1], "on")
        self.commands.append(["C", i1, i2, i3])
        self.handles.append([self.current, i1])
        self.handles.append([i3, i2])
        self.current = i3

    def quad_run(self, points) -> None:
        """One qCurveTo, split at the implied on-curve points it leaves out."""
        controls = list(points[:-1])
        end = points[-1]
        for position, control in enumerate(controls):
            control_index = self.add(control[0], control[1], "off")
            if position == len(controls) - 1:
                end_index = self.add(end[0], end[1], "on")
            else:
                nxt = controls[position + 1]
                end_index = self.add(
                    (control[0] + nxt[0]) / 2, (control[1] + nxt[1]) / 2, "implied"
                )
            self.commands.append(["Q", control_index, end_index])
            self.handles.append([self.current, control_index])
            self.handles.append([end_index, control_index])
            self.current = end_index

    def close(self) -> None:
        """Close the contour, folding a duplicated last point onto the first."""
        start = self.contour_start
        if start is not None and self.current is not None and self.current != start:
            here = self.nodes[self.current]
            there = self.nodes[start]
            if abs(here["x"] - there["x"]) < 0.6 and abs(here["y"] - there["y"]) < 0.6:
                self.replace(self.current, start)
        self.commands.append(["Z"])
        self.current = None
        self.contour_start = None

    def replace(self, old: int, new: int) -> None:
        """Point every reference at `new` and drop `old` if it is now unused."""
        for command in self.commands:
            for position in range(1, len(command)):
                if command[position] == old:
                    command[position] = new
        for handle in self.handles:
            for position in (0, 1):
                if handle[position] == old:
                    handle[position] = new
        if old == len(self.nodes) - 1:
            self.nodes.pop()

    def classify(self) -> None:
        """Mark the on-curve nodes whose two handles continue one direction."""
        attached: dict[int, list[int]] = {}
        for on_index, off_index in self.handles:
            attached.setdefault(on_index, []).append(off_index)
        for index, node in enumerate(self.nodes):
            if node["t"] == "off":
                continue
            controls = attached.get(index, [])
            if len(controls) != 2:
                continue
            first = self.nodes[controls[0]]
            second = self.nodes[controls[1]]
            node["s"] = (
                angle_between(
                    node["x"] - first["x"],
                    node["y"] - first["y"],
                    second["x"] - node["x"],
                    second["y"] - node["y"],
                )
                <= SMOOTH_TOLERANCE_DEGREES
            )

    def path(self) -> str:
        """The outline as an SVG path, y flipped so a page can draw it."""

        def point(index: int) -> str:
            node = self.nodes[index]
            return f"{round(node['x'])} {round(-node['y'])}"

        out = []
        for command in self.commands:
            if command[0] == "Z":
                out.append("Z")
            else:
                out.append(command[0] + " ".join(point(i) for i in command[1:]))
        return "".join(out)

    def polygons(self, steps: int = 24) -> list[list[tuple[float, float]]]:
        """Every contour as a closed polyline, for measuring the outline."""

        def at(index: int) -> tuple[float, float]:
            node = self.nodes[index]
            return (node["x"], node["y"])

        def bezier(points, t):
            pts = list(points)
            while len(pts) > 1:
                pts = [
                    (
                        pts[i][0] + (pts[i + 1][0] - pts[i][0]) * t,
                        pts[i][1] + (pts[i + 1][1] - pts[i][1]) * t,
                    )
                    for i in range(len(pts) - 1)
                ]
            return pts[0]

        polygons: list[list[tuple[float, float]]] = []
        current: list[tuple[float, float]] = []
        here = (0.0, 0.0)
        for command in self.commands:
            kind = command[0]
            if kind == "M":
                current = [at(command[1])]
                here = at(command[1])
            elif kind == "L":
                current.append(at(command[1]))
                here = at(command[1])
            elif kind in ("Q", "C"):
                run = [here] + [at(i) for i in command[1:]]
                for step in range(1, steps + 1):
                    current.append(bezier(run, step / steps))
                here = at(command[-1])
            elif kind == "Z":
                if len(current) > 2:
                    polygons.append(current)
                current = []
        return polygons


def read_outline(glyph_set, glyph_name: str) -> Outline:
    pen = RecordingPen()
    glyph_set[glyph_name].draw(pen)
    outline = Outline()
    for operator, operands in pen.value:
        if operator == "moveTo":
            outline.move_to(operands[0])
        elif operator == "lineTo":
            outline.line_to(operands[0])
        elif operator == "curveTo":
            points = list(operands)
            # A curveTo with more than three points is a run of cubics.
            for start in range(0, len(points) - 1, 2):
                if start + 2 < len(points):
                    outline.cubic_to(points[start], points[start + 1], points[start + 2])
                else:
                    outline.cubic_to(points[start], points[start], points[-1])
        elif operator == "qCurveTo":
            points = list(operands)
            if points[-1] is None:
                # An all-off-curve contour closes on an implied point.
                points = points[:-1]
                implied = (
                    (points[0][0] + points[-1][0]) / 2,
                    (points[0][1] + points[-1][1]) / 2,
                )
                points = points + [implied]
            outline.quad_run(points)
        elif operator == "closePath":
            outline.close()
    outline.classify()
    return outline


def specimen(path: pathlib.Path, character: str, key: str, font_number: int) -> dict:
    font = TTFont(path, fontNumber=font_number) if font_number >= 0 else TTFont(path)
    upem = font["head"].unitsPerEm
    code = ord(character)
    cmap = font.getBestCmap()
    if code not in cmap:
        raise SystemExit(f"{path.name} has no glyph for U+{code:04X}")
    glyph_name = cmap[code]
    outline = read_outline(font.getGlyphSet(), glyph_name)

    advance = font["hmtx"][glyph_name][0]
    os2 = font["OS/2"]
    ascender = int(getattr(os2, "sTypoAscender", font["hhea"].ascender))
    descender = int(getattr(os2, "sTypoDescender", font["hhea"].descender))
    cap_height = int(getattr(os2, "sCapHeight", 0) or round(upem * 0.7))
    x_height = int(getattr(os2, "sxHeight", 0) or round(upem * 0.5))

    flattened = [point for polygon in outline.polygons() for point in polygon]
    xs = [point[0] for point in flattened]
    ys = [point[1] for point in flattened]

    if key == "latin":
        guides = [
            ["cap height", cap_height],
            ["x-height", x_height],
            ["baseline", 0],
            ["descender", descender],
        ]
    else:
        # Latin cap height and x-height are in the font, but they say nothing
        # about a Han glyph; the em box and the baseline do.
        guides = [["ascender", ascender], ["baseline", 0], ["descender", descender]]

    curve_type = "cubic" if any(c[0] == "C" for c in outline.commands) else "quadratic"
    on_curve = sum(1 for node in outline.nodes if node["t"] == "on")
    implied = sum(1 for node in outline.nodes if node["t"] == "implied")

    family = font["name"].getDebugName(16) or font["name"].getDebugName(1)
    style = font["name"].getDebugName(17) or font["name"].getDebugName(2)

    return {
        "key": key,
        "char": character,
        "glyphName": glyph_name,
        "unicode": f"U+{code:04X}",
        "upem": upem,
        "advance": advance,
        "curveType": curve_type,
        "points": on_curve,
        "implied": implied,
        "bbox": [round(min(xs)), round(min(ys)), round(max(xs)), round(max(ys))],
        "guides": guides,
        "ascender": ascender,
        "descender": descender,
        "nodes": outline.nodes,
        "commands": outline.commands,
        "handles": outline.handles,
        "path": outline.path(),
        "source": f"{family} {style}".strip(),
    }


def centre(value: dict) -> int:
    """Offset that puts the glyph's advance in the middle of the shared frame."""
    return round(FRAME[0] + FRAME[2] / 2 - value["advance"] / 2)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--latin", required=True, type=pathlib.Path)
    parser.add_argument("--cjk", required=True, type=pathlib.Path)
    parser.add_argument("--cjk-font-number", type=int, default=-1)
    parser.add_argument("--latin-font-number", type=int, default=-1)
    parser.add_argument("--latin-char", default="g")
    parser.add_argument("--cjk-char", default="永")
    parser.add_argument(
        "--out", type=pathlib.Path, default=pathlib.Path("site/src/data/specimens.ts")
    )
    args = parser.parse_args()

    data = {
        "frame": list(FRAME),
        "guideLeft": GUIDE_LEFT,
        "guideRight": GUIDE_RIGHT,
        "labelSize": LABEL_SIZE,
        "latin": specimen(args.latin, args.latin_char, "latin", args.latin_font_number),
        "cjk": specimen(args.cjk, args.cjk_char, "cjk", args.cjk_font_number),
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        "/* Generated by site/tools/extract-outlines.py. Do not edit.\n"
        " *\n"
        " * Two real glyphs, read out of OFL-licensed fonts: every coordinate, guide and\n"
        " * command below comes from the font file, not from a drawing of one. See\n"
        " * THIRD_PARTY.md for the faces and site/README.md for the command.\n"
        " */\n"
        "\n"
        "import type { SpecimenSet } from '../lib/glyph';\n"
        "\n"
        "export const specimens: SpecimenSet = "
        f"{json.dumps(data, ensure_ascii=False, separators=(',', ':'))};\n",
        encoding="utf-8",
    )

    for key in ("latin", "cjk"):
        value = data[key]
        print(
            f"{key}: {value['char']} {value['unicode']} from {value['source']} — "
            f"{value['points']} on-curve points ({value['implied']} implied), "
            f"{value['curveType']}, advance {value['advance']}/{value['upem']}"
        )
    print(f"wrote {args.out} ({args.out.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
