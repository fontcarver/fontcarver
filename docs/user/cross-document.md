# Working across two fonts

FontCarver compares and transfers between the documents you have open. Every
operation previews first, states exactly what it will change, and writes a
single undo entry when you apply it. Nothing is written to disk until you save.

## Paste Special

**Edit → Paste Special…** copies glyphs from another open font. Choose the
source font, the source and target masters, and the glyphs: type their names, or
leave the field empty to use the source font's current selection.

Two choices decide what happens:

| Paste destination                   | Effect                                                                |
| ----------------------------------- | --------------------------------------------------------------------- |
| **Same glyph names**                | Replaces glyphs of the same name and adds names the target lacks.     |
| **Selected target glyphs**          | Replaces the glyphs selected in the target, keeping their names.      |
| **As components in selected glyphs**| Appends a reference to the source glyph, keeping existing content.    |

| Data to paste              | Effect                                                              |
| -------------------------- | --------------------------------------------------------------------- |
| **All glyph data**         | The whole glyph record: every compatible layer and its metadata.     |
| **Selected master layer**  | Only the chosen source master's layer, into the chosen target master. |
| **Kerning group membership**| Only first- and second-side group membership; no pair values.        |
| **Metric keys**            | Only the metrics formulas; stored widths stay unchanged.             |

All twelve combinations are supported. Fields the chosen payload does not cover
are left as they are in the target.

**Preview transfer** validates the whole operation against the real editing
rules without changing anything, and reports the glyphs it would write, the
extra bases it would add and any warnings. **Apply transfer** re-checks both
documents' revisions and writes one undo entry, `Paste special`. If either
document changed since the preview, the transfer is refused and you preview
again; the target is never left half-written.

Source and target must agree where it matters. A source master with no unique
counterpart in the target, variable alternate layers under different axes,
TrueType instructions under a different global program or units per em, and
colour layers under different palettes are rejected with a message rather than
guessed at. A paste is bounded to 512 requested glyphs.

### Missing component bases

A copied glyph often references bases the target does not have. Choose what
should happen:

- **Copy missing bases from source** brings each missing base across, following
  the dependency chain.
- **Create empty, nonexporting placeholders** adds an empty glyph in its place,
  marks it as not exporting and tags it `missing-component-base`. The review
  lists every placeholder it will create and the smart filter that finds them
  again, `tags~"missing-component-base"`. Placeholder cells appear in the
  inventory like any other non-exporting glyph, so you can fill them in later.
- **Reject missing bases** refuses the whole transfer.
- **Decompose source components** converts the source geometry to contours, so
  no base is needed. It cannot be combined with the as-components destination.

Cyclic references and excessively deep component chains are rejected before
anything is written.

## Relink Components

**Edit → Relink Components…** repairs references by name: every component whose
base is the name you enter is pointed at the replacement glyph instead.
Transforms, smart-component values and colour-layer components are preserved.
Search the selected glyphs or every glyph in the document. A replacement that
would create a component cycle is rejected and the document is left untouched.
The repair is one undo entry, `Relink components`.

## Compare Fonts

**Font → Compare Fonts / Family…** compares the current document with another
open font, master by master. Choose the source font and both masters, then
press **Compare fonts**.

The result reports, for the whole font:

- **Glyph differences** — counts of equal, differing outlines, metrics, Unicode
  assignments, components and missing glyphs, and a sorted, paged table.
  Clicking a glyph name draws both outlines on top of each other, with an option
  to normalize a different em size.
- **Differences in font setup** — family name, units per em, axes, masters,
  glyph inventory, master metrics, metadata, kerning and OpenType layout.
- **Metadata differences** — every other font-info field that differs, with both
  values.
- **Kerning differences** — groups present on only one side or with different
  members, and counts plus a sample of records that differ.
- **OpenType layout differences** — prefixes, features and glyph classes present
  on only one side or with different source, whether the authoritative
  representation differs, and whether the assembled feature source is identical.

Comparison never changes either document.

### Compare a family

**Compare open family** compares every master of the current document and of
every other open font against the chosen reference master, and says whether the
whole family matches. Each member row shows which setup flags differ, its own
difference counts and a sample of differing glyph names, so a mismatched master
inside one document is as visible as a mismatched sibling font.

### Compare against a reference binary

Enter a path in **Reference font path** and press **Open reference** to open a
compiled or source font as a comparison reference. A reference is a *read-only*
document: the kernel refuses every edit, undo, redo and save on it, so comparing
against it cannot change it. It appears in the source list like any other open
font, and the dialog says which source is a reference.

## Conflict-resolved merge

The comparison is also the merge screen. For each differing glyph choose a
resolution — **Leave unchanged**, **Keep this font** or **Take the reference** —
and tick the font-wide resources you want from the reference:

- **Take reference metadata** replaces every font-info field, including the
  family name and units per em.
- **Take reference master metrics** replaces the target master's metrics.
- **Take reference kerning groups and pairs** replaces the groups and the
  target master's kerning records. Group members and records whose glyphs are
  absent from the merged result are skipped and counted in a warning.

**Preview merge** validates the whole merge and reports the glyphs it would
replace or add, the glyphs you chose to keep, the extra bases and the resources
it would adopt. **Merge into target** applies everything as one transaction and
writes a single undo entry, `Merge fonts`; a changed document refuses the apply
and asks for a new preview.

A merge deliberately does **not** adopt OpenType layout source, bitmap strikes,
colour palettes or global hint programs. Layout source is edited through the
features editor, which has its own revision and provenance checks; adopt it
there instead.
