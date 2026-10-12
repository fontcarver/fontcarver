# COLRv1 paint graphs

Open **File → Font Info → Color fonts**. Palettes are shared with the
[COLR v0 editor](color.md). The COLRv1 section edits static solid fills,
linear/radial/sweep gradients, layers, glyph clips, named color-glyph references,
affine/translate/scale/rotate/skew transforms, and all 28 composite modes.

Enter an existing glyph name and choose **Add gradient** to create a two-stop
linear fill clipped to that glyph. This replaces that glyph's previous root;
other roots remain. Edit **Paint graph JSON** for the complete graph. Apply is
one undoable transaction. Invalid JSON, fields, missing references, cycles,
unbounded roots and excessive work are rejected without replacing the draft.
Save and close use the same validation. **Discard and close** abandons the draft.

The graph has three fields:

- `glyphs`: glyph names mapped to zero-based indices in `paints`.
- `paints`: shared nodes with a `kind` and the fields described below.
- `clips`: optional glyph names mapped to `[xMin, yMin, xMax, yMax]` boxes.

| Kind | Fields |
| --- | --- |
| `Layers` | `paints`: indices in bottom-to-top order, at most 255 |
| `Solid` | `palette_index`, `alpha` |
| `Linear` | `color_line`, `points`: origin, end, perpendicular-direction point |
| `Radial` | `color_line`, `centers`: two `[x,y]` pairs, `radii`: two unsigned radii |
| `Sweep` | `color_line`, `center`: `[x,y]`, `angles`: start/end degrees |
| `Glyph` | `glyph`: outline name, `paint`: child index |
| `ColrGlyph` | `glyph`: another root name |
| `Transform` | `paint`: child index, `transform`: tagged transform |
| `Composite` | `source`, `backdrop`: indices, `mode`: OpenType mode 0–27 |

A color line has `extend` (0 pad, 1 repeat, 2 reflect) and `stops` containing
`offset`, `palette_index` and `alpha`. A null palette index uses the text
foreground; other indices address the shared palette. Alpha is 0–1 and multiplies
the palette alpha. Stops retain their source order, including coincident stops.
Deleting a palette entry used by any graph node is blocked. Removing an unused
entry renumbers higher indices in both graph nodes and v0 layers.

Transforms use `kind`: `Affine` has `matrix` in `[xx,yx,xy,yy,dx,dy]` order;
`Translate` has `offset`; `Scale` has `scale` and `center`; `Rotate` has `angle`
and `center`; `Skew` has `angles` and `center`. Coordinates use font units with
Y up. Binary export quantizes affine values to 16.16 and gradient offsets,
alpha and scale to F2DOT14. Rotation/skew degrees are stored in fractions of
180; sweep degrees use OpenType's biased encoding.

The preview uses the default master and selected palette. Enter a root name to
inspect another glyph and change the foreground swatch to check foreground
references. It compiles an isolated font and uses the browser's COLRv1 renderer;
a browser without COLRv1 support displays an error. Preview is limited to 2,048
reachable paints, 1,024 outlines, 100,000 path elements and an 8 MiB font. The
preview never changes the document. The grid and text tab remain monochrome.

Graphs allow at most 65,535 paint nodes, depth 64, one million stored items and
one million expanded root operations; encoded COLR is limited to 32 MiB. The
source editor accepts up to 4 MiB of JSON. Every root needs finite bounds from
its paint graph or a clip box. Renames update references, duplicates share the
root, and deleting an outline removes dependent color roots atomically. Export
exclusions drop affected roots with the existing color-reference warning.

Native `.fcf`/`.fcf.gz` and UFO/designspace metadata retain the graph. TTF, OTF,
WOFF and WOFF2 exports remap names to the actual glyph order, preserve v0
fallbacks and rebuild COLR/CPAL together. Variable paints, variable clip boxes,
variation stores and malformed imports keep their original tables and are not
partially decoded. Variable paint authoring and bitmap-color authoring remain
outside this editor. [SVG documents](svg.md) have a separate editor.
