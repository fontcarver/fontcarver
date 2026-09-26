# Color palettes and layered glyphs

Open **Font Info** (`Ctrl+I`, or `Cmd+I` on macOS), then choose **Color fonts**.
The editor supports COLR v0 compositions and CPAL palettes. Each composition
paints existing outline glyphs in order; edit those outlines in the glyph editor.

## Palettes

Choose a palette, edit its color swatches, or enter `#RRGGBB` / `#RRGGBBAA`.
The final two digits control opacity: `00` is transparent and `FF` is opaque.
**Duplicate palette** creates another palette with the same entry indices.
**Add entry to all palettes** extends every palette together. An entry can be
removed only when no layer uses it; later indices are adjusted automatically.

Palettes share their entry count and indices, so changing the selected palette
changes the preview without changing the composition. Entries are shown in
pages of 16. Large palettes use an integer entry field and a separate
**Text foreground** checkbox for layer paint. More than 256 palettes use an
integer palette selector. CPAL v1 names and flags survive editing and export;
dedicated controls for these metadata fields are not included yet.

## Compositions and preview

Enter an existing glyph name and choose **Add layered glyph**, or select a
composition from the filtered list. Each layer names an existing outline glyph
and chooses a palette entry or **Text foreground**. Layers paint from bottom to
top: layer 1 is the bottom layer. The movement buttons change this paint order.
Removing a composition leaves the underlying glyphs intact.

The preview uses the default master's monochrome outlines and the selected
palette. The foreground swatch changes only the preview's text foreground.
Transparent colors blend with preceding layers. A reference to the base glyph
itself is valid; nested COLR compositions are not recursively painted. Stacks
are edited in pages of 16 layers. Preview is limited to 1,024 layers.

The list displays at most 2,000 matching composition names; narrow the filter
to reach other names in a large font. Font grid and text previews currently
remain monochrome.

## Apply, undo and save

**Apply changes** commits the palette and composition edits as one undo step.
**Close**, Escape and clicking outside the dialog apply a valid draft before
closing. Save and document handovers also flush a valid draft. Invalid color
text or missing glyph references remain visible and block the handover.
Correct them or choose **Discard and close**. Discard abandons only the current
unapplied draft. If the document changed elsewhere, reload before applying.

Use document Undo and Redo after closing the editor. Save as `.fcf` or `.fcf.gz`
to retain editable color data. Glyph rename, duplication and deletion maintain
composition references and participate in undo. **Delete color data** removes
both COLR and CPAL when applied; Undo can restore them.

## Import and export

Supported COLR v0 / CPAL v0 or v1 data becomes editable on import. Unsupported
or malformed tables stay together as original bytes with a diagnostic. Older
native documents with original tables can offer **Make imported colors
editable**. Replacing or deleting original data is an explicit edit.

TTF, OTF/CFF, WOFF and WOFF2 export the editable data using the exported glyph
order. References to excluded glyphs are omitted with a warning. Selecting
COLR in table selection also includes CPAL. UFO and designspace store color
data in FontCarver's private metadata; other editors may ignore that metadata.
Formats without color preservation report the loss.

This editor covers static COLR v0 compositions. COLRv1 paint graphs, SVG color
documents, variable color authoring and bitmap-color authoring are separate
features. Highly overlapping imported ranges may exceed the writer's 16-bit
packing limits after editing and produce an explicit export error. The browser
development transport accepts request bodies up to 64 MiB; model item limits
apply independently.
