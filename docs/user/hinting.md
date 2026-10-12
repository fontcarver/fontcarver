# Hint authoring and automatic hinting

Open a glyph and expand **Hints** in the inspector, then choose **PostScript
hints**. The dialog shows the selected outline with horizontal and vertical stem
bands. Edit each stem's start and width; ghost widths are `-20` and
`-21`. You can add point-bound replacement masks and counter masks. Mask indices
refer to stored contour points, including off-curve points in the numbering;
only explicit on-curve points may receive a mask. Enable **Closing segment** to
place a mask before the edge returning to the first on-curve point.

**Analyze stems and zones** computes a preview for a master layer. It finds
facing horizontal/vertical edges with ink between them and proposes missing
master private values. Existing explicit private values remain authoritative.
Counter checks inspect three sections through each candidate, so a crossbar does
not by itself make the surrounding counter qualify as a stem. This remains a
geometric heuristic; inspect the proposed hints before applying them.
Background and supplemental layers can be inspected and edited, but automatic
analysis requires selecting the master layer. Apply any master-parameter draft
before requesting another automatic analysis.

Analysis never discards what it did not propose: a layer's flex hints, dot
sections and unresolved-flex marker survive it, and a replacement mask is not
placed where a flex operator owns the point.

The analysis proposes four kinds of hint:

- **Stems** from facing edges, scored by overlap, width and how closely the
  width matches a named master stem. A stem whose edges fall inside an alignment
  zone is preferred where two candidates otherwise rank the same.
- **Ghost hints** for an alignment edge that no stem constrains: an edge inside
  an alignment zone, long enough to matter, with ink on exactly one side. A top
  edge becomes a `-20` hint at the edge; a bottom edge becomes a `-21` hint
  whose stored start is the edge plus 21.
- **Hint replacement** when two stems conflict. The highest scoring compatible
  stems stay active for the whole glyph; a conflicting stem that is at least as
  long as it is wide is kept and bound by masks to the points it governs, so a
  glyph whose stems overlap keeps both instead of losing the weaker one. A layer
  built from components has no point identity of its own, so it receives stems
  without replacement masks.
- **A counter group** when one axis holds exactly three stems with equal outer
  widths and equally spaced centres, which is the `hstem3`/`vstem3` form every
  interpreter understands.

For a CID font the analysis proposes values for every private dictionary the
selected glyphs use, inferring each dictionary's standard widths and stem snaps
from the glyphs assigned to it. Alignment zones and named stems still belong to
FD0.

The automatic analysis does not propose diagonal hints, and it does not emit a
whole-glyph "stem" for an axis with no detected stem the way some command-line
hinters do for diagonal-only glyphs such as `A` and `V`. On the committed CFF
fixtures the measured edge-alignment quality of the proposed hints matches the
fonts' own hints and is clearly better than no hints at all; the measurement is
recorded in `docs/p5f-k-hinting-acceptance.md`.

**Flex hints** cover two consecutive curve segments of one contour, for cupped
serifs and other very shallow curves that should disappear at small sizes. Give
the contour and the stored index of the on-curve point the first curve leaves;
the editor highlights the resolved curve pair on the outline and says so when a
point no longer starts a pair. Flex depth is the height, in hundredths of a
device pixel, at which the curves are used instead of a straight line; Adobe
recommends `50` for features that interact with overshoot zones. Type 1 output
additionally requires the two outer endpoints to share an x or y coordinate, and
the flex height must be 20 units or less; CFF output has no orientation
requirement and writes `flex1` at depth 50 or `flex` otherwise. If you use flex,
set `BlueShift` to at least the largest flex height plus one.

A hint mask may bind to the pair's joining point — its mask is written just
before the flex operator — but not to any point from there to the pair's end,
because one operator draws both curves.

**Dot sections** bracket one closed contour, such as the dot of an `i`, so stem
hints and alignment zones do not distort it. Type 1 output writes the
`dotsection` commands around that subpath. Type 2 lists the operator as
obsolete, so CFF export keeps the data in the document and reports
`cff-dotsection-dropped` instead of writing it.

The lower part of the dialog edits alignment zones, preferred stem widths and
PostScript private values, including blue zones and stem snaps. You can also open
these master parameters from **Font Info → Masters** without opening a glyph.
Invalid values remain visible for correction. **Apply changes** commits the whole
draft as one undo step; **Cancel** discards it. If the document changes while the
dialog is open, application fails and keeps the draft so edits are not silently
overwritten. Outline edits clear stale hint sets on that source and its component
dependents; undo restores the outline and hints together.

