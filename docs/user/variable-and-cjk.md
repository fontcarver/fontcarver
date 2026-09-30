# Variable fonts and CJK editing

## Axes, masters and instances

Open **File → Font Info** (`Ctrl+I`, or the platform equivalent), choose
**Axes / Instances**, then **Edit design space**. Define four-character axis
tags, user-coordinate ranges and optional monotonic coordinate maps. Master
and instance locations use mapped design coordinates. One master must remain
at the mapped default, and source positions must be distinct.

Adding a master copies the first master shown when the dialog was opened,
including its local smart sources. Rename axes in this dialog to migrate brace
and bracket references atomically. Apply changes before previewing; failed
writes keep the draft. Close applies the draft, while Discard and close reloads
the saved document state. Document save flushes a pending design-space draft.

The preview offers named instances, axis sliders, master overlays and a safe
compatibility repair. Each axis has a number field beside its slider and a
reset button that returns that one axis to its default. The slider and the
field step in whole axis units — refined only as far as a named location needs,
so a width axis carrying 87.5 steps by 0.5 — and the field also takes a typed
coordinate, which is clamped to the axis range. Shift multiplies the step by
ten, so ten presses move a weight axis from 400 to 500 exactly. The glyph inspector has the same live preview for its
current glyph, beside the editable source: the axis sliders scrub an
interpolated overlay on the editing canvas continuously, and scrubbing never
changes outlines.

The compatibility view colours and numbers shapes by **shape order**, which is
the interpolation contract, so the same colour is the same shape in every
source. Connectors join the corresponding start points and shape centres across
sources; crossing connectors mean two compatible shapes are matched in the wrong
order — the "shape shifter". Segments are coloured green when they agree, yellow
when one source's direction differs by more than 20 degrees (usually a missing
node) and red when the structure cannot interpolate.

Fix a shape shifter by dragging one shape's handle onto another in the source
the **Source to reorder** picker names, or with its _Move shape earlier_ and _Move
shape later_ buttons. Either gesture is one undo entry and reorders only that
source. **Repair source compatibility** aligns contour ordering, direction and
starting points in master, brace, smart and matching bracket source groups. It
does not add or delete points; topology differences require manual editing.

## Supplemental sources and smart components

In a glyph's Layers panel, **Add variation source** duplicates the active layer
as an intermediate, alternate or local smart source. An intermediate source
inherits omitted global coordinates from its associated master.

An alternate layer's condition is a **set** of inclusive design-coordinate
ranges, one per axis, and the layer applies where every range holds — the
OpenType `ConditionSet` model, written `[80‹wght, 25‹wdth‹50]`. The glyph
inspector's **Conditional sources** section lists one row per alternate region
with the sources that share it; _Edit condition_ adds and removes axis ranges
and moves every source of the region together in one undo entry. _Add
conditional source from this layer_ creates a new region from the layer being
edited. Each alternate region needs a default-master source for variable export.
Overlapping regions and ambiguous overlapping component substitutions are
rejected; two regions that differ on any axis are disjoint and both compile.

**Smart component axes** defines named local axes on the reusable base glyph.
Add local source layers for the required neighboring corners in each global
master. The Components panel exposes parameter values for each use; omitted
values use the base glyph's defaults. Nested resolution and decomposition use
the same evaluated geometry. Missing corners, incompatible topology, cycles
and out-of-range values report errors.

FCF keeps all source roles. Glyphs supports native brace/bracket and endpoint
smart-pole data, including multi-axis `axisRules`. UFO preserves local sources
and parameters. Designspace exports bracket condition sets, including compound
multi-axis ones, as real alternate GLIFs and native `rvrn` rules, and reads them
back into editable alternate layers. Rules that cannot be represented as
editable bracket layers remain preserved for source round trips and explicitly
block preview/binary compilation. These include chained substitutions,
late/custom-feature processing and alternates with supplemental sources.

## Export

