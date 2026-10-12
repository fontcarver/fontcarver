#!/usr/bin/env python3
"""Independent static COLRv1 acceptance using original FontTools-built artwork.

Run against an isolated fc-devserver. No third-party font data is downloaded.
Covers every static paint format, all 28 composite modes, unsorted stops,
shared paints, clips, CPAL v1, v0 fallbacks, native persistence and four exports.
"""
from __future__ import annotations
import argparse
import base64
import copy
import json
import time
import urllib.request
from pathlib import Path
import fontTools
from fontTools.colorLib.builder import buildCOLR
from fontTools.ttLib import TTFont
from p5_color_acceptance import synthetic as base_font


def synthetic(path):
    base_font(path)
    font = TTFont(path)
    solid = {"Format": 2, "PaletteIndex": 0, "Alpha": 0.5}
    line = {"Extend": 2, "ColorStop": [
        {"StopOffset": 1, "PaletteIndex": 1, "Alpha": 1},
        {"StopOffset": 0, "PaletteIndex": 0, "Alpha": 0.5},
        {"StopOffset": 1, "PaletteIndex": 0xFFFF, "Alpha": 1}]}
    linear = {"Format": 4, "ColorLine": line, "x0": 0, "y0": 0, "x1": 600, "y1": 0, "x2": 0, "y2": 700}
    radial = {"Format": 6, "ColorLine": line, "x0": 300, "y0": 350, "r0": 0, "x1": 100, "y1": 100, "r1": 600}
    sweep = {"Format": 8, "ColorLine": line, "centerX": 300, "centerY": 350, "startAngle": 0, "endAngle": 360}
    painted = {"Format": 10, "Glyph": "triangle", "Paint": linear}
    paints = [solid, linear, radial, sweep, painted]
    for format_, fields in [
        (12, {"Transform": (1, 0.25, -0.5, 1, 30, -50)}),
        (14, {"dx": -30, "dy": 50}),
        (16, {"scaleX": -1, "scaleY": 0.5}),
        (18, {"scaleX": 0.5, "scaleY": 1, "centerX": 300, "centerY": 350}),
        (20, {"scale": 0.5}),
        (22, {"scale": 0.5, "centerX": 300, "centerY": 350}),
        (24, {"angle": 45}), (26, {"angle": 45, "centerX": 300, "centerY": 350}),
        (28, {"xSkewAngle": 22.5, "ySkewAngle": -45}),
        (30, {"xSkewAngle": 22.5, "ySkewAngle": -45, "centerX": 300, "centerY": 350}),
    ]:
        paints.append({"Format": format_, "Paint": painted, **fields})
    for mode in range(28):
        paints.append({"Format": 32, "SourcePaint": painted, "BackdropPaint": {"Format": 10, "Glyph": "bar", "Paint": radial}, "CompositeMode": mode})
    font["COLR"] = buildCOLR({
        "A": {"Format": 10, "Glyph": "square", "Paint": {"Format": 1, "Layers": paints}},
        "B": {"Format": 11, "Glyph": "A"},
        "space": [("bar", 2)]}, version=None, glyphMap=font.getReverseGlyphMap(),
        clipBoxes={"A": (-20, -50, 700, 800), "B": (-20, -50, 700, 800)})
    font.save(path)


def snapshot(font):
    table = font["COLR"].table
    def paint(p):
        f = int(p.Format)
        if f == 1:
            return ["Layers", [paint(c) for c in table.LayerList.Paint[p.FirstLayerIndex:p.FirstLayerIndex+p.NumLayers]]]
        if f == 2: return ["Solid", p.PaletteIndex, p.Alpha]
        if f in (4, 6, 8):
            fields = {4: ["x0", "y0", "x1", "y1", "x2", "y2"], 6: ["x0", "y0", "r0", "x1", "y1", "r1"], 8: ["centerX", "centerY", "startAngle", "endAngle"]}[f]
            return [f, int(p.ColorLine.Extend), [(s.StopOffset, s.PaletteIndex, s.Alpha) for s in p.ColorLine.ColorStop], [getattr(p, k) for k in fields]]
        if f == 10: return ["Glyph", font.getGlyphID(p.Glyph), paint(p.Paint)]
        if f == 11: return ["ColrGlyph", font.getGlyphID(p.Glyph)]
        if f in (12, 14, 16, 18, 20, 22, 24, 26, 28, 30):
            return ["Transform", [round(v, 8) for v in p.getTransform()], paint(p.Paint)]
        if f == 32: return ["Composite", int(p.CompositeMode), paint(p.SourcePaint), paint(p.BackdropPaint)]
        raise AssertionError(f)
    cpal = font["CPAL"]
    layers = table.LayerRecordArray.LayerRecord if table.LayerRecordArray else []
    bases = table.BaseGlyphRecordArray.BaseGlyphRecord if table.BaseGlyphRecordArray else []
    clips = table.ClipList.clips if table.ClipList else {}
    return {
        "roots": sorted((font.getGlyphID(r.BaseGlyph), paint(r.Paint)) for r in table.BaseGlyphList.BaseGlyphPaintRecord),
        "v0": [(font.getGlyphID(r.BaseGlyph), [(font.getGlyphID(l.LayerGlyph), l.PaletteIndex) for l in layers[r.FirstLayerIndex:r.FirstLayerIndex+r.NumLayers]]) for r in bases],
        "clips": sorted((font.getGlyphID(n), (c.xMin, c.yMin, c.xMax, c.yMax)) for n, c in clips.items()),
        "palettes": [[[c.red, c.green, c.blue, c.alpha] for c in p] for p in cpal.palettes],
        "metadata": [cpal.version, cpal.paletteTypes, cpal.paletteLabels, cpal.paletteEntryLabels],
    }