For CID fonts, **Edit private dictionary** selects FD0 or another imported font
dictionary. **Add private dictionary** copies FD0 values into a new dictionary;
up to 256 dictionaries are supported. Each dictionary's private values can be
edited independently. Alignment zones and named preferred stems belong to FD0.
With a glyph open, **Glyph private dictionary** selects the dictionary used for
that layer's CFF export. Dictionary edits and assignment changes apply together
as one undo step and survive FCF save and reopen. Automatic analysis currently
supports glyphs assigned to FD0 only; apply assignment changes and reopen the
editor before running analysis.

Static OTF export writes supported stems, masks, ghost hints and private values
into CFF. Imported ordinary CFF hints become editable. Non-axis-aligned font
matrices and seac transformations may prevent faithful hint import and produce a
warning. CID sources with multiple private dictionaries retain the first
as FD0 and retain the additional CFF1 dictionaries with each glyph's assignment.
Static CFF1 output writes those dictionaries and assignments independently.
Diagonal stems and coordinate-range hints must be converted to supported stems
and point masks before CFF export.

Variable OTF export writes the CFF2 private dictionary from the design default
master and varies every blendable value — blue zones, family zones, stem snaps,
standard widths and the blue thresholds — across the design space with the same
interpolation model as the outlines. `LanguageGroup` selects a hinting mode
rather than a measurement, so CFF2 does not allow it to vary and the default
master's value is used. Masters that declare different numbers of blue values or
stem snaps cannot be interpolated value by value; that is reported as
`cff2-private-not-varied` and the default master's values are written without
variation. CFF2 has no stem or hint-mask operators at all, so glyph stems,
replacement masks and flex are not part of a variable OTF.

Importing a variable OTF whose private dictionary varies reads its values at the
default location and reports `cff2-private-variation-flattened`: the document
model has one master for an imported variable font, so a re-export authors fresh
variation from this document's masters rather than replaying the original
deltas.

Imported CFF flex is recovered. The outline dependency expands `flex`, `hflex`,
`hflex1` and `flex1` into ordinary curves before FontCarver sees them, so a
separate bounded scan of the same charstring finds each flex and binds it to the
imported points. A flex that cannot be bound unambiguously, a `seac`
composition, or a charstring the scan cannot follow is reported as
`cff-flex-not-imported` and keeps the unresolved flex marker, which export
refuses rather than writing as ordinary curves. CFF2 charstrings are not scanned.
The obsolete Type 2 `dotsection` operator is reported as
`cff-dotsection-not-imported`.

## TrueType instructions

From a glyph's master layer, open **Hints → TrueType instructions**. The point
view shows the actual compiled quadratic outline, including off-curve controls
and expanded component points. Page through the coordinate table to inspect dense
outlines. Four phantom points follow the outline points. Horizontal phantom
coordinates are nominal parent metrics; a `USE_MY_METRICS` component can replace
them during interpretation. Without explicit vertical metrics, the vertical
phantom coordinates are marked as interpreter defaults.

Edit the glyph program, font program (`fpgm`), preparation program (`prep`), CVT and
interpreter maxima together. Global values affect every glyph in the selected
master. Assembly has one canonical instruction per line, decimal push operands,
and `#` or `;` comments. Push counts use the encoded count minus one:

```text
SVTCA[x]
PUSHB[0] 0
MDAP[rnd]
PUSHB[1] 0 64
SHPIX
```

This example rounds point 0 in the horizontal direction and moves it by one pixel.
Use actual point indices from the dialog. Syntax checks validate bytecode framing,
not runtime correctness. Set interpreter maxima to match the program; export
computes the maximum glyph-program size. Missing maxima begin with zero storage,
definitions and twilight points, two zones and a 256-element stack.

**Run instruction preview** executes the unsaved draft at one to eight sizes from
1 to 128 pixels per em. Filled outlines are hinted, dashed outlines are unhinted,
and the vertical line shows the adjusted advance. The preview uses Skrifa's
bounded interpreter and propagates reported execution errors. Glyph execution is
pedantic; global initialization uses Skrifa's compatibility behavior, which can
tolerate some stack errors. Preview success is not a proof that every interpreter
will accept a program. In particular, tested FreeType 2.13.2 and 2.14.3 skip legal
zero-contour programs, while Skrifa executes their phantom-point movements.

**Apply changes** checks the captured document revision and compiled point
identity, then stores glyph and master programs as one undo step. Cancel and
preview do not modify the document. FCF and compressed FCF preserve these programs
and bindings. Outline or metric edits retain the source for review; export rejects
stale authored programs. Reopen the editor, inspect the current points, correct the
draft and acknowledge the review before rebinding. Imported programs with a
different or unknown point identity remain available as reference, with an empty
editable glyph draft. Malformed imported bytes remain visible without preventing
PostScript editing.

