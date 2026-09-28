#!/usr/bin/env python3
"""Independent analytic smart-corner checks through the API and FontTools.

Creates original synthetic outlines; checks live commands, native round trips,
undo, baking, translated components and four compiled containers. No fixtures
or third-party source code are copied into the repository.
"""
from __future__ import annotations
import argparse
import json
import math
import time
import urllib.error
import urllib.request
from pathlib import Path

import fontTools
from fontTools.ttLib import TTFont
from p5_style_acceptance import synthetic, inside, native_semantics
from p5_layer_effects_acceptance import point, mask


def polygon_contains(vertices, x, y):
    """Even/odd crossing for one analytically constructed simple polygon."""
    result = False
    for a, b in zip(vertices, vertices[1:] + vertices[:1]):
        if (a[1] > y) != (b[1] > y):
            if x < a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
                result = not result
    return result


def trap_polygon(vertices, indices, depth, ink_toward_center=True):
    center = tuple(sum(v[k] for v in vertices) / len(vertices) for k in (0, 1))
    result = []
    for i, vertex in enumerate(vertices):
        if i not in indices:
            result.append(vertex)
            continue
        neighbors = (vertices[i - 1], vertices[(i + 1) % len(vertices)])
        edges = [tuple((n[k] - vertex[k]) / math.dist(n, vertex) for k in (0, 1)) for n in neighbors]
        direction = tuple(edges[0][k] + edges[1][k] for k in (0, 1))
        length = math.hypot(*direction)
        direction = tuple(v / length for v in direction)
        if sum(direction[k] * (center[k] - vertex[k]) for k in (0, 1)) < 0:
            direction = tuple(-v for v in direction)
        sign = 1 if ink_toward_center else -1
        result.extend([
            tuple(vertex[k] + depth * edges[0][k] for k in (0, 1)),
            tuple(vertex[k] + sign * depth * direction[k] for k in (0, 1)),
            tuple(vertex[k] + depth * edges[1][k] for k in (0, 1)),
        ])
    return result


