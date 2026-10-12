# SVG color glyphs

Open **File → Font Info → SVG color glyphs**. Each document contains SVG source
and one or more bindings such as `A = art`, where `A` is an existing font glyph
and `art` is an element ID in that document. A glyph can belong to only one
document. Multiple glyphs can share an element or shared definitions.

**Create SVG document** supplies a small editable triangle with ID `art`; enter
a glyph binding before applying it. **Add document** adds independent artwork.
Use the document number to navigate large collections. The preview glyph-name
field selects a binding from the current document. SVG coordinates have positive
Y pointing down; artwork above the font baseline usually uses negative Y values.

**Apply changes** makes one undo step. **Close** applies valid pending changes;
an error keeps the draft available for correction. **Discard and close** discards
the editor draft. Document Save also flushes the editor, and rejects invalid
bindings or XML. Deleting an SVG table is explicit and undoable. Save as FCF or
FCF.GZ to retain the entire editable document; UFO export also stores the source
and ownership metadata.

TTF, OTF, WOFF and WOFF2 exports write SVG records using the compiled glyph order.
Renaming or reordering glyphs keeps the bindings correct. Excluding a glyph from
export drops only its binding and produces a warning. Shared definitions remain
available to other glyphs. Other export formats report when they omit SVG data.

The editable subset supports static paths, rectangles, circles, ellipses, lines,
polygons, groups, local `use` references, gradients, clipping, masks and patterns,
with presentation attributes or simple inline CSS. References must use local IDs.
Stylesheets, animations, embedded raster images, filters, external resources,
DTD/entity declarations, scripts and foreign content are outside the subset.
Unsupported imported SVG tables remain preserved original data; the editor shows
the reason and offers explicit replacement or deletion.

Documents are limited to 2 MiB each and 32 MiB in total. XML nesting and reference
expansion are bounded; cycles and missing or duplicate IDs are rejected. Shared
artwork duplicated for export must also fit the output budget.

The editor preview uses the browser's isolated SVG image mode. Text and glyph-grid
views still render monochrome outlines. SVG palettes use any fallback colors in
the source; the preview does not inject the font's CPAL palette. Native desktop
input and cross-platform SVG rendering are not covered by this browser workflow.
