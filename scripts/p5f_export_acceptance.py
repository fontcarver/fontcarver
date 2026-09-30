#!/usr/bin/env python3
"""Independently verify FontCarver export outputs for the P5 export rows.

Builds a small two-axis source with FontTools, drives the `fontcarver` command
line for every format and production option the exit checklist names, and reads
each output back with FontTools, FreeType and HarfBuzz — none of which share
code with the exporter.

Covers audit rows 54 (design-space subsetting), 58 and 63 (project-stored
profiles shared with the command line), 59 (formats) and 60 (production
post-processing).

Outputs stay outside the repository: pass `--out` a scratch directory.
"""

from __future__ import annotations

import argparse
import copy
import json
import shutil
import subprocess
import sys
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from types import SimpleNamespace

import freetype
import uharfbuzz as hb
from fontTools.designspaceLib import (
    AxisDescriptor,
    DesignSpaceDocument,
    InstanceDescriptor,
    SourceDescriptor,
)
from fontTools.pens.pointPen import SegmentToPointPen
from fontTools.ttLib import TTFont
from fontTools.ufoLib import UFOWriter

# A square-ish "A" whose curve needs an extremum inserted, and a plain "B".
GLYPHS = {
    "A": [
        ("moveTo", ((100, 0),)),
        # The control points push the extremum off the existing nodes.
        ("curveTo", ((100, 500), (400, 700), (500, 300))),
        ("lineTo", ((500, 0),)),
        ("closePath", ()),
    ],
    "B": [
        ("moveTo", ((80, 0),)),
        ("lineTo", ((80, 700),)),
        ("lineTo", ((520, 700),)),
        ("lineTo", ((520, 0),)),
        ("closePath", ()),
    ],
}

# Several glyphs that share one long outline, so whole-program CFF sharing has
# something to factor into a global subroutine.
REPEATED = [
    ("moveTo", ((60, 0),)),
    ("lineTo", ((60, 120),)),
    ("lineTo", ((180, 120),)),
    ("lineTo", ((180, 240),)),
    ("lineTo", ((300, 240),)),
    ("lineTo", ((300, 360),)),
    ("lineTo", ((420, 360),)),
    ("lineTo", ((420, 480),)),
    ("lineTo", ((540, 480),)),
    ("lineTo", ((540, 0),)),
    ("closePath", ()),
]
for repeated in "CDEF":
    GLYPHS[repeated] = REPEATED


@dataclass
class Report:
    """Collected checks, so one failure does not hide the rest."""

    passes: list[str] = field(default_factory=list)
    failures: list[str] = field(default_factory=list)

    def check(self, condition: bool, message: str) -> bool:
        (self.passes if condition else self.failures).append(message)
        return condition

    def print(self) -> None:
        for line in self.passes:
            print(f"PASS {line}")
        for line in self.failures:
            print(f"FAIL {line}")
        print(f"{len(self.passes)} passed, {len(self.failures)} failed")


def draw(pen, recording) -> None:
    for operator, operands in recording:
        getattr(pen, operator)(*operands)


def scaled(recording, scale: float):
    """The same drawing with every x multiplied, so the axes really vary."""
    return [
        (operator, tuple((x * scale, y) for x, y in operands))
        for operator, operands in recording
    ]


class SourceGlyph:
    """A UFO glyph object with a segment-pen drawing."""

    def __init__(self, name: str, recording, width: float) -> None:
        self.name = name
        self.width = width
        self.unicodes = [ord(name)]
        self.height = 0
        self.lib = {}
        self.note = None
        self.anchors = []
        self.guidelines = []
        self.image = None
        self._recording = recording

    def drawPoints(self, pointPen) -> None:  # noqa: N803 - fontTools protocol
        draw(SegmentToPointPen(pointPen), self._recording)


