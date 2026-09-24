#!/usr/bin/env python3
"""Extract real glyph outlines for the FontCarver website hero.

Development tool. It is not deployed and CI never runs it. It reads two
OFL-licensed fonts, records the outline of one Latin and one CJK glyph exactly
as the font stores it — on-curve and off-curve points, segment types, smooth
versus corner nodes — rasterises each glyph into small bitmap strikes from the
same outline, and writes:

  site/assets/js/specimens.js      the data the hero reads
  site/assets/svg/hero-<key>.svg   a standalone static figure per specimen

Run it from the repository root:

    python3 site/tools/extract-outlines.py \
        --latin  <path to Source Serif 4 .woff2/.ttf/.otf> \
        --cjk    <path to Noto Serif CJK .ttc/.otf>

Requires fontTools and brotli, which are development dependencies only.
"""

from __future__ import annotations

import argparse
import json
import math
import pathlib
from dataclasses import dataclass, field

from fontTools.pens.recordingPen import RecordingPen
from fontTools.ttLib import TTFont

# Two on-curve tangents closer than this are the same direction, which is what
# makes a node smooth rather than a corner. The editor uses the same idea.
SMOOTH_TOLERANCE_DEGREES = 4.0

# Curves are flattened to this many line segments per segment before the
# scanline fill; far finer than any strike we draw.
FLATTEN_STEPS = 24


@dataclass
class Node:
    x: float
    y: float
    on: bool
    smooth: bool = False
    # Index of the on-curve node this off-curve point belongs to, for the HUD.
    owner: int | None = None


@dataclass
class Contour:
    segments: list = field(default_factory=list)  # (kind, [(x, y), ...])
    nodes: list[Node] = field(default_factory=list)


def angle_between(ax: float, ay: float, bx: float, by: float) -> float:
    """Angle in degrees between two vectors, or 180 if either is degenerate."""
    la = math.hypot(ax, ay)
    lb = math.hypot(bx, by)
    if la < 1e-9 or lb < 1e-9:
        return 180.0
    cos = max(-1.0, min(1.0, (ax * bx + ay * by) / (la * lb)))
    return math.degrees(math.acos(cos))


def read_contours(glyph_set, glyph_name: str) -> list[Contour]:
    """Record one glyph's contours, keeping the font's own segment types."""
    pen = RecordingPen()
    glyph_set[glyph_name].draw(pen)

    contours: list[Contour] = []
    current: Contour | None = None
    start: tuple[float, float] | None = None
    here: tuple[float, float] | None = None

    for operator, operands in pen.value:
        if operator == "moveTo":
            current = Contour()
            contours.append(current)
            start = here = operands[0]
            current.segments.append(("move", [here]))
        elif operator == "lineTo":
            assert current is not None
            here = operands[0]
            current.segments.append(("line", [here]))
        elif operator == "curveTo":
            assert current is not None
            points = list(operands)
            current.segments.append(("cubic", points))
            here = points[-1]
        elif operator == "qCurveTo":
            assert current is not None
            points = list(operands)
            # A trailing None means the contour is all off-curve points and
            # closes on an implied on-curve point; resolve it the way the
            # TrueType rasteriser does.
            if points[-1] is None:
                points = points[:-1]
                implied = (
                    (points[0][0] + points[-1][0]) / 2,
                    (points[0][1] + points[-1][1]) / 2,
                )
                points = points + [implied]
            current.segments.append(("quad", points))
            here = points[-1]
        elif operator == "closePath":
            assert current is not None and start is not None
            if here is not None and (
                abs(here[0] - start[0]) > 1e-6 or abs(here[1] - start[1]) > 1e-6
            ):
                current.segments.append(("line", [start]))
            current.segments.append(("close", []))
            current = None
            here = None

    for contour in contours:
        classify_nodes(contour)
    return contours


