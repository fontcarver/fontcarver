#!/usr/bin/env python3
"""Independently decode and render hinting exports from the shared CLI.

Requires FontTools and freetype-py. Outputs and subset fixtures stay outside the
repository. An optional PATH-installed ttfautohint exercises the real bridge.
"""

from __future__ import annotations

import argparse
import hashlib
import json
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
