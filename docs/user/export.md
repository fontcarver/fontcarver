# Exporting fonts

FontCarver writes every output through one pipeline: the export dialog, the
export matrix and `fontcarver build` all resolve the same settings, so a build
on a continuous-integration machine produces the file the editor produces.

## Export profiles live in the project

An **export profile** is a named settings record stored in the `.fcf` document,
not in the application. Because it is project data it travels with the font, it
is versioned in git with the rest of the source, and the command line reads it
back without any extra configuration file.

A profile holds:

| Field              | Meaning                                                    |
| ------------------ | ---------------------------------------------------------- |
| Name               | Unique, up to 80 characters.                               |
| Output subfolder   | Relative folder under the destination; no `..` components. |
| Output file name   | Base name without extension; empty derives one.            |
| Format and options | Everything the Export dialog's option list contains.       |

Open **File → Export…**, set the format and options you want, type a profile
name and press **Save profile**. The profile picker then offers **Apply
profile** (load it into the form), **Update profile** (replace it with the
current form) and **Delete profile**. Saving, updating and deleting are ordinary
document edits: they appear in the undo history and are written on the next
save.

A profile does not store a destination path. The single-export field and the
matrix's output directory stay with the person doing the export.

One profile is run over many rows, so its stored file name is a stem, not the
whole name: exporting a named instance or a variable-font configuration with it
appends that row's name (`MyFont` becomes `MyFont-Bold`). A variable-font
configuration that carries its own file name is already per-row, so that name is
used unchanged. A profile with no file name derives one from the family and the
row. Every row of a batch therefore resolves to its own file.

## Variable-font configurations

A **variable-font configuration** names a design-space subset: the source
masters to keep, the axes to keep, the named instances to write, and an output
file name. It is stored beside the profiles.

Turn on **Export a variable font**, choose the masters, axes and named
instances, give the configuration a name and press **Save configuration**.

Subsetting is real: after the masters are reduced, each axis is clamped to the
span the remaining masters cover, an axis whose masters no longer differ is
dropped from `fvar`, named instances outside the retained space are left out,
and alternate-layer condition ranges on a dropped axis are removed. Each of
these is reported as an export warning rather than happening silently.

## The export matrix

**Batch export** inside the Export dialog is a matrix: rows are the document's
active named instances plus its stored variable-font configurations, columns are
the stored profiles.

1. Set the **output directory**.
2. Press **Load instances and profiles**.
3. Tick the rows and profiles you want.
4. Press **Plan destinations**. Every cell is resolved by the kernel, so the
   preview shows the exact paths — including each profile's subfolder and file
   name — before anything is written.
5. Press **Export batch**.

A plan is refused, with nothing written, when two cells resolve to the same
path, when a resolved path would leave the output directory, or when it exceeds
100 files. During the run each finished file reports its size, timing and
warnings; a failure is reported per item and the rest of the batch continues.

## Preflight

**Check before export** runs the document's problem finder and lists what it
found with severity counts. Each finding that names a glyph has a button that
opens that glyph in an Edit tab, so a structural error can be fixed without
hunting for it. Mounting the panel makes no request: the check is explicit.

## Production options

| Option                     | Effect on the exported copy                                     |
| -------------------------- | --------------------------------------------------------------- |
| Remove overlap             | Merges overlapping contours.                                    |
| Add extrema                | Splits curves at horizontal and vertical extrema.               |
| Round coordinates          | Rounds points, component offsets, advances and metrics.         |
| Decompose components       | Replaces component references with contours.                    |
| Production names           | Writes the production glyph names the model supplies.           |
| Subroutinize               | Shares repeated static CFF charstrings (OTF and bare CFF).      |
| Automatic hinting          | PostScript stem analysis, or `ttfautohint` for static TrueType. |
| Keep TrueType instructions | Reuses imported hinting for untouched glyphs.                   |
| Preserve untouched tables  | Copies imported tables the editor did not rewrite.              |

The source document is never modified: every option is applied to a disposable
export copy, and options a format cannot honour are refused before the
destination is touched rather than being ignored.

## The command line

```sh
# What this project stores
fontcarver profiles MyFont.fcf

# One output from a stored profile
fontcarver build MyFont.fcf --profile Web --out-dir dist

# Every active named instance, with the same profile
fontcarver build MyFont.fcf --profile Web --instances all --out-dir dist

# A named subset of instances
fontcarver build MyFont.fcf --profile Web --instances "Regular,Bold" --out-dir dist

# A stored variable-font configuration
fontcarver build MyFont.fcf --profile VF --variable-config "Text VF" --out-dir dist

# The same preflight report the dialog shows, before writing
fontcarver build MyFont.fcf --profile Web --out-dir dist --preflight --fail-on-preflight
```

`--out-dir` is joined with the profile's subfolder and file name, so the command
line writes the same tree the matrix previews. `--profile` refuses to be
combined with the ad-hoc option flags (`--remove-overlap`, `--variable`,
`--instance`, …): a stored profile is the single description of that output, and
changing it means editing the profile. Unknown profile, configuration and
instance names are refused with exit code 2.

Without `--profile`, `fontcarver build INPUT OUTPUT` still takes the individual
flags for a one-off build.
