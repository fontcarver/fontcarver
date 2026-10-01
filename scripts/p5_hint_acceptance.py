#!/usr/bin/env python3
"""Independently decode and render hinting exports from the shared CLI.

Requires FontTools and freetype-py. Outputs and subset fixtures stay outside the
repository. An optional PATH-installed ttfautohint exercises the real bridge.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

import freetype
from fontTools import subset
from fontTools.ttLib import TTFont


def run(cli: Path, *args: str | Path) -> str:
    result = subprocess.run(
        [str(cli), *map(str, args)],
        capture_output=True,
        text=True,
        timeout=150,
        check=False,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stderr


def render(path: Path) -> tuple[int, str]:
    if path.suffix in {".woff", ".woff2"}:
        # Decode with FontTools: the host FreeType may lack Brotli support.
        font = TTFont(path)
        font.flavor = None
        path = path.with_suffix(path.suffix + ".decoded.ttf")
        font.save(path)
    face = freetype.Face(str(path))
    digest = hashlib.sha256()
    count = 0
    for ppem in [9, 12, 16, 24, 36, 48]:
        face.set_pixel_sizes(0, ppem)
        for gid in range(face.num_glyphs):
            face.load_glyph(gid, freetype.FT_LOAD_RENDER | freetype.FT_LOAD_NO_AUTOHINT)
            digest.update(bytes(face.glyph.bitmap.buffer))
            count += 1
    return count, digest.hexdigest()


def cff_stems(path: Path, name: str) -> dict[str, list[tuple[float, float]]]:
    font = TTFont(path)
    charstring = font["CFF "].cff.topDictIndex[0].CharStrings[name]
    charstring.decompile()
    result: dict[str, list[tuple[float, float]]] = {"h": [], "v": []}
    stack: list[float] = []
    for token in charstring.program:
        if isinstance(token, (float, int)):
            stack.append(token)
        elif isinstance(token, str):
            if token in ["hstem", "hstemhm", "vstem", "vstemhm"]:
                if len(stack) % 2:
                    stack.pop(0)  # The first operator may carry a charstring width.
                edge = 0
                for delta, width in zip(stack[::2], stack[1::2]):
                    start = edge + delta
                    result[token[0]].append((start, width))
                    edge = start + width
            stack = []
    return result


# Small sizes are where hinting earns its keep; 20 ppem checks it does no harm.
QUALITY_PPEMS = (9, 10, 11, 12, 14, 16, 20)


def edge_alignment(path: Path, ppems: tuple[int, ...], hinted: bool) -> tuple[int, float]:
    """Mean distance from a glyph's outer edges to the pixel grid, in pixels.

    A hint set exists to put the alignment edges of a glyph on whole pixels, so
    the mean distance from the hinted outline's extreme coordinates to the
    nearest integer is a direct, implementation-independent measure of how well
    it does that. Zero is perfect snapping; an unhinted outline averages about a
    quarter of a pixel because its edges fall wherever the scale puts them.
    """
    face = freetype.Face(str(path))
    flags = freetype.FT_LOAD_NO_BITMAP | freetype.FT_LOAD_TARGET_MONO
    if not hinted:
        flags |= freetype.FT_LOAD_NO_HINTING
    total = 0.0
    cells = 0
    for ppem in ppems:
        face.set_pixel_sizes(0, ppem)
        for gid in range(face.num_glyphs):
            face.load_glyph(gid, flags)
            points = face.glyph.outline.points
            if not points:
                continue
            xs = [point[0] / 64.0 for point in points]
            ys = [point[1] / 64.0 for point in points]
            for value in (min(xs), max(xs), min(ys), max(ys)):
                total += abs(value - round(value))
                cells += 1
    return cells, total / cells if cells else 0.0


def quality_gate(cli: Path, root: Path, source: Path, name: str) -> dict:
    """Compares FontCarver's analysis with the font's own hints and no hints.

    The source's own hints are exported untouched, the same source is exported
    with automatic hinting, and the first output is also measured with hinting
    switched off in FreeType. The analysis must snap edges at least as well as
    the source's own hints, within one hundredth of a pixel, and clearly better
    than no hints at all.
    """
    native = root / f"{name}-quality.fcf"
    run(cli, "convert", source, native)
    kept = root / f"{name}-kept.otf"
    run(cli, "build", native, kept)
    auto = root / f"{name}-quality-auto.otf"
    run(cli, "build", native, auto, "--autohint")
    cells, original = edge_alignment(kept, QUALITY_PPEMS, hinted=True)
    _, analysed = edge_alignment(auto, QUALITY_PPEMS, hinted=True)
    _, unhinted = edge_alignment(kept, QUALITY_PPEMS, hinted=False)
    def stem_count(path: Path) -> int:
        return sum(
            len(stems["h"]) + len(stems["v"])
            for stems in (cff_stems(path, glyph) for glyph in TTFont(path).getGlyphOrder())
        )

    source_stems = stem_count(kept)
    analysed_stems = stem_count(auto)
    assert analysed_stems > 0, name
    assert analysed < unhinted, (name, analysed, unhinted)
    if source_stems:
        assert analysed <= original + 0.01, (name, analysed, original)
    return {
        "measure": "mean pixels from a glyph's outer edges to the pixel grid",
        "ppem": list(QUALITY_PPEMS),
        "edges": cells,
        "source_stems": source_stems,
        "analysed_stems": analysed_stems,
        "with_source_hints": round(original, 5),
        "with_analysis": round(analysed, 5),
        "without_hints": round(unhinted, 5),
    }


def absent_hinter(cli: Path, root: Path) -> dict:
    """The TrueType path must disclose an absent hinter, never imply success."""
    source = root / "absent-source.ttf"
    font = TTFont("fixtures/fonts/NotoSans-Regular.ttf")
    options = subset.Options()
    options.hinting = False
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=range(65, 91))
    subsetter.subset(font)
    font.save(source)
    output = root / "absent-auto.ttf"
    warnings = run(cli, "build", source, output, "--autohint", "--no-preserve")
    assert "autohint-unavailable" in warnings, warnings
    result = TTFont(output)
    assert not {"cvt ", "prep", "fpgm"} & set(result.keys()), result.keys()
    programs = sum(
        bool(
            getattr(result["glyf"][name], "program", None)
            and result["glyf"][name].program.getBytecode()
        )
        for name in result.getGlyphOrder()
    )
    assert programs == 0, programs
    count, digest = render(output)
    return {"warning": "autohint-unavailable", "renders": count, "digest": digest}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=Path("target/debug/fontcarver"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--ttfautohint", action="store_true")
    args = parser.parse_args()
    cli, root = args.cli.resolve(), args.output.resolve()
    root.mkdir(parents=True, exist_ok=True)
    results = {}
    for name in ["SourceSans3Min", "SourceSerif4Min"]:
        source = Path("fixtures/fonts") / f"{name}.otf"
        native = root / f"{name}.fcf"
        run(cli, "convert", source, native)
        document = json.loads(native.read_text())
        # Authored stems exercise Type 2 stack splitting independently of import.
        glyph_name = "A" if "A" in document["font"]["glyphs"] else "o"
        layer = document["font"]["glyphs"][glyph_name]["layers"][0]
        expected = [(i * 100, 20) for i in range(48)]
        layer["hints"] = {
            "h_stems": [],
            "v_stems": [{"start": x, "width": w, "ranges": []} for x, w in expected],
            "hint_masks": [],
        }
        authored = root / f"{name}-authored.fcf"
        authored.write_text(json.dumps(document))
        output = root / f"{name}-authored.otf"
        run(cli, "build", authored, output)
        assert cff_stems(output, glyph_name)["v"] == expected
        count, digest = render(output)
        results[output.name] = {
            "renders": count,
            "digest": digest,
            "authored_stems": 48,
        }
        auto = root / f"{name}-auto.otf"
        warnings = run(cli, "build", source, auto, "--autohint", "--remove-overlap")
        assert "autohint-unavailable" not in warnings
        assert any(cff_stems(auto, glyph_name).values())
        count, digest = render(auto)
        results[auto.name] = {"renders": count, "digest": digest}
        results[f"{name}-quality"] = quality_gate(cli, root, source, name)

    if shutil.which("ttfautohint") is None:
        results["ttfautohint-absent"] = absent_hinter(cli, root)

    if args.ttfautohint:
        source = root / "NotoSans-latin.ttf"
        font = TTFont("fixtures/fonts/NotoSans-Regular.ttf")
        options = subset.Options()
        options.hinting = False
        subsetter = subset.Subsetter(options)
        subsetter.populate(unicodes=range(32, 127))
        subsetter.subset(font)
        font.save(source)
        plain = root / "NotoSans-plain.ttf"
        run(cli, "build", source, plain, "--no-preserve")
        for extension in ["ttf", "woff", "woff2"]:
            output = root / f"NotoSans-auto.{extension}"
            warnings = run(cli, "build", source, output, "--autohint", "--no-preserve")
            assert "autohint-unavailable" not in warnings
            hinted = TTFont(output)
            original = TTFont(plain)
            assert hinted.getBestCmap() == original.getBestCmap()
            assert hinted["hmtx"].metrics == original["hmtx"].metrics
            assert {"cvt ", "prep", "fpgm"} <= set(hinted.keys())
            # Decompile lazy glyph entries before counting their instructions.
            programs = sum(
                bool(
                    getattr(hinted["glyf"][name], "program", None)
                    and hinted["glyf"][name].program.getBytecode()
                )
                for name in hinted.getGlyphOrder()
            )
            assert programs > 50
            for name in original.getGlyphOrder():
                a = original["glyf"][name].getCoordinates(original["glyf"])
                b = hinted["glyf"][name].getCoordinates(hinted["glyf"])
                assert a == b, name
            count, digest = render(output)
            results[output.name] = {
                "renders": count,
                "digest": digest,
                "programs": programs,
            }
    (root / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
