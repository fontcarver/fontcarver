#!/usr/bin/env python3
"""Independently verify FontCarver's bitmap outputs for the P5 bitmap rows.

Drives a running `fc-devserver` over HTTP — the same kernel the application
uses — and reads everything it writes back with FontTools, FreeType and
HarfBuzz, none of which share code with the exporter.

Covers audit rows 64 (strike identity and undo), 67 (rasterization and its
threshold), 69 (the editable pixel glyph and both component export modes),
71 (BDF/PCF round trips and typed properties), 102 (EBDT/EBLC, CBDT/CBLC and
sbix) and 103 (no pixel-font false positive on an outline font).

Run FreeType with libpng. The `freetype-py` wheel bundles a build without it
and cannot decode a colour bitmap at all, reporting `unimplemented feature`;
`--system-freetype` points the binding at the system library instead:

    cargo build -p fc-devserver
    target/debug/fc-devserver --port 8807 &
    python3 scripts/p5f_g_bitmap_acceptance.py --out <scratch> --port 8807 \\
        --system-freetype

Outputs stay outside the repository: pass `--out` a scratch directory.
"""

from __future__ import annotations

import argparse
import ctypes.util
import json
import shutil
import sys
import urllib.error
import urllib.request
from pathlib import Path

BDF = """STARTFONT 2.1
FONT -FontCarver-Verify-Medium-R-Normal--8-80-72-72-C-80-ISO10646-1
SIZE 8 72 72
FONTBOUNDINGBOX 8 8 0 -2
STARTPROPERTIES 4
FONT_ASCENT 6
FONT_DESCENT 2
CHARSET_REGISTRY "ISO10646"
CHARSET_ENCODING "1"
ENDPROPERTIES
CHARS 2
STARTCHAR A
ENCODING 65
SWIDTH 1000 0
DWIDTH 8 0
BBX 8 8 0 -2
BITMAP
00
18
24
42
7E
42
42
00
ENDCHAR
STARTCHAR B
ENCODING 66
SWIDTH 1000 0
DWIDTH 8 0
BBX 8 8 0 -2
BITMAP
00
7C
42
7C
42
42
7C
00
ENDCHAR
ENDFONT
"""

# The source rows above, one byte per 8-pixel row, as every decoder must see them.
EXPECTED = {
    "A": [0x00, 0x18, 0x24, 0x42, 0x7E, 0x42, 0x42, 0x00],
    "B": [0x00, 0x7C, 0x42, 0x7C, 0x42, 0x42, 0x7C, 0x00],
}

ROOT = Path(__file__).resolve().parents[1]


def use_system_freetype(out: Path) -> None:
    """Point freetype-py at the system library, which has PNG support.

    The binding loads a bundled `libfreetype.so` from its own package directory
    if one is there, so the package is copied beside a symlink to the system
    library and that copy is put first on the import path.
    """
    import freetype as bundled  # noqa: PLC0415  (probe before the swap)

    package = Path(bundled.__file__).resolve().parent
    system = ctypes.util.find_library("freetype")
    if system is None:
        raise SystemExit("no system FreeType found; drop --system-freetype")
    if not Path(system).is_absolute():
        for root in ("/usr/lib/x86_64-linux-gnu", "/usr/lib", "/usr/lib64"):
            candidate = Path(root) / system
            if candidate.exists():
                system = str(candidate)
                break
    target = out / "freetype-system"
    shutil.rmtree(target, ignore_errors=True)
    target.mkdir(parents=True)
    shutil.copytree(package, target / "freetype")
    (target / "freetype" / "libfreetype.so").unlink(missing_ok=True)
    (target / "freetype" / "libfreetype.so").symlink_to(system)
    for name in [key for key in sys.modules if key.split(".")[0] == "freetype"]:
        del sys.modules[name]
    sys.path.insert(0, str(target))


class Kernel:
    """One `fc-devserver` over its HTTP transport."""

    def __init__(self, port: int) -> None:
        self.base = f"http://127.0.0.1:{port}/api/"

    def call(self, method: str, params: dict) -> object:
        request = urllib.request.Request(
            self.base + method,
            data=json.dumps(params).encode(),
            headers={"content-type": "application/json"},
        )
        try:
            with urllib.request.urlopen(request, timeout=180) as response:
                envelope = json.load(response)
        except urllib.error.HTTPError as error:
            envelope = json.load(error)
        except urllib.error.URLError as error:  # pragma: no cover - setup error
            raise SystemExit(
                f"no kernel on {self.base}: {error}. Start fc-devserver first."
            ) from error
        if envelope.get("error"):
            raise RuntimeError(f"{method}: {envelope['error']['message']}")
        return envelope.get("result")

    def generation(self, doc: int) -> int:
        return self.call("bitmap.state", {"doc": doc})["generation"]

    def export(self, doc: int, path: Path, options: dict) -> Path:
        self.call("session.export", {"doc": doc, "path": str(path), "options": options})
        return path