def write_master(path: Path, family: str, style: str, scale: float) -> None:
    """One UFO master whose drawing is widened by `scale`."""
    writer = UFOWriter(str(path), formatVersion=3)
    writer.writeInfo(
        SimpleNamespace(
            familyName=family,
            styleName=style,
            unitsPerEm=1000,
            ascender=800,
            descender=-200,
            xHeight=500,
            capHeight=700,
            italicAngle=0,
        )
    )
    glyphs = writer.getGlyphSet()
    for name, recording in GLYPHS.items():
        glyph = SourceGlyph(name, scaled(recording, scale), 600 * scale)
        glyphs.writeGlyph(name, glyph, glyph.drawPoints)
    glyphs.writeContents()
    writer.writeLayerContents()
    writer.close()


def build_source(out: Path) -> Path:
    """A designspace with weight and width axes and four corner masters."""
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True, exist_ok=True)
    document = DesignSpaceDocument()
    for tag, name, minimum, default, maximum in [
        ("wght", "Weight", 400, 400, 700),
        ("wdth", "Width", 75, 100, 100),
    ]:
        axis = AxisDescriptor()
        axis.tag = tag
        axis.name = name
        axis.minimum = minimum
        axis.default = default
        axis.maximum = maximum
        document.addAxis(axis)
    corners = [
        ("Regular", 400, 100, 1.0),
        ("Bold", 700, 100, 1.1),
        ("Condensed", 400, 75, 0.8),
        ("Bold Condensed", 700, 75, 0.9),
    ]
    for style, weight, width, scale in corners:
        ufo = out / f"Acceptance-{style.replace(' ', '')}.ufo"
        write_master(ufo, "Acceptance", style, scale)
        source = SourceDescriptor()
        source.path = str(ufo)
        source.filename = ufo.name
        source.familyName = "Acceptance"
        source.styleName = style
        source.location = {"Weight": weight, "Width": width}
        if style == "Regular":
            source.copyInfo = True
        document.addSource(source)
    for style, weight, width, _ in corners:
        instance = InstanceDescriptor()
        instance.familyName = "Acceptance"
        instance.styleName = style
        instance.name = style
        instance.location = {"Weight": weight, "Width": width}
        document.addInstance(instance)
    path = out / "Acceptance.designspace"
    document.write(str(path))
    return path


def run_cli(cli: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        [str(cli), *arguments], capture_output=True, text=True, check=False
    )
    if result.returncode != 0:
        print(result.stdout)
        print(result.stderr, file=sys.stderr)
    return result


def freetype_loads(path: Path, report: Report, label: str) -> None:
    try:
        face = freetype.Face(str(path))
        face.set_char_size(32 * 64)
        face.load_char("A")
        report.check(
            face.glyph.outline.n_points > 0 or face.glyph.bitmap.width > 0,
            f"{label}: FreeType renders A",
        )
    except Exception as error:  # noqa: BLE001 - report, do not abort the suite
        report.check(False, f"{label}: FreeType failed: {error}")


def harfbuzz_shapes(path: Path, report: Report, label: str) -> None:
    try:
        blob = hb.Blob.from_file_path(str(path))
        font = hb.Font(hb.Face(blob))
        buffer = hb.Buffer()
        buffer.add_str("AB")
        buffer.guess_segment_properties()
        hb.shape(font, buffer)
        report.check(
            len(buffer.glyph_infos) == 2
            and all(info.codepoint != 0 for info in buffer.glyph_infos),
            f"{label}: HarfBuzz maps both characters",
        )
    except Exception as error:  # noqa: BLE001
        report.check(False, f"{label}: HarfBuzz failed: {error}")


def on_curve_points(font: TTFont, name: str) -> int:
    glyph = font["glyf"][name]
    if glyph.numberOfContours <= 0:
        return 0
    return sum(1 for flag in glyph.flags if flag & 0x01)


