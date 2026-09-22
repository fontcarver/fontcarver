# FontCarver document (`.fcf`)

Version 1 is a UTF-8 JSON document with this envelope:

```json
{
  "format": "fontcarver",
  "version": 1,
  "font": {}
}
```

`format` is `fontcarver`. Documents written before the product was renamed
carry the legacy tags `fontcutter` or `fontworkshop`; readers accept both and writers re-stamp
such a document with `fontcarver` on its next save. Readers accept versions up to their current
version and ignore unknown fields for forward compatibility. The optional
`document_path` envelope field is written only for recovery autosaves. Files
named `*.fcf.gz` are gzip-compressed; readers detect gzip from the `1f 8b`
magic regardless of the file extension.

The `font` value uses the serde shapes defined by the public types in
[`fc-model`](../../../fc-model/src/lib.rs). `IndexMap` preserves glyph order,
and binary model fields use the base64 encodings defined by `fc-model`.
Glyph `metrics_keys` (`left`, `right`, `width`, `vertical_origin`, and
`vertical_width`) are existing version-1 model fields and remain persisted
unchanged; resolved metrics and edit-session state are derived and are not
written to the document.

Feature prefixes/blocks and classes carry a persistent string `id`, independent
of their tag, name, code or ordering. New items receive UUID v4 IDs without
separators. Older documents may omit IDs; reading assigns missing IDs and
replaces empty or repeated IDs, preserving the first occurrence of a nonempty
stored ID. Saving persists these identities. Source maps are derived and are
not written to FCF. P2b-02 adds these optional font-level fields, with defaults for version-1 files:

- `feature_draft`: nullable `{source: Features, settings: FeatureGenerationSettings,
revision: u64}`. Invalid Unicode FEA text (including CRLF and comments) is
  persisted without parsing. `null` means no saved draft. Merely reading source
  does not create a draft. `font.features` always remains the effective source.
- `feature_settings`: the active generator/writer policy. Name-derived recipes
  default off; existing kerning/anchor writer policy defaults on. Saved candidate
  settings live inside the draft and are inactive until validated application.
  `localized` entries store `{script, language, substitutions}`;
  `discretionary_ligatures` entries store
  `{script, language, components, replacement}`. Missing discretionary scopes
  in older files default to `DFLT`/`dflt`. Generated-source identities and the
  exported-glyph fingerprint are server-maintained provenance stored with the
  settings; editor API callers cannot forge them.
- `feature_revision`: a monotonic editor revision, initially zero. Draft changes,
  discard, successful activation and feature undo/redo advance it. A stored
  draft's `revision` matches this counter. Undo restores source contents but does
  not reuse a prior editor revision. Effective compile revisions are runtime-only
  and deliberately exclude draft-only changes.
- `layout_conversion`: nullable original-layout revert metadata. It stores
  `original_features`, `original_settings`, `original_glyph_order`, a SHA-256
  `baseline_fingerprint`, optional `lifted_kerning` and `lifted_anchors`. The
  kerning backup contains affected masters' horizontal pairs, shared groups and
  per-glyph side memberships. Anchor backups contain only a glyph name, layer
  ID and its original anchor list. Outlines and unrelated metadata are not
  duplicated. Original table bytes remain in `extra.raw_tables`; validation
  tokens and parsed layout graphs are never persisted.

Native save, gzip save and recovery autosave use these identical serde shapes.
Absent fields preserve old-document semantics; historic `automatic: true`
blocks do not enable generation recipes by themselves.

The older UFO importer stored an entire FEA file as the only feature block with
`tag: "fea"`, `automatic: false` and empty notes. When source authority is `Fea`
and there are no other prefixes/classes/features, reading moves this block to
`prefixes` without changing its ID, text, disabled flag or tag. Other source
arrangements and `ImportedTables` authority are not migrated. Source text keeps
its whitespace, comments and original line endings.

Writers use two-space indentation for the envelope and font-level objects. The
`font.glyphs` map keeps its deterministic `IndexMap` order, but each map entry's
entire glyph value is compact JSON on one physical line. This hybrid layout
makes a glyph edit appear as a focused per-glyph line change in Git instead of
expanding every contour and point across a large multi-line diff.

## P5 mathematical typesetting

`font.math` (default null) is font-wide editable MATH 1.0 data. It contains
nullable `constants`, `min_connector_overlap` and a `glyphs` dictionary keyed by
persistent glyph names. Constants include all 51 device-adjustable values, two
minimum heights and three signed percentage fields. Font-unit values are finite
numbers, rounded only at binary export.

A mathematical value is `{value, device}`; an optional device stores
`{start_size, deltas}` with consecutive signed pixel corrections. Per-glyph
records contain nullable `italic_correction` and `top_accent`, `extended_shape`,
four nullable `kern` corner functions, and nullable `vertical`/`horizontal`
constructions. Each kern function has ascending `heights` and one more `values`
entry. Constructions hold `variants: [{glyph,advance}]` and an optional
`assembly: {italic_correction,parts}`. Parts contain `glyph`,
`start_connector`, `end_connector`, `full_advance` and `extender`, in left-to-right
or bottom-to-top order.

`font.math_table_owned` (default false) records explicit semantic ownership.
When true, a null `math` means deletion and old raw MATH must not reappear.
A non-null `math` always takes precedence over raw bytes. Legacy documents
without these fields retain their previous raw-table preservation semantics.
The additive fields do not change the FCF version.

## P5 TrueType instruction bindings

