#!/usr/bin/env python3
"""Check one analytic line join on real outlines, for rounding and ink traps.

Uses original FontTools outlines as the fill oracle plus independent circle or
polygon cuts at the edited join. Boundary-adjacent samples allow integer export
rounding. Fixtures remain outside the generated acceptance artifacts.
"""
from __future__ import annotations
import argparse
import json
import math
import time
import urllib.error
import urllib.request
from pathlib import Path
from fontTools.ttLib import TTFont
from p5_style_acceptance import inside, native_semantics
from p5_smart_corners_acceptance import polygon_contains


def construction(font, name, layer, radius):
    for ci, contour in enumerate(layer['contours']):
        points = contour['points']
        for pi, p in enumerate(points):
            triple = [points[pi-1], p, points[(pi+1) % len(points)]]
            if not contour['closed'] or any(p['kind'] != 'Line' for p in triple):
                continue
            a, v, b = [(p['x'], p['y']) for p in triple]
            la, lb = math.dist(a,v), math.dist(v,b)
            if not la or not lb:
                continue
            ta = tuple((v[k]-a[k])/la for k in (0,1))
            tb = tuple((b[k]-v[k])/lb for k in (0,1))
            turn = math.atan2(ta[0]*tb[1]-ta[1]*tb[0], sum(ta[k]*tb[k] for k in (0,1)))
            if not 0.3 < abs(turn) < 2.6:
                continue
            sign = 1 if turn > 0 else -1
            setback = radius * math.tan(abs(turn)/2)
            if min(la,lb) <= max(setback,radius)*2:
                continue
            direction = (-sign*(ta[1]+tb[1]), sign*(ta[0]+tb[0]))
            length = math.hypot(*direction)
            direction = tuple(x/length for x in direction)
            if not inside(font,name,tuple(v[k]+direction[k] for k in (0,1))):
                continue
            if inside(font,name,tuple(v[k]-direction[k] for k in (0,1))):
                continue
            start = tuple(v[k]-ta[k]*setback for k in (0,1))
            end = tuple(v[k]+tb[k]*setback for k in (0,1))
            center = (start[0]-sign*ta[1]*radius, start[1]+sign*ta[0]*radius)
            tip = tuple(v[k]+direction[k]*radius for k in (0,1))
            trap = [tuple(v[k]-ta[k]*radius for k in (0,1)), v,
                    tuple(v[k]+tb[k]*radius for k in (0,1)), tip]
            return dict(contour=ci,point=pi), (start,v,end), center, trap
    raise AssertionError(('No suitable line join',name))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api',default='http://127.0.0.1:8895')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    output=args.output.resolve();output.mkdir(parents=True,exist_ok=True)
    report=dict(complete=False,cases=[])
    def call(method,**params):
        request=urllib.request.Request(args.api+'/api/'+method,data=json.dumps(params).encode(),headers={'Content-Type':'application/json'})
        try:
            with urllib.request.urlopen(request,timeout=120) as response: value=json.load(response)
        except urllib.error.HTTPError as error: raise RuntimeError(error.read().decode()) from error
        assert 'error' not in value,value
        return value['result']
    root=Path(__file__).resolve().parents[1]
    try:
        for filename in ['DejaVuSans.ttf','NotoSans-Regular.ttf']:
            source=root/'fixtures/fonts'/filename
            for sign in [1,-1]:
                doc=call('session.open',path=str(source))['doc']
                try:
                    items=call('glyphs.list',doc=doc,filter={},offset=0,limit=2000,include_paths=False)['items']
                    with TTFont(source) as original:
                        edits={}
                        for name in ['A','H','N']:
                            gid=next(g['gid'] for g in items if g['name']==name)
                            layer=call('glyphs.detail',doc=doc,gid=gid)['glyph']['layers'][0]
                            ref,triangle,center,trap=construction(original,name,layer,20.)
                            start=time.perf_counter()
                            call('doc.apply',doc=doc,label='Real outline corner',command={'SetSmartCorner':dict(glyph=name,layer=layer['id'],points=[ref],radius=sign*20.)})
                            elapsed=time.perf_counter()-start
                            assert elapsed<30
                            edits[name]=dict(ref=ref,triangle=triangle,center=center,trap=trap,seconds=round(elapsed,3),source=layer['contours'])
                        label=source.stem+('-round' if sign>0 else '-trap')
                        native=output/(label+'.fcf');call('session.save',doc=doc,path=str(native))
                        saved=json.loads(native.read_text())['font']
                        for name,edit in edits.items():
                            expected=edit['source']
                            ref=edit['ref'];expected[ref['contour']]['points'][ref['point']]['corner_radius']=sign*20.
                            assert native_semantics(saved['glyphs'][name]['layers'][0]['contours'])==native_semantics(expected)
                        for format_ in ['Ttf','Otf']:
                            path=output/(label+'.'+format_.lower())
                            call('session.export',doc=doc,path=str(path),options={'format':format_,'preserve_untouched_tables':False})
                            with TTFont(path,checkChecksums=2) as edited:
                                count=0
                                for name,edit in edits.items():
                                    triangle,center,trap=edit['triangle'],edit['center'],edit['trap']
                                    def expected(x,y):
                                        before=inside(original,name,(x,y))
                                        cut=polygon_contains(trap,x,y) if sign<0 else (polygon_contains(list(triangle),x,y) and math.dist((x,y),center)>20.)
                                        return before and not cut
                                    v=triangle[1]
                                    # Dense local grid makes a small corner change observable.
                                    for dx in range(-70,71,2):
                                        for dy in range(-70,71,2):
                                            x,y=v[0]+dx+0.371,v[1]+dy+0.613
                                            ink=expected(x,y)
                                            if any(expected(x+ex,y+ey)!=ink for ex,ey in [(1.5,0),(-1.5,0),(0,1.5),(0,-1.5)]): continue
                                            assert inside(edited,name,(x,y))==ink,(label,format_,name,x,y,ink)
                                            count+=1
                                    assert edited['hmtx'][name][0]==original['hmtx'][name][0]
                                report['cases'].append(dict(font=filename,sign=sign,format=format_,samples=count,edits={name:{k:v for k,v in edit.items() if k!='source'} for name,edit in edits.items()}))
                        print(label,'passed',flush=True)
                finally: call('session.close',doc=doc)
        report['complete']=True
    finally: (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(complete=report['complete'],exports=len(report['cases']),samples=sum(c['samples'] for c in report['cases']))))


if __name__=='__main__': main()