class Report:
    """One line per check, and a nonzero exit when any of them failed."""

    def __init__(self) -> None:
        self.checks: list[tuple[str, bool]] = []

    def check(self, name: str, ok: object, detail: str = "") -> None:
        self.checks.append((name, bool(ok)))
        suffix = f" — {detail}" if detail else ""
        print(("PASS " if ok else "FAIL ") + name + suffix)

    def finish(self) -> int:
        failed = [name for name, ok in self.checks if not ok]
        print()
        print(f"{len(self.checks) - len(failed)} of {len(self.checks)} checks passed")
        if failed:
            print("failed: " + "; ".join(failed))
        return 1 if failed else 0


def mono_rows(face, char: str) -> list[int]:
    import freetype

    face.load_char(char, freetype.FT_LOAD_RENDER | freetype.FT_LOAD_MONOCHROME)
    bitmap = face.glyph.bitmap
    return [bitmap.buffer[y * bitmap.pitch] for y in range(bitmap.rows)]


def alpha_rows(face, char: str) -> list[int]:
    """Alpha of a colour strike, packed back into one byte per row."""
    import freetype

    face.load_char(char, freetype.FT_LOAD_RENDER | freetype.FT_LOAD_COLOR)
    bitmap = face.glyph.bitmap
    rows = []
    for y in range(bitmap.rows):
        value = 0
        for x in range(bitmap.width):
            # FreeType hands colour bitmaps over as premultiplied BGRA.
            if bitmap.buffer[y * bitmap.pitch + x * 4 + 3] >= 128:
                value |= 0x80 >> x
        rows.append(value)
    return rows


