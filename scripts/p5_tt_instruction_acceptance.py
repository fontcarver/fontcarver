#!/usr/bin/env python3
"""Check the Rust TT instruction writer using FontTools and native FreeType.

Run the fc-sfnt tt_instruction_acceptance example first. This script reads its
output directory and compares it with the same original static TTF fixture.
Requires fonttools and freetype-py; no reference application is executed.
"""

from __future__ import annotations

import argparse
import struct
from collections import defaultdict
from pathlib import Path

import freetype
from fontTools.ttLib import TTFont


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    original = TTFont(args.source, checkChecksums=2)
    output_path = args.directory / "instructed.ttf"
    output = TTFont(output_path, checkChecksums=2)
    # The optional WOFF2 example uses Google's independent decoder as well as
    # FontTools. Compare raw table bytes so zero-contour instructions are covered.
    for name in ("instructed.woff2", "instructed.decoded.ttf"):
        candidate = args.directory / name
        if candidate.exists():
            wrapped = TTFont(candidate, checkChecksums=2)
            assert set(output.reader.keys()) - {"DSIG"} == set(wrapped.reader.keys())
            for tag in wrapped.reader.keys():
                before, after = output.reader[tag], wrapped.reader[tag]
                if tag == "head":
                    before = before[:8] + bytes(4) + before[12:]
                    after = after[:8] + bytes(4) + after[12:]
                assert before == after, (name, tag)
            print(f"Verified every WOFF2 table and instruction byte: {name}")
    order = original.getGlyphOrder()
    assert output.getGlyphOrder() == order
    programs = {
        int(gid): bytes.fromhex(data)
        for gid, data in (
            line.split("\t")
            for line in (args.directory / "programs.tsv").read_text().splitlines()
        )
    }
    for gid, name in enumerate(order):
        before = original["glyf"][name]
        after = output["glyf"][name]
        assert before.getCoordinates(original["glyf"]) == after.getCoordinates(
            output["glyf"]
        ), name
        actual = (
            bytes(after.program.getBytecode()) if hasattr(after, "program") else b""
        )
        if after.numberOfContours == 0:
            # FontTools 4.63 short-circuits zero-contour glyph parsing. Read the
            # legal phantom-only instruction span through its raw table reader.
            offsets = output["loca"].locations
            raw = output.reader["glyf"][offsets[gid] : offsets[gid + 1]]
            if len(raw) >= 12:
                length = struct.unpack_from(">H", raw, 10)[0]
                actual = raw[12 : 12 + length]
                assert len(actual) == length
        assert actual == programs.get(gid, b""), name
    assert list(output["cvt "].values) == [40, -20, 90]
    assert bytes(output["fpgm"].program.getBytecode()) == bytes(
        [0xB0, 0, 0x2C, 0, 0x2D]
    )
    assert bytes(output["prep"].program.getBytecode()) == bytes([0xB0, 0, 0x2B])
    for field, value in {
        "maxZones": 2,
        "maxTwilightPoints": 2,
        "maxStorage": 8,
        "maxFunctionDefs": 1,
        "maxInstructionDefs": 0,
        "maxStackElements": 32,
        "maxSizeOfInstructions": 10,
    }.items():
        assert getattr(output["maxp"], field) == value, field
    samples = defaultdict(list)
    for line in (args.directory / "points.tsv").read_text().splitlines():
        gid, index, x, y, on_curve, contour = line.split("\t")
        assert int(index) == len(samples[int(gid)])
        samples[int(gid)].append((float(x), float(y), int(on_curve), int(contour)))
    for gid, expected in samples.items():
        coordinates, ends, flags = original["glyf"][order[gid]].getCoordinates(
            original["glyf"]
        )
        contour = 0
        actual = []
        for index, ((x, y), flag) in enumerate(zip(coordinates, flags)):
            while index > ends[contour]:
                contour += 1
            actual.append((x, y, flag & 1, contour))
        assert actual == expected, order[gid]
    face = freetype.Face(str(output_path))
    control = freetype.Face(str(args.directory / "control.ttf"))
    renders = differences = 0
    affected_glyphs = set()
    for ppem in [9, 12, 16, 24, 36, 48]:
        face.set_pixel_sizes(0, ppem)
        control.set_pixel_sizes(0, ppem)
        for gid in programs:
            # Monochrome target enables full horizontal grid fitting. Modern
            # subpixel compatibility modes deliberately suppress some x moves.
            base = (
                freetype.FT_LOAD_NO_AUTOHINT
                | freetype.FT_LOAD_PEDANTIC
                | freetype.FT_LOAD_TARGET_MONO
            )
            face.load_glyph(gid, base)
            hinted = (list(face.glyph.outline.points), face.glyph.advance.x)
            face.load_glyph(gid, base | freetype.FT_LOAD_NO_HINTING)
            unhinted = (list(face.glyph.outline.points), face.glyph.advance.x)
            differences += hinted != unhinted
            control.load_glyph(gid, base)
            without_program = (
                list(control.glyph.outline.points),
                control.glyph.advance.x,
            )
            if hinted != without_program:
                affected_glyphs.add(gid)
            for flags in [base, base | freetype.FT_LOAD_NO_HINTING]:
                face.load_glyph(gid, flags | freetype.FT_LOAD_RENDER)
                renders += 1
    assert differences > 0, "programs did not affect any outline or advance"
    # FreeType 2.13.2 and 2.14.3 skip all zero-contour programs. Their exact bytes
    # are checked above; the Rust acceptance test checks execution with Skrifa.
    visible_glyphs = {
        gid for gid in programs if original["glyf"][order[gid]].numberOfContours != 0
    }
    assert visible_glyphs <= affected_glyphs, "an outline program had no native effect"
    print(
        f"Verified {len(order)} glyph outlines, {len(programs)} exact programs, "
        f"{len(samples)} point arrays, {renders} native renders, "
        f"{differences} changed hinted outlines/advances; FreeType {freetype.version()}, "
        f"{len(set(programs) - affected_glyphs)} phantom-only programs not executed by this renderer"
    )


if __name__ == "__main__":
    main()
