# The outline editor

The Edit tab is where a glyph is drawn. This page covers the tool strip, the
node and modifier grammar, the Path menu, guides, snapping, the background
layer and the always-on outline audit. Shortcuts are written with `Mod`, which
is Ctrl on Linux and Windows and Command on macOS; `Ctrl` always means the
physical Control key.

## The tool strip

| Key  | Tool                 | What it does                                                            |
| ---- | -------------------- | ----------------------------------------------------------------------- |
| `V`  | Select               | Click, Shift-click, Ctrl-click and marquee; drag nodes, handles, segments |
| `Q`  | Lasso                | Freehand region selection; Shift adds, Alt leaves handles out            |
| `P`  | Pen                  | Click for a corner, drag for a curve; click the first node to close      |
| `B`  | Pencil               | Freehand, simplified into cubics; Shift draws straight                   |
| `E`  | Knife                | Drag a line; nodes are inserted at every crossing and the path splits    |
| `⇧E` | Eraser               | Sweep across nodes to delete them in one step; Shift leaves a gap        |
| `F`  | Rectangle            | Drag, or click for the numeric dialog; Shift squares, Alt from centre    |
| `⇧F` | Ellipse              | The same, as a circle                                                   |
| `L`  | Measure              | Drag for distance, angle and every outline crossing                     |
| `R`  | Rotate               | Click to set the origin, then drag                                      |
| `S`  | Scale                | Click to set the origin, then drag                                      |
| `A`  | Annotation           | Text, arrow, circle, rectangle, plus and minus review marks             |
| `Z`  | Zoom                 | Click in, Alt-click out, drag a marquee                                 |
| `H`  | Hand                 | Pan; Space does the same while it is held                               |
| `T`  | Text                 | Switches the tab into shaped-text mode; Escape returns to the glyph      |
| `X`  | Pixel                | Opens the strike editor, where pixel drawing lives                      |

Tapping a key switches tool. Holding it for 200 ms *borrows* that tool until
the key is released, which the strip shows with a dotted outline — useful for
one measurement in the middle of drawing. Every binding is editable in
Preferences → Shortcuts, and a tool can be unbound entirely.

The Pixel tool is a door rather than a drawing tool: choosing it saves whatever
the outline editor has open, opens the bitmap workspace for the same document
and puts the previous tool back, so returning to the Edit tab finds a live tool.

## Nodes and modifiers

- Double-click a node, or press Return, to toggle it between corner and smooth.
  `Alt+Return` does the same without moving the handles, so the outline keeps
  its exact shape.
- A **super-smooth (G2)** node keeps equal curvature on both sides and is drawn
  as a ring. Path → Super-smooth (G2) turns the selected nodes into G2 nodes;
  Path → Harmonize equalises the curvature once without making the node G2.
### The modifier grammar

| Modifier   | Dragging                                  | Arrow keys        |
| ---------- | ----------------------------------------- | ----------------- |
| —          | Move the node with its handles             | 1 unit            |
| `Shift`    | Constrain to 0 / 45 / 90°                  | 10 units          |
| `Mod`      | —                                          | 100 units         |
| `Alt`      | "The other behaviour": node only, keep a handle's angle, marquee without handles, from the centre, all masters | Nudge without handles |
| `Ctrl+Alt` | Nudge: neighbouring unselected handles scale proportionally | —   |
| `Ctrl`     | Suspends snapping for the rest of the gesture | —              |

`Mod` is the platform's command modifier — **Ctrl on Linux and Windows**,
Command on macOS — while `Ctrl` always means the physical Control key. On Linux
and Windows the two therefore coincide: Ctrl is both the ×100 arrow increment
and the snap-suspend key, which is unambiguous because one is a key press and
the other is a modifier held during a pointer drag. On macOS they are separate
keys and the editor keeps them separate.
- Shift-clicking a segment with the Select tool inserts a node at the nearest
  extremum or inflection.

## The Path menu

| Item                              | Shortcut           | Notes                                                       |
| --------------------------------- | ------------------ | ----------------------------------------------------------- |
| Remove Overlap                    | `Mod+Shift+O`      | Whole layer                                                 |
| Correct Direction                 | `Mod+Shift+R`      | Also canonicalises starts and contour order                 |
| Correct Direction in All Masters   | `Mod+Alt+Shift+R`  | One undo entry across every master layer                    |
| Add Extremes                      | `Mod+Shift+E`      | Skips an extremum already within 0.01 units of a node       |
| Force Extremes                    | `Mod+Alt+Shift+E`  | Inserts it anyway                                           |
| Tidy Paths                        | `Mod+Shift+T`      | Canonical start points and contour order                    |
| Tidy Paths in All Masters          | `Mod+Alt+Shift+T`  | One undo entry across every master layer                    |
| Simplify…                          | `Mod+Shift+M`      | Numeric dialog                                              |
| Reverse Contours                  |                    | The contours the selection touches, or all of them          |
| Set Start Point                   |                    | One selected on-curve node of a closed contour              |
| Join Contours                     | `Mod+J`            | Two open ends                                               |
| Break Contour                     |                    | Opens a closed contour, or splits an open one               |
| **Open Corner**                    | `Mod+Alt+O`        | One selected corner node                                    |
| **Reconnect Nodes**                | `Mod+Alt+J`        | The two nodes of an open corner, or two open ends           |

### Open Corner and Reconnect Nodes

