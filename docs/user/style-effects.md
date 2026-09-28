# Batch style effects

Select 1–512 glyphs in a Font tab, then choose **Font → Style effects** or
**Style effects** above the grid. The selection and active master are captured
when the dialog opens. Choose **Apply to all masters** to include every main
master layer of those glyphs.

Choose a starting preset: Embolden, Lighten, Condense, Extend, Oblique, Small
caps, Subscript or Superscript. Every numeric field remains editable. Scale is
applied first, then oblique shear, translation and weight. Presets transform
selected outlines in place; they do not create alternate glyphs or feature rules.

Width and height accept 5–400%. Oblique accepts ±60 degrees. Weight is a boundary
offset in font units: a positive X offset of 20 adds approximately 40 units to
a straight vertical stem. The X/Y offsets must both be zero, or have the same
sign with magnitudes of 0.1–500. Weight requires closed contours. Rounded outer
corners follow the elliptical offset; counters shrink when emboldened and grow
when lightened. Very thin strokes can disappear under contraction.

**Scale advance widths** scales each original horizontal advance by the width
factor. For fixed CJK cells, leave it unchecked and select **Center outlines**.
Centering occurs after all other effects. Named anchors follow scale, shear,
translation and final centering; weight does not independently move anchors.

**Preview changes** shows up to twelve affected layers, with before and after
outlines in the same coordinate frame and their advances. The summary covers the
whole batch. Changing a setting invalidates the preview. **Apply changes** checks
that the document has not changed since preview, then records one undo step.
A stale preview or unrecoverable geometry error leaves the document untouched.

Selected components are resolved from the original document and become contours.
Selecting a base and its composite together applies the effect only once to each.
Unselected composites referencing an edited base will reflect that base's change.
**Keep original outlines and anchors in backup layers** is on by default. These
backups hold resolved original drawing geometry and advances, so later base edits
do not change them. They survive native save/reopen. Existing backup-layer tools
can inspect or restore them. The original metadata and component graph remain
available through Undo; drawing backups do not copy images or hint programs.

When strict curve booleans fail independent checks, the operation may union
flattened outlines and fit new curves. The preview names every affected glyph and
master. Review these outlines before applying. Failed reconstruction rejects the
whole batch. Edited geometry follows the existing PostScript hint invalidation
and TrueType instruction binding rules.

All-master effects use identical settings, but weight can change contour and
point counts independently. Check compatibility before variable export. The work
limits are 1,024 source layers, 4,096 source points per layer, 100,000 resolved
points/anchors in the batch and 250,000 output points. Drawing coordinates must
remain within the supported OpenType design grid. This is a baked outline edit;
reusable non-destructive effects are a separate workflow.
