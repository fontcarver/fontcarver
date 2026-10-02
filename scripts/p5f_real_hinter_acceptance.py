#!/usr/bin/env python3
"""Require real ttfautohint and verify FontCarver exports independently.

Uses an existing CLI, FontTools, freetype-py and uharfbuzz. Every generated font
and configuration stays inside the required output directory. No binary is
installed or bundled by this script.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

import freetype
from fontTools import subset
from fontTools.ttLib import TTFont
import uharfbuzz as hb


def invoke(cli: Path, env: dict[str, str], *args: str | Path, success: bool = True) -> str:
    result = subprocess.run(
        [str(cli), *map(str, args)], capture_output=True, text=True,
        timeout=150, env=env, check=False,
    )
    if success:
        assert result.returncode == 0, result.stderr
        assert "autohint-unavailable" not in result.stderr, result.stderr
    else:
        assert result.returncode != 0, "invalid input unexpectedly succeeded"
    return result.stderr


def programs(font: TTFont) -> int:
    return sum(
        bool(getattr(font["glyf"][name], "program", None)
             and font["glyf"][name].program.getBytecode())
        for name in font.getGlyphOrder()
    )


def shape(path: Path, text: str) -> list[tuple[int, ...]]:
    face = hb.Face(path.read_bytes())
    font = hb.Font(face)
    font.scale = (face.upem, face.upem)
    buffer = hb.Buffer()
    buffer.add_str(text)
    buffer.guess_segment_properties()
    hb.shape(font, buffer)
    return [
        (info.codepoint, info.cluster, pos.x_advance, pos.y_advance,
         pos.x_offset, pos.y_offset)
        for info, pos in zip(buffer.glyph_infos, buffer.glyph_positions)
    ]


def validate(path: Path, plain: Path, mode: str) -> dict:
    with TTFont(path) as font, TTFont(plain) as original:
        names = original.getGlyphOrder()
        assert font.getGlyphOrder()[:len(names)] == names
        if mode == "composites":
            assert font.getGlyphOrder()[len(names):] == [".ttfautohint"]
            helper = font["glyf"][".ttfautohint"]
            assert list(helper.coordinates) == [(0, 0)]
            assert ".ttfautohint" not in font.getBestCmap().values()
            assert "TTFA" in font
        else:
            assert font.getGlyphOrder() == names
        assert font.getBestCmap() == original.getBestCmap()
        assert {n: font["hmtx"][n] for n in names} == original["hmtx"].metrics
        count = programs(font)
        if mode == "dehint":
            assert count == 0
            assert not {"cvt ", "prep", "fpgm"} & set(font.keys())
        else:
            assert count > 50, count
            assert {"cvt ", "prep", "fpgm"} <= set(font.keys())
        if mode == "default":
            for name in names:
                assert original["glyf"][name].getCoordinates(original["glyf"]) == \
                    font["glyf"][name].getCoordinates(font["glyf"]), name
        # HarfBuzz and FreeType consume sfnt, independent of host Brotli support.
        sfnt = path.with_suffix(path.suffix + ".decoded.ttf")
        font.flavor = None
        font.save(sfnt)
    texts = ["AVATAR", "office ffi", "ÀÁÄéâ café"]
    for text in texts:
        assert shape(sfnt, text) == shape(plain, text), (path, text)
    face = freetype.Face(str(sfnt))
    digest = hashlib.sha256()
    renders = 0
    for ppem in [9, 12, 16, 24, 36, 48]:
        face.set_pixel_sizes(0, ppem)
        # Never render the internal helper itself: upstream documents that doing
        # so changes hinting state. It is intentionally absent from cmap.
        for gid in range(len(names)):
            face.load_glyph(gid, freetype.FT_LOAD_RENDER | freetype.FT_LOAD_NO_AUTOHINT)
            digest.update(bytes(face.glyph.bitmap.buffer))
            renders += 1
    return {"glyphs": face.num_glyphs, "programs": count, "renders": renders,
            "shape_samples": len(texts), "raster_sha256": digest.hexdigest()}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=Path("target/debug/fontcarver"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = shutil.which("ttfautohint")
    if binary is None:
        parser.error("real ttfautohint must be on PATH; this acceptance never skips")
    version = subprocess.run([binary, "--version"], capture_output=True,
                             text=True, check=True, timeout=10).stdout.splitlines()[0]
    assert version.startswith("ttfautohint "), version
    root = args.output.resolve()
    root.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, FC_CONFIG_DIR=str(root / "config"))
    cli = args.cli.resolve()
    source = root / "NotoSans-latin.ttf"
    with TTFont("fixtures/fonts/NotoSans-Regular.ttf") as font:
        options = subset.Options()
        options.hinting = False
        subsetter = subset.Subsetter(options)
        subsetter.populate(unicodes=[*range(32, 127), *map(ord, "ÀÁÄéâ")])
        subsetter.subset(font)
        font.save(source)
    source_hash = hashlib.sha256(source.read_bytes()).hexdigest()
    plain = root / "plain.ttf"
    invoke(cli, env, "build", source, plain, "--no-preserve")
    results = {"executable": binary, "version": version, "outputs": {}}
    cases = [(f"default.{ext}", "default", []) for ext in ["ttf", "woff", "woff2"]]
    cases += [
        ("composites.ttf", "composites", [
            "--hint-range-min", "8", "--hint-range-max", "50", "--hint-limit", "200",
            "--hint-fallback-stem-width", "70", "--hint-increase-x-height", "0",
            "--hint-x-height-snapping-exceptions", "10,14-16", "--hint-default-script", "latn",
            "--hint-fallback-script", "none", "--hint-strong-stem-width", "gGD",
            "--hint-composites", "--hint-symbol",
            "--hint-ttfa-table", "--hint-windows-compatibility", "--hint-info", "detailed",
        ]),
        ("adjusted.ttf", "adjusted", ["--hint-adjust-subglyphs", "--hint-composites"]),
        ("dehint.ttf", "dehint", ["--hint-dehint"]),
    ]
    for filename, mode, flags in cases:
        output = root / filename
        invoke(cli, env, "build", source, output, "--autohint", "--no-preserve", *flags)
        results["outputs"][filename] = validate(output, plain, mode)
    existing = root / "keep.ttf"
    existing.write_bytes(b"existing destination must survive")
    malformed = root / "malformed.ttf"
    malformed.write_bytes(b"not a font")
    failures = [(source, ["--hint-range-min", "1"]),
                (source, ["--hint-default-script", "zzzz"]), (malformed, [])]
    for invalid_source, flags in failures:
        invoke(cli, env, "build", invalid_source, existing, "--autohint", *flags, success=False)
        assert existing.read_bytes() == b"existing destination must survive"
    assert hashlib.sha256(source.read_bytes()).hexdigest() == source_hash
    results["invalid_inputs_rejected_with_destination_preserved"] = len(failures)
    results["input_unchanged"] = True
    (root / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