def classify_nodes(contour: Contour) -> None:
    """Fill in on-curve nodes (corner or smooth) and their off-curve points."""
    on_points: list[tuple[float, float]] = []
    incoming: list[tuple[float, float] | None] = []
    outgoing: list[tuple[float, float] | None] = []
    off_points: list[tuple[tuple[float, float], int]] = []

    for kind, points in contour.segments:
        if kind == "move":
            on_points.append(points[0])
            incoming.append(None)
            outgoing.append(None)
        elif kind == "line":
            outgoing[-1] = None
            on_points.append(points[0])
            incoming.append(None)
            outgoing.append(None)
        elif kind in ("cubic", "quad"):
            controls = points[:-1]
            end = points[-1]
            outgoing[-1] = controls[0]
            off_points.append((controls[0], len(on_points) - 1))
            on_points.append(end)
            incoming.append(controls[-1])
            off_points.append((controls[-1], len(on_points) - 1))
            outgoing.append(None)

    # The contour is closed, so the first node's incoming handle is the last
    # segment's, and the last point coincides with the first.
    if len(on_points) > 1 and (
        abs(on_points[0][0] - on_points[-1][0]) < 1e-6
        and abs(on_points[0][1] - on_points[-1][1]) < 1e-6
    ):
        incoming[0] = incoming[-1]
        on_points.pop()
        incoming.pop()
        outgoing.pop()
        for index, (point, owner) in enumerate(off_points):
            if owner == len(on_points):
                off_points[index] = (point, 0)

    for index, (x, y) in enumerate(on_points):
        before = incoming[index]
        after = outgoing[index]
        smooth = False
        if before is not None and after is not None:
            smooth = (
                angle_between(x - before[0], y - before[1], after[0] - x, after[1] - y)
                <= SMOOTH_TOLERANCE_DEGREES
            )
        contour.nodes.append(Node(x, y, on=True, smooth=smooth))

    for (x, y), owner in off_points:
        contour.nodes.append(Node(x, y, on=False, owner=min(owner, len(on_points) - 1)))


def svg_path(contours: list[Contour]) -> str:
    """The outline as an SVG path, y flipped so the page can draw it directly."""

    def point(p):
        return f"{round(p[0])} {round(-p[1])}"

    out: list[str] = []
    for contour in contours:
        for kind, points in contour.segments:
            if kind == "move":
                out.append(f"M{point(points[0])}")
            elif kind == "line":
                out.append(f"L{point(points[0])}")
            elif kind == "cubic":
                out.append("C" + " ".join(point(p) for p in points))
            elif kind == "quad":
                # More than one control point means implied on-curve points;
                # emit one Q per pair the way the font means it.
                controls = points[:-1]
                end = points[-1]
                for index, control in enumerate(controls):
                    if index == len(controls) - 1:
                        out.append(f"Q{point(control)} {point(end)}")
                    else:
                        nxt = controls[index + 1]
                        implied = (
                            (control[0] + nxt[0]) / 2,
                            (control[1] + nxt[1]) / 2,
                        )
                        out.append(f"Q{point(control)} {point(implied)}")
            elif kind == "close":
                out.append("Z")
    return "".join(out)


def flatten(contours: list[Contour]) -> list[list[tuple[float, float]]]:
    """Every contour as a closed polyline, for the scanline fill."""
    polygons: list[list[tuple[float, float]]] = []
    current: list[tuple[float, float]] = []
    here: tuple[float, float] = (0.0, 0.0)

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

    for contour in contours:
        for kind, points in contour.segments:
            if kind == "move":
                current = [points[0]]
                here = points[0]
            elif kind == "line":
                current.append(points[0])
                here = points[0]
            elif kind in ("cubic", "quad"):
                control_run = [here] + list(points)
                for step in range(1, FLATTEN_STEPS + 1):
                    current.append(bezier(control_run, step / FLATTEN_STEPS))
                here = points[-1]
            elif kind == "close":
                if len(current) > 2:
                    polygons.append(current)
                current = []
    return polygons


def rasterise(
    contours: list[Contour],
    upem: int,
    size: int,
    ascender: int,
    descender: int,
    advance: int,
    bbox_right: float,
) -> dict:
    """Fill the outline into a `size` pixel-per-em strike, nonzero winding.

    The grid is anchored the way a real strike is: one cell is `upem / size`
    font units, column 0 starts at the glyph origin and a row boundary sits on
    the baseline, so the page can lay the cells straight over the outline
    without fudging anything.
    """
    cell = upem / size
    rows_above = math.ceil(ascender / cell)
    rows_below = math.ceil(-descender / cell)
    columns = max(1, math.ceil(max(advance, bbox_right) / cell))

    polygons = flatten(contours)
    rows: list[str] = []
    for row in range(rows_above + rows_below):
        y = (rows_above - row - 0.5) * cell
        crossings: list[tuple[float, int]] = []
        for polygon in polygons:
            count = len(polygon)
            for index in range(count):
                x0, y0 = polygon[index]
                x1, y1 = polygon[(index + 1) % count]
                if y0 == y1:
                    continue
                if (y0 <= y < y1) or (y1 <= y < y0):
                    t = (y - y0) / (y1 - y0)
                    crossings.append((x0 + (x1 - x0) * t, 1 if y1 > y0 else -1))
        crossings.sort()

        cells = []
        for column in range(columns):
            x = (column + 0.5) * cell
            winding = sum(direction for cx, direction in crossings if cx <= x)
            cells.append("1" if winding != 0 else "0")
        rows.append("".join(cells))

    return {
        "ppem": size,
        "cell": round(cell, 3),
        "rowsAbove": rows_above,
        "columns": columns,
        "rows": rows,
    }


