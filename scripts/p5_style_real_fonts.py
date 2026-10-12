#!/usr/bin/env python3
"""Real-font style acceptance with independent FontTools point/outline queries.

Requires FontTools and an isolated fc-devserver. Verifies +/-10-unit weight on
A/B/O/S/g/a/e/Aacute from the committed DejaVu Sans and Noto Sans fixtures.
A regular point grid is checked against distance to independently flattened
original boundaries. Samples within 1.5 units of the expected boundary are
excluded to allow quadratic approximation and integer export rounding.
"""
from __future__ import annotations
import argparse
import json
import math
import time
import urllib.error
import urllib.request
from pathlib import Path
from fontTools.pens.basePen import BasePen
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.pointInsidePen import PointInsidePen
from fontTools.ttLib import TTFont


class SegmentsPen(BasePen):
    """Sample curves in their original representation, at <=5-unit intervals."""
    def __init__(self, glyphs):
        super().__init__(glyphs); self.segments = []; self.start = None
    def _moveTo(self, p): self.start = p
    def _lineTo(self, p): self.segments.append((self._getCurrentPoint(), p))
    def _curveToOne(self, a, b, c):
        p = self._getCurrentPoint()
        n = max(2, math.ceil((math.dist(p,a)+math.dist(a,b)+math.dist(b,c))/5))
        prev = p
        for i in range(1,n+1):
            t=i/n; u=1-t
            point=tuple(u**3*p[k]+3*u*u*t*a[k]+3*u*t*t*b[k]+t**3*c[k] for k in (0,1))
            self.segments.append((prev,point));prev=point
    def _qCurveToOne(self, a, b):
        p=self._getCurrentPoint()
        n=max(2,math.ceil((math.dist(p,a)+math.dist(a,b))/5));prev=p
        for i in range(1,n+1):
            t=i/n;u=1-t;point=tuple(u*u*p[k]+2*u*t*a[k]+t*t*b[k] for k in (0,1))
            self.segments.append((prev,point));prev=point
    def _closePath(self):
        if self.start != self._getCurrentPoint(): self._lineTo(self.start)
    def _endPath(self): pass


def distance(p, segments):
    result = float('inf')
    for a,b in segments:
        dx=b[0]-a[0];dy=b[1]-a[1];length=dx*dx+dy*dy
        t=max(0,min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/length)) if length else 0
        result=min(result,(p[0]-a[0]-t*dx)**2+(p[1]-a[1]-t*dy)**2)
    return math.sqrt(result)


def inside(glyphs, name, point):
    pen=PointInsidePen(glyphs,point,evenOdd=False);glyphs[name].draw(pen)
    return pen.getResult()


def verify(source, output, names, weight):
    with TTFont(source) as original, TTFont(output) as edited:
        before=original.getGlyphSet();after=edited.getGlyphSet();samples=0
        for name in names:
            pen=BoundsPen(before);before[name].draw(pen)
            x0,y0,x1,y1=pen.bounds
            boundary=SegmentsPen(before);before[name].draw(boundary)
            for i in range(25):
                for j in range(25):
                    # Avoid integer/half-grid scanlines through source vertices:
                    # PointInsidePen is ambiguous on exactly coincident rays.
                    p=(x0-20+(x1-x0+40)*(i+0.371)/25,y0-20+(y1-y0+40)*(j+0.613)/25)
                    dist=distance(p,boundary.segments)
                    if abs(dist-abs(weight)) < 1.5: continue
                    was_inside=inside(before,name,p)
                    expected=(was_inside or dist < weight) if weight > 0 else (was_inside and dist > -weight)
                    assert inside(after,name,p)==expected,(str(source),weight,name,p,dist,expected)
                    samples+=1
            assert edited['hmtx'][name][0] == original['hmtx'][name][0]
        return samples


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api',default='http://127.0.0.1:8895')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();output=args.output.resolve();output.mkdir(parents=True,exist_ok=True)
    root=Path(__file__).resolve().parents[1];report={'complete':False,'cases':[]}
    def call(method,**params):
        req=urllib.request.Request(args.api+'/api/'+method,data=json.dumps(params).encode(),headers={'Content-Type':'application/json'})
        try:
            with urllib.request.urlopen(req,timeout=120) as response: envelope=json.load(response)
        except urllib.error.HTTPError as error: raise RuntimeError(error.read().decode()) from error
        assert 'error' not in envelope,envelope
        return envelope['result']
    try:
        for filename in ['DejaVuSans.ttf','NotoSans-Regular.ttf']:
            source=root/'fixtures/fonts'/filename
            for weight in [10,-10]:
                doc=call('session.open',path=str(source))['doc']
                try:
                    items=call('glyphs.list',doc=doc,filter={},offset=0,limit=2000,include_paths=False)['items']
                    selected=[g for g in items if g['name'] in ['A','B','O','S','g','a','e','Aacute']]
                    params=dict(doc=doc,glyphs=[g['gid'] for g in selected],master=None,options=dict(scale=[1,1],translate=[0,0],oblique=0,weight=[weight,weight]),scale_advance=False,center=False,backup=True,dry_run=True,expected_generation=None,label='Real font weight')
                    start=time.perf_counter();preview=call('style.batch',**params);elapsed=time.perf_counter()-start
                    assert elapsed < 30,(filename,weight,elapsed)
                    call('style.batch',**{**params,'dry_run':False,'expected_generation':preview['generation']})
                    path=output/f'{source.stem}-{weight}.ttf'
                    call('session.export',doc=doc,path=str(path),options={'format':'Ttf'})
                    samples=verify(source,path,[g['name'] for g in selected],weight)
                    case=dict(font=filename,weight=weight,glyphs=len(selected),preview_seconds=round(elapsed,3),approximated_layers=preview['approximated_layers'],verified_samples=samples)
                    report['cases'].append(case);print(case,flush=True)
                finally:call('session.close',doc=doc)
        report['complete']=True
    finally:(output/'real-font-results.json').write_text(json.dumps(report,indent=2)+'\n')

if __name__=='__main__':main()
