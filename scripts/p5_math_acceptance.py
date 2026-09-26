#!/usr/bin/env python3
"""Independently verify MATH import, editing, persistence and export.

Requires FontTools and uharfbuzz plus a running fc-devserver. Supply real math
fonts by external path; no fonts or generated exports are added to the repo.
Example:
  python3 scripts/p5_math_acceptance.py --api http://127.0.0.1:8894 \
    --output /tmp/fontcarver-math fixtures/fonts/DejaVuSans.ttf \
    /usr/share/fonts/truetype/dejavu/DejaVuMathTeXGyre.ttf
"""

from __future__ import annotations

import argparse
import copy
import io
import json
import time
import urllib.error
import urllib.request
from pathlib import Path

import fontTools
import uharfbuzz as hb
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables import otTables


def semantic(value):
    """Compare decoded data without offset placement or device bit packing."""
    if isinstance(value, (str, int, float, type(None))):
        return value
    if isinstance(value, list):
        return [semantic(item) for item in value]
    return {
        key: semantic(item)
        for key, item in vars(value).items()
        if key not in {"Format", "DeltaFormat"} and not key.startswith("_")
    }


def harfbuzz_font(font):
    stream = io.BytesIO()
    font.flavor = None
    font.save(stream)
    result = hb.Font(hb.Face(stream.getvalue()))
    assert result.face.has_math_data
    return result


def engine_snapshot(font, order, names):
    gids = {name: gid for gid, name in enumerate(order)}
    records = {}
    for name in names:
        gid = gids[name]
        constructions = []
        for direction in ("btt", "ltr"):
            variants = [
                (order[v.glyph], v.advance)
                for v in font.get_math_glyph_variants(gid, direction)
            ]
            parts, italic = font.get_math_glyph_assembly(gid, direction)
            assembly = [
                (order[p.glyph], p.start_connector_length,
                 p.end_connector_length, p.full_advance, int(p.flags))
                for p in parts
            ]
            constructions.append((variants, assembly, italic))
        records[name] = (
            font.get_math_glyph_italics_correction(gid),
            font.get_math_glyph_top_accent_attachment(gid),
            font.face.is_glyph_extended_math_shape(gid),
            [font.get_math_glyph_kernings(gid, corner) for corner in hb.OTMathKern],
            constructions,
        )
    return (
        [font.get_math_constant(c) for c in hb.OTMathConstant],
        [font.get_math_min_connector_overlap(d) for d in ("btt", "ltr")],
        records,
    )


