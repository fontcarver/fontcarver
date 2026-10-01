# Bitmap fonts

FontCarver edits bitmap strikes as part of the same document as the outlines:
one `.fcf` holds the drawing, the strikes and the format metadata, every pixel
edit is an undo step beside the outline edits, and a strike is exported to BDF,
PCF, PSF, FNT/FON or embedded in a TTF, OTB, CBDT or sbix font.

Open the editor with **Bitmap ▸ Bitmap editor**.

## Strikes

A strike is identified by its size and depth, written `16px · 1bpp`, and no two
strikes share that pair. Each one carries its own pixel ascent and descent, its
own format properties and its own images, so a 16-pixel strike can be spaced
differently from a 12-pixel one.

- **Create empty strike** adds a strike at the chosen size and depth.
- **Rasterize outlines** fills a strike from the outline master. It works on the
  selected glyph, or on every glyph with **All glyphs**, and takes a monochrome
  threshold and an optional grid-fitting mode (the preserved hint programs, or
  automatic fitting). It never changes the outlines.
- **Delete strike** removes one, and undo brings it back with its pixels.

Depths are 1, 2, 4 and 8 bits of coverage, or 32-bit RGBA for colour strikes.

### Integer pixel grids

**Integer pixel grid** lists every size whose pixel is a whole number of font
units, from 4 pixels per em up to the em itself — for a 1000-unit em that is 4,
5, 8, 10, 20, 25, 40, 50, 100, 125, 200, 250, 500 and 1000, and for a 1024 or
2048-unit em the powers of two in the same range. Above the em a pixel would be
smaller than a font unit, so nothing is offered there. Choosing a size sets it
and the strike metrics the master implies at that size.

Any other size is still usable — most bitmap fonts were never drawn on an exact
grid — but the editor says what it means and offers the nearest exact size
beside it:

> 16 ppem gives 62.5 font units per pixel, which is not a whole number, so pixel
> edges will not land on integer coordinates. The size is still usable; most
> bitmap fonts were not drawn on an exact grid. **Use 20 ppem instead**

An exact grid matters when pixels will be traced back into outlines: on an exact
grid every pixel edge is an integer coordinate.

## Drawing

The tool strip above the canvas holds the pencil, eraser, line, rectangle,
ellipse, fill, marquee, eyedropper and hand. Every tool is also in the **Bitmap**
menu and the command palette, with a single-key shortcut:

| Tool       | Key | Behaviour                                                          |
| ---------- | --- | ------------------------------------------------------------------ |
| Pencil     | B   | Toggles the first pixel, then paints every later pixel to match it |
| Eraser     | E   | Clears pixels                                                      |
| Line       | L   | Drag from one end to the other                                     |
| Rectangle  | R   | Outline of the dragged rectangle                                   |
| Ellipse    | O   | Outline of the ellipse inscribed in the dragged rectangle          |
| Fill       | G   | Flood fill of the connected area with the same value               |
| Select     | M   | Marquee; drag inside it to move, hold Alt to stamp a copy          |
| Eyedropper | I   | Reads the value under the pointer into the ink, without editing    |
| Hand       | H   | Pans the canvas; the arrow keys pan too                            |

The coverage slider sets the ink; an RGBA strike adds a colour well. Arrow keys
move the selected pixels, or the whole image when nothing is selected, and the
toolbar under the canvas flips, rotates, crops, inverts, scales and
copies/cuts/pastes. The clipboard carries pixels between glyphs of the same
depth.

## What the canvas shows

Four switches above the canvas, each also a command in the **Bitmap** menu, and
each remembered for the next session:

- **Pixel grid** — one line per pixel.
- **Outline overlay** — the master's outline for this glyph, drawn at the
  strike's scale, so pixels can be checked against the shape they come from.
- **Advance column** — shades the column from the origin to the advance width,
  which is the space the glyph claims in a line of text.
- **Font units** — adds font units beside every pixel readout.

Beside them the editor states the advance in pixels and units, the units per
pixel, and the pointer's position and drag delta, for example
`X 5, Y 1 px (625, 125 units) · ΔX 3, ΔY -2 px (375, -250 units)`. Pixel
coordinates are y-up from the glyph origin, like the rest of the editor.

## Between strikes

**Copy between strikes** copies the selected glyph, or every image in the
strike, into another size and depth in one undo step, creating the target strike
if it does not exist. With **Scale pixels and metrics to target size** the
pixels, origins, advances and strike metrics are scaled by the size ratio, with
a choice of filter:

- **Nearest pixel** keeps hard edges and is what a pixel font wants.
- **Area average** averages the source over each destination pixel, which needs
  a deeper target: into a 1-bit strike the average is thresholded back to ink or
  nothing, while an 8-bit target keeps the grey.

