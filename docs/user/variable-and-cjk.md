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
compatibility repair. The glyph inspector has the same live preview for its
current glyph, beside the editable source. Scrubbing does not change outlines.
Repair aligns contour ordering, direction and starting points in master, brace,
smart and matching bracket source groups. It does not add or delete points;
topology differences require manual editing.

## Supplemental sources and smart components

In a glyph's Layers panel, **Add variation source** duplicates the active layer
as an intermediate, alternate or local smart source. An intermediate source
inherits omitted global coordinates from its associated master. Alternate
bounds are inclusive design coordinates on one axis. Each alternate region
needs a default-master source for variable export. Overlapping regions and
ambiguous overlapping component substitutions are rejected.

**Smart component axes** defines named local axes on the reusable base glyph.
Add local source layers for the required neighboring corners in each global
master. The Components panel exposes parameter values for each use; omitted
values use the base glyph's defaults. Nested resolution and decomposition use
the same evaluated geometry. Missing corners, incompatible topology, cycles
and out-of-range values report errors.

FCF keeps all source roles. Glyphs supports native brace/bracket and endpoint
smart-pole data. UFO preserves local sources and parameters. Designspace exports
single-axis bracket conditions as real alternate GLIFs and native `rvrn` rules.
Rules that cannot be represented as editable bracket layers remain preserved
for source round trips and explicitly block preview/binary compilation.
These include compound multi-axis conditions, chained substitutions,
late/custom-feature processing and alternates with supplemental sources.

## Export

In Export, choose **Variable font**, select the source masters and retain the
default master. TTF produces `gvar` outlines; OTF produces CFF2. WOFF and WOFF2
wrap variable TrueType. A named static instance interpolates outlines, metrics,
anchors and kerning and writes instance naming/style metadata. Point-indexed
source hint programs are discarded when interpolation changes their input.

The CLI uses the same kernel:

```sh
fontcarver build Family.designspace Family-VF.ttf --variable
fontcarver build Family.designspace Family-VF.otf --variable
fontcarver build Family.designspace Family-Medium.ttf --instance 0
fontcarver build Family.designspace Family-Subset.ttf --variable \
  --source-master Regular --source-master Bold
```

Variable sources use compatible topology and bounded source counts. CFF2
charstrings obey the 65,535-byte limit and supported operand-stack limits.
Resource or representation failures are reported instead of producing a
static replacement for a requested variable font.

## Vertical metrics and text

Edit vertical advance and origin in the glyph inspector. **Use master vertical
metrics** clears per-layer overrides; an explicit zero remains an override.
Master defaults and `vhea`/horizontal metrics are in the design-space editor.

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
