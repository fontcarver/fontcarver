# The Font view: browsing, searching and filling out a character set

The Font view lists every glyph of the open document. The toolbar chooses how it
is presented, the left sidebar narrows what is listed, and the grid or list shows
the result. Everything in the toolbar and the column set is remembered between
sessions and applies to the next document you open.

## Grid and list

**Grid view** draws each glyph in a cell. The **Cell size** slider runs from 24
to 256 pixels; the **Labels** menu puts the glyph name, its Unicode value or its
position under each cell. Only the cells on screen are drawn, so a 65,000-glyph
font scrolls at full speed.

**List view** shows one row per glyph. Click any column heading to sort by it:
Name, Unicode and Width sort by their value, Category sorts by category and then
subcategory, Script by its ISO 15924 code, and Status puts the glyphs that need
attention first. Sorting is always ascending.

Drag a column heading onto another to move it, or use the small arrows in the
heading, which work from the keyboard. The **Choose columns** button at the right
of the heading row adds or removes columns, including Subcategory; at least one
column always remains.

## Cell badges

A cell carries a small badge for each thing worth knowing about its glyph:

| Badge                    | Meaning                                                                                    |
| ------------------------ | ------------------------------------------------------------------------------------------ |
| Metrics out of sync      | A metrics formula does not match the stored sidebearings or advance, or cannot be resolved |
| Incompatible masters     | A master layer is missing, or the layers cannot interpolate                                |
| Annotations              | The glyph has a note, or a layer carries editor annotations                                |
| Custom glyph information | Stored classification differs from the glyph database, or format-specific data is attached |
| Not exported             | The glyph is excluded from export; its cell is also hatched                                |
| Locked                   | The glyph is locked against editing                                                        |
| Placeholder              | A stand-in a cross-document transfer created for a component base this font lacks          |

Point at a badge for its name. The **Statuses** section of the sidebar turns each
badge into a filter, and the list's Status column and its sort order use the same
information. A colour label appears as a stripe along the top of the cell; the
selected layer's own colour label, which is independent of it, appears as a small
triangle in the upper-left corner.

## Search

The search field takes a glyph name, a Unicode value (`U+0041`, `uni0041` or the
character itself), a Mandarin reading or a stroke count. The scope menu limits it
to **Name**, **Unicode** or **Note**; **All** searches all of them. `Aa` matches
case-insensitively until you turn on **Match case**, and the **.\*** button
treats the text as a regular expression. A pattern that does not compile is
reported under the field and the listing is left as it was.

## Sidebar

Sections narrow the listing. Values inside one section are combined with "or";
sections are combined with "and", so a category and a script intersect. Click a
selected row again to drop it.

- **Lists** are named sets of exact glyph names saved with the project.
- **Smart filters** are saved predicate expressions; each row shows how many
  glyphs it matches out of the whole font.
- **Statuses** filters by a cell badge.
- **Categories** and their subcategories, **Languages**, **Scripts** and
  **Unicode blocks** classify glyphs by their first Unicode value.
- **Colours** lists the colour labels in use.

Every sectioned row shows a badge of `characters encoded / characters in the
repertoire`; point at the row for the glyph count as well. A row that is missing
some of its characters gets a **+** button: it opens **Add Glyphs** with the
missing characters already in the recipe, so you can review them before they are
created. A row missing more than 4,096 characters preloads the first of them and
says so.

## Add Glyphs

**Add Glyphs** (⌘⇧G) accepts, separated by spaces or commas:

- glyph names: `Aacute`, `a.sc`, `f_i`
- single characters: `A`, `一`
- Unicode values: `U+0041`, `uni4E00`
- inclusive ranges: `A-Z`, `U+0041-U+005A`, `uni4E00-uni4E1F`
- repertoires: `@charset:GB2312`, `@charset:big5`, `@charset:jisx0208`,
  `@charset:ksx1001`, `@charset:gb18030level1`, `@charset:iicore`

The character-set list shows how much of each repertoire is still missing, and
the preview lists exactly what would be created. With **Build composites from the
glyph database** on, a glyph the database defines as a composition — `Aacute` as
`A` plus `acutecomb` — is created as a composite, with each mark placed so its
entry anchor (`_top`) lands on the base's matching anchor (`top`); marks stack, so
a second mark sits above the first. The composite takes the base's advance and
its anchors. A glyph whose components the font does not have is created empty and
counted separately, so nothing is silently wrong. Turn the option off to create
plain empty glyphs. The whole batch is one undo step.

## Colour labels, tags and glyph information

Right-click a selection in the grid or the list for the twelve colour labels and
**None**; the change covers the whole selection in one undo step. The same
entries are in the Glyph menu and the command palette. Free-text tags are edited
in the Edit view's inspector, which completes from the tags already used in the
font; tags, colour labels and layer colours are all usable as smart-filter
predicates.

**Update Glyph Info** re-applies the glyph database to the selection —
category, subcategory, script and production name — in one undo step. Unicode
values are the exception: a glyph keeps the encoding it has unless the database
names that exact glyph and says what it encodes, so a suffixed, private-use or
otherwise unlisted glyph never loses its `cmap` entry to a refresh, and nothing
is re-encoded from a name that happens to resemble a known one. The
Edit view's **Glyph database** panel shows what the database holds for the
current glyph — including its suggested anchors and composition — and lets you
edit a per-project overlay as JSON or import one from a file. An overlay record
replaces only the fields it names, and suffixed names inherit from their base, so
`A.ss01` follows `A`.

## Batch rename

Select glyphs and choose **Batch rename** (from the Glyph menu or the grid's
context menu). Find and replace is literal and case-sensitive by default;
**Ignore case** relaxes the match and **Regular expression** enables patterns
with capture groups, where `$1` inserts the first group. A pattern that does not
compile is reported under the field and stages nothing.

The preview lists every name before and after, a page at a time, and names that
would collide or are not valid glyph names are reported instead of applied. The
whole batch is one undo step; every source moves through a temporary name first,
so swaps and chains work. A rename follows through components, constructions,
kerning pairs and groups, bitmap references, CID, MATH, colour and SVG data,
metrics formulas, feature classes and feature code, and the project's saved glyph
lists. Feature source is text, so a glyph name that only appears in an included
file is not rewritten; a glyph whose name is also a feature-syntax keyword is
left alone there too.

## CJK

Empty cells whose glyph has a Unicode value show that character drawn with a
system face, in a muted colour, so a large CJK font is navigable before anything
is drawn. The **Reference characters** button in the toolbar turns this off. The
sidebar's radical, stroke-count and Unicode-block sections, and the Radical,
Total strokes and Mandarin reading sort orders, are described in
[Variable fonts and CJK](variable-and-cjk.md).
