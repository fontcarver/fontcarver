#!/usr/bin/env python3
"""Independent live-effect geometry, native persistence and four-container checks."""
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
from p5_style_acceptance import synthetic, inside, bounds, native_semantics


def point(x, y, move=False):
    return dict(x=x, y=y, kind='Move' if move else 'Line', smooth=False, name=None, tt_index='Unnumbered')


def stroke(width=40, height=80, angle=0, cap='round', join='round', nib='ellipse'):
    return dict(enabled=True, effect=dict(kind='stroke', nib=nib, width=width, height=height,
                                         angle=angle, cap=cap, join=join, miter_limit=4))


def mask(operation, path):
    return dict(enabled=True, effect=dict(kind='boolean', operation=operation, path=path))


def ellipse_line(x, y, angle, cap):
    """Analytic pen-coordinate distance to a horizontal centerline (100,0)-(500,0)."""
    angle = math.radians(angle)
    def transform(x, y):
        return ((math.cos(angle)*x+math.sin(angle)*y)*2,
                -math.sin(angle)*x+math.cos(angle)*y)
    px, py = transform(x-100, y)
    dx, dy = transform(400, 0)
    length = math.hypot(dx,dy)
    along = (px*dx+py*dy)/length
    across = (px*dy-py*dx)/length
    if cap == 'round':
        return math.hypot(across, max(-along, along-length, 0)) <= 40
    low, high = (0, length) if cap == 'butt' else (-40,length+40)
    return low <= along <= high and abs(across) <= 40


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', default='http://127.0.0.1:8895')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--case-prefix', default='')
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=True)
    source = out/'source.ttf'; synthetic(source)
    report = dict(complete=False, fonttools=fontTools.__version__, exports=[], samples=0, timings=[])
    def call(method, **params):
        started = time.perf_counter()
        req = urllib.request.Request(f'{args.api}/api/{method}', data=json.dumps(params).encode(),headers={'Content-Type':'application/json'})
        try:
            with urllib.request.urlopen(req, timeout=60) as response: value = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(f'{method}: {error.read().decode()}') from error
        assert 'error' not in value, value
        report['timings'].append(dict(method=method,seconds=round(time.perf_counter()-started,3)))
        return value['result']
    cases = []
    for cap in ['round','butt','square']:
        for angle in [0,30]:
            cases.append((f'ellipse-{cap}-{angle}', [stroke(angle=angle,cap=cap)],
                          lambda x,y,a=angle,c=cap: ellipse_line(x,y,a,c), False))
    for angle in [0,30]:
        theta = math.radians(angle)
        vertices = sorted((end+math.cos(theta)*dx-math.sin(theta)*dy,math.sin(theta)*dx+math.cos(theta)*dy)
                          for end in [100,500] for dx in [-20,20] for dy in [-40,40])
        def cross(o,a,b): return (a[0]-o[0])*(b[1]-o[1])-(a[1]-o[1])*(b[0]-o[0])
        def half(points):
            out=[]
            for p in points:
                while len(out)>=2 and cross(out[-2],out[-1],p)<=0: out.pop()
                out.append(p)
            return out
        polygon=half(vertices)[:-1]+half(list(reversed(vertices)))[:-1]
        def expected(x,y,polygon=polygon):
            return all(cross(a,b,(x,y))>=0 for a,b in zip(polygon,polygon[1:]+polygon[:1]))
        cases.append((f'rectangle-round-{angle}',[stroke(angle=angle,nib='rectangle')],expected,False))
    for operation in ['union','intersect','subtract','xor']:
        def expected(x,y,operation=operation):
            a = ellipse_line(x,y,0,'round'); b = 300 <= x <= 600 and -20 <= y <= 70
            return {'union':a or b,'intersect':a and b,'subtract':a and not b,'xor':a != b}[operation]
        cases.append((f'mask-{operation}', [stroke(),mask(operation,'M300 -20L600 -20L600 70L300 70Z')], expected, False))
    for join in ['round','bevel','miter']:
        def expected(x,y,join=join):
            # A 10×10 closed rectangle with a 40-unit pen. Its entire inner
            # region is ink. Corners depend on the selected join.
            dx=max(100-x,x-110,0);dy=max(-y,y-10,0)
            if join=='round': return math.hypot(dx,dy)<=20
            if join=='bevel': return dx+dy<=20
            return max(dx,dy)<=20
        cases.append((f'thick-closed-{join}',[stroke(40,40,join=join)],expected,True))
    try:
        for case,effects,expected,closed in cases:
            if not case.startswith(args.case_prefix): continue
            doc = call('session.open',path=str(source))['doc']
            try:
                cells = call('glyphs.list',doc=doc,filter={},offset=0,limit=10,include_paths=True)['items']
                gid = next(g['gid'] for g in cells if g['name']=='A')
                detail = call('glyphs.detail',doc=doc,gid=gid)
                layer = detail['glyph']['layers'][0]
                points = [point(100,0),point(110,0),point(110,10),point(100,10)] if closed else [point(100,0,True),point(500,0)]
                contours = [dict(points=points,closed=closed,name=None,id=None)]
                call('doc.apply',doc=doc,label='Acceptance source',command={'ReplaceLayerOutline':dict(glyph='A',layer=layer['id'],contours=contours,components=[],anchors=[])})
                params = dict(doc=doc,glyph='A',layer=layer['id'],effects=effects,dry_run=True,bake=False,expected_generation=None)
                preview = call('effects.apply',**params)
                call('effects.apply',**{**params,'dry_run':False,'expected_generation':preview['generation']})
                native = out/f'{case}.fcf';call('session.save',doc=doc,path=str(native))
                saved = json.loads(native.read_text())['font']
                stored = saved['glyphs']['A']['layers'][0]
                assert native_semantics(stored['contours']) == native_semantics(contours)
                assert stored['effects']==effects
                assert saved['glyphs']['B']['layers'][0]['components'], 'live component reference must survive'
                reopened = call('session.open',path=str(native))['doc']
                call('session.save',doc=reopened,path=str(out/'reopened.fcf'))
                assert native_semantics(json.loads((out/'reopened.fcf').read_text())['font']) == native_semantics(saved)
                call('session.close',doc=reopened)
                call('doc.undo',doc=doc);call('doc.redo',doc=doc)
                for format_ in ['Ttf','Otf','Woff','Woff2']:
                    path = out/f'{case}.{format_.lower()}'
                    call('session.export',doc=doc,path=str(path),options={'format':format_})
                    with TTFont(path,checkChecksums=2) as font:
                        checked=0
                        for name,shift in [('A',0),('B',100)]:
                            for x in range(50,651,13 if not closed else 3):
                                if closed and x>160: break
                                for y in range(-80,101,11 if not closed else 3):
                                    # Stay away from analytic edges and horizontal-ray
                                    # coincidences in independent quadratic containment.
                                    px=x+0.371;py=y+0.613
                                    ink=expected(px,py)
                                    if any(expected(px+dx,py+dy)!=ink for dx,dy in [(1.5,0),(-1.5,0),(0,1.5),(0,-1.5)]): continue
                                    actual=inside(font,name,(px+shift,py))
                                    assert actual==ink,(case,format_,name,px,py,actual,ink)
                                    checked+=1
                            assert font['hmtx'][name][0]==700
                        report['samples']+=checked
                        report['exports'].append(dict(case=case,format=format_,points=checked,bounds=bounds(font,'A')))
                bake = call('effects.apply',**params)
                call('effects.apply',**{**params,'dry_run':False,'bake':True,'expected_generation':bake['generation']})
                call('doc.undo',doc=doc)
                call('session.save',doc=doc,path=str(out/'undo.fcf'))
                assert native_semantics(json.loads((out/'undo.fcf').read_text())['font'])==native_semantics(saved)
                print(case,'passed',flush=True)
            finally: call('session.close',doc=doc)
        report['complete']=True
    finally:
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(complete=True,exports=len(report['exports']),samples=report['samples'])))


if __name__=='__main__': main()
