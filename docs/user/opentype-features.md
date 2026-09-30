# OpenType features and layout

The **Features** tab holds everything that becomes GSUB, GPOS and GDEF in the
exported font. It has three modes: **Source** for feature-file text, **Visual**
for typed lookup forms, and **Lookups** for inspecting imported tables.

Editing never changes the font on its own. Your text is a draft; **Check**
compiles it for every master and reports located errors, and **Compile** applies
it as a single undo step. Export refuses a document with unapplied changes.

## The source tree: prefixes, classes and features

Source is stored as separate entries, each with its own identity, so two entries
can share a tag without losing their place:

- **Prefixes** are emitted first: `languagesystem` lines, `markClass`
  definitions, shared `lookup` blocks and anything else that must precede the
  features.
- **Classes** are `@name` glyph classes.
- **Features** are tagged blocks such as `liga` or `kern`.

Select an entry to edit it. The toolbar renames it, enables or disables it,
moves it inside its section and deletes it. Disabled prefixes and features are
left out of compilation and export; their text is kept.

## Automatic versus manual, per entry

Every entry has an **Automatic** switch.

- Switched **on**, FontCarver generates that entry from the font each time you
  check or compile, and the editor is read-only. The panel below the toolbar
  names the recipe and the glyphs the rules came from.
- Switched **off**, the entry keeps the text it has and becomes yours to edit.
  The recipe stays enabled for its other entries, so you can, for example, hand
  `ss03` over to yourself while `ss01` and `ss02` keep regenerating.
- Switching a manual entry **on** asks first, because the recipe replaces the
  text you wrote. Cancel leaves the draft untouched.

The switch is disabled, with the reason on hover, when no recipe can produce an
entry with that name — every prefix, and any feature tag outside the recipe
list. Localized substitutions and discretionary ligatures need their mappings
filled in in the **Automatic generation** panel before their entry can be
switched on, because those two recipes generate from explicit mappings rather
than from glyph names.

## Automatic generation

The **Automatic generation** panel switches whole recipe families on and off.
Every recipe reads exported glyphs only, and every rule is written into an
ordinary source entry you can inspect, freeze and edit.

| Recipe                        | Produces                                                                                 | Reads                                                                           |
| ----------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Small capitals                | `smcp`                                                                                   | `.sc` suffixed glyphs                                                           |
| Capitals to small capitals    | `c2sc`                                                                                   | `.c2sc` glyphs, and `.sc` glyphs whose base is a lowercase letter               |
| Stylistic sets                | `ss01`–`ss20`                                                                            | `.ss01`–`.ss20` suffixed glyphs                                                 |
| Case forms                    | `case`                                                                                   | `.case` suffixed glyphs                                                         |
| Vertical forms                | `vert`, `vrt2`                                                                           | `.vert` glyphs (both tags) and `.vrt2` glyphs (`vrt2` only)                     |
| Standard ligatures            | `liga`                                                                                   | underscore names whose components all exist, unambiguously                      |
| Discretionary ligatures       | `dlig`                                                                                   | the explicit mapping list in this panel                                         |
| Localized substitutions       | `locl`                                                                                   | the explicit script/language mapping list in this panel                         |
| Figure styles                 | `lnum`, `onum`, `tnum`, `pnum`                                                           | `.lf`/`.lnum`, `.osf`/`.oldstyle`/`.onum`, `.tf`/`.tnum`, `.pf`/`.pnum`/`.prop` |
| Positional figures            | `sups`, `sinf`, `numr`, `dnom`                                                           | `.sups`/`.superior`/`.sup`, `.sinf`/`.inferior`, `.numr`, `.dnom`               |
| Fractions                     | `frac`                                                                                   | `.numr` and `.dnom` figures, `fraction`, and `slash` when present               |
| Ordinals                      | `ordn`                                                                                   | `ordfeminine`/`ordmasculine` with `a`/`o`, and `.ordn` glyphs                   |
| Composition and dotless forms | `ccmp`                                                                                   | composites made of a base plus mark components, and dotless pairs               |
| Access all alternates         | `aalt`                                                                                   | the alternate features already present in the source                            |
| Automatic glyph classes       | `@Uppercase`, `@Lowercase`, `@Letters`, `@Figures`, `@Marks`, `@Punctuation`, `@Symbols` | glyph category, sub-category and code points                                    |