def check_variable(cli: Path, source: Path, out: Path, report: Report) -> None:
    variable = out / "variable.ttf"
    result = run_cli(cli, "build", str(source), str(variable), "--variable")
    if not report.check(result.returncode == 0, "variable TTF builds"):
        return
    font = TTFont(variable)
    axes = {axis.axisTag: (axis.minValue, axis.defaultValue, axis.maxValue) for axis in font["fvar"].axes}
    report.check(axes.get("wght") == (400, 400, 700), f"variable fvar wght: {axes}")
    report.check(axes.get("wdth") == (75, 100, 100), f"variable fvar wdth: {axes}")
    report.check("gvar" in font, "variable TTF has gvar deltas")
    report.check(
        len(font["fvar"].instances) == 4,
        f"variable TTF writes four named instances: {len(font['fvar'].instances)}",
    )
    freetype_loads(variable, report, "variable TTF")
    harfbuzz_shapes(variable, report, "variable TTF")

    cff2 = out / "variable.otf"
    result = run_cli(cli, "build", str(source), str(cff2), "--variable")
    if report.check(result.returncode == 0, "variable CFF2 OTF builds"):
        font = TTFont(cff2)
        report.check("CFF2" in font, "variable OTF has CFF2")
        freetype_loads(cff2, report, "variable OTF")

    # Row 54: a master subset collapses the axis its masters no longer vary.
    subset = out / "subset.ttf"
    result = run_cli(
        cli,
        "build",
        str(source),
        str(subset),
        "--variable",
        "--source-master",
        "Regular",
        "--source-master",
        "Bold",
    )
    if report.check(result.returncode == 0, "design-space subset builds"):
        font = TTFont(subset)
        tags = [axis.axisTag for axis in font["fvar"].axes]
        report.check(tags == ["wght"], f"subset keeps only the varying axis: {tags}")
        names = [
            font["name"].getDebugName(instance.subfamilyNameID)
            for instance in font["fvar"].instances
        ]
        report.check(
            sorted(names) == ["Bold", "Regular"],
            f"subset drops instances outside the retained space: {names}",
        )
        freetype_loads(subset, report, "subset TTF")


def check_static_formats(cli: Path, source: Path, out: Path, report: Report) -> None:
    instance = out / "regular.otf"
    result = run_cli(
        cli, "build", str(source), str(instance), "--instance-name", "Bold"
    )
    if report.check(result.returncode == 0, "static instance OTF builds"):
        font = TTFont(instance)
        report.check("CFF " in font, "static OTF has CFF 1")
        report.check("fvar" not in font, "static OTF has no fvar")
        report.check(
            font["OS/2"].usWeightClass == 700,
            f"static instance carries its weight class: {font['OS/2'].usWeightClass}",
        )
        freetype_loads(instance, report, "static OTF")
        harfbuzz_shapes(instance, report, "static OTF")

    for extension, flavor in [("woff", "woff"), ("woff2", "woff2")]:
        path = out / f"webfont.{extension}"
        result = run_cli(cli, "build", str(source), str(path), "--variable")
        if report.check(result.returncode == 0, f"{extension.upper()} builds"):
            font = TTFont(path)
            report.check(
                font.flavor == flavor, f"{extension.upper()} flavor is {font.flavor}"
            )
            report.check(
                font.getGlyphOrder()[:3] == TTFont(out / "variable.ttf").getGlyphOrder()[:3],
                f"{extension.upper()} keeps the glyph order",
            )

    # Row 59: UFO and designspace round trip through the command line. The
    # designspace destination is a directory, like a UFO package.
    directory = out / "roundtrip.designspace"
    result = run_cli(cli, "convert", str(source), str(directory))
    if report.check(result.returncode == 0, "designspace + UFO writes"):
        written = sorted(directory.glob("*.designspace"))
        if report.check(len(written) == 1, f"one designspace document: {written}"):
            designspace = written[0]
            document = DesignSpaceDocument.fromfile(str(designspace))
            report.check(
                len(document.sources) == 4,
                f"designspace lists every master: {len(document.sources)}",
            )
            report.check(
                all(
                    (designspace.parent / entry.filename).is_dir()
                    for entry in document.sources
                ),
                "every designspace source UFO is on disk",
            )
            again = out / "roundtrip.ttf"
            result = run_cli(cli, "build", str(designspace), str(again), "--variable")
            report.check(
                result.returncode == 0, "the written designspace builds again"
            )
            if result.returncode == 0:
                freetype_loads(again, report, "designspace round trip")

    single = out / "single.ufo"
    result = run_cli(cli, "convert", str(source), str(single))
    if report.check(result.returncode == 0, "one UFO master writes"):
        report.check(
            (single / "glyphs" / "contents.plist").is_file(),
            "the UFO package has a glyph layer",
        )


