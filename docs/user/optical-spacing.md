# Optical spacing and automatic kerning

Select up to 128 glyphs in the font grid, then choose **Optical spacing**. Choose
sidebearings or automatic kerning, the current master or all masters, and a
**Density** from 0.25 to 4. Higher density produces tighter spacing.

**Preview** shows proposed sidebearings and advances, or the effective existing
and proposed kerning values. Review the changes, then **Apply** records the batch
as one undo step. Changing the options discards the preview. A document edit
made after preview requires another preview before application.

## Accepting and rejecting single proposals

The preview lists every proposed change in a review table: the glyph or the
pair, its master, and the values before and after. Each row has an **Accept**
box, ticked to begin with. Clear a row to leave that layer or that pair
unchanged. **Accept all** and **Reject all** act on every row, not just the ones
on screen; a large proposal is shown a window at a time, with **Show more rows**.

Applying sends exactly the accepted rows, so spacing one glyph of a selection or
taking half the proposed pairs is one undo step like any other. A row the kernel
no longer proposes — because the document changed after the preview — is refused
by name and nothing is applied, so a review of a stale document cannot write
something it did not show. Clearing every row disables **Apply**.

Spacing measures the resolved outlines, including components and live geometry.
It compares the left and right curve silhouettes to a density-scaled space
budget, so round and slanted edges can receive different sidebearings from
rectangular edges. Selected component bases are processed before their dependent
composites. The operation preserves components and outline shapes.

Automatic kerning measures all ordered pairs in the selected glyph set. It
creates concrete glyph-pair exceptions and preserves group membership. Existing
effective kerning, including group rules and explicit zero exceptions, stays
unchanged unless **Replace existing kerning** is enabled. Imported layout and
feature-controlled kerning must first become editable through the existing
layout-source workflow.

Locked glyphs, empty outlines and zero-advance marks are skipped. Spacing also
skips glyphs controlled by metric formulas. The preview reports the skipped
sources; pairs with no sampled vertical overlap are skipped too. Operations are
limited to 512 source layers, 16,384 pairs across masters and 16 million segment
samples. Reduce the selection or master scope when a limit is reached.

The algorithm uses 256 horizontal curve samples and an approximate clearance
floor. It provides an initial spacing proposal for manual proofing. It does not
perform continuous collision detection or apply script-specific spacing rules.