`frac` converts any number of digits on each side of the fraction slash: the
numerators are rewritten by a reverse-chaining lookup and the denominators chain
forward from the slash. `ccmp` composes a base and its marks into the
precomposed glyph when the precomposed glyph really is built from those
components with plain translation, and replaces `i`/`j` with their dotless form
before a mark that sits above the base — a cedilla or a dot below leaves the dot
in place. `aalt` references the alternate-producing features that exist, in a
fixed order, rather than copying their rules.

A recipe never overwrites a feature you own. If you have written an `ordn`
feature yourself, enabling the ordinals recipe reports the conflict for that one
tag and generates the rest of its family as usual. Switch that entry's
**Automatic** switch on if you want the recipe to take it over, or leave the
recipe off. Switching one entry on is safe with hand-written siblings: the other
entries of that family you still own are held manual in the same change, so a
hand-written `onum` survives when you hand `lnum` to the recipe.

Naming: a generated entry is named after what it is — the feature tag (`smcp`,
`frac`) or the glyph class (`@Uppercase`, `@Figures`). Only the helper classes
and lookups a recipe writes _inside_ generated code use the reserved `fcGen`
prefix, such as `@fcGenFracNumerator`. Do not define your own names with that
prefix; if you do, the compiler reports the duplicate definition when you check
the source.

## Kerning, marks and cursive attachment

Four features come from model data rather than from source text, controlled by
the two writer checkboxes in the generation panel:

- **kern** from the master's horizontal kerning: class pairs from kerning
  groups, and glyph pairs as exceptions inside those classes.
- **mark** from anchors: a base glyph's `top` (or any other name) attaches marks
  whose matching anchor is `_top`.
- **mkmk** from the same anchors when a mark itself carries an attachment anchor,
  so marks stack.
- **curs** from `entry` and `exit` anchors.

Precedence is explicit:

1. If you write a feature with one of those tags, your feature owns the tag and
   the model's rules are left out.
2. If that feature contains the line `# Automatic Code`, the model's rules are
   inserted there instead.
3. Mixing `# Automatic Code` with your own rules of the same kind in one feature
   is rejected before the source is applied, so the result is never ambiguous.
4. Turning a writer checkbox off removes the model's rules from compilation
   entirely and leaves your kerning and anchor data untouched.

## Tokens and conditional blocks

Source accepts a bounded FontCarver dialect that resolves at compile time:

- `$unitsPerEm`, `$ascender`, `$descender`, `$xHeight`, `$capHeight`,
  `$italicAngle`.
- `${A:width}`, `${A:verticalWidth}`, `${A:verticalOrigin}`, with arithmetic:
  `${-A:width / 10}`.
- `$customNumber` for numeric entries of the font's or master's
  `user_data.featureNumbers`.
- `$[name ENDSWITH '.sc']` for a glyph list from glyph metadata; wrap it in
  brackets when a class is wanted: `[$[category == 'Letter']]`.
- Whole-line `#ifdef VARIABLE`, `#ifndef VARIABLE`, `#else` and `#endif`, which
  select different code for static and variable exports.

The editor highlights directives and tokens distinctly from comments and marks
an unterminated token as invalid. Autocomplete offers the directives after `#`
at the start of a line, and the built-in metric tokens after `$`.

FontCarver reads one self-contained feature source: `include(...)` is not
followed, in the editor, in the compiler or in the kernel's lookup reader.
Inline the included file instead.

## Diagnostics, completion and class preview

Errors and warnings appear under the editor with a one-based line and column;
selecting one jumps to the exact span in the entry that produced it. Completion
offers glyph names, `@classes`, keywords and feature tags, and queries the font
for further glyph names as you type. Alt-click — or Alt+Enter with the caret
inside the name — on a `@class` opens a read-only preview of its members with
their current outlines, following nested classes and reporting missing, cyclic
or duplicated definitions.

## Live shaping while you edit

The Text tab and the preview window shape through a font FontCarver keeps
compiled in the background. A change that only touches features, kerning or
anchors rebuilds GSUB, GPOS, GDEF and BASE and reuses the existing outline,
metric and `cmap` tables byte for byte, so live typing stays responsive; a change
to outlines, glyph order, code points, metrics or a feature-file `table` block
falls back to a full compile.
