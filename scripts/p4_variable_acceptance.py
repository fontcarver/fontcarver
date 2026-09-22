#!/usr/bin/env python3
"""Build real-outline master fixtures and independently sample CLI variable exports.

Inputs stay outside the repository. Requires fontTools and a built fontcarver CLI.
Generated fixtures have distinct family names and retain upstream license metadata.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from types import SimpleNamespace

from fontTools.designspaceLib import (
    AxisDescriptor,
    DesignSpaceDocument,
    InstanceDescriptor,
    SourceDescriptor,
)
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.pointPen import SegmentToPointPen
from fontTools.pens.recordingPen import DecomposingRecordingPen, RecordingPen
from fontTools.ttLib import TTFont
from fontTools.ufoLib import UFOWriter


def outline(font: TTFont, name: str) -> list:
    glyphs = font.getGlyphSet()
    pen = DecomposingRecordingPen(glyphs)
    glyphs[name].draw(pen)
    return pen.value


def signature(value: list) -> list:
    return [
        (op, len(points), tuple(point is None for point in points))
        for op, points in value
    ]


def record(value: list) -> RecordingPen:
    pen = RecordingPen()
    pen.value = value
    return pen


def blend(left: list, right: list, factor: float) -> list:
    return [
        (
            op,
            tuple(
                None if a is None else tuple(x + (y - x) * factor for x, y in zip(a, b))
                for a, b in zip(points, other)
            ),
        )
        for (op, points), (_, other) in zip(left, right)
    ]


def bounds(value: list) -> tuple | None:
    pen = BoundsPen(None)
    record(value).replay(pen)
    return pen.bounds


def make_sources(
    root: Path, family: str, paths: list[Path], candidates: list[int], limit: int
) -> tuple[Path, list, list]:
    fonts = [TTFont(path) for path in paths]
    cmaps = [font.getBestCmap() for font in fonts]
    chosen = []
    masters = [[], []]
    for cp in dict.fromkeys(candidates):
        if not all(cp in cmap for cmap in cmaps):
            continue
        values = [outline(font, cmap[cp]) for font, cmap in zip(fonts, cmaps)]
        if signature(values[0]) != signature(values[1]):
            continue
        name = f"uni{cp:04X}"
        chosen.append((name, cp))
        for index, (font, cmap, value) in enumerate(zip(fonts, cmaps, values)):
            masters[index].append((name, value, font["hmtx"][cmap[cp]][0]))
        if len(chosen) == limit:
            break
    if len(chosen) < min(40, limit):
        raise RuntimeError(f"Too few compatible real glyphs: {family}: {len(chosen)}")
    document = DesignSpaceDocument()
    axis = AxisDescriptor()
    axis.name, axis.tag = "Weight", "wght"
    axis.minimum, axis.default, axis.maximum = 400, 400, 700
    document.addAxis(axis)
    for index, (font, glyphs) in enumerate(zip(fonts, masters)):
        path = root / f"{family}-{index}.ufo"
        writer = UFOWriter(path, formatVersion=3)
        info = SimpleNamespace(
            familyName=family,
            styleName=["Regular", "Bold"][index],
            unitsPerEm=font["head"].unitsPerEm,
            ascender=font["hhea"].ascent,
            descender=font["hhea"].descent,
            xHeight=getattr(font["OS/2"], "sxHeight", 500),
            capHeight=getattr(font["OS/2"], "sCapHeight", 700),
            copyright=font["name"].getDebugName(0),
            openTypeNameLicense=font["name"].getDebugName(13),
            openTypeNameLicenseURL=font["name"].getDebugName(14),
            openTypeOS2WeightClass=[400, 700][index],
        )
        writer.writeInfo(info)
        glyph_set = writer.getGlyphSet()
        for (name, cp), (_, value, width) in zip(chosen, glyphs):
            glyph = SimpleNamespace(
                width=width, height=1000 + index * 100, unicodes=[cp]
            )

            def draw_points(pen, value=value):
                record(value).replay(SegmentToPointPen(pen))

            glyph_set.writeGlyph(name, glyph, draw_points)
        glyph_set.writeContents()
        writer.writeLayerContents()
        writer.writeLib({"public.glyphOrder": [name for name, _ in chosen]})
        writer.close()
        source = SourceDescriptor()
        source.path, source.name, source.familyName = str(path), info.styleName, family
        source.styleName, source.location = (
            info.styleName,
            {"Weight": [400, 700][index]},
        )
        document.addSource(source)
    instance = InstanceDescriptor()
    instance.name, instance.familyName, instance.styleName = "Medium", family, "Medium"
    instance.location = {"Weight": 550}
    document.addInstance(instance)
    path = root / f"{family}.designspace"
    document.write(path)
    inputs = [
        {
            "path": str(path),
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "license": font["name"].getDebugName(13),
        }
        for path, font in zip(paths, fonts)
    ]
    return path, masters, inputs


def verify(path: Path, masters: list) -> dict:
    font = TTFont(path)
    assert [
        (axis.axisTag, axis.minValue, axis.defaultValue, axis.maxValue)
        for axis in font["fvar"].axes
    ] == [("wght", 400, 400, 700)]
    assert font["fvar"].instances[0].coordinates == {"wght": 550}
    assert ("gvar" in font) != ("CFF2" in font)
    assert all(tag in font for tag in ["HVAR", "VVAR", "vhea", "vmtx"])
    assert len(font.getBestCmap()) == len(masters[0])
    maximum_error = 0.0
    for weight in [400, 475, 550, 625, 700]:
        factor = (weight - 400) / 300
        glyphs = font.getGlyphSet(location={"wght": weight})
        cmap = font.getBestCmap()
        for (name, left, width0), (_, right, width1) in zip(*masters):
            pen = BoundsPen(glyphs)
            actual_name = cmap[int(name[3:], 16)]
            glyphs[actual_name].draw(pen)
            expected = bounds(blend(left, right, factor))
            if expected is None:
                assert pen.bounds is None
            else:
                error = max(abs(a - b) for a, b in zip(pen.bounds, expected))
                maximum_error = max(maximum_error, error)
                assert error < 1.6, (path, name, weight, pen.bounds, expected)
            assert (
                abs(glyphs[actual_name].width - (width0 + (width1 - width0) * factor))
                < 1.1
            )
    return {
        "path": str(path),
        "bytes": path.stat().st_size,
        "glyphs": len(masters[0]),
        "locations": [400, 475, 550, 625, 700],
        "maximum_bounds_error": maximum_error,
        "tables": sorted(font.keys()),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cli", type=Path, default=Path("target/debug/fontcarver"))
    parser.add_argument(
        "--latin-regular",
        type=Path,
        default=Path("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
    )
    parser.add_argument(
        "--latin-bold",
        type=Path,
        default=Path("/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf"),
    )
    parser.add_argument("--cjk-regular", type=Path, required=True)
    parser.add_argument("--cjk-bold", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = []
    for family, paths, candidates, limit in [
        (
            "FontCarverP4Latin",
            [args.latin_regular, args.latin_bold],
            list(range(32, 127)),
            95,
        ),
        (
            "FontCarverP4CJK",
            [args.cjk_regular, args.cjk_bold],
            list(dict.fromkeys(map(ord, "水汉字中文测试日本語永一二三木火土山人")))
            + list(range(0x4E00, 0x5100)),
            256,
        ),
    ]:
        designspace, masters, inputs = make_sources(
            args.output, family, paths, candidates, limit
        )
        outputs = []
        for extension in ["ttf", "otf"]:
            output = args.output / f"{family}-Variable.{extension}"
            subprocess.run(
                [
                    str(args.cli.resolve()),
                    "build",
                    str(designspace),
                    str(output),
                    "--variable",
                    "--no-preserve",
                ],
                check=True,
            )
            outputs.append(verify(output, masters))
        static = args.output / f"{family}-Medium.ttf"
        subprocess.run(
            [
                str(args.cli.resolve()),
                "build",
                str(designspace),
                str(static),
                "--instance",
                "0",
                "--no-preserve",
            ],
            check=True,
        )
        font = TTFont(static)
        assert "fvar" not in font and font["OS/2"].usWeightClass == 550
        report.append(
            {
                "family": family,
                "inputs": inputs,
                "sources": str(designspace),
                "outputs": outputs,
                "static_instance": str(static),
            }
        )
    (args.output / "report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    )
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
