#!/usr/bin/env python3
"""Rebuild the small, permissively licensed binary font fixtures."""

from __future__ import annotations

import hashlib
import tempfile
import urllib.request
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont


FIXTURES = Path(__file__).resolve().parent
FONTS = FIXTURES / "fonts"
SOURCE_SANS_URL = (
    "https://raw.githubusercontent.com/adobe-fonts/source-sans/3.052R/"
    "OTF/SourceSans3-Regular.otf"
)
SOURCE_SERIF_URL = (
    "https://raw.githubusercontent.com/adobe-fonts/source-serif/4.005R/"
    "OTF/SourceSerif4-Regular.otf"
)

COMMON_OPTIONS = [
    "--layout-features=*",
    "--no-layout-closure",
    "--glyph-names",
    "--name-IDs=*",
    "--name-legacy",
    "--name-languages=*",
    "--symbol-cmap",
    "--legacy-cmap",
    "--notdef-glyph",
    "--notdef-outline",
    "--no-recommended-glyphs",
    "--no-recalc-timestamp",
]


def download(url: str, destination: Path) -> None:
    request = urllib.request.Request(url, headers={"User-Agent": "Font Cutter fixtures"})
    with urllib.request.urlopen(request, timeout=60) as response:
        destination.write_bytes(response.read())


def run_subset(source: Path, destination: Path, *options: str) -> None:
    subset.main(
        [
            str(source),
            f"--output-file={destination}",
            *options,
            *COMMON_OPTIONS,
        ]
    )


def rename_ofl_subset(path: Path, family: str, postscript_name: str) -> None:
    """Remove Adobe's reserved name from the primary names of a modified font."""
    font = TTFont(path, recalcTimestamp=False)
    replacements = {
        1: family,
        2: "Regular",
        3: f"{postscript_name};fixture-v1",
        4: f"{family} Regular",
        6: postscript_name,
        16: family,
        17: "Regular",
        21: family,
        22: "Regular",
    }
    for record in font["name"].names:
        if record.nameID in replacements:
            record.string = replacements[record.nameID].encode(record.getEncoding())

    cff = font["CFF "].cff
    cff.fontNames[0] = postscript_name
    top_dict = cff.topDictIndex[0]
    top_dict.FamilyName = family
    top_dict.FullName = f"{family} Regular"
    font.save(path, reorderTables=False)
    font.close()


def name_values(font: TTFont, name_id: int) -> set[str]:
    return {
        record.toUnicode()
        for record in font["name"].names
        if record.nameID == name_id
    }


def validate() -> None:
    expected = {
        "DejaVuSansMin.woff2": (53, "woff2", "DejaVu Sans"),
        "SourceSans3Min.otf": (5, None, "FontWorkshop CFF Sans Fixture"),
        "SourceSerif4Min.otf": (2, None, "FontWorkshop CFF Serif Fixture"),
    }
    for filename, (glyph_count, flavor, family) in expected.items():
        path = FONTS / filename
        font = TTFont(path)
        assert len(font.getGlyphOrder()) == glyph_count
        assert font.flavor == flavor
        assert family in name_values(font, 1)
        if filename.endswith(".otf"):
            assert font.sfntVersion == "OTTO"
            assert "CFF " in font and "glyf" not in font
            assert any("Open Font License" in value for value in name_values(font, 13))
            assert path.stat().st_size < 300_000
            top_dict = font["CFF "].cff.topDictIndex[0]
            assert top_dict.Encoding == "StandardEncoding"
            if filename == "SourceSans3Min.otf":
                assert top_dict.Private.rawDict["StdHW"] == 73
                assert top_dict.Private.rawDict["StdVW"] == 84
                charstring = top_dict.CharStrings[".notdef"]
                charstring.decompile()
                assert "hstem" in charstring.program and "vstem" in charstring.program
            else:
                assert top_dict.Private.rawDict == {"nominalWidthX": 603}
        else:
            assert any("Bitstream" in value for value in name_values(font, 13))
        font.close()
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        print(f"{filename}: {path.stat().st_size} bytes, sha256 {digest}")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="fontcutter-fixtures-") as temp:
        temp_dir = Path(temp)
        source_sans = temp_dir / "SourceSans3-Regular.otf"
        source_serif = temp_dir / "SourceSerif4-Regular.otf"
        download(SOURCE_SANS_URL, source_sans)
        download(SOURCE_SERIF_URL, source_serif)

        run_subset(
            FONTS / "DejaVuSans.ttf",
            FONTS / "DejaVuSansMin.woff2",
            "--flavor=woff2",
            "--unicodes=U+0041-005A,U+0061-007A",
            "--hinting",
        )
        run_subset(
            source_sans,
            FONTS / "SourceSans3Min.otf",
            "--unicodes=U+0020,U+0041,U+0056,U+006F",
            "--hinting",
        )
        rename_ofl_subset(
            FONTS / "SourceSans3Min.otf",
            "FontWorkshop CFF Sans Fixture",
            "FontWorkshopCFFSansFixture-Regular",
        )

        # The second CFF stays unhinted to preserve the old Nimbus fixture's
        # distinct no-cff-warning coverage. Its Private DICT widths remain.
        run_subset(
            source_serif,
            FONTS / "SourceSerif4Min.otf",
            "--glyphs=.notdef,o",
            "--no-hinting",
        )
        rename_ofl_subset(
            FONTS / "SourceSerif4Min.otf",
            "FontWorkshop CFF Serif Fixture",
            "FontWorkshopCFFSerifFixture-Regular",
        )

    validate()


if __name__ == "__main__":
    main()
