#!/usr/bin/env python3
"""Independent batch style acceptance with original FontTools-built outlines.

No downloaded font data. Checks affine presets, weight/counters, decomposition,
advances, native backup persistence and four compiled containers.
"""
from __future__ import annotations
import argparse
import copy
import json
import math
import time
import urllib.error
import urllib.request
from pathlib import Path
import fontTools
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.pointInsidePen import PointInsidePen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont


def synthetic(path):
    builder = FontBuilder(1000, isTTF=True)
    order = ['.notdef', 'space', 'A', 'B']
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap({32:'space', 65:'A', 66:'B'})
    glyphs = {}
    for name in order:
        pen = TTGlyphPen(glyphs)
        if name == 'A':
            for points in [[(50,0),(50,700),(550,700),(550,0)], [(150,100),(450,100),(450,600),(150,600)]]:
                pen.moveTo(points[0])
                for point in points[1:]: pen.lineTo(point)
                pen.closePath()
        if name == 'B': pen.addComponent('A', (1,0,0,1,100,0))
        glyphs[name] = pen.glyph()
    builder.setupGlyf(glyphs)
    builder.setupHorizontalMetrics({name: (700, 50 if name == 'A' else 150 if name == 'B' else 0) for name in order})
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable({'familyName':'Style Acceptance','styleName':'Regular','uniqueFontIdentifier':'FontCarver Style Acceptance 1','fullName':'Style Acceptance Regular','psName':'StyleAcceptance-Regular','version':'Version 1.0'})
    builder.setupOS2(sTypoAscender=800,sTypoDescender=-200,usWinAscent=800,usWinDescent=200)
    builder.setupPost()
    builder.font.save(path)


def bounds(font, name):
    glyphs = font.getGlyphSet()
    pen = BoundsPen(glyphs)
    glyphs[name].draw(pen)
    return pen.bounds


def inside(font, name, point):
    glyphs = font.getGlyphSet()
    pen = PointInsidePen(glyphs, point, evenOdd=False)
    glyphs[name].draw(pen)
    return pen.getResult()


def native_semantics(value):
    # JSON parse/print may change the last f64 bit; compare font-unit semantics.
    if isinstance(value, float): return round(value, 9)
    if isinstance(value, dict): return {k:native_semantics(v) for k,v in value.items()}
    if isinstance(value, list): return [native_semantics(v) for v in value]
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', default='http://127.0.0.1:8895')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve(); output.mkdir(parents=True, exist_ok=True)
    source = output / 'style-acceptance.ttf'; synthetic(source)
    report = {'fonttools':fontTools.__version__, 'complete':False, 'exports':[], 'timings':[]}
    def call(method, **params):
        start = time.perf_counter()
        request = urllib.request.Request(f'{args.api}/api/{method}', data=json.dumps(params).encode(), headers={'Content-Type':'application/json'})
        try:
            with urllib.request.urlopen(request, timeout=60) as response: envelope = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(f'{method}: {error.read().decode()}') from error
        assert 'error' not in envelope, envelope
        elapsed = time.perf_counter()-start
        assert elapsed < 60, (method, elapsed)
        report['timings'].append({'method':method,'seconds':round(elapsed,3)})
        return envelope['result']
    base = {'scale':[1,1], 'translate':[0,0], 'oblique':0, 'weight':[0,0]}
    cases = {
        'embolden':{'weight':[20,10]}, 'lighten':{'weight':[-20,-10]},
        'condense':{'scale':[0.85,1]}, 'extend':{'scale':[1.15,1]},
        'oblique':{'oblique':12}, 'small-caps':{'scale':[0.75,0.75]},
        'subscript':{'scale':[0.6,0.6],'translate':[0,-150]},
        'superscript':{'scale':[0.6,0.6],'translate':[0,350]},
    }
    try:
        for case, overrides in cases.items():
            doc = call('session.open', path=str(source))['doc']
            try:
                listed = call('glyphs.list',doc=doc,filter={},offset=0,limit=10,include_paths=True)['items']
                glyphs = [g['gid'] for g in listed if g['name'] in ['A','B']]
                options = {**copy.deepcopy(base), **overrides}
                args_ = {'doc':doc,'glyphs':glyphs,'master':None,'options':options,'scale_advance':True,'center':False,'backup':True,'dry_run':True,'expected_generation':None,'label':case}
                preview = call('style.batch', **args_)
                assert preview['glyph_count'] == 2 and preview['decomposed_layers'] == 1
                call('style.batch', **{**args_,'dry_run':False,'expected_generation':preview['generation']})
                native = output / f'{case}.fcf'
                call('session.save',doc=doc,path=str(native))
                saved = json.loads(native.read_text())['font']
                for name in ['A','B']:
                    layers = saved['glyphs'][name]['layers']
                    assert len(layers)==2
                    assert all(not layer['components'] for layer in layers)
                    assert sum('Backup' in layer['kind'] for layer in layers)==1
                reopened = call('session.open',path=str(native))['doc']
                try:
                    path2 = output / f'{case}-reopened.fcf'
                    call('session.save',doc=reopened,path=str(path2))
                    assert native_semantics(json.loads(path2.read_text())['font']) == native_semantics(saved)
                finally: call('session.close',doc=reopened)
                call('doc.undo',doc=doc)
                restored = call('glyphs.list',doc=doc,filter={},offset=0,limit=10,include_paths=True)['items']
                assert [(g['name'],g['path'],g['width']) for g in restored] == [(g['name'],g['path'],g['width']) for g in listed]
                call('doc.redo',doc=doc)
                for format_ in ['Ttf','Otf','Woff','Woff2']:
                    path = output / f'{case}.{format_.lower()}'
                    call('session.export',doc=doc,path=str(path),options={'format':format_})
                    with TTFont(path,checkChecksums=2) as font:
                        for name, shift in [('A',0),('B',100)]:
                            sx,sy = options['scale']; tx,ty = options['translate']; wx,wy = options['weight']
                            shear = math.tan(math.radians(options['oblique']))
                            x0=(50+shift)*sx+tx; x1=(550+shift)*sx+tx
                            expected=(x0-wx,ty-wy,x1+700*sy*shear+wx,700*sy+ty+wy)
                            actual=bounds(font,name)
                            assert max(abs(a-b) for a,b in zip(actual,expected)) <= 1.1, (case,format_,name,actual,expected)
                            assert abs(font['hmtx'][name][0] - 700*sx) <= 0.5
                            if wx:
                                assert not inside(font,name,(300+shift,350)), (case,'counter filled')
                                assert inside(font,name,(155+shift,350)) == (wx>0), (case,'wrong inner boundary')
                            if 'glyf' in font: assert not font['glyf'][name].isComposite()
                    report['exports'].append(path.name)
                print(f'{case}: four independently decoded exports, backups, save/reopen, undo/redo',flush=True)
            finally: call('session.close',doc=doc)
        report['complete'] = True
    finally: (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')

if __name__ == '__main__': main()