def check_production_options(cli: Path, source: Path, out: Path, report: Report) -> None:
    plain = out / "plain.ttf"
    run_cli(cli, "build", str(source), str(plain), "--instance-name", "Regular")
    plain_points = on_curve_points(TTFont(plain), "A")

    extrema = out / "extrema.ttf"
    result = run_cli(
        cli,
        "build",
        str(source),
        str(extrema),
        "--instance-name",
        "Regular",
        "--add-extrema",
    )
    if report.check(result.returncode == 0, "extrema insertion builds"):
        points = on_curve_points(TTFont(extrema), "A")
        report.check(
            points > plain_points,
            f"extrema insertion adds on-curve points: {plain_points} -> {points}",
        )
        freetype_loads(extrema, report, "extrema TTF")

    rounded = out / "rounded.ttf"
    result = run_cli(
        cli,
        "build",
        str(source),
        str(rounded),
        "--instance-name",
        "Condensed",
        "--round-coordinates",
    )
    if report.check(result.returncode == 0, "coordinate rounding builds"):
        font = TTFont(rounded)
        integral = True
        for name in font.getGlyphOrder():
            glyph = font["glyf"][name]
            if glyph.numberOfContours > 0:
                integral &= all(
                    float(value).is_integer()
                    for value in list(glyph.coordinates.array)
                )
        report.check(integral, "every rounded glyf coordinate is integral")

    # A composite input: the designspace has none, so add one to a project.
    composite_project = out / "composite.fcf"
    if run_cli(cli, "convert", str(source), str(composite_project)).returncode == 0:
        document = json.loads(composite_project.read_text(encoding="utf-8"))
        glyphs = document["font"]["glyphs"]
        composite = copy.deepcopy(glyphs["A"])
        composite["name"] = "Aacute"
        composite["unicodes"] = [0x00C1]
        for layer in composite["layers"]:
            layer["id"] = uuid.uuid4().hex
            layer["contours"] = []
            layer["components"] = [
                {"base": "A", "transform": [1, 0, 0, 1, 40, 60]}
            ]
        glyphs["Aacute"] = composite
        composite_project.write_text(json.dumps(document), encoding="utf-8")

        # First prove the fixture really compiles to a composite glyph.
        kept = out / "composite.ttf"
        result = run_cli(
            cli,
            "build",
            str(composite_project),
            str(kept),
            "--instance-name",
            "Regular",
        )
        if report.check(result.returncode == 0, "a composite input builds"):
            font = TTFont(kept)
            glyph = font["glyf"]["Aacute"]
            report.check(
                glyph.numberOfContours == -1,
                "the component glyph compiles to a composite",
            )
            report.check(
                [c.glyphName for c in glyph.components] == ["A"],
                "the composite references its base glyph",
            )

        decomposed = out / "decomposed.ttf"
        result = run_cli(
            cli,
            "build",
            str(composite_project),
            str(decomposed),
            "--instance-name",
            "Regular",
            "--decompose-components",
        )
        if report.check(result.returncode == 0, "component decomposition builds"):
            font = TTFont(decomposed)
            composites = [
                name
                for name in font.getGlyphOrder()
                if font["glyf"][name].numberOfContours == -1
            ]
            report.check(
                not composites, f"no composite glyph remains: {composites}"
            )
            outline = font["glyf"]["Aacute"]
            report.check(
                outline.numberOfContours > 0,
                "the decomposed glyph kept its outline",
            )
            # The component's offset survives decomposition.
            base = font["glyf"]["A"]
            report.check(
                min(outline.coordinates.array[0::2])
                == min(base.coordinates.array[0::2]) + 40,
                "the decomposed outline keeps the component offset",
            )

    # Row 60: a production name the source supplies must reach `post`.
    project = out / "production.fcf"
    if run_cli(cli, "convert", str(source), str(project)).returncode == 0:
        document = json.loads(project.read_text(encoding="utf-8"))
        document["font"]["glyphs"]["A"]["production_name"] = "uni0041"
        project.write_text(json.dumps(document), encoding="utf-8")
        production = out / "production.ttf"
        result = run_cli(
            cli,
            "build",
            str(project),
            str(production),
            "--instance-name",
            "Regular",
            "--production-names",
        )
        if report.check(result.returncode == 0, "production naming builds"):
            names = TTFont(production).getGlyphOrder()
            report.check(
                "uni0041" in names and "A" not in names,
                f"the production name replaces the source name: {names[:5]}",
            )
            plain_names = TTFont(out / "plain.ttf").getGlyphOrder()
            report.check(
                "A" in plain_names,
                f"without the option the source name is kept: {plain_names[:5]}",
            )

    subroutinized = out / "subroutinized.otf"
    result = run_cli(
        cli,
        "build",
        str(source),
        str(subroutinized),
        "--instance-name",
        "Regular",
        "--subroutinize",
    )
    if report.check(result.returncode == 0, "CFF subroutinization builds"):
        font = TTFont(subroutinized)
        cff = font["CFF "].cff
        top = cff[cff.fontNames[0]]
        globals_count = len(cff.GlobalSubrs)
        locals_count = len(getattr(top.Private, "Subrs", []) or [])
        report.check(
            globals_count + locals_count > 0,
            f"subroutinized CFF has subroutines: {globals_count} global, {locals_count} local",
        )
        plain_otf = out / "plain.otf"
        run_cli(cli, "build", str(source), str(plain_otf), "--instance-name", "Regular")
        report.check(
            subroutinized.stat().st_size <= plain_otf.stat().st_size,
            f"subroutinized CFF is not larger: {subroutinized.stat().st_size} <= {plain_otf.stat().st_size}",
        )
        freetype_loads(subroutinized, report, "subroutinized OTF")


