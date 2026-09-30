# Interchange formats

FontCarver can open SVG font sources, single-glyph EPS outlines, AFM/PFM metric
files, raw-table TTX, bare CFF1, Type42 and TTC/OTC collections through the same
session API and command-line reader. Use `fontcarver convert INPUT OUTPUT` to
choose an output by extension, or the application's export format selector.

- SVG is the SVG 1.1 `<font>` format, with horizontal glyph outlines and scalar
  Unicode mappings. It is separate from embedded SVG color glyph documents.
  Imported path commands are M/L/H/V/Q/C/Z, including relative/repeated forms.
  Unsupported shaping, vertical, external and drawing semantics are rejected.
- EPS contains one filled outline. Export requires `--glyph NAME`; the importer
  accepts a declarative path subset and never executes PostScript. Arbitrary
  EPS artwork, transformations, strokes and executable programs are rejected.
- AFM/PFM are metrics-only formats. Opening one creates glyphs with widths and
  empty outlines and reports a warning. They cannot store a complete source font.
  AFM retains supported global/character metrics and pair kerning; PFM has the
  documented Windows metric and encoding limitations in its codec.
- TTX exports complete binary tables as `raw="True"`/`hexdata` XML understood by
  fontTools. This preserves unedited tables without a table-specific XML parser.
  Decompiled table XML and split/external references are rejected explicitly.
  Use fontTools to compile decompiled TTX before opening it here.
- Bare CFF accepts one CFF1 font. It has no OpenType character map or layout
  tables; Unicode is inferred only when glyph names are unambiguous. CFF2 and
  multi-font CFF sets require their surrounding OpenType context.
- Type42 wraps TrueType sfnt data in a bounded PostScript font resource. The
  importer validates its supported declarative wrapper without execution.
- TTC/OTC export retains each face independently. Glyphs are never merged or
  renumbered, and table sharing is not attempted. Collection version 1 is used.

To collect source documents or existing fonts in order:

```sh
fontcarver collection --output Family.ttc Regular.ufo Bold.ufo
fontcarver inspect Family.ttc --face 1 --json
```

A TTC/OTC input to `collection` contributes all its faces. The API equivalent is
`session.export` with `options.format: "Ttc"` and `collection_docs` holding the
additional open document identifiers after the primary `doc`.

TTX and collection reconstruction reject DSIG signatures because repacking
invalidates them. They accept at most 128 MiB of decoded font data; collection
output is limited to 256 faces. SVG/EPS input/output is limited to 32 MiB. All
codecs reject unsupported syntax before replacing output files. Native `.fcf`
remains the format for full editable document preservation.