def verify(kernel: Kernel, out: Path, report: Report) -> None:
    import freetype
    import uharfbuzz as hb
    from fontTools.ttLib import TTFont

    print(f"FreeType {freetype.version()} from {freetype.raw.filename}")

    source = out / "verify.bdf"
    source.write_text(BDF)
    doc = kernel.call("session.open", {"path": str(source)})["doc"]
    state = kernel.call("bitmap.state", {"doc": doc})
    report.check(
        "row 71: BDF import gives one 8 ppem 1 bpp strike with two images",
        [(s["ppem"], s["depth"], s["glyph_count"]) for s in state["strikes"]]
        == [(8, 1, 2)],
        str([(s["ppem"], s["depth"], s["glyph_count"]) for s in state["strikes"]]),
    )
    report.check(
        "row 71: typed BDF properties survive the import",
        state["strikes"][0]["properties"]["CHARSET_REGISTRY"] == {"Str": "ISO10646"}
        and state["strikes"][0]["properties"]["FONT_ASCENT"] == {"Int": 6},
        str(state["strikes"][0]["properties"]),
    )

    # --- Coverage strikes: TTF (EBDT/EBLC), OTB, BDF, PCF -----------------
    ttf = kernel.export(doc, out / "mono.ttf", {"format": "Ttf"})
    otb = kernel.export(doc, out / "bitmaps.otb", {"format": "Otb"})
    bdf = kernel.export(
        doc, out / "roundtrip.bdf", {"format": "Bdf", "bitmap_strike": [8, 1]}
    )
    pcf = kernel.export(
        doc, out / "roundtrip.pcf", {"format": "Pcf", "bitmap_strike": [8, 1]}
    )

    for path, label in ((ttf, "TTF"), (otb, "OTB")):
        font = TTFont(path)
        report.check(
            f"row 102: {label} has EBLC and EBDT", "EBLC" in font and "EBDT" in font
        )
        sizes = [
            (strike.bitmapSizeTable.ppemX, strike.bitmapSizeTable.bitDepth)
            for strike in font["EBLC"].strikes
        ]
        report.check(
            f"row 102: {label} EBLC advertises 8 ppem at 1 bpp",
            sizes == [(8, 1)],
            str(sizes),
        )
        strike = font["EBDT"].strikeData[0]
        ok = True
        for name, rows in EXPECTED.items():
            glyph = strike.get(name)
            if glyph is None or list(glyph.imageData) != rows:
                ok = False
                print(f"  {label} {name}: {glyph and list(glyph.imageData)} != {rows}")
            elif (
                glyph.metrics.width,
                glyph.metrics.height,
                glyph.metrics.BearingX,
                glyph.metrics.BearingY,
                glyph.metrics.Advance,
            ) != (8, 8, 0, 6, 8):
                ok = False
        report.check(
            f"row 102: {label} EBDT pixels, bearings and advances match the source", ok
        )
        if label == "OTB":
            report.check(
                "row 102: OTB is bitmap-only, with no outlines",
                all(font["glyf"][name].numberOfContours == 0 for name in EXPECTED),
            )

    for path, label in ((ttf, "TTF"), (otb, "OTB"), (bdf, "BDF"), (pcf, "PCF")):
        face = freetype.Face(str(path))
        report.check(
            f"row 102: FreeType advertises an 8 pixel strike in the {label}",
            any(size.height == 8 for size in face.available_sizes),
            str([(size.width, size.height) for size in face.available_sizes]),
        )
        face.set_pixel_sizes(0, 8)
        report.check(
            f"row 102: FreeType renders the {label} with the exact source pixels",
            all(mono_rows(face, name) == rows for name, rows in EXPECTED.items()),
        )

    for path, label in ((bdf, "BDF"), (pcf, "PCF")):
        reopened = kernel.call("session.open", {"path": str(path)})["doc"]
        ok = True
        for name, rows in EXPECTED.items():
            value = kernel.call(
                "bitmap.glyph", {"doc": reopened, "ppem": 8, "depth": 1, "glyph": name}
            )["value"]
            if value is None or value["bitmap"]["rows"] != rows:
                ok = False
        report.check(
            f"row 71: {label} reopens through the kernel with every pixel intact", ok
        )
        kernel.call("session.close", {"doc": reopened})

    hb_font = hb.Font(hb.Face(hb.Blob.from_file_path(str(ttf))))
    buf = hb.Buffer()
    buf.add_str("AB")
    buf.guess_segment_properties()
    hb.shape(hb_font, buf)
    names = [hb_font.get_glyph_name(info.codepoint) for info in buf.glyph_infos]
    report.check(
        "row 102: HarfBuzz shapes AB from the embedded-strike TTF",
        names == ["A", "B"],
        str(names),
    )

    # --- Colour strikes: CBDT/CBLC and sbix -------------------------------
    kernel.call(
        "bitmap.transfer",
        {
            "doc": doc,
            "expected_generation": kernel.generation(doc),
            "source_ppem": 8,
            "source_depth": 1,
            "target_ppem": 8,
            "target_depth": 32,
            "glyphs": [],
            "scale": False,
        },
    )
    # The dedicated colour flavours accept RGBA strikes only.
    kernel.call(
        "bitmap.apply",
        {
            "doc": doc,
            "expected_generation": kernel.generation(doc),
            "label": "Remove the coverage strike",
            "command": {"RemoveStrike": {"ppem": 8, "depth": 1}},
        },
    )
    cbdt = kernel.export(doc, out / "color.ttf", {"format": "Cbdt"})
    sbix = kernel.export(doc, out / "sbix.ttf", {"format": "Sbix"})
    kernel.call("doc.undo", {"doc": doc})
    report.check(
        "row 64: undo restores the removed coverage strike beside the RGBA one",
        [
            (s["ppem"], s["depth"])
            for s in kernel.call("bitmap.state", {"doc": doc})["strikes"]
        ]
        == [(8, 1), (8, 32)],
    )

    color = TTFont(cbdt)
    report.check(
        "row 102: CBDT and CBLC written for the RGBA strike",
        "CBDT" in color and "CBLC" in color,
    )
    report.check(
        "row 102: CBDT image data is a PNG",
        color["CBDT"].strikeData[0]["A"].imageData[:8] == b"\x89PNG\r\n\x1a\n",
    )
    graphics = TTFont(sbix)
    report.check("row 102: sbix written for the RGBA strike", "sbix" in graphics)
    glyph = graphics["sbix"].strikes[8].glyphs["A"]
    report.check(
        "row 102: sbix holds a PNG at 8 ppem",
        glyph.graphicType == "png " and glyph.imageData[:8] == b"\x89PNG\r\n\x1a\n",
    )
    for path, label in ((cbdt, "CBDT"), (sbix, "sbix")):
        face = freetype.Face(str(path))
        face.set_pixel_sizes(0, 8)
        report.check(
            f"row 102: FreeType decodes the {label} strike to the source coverage",
            all(alpha_rows(face, name) == rows for name, rows in EXPECTED.items()),
            str({name: alpha_rows(face, name) for name in EXPECTED}),
        )

    # --- Row 69: the editable pixel glyph and the two export modes --------
    kernel.call(
        "bitmap.pixel_components",
        {
            "doc": doc,
            "expected_generation": kernel.generation(doc),
            "ppem": 8,
            "depth": 1,
            "master": None,
            "glyphs": ["A"],
            "threshold": 128,
            "pixel_glyph": "pixel",
        },
    )
    # The outline flavours are compared without a strike in the way: a font that
    # carries bitmap tables is served from them by FreeType.
    for ppem, depth in ((8, 1), (8, 32)):
        kernel.call(
            "bitmap.apply",
            {
                "doc": doc,
                "expected_generation": kernel.generation(doc),
                "label": "Remove the strike",
                "command": {"RemoveStrike": {"ppem": ppem, "depth": depth}},
            },
        )
    kept = kernel.export(
        doc, out / "components.ttf", {"format": "Ttf", "decompose_components": False}
    )
    flat = kernel.export(
        doc, out / "decomposed.ttf", {"format": "Ttf", "decompose_components": True}
    )
    kept_font, flat_font = TTFont(kept), TTFont(flat)
    kept_glyph, flat_glyph = kept_font["glyf"]["A"], flat_font["glyf"]["A"]
    report.check(
        "row 69: keeping components exports A as a composite of the pixel glyph",
        kept_glyph.isComposite()
        and {component.glyphName for component in kept_glyph.components} == {"pixel"},
    )
    report.check(
        "row 69: the pixel glyph travels with the composite font",
        "pixel" in kept_font.getGlyphOrder(),
    )
    report.check(
        "row 69: decomposing writes A as simple contours with no components",
        not flat_glyph.isComposite() and flat_glyph.numberOfContours > 0,
        f"{flat_glyph.numberOfContours} contours",
    )
    renders = []
    for path in (kept, flat):
        face = freetype.Face(str(path))
        face.set_char_size(32 * 64)
        face.load_char("A", freetype.FT_LOAD_RENDER | freetype.FT_LOAD_MONOCHROME)
        bitmap = face.glyph.bitmap
        renders.append(
            [
                list(bitmap.buffer[y * bitmap.pitch : (y + 1) * bitmap.pitch])
                for y in range(bitmap.rows)
            ]
        )
    report.check(
        "row 69: FreeType renders the composite and the decomposed glyph identically",
        renders[0] == renders[1] and any(any(row) for row in renders[0]),
        f"{len(renders[0])} rows",
    )
    kernel.call("session.close", {"doc": doc})

    # --- Rows 67 and 103 on a real outline font ---------------------------
    real = kernel.call(
        "session.open", {"path": str(ROOT / "fixtures/fonts/DejaVuSans.ttf")}
    )["doc"]
    detection = kernel.call("bitmap.detect_pixel_grid", {"doc": real, "master": None})
    report.check(
        "row 103: a normal outline font is not detected as a pixel font",
        detection["ppem"] is None and detection["evidence_glyphs"] == 0,
        str(detection),
    )
    kernel.call(
        "bitmap.rasterize",
        {
            "doc": real,
            "expected_generation": kernel.generation(real),
            "master": None,
            "ppem": 16,
            "depth": 1,
            "glyphs": ["A", "B"],
            "hinting": None,
            "threshold": 128,
        },
    )
    raster = kernel.call(
        "bitmap.glyph", {"doc": real, "ppem": 16, "depth": 1, "glyph": "A"}
    )["value"]
    rastered = kernel.export(
        real, out / "rasterized.bdf", {"format": "Bdf", "bitmap_strike": [16, 1]}
    )
    face = freetype.Face(str(rastered))
    face.set_pixel_sizes(0, 16)
    face.load_char("A", freetype.FT_LOAD_RENDER | freetype.FT_LOAD_MONOCHROME)
    bitmap = face.glyph.bitmap
    stride = (raster["bitmap"]["width"] + 7) // 8
    stored = [
        raster["bitmap"]["rows"][y * stride : (y + 1) * stride]
        for y in range(raster["bitmap"]["height"])
    ]
    rendered = [
        list(bitmap.buffer[y * bitmap.pitch : y * bitmap.pitch + stride])
        for y in range(bitmap.rows)
    ]
    report.check(
        "row 67: FreeType renders the rasterized 16 ppem strike exactly as stored",
        stored == rendered,
        f"{stored} vs {rendered}",
    )
    kernel.call(
        "bitmap.rasterize",
        {
            "doc": real,
            "expected_generation": kernel.generation(real),
            "master": None,
            "ppem": 16,
            "depth": 1,
            "glyphs": ["A"],
            "hinting": None,
            "threshold": 16,
        },
    )
    loose = kernel.call(
        "bitmap.glyph", {"doc": real, "ppem": 16, "depth": 1, "glyph": "A"}
    )["value"]
    report.check(
        "row 67: a lower monochrome threshold keeps at least as much ink",
        sum(bin(byte).count("1") for byte in loose["bitmap"]["rows"])
        >= sum(bin(byte).count("1") for byte in raster["bitmap"]["rows"]),
    )
    kernel.call("session.close", {"doc": real})


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, type=Path, help="scratch directory")
    parser.add_argument("--port", type=int, default=8807, help="fc-devserver port")
    parser.add_argument(
        "--system-freetype",
        action="store_true",
        help="use the system FreeType, which has PNG support",
    )
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if args.system_freetype:
        use_system_freetype(out)
    report = Report()
    verify(Kernel(args.port), out, report)
    return report.finish()


if __name__ == "__main__":
    sys.exit(main())
