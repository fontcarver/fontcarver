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
Background and supplemental layers can be inspected and edited, but automatic
analysis requires selecting the master layer. Apply any master-parameter draft
before requesting another automatic analysis.

The lower part of the dialog edits alignment zones, preferred stem widths and
PostScript private values, including blue zones and stem snaps. You can also open
these master parameters from **Font Info → Masters** without opening a glyph.
Invalid values remain visible for correction. **Apply changes** commits the whole
draft as one undo step; **Cancel** discards it. If the document changes while the
dialog is open, application fails and keeps the draft so edits are not silently
overwritten. Outline edits clear stale hint sets on that source and its component
dependents; undo restores the outline and hints together.

Static OTF export writes supported stems, masks, ghost hints and private values
into CFF. Imported ordinary CFF hints become editable. Non-axis-aligned font
matrices and seac transformations may prevent faithful hint import and produce a
warning. CID sources with multiple private dictionaries retain only the first
as editable master parameters; export currently uses this dictionary for every
glyph and reports that limitation. Diagonal stems and coordinate-range hints must be converted to supported
stems and point masks before CFF export.

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

Variable TT output, generated static instances, overlap removal, automatic TT
hinting, smart-component resolution and bracket layers currently reject authored
TT programs. Imported raw layout using point-index references can also prevent
safe export after recompilation. Component alignment through phantom points is
not yet supported in the point view. These are remaining hinting boundaries.

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

Type 1 counter hints use `hstem3` or `vstem3`: three nonoverlapping positive-width
stems with equal outer widths and equally spaced centers. Declare the counter
mask before the first contour. A counter axis cannot contain ordinary stems,
and each active mask must enable the complete triple or none of it. The same
triple must be used throughout the glyph. General counter-control procedures
are not yet supported.

The importer reads literal private values without executing PostScript. A literal
private value followed by runtime PostScript is omitted with a warning; malformed
numeric arrays are rejected. Glyphs using dot sections, flex, seac, general
counter-control procedures or invalid replacement metadata retain their supported
outlines without a partial stem set. An import warning identifies the limitation;
affected glyph names remain in source metadata. Private values that cannot be
represented after reflection or an out-of-range transform remain in source
metadata and produce a separate warning.

Export rejects invalid active sets, unsupported counter groups, diagonal stems,
active ranges and flex metadata before replacing an existing destination.
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
  that automatic hinting was not performed. An installed hinter's failure,
  invalid output or timeout makes the export fail; its error is retained.

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