def render_script(output):
    """Emit a browser check: original/edited FontTools vs each exported container."""
    groups = []
    for phase, source in [("baseline", "colrv1-acceptance.ttf"), ("edited", "edited-expected.ttf")]:
        paths = [source] + [f"{phase}.{ext}" for ext in ("ttf", "otf", "woff", "woff2")]
        groups.append([{"name": name, "data": base64.b64encode((output/name).read_bytes()).decode()} for name in paths])
    code = "const groups = " + json.dumps(groups) + ";\n" + r'''
const render = async (item, id) => {
  const bytes = Uint8Array.from(atob(item.data), c => c.charCodeAt(0));
  const face = new FontFace('fcColrv1Check' + id, bytes);
  await face.load(); document.fonts.add(face);
  try {
    const canvas = document.createElement('canvas'); canvas.width=480; canvas.height=300;
    const ctx=canvas.getContext('2d'); ctx.font='220px fcColrv1Check' + id;
    ctx.fillStyle='#202020'; ctx.fillText('AB', 10, 230);
    return ctx.getImageData(0,0,480,300).data;
  } finally { document.fonts.delete(face); }
};
if(!CSS.supports('font-tech(color-COLRv1)')) throw new Error('COLRv1 renderer required');
const results=[];
for(let group=0; group<groups.length; group++) {
  const reference=await render(groups[group][0], group*10);
  for(let i=1; i<groups[group].length; i++) {
    const pixels=await render(groups[group][i], group*10+i);
    let different=0, ink=0;
    for(let j=0;j<pixels.length;j++) {
      if(pixels[j]!==reference[j]) different++;
      if(j%4===3 && pixels[j]) ink++;
    }
    results.push({file:groups[group][i].name, differingChannels:different, inkPixels:ink});
  }
}
return results;
'''
    (output/"render-check.js").write_text("(async () => {\n"+code+"\n})()\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api", default="http://127.0.0.1:8895")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output = args.output.resolve(); args.output.mkdir(parents=True, exist_ok=True)
    source = args.output / "colrv1-acceptance.ttf"; synthetic(source)
    report = {"fonttools": fontTools.__version__, "complete": False, "exports": [], "timings": []}
    def call(method, **params):
        start = time.perf_counter()
        request = urllib.request.Request(f"{args.api}/api/{method}", data=json.dumps(params).encode(), headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(request, timeout=120) as response: result = json.load(response)
        assert "error" not in result, result
        report["timings"].append([method, round(time.perf_counter()-start, 3)])
        print(method, report["timings"][-1][1], flush=True)
        return result["result"]
    doc = call("session.open", path=str(source))["doc"]
    try:
        initial = call("color.state", doc=doc)
        assert initial["status"] == "Editable" and initial["color"]["graph"], initial
        expected = TTFont(source)
        for phase in ("baseline", "edited"):
            if phase == "edited":
                draft = copy.deepcopy(initial["color"])
                draft["palettes"][0]["colors"][0] = {"red": 160, "green": 35, "blue": 215, "alpha": 117}
                for p in draft["graph"]["paints"]:
                    if p["kind"] == "Solid": p["alpha"] = 0.25
                call("color.apply", doc=doc, expected_generation=initial["generation"], color=draft, label="Paint graph acceptance")
                expected["CPAL"].palettes[0][0] = type(expected["CPAL"].palettes[0][0])(215, 35, 160, 117)
                seen = set()
                def edit(p):
                    if id(p) in seen: return
                    seen.add(id(p))
                    if p.Format == 2: p.Alpha = 0.25
                    if p.Format == 1:
                        for child in expected["COLR"].table.LayerList.Paint[p.FirstLayerIndex:p.FirstLayerIndex+p.NumLayers]: edit(child)
                    for k in ("Paint", "SourcePaint", "BackdropPaint"):
                        if hasattr(p, k): edit(getattr(p, k))
                for root in expected["COLR"].table.BaseGlyphList.BaseGlyphPaintRecord: edit(root.Paint)
                for extension in ("fcf", "fcf.gz", "ufo"):
                    native = args.output / f"edited.{extension}"
                    call("session.save", doc=doc, path=str(native)) if extension != "ufo" else call("session.export", doc=doc, path=str(native), options={"format": "Ufo"})
                    reopened = call("session.open", path=str(native))["doc"]
                    try: assert call("color.state", doc=reopened)["color"] == draft
                    finally: call("session.close", doc=reopened)
                call("doc.undo", doc=doc); assert call("color.state", doc=doc)["color"] == initial["color"]
                call("doc.redo", doc=doc); assert call("color.state", doc=doc)["color"] == draft
                preview = call("color.graph_preview", doc=doc, glyph="A", color=draft, palette_index=1)
                (args.output / "preview.ttf").write_bytes(bytes(preview["bytes"]))
                assert call("color.state", doc=doc)["color"] == draft
            for format_ in ("Ttf", "Otf", "Woff", "Woff2"):
                path = args.output / f"{phase}.{format_.lower()}"
                call("session.export", doc=doc, path=str(path), options={"format": format_})
                with TTFont(path) as actual: assert snapshot(actual) == snapshot(expected), path
                report["exports"].append(path.name)
        expected.save(args.output / "edited-expected.ttf")
        render_script(args.output)
        report["render_script"] = "render-check.js (run with agent-browser eval --stdin)"
        report["complete"] = True
    finally:
        (args.output / "report.json").write_text(json.dumps(report, indent=2)+"\n")
        call("session.close", doc=doc)

if __name__ == "__main__": main()