def verify(expected, path, names):
    with TTFont(path, checkChecksums=2) as actual:
        expected["MATH"].ensureDecompiled()
        actual["MATH"].ensureDecompiled()
        assert semantic(expected["MATH"].table) == semantic(actual["MATH"].table), path
        before, after = harfbuzz_font(expected), harfbuzz_font(actual)
        # Nonzero ppem exercises signed Device corrections independently.
        for ppem in (0, 12, 13, 14, 24):
            before.ppem = after.ppem = (ppem, ppem)
            before_data = engine_snapshot(before, expected.getGlyphOrder(), names)
            after_data = engine_snapshot(after, actual.getGlyphOrder(), names)
            assert before_data == after_data, (path, ppem)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fonts", type=Path, nargs="+")
    parser.add_argument("--api", default="http://127.0.0.1:8894")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--max-operation-seconds", type=float, default=120)
    parser.add_argument("--server-pid", type=int)
    parser.add_argument("--max-rss-mib", type=int, default=1024)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    timings = []

    def call(method, **params):
        request = urllib.request.Request(
            f"{args.api}/api/{method}",
            data=json.dumps(params).encode(),
            headers={"Content-Type": "application/json"},
        )
        start = time.perf_counter()
        try:
            with urllib.request.urlopen(
                request, timeout=args.max_operation_seconds
            ) as response:
                envelope = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(f"{method}: {error.read().decode()}") from error
        elapsed = time.perf_counter() - start
        assert "error" not in envelope, envelope
        assert elapsed < args.max_operation_seconds, (method, elapsed)
        timings.append({"method": method, "seconds": round(elapsed, 3)})
        print(f"{method}: {elapsed:.3f}s", flush=True)
        return envelope["result"]

    report = {"fonttools": fontTools.__version__, "uharfbuzz": hb.__version__,
              "harfbuzz": hb.version_string(), "fonts": []}
    for index, source in enumerate(args.fonts):
        source = source.resolve()
        directory = args.output / f"{index}-{source.stem}"
        directory.mkdir(exist_ok=True)
        start_index = len(timings)
        doc = call("session.open", path=str(source))["doc"]
        try:
            initial = call("math.state", doc=doc)
            assert initial["status"] == "Editable", (
                initial["status"], initial["import_error"]
            )
            table = initial["math"]
            names = sorted(table["glyphs"])
            with TTFont(source) as expected:
                baseline = directory / "baseline.ttf"
                call("session.export", doc=doc, path=str(baseline),
                     options={"format": "Ttf"})
                verify(expected, baseline, names)

                constants = copy.deepcopy(table["constants"])
                constants["axis_height"] = {
                    "value": 777, "device": {"start_size": 12, "deltas": [-2, 0, 3]},
                }
                glyph, info = next(
                    (name, copy.deepcopy(info))
                    for name, info in table["glyphs"].items()
                    if info["vertical"] and info["vertical"]["assembly"]
                )
                info["vertical"]["assembly"]["italic_correction"]["value"] = 17
                call("math.apply", doc=doc, expected_generation=initial["generation"],
                     label="MATH independent acceptance", commands=[
                         {"SetGlobals": {
                             "constants": constants,
                             "min_connector_overlap": table["min_connector_overlap"],
                         }},
                         {"SetGlyph": {"glyph": glyph, "info": info}},
                     ])
                edited = call("math.state", doc=doc)
                assert (edited["math"]["constants"]["axis_height"]
                        == constants["axis_height"])
                preview = call("math.preview", doc=doc, glyph=glyph, vertical=True,
                               target=expected["head"].unitsPerEm * 8,
                               construction=None)
                assert preview["assembly"] and preview["assembly"]["parts"], preview
                assert call("math.state", doc=doc)["math"] == edited["math"]

                native = directory / "edited.fcf"
                call("session.save", doc=doc, path=str(native))
                reopened = call("session.open", path=str(native))["doc"]
                try:
                    assert call("math.state", doc=reopened)["math"] == edited["math"]
                finally:
                    call("session.close", doc=reopened)
                call("doc.undo", doc=doc)
                assert call("math.state", doc=doc)["math"] == table
                call("doc.redo", doc=doc)
                assert call("math.state", doc=doc)["math"] == edited["math"]

                # Build the expected edits with FontTools, independently of our codec.
                axis = expected["MATH"].table.MathConstants.AxisHeight
                axis.Value = 777
                axis.DeviceTable = otTables.Device()
                axis.DeviceTable.StartSize = 12
                axis.DeviceTable.EndSize = 14
                axis.DeviceTable.DeltaFormat = 2
                axis.DeviceTable.DeltaValue = [-2, 0, 3]
                variants = expected["MATH"].table.MathVariants
                position = variants.VertGlyphCoverage.glyphs.index(glyph)
                construction = variants.VertGlyphConstruction[position]
                construction.GlyphAssembly.ItalicsCorrection.Value = 17
                for format_, extension in (
                    ("Ttf", "ttf"), ("Otf", "otf"),
                    ("Woff", "woff"), ("Woff2", "woff2"),
                ):
                    output = directory / f"edited.{extension}"
                    # DejaVu's GPOS point anchors cannot survive quadratic-to-CFF
                    # conversion. Explicitly omit raw tables for that export;
                    # the owned MATH table must still be compiled and compared.
                    call("session.export", doc=doc, path=str(output), options={
                        "format": format_,
                        "preserve_untouched_tables": format_ != "Otf",
                    })
                    verify(expected, output, names)
                summary = {
                    "source": str(source), "glyphs": len(expected.getGlyphOrder()),
                    "math_records": len(names), "timings": timings[start_index:],
                }
                report["fonts"].append(summary)
                print(json.dumps(summary), flush=True)
        finally:
            call("session.close", doc=doc)
    if args.server_pid:
        status = Path(f"/proc/{args.server_pid}/status").read_text().splitlines()
        report["server_memory"] = [
            s for s in status if s.startswith(("VmRSS:", "VmHWM:"))
        ]
        peak_kib = int(next(s for s in status if s.startswith("VmHWM:")).split()[1])
        assert peak_kib <= args.max_rss_mib * 1024, report["server_memory"]
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"MATH verified with FontTools and HarfBuzz: {len(report['fonts'])} fonts")


if __name__ == "__main__":
    main()