Layers may contain `tt_binding: {fingerprint, authored}` beside the existing
`tt_instructions` byte array. `fingerprint` is a versioned compiled point/metric
digest, currently `tt-points-v1:` followed by a lowercase SHA-256 hex digest.
`authored` defaults to false and distinguishes retained imported bytes from an
explicitly authored program. The optional field is omitted when absent. Older
documents retain legacy import semantics; a missing binding must never be
interpreted as permission to attach a program to newly compiled point numbers.

`master.tt.authored_units_per_em` is an optional integer recording explicit global
program ownership and the source scale for CVT/fpgm/prep. Absence identifies
imported/legacy data. These additive fields do not change the schema version.
Geometry edits retain authored programs and their bindings for review; only
comparison with current compiled geometry can establish export eligibility.

## P3 bitmap strikes

`font.bitmap_strikes` (default `[]`) contains strikes keyed by the unique pair
`(ppem, depth)`. Each strike has nonnegative pixel `ascent` and `descent`, a
`properties` dictionary of BDF values (`{"Int":123}` or `{"Str":"text"}`), and a
`glyphs` dictionary keyed by existing persistent glyph names. Supported depths
are 1, 2, 4, 8 (coverage) and 32 (straight RGBA8).

Each bitmap glyph stores `advance`, nullable `vertical_advance`, optional
`vertical_origin: [x,y]` (default null), `bitmap` and `refs`. Vertical origin is
relative to the horizontal writing origin. References are `{base,dx,dy}` in y-up
pixels and must form a bounded acyclic graph within that strike.

The bitmap stores `width`, `height`, `origin_x`, `origin_y`, `depth` and `rows`.
Origins locate the lower-left corner relative to the baseline. Rows run from top
to bottom, with MSB-first packed samples and each row aligned to a byte. Unused
row bits are zero; RGBA uses four bytes per pixel in red/green/blue/alpha order.
Unlike opaque sfnt byte tables, the pre-existing `rows: Vec<u8>` serde contract
uses a JSON array of bytes, retained for version-1 compatibility and the editor
API. Empty bitmaps retain advance metrics.

`font.bitmap_tables_owned` (default false) records that decoded/edited strikes
own EBDT/EBLC, CBDT/CBLC and sbix tables. Once owned, exporting an empty strike
collection removes old embedded tables. Native save/recovery and UFO extension
metadata retain this flag, so deleted strikes cannot reappear after reopening.

Read and write validate the exact buffer length, referenced names, depth,
origins, cycles and resource limits (4096-pixel dimensions/ppem, 16 MiB per
image, 256 MiB decoded pixels and 256 strikes per font). Pixel undo records one
image; strike changes record the strike collection. Neither changes outline
hint-program eligibility. Glyph rename/duplicate/delete update strike ownership;
deleting a referenced glyph resolves surviving dependants before removal.

## P4 local smart components

Glyphs have `smart_axes` (default `[]`), each with `name`, `min`, `default`, and
`max`. The ordinary master layer is the local default. Supplemental sources use
`{"Smart":{"master":"master-id","location":{"Width":100}}}` as their layer
kind. Coordinates use local axis names and are independent of the font axes.
Component `smart` dictionaries select these local values; omitted keys use defaults.
Smart source grids require all neighboring corners at the requested position.
The axes and source layers are editable data; evaluated component outlines are derived.

## P4 design-space, vertical and CID metadata

Existing `axes`, `masters` and `instances` store user-coordinate axis bounds and
maps, with master/instance locations in mapped design coordinates. Supplemental
layers use `Brace {master,location}` or `Bracket {master,axis,min,max}`; bracket
bounds are inclusive design coordinates. Native schema version 1 is unchanged.
Explicit axis rename commands migrate supplemental references in the same undo
transaction as axis/master/instance metadata.

Master metrics add nullable `default_vertical_origin` and
`default_vertical_advance`, both defaulting to null in older documents. Layer
`vertical_width` and `vertical_origin` remain nullable overrides; null inherits
master defaults. Existing horizontal and vertical kerning dictionaries share
named groups, so group renames and deletion update both directions.

The existing optional `cid` object contains `registry`, `ordering`, `supplement`
and a `cid_map` from decimal CID strings to glyph names. Editing, renaming and
deleting glyphs preserve map consistency and undo restores it. Mapping and
numeric glyph order are separate values committed together when requested.
Unihan records, reverse Unicode indexes, preview paths and captured batch
previews are derived data and are not serialized.

## P5 PostScript hint authoring

Existing `layer.hints` stores horizontal/vertical stems (`start`, `width`,
`ranges`), diagonal hints, point-bound `hint_masks`, and the `flex` preference.
Hint masks add two optional booleans, both defaulting to `false`:

- `counter` distinguishes a Type 2 `cntrmask` from an active `hintmask`.
- `closing` binds the mask to the segment closing onto the first explicit
  on-curve point. Without this flag, that point identifies the initial move.

`contour` and `point` remain stored contour/point indices. Mask bytes are a JSON
byte array, MSB first, with horizontal stems followed by vertical stems and zero
padding bits. Off-curve and missing points are invalid mask targets. Widths `-20` and `-21` retain PostScript ghost hints. Ordinary widths are positive.

Master alignment zones, named preferred stems and `private_dict` retain their
existing version-1 shapes. The editor validates format limits before applying
changes. Hint edits, save/reopen and undo preserve these values. Geometry changes
invalidate unchanged hint sets on affected layers and components within the same
master; hints on independent master layers remain intact. Hint previews and
export-only automatic hinting never mutate saved hint data.