In Export, choose **Variable font**, select the source masters, axes and named
instances, and retain the default master. TTF produces `gvar` outlines; OTF
produces CFF2. WOFF and WOFF2 wrap variable TrueType. A named static instance
interpolates outlines, metrics, anchors and kerning and writes instance
naming/style metadata, including the per-instance export overrides set in Axes /
Instances. An overridden preferred family or subfamily name (name IDs 16 and 17)
is also written into the full font name (name ID 4) and, in CFF, its FullName;
overriding only the subfamily still writes the family, because the typographic
pair is not valid half-written. Point-indexed source hint programs are discarded when interpolation
changes their input.

Reducing the design space really reduces it: each axis is clamped to the span
its remaining masters cover, an axis whose masters no longer differ is dropped,
named instances outside the retained space are not written, and alternate-layer
condition ranges on a dropped axis are removed. Every one of these is reported
as an export warning. A subset that leaves no varying axis, or no master at an
axis default, is refused with the axis named.

Save a design-space subset as a named **variable-font configuration** to reuse it
from the export matrix and the command line; see
[exporting fonts](export.md).

The CLI uses the same kernel:

```sh
fontcarver build Family.designspace Family-VF.ttf --variable
fontcarver build Family.designspace Family-VF.otf --variable
fontcarver build Family.designspace Family-Medium.ttf --instance-name Medium
fontcarver build Family.designspace Family-Subset.ttf --variable \
  --source-master Regular --source-master Bold
fontcarver build Family.fcf --profile VF --variable-config "Text VF" --out-dir dist
```

Variable sources use compatible topology and bounded source counts. CFF2
charstrings obey the 65,535-byte limit and supported operand-stack limits.
Resource or representation failures are reported instead of producing a
static replacement for a requested variable font.

## Vertical metrics and text

Edit vertical advance and origin in the glyph inspector. **Use master vertical
metrics** clears per-layer overrides; an explicit zero remains an override.
Master defaults and `vhea`/horizontal metrics are in the design-space editor.

**TSB** is the distance from the top of the resolved outline to the vertical
origin. Editing it changes the origin and retains the current advance.
**BSB** is the remaining space below the resolved outline within that advance.
Editing it changes the advance and retains the origin. These fields include
component bounds and are disabled for glyphs without an outline. Resetting to
master metrics also resets the derived sidebearings. CFF exports write edited
origins into `VORG` and recompute vertical sidebearings from the compiled ink.

Choose top-to-bottom direction in text preview. `vert`/`vrt2` alternates and
`vkrn` are enabled according to the feature controls; explicitly disabled
features stay disabled. Vertical kerning uses the same group/exception model
as horizontal kerning. Negative values tighten the pair. Renaming or deleting
a shared group updates both directions. Manual feature source retains its
ownership of a manually declared `vkrn` feature.

## Unihan, spacing and CID

The font sidebar offers Kangxi radical, total-stroke range, reading prefix and
charset filters. All criteria must match one encoding of a glyph. The glyph
inspector shows original Unicode 17.0 Unihan readings and radical alternatives.
Tone-insensitive search does not alter displayed source values. Radical/stroke,
reading and numeric CID sorts are available.

The charset choices include actual GB 2312, GB 18030-2022 Level 1, Big5,
JIS X 0208, KS X 1001 and Unihan IICore repertoires. These are glyph-coverage
filters, not font certification or legacy encoding assignments.

Select glyphs, then **Batch spacing**. Preview the captured selection, choose
one or all masters, and review width/centering/vertical changes before applying.
Changing an option requires a new preview. A document change makes an older
preview stale. The entire operation is one undo entry; component bases are
positioned before selected composites.

**Font Info → CID mapping** edits registry, ordering, supplement and explicit
`CID glyphName` lines. Identity auto-fill is available only for Adobe-Identity.
CID 0 belongs to `.notdef`; duplicate IDs or glyph assignments are invalid.
Optional CID ordering places mapped glyphs numerically and keeps unmapped
relative order. CID-keyed CFF export requires complete mappings for exported
glyphs. Rename/delete and undo maintain CID references.