Select one corner node and choose **Open Corner**. The node is replaced by two
nodes placed along the two adjacent tangents *past* the corner, joined by a
straight mouth, so the contour crosses itself in a small triangle. Removing the
overlap — on export, or from the Path menu — reproduces the original sharp
corner exactly. This is what keeps a joint sharp while the two strokes that
form it stay separately editable.

The setback is a small fraction of the shorter adjacent segment, at most one
fiftieth of the em, so an opened corner stays recognisable at any upem.

**Reconnect Nodes** is the exact inverse: select the two nodes of the mouth and
they merge into one corner node at the intersection of the same two tangents,
keeping the first node's identity. The same command also joins two open contour
ends, which is the other thing "reconnect" can mean.

Two limits worth knowing. Only a *concave* (reflex) corner can cross its own
contour, so removing the overlap of an opened convex corner leaves the triangle
as a separate ear; the construction is still exactly reversible, which is what
Reconnect Nodes is for. And a node carrying a live smart corner has to have its
radius baked or cleared first, because the radius already owns that join.

## Guides

Local guides belong to one glyph, global guides to a master. Both are listed in
the Guides panel with their position, angle, name and lock, and both can be
dragged out of a ruler with Alt (Shift puts it in the master's global list).
A guide can be turned into a **measurement guide**, which labels the distance
between each pair of outline crossings along it. The measurement switch is a
view setting rather than stored data.

**Anchoring** decides what a guide's horizontal position is measured from:

- *Nothing (absolute)* — the default, and how every imported guide arrives.
- *Left sidebearing* — the stored value is a distance from the left edge of the
  ink, so the guide follows the drawing when the advance or the outline moves.
- *Right sidebearing* — the same from the right edge; a negative value sits
  inside the ink.

Changing the anchor rebases the stored offset, so the line does not jump: what
changes is what it follows from then on. A blank layer has no ink edge, so an
anchored guide there behaves as absolute.

## Snapping

Preferences → Snapping lists nine independent targets: nodes, handles, anchors,
guides, metrics, grid, component ghost nodes, alignment zones and background
points. Each has its own switch **and its own reach in screen pixels**. A
target left at zero uses the shared snap distance below the list, so a
preference file that has never been edited behaves exactly as one radius did.
Approaching a component exposes its ghost nodes so they can be snapped to
without decomposing it. Holding `Ctrl` suspends snapping for one gesture.

The grid itself, its subdivision and the arrow-key increments can belong to the
font rather than to you: when the project stores them, the editor draws that
grid, snaps to it and nudges by those increments in every window that opens the
font, and your preference applies only to fonts that store none.

Hit distance is separate: it is the radius at which the pointer grabs a node,
and the other hit families scale with it. A node or handle marker drawn larger
than that radius is grabbable over its whole face, so raising *Node size* or
*Handle size* in Preferences → Editor makes those points easier to hit as well
as easier to see.

## The background layer

The Layer menu holds the background workflow for the current master: Add
Background, Copy to Background, Copy selected paths to background, Swap with
Background, **Clear Background** and Edit foreground / background (`Mod+B`,
which toggles which of the two the canvas is editing). Clearing empties the
layer and keeps it, so the next copy starts from a clean sheet. Show background
in the Layers panel controls whether it is drawn behind the outline.

## The outline audit

The audit runs continuously on the layer being edited and draws a marker at
every issue; hovering one explains it and clicking it selects the point.
Four checks can be switched on and off independently, and the switches are
remembered between sessions:

| Check                | What it reports                                     | Fix                                          |
| -------------------- | --------------------------------------------------- | -------------------------------------------- |
| Open contours        | A contour that is not closed                        | Joins its two ends, when both are nodes      |
| Fractional coordinates | A point off the integer grid                      | Rounds that point                            |
| Coincident points    | Stored points sharing one position                  | Deletes adjacent duplicate nodes of one contour |
| Missing extremes     | A curve reaching further than any of its nodes      | Inserts a node at the extreme                |

Fix repairs one issue; Fix All repairs everything shown. Both refetch the layer
first and refuse to act on a reading the kernel no longer agrees with, and both
land as a single undo entry. A coincidence spanning two contours, or between a
node and a handle, is ordinary drawing rather than a defect, so it is reported
for review without a fix. Because a structural repair renumbers the points of
its contour — and inserting a node re-parameterises the segment it splits — Fix
All applies one kind of structural change per contour and one extreme per
segment; press it again for the rest.

While an outline is being dragged the audit follows the shape on the canvas
rather than the last saved one, refreshing at least every 250 ms, and Fix stays
unavailable until the kernel has accepted the edit.

## Components

The Components section lists each component with its base glyph, offset and
scale. **Auto-align** places a component by its anchors instead of by a stored
offset: the component's own mark anchor — the one whose name begins with `_` —
is put on the base attachment anchor of the same name, looked for first in the
carrier's own anchors and then in the base glyphs of the components before it.
Only the translation is solved; scale, rotation and skew stay as drawn. Turning
auto-align on places the component in the same undo entry, and **Glyph → Align
Components to Anchors** places every auto-aligned component of the layer again
after the anchors have moved. An auto-aligned component cannot be dragged, which
the canvas shows by hatching it on hover.

## Locking and focus

The Glyph panel can lock a whole glyph, and the Paths section can lock
individual paths or put one path in focus, which dims the others and takes them
out of clicking, marquee, lasso and knife range. Locked components and guides
stop responding without being renumbered, so indexes in the inspector stay
stable. Locks are stored with the document; focus is a session setting.

Pressing a locked path, component or guide says so rather than doing nothing:
the canvas shows the reason once, and the Paths and annotations panel repeats it
beside the controls the lock disables.