def cases():
    rectangle = [(50, 0), (550, 0), (550, 700), (50, 700)]
    def rounded(x, y):
        return math.hypot(x - max(110, min(490, x)), y - max(60, min(640, y))) <= 60
    for reverse in (False, True):
        vertices = list(reversed(rectangle)) if reverse else rectangle
        yield ('round-cw' if reverse else 'round-ccw', [vertices], [(0, i, 60) for i in range(4)], [], rounded)
    yield ('round-mask', [rectangle], [(0, i, 60) for i in range(4)],
           [mask('subtract', 'M250 -50L350 -50L350 750L250 750Z')],
           lambda x, y: rounded(x, y) and not 250 <= x <= 350)
    traps = trap_polygon(rectangle, range(4), 60)
    yield ('traps', [rectangle], [(0, i, -60) for i in range(4)], [],
           lambda x, y: polygon_contains(traps, x, y))
    counter = [(150, 100), (150, 600), (450, 600), (450, 100)]
    cutout = trap_polygon(counter, [3], 40, False)
    yield ('counter-trap', [rectangle, counter], [(1, 3, -40)], [],
           lambda x, y: 50 <= x <= 550 and 0 <= y <= 700 and not polygon_contains(cutout, x, y))
    concave = [(50, 0), (550, 0), (550, 350), (300, 350), (300, 700), (50, 700)]
    yield ('concave-round', [concave], [(0, 3, 40)], [],
           lambda x, y: polygon_contains(concave, x, y) or
           (300 <= x <= 340 and 350 <= y <= 390 and math.hypot(x - 340, y - 390) >= 40))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', default='http://127.0.0.1:8895')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    source = output / 'source.ttf'
    synthetic(source)
    report = dict(complete=False, fonttools=fontTools.__version__, exports=[], samples=0, timings=[])

    def call(method, **params):
        start = time.perf_counter()
        req = urllib.request.Request(args.api + '/api/' + method,
            data=json.dumps(params).encode(), headers={'Content-Type': 'application/json'})
        try:
            with urllib.request.urlopen(req, timeout=60) as response:
                value = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(error.read().decode()) from error
        assert 'error' not in value, value
        elapsed = time.perf_counter() - start
        assert elapsed < 60
        report['timings'].append(dict(method=method, seconds=round(elapsed, 3)))
        return value['result']

    try:
        for label, polygons, radii, effects, expected in cases():
            doc = call('session.open', path=str(source))['doc']
            try:
                cells = call('glyphs.list', doc=doc, filter={}, offset=0, limit=10, include_paths=True)['items']
                gid = next(g['gid'] for g in cells if g['name'] == 'A')
                layer = call('glyphs.detail', doc=doc, gid=gid)['glyph']['layers'][0]
                contours = [dict(points=[point(x,y) for x,y in polygon], closed=True, name=None, id=None) for polygon in polygons]
                call('doc.apply', doc=doc, label='Corner source', command={'ReplaceLayerOutline':dict(
                    glyph='A', layer=layer['id'], contours=contours, components=[], anchors=[])})
                for ci, pi, radius in radii:
                    call('doc.apply', doc=doc, label='Smart corner', command={'SetSmartCorner':dict(
                        glyph='A', layer=layer['id'], points=[dict(contour=ci,point=pi)], radius=radius)})
                    contours[ci]['points'][pi]['corner_radius'] = radius
                params = dict(doc=doc, glyph='A', layer=layer['id'], effects=effects,
                              dry_run=True, bake=False, expected_generation=None)
                preview = call('effects.apply', **params)
                if effects:
                    call('effects.apply', **{**params, 'dry_run':False, 'expected_generation':preview['generation']})
                native = output / (label + '.fcf')
                call('session.save', doc=doc, path=str(native))
                saved = json.loads(native.read_text())['font']
                assert native_semantics(saved['glyphs']['A']['layers'][0]['contours']) == native_semantics(contours)
                assert saved['glyphs']['B']['layers'][0]['components']
                reopened = call('session.open', path=str(native))['doc']
                try:
                    call('session.save', doc=reopened, path=str(output/'reopened.fcf'))
                    assert native_semantics(json.loads((output/'reopened.fcf').read_text())['font']) == native_semantics(saved)
                finally:
                    call('session.close', doc=reopened)
                call('doc.undo', doc=doc)
                call('doc.redo', doc=doc)
                for format_ in ['Ttf', 'Otf', 'Woff', 'Woff2']:
                    path = output / (label + '.' + format_.lower())
                    call('session.export', doc=doc, path=str(path), options={'format':format_})
                    with TTFont(path, checkChecksums=2) as font:
                        count = 0
                        for x in range(20, 581, 9):
                            for y in range(-30, 731, 9):
                                px, py = x + 0.371, y + 0.613
                                ink = expected(px, py)
                                if any(expected(px+dx,py+dy) != ink for dx,dy in [(1.5,0),(-1.5,0),(0,1.5),(0,-1.5)]):
                                    continue
                                for name, shift in [('A',0), ('B',100)]:
                                    actual = inside(font,name,(px+shift,py))
                                    assert actual == ink, (label, format_, name, px, py, actual, ink)
                                    count += 1
                        assert font['hmtx']['A'][0] == font['hmtx']['B'][0] == 700
                        report['exports'].append(dict(case=label, format=format_, samples=count))
                        report['samples'] += count
                preview = call('effects.apply', **params)
                call('effects.apply', **{**params, 'dry_run':False, 'bake':True, 'expected_generation':preview['generation']})
                call('session.save', doc=doc, path=str(output/'baked.fcf'))
                baked = json.loads((output/'baked.fcf').read_text())['font']['glyphs']['A']['layers'][0]
                assert not baked.get('effects')
                assert all(not p.get('corner_radius') for c in baked['contours'] for p in c['points'])
                call('doc.undo', doc=doc)
                call('session.save', doc=doc, path=str(output/'undo.fcf'))
                assert native_semantics(json.loads((output/'undo.fcf').read_text())['font']) == native_semantics(saved)
                print(label, 'passed', flush=True)
            finally:
                call('session.close', doc=doc)
        report['complete'] = True
    finally:
        (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(complete=report['complete'], exports=len(report['exports']), samples=report['samples'])))


if __name__ == '__main__':
    main()