The waterfall at the bottom draws the sample text at every strike size, using
each strike's own pixels and advances. **Show only _glyph_ at every size** swaps
the sample for the selected glyph, which is the comparison to make while drawing
the same letter at several sizes. It names the glyph rather than spelling it, so
a glyph with no Unicode value — `.notdef`, a stylistic alternate, the reusable
`pixel` glyph — can be compared the same way. The waterfall does not shape OpenType layout
and never scales one strike to stand in for a missing one.

## Pixels and outlines

**Replace outline with pixel trace** turns the strike's pixels into contours in
the selected master, with the advance scaled from the strike. **Every image in
this strike** traces the whole strike in one undo step. Four corner treatments:

| Mode              | Result                                                          |
| ----------------- | --------------------------------------------------------------- |
| Sharp             | Exact pixel edges, with collinear points removed                |
| Chamfer           | Each corner cut back by a quarter of a pixel                    |
| Round             | Each corner replaced by a tangent cubic quarter circle          |
| Smooth staircases | Staircase turns cut diagonally; isolated rectangles stay square |

**Join diagonal pixels** adds a quarter-pixel bridge wherever two pixels touch
only at a corner, which makes the foreground eight-connected; without it the two
pixels stay separate shapes. Outer contours are clockwise and holes run the
other way, so the result fills correctly in every exporter.

**Editable pixel components** is the other direction of the same idea: each
inked pixel becomes a component of one reusable glyph (`pixel` by default) that
is a square the size of the em, placed and scaled to one pixel cell. Edit that
glyph in the outline editor — round its corners, cut a notch in it — and every
pixel of every glyph built this way follows. On export, **Expand components into
contours** decides whether the font keeps the composites (smaller, and the pixel
glyph travels with it) or writes each glyph as plain contours.

**Recover a pixel-font strike** goes the other way again, for a pixel font that
arrived as outlines: it looks for a single exact orthogonal grid across every
glyph of the master and offers to rebuild the strike from it. Detection is
deliberately conservative — 4 to 256 pixels per em, at least three different
staircase shapes, every outline axis-aligned on the same grid, no curves,
effects or smart layers — so an ordinary text font is never mistaken for one.

## Format metadata

**Strike metrics and BDF properties** edits the strike's ascent and descent and
its property dictionary. Standard XLFD names (`FOUNDRY`, `RESOLUTION_X`,
`CHARSET_REGISTRY`, …) are offered with their declared type, which the editor
then holds; a name outside the list is a custom property whose type you choose
and can change later. Each property can be edited in place, renamed or removed.
`FONT_ASCENT`, `FONT_DESCENT` and `PIXEL_SIZE` are marked _written from the
strike_: the BDF writer derives them, so editing them by hand has no effect on
the exported file.

A property name is one token with no spaces, and a string value is one line,
because that is all BDF, PCF and FNT can store. The editor refuses anything
else before it reaches the document, rather than letting the export fail later.

## PNG sprite sheets

**PNG sprite sheet** exports the strike as a grid of fixed cells and imports one
back. List the glyph names in cell order, set the cell size and the number of
columns, and **Prepare PNG export** produces the file. The exported PNG carries
its own layout in a text chunk, so importing it back restores the cells, the
columns and the glyph order without retyping them; the same layout is shown as
copyable text beside the download.

Each image sits at the bottom left of its cell. An import keeps every existing
image's bounds, origin and advances and replaces only the pixels, so resize a
glyph first if the new artwork is larger than the old one; ink outside an
existing image's bounds, or in an unassigned cell, is refused rather than
silently cropped. For a 1- to 8-bit strike the PNG's alpha channel is the
coverage; an RGBA strike keeps the colours. Everything the sheet names is
replaced in one undo step, and no other glyph is touched.

## Exporting

| Output                 | What it carries                                                           |
| ---------------------- | ------------------------------------------------------------------------- |
| BDF, PCF, PSF, FNT/FON | One strike (FON writes every representable strike), with its properties   |
| OTB                    | A bitmap-only OpenType font: EBDT/EBLC with empty outlines                |
| TTF, OTF, WOFF, WOFF2  | The outlines plus every editable strike, as EBDT/EBLC (and CBDT for RGBA) |
| CBDT, sbix             | Colour strikes only, as PNG images                                        |

`fontcarver convert` reaches the same paths from the command line, with
`--ppem` and `--depth` choosing the strike for a single-strike output:

```sh
fontcarver convert source.bdf edited.fcf
fontcarver convert edited.fcf output.pcf --ppem 16 --depth 1
fontcarver convert edited.fcf output.otb
fontcarver convert console.bdf console.psf --psf1 --ppem 8 --depth 1
```

Interchange formats cannot hold a complete editable document: keep the `.fcf`
beside whatever you export.
