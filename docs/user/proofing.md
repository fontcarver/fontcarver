# Previewing and proofing text

The Edit tab is the proofing surface: one shaped run of text, a waterfall strip
under the canvas, and panels for shaping overrides, the project's saved proof
strings and proof-sheet export. Everything here reads the document you are
editing, so a change to a glyph shows up in the proof without an export.

## The preview strip

The strip under the canvas renders the current run as outlines at several sizes
at once. `Mod+3` (**View → Text preview strip**) shows and hides it, and the
divider at its top resizes it with the pointer or, once focused, with
`Home`, `End`, `↑` and `↓`.

| Control            | What it does                                                                                    |
| ------------------ | ----------------------------------------------------------------------------------------------- |
| Preview instance   | Renders a design-space master or a named instance (see below).                                  |
| Preview content    | The whole run, or the active glyph alone at every step.                                         |
| Blur (px)          | 0 to 4 pixels of blur, for judging weight and rhythm the way distance or a screen softens them. |
| Waterfall sizes    | The steps the strip draws; see **Waterfall steps**.                                             |
| Flip horizontally  | Mirrors the preview, which exposes imbalance the eye has stopped noticing.                      |
| Flip vertically    | Mirrors it top to bottom, for the same reason.                                                  |
| Light/dark preview | Inverts the preview's ink and background independently of the application theme.                |
| Detach preview     | Moves the strip into its own window.                                                            |

Every one of these settings belongs to the tab, not to the strip: the detached
window shows exactly what the docked strip shows, and a saved workspace reopens
with the same waterfall, flip, blur and content choice.

### Waterfall steps

The **Waterfall sizes** field takes either form:

- a list, `12, 24, 48`, of up to eight sizes; or
- a range with a step, `8-72/8`, which expands to `8, 16, 24, …`. `8..72` and
  `8 to 72` are the same thing, and a range without `/step` picks the smallest
  whole step that still fits eight rows.

Sizes run from 6 to 256 pixels per em. The field always shows the list that is
actually being drawn, so an accepted range is replaced by the sizes it produced.

A waterfall draws at most eight steps. A **range** longer than that is cut to its
first eight and says so, because a range describes an intention that is still
useful when it does not fit — `8-72/8` asks for nine steps and draws `8` to `64`.
A **list** is an explicit choice, so a ninth entry is refused rather than
dropped. Any refused entry names the rule it broke — an unreadable number, a size
out of range, a repeated size, a reversed range — and leaves the drawn waterfall
alone.

### Previewing a master or a named instance

The **Preview instance** menu lists the document's masters first and then its
named instances. Choosing a master moves the whole run to that master, because
previewing a master means shaping with it — the canvas, the metrics and the
kerning follow. Choosing a named instance leaves the edited master alone and
renders the interpolated outlines of that instance in the strip only; the
instance is a proof, not an editing target.

A variable font also has the **Variable font proof** panel, which adds a slider
per axis for locations that have no named instance.

## Shaping controls

The **Shaping controls** panel owns what the shaper is asked for.

- **Script** and **Language** override automatic detection. `Latn` and `zh-Hant`
  are accepted; anything that is not a four-letter script code or a well-formed
  language tag is refused, and an empty field means automatic detection.
- Each feature has three states. **Default** leaves the tag out of the request,
  so the font's own default applies; **On** and **Off** force it. A feature a
  font applies by default is only suppressed by **Off** — removing an override is
  not the same as turning a feature off.
- **Custom feature tag** adds any four-character tag to the list at Default,
  which changes nothing until you choose On or Off.
- **Glyph clusters** lists the shaped run grouped by source cluster: the cluster
  number in visual order, its UTF-8 source offset, the characters it came from,
  and each glyph's name, position and advance in font units. A base and its marks
  share one cluster; a right-to-left run reads in the order it is drawn while its
  offsets still name the original text.

## The proof library

**Proof library** holds the project's own strings — pairs for spacing, phrases
for reading. They live in the document, so they travel with the `.fcf` file and
with anyone you send it to, and every change is one undo step.

- **Add built-in samples** writes a grouped, script-aware starting set into the
  project: Latin, Cyrillic and Greek pangrams, uppercase, lowercase, figures and
  diacritics; Latin, Cyrillic and Greek kerning pairs and spacing control words;
  Han phrases including a vertical one; and Arabic phrases and joining forms.
  Pressing it again adds nothing, and a sample you have renamed or rewritten is
  never replaced.
- **Save current string** stores the text, script, language and direction that
  are in force. **Update selected string** replaces the selected entry;
  **Remove selected string** deletes it.
- The two menus filter by group and by script, and the stepping below only moves
  through what the filters leave visible.
- **Export as text** writes the library as one tab-separated line per entry
  (`group`, `name`, `script`, `language`, `direction`, `text`), which is small
  enough to keep in a repository and read in a diff. **Import from text** merges
  such a file: an entry whose group and name match an existing one updates it,
  everything else is added, and a file with one malformed line imports nothing
  rather than half of itself.

### Stepping while you space

`↑` and `↓` step to the previous and next visible proof string. They work in the
text field and on the canvas, so you can keep spacing with `Mod+←`, `Mod+→` and
the brackets and change the proof string without reaching for the mouse. While a
node is selected the arrows still nudge it, every modified arrow keeps its usual
meaning, and a control that moves on `↑` and `↓` itself — the preview strip's
resize handle, for instance — keeps its own keys while it has focus.

The exported file keeps a proof exactly as it was saved, including trailing
spaces — a spacing string such as `HH HOH ` depends on them. A group or name that
starts with `#` is written escaped, so it is not mistaken for one of the file's
comment lines on the way back in.

## Vertical proofing

The **Vertical** direction button switches the run to top-to-bottom. The shaper
is then asked for `vert` and `vkrn`, the run uses vertical advances and origins,
the preview strip grows to fit the column, and the exported sheet keeps the same
vertical layout — a vector sheet by its bounds, an embedded-font sheet with CSS
`writing-mode: vertical-rl`.

## Proof sheets

**Proof sheet** writes a standalone file you can print, send or archive.

- **Proof content** exports either the current string or every string the
  filters leave visible.
- **Proof sizes (pt)** takes up to eight point sizes; each string is written once
  per size, and each row names its own size, so the sheet is a waterfall.
- **HTML rendering** chooses between:
  - **Embedded font** — a self-contained page carrying the compiled WOFF 2 of the
    face in an `@font-face` rule, with the real text set in it. The text stays
    selectable and searchable, and the page proves the exported font itself works
    in a browser. The proof's direction, language and feature overrides are
    written into the page as `dir`, `lang` and `font-feature-settings`.
  - **Outlines** — the exact shaped snapshot as inline SVG vectors. Nothing is
    re-shaped by the viewer, so this is the faithful record of what the editor
    drew, and it needs no font at all.
- **Export PDF** always writes real filled vector outlines — never a screenshot —
  paginated on A4 with a valid cross-reference table. A run too wide or tall for
  the page is fitted and annotated with its requested and actual point size.

Both exports re-shape every string completely first: the strip's bounded preview
subset is never what gets exported. An export refuses to proceed if the document
changes while it runs, if the font cannot compile, or if a string contains glyphs
the document does not have, and it can be cancelled.

PDF labels use the built-in Helvetica without an embedded font, so a non-ASCII
character in a label is shown as `U+XXXX`; the original text is kept in the PDF's
`ActualText` and title metadata, so copying and searching still find it. The
specimen outlines themselves are unaffected and cover every script.
