#!/usr/bin/env python3
"""Independent checks for FontCarver audit rows 82, 87, 93 and 97.

Everything the kernel is asked to produce is compared against a reference that
this script computes for itself:

* **Row 87, variable pens.** An independent Python stroker sweeps the nib along
  the source centreline and answers inside/outside for any point. Exported TTF
  and OTF outlines are compared against it with FontTools point-in-path and with
  FreeType's own rasteriser, over width, height, angle, both progressions, all
  three caps and all three joins.
* **Row 93, autotrace.** An analytic shape is rasterised into a PNG here, traced
  by the kernel with curve fitting, and the traced outline is compared back
  against the analytic shape — the clean-scan case the row asks for.
* **Row 82, smart components.** A CJK-style radical with two local axes is used
  by two component instances with different coordinates; the exported outlines
  must be the interpolations, not the default source.
* **Row 97, virtual masters.** A sparse source at a named virtual position must
  reach `gvar` at that location while `fvar` and `avar` stay consistent and no
  phantom master appears.

No fixture font or third-party source is copied into the repository.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import time
import urllib.error
import urllib.request
import zlib

import numpy
from pathlib import Path

import fontTools
import freetype
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.pointInsidePen import PointInsidePen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

UPEM = 1000
# Tolerance band around a reference boundary, in font units. Inside it the two
# descriptions are allowed to disagree: the kernel documents that variable-pen
# geometry is sampled and refitted, and a traced pixel ring is quantised.
STROKE_BAND = 1.0
TRACE_BAND = 3.0


class Api:
    """Minimal HTTP client for the development server."""

    def __init__(self, base: str) -> None:
        self.base = base.rstrip('/')
        self.timings: list[dict[str, float | str]] = []

    def __call__(self, method: str, **params: object) -> object:
        started = time.perf_counter()
        request = urllib.request.Request(
            f'{self.base}/api/{method}',
            data=json.dumps(params).encode(),
            headers={'Content-Type': 'application/json'},
        )
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                envelope = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(f'{method}: {error.read().decode()}') from error
        if 'error' in envelope:
            raise RuntimeError(f'{method}: {envelope["error"]}')
        self.timings.append(
            {'method': method, 'seconds': round(time.perf_counter() - started, 3)}
        )
        return envelope['result']


def glyph_gid(api: Api, doc: int, name: str) -> int:
    items = api(
        'glyphs.list', doc=doc, filter={}, offset=0, limit=400, include_paths=False
    )['items']
    return next(item['gid'] for item in items if item['name'] == name)


def layers(api: Api, doc: int, name: str) -> list[dict[str, object]]:
    gid = glyph_gid(api, doc, name)
    return api('glyphs.detail', doc=doc, gid=gid)['glyph']['layers']


def generation(api: Api, doc: int) -> int:
    return api('variation.state', doc=doc)['generation']


def point(x: float, y: float, kind: str = 'Line') -> dict[str, object]:
    return {'x': x, 'y': y, 'kind': kind}


def contour(points: list[dict[str, object]], closed: bool) -> dict[str, object]:
    return {'closed': closed, 'points': points, 'name': None, 'id': None}


# --------------------------------------------------------------------------
# Row 87: an independent stroker
# --------------------------------------------------------------------------


def interpolate(points: list[dict[str, float]], t: float, smooth: bool) -> tuple[float, float, float]:
    """Pen dimensions at normalised position `t`, independently of the kernel."""
    first = points[0]
    last = points[-1]
    if t <= first['position']:
        return first['width'], first['height'], first['angle']
    if t >= last['position']:
        return last['width'], last['height'], last['angle']
    for a, b in zip(points, points[1:]):
        if a['position'] <= t <= b['position']:
            span = b['position'] - a['position']
            local = 0.0 if span <= 0 else (t - a['position']) / span
            if smooth:
                local = local * local * (3 - 2 * local)
            delta = (b['angle'] - a['angle'] + 180) % 360 - 180
            return (
                a['width'] + (b['width'] - a['width']) * local,
                a['height'] + (b['height'] - a['height']) * local,
                a['angle'] + delta * local,
            )
    return last['width'], last['height'], last['angle']


def support(width: float, height: float, angle: float, normal: tuple[float, float], rectangle: bool) -> tuple[float, float]:
    """Nib support point in a direction: the documented pen geometry.

    This mirrors the definition the variable pen is specified by — the farthest
    point of the nib in one direction — not the kernel's code: an ellipse's
    support is solved from its own axes and a rectangle's from the sign of the
    rotated direction.
    """
    radians = math.radians(angle)
    cos = math.cos(radians)
    sin = math.sin(radians)
    local = (
        normal[0] * cos + normal[1] * sin,
        -normal[0] * sin + normal[1] * cos,
    )
    a = width / 2
    b = height / 2
    if rectangle:
        def sign(value: float) -> float:
            return 0.0 if abs(value) < 1e-12 else math.copysign(1.0, value)

        offset = (a * sign(local[0]), b * sign(local[1]))
    else:
        length = math.hypot(a * local[0], b * local[1])
        if length <= 0:
            offset = (0.0, 0.0)
        else:
            offset = (a * a * local[0] / length, b * b * local[1] / length)
    return (
        offset[0] * cos - offset[1] * sin,
        offset[0] * sin + offset[1] * cos,
    )


def nib_outline(
    width: float, height: float, angle: float, rectangle: bool, steps: int = 64
) -> list[tuple[float, float]]:
    """The nib boundary, used for a round cap and a round join."""
    if rectangle:
        radians = math.radians(angle)
        cos = math.cos(radians)
        sin = math.sin(radians)
        a = width / 2
        b = height / 2
        return [
            (x * cos - y * sin, x * sin + y * cos)
            for x, y in ((-a, -b), (a, -b), (a, b), (-a, b))
        ]
    radians = math.radians(angle)
    cos = math.cos(radians)
    sin = math.sin(radians)
    result = []
    for step in range(steps):
        theta = 2 * math.pi * step / steps
        x = width / 2 * math.cos(theta)
        y = height / 2 * math.sin(theta)
        result.append((x * cos - y * sin, x * sin + y * cos))
    return result


def arc_between(
    nib: list[tuple[float, float]],
    start: tuple[float, float],
    end: tuple[float, float],
) -> list[tuple[float, float]]:
    """Nib boundary points between two support offsets, counter-clockwise."""
    def angle_of(v: tuple[float, float]) -> float:
        return math.atan2(v[1], v[0]) % (2 * math.pi)

    low = angle_of(start)
    high = angle_of(end)
    sweep = (high - low) % (2 * math.pi)
    if sweep < 1e-9:
        sweep = 2 * math.pi
    chosen = [start]
    for offset in nib:
        local = (angle_of(offset) - low) % (2 * math.pi)
        if 1e-9 < local < sweep - 1e-9:
            chosen.append((local, offset))
    ordered = [point for point in chosen[1:]]
    ordered.sort(key=lambda entry: entry[0])
    return [start] + [offset for _, offset in ordered] + [end]


def patch_inside(points, polygon: list[tuple[float, float]]):
    """Whether each point is in one convex patch, by half-plane tests."""
    if len(polygon) < 3:
        return numpy.zeros(len(points), dtype=bool)
    ring = numpy.asarray(polygon, dtype=float)
    area = float(
        numpy.sum(
            ring[:, 0] * numpy.roll(ring[:, 1], -1)
            - numpy.roll(ring[:, 0], -1) * ring[:, 1]
        )
    )
    if area < 0:
        ring = ring[::-1]
    edges = numpy.roll(ring, -1, axis=0) - ring
    lengths = numpy.hypot(edges[:, 0], edges[:, 1])
    keep = lengths > 1e-12
    if not keep.any():
        return numpy.zeros(len(points), dtype=bool)
    ring = ring[keep]
    edges = edges[keep]
    offsets = points[:, None, :] - ring[None, :, :]
    # Counter-clockwise ring: an interior point is left of every edge.
    signed = (
        offsets[:, :, 0] * edges[None, :, 1] - offsets[:, :, 1] * edges[None, :, 0]
    )
    return numpy.all(signed <= 0, axis=1)


def union_inside(points, patches: list[list[tuple[float, float]]]):
    """Whether each point is in the union of the patches."""
    inside = numpy.zeros(len(points), dtype=bool)
    for patch in patches:
        inside |= patch_inside(points, patch)
    return inside


def classify(points, patches: list[list[tuple[float, float]]], band: float):
    """Points the reference places clearly inside, and clearly outside.

    A patch union has internal edges, so the distance to one patch says nothing
    about the distance to the union's own boundary. Testing a ring of probes
    around each point answers the only question the comparison needs: is this
    point inside, or outside, with `band` units to spare either way.
    """
    probes = [numpy.zeros(2)]
    for step in range(8):
        angle = 2 * math.pi * step / 8
        probes.append(numpy.array([band * math.cos(angle), band * math.sin(angle)]))
    votes = [union_inside(points + probe, patches) for probe in probes]
    stacked = numpy.vstack(votes)
    return stacked.all(axis=0), ~stacked.any(axis=0)


def stroke_patches(
    centreline: list[tuple[float, float]],
    profile: list[dict[str, float]],
    nib: str,
    cap: str,
    join: str,
    smooth: bool,
    miter_limit: float,
    per_segment: int = 48,
) -> list[list[tuple[float, float]]]:
    """Independent patch model of a variable-pen stroke.

    The pen sweeps along the centreline: each sub-segment contributes the
    quadrilateral between the two support offsets at its ends, each interior
    corner contributes its join and each open end its cap. That is the published
    definition of the effect; the union is taken here by point containment rather
    than by any polygon library.
    """
    rectangle = nib == 'rectangle'
    lengths = [math.dist(a, b) for a, b in zip(centreline, centreline[1:])]
    total = sum(lengths)
    patches: list[list[tuple[float, float]]] = []
    travelled = 0.0
    for (a, b), length in zip(zip(centreline, centreline[1:]), lengths):
        if length <= 0:
            continue
        direction = ((b[0] - a[0]) / length, (b[1] - a[1]) / length)
        normal = (-direction[1], direction[0])
        for step in range(per_segment):
            t0 = (travelled + length * step / per_segment) / total
            t1 = (travelled + length * (step + 1) / per_segment) / total
            p0 = (
                a[0] + direction[0] * length * step / per_segment,
                a[1] + direction[1] * length * step / per_segment,
            )
            p1 = (
                a[0] + direction[0] * length * (step + 1) / per_segment,
                a[1] + direction[1] * length * (step + 1) / per_segment,
            )
            w0, h0, angle0 = interpolate(profile, t0, smooth)
            w1, h1, angle1 = interpolate(profile, t1, smooth)
            offset0 = support(w0, h0, angle0, normal, rectangle)
            offset1 = support(w1, h1, angle1, normal, rectangle)
            patches.append(
                [
                    (p0[0] + offset0[0], p0[1] + offset0[1]),
                    (p1[0] + offset1[0], p1[1] + offset1[1]),
                    (p1[0] - offset1[0], p1[1] - offset1[1]),
                    (p0[0] - offset0[0], p0[1] - offset0[1]),
                ]
            )
        travelled += length
    # Joins at every interior corner of the polyline.
    travelled = 0.0
    for index in range(1, len(centreline) - 1):
        travelled += lengths[index - 1]
        position = travelled / total
        width, height, angle = interpolate(profile, position, smooth)
        here = centreline[index]
        incoming = (
            (here[0] - centreline[index - 1][0]) / lengths[index - 1],
            (here[1] - centreline[index - 1][1]) / lengths[index - 1],
        )
        outgoing = (
            (centreline[index + 1][0] - here[0]) / lengths[index],
            (centreline[index + 1][1] - here[1]) / lengths[index],
        )
        cross = incoming[0] * outgoing[1] - incoming[1] * outgoing[0]
        nib_points = nib_outline(width, height, angle, rectangle)
        for side in (-1.0, 1.0):
            na = (-incoming[1] * side, incoming[0] * side)
            nb = (-outgoing[1] * side, outgoing[0] * side)
            offset_a = support(width, height, angle, na, rectangle)
            offset_b = support(width, height, angle, nb, rectangle)
            outer = cross * side < 0
            patch = [here, (here[0] + offset_a[0], here[1] + offset_a[1])]
            if outer and join == 'round':
                arc = (
                    arc_between(nib_points, offset_a, offset_b)
                    if cross > 0
                    else list(reversed(arc_between(nib_points, offset_b, offset_a)))
                )
                patch.extend((here[0] + x, here[1] + y) for x, y in arc)
            elif outer and join == 'miter' and abs(cross) > 1e-10:
                delta = (offset_b[0] - offset_a[0], offset_b[1] - offset_a[1])
                scale = (delta[0] * outgoing[1] - delta[1] * outgoing[0]) / cross
                tip = (
                    offset_a[0] + incoming[0] * scale,
                    offset_a[1] + incoming[1] * scale,
                )
                if math.hypot(*tip) <= miter_limit * max(width, height) * 0.5:
                    patch.append((here[0] + tip[0], here[1] + tip[1]))
            patch.append((here[0] + offset_b[0], here[1] + offset_b[1]))
            patches.append(patch)
    # Caps at the two open ends.
    for index in (0, len(centreline) - 1):
        neighbour = 1 if index == 0 else len(centreline) - 2
        segment = lengths[0] if index == 0 else lengths[-1]
        here = centreline[index]
        other = centreline[neighbour]
        # Outward direction of this end.
        direction = (
            (here[0] - other[0]) / segment,
            (here[1] - other[1]) / segment,
        )
        position = 0.0 if index == 0 else 1.0
        width, height, angle = interpolate(profile, position, smooth)
        normal = (-direction[1], direction[0])
        left = support(width, height, angle, normal, rectangle)
        right = (-left[0], -left[1])
        if cap == 'butt':
            continue
        if cap == 'square':
            extension = support(width, height, angle, direction, rectangle)
            if rectangle:
                dot = extension[0] * direction[0] + extension[1] * direction[1]
                extension = (direction[0] * dot, direction[1] * dot)
            patches.append(
                [
                    (here[0] + left[0], here[1] + left[1]),
                    (here[0] + left[0] + extension[0], here[1] + left[1] + extension[1]),
                    (here[0] + right[0] + extension[0], here[1] + right[1] + extension[1]),
                    (here[0] + right[0], here[1] + right[1]),
                ]
            )
        else:
            nib_points = nib_outline(width, height, angle, rectangle)
            arc = arc_between(nib_points, right, left)
            patches.append([(here[0] + x, here[1] + y) for x, y in arc])
    return patches


def patch_distances(
    points, patches: list[list[tuple[float, float]]]
):
    """Signed distance of many points to the union, negative inside.

    The same convex half-plane test as [`convex_distance`], evaluated for every
    point at once so one case can sample thousands of positions.
    """
    best = numpy.full(len(points), numpy.inf)
    for patch in patches:
        if len(patch) < 3:
            continue
        ring = numpy.asarray(patch, dtype=float)
        area = float(
            numpy.sum(
                ring[:, 0] * numpy.roll(ring[:, 1], -1)
                - numpy.roll(ring[:, 0], -1) * ring[:, 1]
            )
        )
        if area < 0:
            ring = ring[::-1]
        edges = numpy.roll(ring, -1, axis=0) - ring
        lengths = numpy.hypot(edges[:, 0], edges[:, 1])
        keep = lengths > 1e-12
        if not keep.any():
            continue
        ring = ring[keep]
        edges = edges[keep]
        lengths = lengths[keep]
        offsets = points[:, None, :] - ring[None, :, :]
        signed = (
            offsets[:, :, 0] * edges[None, :, 1]
            - offsets[:, :, 1] * edges[None, :, 0]
        ) / lengths[None, :]
        best = numpy.minimum(best, signed.max(axis=1))
    return best


def inside_glyph(font: TTFont, name: str, at: tuple[float, float]) -> bool:
    glyphs = font.getGlyphSet()
    pen = PointInsidePen(glyphs, at, evenOdd=False)
    glyphs[name].draw(pen)
    return pen.getResult()


class Raster:
    """One glyph rendered by FreeType at one pixel per font unit.

    `freetype.Bitmap.buffer` rebuilds a Python list on every access, so the
    coverage is copied once here and then read as bytes.
    """

    def __init__(self, path: Path, name: str) -> None:
        face = freetype.Face(str(path))
        face.set_char_size(UPEM * 64, 0, 72, 72)
        index = face.get_name_index(name.encode())
        assert index != 0, (path.name, name, 'glyph has no name in the export')
        face.load_glyph(index, freetype.FT_LOAD_RENDER)
        bitmap = face.glyph.bitmap
        self.left = face.glyph.bitmap_left
        self.top = face.glyph.bitmap_top
        self.width = bitmap.width
        self.rows = bitmap.rows
        self.pitch = bitmap.pitch
        self.buffer = bytes(bitmap.buffer)

    def covered(self, at: tuple[float, float]) -> bool:
        """Whether FreeType painted the pixel at a font-unit point."""
        column = int(math.floor(at[0])) - self.left
        row = self.top - 1 - int(math.floor(at[1]))
        if not (0 <= column < self.width and 0 <= row < self.rows):
            return False
        return self.buffer[row * self.pitch + column] >= 128


# --------------------------------------------------------------------------
# Row 93: an analytic shape, a PNG of it, and the comparison back
# --------------------------------------------------------------------------


def grayscale_png(width: int, height: int, pixels: bytes) -> bytes:
    """Write an 8-bit grayscale PNG without an image library."""
    raw = b''.join(
        b'\x00' + pixels[row * width : (row + 1) * width] for row in range(height)
    )

    def chunk(kind: bytes, data: bytes) -> bytes:
        body = kind + data
        return struct.pack('>I', len(data)) + body + struct.pack('>I', zlib.crc32(body))

    header = struct.pack('>IIBBBBB', width, height, 8, 0, 0, 0, 0)
    return (
        b'\x89PNG\r\n\x1a\n'
        + chunk(b'IHDR', header)
        + chunk(b'IDAT', zlib.compress(raw, 6))
        + chunk(b'IEND', b'')
    )


# The traced shape: an elliptical annulus whose left side is cut square, so the
# trace has to find two corners and two straight runs as well as four arcs.
STEM_X = 220.0


def trace_reference(x: float, y: float) -> float:
    """Signed distance to the analytic shape, in image pixels, negative inside.

    The shape is the intersection of the annulus with the half plane to the
    right of `STEM_X`, so the signed distance is the largest of the three
    constraints — exact for the half plane and for the circle, and a close
    under-estimate for the squashed ellipse, which is all the comparison band
    needs.
    """
    distance = math.hypot(x - 500.0, (y - 500.0) / 1.25)
    return max(250.0 - distance, distance - 400.0, STEM_X - x)


def ring_ink(x: float, y: float) -> bool:
    """Whether one image pixel is ink."""
    return trace_reference(x, y) <= 0.0


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', default='http://127.0.0.1:8810')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--seed', type=int, default=20261001)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    api = Api(args.api)
    report: dict[str, object] = {
        'fonttools': fontTools.__version__,
        'freetype': freetype.version(),
        'complete': False,
        'rows': {},
    }
    try:
        report['rows']['87'] = check_variable_pens(api, output, args.seed)
        report['rows']['93'] = check_autotrace(api, output, args.seed)
        report['rows']['82'] = check_smart_components(api, output)
        report['rows']['97'] = check_virtual_masters(api, output)
        report['complete'] = True
    finally:
        report['timings'] = api.timings[-40:]
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['rows'], indent=2))


# --------------------------------------------------------------------------


STROKE_CASES = [
    (
        'taper-ellipse',
        'ellipse',
        'butt',
        'bevel',
        'linear',
        [
            {'position': 0.0, 'width': 100.0, 'height': 100.0, 'angle': 0.0},
            {'position': 1.0, 'width': 220.0, 'height': 220.0, 'angle': 0.0},
        ],
        [(150.0, 350.0), (550.0, 350.0)],
    ),
    (
        'height-only',
        'ellipse',
        'butt',
        'bevel',
        'linear',
        [
            {'position': 0.0, 'width': 160.0, 'height': 60.0, 'angle': 0.0},
            {'position': 1.0, 'width': 160.0, 'height': 240.0, 'angle': 0.0},
        ],
        [(150.0, 350.0), (550.0, 350.0)],
    ),
    (
        'angle-rectangle',
        'rectangle',
        'butt',
        'bevel',
        'linear',
        [
            {'position': 0.0, 'width': 200.0, 'height': 80.0, 'angle': 30.0},
            {'position': 1.0, 'width': 200.0, 'height': 80.0, 'angle': 30.0},
        ],
        [(150.0, 350.0), (550.0, 350.0)],
    ),
    (
        'smooth-progression',
        'ellipse',
        'butt',
        'bevel',
        'smooth',
        [
            {'position': 0.0, 'width': 80.0, 'height': 80.0, 'angle': 0.0},
            {'position': 1.0, 'width': 240.0, 'height': 240.0, 'angle': 0.0},
        ],
        [(150.0, 350.0), (550.0, 350.0)],
    ),
    (
        'cap-round',
        'ellipse',
        'round',
        'bevel',
        'linear',
        [
            {'position': 0.0, 'width': 140.0, 'height': 140.0, 'angle': 0.0},
            {'position': 1.0, 'width': 200.0, 'height': 200.0, 'angle': 0.0},
        ],
        [(200.0, 350.0), (500.0, 350.0)],
    ),
    (
        'cap-square',
        'rectangle',
        'square',
        'bevel',
        'linear',
        [
            {'position': 0.0, 'width': 140.0, 'height': 140.0, 'angle': 0.0},
            {'position': 1.0, 'width': 140.0, 'height': 140.0, 'angle': 0.0},
        ],
        [(200.0, 350.0), (500.0, 350.0)],
    ),
    (
        'join-round',
        'ellipse',
        'butt',
        'round',
        'linear',
        [
            {'position': 0.0, 'width': 120.0, 'height': 120.0, 'angle': 0.0},
            {'position': 1.0, 'width': 200.0, 'height': 200.0, 'angle': 0.0},
        ],
        [(150.0, 150.0), (450.0, 150.0), (450.0, 600.0)],
    ),
    (
        'join-miter',
        'rectangle',
        'butt',
        'miter',
        'linear',
        [
            {'position': 0.0, 'width': 140.0, 'height': 140.0, 'angle': 0.0},
            {'position': 1.0, 'width': 140.0, 'height': 140.0, 'angle': 0.0},
        ],
        [(150.0, 150.0), (450.0, 150.0), (450.0, 600.0)],
    ),
    (
        'join-bevel',
        'rectangle',
        'butt',
        'bevel',
        'linear',
        [
            {'position': 0.0, 'width': 140.0, 'height': 140.0, 'angle': 0.0},
            {'position': 1.0, 'width': 140.0, 'height': 140.0, 'angle': 0.0},
        ],
        [(150.0, 150.0), (450.0, 150.0), (450.0, 600.0)],
    ),
]


def check_variable_pens(api: Api, output: Path, seed: int) -> dict[str, object]:
    """Export every pen case and compare it against the independent stroker."""
    results = []
    for case, nib, cap, join, progression, profile, centreline in STROKE_CASES:
        doc = api('session.new', family=f'Pen {case}', units_per_em=UPEM)['doc']
        try:
            api('glyphs.add', doc=doc, recipe='A', master=None, dry_run=False)
            layer = layers(api, doc, 'A')[0]['id']
            points = [
                point(x, y, 'Move' if index == 0 else 'Line')
                for index, (x, y) in enumerate(centreline)
            ]
            api(
                'doc.apply',
                doc=doc,
                command={
                    'Batch': [
                        {
                            'ReplaceLayerOutline': {
                                'glyph': 'A',
                                'layer': layer,
                                'contours': [contour(points, False)],
                                'components': [],
                                'anchors': [],
                            }
                        },
                        {
                            'SetMetrics': {
                                'glyph': 'A',
                                'layer': layer,
                                'width': 700.0,
                                'lsb': None,
                                'rsb': None,
                            }
                        },
                    ]
                },
                label=f'centreline {case}',
            )
            effects = [
                {
                    'enabled': True,
                    'effect': {
                        'kind': 'variable_stroke',
                        'nib': nib,
                        'cap': cap,
                        'join': join,
                        'miter_limit': 4.0,
                        'progression': progression,
                        'points': profile,
                    },
                }
            ]
            preview = api(
                'effects.apply',
                doc=doc,
                glyph='A',
                layer=layer,
                effects=effects,
                dry_run=True,
                bake=False,
                expected_generation=None,
            )
            api(
                'effects.apply',
                doc=doc,
                glyph='A',
                layer=layer,
                effects=effects,
                dry_run=False,
                bake=False,
                expected_generation=preview['generation'],
            )
            smooth = progression == 'smooth'
            patches = stroke_patches(
                centreline, profile, nib, cap, join, smooth, 4.0
            )
            # Sample a box around the whole stroke, keeping only the points the
            # reference places clearly inside or clearly outside.
            generator = numpy.random.default_rng(seed + len(case))
            candidates = generator.uniform(
                [-50.0, -50.0], [700.0, 800.0], size=(40000, 2)
            )
            interior_mask, exterior_mask = classify(candidates, patches, STROKE_BAND)
            # Balance the sample: the stroke covers a small part of the box, so
            # taking the first clear candidates would test almost no ink.
            interior = candidates[interior_mask][:700]
            exterior = candidates[exterior_mask][:700]
            tests: list[tuple[tuple[float, float], bool]] = [
                ((float(x), float(y)), True) for x, y in interior
            ] + [((float(x), float(y)), False) for x, y in exterior]
            assert len(interior) > 300 and len(exterior) > 300, (
                case,
                len(interior),
                len(exterior),
            )
            case_result: dict[str, object] = {
                'nib': nib,
                'cap': cap,
                'join': join,
                'progression': progression,
                'sampled': len(tests),
                'exports': {},
            }
            for container in ('Ttf', 'Otf'):
                path = output / f'pen-{case}.{container.lower()}'
                api(
                    'session.export',
                    doc=doc,
                    path=str(path),
                    options={'format': container},
                )
                with TTFont(path, checkChecksums=2) as font:
                    wrong = [
                        (at, expected)
                        for at, expected in tests
                        if inside_glyph(font, 'A', at) != expected
                    ]
                raster = Raster(path, 'A')
                raster_wrong = [
                    (at, expected)
                    for at, expected in tests
                    if raster.covered(at) != expected
                ]
                assert not wrong, (case, container, 'fontTools', wrong[:4])
                assert not raster_wrong, (case, container, 'freetype', raster_wrong[:4])
                case_result['exports'][container] = {
                    'fonttools_mismatches': len(wrong),
                    'freetype_mismatches': len(raster_wrong),
                }
            results.append({case: case_result})
            print(
                f'row 87 {case}: {len(tests)} sampled points agree in TTF and OTF '
                f'with FontTools and FreeType',
                flush=True,
            )
        finally:
            api('session.close', doc=doc)
    return {'band': STROKE_BAND, 'cases': results}


def check_autotrace(api: Api, output: Path, seed: int) -> dict[str, object]:
    """Trace a clean raster of an analytic ring and compare the result back."""
    size = 1000
    pixels = bytearray(size * size)
    for row in range(size):
        for column in range(size):
            pixels[row * size + column] = (
                0 if ring_ink(column + 0.5, row + 0.5) else 255
            )
    png = grayscale_png(size, size, bytes(pixels))
    (output / 'trace-source.png').write_bytes(png)
    doc = api('session.new', family='Autotrace', units_per_em=UPEM)['doc']
    try:
        api('glyphs.add', doc=doc, recipe='O', master=None, dry_run=False)
        layer = layers(api, doc, 'O')[0]['id']
        state = api('image.state', doc=doc, glyph='O', layer=layer)
        api(
            'image.import',
            doc=doc,
            glyph='O',
            layer=layer,
            expected_generation=state['generation'],
            path=None,
            data=list(png),
            file_name='ring.png',
            embed=True,
        )
        state = api('image.state', doc=doc, glyph='O', layer=layer)
        image = dict(state['image'])
        # One font unit per image pixel keeps the comparison in image space.
        image['transform'] = [1, 0, 0, 1, 0, 0]
        api(
            'image.set',
            doc=doc,
            glyph='O',
            layer=layer,
            expected_generation=state['generation'],
            image=image,
        )
        shared = dict(
            doc=doc,
            glyph='O',
            layer=layer,
            threshold=128,
            append=False,
            expected_source_hash=None,
        )
        polygonal = api(
            'image.trace',
            **shared,
            tolerance=1.0,
            curves=False,
            corner_angle=35.0,
            smoothing=0.0,
            min_area=0.0,
            dry_run=True,
            expected_generation=None,
        )
        curved = api(
            'image.trace',
            **shared,
            tolerance=1.0,
            curves=True,
            corner_angle=35.0,
            smoothing=0.0,
            min_area=0.0,
            dry_run=True,
            expected_generation=None,
        )
        api(
            'image.trace',
            **{**shared, 'expected_source_hash': curved['source_hash']},
            tolerance=1.0,
            curves=True,
            corner_angle=35.0,
            smoothing=0.0,
            min_area=0.0,
            dry_run=False,
            expected_generation=curved['generation'],
        )
        assert len(curved['contours']) == 2, curved['contours']
        # The flat left side has to come back as straight segments, not curves.
        straight = sum(
            1
            for contour in curved['contours']
            for point in contour['points']
            if point['kind'] == 'Line'
        )
        assert straight >= 2, curved['contours']
        assert curved['curve_segments'] > 0
        assert curved['nodes'] * 8 < polygonal['nodes'], (
            curved['nodes'],
            polygonal['nodes'],
        )
        path = output / 'trace.ttf'
        api('session.export', doc=doc, path=str(path), options={'format': 'Ttf'})
        generator = numpy.random.default_rng(seed)
        tests = []
        while len(tests) < 4000:
            at = (
                float(generator.uniform(0, size)),
                float(generator.uniform(0, size)),
            )
            distance = trace_reference(*at)
            if abs(distance) <= TRACE_BAND:
                continue
            # The traced outline is placed in y-up font units: the image's row
            # zero is the top, so the reference y flips.
            tests.append(((at[0], size - at[1]), distance < 0))
        with TTFont(path, checkChecksums=2) as font:
            wrong = [
                (at, expected)
                for at, expected in tests
                if inside_glyph(font, 'O', at) != expected
            ]
        raster = Raster(path, 'O')
        raster_wrong = [
            (at, expected) for at, expected in tests if raster.covered(at) != expected
        ]
        assert not wrong, ('autotrace fontTools', wrong[:4])
        assert not raster_wrong, ('autotrace freetype', raster_wrong[:4])
        print(
            f'row 93: {curved["nodes"]} curve-fitted nodes against '
            f'{polygonal["nodes"]} pixel-edge nodes, deviation '
            f'{curved["max_error"]:.2f} px, {len(tests)} sampled points agree',
            flush=True,
        )
        return {
            'band': TRACE_BAND,
            'straight_segments': straight,
            'curve_nodes': curved['nodes'],
            'curve_segments': curved['curve_segments'],
            'polygon_nodes': polygonal['nodes'],
            'max_error': curved['max_error'],
            'fallback': curved['fallback'],
            'sampled': len(tests),
            'mismatches': 0,
        }
    finally:
        api('session.close', doc=doc)


def check_smart_components(api: Api, output: Path) -> dict[str, object]:
    """Two instances of one two-axis radical must export their own shapes."""
    doc = api('session.new', family='Smart radical', units_per_em=UPEM)['doc']
    try:
        api('glyphs.add', doc=doc, recipe='A B C', master=None, dry_run=False)
        layer = layers(api, doc, 'A')[0]['id']
        master = api('variation.state', doc=doc)['masters'][0]['id']
        # The radical's default source is a 100-wide, 100-tall bar.
        def bar(width: float, height: float) -> dict[str, object]:
            return contour(
                [
                    point(0, 0),
                    point(width, 0),
                    point(width, height),
                    point(0, height),
                ],
                True,
            )

        api(
            'doc.apply',
            doc=doc,
            command={
                'ReplaceLayerOutline': {
                    'glyph': 'A',
                    'layer': layer,
                    'contours': [bar(100, 100)],
                    'components': [],
                    'anchors': [],
                }
            },
            label='radical default',
        )
        state = api('variation.state', doc=doc)
        api(
            'variation.apply',
            doc=doc,
            expected_generation=state['generation'],
            label='smart axes',
            command={
                'SmartAxes': {
                    'glyph': 'A',
                    'axes': [
                        {'name': 'wide', 'min': 100.0, 'max': 500.0, 'default': 100.0},
                        {'name': 'tall', 'min': 100.0, 'max': 700.0, 'default': 100.0},
                    ],
                }
            },
        )
        # A smart glyph needs a source at every corner of its local axis grid.
        for axis in ('wide', 'tall', 'both'):
            state = api('variation.state', doc=doc)
            source = layers(api, doc, 'A')[0]['id']
            api(
                'variation.apply',
                doc=doc,
                expected_generation=state['generation'],
                label=f'{axis} source',
                command={
                    'Layer': {
                        'glyph': 'A',
                        'source': source,
                        'duplicate': True,
                        'name': axis,
                        'kind': {
                            'Smart': {
                                'master': master,
                                'location': {
                                    'wide': 500.0 if axis in ('wide', 'both') else 100.0,
                                    'tall': 700.0 if axis in ('tall', 'both') else 100.0,
                                },
                            }
                        },
                    }
                },
            )
            smart = [
                entry
                for entry in layers(api, doc, 'A')
                if 'Smart' in entry['kind'] and entry['name'] == axis
            ][0]
            api(
                'doc.apply',
                doc=doc,
                command={
                    'ReplaceLayerOutline': {
                        'glyph': 'A',
                        'layer': smart['id'],
                        'contours': [
                            bar(
                                500 if axis in ('wide', 'both') else 100,
                                700 if axis in ('tall', 'both') else 100,
                            )
                        ],
                        'components': [],
                        'anchors': [],
                    }
                },
                label=f'{axis} geometry',
            )
        # B uses the mid coordinate on one axis, C the maximum on the other.
        instances = {'B': {'wide': 300.0}, 'C': {'tall': 700.0}}
        for name, values in instances.items():
            target = layers(api, doc, name)[0]['id']
            api(
                'doc.apply',
                doc=doc,
                command={
                    'Batch': [
                        {
                            'ReplaceLayerOutline': {
                                'glyph': name,
                                'layer': target,
                                'contours': [],
                                'components': [
                                    {
                                        'base': 'A',
                                        'transform': [1, 0, 0, 1, 0, 0],
                                        'anchor': None,
                                        'auto_align': False,
                                        'locked': False,
                                        'use_my_metrics': False,
                                        'round_to_grid': False,
                                        'smart': values,
                                        'point_match': None,
                                    }
                                ],
                                'anchors': [],
                            }
                        },
                        {
                            'SetMetrics': {
                                'glyph': name,
                                'layer': target,
                                'width': 700.0,
                                'lsb': None,
                                'rsb': None,
                            }
                        },
                    ]
                },
                label=f'{name} instance',
            )
        path = output / 'smart-components.ttf'
        api('session.export', doc=doc, path=str(path), options={'format': 'Ttf'})
        measured = {}
        with TTFont(path, checkChecksums=2) as font:
            glyphs = font.getGlyphSet()
            for name in ('A', 'B', 'C'):
                pen = BoundsPen(glyphs)
                glyphs[name].draw(pen)
                measured[name] = pen.bounds
        assert measured['A'] == (0, 0, 100, 100), measured
        # 300 on a 100..500 axis is the midpoint of the two sources.
        assert abs(measured['B'][2] - 300) <= 1, measured
        assert abs(measured['B'][3] - 100) <= 1, measured
        assert abs(measured['C'][2] - 100) <= 1, measured
        assert abs(measured['C'][3] - 700) <= 1, measured
        raster = Raster(path, 'B')
        assert raster.covered((290, 50))
        assert not raster.covered((310, 50))
        print(f'row 82: exported instance bounds {measured}', flush=True)
        return {'bounds': measured}
    finally:
        api('session.close', doc=doc)


def check_virtual_masters(api: Api, output: Path) -> dict[str, object]:
    """A sparse source at a virtual position must reach gvar at that location."""
    doc = api('session.new', family='Virtual', units_per_em=UPEM)['doc']
    try:
        api('glyphs.add', doc=doc, recipe='A', master=None, dry_run=False)
        state = api('variation.state', doc=doc)
        regular = state['masters'][0]
        bold = {
            **regular,
            'id': 'boldmaster',
            'name': 'Bold',
            'location': {'wght': 100.0},
        }
        axes = [
            {
                'tag': [119, 103, 104, 116],
                'name': 'Weight',
                'min': 100.0,
                'default': 100.0,
                'max': 900.0,
                'hidden': False,
                'map': [[100.0, 0.0], [900.0, 100.0]],
            }
        ]
        api(
            'variation.apply',
            doc=doc,
            expected_generation=state['generation'],
            label='design space',
            command={
                'Set': {
                    'axis_renames': {},
                    'axes': axes,
                    'masters': [{**regular, 'location': {'wght': 0.0}}, bold],
                    'instances': [],
                    'virtual_masters': [{'name': 'Middle', 'location': {'wght': 50.0}}],
                    'copy_from': regular['id'],
                }
            },
        )
        # Two physical masters with different widths, so a delta must exist.
        for master_index, width in ((0, 100.0), (1, 400.0)):
            state = api('variation.state', doc=doc)
            master = state['masters'][master_index]['id']
            layer = [
                entry
                for entry in layers(api, doc, 'A')
                if entry['kind'].get('Master') == master
            ][0]['id']
            api(
                'doc.apply',
                doc=doc,
                command={
                    'Batch': [
                        {
                            'ReplaceLayerOutline': {
                                'glyph': 'A',
                                'layer': layer,
                                'contours': [
                                    contour(
                                        [
                                            point(0, 0),
                                            point(width, 0),
                                            point(width, 700),
                                            point(0, 700),
                                        ],
                                        True,
                                    )
                                ],
                                'components': [],
                                'anchors': [],
                            }
                        },
                        {
                            'SetMetrics': {
                                'glyph': 'A',
                                'layer': layer,
                                'width': 700.0,
                                'lsb': None,
                                'rsb': None,
                            }
                        },
                    ]
                },
                label='master geometry',
            )
        # A sparse source at the virtual position, deliberately off the line.
        state = api('variation.state', doc=doc)
        default_layer = [
            entry
            for entry in layers(api, doc, 'A')
            if entry['kind'].get('Master') == state['masters'][0]['id']
        ][0]['id']
        api(
            'variation.apply',
            doc=doc,
            expected_generation=state['generation'],
            label='virtual source',
            command={
                'VirtualSource': {'glyph': 'A', 'source': default_layer, 'index': 0}
            },
        )
        brace = [
            entry for entry in layers(api, doc, 'A') if 'Brace' in entry['kind']
        ][0]
        api(
            'doc.apply',
            doc=doc,
            command={
                'ReplaceLayerOutline': {
                    'glyph': 'A',
                    'layer': brace['id'],
                    'contours': [
                        contour(
                            [
                                point(0, 0),
                                point(400, 0),
                                point(400, 700),
                                point(0, 700),
                            ],
                            True,
                        )
                    ],
                    'components': [],
                    'anchors': [],
                }
            },
            label='virtual geometry',
        )
        path = output / 'virtual-masters.ttf'
        api(
            'session.export',
            doc=doc,
            path=str(path),
            options={'format': 'Ttf', 'variable': True},
        )
        with TTFont(path, checkChecksums=2) as font:
            fvar = font['fvar']
            assert [axis.axisTag for axis in fvar.axes] == ['wght'], fvar.axes
            axis = fvar.axes[0]
            assert (axis.minValue, axis.defaultValue, axis.maxValue) == (
                100.0,
                100.0,
                900.0,
            ), axis
            # A virtual position is not a named instance and must not become one.
            assert fvar.instances == [], fvar.instances
            segments = font['avar'].segments['wght'] if 'avar' in font else {}
            tuples = font['gvar'].variations['A']
            peaks = sorted(
                round(entry.axes['wght'][1], 4)
                for entry in tuples
                if 'wght' in entry.axes
            )
            # The intermediate source becomes its own region at the virtual
            # position, alongside the full-weight region.
            assert 1.0 in peaks, peaks
            assert any(0.0 < peak < 1.0 for peak in peaks), peaks
        # FontTools instantiates the variable font itself, so the intermediate
        # source is read back through a second implementation of gvar.
        glyphs = {}
        for user, expected in ((100.0, 100.0), (500.0, 400.0), (900.0, 400.0)):
            with TTFont(path, checkChecksums=2) as font:
                instantiated = instantiateVariableFont(
                    font, {'wght': user}, inplace=True, updateFontNames=False
                )
                pen = BoundsPen(instantiated.getGlyphSet())
                instantiated.getGlyphSet()['A'].draw(pen)
                glyphs[user] = pen.bounds
                assert abs(pen.bounds[2] - expected) <= 2, (user, pen.bounds)
                assert 'gvar' not in instantiated, 'an instance keeps no gvar'
        print(
            f'row 97: gvar peaks {peaks}, avar {segments}, instance widths '
            f'{ {k: v[2] for k, v in glyphs.items()} }',
            flush=True,
        )
        return {
            'gvar_peaks': peaks,
            'avar': {str(k): v for k, v in segments.items()},
            'instances': {str(k): v for k, v in glyphs.items()},
        }
    finally:
        api('session.close', doc=doc)


if __name__ == '__main__':
    main()
