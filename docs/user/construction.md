# Linked construction

Four kinds of live relationship let one drawing follow another: reusable glyph
attachments, guides that own nodes, servant nodes and smart components. All four
are stored in the document, re-evaluated whenever what they depend on changes,
and baked into ordinary outlines on export.

## Reusable glyph attachments

A serif, a terminal, a repeated texture or a decorative segment can live in its
own glyph and be attached to many carriers. Select the carrier nodes in the Edit
tab, then open **Linked construction → Edit glyph attachments**.

| Kind        | Attaches to                        | Placement                                                       |
| ----------- | ---------------------------------- | --------------------------------------------------------------- |
| **Corner**  | one interior on-curve node         | source origin at the node, +x along the outgoing tangent        |
| **Cap**     | one endpoint of an open contour    | source origin at the endpoint, +x pointing out of the contour   |
| **Segment** | two on-curve nodes of one contour  | a straight source baseline mapped onto the chord between them   |
| **Brush**   | one node, and the contour it is on | the source repeated by arc length along the rest of the contour |

Source glyphs are conventionally named `_corner.`…, `_cap.`…, `_segment.`… and
`_brush.`…, and the editor offers the glyphs with the right prefix for the kind
you chose. The prefix is a convention, not a rule: any glyph can be attached,
and one named otherwise is simply pointed out, so a document that predates the
convention keeps working.

**Keep carrier contour** decides whether the carrier itself is still drawn. A
brush along an invisible spine draws only its stamps; a corner on a visible
stem draws both.

**Preview** evaluates the whole layer. **Keep live** stores the attachment, so
editing the source glyph or moving the carrier node updates every carrier that
uses it. **Bake to outlines** replaces the layer with the evaluated drawing and
clears the attachments.

Attachments are checked before they are stored: a missing source glyph, a
reference cycle, an off-curve node, a corner on an endpoint, a cap on a closed
contour or a segment across two contours is refused and the document is left
alone. Renaming a source glyph follows through into every attachment, and a
source glyph cannot be deleted while something attaches to it.

Placement is affine. A corner or cap rotates to the carrier tangent, a segment
scales to the chord, a brush follows arc length; none of them bend the source to
the carrier's curvature and none trims the carrier. Work is bounded to 128
attachments per layer, 1024 brush stamps and 65,535 evaluated path elements.

## Node-linked guides, and the Magnet tool

An ordinary layer guide can own outline nodes. Moving the guide then moves every
node it owns, keeping each node's signed distance from the guide — which is what
keeps a row of stem feet, serif heights or a crossbar consistent while they are
still editable individually.

The **Magnet** tool (`M`) is the direct way to make that relationship: press a
layer guide, drag it, and every on-curve node within the magnet radius of where
the guide now lies is marked and will follow it. Release to commit; the guide's
move and the links land as one undo step. Hold **Alt** to project the caught
nodes onto the guide first, so they sit exactly on it instead of keeping their
offsets. **Escape** abandons the drag.

Only a layer guide can own nodes; the tool says so when a master guide or a
locked guide is pressed. A locked guide refuses every change — linking,
projecting, unlinking, moving and deleting — until it is explicitly unlocked,
whichever route the change comes from. The same relationships can be made from the inspector
instead: select nodes, choose a guide under **Linked construction**, then
**Link to guide** or **Snap and link**. **Unlink nodes** releases a guide's
nodes where they stand.

A node can follow at most two guides, and they must not be parallel — two
non-parallel guides fix a point exactly. A guide that owns nodes cannot be
deleted until they are unlinked.

### Automatic stem detection

**Detect stems** looks for pairs of straight, axis-aligned edges that bound ink
rather than a counter, up to the **Maximum stem width** you give, and reports how
many it found. **Add linked stem guides** then creates one centre-line guide per
stem, owning both edges' nodes, so the whole stem moves together and keeps its
width. Candidates come from straight edges; curved boundaries are sampled only
to decide what is ink, and a very complex outline returns no candidates.

## Servant nodes

A servant node takes one coordinate from two other nodes. Select one interior
on-curve node and press **X servant** or **Y servant**: the node's current
position is recorded as a fraction of the span between its two neighbouring
independent nodes, and from then on that coordinate is interpolated. Its
adjacent handles follow the same span, so the curve keeps its shape. Press the
same button again to release the axis, leaving the node where it is.

Ratios outside 0 to 1 are allowed, so a servant can sit beyond its boundaries.
Each coordinate has at most one owner, a cycle is refused, and a node that is
owned by a guide cannot also be a servant.

## Smart components

A glyph with local axes — a CJK radical that has to be narrow in one place and
wide in another, for example — is drawn once with one source per corner of its
axis grid, and every component that uses it carries its own coordinates.

Select a component row in the inspector to edit those coordinates numerically,
with a slider per axis and a two-axis plane. The same component also shows one
axis track per local axis on the canvas: drag the handle along its track to set
that instance's coordinate. The first four axes get a track, the rest stay in
the inspector, and every drag is one undo step. A locked component shows no
tracks.
