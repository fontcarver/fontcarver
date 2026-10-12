# Mathematical typesetting data

Open **Font Info** (`Ctrl+I`, or `Cmd+I` on macOS), then choose **MATH table**.
The editor contains font-wide constants and per-glyph positioning, kerning,
size variants and stretchable assemblies. Values are in font units unless the
label specifies a percentage or pixel correction.

## Constants and pixel corrections

The **Constants** tab groups the OpenType math constants by use: general,
scripts, limits, stacks, fractions, bars and radicals. Expand a group to edit
its values. **Include global constants** controls whether the constants
subtable is present. **Minimum connector overlap** sets the minimum overlap
between adjacent assembly parts when their connectors allow it.

Most constants accept a signed value and optional size-specific corrections.
For example, `12: -2 0 3` means corrections of minus two, zero and plus three
pixels at 12, 13 and 14 pixels per em. Leave the correction field empty to
remove it. The start size is an unsigned 16-bit integer; each delta is a whole
number from -128 to 127. Font-unit values are rounded when the font is exported.

## Glyph records

In **Glyph records**, filter the list and select a name to edit it. To add a
record, enter an existing glyph name and choose **Add record**. Removing a
record removes its MATH data, not the glyph itself.

Each record can have an italic correction, a top accent attachment, an
extended-shape flag and four corner-kerning functions. Enable a corner to edit
its values. **Add height** introduces another interval: heights must increase,
and the final kern value applies above the last height. Values and heights can
also have pixel corrections.

Enable **Vertical construction** or **Horizontal construction** to define
ready-made size variants, assemblies, or both. List variants from smallest to
largest, with their extent in the construction direction. For assemblies, list
parts from bottom to top for vertical constructions or left to right for
horizontal ones. Each part has a full advance, start and end connector lengths,
and an optional **Extender** flag. Extenders may be omitted or repeated.

The requested-size slider previews the current draft without applying it. A
large enough variant takes priority; otherwise the preview assembles parts or
falls back to the largest variant. Alternating colours distinguish parts. The
result reports its actual size and extender count. Discrete part sizes and
connector limits can make the result larger than requested. Short connectors
and the preview's part limit are reported. This previews glyph construction;
it does not lay out a complete mathematical expression.

## Apply, undo and save

**Apply changes** commits the constants and glyph records together as one undo
step and keeps the editor open. **Close**, Escape and clicking outside the
dialog apply a valid draft before closing. **Discard and close** abandons the
current unapplied draft; it does not undo earlier applications.

Invalid numeric or correction text remains visible. Correct it before applying,
closing, switching sections or selecting another record. Document save and
navigation handovers also check the mounted form. A missing glyph reference or
other semantic error is reported by the kernel and leaves the draft intact.
If the document changed elsewhere, the revision check rejects the stale draft;
use **Reload** to discard it and read the current document.

Native `.fcf` and compressed `.fcf.gz` documents preserve editable MATH data.
After closing the editor, use the normal document Undo and Redo commands to
revert or restore an applied edit. Glyph rename updates MATH references; glyph
deletion removes affected records, variants and assemblies. These operations
are undoable.

## Import and export

Supported imported MATH tables become editable automatically. Unsupported or
malformed data stays as original bytes with a diagnostic. An older native
document with raw data offers **Make imported MATH editable** when decoding is
possible. Replacing or deleting that raw table is an explicit edit.

TTF, OTF/CFF, WOFF and WOFF2 exports write editable MATH with the exported glyph
order. The same writer supports variable exports and static instances; MATH
values are font-wide and are not interpolated between masters. Ordinary Device
corrections are supported; VariationIndex corrections are not editable.

UFO export preserves MATH in FontCarver's own extras. Other applications need
not interpret those extras. Formats that cannot carry MATH, including Glyphs,
SFD, Type 1 and bitmap-only formats, report its omission. If non-exporting
glyphs are referenced, export drops the affected references from the output
and reports a warning while leaving the document unchanged.

**Delete MATH table** stages an explicit deletion. After applying it, subsequent
exports omit MATH instead of restoring the original imported table. Undo
restores the data.

Converting a TrueType source to OTF/CFF can require rebuilding unrelated
imported layout data. For example, DejaVu Sans has GPOS point-index anchors
that cannot be preserved through outline conversion. The export reports this;
disabling **Preserve untouched tables** explicitly omits that raw layout data.
This option does not remove authored MATH.
