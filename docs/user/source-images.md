# Source images and autotrace

A source image is a PNG placed behind one layer's outline. It belongs to the
layer, so each master — and each alternate or intermediate source — has its own
image and its own placement.

## Importing

Open **Source image** in the glyph inspector.

- **Upload PNG** embeds the file in the document. Nothing outside the document
  is needed afterwards.
- **PNG path on this computer** reads the file through the font server. Leave
  **Embed path image** off to store a link relative to the saved document, which
  keeps the document small and lets the scan be edited outside FontCarver. A
  link needs a saved document; save first.

Limits are 16 MiB per file, 4096 pixels per side and one million pixels. Only
static PNG is accepted; an animated PNG is refused with a message rather than
partly read.

## Broken links

When a linked file cannot be read, the section shows **Broken link**, the
message from the kernel and the absolute path that was tried. The placement, the
lock and the trace settings are all kept.

- **Try again** re-reads the same path, which is what you want after restoring
  the file or reconnecting a drive.
- **Relink the same path** re-imports from the path that failed, which repairs
  the stored link after a save moved the document.
- **Relink to the path above** uses whatever you typed in the import field.

## Placement

The image is placed by a six-coefficient affine transform that maps the image's
own pixel frame, with the origin at its bottom-left corner, into font units.

**Adjust on canvas** turns on direct manipulation:

- drag the image to move it,
- drag a corner to scale, with **Shift** for one uniform factor,
- drag an edge to skew, keeping the opposite edge in place,
- drag the knob above the top edge to rotate, with **Shift** snapping to 15°.

Each gesture is one undo step. For keyboard work, or for an exact placement, the
same section has **Move left/right/up/down**, **Scale**, **Rotate**, **Skew X**
and **Skew Y** with their own step fields, and the six coefficients themselves.
Scale, rotate and skew turn about the image's centre.

**Lock image placement** refuses every change to the image — including a canvas
gesture and a new import — until it is unlocked. A locked glyph refuses image
edits as well, and says so.

## Autotrace

**Trace PNG** converts the image's dark ink into outlines. Pixels are
composited over white, and anything at least as dark as the **Ink threshold**
counts as ink. The source image is never consumed: it stays on the layer after
a trace.

- **Tolerance** is the largest deviation from the pixel edges, in source pixels.
- **Fit curves** fits cubic segments. With it off, the trace keeps straight
  lines and only removes points the tolerance allows.
- **Corner angle** is the turn, in degrees, at which a structural vertex stays
  a corner instead of becoming a smooth node. Zero makes every vertex a corner,
  which returns a polygon; 180 makes the whole ring smooth.
- **Smoothing** low-pass filters the pixel ring before fitting, in source
  pixels. Use it on a noisy or low-resolution scan.
- **Minimum area** drops rings smaller than that many square source pixels,
  which removes scanner speckle. It starts at four square pixels, small enough to
  keep any mark a designer drew; set it to zero to keep every ring.
- **Append to existing outlines** adds the trace to what the layer already has.
  With it off, applying replaces the layer's contours and components.

**Preview trace** reports the node count, the number of curve segments and the
largest deviation from the pixel edges, and draws the result. **Apply trace**
commits that reviewed result as one undo step. Changing any option discards the
preview, and so does an edit to the document, because the kernel checks both the
document revision and a hash of the source bytes before applying.

### What the numbers mean

A one-bit raster of a curve has a flat run of several pixels at every extreme,
so no smooth curve can follow such a ring closer than about one pixel however
many nodes it is given. The tolerance is therefore a target: the trace stops
splitting a segment once splitting no longer reduces its deviation, and the
deviation that remains is the number the preview reports. Raising **Smoothing**
lowers that floor at the cost of fine detail.

If a fitted ring would have changed the drawing — merged two shapes, inverted a
hole or crossed itself — the trace keeps that ring's exact pixel edges and says
so. That is a guard, not a failure: tighten the tolerance or raise the
threshold.

### Limits

Tracing reads one brightness threshold. A photograph, a coloured drawing or a
scan with shading has no single threshold that separates ink from paper, so
tracing one produces the outline of whatever happens to fall below the
threshold. Prepare such an image in an image editor first — flatten it to black
on white — or trace it in several passes with different thresholds and combine
the results. Colour segmentation, posterisation and centre-line tracing are not
implemented.
