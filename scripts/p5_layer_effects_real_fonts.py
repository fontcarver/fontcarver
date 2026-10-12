#!/usr/bin/env python3
"""Independently check live round strokes on committed real-font outlines.

Requires FontTools and an isolated fc-devserver. The expected stroke is the
10-unit neighborhood of the original outline, sampled independently with
FontTools. Boundary-adjacent samples are excluded for export rounding. Source
contours, anchors and components must remain unchanged in native documents.
"""
from __future__ import annotations

import argparse
import json
import time
import urllib.error
import urllib.request
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.ttLib import TTFont

from p5_style_real_fonts import SegmentsPen, distance, inside


def verify(source, output, names):
    with TTFont(source) as original, TTFont(output, checkChecksums=2) as edited:
        before = original.getGlyphSet()
        after = edited.getGlyphSet()
        samples = 0
        for name in names:
            bounds = BoundsPen(before)
            before[name].draw(bounds)
            x0, y0, x1, y1 = bounds.bounds
            boundary = SegmentsPen(before)
            before[name].draw(boundary)
            for i in range(31):
                for j in range(31):
                    p = (x0 - 20 + (x1 - x0 + 40) * (i + 0.371) / 31,
                         y0 - 20 + (y1 - y0 + 40) * (j + 0.613) / 31)
                    dist = distance(p, boundary.segments)
                    if abs(dist - 10) < 1.5:
                        continue
                    expected = dist < 10
                    assert inside(after, name, p) == expected, (
                        str(source), str(output), name, p, dist, expected)
                    samples += 1
            assert edited['hmtx'][name][0] == original['hmtx'][name][0]
        return samples


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', default='http://127.0.0.1:8895')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    root = Path(__file__).resolve().parents[1]
    report = {'complete': False, 'cases': []}

    def call(method, **params):
        req = urllib.request.Request(args.api + '/api/' + method,
            data=json.dumps(params).encode(), headers={'Content-Type': 'application/json'})
        try:
            with urllib.request.urlopen(req, timeout=180) as response:
                envelope = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(error.read().decode()) from error
        assert 'error' not in envelope, envelope
        return envelope['result']

    effects = [dict(enabled=True, effect=dict(kind='stroke', nib='ellipse',
        width=20, height=20, angle=0, cap='round', join='round', miter_limit=4))]
    try:
        for filename in ['DejaVuSans.ttf', 'NotoSans-Regular.ttf']:
            source = root / 'fixtures/fonts' / filename
            # Separate the composite from A: applying effects to both at once
            # intentionally evaluates the child before the parent's stroke.
            for names in [['A', 'B', 'O', 'S', 'g', 'a', 'e'], ['Aacute']]:
                doc = call('session.open', path=str(source))['doc']
                try:
                    items = call('glyphs.list', doc=doc, filter={}, offset=0,
                                 limit=2000, include_paths=False)['items']
                    glyphs = {g['name']: g for g in items}
                    saved_sources = {}
                    previews = []
                    for name in names:
                        detail = call('glyphs.detail', doc=doc, gid=glyphs[name]['gid'])
                        layer = detail['glyph']['layers'][0]
                        saved_sources[name] = layer
                        params = dict(doc=doc, glyph=name, layer=layer['id'],
                            effects=effects, dry_run=True, bake=False, expected_generation=None)
                        start = time.perf_counter()
                        preview = call('effects.apply', **params)
                        elapsed = time.perf_counter() - start
                        assert elapsed < 30, (filename, name, elapsed)
                        call('effects.apply', **{**params, 'dry_run': False,
                             'expected_generation': preview['generation']})
                        previews.append(dict(glyph=name, seconds=round(elapsed, 3),
                            approximated_layers=preview['approximated_layers']))
                    label = source.stem + ('-composite' if names == ['Aacute'] else '-simple')
                    native = output / (label + '.fcf')
                    call('session.save', doc=doc, path=str(native))
                    saved = json.loads(native.read_text())['font']
                    for name, original in saved_sources.items():
                        layer = saved['glyphs'][name]['layers'][0]
                        for field in ['contours', 'components', 'anchors', 'width']:
                            assert layer.get(field, []) == original.get(field, []), (name, field)
                        assert layer['effects'] == effects
                    for format_ in ['Ttf', 'Otf']:
                        path = output / (label + '.' + format_.lower())
                        # These checks measure geometry. DejaVu contains layout
                        # anchors bound to source point indices; keeping those
                        # after stroke expansion is correctly rejected.
                        call('session.export', doc=doc, path=str(path),
                             options={'format': format_, 'preserve_untouched_tables': False})
                        samples = verify(source, path, names)
                        case = dict(font=filename, format=format_, glyphs=names,
                            verified_samples=samples, previews=previews)
                        report['cases'].append(case)
                        print(json.dumps(case), flush=True)
                finally:
                    call('session.close', doc=doc)
        report['complete'] = True
    finally:
        (output / 'real-font-results.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
