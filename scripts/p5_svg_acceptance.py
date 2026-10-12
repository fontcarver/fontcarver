#!/usr/bin/env python3
"""Independently check original static OpenType SVG artwork with FontTools/XML.

Start an isolated fc-devserver on 8895; all generated artifacts stay in --output.
"""
from __future__ import annotations
import argparse
import copy
import json
import re
import time
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path
import fontTools
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables.S_V_G_ import SVGDocument
from p5_color_acceptance import synthetic as color_fixture

SOURCE = '''<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<defs><linearGradient id="ink" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2563eb"/><stop offset="1" stop-color="#d946ef"/></linearGradient><path id="triangle" d="M80 0L350 -700L620 0Z"/></defs>
<g id="glyph2"><use xlink:href="#triangle" fill="url(#ink)"/><circle cx="350" cy="-210" r="60" fill="#fbbf24" fill-opacity="0.7"/></g>
<g id="glyph3"><rect x="80" y="-650" width="500" height="650" rx="90" fill="#059669"/></g>
<g id="glyph5"><use href="#glyph2" transform="translate(20 0)"/></g>
</svg>'''


def synthetic(path):
    color_fixture(path)
    with TTFont(path) as font:
        del font['COLR']
        del font['CPAL']
        font['SVG '] = newTable('SVG ')
        font['SVG '].docList = [SVGDocument(SOURCE, 2, 3, compressed=True),
                               SVGDocument(SOURCE, 5, 5, compressed=True)]
        font.save(path)


def semantic(source, target):
    """Compare XML and target identity while ignoring renamed identifiers/spacing."""
    root = ET.fromstring(source)
    ids = {e.attrib['id']: f'id{i}' for i, e in enumerate(root.iter()) if 'id' in e.attrib}
    def value(key, text):
        if key == 'id':
            return ids[text]
        if key.split('}')[-1] == 'href':
            return '#' + ids[text[1:]]
        return re.sub(r'url\(\s*[\'\"]?#([^\s\)\'\"]+)[\'\"]?\s*\)',
                      lambda match: 'url(#' + ids[match[1]] + ')', text)
    def node(e):
        return (e.tag, sorted((k, value(k, v)) for k, v in e.attrib.items()),
                (e.text or '').strip(), [node(child) for child in e])
    return node(root), ids[target]


def snapshot(path):
    result = {}
    with TTFont(path, checkChecksums=2) as font:
        for document in font['SVG '].docList:
            for gid in range(document.startGlyphID, document.endGlyphID + 1):
                result[font.getGlyphName(gid)] = semantic(document.data, f'glyph{gid}')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', default='http://127.0.0.1:8895')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'fonttools': fontTools.__version__, 'complete': False, 'operations': [], 'exports': []}
    report_path = args.output / 'report.json'
    def save():
        report_path.write_text(json.dumps(report, indent=2) + '\n')
    def call(method, **params):
        start = time.perf_counter()
        request = urllib.request.Request(f'{args.api}/api/{method}', data=json.dumps(params).encode(), headers={'Content-Type': 'application/json'})
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                result = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(error.read().decode()) from error
        assert 'error' not in result, result
        report['operations'].append({'method': method, 'seconds': round(time.perf_counter()-start, 3)})
        save()
        return result['result']
    try:
        fixture = args.output / 'original-svg.ttf'
        synthetic(fixture)
        doc = call('session.open', path=str(fixture))['doc']
        initial = call('svg.state', doc=doc)
        assert initial['status'] == 'Editable'
        assert len(initial['svg']['documents']) == 1, 'shared documents should be decoded once'
        expected = snapshot(fixture)
        for stage in ['baseline', 'edited']:
            if stage == 'edited':
                table = copy.deepcopy(initial['svg'])
                table['documents'][0]['source'] = table['documents'][0]['source'].replace('#2563eb', '#ef4444')
                call('svg.apply', doc=doc, expected_generation=initial['generation'], svg=table, label='SVG color')
                call('doc.apply', doc=doc, command={'RenameGlyph': {'from': 'A', 'to': 'custom.A'}}, label='Rename SVG base')
                expected = {('custom.A' if name == 'A' else name): semantic(SOURCE.replace('#2563eb', '#ef4444'), f'glyph{gid}')
                            for name, gid in [('A', 2), ('B', 3), ('triangle', 5)]}
            for fmt in ['Ttf', 'Otf', 'Woff', 'Woff2']:
                path = args.output / f'{stage}.{fmt.lower()}'
                call('session.export', doc=doc, path=str(path), options={'format': fmt, 'preserve_tables': False})
                assert snapshot(path) == expected, path
                report['exports'].append(str(path))
                save()
        edited = call('svg.state', doc=doc)
        call('doc.undo', doc=doc)
        assert 'A' in call('svg.state', doc=doc)['svg']['documents'][0]['glyphs']
        call('doc.redo', doc=doc)
        assert call('svg.state', doc=doc)['svg'] == edited['svg']
        native = args.output / 'edited.fcf'
        call('session.save', doc=doc, path=str(native))
        reopened = call('session.open', path=str(native))['doc']
        assert call('svg.state', doc=reopened)['svg'] == edited['svg']
        report['complete'] = True
    except Exception as error:
        report['failure'] = str(error)
        raise
    finally:
        save()
    print(json.dumps({'complete': True, 'exports': len(report['exports']), 'report': str(report_path)}))


if __name__ == '__main__':
    main()