def check_project_profiles(cli: Path, source: Path, out: Path, report: Report) -> None:
    """Rows 58 and 63: the profiles live in the project file the CLI reads."""
    project = out / "Acceptance.fcf"
    result = run_cli(cli, "convert", str(source), str(project))
    if not report.check(result.returncode == 0, "project file writes"):
        return
    document = json.loads(project.read_text(encoding="utf-8"))
    inventory = (
        document["font"].setdefault("info", {}).setdefault("user_data", {})
    )
    if not isinstance(inventory, dict):
        report.check(False, "font info user data is an object")
        return
    collection = inventory.setdefault("com.fontcarver.inventory", {})
    options = {
        "format": "Woff2",
        "master": None,
        "variable": False,
        "remove_overlap": False,
        "autohint": False,
        "keep_tt_instructions": False,
        "preserve_untouched_tables": True,
        "production_names": True,
        "table_subset": None,
    }
    collection["export_profiles"] = [
        {"name": "Web", "subfolder": "web", "file_name": "", "options": options},
        {
            "name": "VF",
            "subfolder": "",
            "file_name": "Acceptance-VF",
            "options": {**options, "format": "Ttf", "variable": True},
        },
        {
            # One stored file name for a profile run over several rows.
            "name": "Desktop",
            "subfolder": "desktop",
            "file_name": "Acceptance",
            "options": {**options, "format": "Ttf"},
        },
    ]
    collection["export_variable_configs"] = [
        {
            "name": "Text",
            "file_name": "Acceptance-Text",
            "masters": [],
            "axes": ["wght"],
            "instances": ["Regular", "Bold"],
        }
    ]
    project.write_text(json.dumps(document), encoding="utf-8")

    listed = run_cli(cli, "profiles", str(project))
    report.check(
        listed.returncode == 0 and "profile Web: format=Woff2" in listed.stdout,
        f"the command line lists the stored profiles: {listed.stdout.strip()[:120]}",
    )

    dist = out / "dist"
    built = run_cli(
        cli,
        "build",
        str(project),
        "--profile",
        "Web",
        "--instances",
        "all",
        "--out-dir",
        str(dist),
        "--preflight",
    )
    report.check(built.returncode == 0, "a profile builds every named instance")
    report.check("preflight:" in built.stdout, "the build prints its preflight report")
    written = sorted(path.name for path in (dist / "web").glob("*.woff2"))
    report.check(
        len(written) == 4,
        f"one file per active instance under the profile subfolder: {written}",
    )
    for name in written:
        font = TTFont(dist / "web" / name)
        report.check(font.flavor == "woff2", f"{name} is WOFF2")

    # A profile with its own file name must still give every row its own file.
    named = run_cli(
        cli,
        "build",
        str(project),
        "--profile",
        "Desktop",
        "--instances",
        "all",
        "--out-dir",
        str(dist),
    )
    if report.check(named.returncode == 0, "a named profile builds every instance"):
        rows = sorted(path.name for path in (dist / "desktop").glob("*.ttf"))
        report.check(
            rows
            == [
                "Acceptance-Bold-Condensed.ttf",
                "Acceptance-Bold.ttf",
                "Acceptance-Condensed.ttf",
                "Acceptance-Regular.ttf",
            ],
            f"the stored file name carries each row's name: {rows}",
        )

    variable = run_cli(
        cli,
        "build",
        str(project),
        "--profile",
        "VF",
        "--variable-config",
        "Text",
        "--out-dir",
        str(dist),
    )
    if report.check(variable.returncode == 0, "a stored variable configuration builds"):
        # The configuration's own file name is per-row, so it wins.
        path = dist / "Acceptance-Text.ttf"
        report.check(path.is_file(), f"{path.name} is written")
        if path.is_file():
            font = TTFont(path)
            tags = [axis.axisTag for axis in font["fvar"].axes]
            report.check(
                tags == ["wght"],
                f"the configuration's axis subset reaches the output: {tags}",
            )
            names = sorted(
                font["name"].getDebugName(instance.subfamilyNameID)
                for instance in font["fvar"].instances
            )
            report.check(
                names == ["Bold", "Regular"],
                f"the configuration's instance subset reaches the output: {names}",
            )
            freetype_loads(path, report, "configuration TTF")

    # The same profile must refuse ad-hoc flags that would change its output.
    conflict = run_cli(
        cli,
        "build",
        str(project),
        "--profile",
        "Web",
        "--out-dir",
        str(out / "conflict"),
        "--remove-overlap",
    )
    report.check(
        conflict.returncode == 2 and "--remove-overlap" in conflict.stderr,
        "a profile refuses ad-hoc option flags",
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path, help="fontcarver binary")
    parser.add_argument("--out", required=True, type=Path, help="scratch directory")
    arguments = parser.parse_args()
    out: Path = arguments.out
    out.mkdir(parents=True, exist_ok=True)
    report = Report()
    source = build_source(out / "source")
    check_variable(arguments.cli, source, out, report)
    check_static_formats(arguments.cli, source, out, report)
    check_production_options(arguments.cli, source, out, report)
    check_project_profiles(arguments.cli, source, out, report)
    report.print()
    return 1 if report.failures else 0


if __name__ == "__main__":
    sys.exit(main())