Static master TTF, WOFF and WOFF2 export writes authored programs even when
**Keep TrueType instructions** is off; that option controls imported programs.
Compatible imported programs can remain alongside authored ones. Programs that no
longer match the compiled points are omitted with a warning. WOFF2 uses null
transforms when needed to preserve zero-contour instructions. CFF output reports
that authored TT programs cannot be included and leaves them in the document.

**Variable fonts.** Authored global data — the font program, the control value
program, the control value table and the interpreter maxima — exports with a
variable TrueType font, and the control values vary across the design space: the
design default master's values go into `cvt `, and every other master's values
become `cvar` deltas computed with the same interpolation model as the outlines.
Each master must declare the same number of control values, because a `cvar`
delta addresses one control value by index; a master that does not is reported as
`cvar-not-authored` and the table is written without variation. The same warning
is reported when a master other than the design default is selected, because
only the default master's values belong in `cvt `.

Authored **glyph** programs still require a static master export without geometry
preprocessing or automatic hinting. Their point binding records the identity of
the stored outline the editor showed, and the editor authors against a static
compilation, so a variable compilation's binding never matches; export refuses
rather than rebinding a program to points it was not reviewed against. Generated
static instances, overlap removal, automatic TT hinting, coordinate rounding,
extrema insertion, component decomposition, smart-component resolution and
bracket layers also reject authored glyph programs. Imported raw layout using
point-index references can prevent safe export after recompilation. Component
alignment through phantom points is not supported in the point view. These are
the remaining hinting boundaries.

## Type 1 hints

Static PFA/PFB export writes the selected layer's horizontal and vertical stems,
including fractional positions, `-20`/`-21` ghost hints and point-bound active
masks. It also writes blue zones, family zones, standard stem widths, stem snaps,
BlueScale/BlueShift/BlueFuzz, ForceBold and LanguageGroup. Zone pairs keep their
original order; only the values within each pair must be low-to-high.
Component outlines are expanded; their base glyphs' hint sets are not inherited.

The importer follows ordinary and standard hint-replacement subroutines, adds the
original sidebearing to vertical positions and normalizes representable hint
coordinates with FontMatrix. Replacement masks refer to the next segment's
on-curve endpoint, including closing edges. Export uses native replacement
subroutines; quadratic outlines become cubics while retaining the mask's segment
association. Stems may overlap only when every active mask selects a nonoverlapping
set. Values are also validated at the output's 16.16 precision.

Type 1 counter hints prefer `hstem3` or `vstem3`, which every PostScript
interpreter understands: three nonoverlapping positive-width stems with equal
outer widths and equally spaced centers, declared before the first contour, on an
axis with no ordinary stems, with each active mask enabling the complete triple
or none of it, and the same triple throughout the glyph.

