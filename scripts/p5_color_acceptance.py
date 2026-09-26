#!/usr/bin/env python3
"""Independently verify COLR v0/CPAL import, editing, persistence and export.

Requires FontTools and a running isolated fc-devserver. Always generates an
original synthetic CPAL v1 font; optional external fonts add real-font coverage.
No downloaded fonts, generated exports or third-party art are committed.

Example:
  python3 scripts/p5_color_acceptance.py --output /tmp/fontcarver-color-qa/acceptance \
    /tmp/fontcarver-color-qa/Twemoji.Mozilla.ttf
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import time
import urllib.error
import urllib.request
from pathlib import Path

import fontTools
from fontTools.colorLib.builder import buildCOLR, buildCPAL
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont


def synthetic(path):
    """Original geometric artwork, with sparse labels and transparent colors."""
    builder = FontBuilder(1000, isTTF=True)
    order = [".notdef", "space", "A", "B", "square", "triangle", "bar"]
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap({32: "space", 65: "A", 66: "B"})
    outlines = {}
    shapes = {
        "A": [(60, 0), (60, 700), (640, 700), (640, 0)],
        "B": [(60, 0), (350, 700), (640, 0)],
        "square": [(60, 0), (60, 700), (640, 700), (640, 0)],
        "triangle": [(140, 100), (350, 610), (560, 100)],
        "bar": [(140, 240), (140, 320), (560, 320), (560, 240)],
    }
    for name in order:
        pen = TTGlyphPen(None)
        points = shapes.get(name, [])
        if points:
            pen.moveTo(points[0])
            for point in points[1:]:
                pen.lineTo(point)
            pen.closePath()
        outlines[name] = pen.glyph()
    builder.setupGlyf(outlines)
    builder.setupHorizontalMetrics({name: (700, 0) for name in order})
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable({"familyName": "Color Acceptance", "styleName": "Regular",
                            "uniqueFontIdentifier": "FontCarver Color Acceptance 1",
                            "fullName": "Color Acceptance Regular",
                            "psName": "ColorAcceptance-Regular", "version": "Version 1.0"})
    builder.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    builder.setupPost()
    font = builder.font
    palettes = [
        [(0.10, 0.40, 0.85, 1), (1, 0.65, 0.12, 0.6), (0, 0, 0, 0)],
        [(0.72, 0.18, 0.50, 1), (0.10, 0.78, 0.62, 0.8), (1, 1, 1, 0)],
    ]
    font["CPAL"] = buildCPAL(palettes, paletteTypes=[1, 2],
                            paletteLabels=["Day", "Night"],
                            paletteEntryLabels=["Base", None, "Invisible"],
                            nameTable=font["name"])
    font["COLR"] = buildCOLR({"A": [("square", 0), ("triangle", 1), ("bar", 0xFFFF)],
                               "B": [("B", 0), ("triangle", 2)]}, version=0,
                              glyphMap=font.getReverseGlyphMap())
    font.save(path)


def ft_snapshot(font):
    cpal = font["CPAL"]
    colr = font.get("COLR")
    assert colr is None or colr.version == 0
    count, entries = len(cpal.palettes), cpal.numPaletteEntries
    optional = lambda values: [None if v == 0xFFFF else v for v in values]
    return {
        "version": cpal.version,
        "palettes": [[[c.red, c.green, c.blue, c.alpha] for c in p] for p in cpal.palettes],
        "types": getattr(cpal, "paletteTypes", [0] * count),
        "labels": optional(getattr(cpal, "paletteLabels", [0xFFFF] * count)),
        "entry_labels": optional(getattr(cpal, "paletteEntryLabels", [0xFFFF] * entries)),
        "glyphs": sorted((font.getGlyphID(base), [(font.getGlyphID(l.name), l.colorID) for l in layers])
                         for base, layers in (colr.ColorLayers.items() if colr else [])),
    }


def model_snapshot(table, names):
    gids = {name: index for index, name in enumerate(names)}
    return {
        "version": table["palette_version"],
        "palettes": [[[c[k] for k in ("red", "green", "blue", "alpha")] for c in p["colors"]]
                     for p in table["palettes"]],
        "types": [p["flags"] for p in table["palettes"]],
        "labels": [p["label"] for p in table["palettes"]],
        "entry_labels": table["entry_labels"] or [None] * len(table["palettes"][0]["colors"]),
        "glyphs": sorted((gids[base], [(gids[l["glyph"]], l["palette_index"] if l["palette_index"] is not None else 0xFFFF)
                                      for l in layers]) for base, layers in table["glyphs"].items()),
    }


def verify(expected, path):
    with TTFont(path, checkChecksums=2) as actual:
        assert ft_snapshot(actual) == ft_snapshot(expected), f"color semantics differ: {path}"
        # Palette labels also need the strings referenced by their name IDs.
        snap = ft_snapshot(expected)
        for name_id in set(snap["labels"] + snap["entry_labels"]) - {None}:
            assert actual["name"].getDebugName(name_id) == expected["name"].getDebugName(name_id), (path, name_id)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fonts", type=Path, nargs="*")
    parser.add_argument("--api", default="http://127.0.0.1:8894")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--max-operation-seconds", type=float, default=120)
    args = parser.parse_args()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    fixture = args.output / "color-acceptance.ttf"
    synthetic(fixture)
    timings = []

    def call(method, **params):
        start = time.perf_counter()
        request = urllib.request.Request(f"{args.api}/api/{method}",
                                         data=json.dumps(params).encode(),
                                         headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(request, timeout=args.max_operation_seconds) as response:
                envelope = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(f"{method}: {error.read().decode()}") from error
        assert "error" not in envelope, envelope
        elapsed = time.perf_counter() - start
        assert elapsed < args.max_operation_seconds, (method, elapsed)
        timings.append({"method": method, "seconds": round(elapsed, 3)})
        print(f"{method}: {elapsed:.3f}s", flush=True)
        return envelope["result"]

    report = {"fonttools": fontTools.__version__, "fonts": []}
    for index, source in enumerate([fixture, *args.fonts]):
        source = source.resolve()
        directory = args.output / f"{index}-{source.stem}"
        directory.mkdir(exist_ok=True)
        start = len(timings)
        doc = call("session.open", path=str(source))["doc"]
        try:
            initial = call("color.state", doc=doc)
            assert initial["status"] == "Editable", initial.get("import_error")
            names = []
            while True:
                page = call("glyphs.list", doc=doc, master=None, sort="Order", offset=len(names), limit=1024,
                            include_paths=False, filter={"query": None, "categories": [], "scripts": [],
                                "blocks": [], "unencoded_only": False, "color_labels": [], "tags": [],
                                "unicode_ranges": [], "has_components": None, "changed_only": False})
                assert page["items"], "glyph paging made no progress"
                names.extend(g["name"] for g in page["items"])
                if len(names) == page["total"]:
                    break
            table = initial["color"]
            with TTFont(source) as expected:
                assert model_snapshot(table, names) == ft_snapshot(expected), "import differs from FontTools"
                for format_, extension in [("Ttf", "ttf"), ("Otf", "otf"), ("Woff", "woff"), ("Woff2", "woff2")]:
                    path = directory / f"baseline.{extension}"
                    call("session.export", doc=doc, path=str(path), options={"format": format_, "preserve_untouched_tables": format_ != "Otf"})
                    verify(expected, path)
                draft = copy.deepcopy(table)
                color = {"red": 215, "green": 35, "blue": 105, "alpha": 117}
                draft["palettes"][0]["colors"][0] = color
                base = next(iter(draft["glyphs"]))
                draft["glyphs"][base].reverse()
                draft["glyphs"][base][0]["palette_index"] = None
                call("color.apply", doc=doc, expected_generation=initial["generation"], color=draft, label="Color acceptance")
                edited = call("color.state", doc=doc)
                assert edited["color"] == draft
                native = directory / "edited.fcf"
                call("session.save", doc=doc, path=str(native))
                reopened = call("session.open", path=str(native))["doc"]
                try:
                    assert call("color.state", doc=reopened)["color"] == draft
                finally:
                    call("session.close", doc=reopened)
                call("doc.undo", doc=doc)
                assert call("color.state", doc=doc)["color"] == table
                call("doc.redo", doc=doc)
                assert call("color.state", doc=doc)["color"] == draft
                # Apply expected edits with FontTools, not FontCarver's codec.
                expected["CPAL"].palettes[0][0] = type(expected["CPAL"].palettes[0][0])(105, 35, 215, 117)
                original_base = expected.getGlyphOrder()[names.index(base)]
                expected["COLR"].ColorLayers[original_base].reverse()
                expected["COLR"].ColorLayers[original_base][0].colorID = 0xFFFF
                for format_, extension in [("Ttf", "ttf"), ("Otf", "otf"), ("Woff", "woff"), ("Woff2", "woff2")]:
                    path = directory / f"edited.{extension}"
                    call("session.export", doc=doc, path=str(path), options={"format": format_, "preserve_untouched_tables": format_ != "Otf"})
                    verify(expected, path)
                preview = call("color.preview", doc=doc, glyph=base, layers=None, palette=None, palette_index=0)
                assert [l["glyph"] for l in preview["layers"]] == [l["glyph"] for l in draft["glyphs"][base]]
                assert preview["layers"][0]["color"] is None
                assert call("color.state", doc=doc)["color"] == draft
                summary = {"source": str(source), "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                           "glyphs": len(names), "color_glyphs": len(table["glyphs"]),
                           "layers": sum(map(len, table["glyphs"].values())),
                           "palettes": len(table["palettes"]), "palette_entries": len(table["palettes"][0]["colors"]),
                           "timings": timings[start:]}
                report["fonts"].append(summary)
                print(json.dumps(summary), flush=True)
        finally:
            call("session.close", doc=doc)
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"COLR/CPAL verified with FontTools: {len(report['fonts'])} fonts")


if __name__ == "__main__":
    main()