def specimen(path: pathlib.Path, character: str, key: str, font_number: int = -1) -> dict:
    font = TTFont(path, fontNumber=font_number) if font_number >= 0 else TTFont(path)
    upem = font["head"].unitsPerEm
    cmap = font.getBestCmap()
    code = ord(character)
    if code not in cmap:
        raise SystemExit(f"{path.name} has no glyph for U+{code:04X}")
    glyph_name = cmap[code]
    glyph_set = font.getGlyphSet()
    contours = read_contours(glyph_set, glyph_name)

    advance = font["hmtx"][glyph_name][0]
    os2 = font["OS/2"]
    metrics = {
        "ascender": int(getattr(os2, "sTypoAscender", font["hhea"].ascender)),
        "descender": int(getattr(os2, "sTypoDescender", font["hhea"].descender)),
        "capHeight": int(getattr(os2, "sCapHeight", 0) or round(upem * 0.7)),
        "xHeight": int(getattr(os2, "sxHeight", 0) or round(upem * 0.5)),
    }

    nodes = []
    on_curve = 0
    for contour_index, contour in enumerate(contours):
        for node in contour.nodes:
            nodes.append(
                {
                    "x": round(node.x, 1),
                    "y": round(node.y, 1),
                    "on": node.on,
                    "smooth": node.smooth,
                    "owner": node.owner,
                    "contour": contour_index,
                }
            )
            if node.on:
                on_curve += 1

    kinds = {kind for contour in contours for kind, _ in contour.segments}
    curve_type = "cubic" if "cubic" in kinds else "quadratic"

    # The bounding box is the drawn outline's, not the control points',
    # so the figure can state a number the editor would also state.
    flattened = [point for polygon in flatten(contours) for point in polygon]
    xs = [point[0] for point in flattened]
    ys = [point[1] for point in flattened]

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
        "bbox": [round(min(xs)), round(min(ys)), round(max(xs)), round(max(ys))],
        "metrics": metrics,
        "path": svg_path(contours),
        "nodes": nodes,
        "strikes": {
            str(size): rasterise(
                contours,
                upem,
                size,
                metrics["ascender"],
                metrics["descender"],
                advance,
                max(xs),
            )
            for size in (12, 16, 24)
        },
        "source": f"{family} {style}".strip(),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--latin", required=True, type=pathlib.Path)
    parser.add_argument("--cjk", required=True, type=pathlib.Path)
    parser.add_argument("--cjk-font-number", type=int, default=-1)
    parser.add_argument("--latin-char", default="g")
    parser.add_argument("--cjk-char", default="永")
    parser.add_argument(
        "--out",
        type=pathlib.Path,
        default=pathlib.Path("site/assets/js/specimens.js"),
    )
    args = parser.parse_args()

    data = {
        "latin": specimen(args.latin, args.latin_char, "latin"),
        "cjk": specimen(
            args.cjk, args.cjk_char, "cjk", font_number=args.cjk_font_number
        ),
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    body = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
    args.out.write_text(
        "/* Generated by site/tools/extract-outlines.py. Do not edit.\n"
        " * Outlines are read from OFL-licensed fonts; see THIRD_PARTY.md.\n"
        " */\n"
        f"window.FC_SPECIMENS = {body};\n",
        encoding="utf-8",
    )
    for key, value in data.items():
        print(
            f"{key}: {value['char']} {value['unicode']} from {value['source']} — "
            f"{value['points']} on-curve points, {value['curveType']}, "
            f"advance {value['advance']}/{value['upem']}"
        )
    print(f"wrote {args.out} ({args.out.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
