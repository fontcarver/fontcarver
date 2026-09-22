# PostScript hints and automatic hinting

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
stems and point masks before CFF export. A full TrueType instruction editor is
still under development.

## Type 1 hints

Static PFA/PFB export writes the selected layer's whole-glyph horizontal and
vertical stems, including fractional positions and `-20`/`-21` ghost hints.
It also writes blue zones, family zones, standard stem widths, stem snaps,
BlueScale/BlueShift/BlueFuzz, ForceBold and LanguageGroup. Zone pairs keep their
original order; only the values within each pair must be low-to-high.
Component outlines are expanded; their base glyphs' hint sets are not inherited.

The importer retains whole-glyph stems from direct charstrings and ordinary
subroutines, adds the original sidebearing to vertical positions and normalizes
representable hint coordinates with FontMatrix. It reads literal private values
without executing PostScript. A literal private value followed by runtime
PostScript is omitted with a warning; malformed numeric arrays are rejected. Source glyphs
using hint replacement, counter stems, dot sections, flex or seac currently import
with editable outlines but without their stem sets. An import warning identifies
this limitation; affected glyph names remain in source metadata. Private values
that cannot be represented after reflection or an out-of-range transform remain
in source metadata and produce a separate warning.

Export rejects overlapping stems, point masks, counter masks, diagonal stems,
active ranges and flex metadata until they are converted to supported whole-glyph
stems or removed. A failed export leaves an existing destination unchanged.
Automatic hinting and overlap removal through the Type 1 export options are still
pending. These boundaries are part of the unfinished Type 1 hinting work.

## Automatic hints when exporting

In the export dialog, enable **Automatic hinting** for a static OTF, TTF, WOFF or
WOFF2 export. It operates on a temporary copy; your source document remains
unchanged. **Remove overlap** runs before automatic hinting and also affects only
the exported copy. These options currently require a static master or instance.

- OTF uses FontCarver's geometric PostScript stem analysis and zone inference.
- TrueType output uses the separately installed `ttfautohint` executable on
  `PATH`. FontCarver does not bundle or link it. If it is absent, export reports
  that automatic hinting was not performed. An installed hinter's failure,
  invalid output or timeout makes the export fail; its error is retained.

The shared CLI exposes the same operations:

```sh
fontcarver build Source.fcf Output.otf --autohint --remove-overlap
fontcarver build Source.fcf Output.ttf --autohint --master Regular
```

The external process uses private temporary files, a 120-second deadline and
bounded font/log sizes. It does not override font permission flags. The resulting
hint programs still require visual inspection at the intended pixel sizes;
automatic analysis is not a substitute for reviewing a typeface's rendering.

For the external hinter's behavior and installation, see the
[ttfautohint manual](https://freetype.org/ttfautohint/doc/ttfautohint.html).