Any other counter mask is written as general counter control through
`OtherSubrs` 12 and 13 (Adobe Technical Note #5015, sections 2.2 to 2.6): the
groups follow `hsbw` as pairs of distances and widths, split into calls of at
most 22 operands from the end of the data, with the group order carrying the
priority in which the rasterizer allocates white pixels. A group needs at least
two stems to delimit a counter, its stems must not overlap each other, and ghost
hints cannot take part. Those sequences are also imported again, and the stems a
group names must be declared as ordinary stem hints in the same glyph.

Terminal `seac` compositions inherit supported base and accent stems, ghost hints
and replacement masks. The importer expands their outlines, adjusts accent hints
for the sidebearings and offset, and keeps each component's active hints on its own
contours. PFA/PFB output writes the expanded geometry and replacement masks.
Nested compositions, parent drawing or hints before `seac`, a mismatched accent
sidebearing, component-scoped counter groups and components without bindable
contours remain unsupported. Their imported outlines retain the existing warning
instead of receiving a partial stem set.

Flex and dot sections round-trip. Export writes the `Subrs` 1/2/0 sequence of
section 8.3 with the reference point the specification prescribes — it shares the
joining point's cross coordinate and the starting point's own coordinate — and
reserves the four standard `Subrs` entries whenever flex or hint replacement is
used. `OtherSubrs` entries 0 to 2 are written as empty procedures: section 8.3
allows an interpreter to provide a built-in equivalent, every interpreter does,
and the real fonts that fill those entries delegate to `FlxProc` inside Adobe's
own interpreter, which is not available elsewhere. Entry 13 clears the
counter-control arguments that entry 12 accumulated, as interpreters that ignore
counter control require.

The importer reads literal private values without executing PostScript. A literal
private value followed by runtime PostScript is omitted with a warning; malformed
numeric arrays are rejected. Glyphs using unsupported `seac`, an unrepresentable
counter group or invalid replacement metadata retain their supported
outlines without a partial stem set. An import warning identifies the limitation;
affected glyph names remain in source metadata. Private values that cannot be
represented after reflection or an out-of-range transform remain in source
metadata and produce a separate warning.

Export rejects invalid active sets, unsupported counter groups, diagonal stems,
active ranges and an unresolved imported flex marker before replacing an existing
destination.
The Type 1 export dialog offers **Automatic hinting** and **Remove overlap**.
Removing overlaps without rehinting clears old glyph hints in the exported copy
and reports that change. The source document and its undo history remain intact.

## Automatic hints when exporting

In the export dialog, enable **Automatic hinting** for a static OTF, TTF, WOFF,
WOFF2, PFA or PFB export. It operates on a temporary copy; your source document remains
unchanged. **Remove overlap** runs before automatic hinting and also affects only
the exported copy. These options currently require a static master or instance.

- OTF, PFA and PFB use FontCarver's geometric PostScript stem analysis and zone inference.
- TrueType output uses the separately installed `ttfautohint` executable on
  `PATH`. FontCarver does not bundle or link it. If it is absent, export reports
  `autohint-unavailable` and leaves the font unhinted — it never claims success.
  An installed hinter's failure, invalid output or timeout makes the export fail;
  its error is retained. Messages an installed hinter writes while succeeding are
  reported as `autohint-message` warnings, so a run that worked around something
  still says so.

With **Automatic hinting** enabled for a TTF, WOFF or WOFF2 export, the dialog
shows an **External TrueType hinter** section. Its first line states whether a
`ttfautohint` executable was found on `PATH` and which version answered, before
the export runs. Below it are the tool's documented options:

| Option                        | Command line                            |
| ----------------------------- | --------------------------------------- |
| Hinting range minimum/maximum | `--hinting-range-min/-max`              |
| Hinting limit                 | `--hinting-limit` (0 removes it)        |
| Fallback stem width           | `--fallback-stem-width`                 |
| Increase x-height up to       | `--increase-x-height` (0 is off)        |
| Default and fallback script   | `--default-script`, `--fallback-script` |
| Strong stem widths            | `--strong-stem-width`                   |
| x-height snapping exceptions  | `--x-height-snapping-exceptions`        |
| Adjust subglyphs              | `--adjust-subglyphs`                    |
| Hint composites as a whole    | `--composites`                          |
| Remove all hints instead      | `--dehint`                              |
| Accept a symbol font          | `--symbol`                              |
| Record parameters             | `--ttfa-table`                          |
| Windows compatibility zones   | `--windows-compatibility`               |
| Version information           | `--no-info` / `--detailed-info`         |

Leave a number empty to keep the tool's own default. Every value is checked
before any process starts: ppem values must be between 2 and 10000 with the
minimum no greater than the maximum, a hinting limit must be zero or at least the
range maximum, a script is exactly four lowercase letters, strong stem widths
contain only `g`, `G` and `D`, and snapping exceptions are `-` or a
comma-separated list of ppem values and `min-max` ranges. An invalid value fails
the export request rather than being dropped. FontCarver never passes
`--ignore-restrictions`: a font's permission bits are the tool's to honour.

The command line takes the same options, so a scripted build hints exactly like
the application:

```sh
fontcarver build Source.fcf Output.ttf --autohint \
  --hint-range-min 8 --hint-range-max 50 --hint-limit 0 \
  --hint-fallback-stem-width 70 --hint-increase-x-height 14 \
  --hint-default-script latn --hint-fallback-script none \
  --hint-strong-stem-width gGD --hint-x-height-snapping-exceptions 10,14-16 \
  --hint-adjust-subglyphs --hint-composites --hint-symbol \
  --hint-ttfa-table --hint-windows-compatibility --hint-info detailed
```

Each flag requires `--autohint`, the same kernel validation applies, and an
invalid value fails the build with the kernel's own message before anything is
written. A stored profile carries its own options, so `--profile` and any
`--hint-…` flag together are refused rather than silently preferring one.

The shared CLI exposes the same operations:

```sh
fontcarver build Source.fcf Output.otf --autohint --remove-overlap
fontcarver build Source.fcf Output.pfb --autohint --remove-overlap
fontcarver build Source.fcf Output.ttf --autohint --master Regular
```

The external process uses private temporary files, a 120-second deadline and
bounded font/log sizes. It does not override font permission flags. The resulting
hint programs still require visual inspection at the intended pixel sizes;
automatic analysis is not a substitute for reviewing a typeface's rendering.

For the external hinter's behavior and installation, see the
[ttfautohint manual](https://freetype.org/ttfautohint/doc/ttfautohint.html).
