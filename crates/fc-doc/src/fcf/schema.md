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

Project inventory collections use the existing
`font.info.user_data["com.fontcarver.inventory"]` dictionary. Its optional
`smart_filters` entry is an ordered array of `{name, expression}` values;
expressions contain the bounded smart-filter DSL, and names are unique after
trimming. The editing API accepts at most 256 filters, 256 UTF-8 bytes per name,
and 1 MiB of combined name/expression text. The optional `glyph_lists` entry
stores an ordered array of `{id, name, glyphs}` values, where `glyphs` contains
ordered exact glyph names. List identifiers are unique and contain 1–80 ASCII
letters, digits, underscores, or hyphens. Display names are trimmed and contain
1–128 characters without control characters. At most 128 lists, 8192 names per
list, 65536 total names, and 2 MiB of serialized list metadata are accepted.
Glyph names use the recipe-safe ASCII letter/digit/dot/underscore grammar, begin
with a letter, dot, or underscore, and contain at most 127 bytes. Duplicate glyph
names are removed while preserving first occurrence. Live `present`/`missing`
coverage is derived by the API and is not persisted.
Unknown sibling keys and other user data are retained.
These metadata edits use ordinary font-info undo snapshots and native/autosave
serialization; no FCF version or structural model change is required.

Layer `metrics_keys` is an optional version-1 object with the same fields as
glyph metrics keys. Each non-null layer field overrides that glyph field;
omitted/null fields inherit it. Double-equals formulas (`==n`) are layer-local
and are rejected by the glyph-wide formula command. The local object is omitted
when absent. Horizontal formulas support references, mirrored sidebearings,
finite literals, arithmetic, baseline-height measurements (`=n@200`), and offsets
from automatic component metrics (`=+25`). Synchronization updates the stored
outline/advance through an undoable command; it does not rewrite the formulas.

`Component.locked` is an optional boolean (default `false`). Native documents
retain component editing locks; imported compiled-font components start unlocked.
Editor commands preserve a locked component's content and ordering until an
explicit `SetComponent` edit changes only `locked` to `false`. Locks do not alter
component resolution or compiled outline geometry.

P5-09 adds optional `Layer.effects`, an ordered array (default `[]`, omitted
when empty). Each entry is `{enabled: bool, effect: {...}}`; missing `enabled`
means `true`. Source contours, components, anchors and metrics remain unchanged.
The tagged `effect` value is one of:

- `{kind: "stroke", nib: "ellipse" | "rectangle", width, height, angle,
  cap: "round" | "butt" | "square", join: "round" | "bevel" | "miter",
  miter_limit}`. Dimensions are full pen sizes in font units, in `[0.1,8192]`,
  with aspect ratio at most 100. Angle is counterclockwise degrees in
  `[-360,360]`; miter limit is in `[1,100]`. All numbers must be finite.
- `{kind: "remove_overlap"}`, using nonzero fill and retaining open paths.
- `{kind: "boolean", operation: "union" | "intersect" | "subtract" | "xor",
  path: string}`. The SVG path is a stored mask in layer coordinates, not a
  live reference. It has at most 256 KiB and 4096 path elements, explicitly
  closed contours, and finite coordinates of magnitude at most 32767.

A layer may have at most 16 effects, including disabled entries. Native read,
write and autosave validate them. Effects apply after local smart interpolation
and component resolution, before the parent component's transform. Smart source
corners require identical stacks. Preview caches and refitting warnings are
derived. Compiled fonts evaluate a copy; native documents retain sources and
stacks. Baking replaces source contours and clears the stack in one undo step.
Other source formats currently reject unbaked stacks before touching output.

P5-10 adds optional `Point.corner_radius` (default zero, omitted when zero).
It is a finite signed font-unit distance with magnitude at most 8192. Positive
values request a tangent circular fillet; negative values request a V-shaped ink
trap with that mouth setback and tip depth. Only on-curve interior nodes accept
radii; traps require closed contours. Each layer permits at most 1024 active
corners. The original point list stays unchanged until an explicit bake.
Magnitudes below 1e-7 font units retain metadata but evaluate as the source join,
so interpolation can pass through zero without constructing sub-resolution arcs.
Source-node affine edits retain the numeric radius in font units; placing a
component transforms its already evaluated corner geometry.
Corner metadata travels with its node through insertion/reversal and native
history. Source-node geometry is evaluated before the layer-effect stack and
before a parent component transform. Ink-trap direction uses the layer's nonzero
fill, including counter contours. Colliding radii, singular tangents and corners
that do not fit adjacent segments fail explicitly. Local smart interpolation
interpolates signed radii; globally baked master topology must be compatible.
Other editable source writers reject live corners before writing. Point-bound
hint policies are the same as for other geometry-changing live effects.

P5-11 adds `{kind: "variable_stroke", nib, cap, join, miter_limit,
progression: "linear" | "smooth", points: [{position,width,height,angle}]}`
to `Layer.effects`. There are 2–64 strictly increasing, finite positions in
`[0,1]`, beginning at exactly 0 and ending at exactly 1. Positions measure
normalized centerline arc length independently on every input contour; source
edits retain the positions and resample against the new length. Pen dimensions,
angle and miter limits use the fixed-stroke bounds. Dimensions interpolate
linearly or with cubic smoothstep; angles follow the shortest angular interval.
Closed contours require matching endpoint dimensions and angles modulo 360°.
Evaluation rejects incompatible closed seams, excessive sampling or output
geometry before committing an edit. Evaluation samples source curves and nib
supports, unions filled cap/join/span patches, and fits cubic output; it always
reports approximation through the existing effect outcome. Native files keep
the original source and pen points; bake/export use the shared effect evaluator.

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

For static CID CFF1 sources, master `cff_private_dicts` retains additional
PostScript private dictionaries (FD 1 onward); the existing `private_dict` is
FD 0. Layer `cff_private_dict_index` selects a dictionary within its master;
absent or zero selects FD 0. Both fields are optional for backward compatibility.
The vector and selection persist in JSON and compressed FCF. CFF1 export rejects
out-of-range selections or additional dictionaries without CID metadata.

Master alignment zones, named preferred stems and `private_dict` retain their
existing version-1 shapes. The editor validates format limits before applying
changes. Hint edits, save/reopen and undo preserve these values. Geometry changes
invalidate unchanged hint sets on affected layers and components within the same
master; hints on independent master layers remain intact. Hint previews and
export-only automatic hinting never mutate saved hint data.

## P5 COLR v0/v1 / CPAL colors

`font.color` defaults to null. A `ColorTable` contains `palette_version` (0 or
1), equally sized nonempty `palettes`, optional `entry_labels`, and `glyphs`
(mapping base glyph names to nonempty, bottom-to-top `ColrLayer` arrays).
Each layer stores `glyph` (a monochrome outline reference) and
`palette_index` (null means text foreground / binary 0xFFFF). Nested COLR
compositions are not followed, and a self-reference is valid.

Each `ColorPalette` contains `colors` with eight-bit `red`, `green`, `blue`,
`alpha` channels (unpremultiplied sRGB), CPAL v1 `flags` (0..3), and optional
`label` name ID. `entry_labels` is empty when absent, otherwise one optional
name ID per shared entry. Label IDs refer to preserved `font.info.names`.

`font.color_tables_owned` defaults to false. When true, null `color` means
explicit deletion of both COLR and CPAL. Non-null editable color data always
supersedes raw tables. Unsupported variable COLRv1 and malformed inputs retain both
original tables atomically under the existing raw preservation policy. Legacy
`Layer.color_layers` remains an opaque outline/paint container; it is not the
named COLR v0 model and is not synthesized into these tables.

### Static COLRv1 graphs

`ColorTable.graph` defaults to null for old documents. A non-null `ColrGraph`
contains `glyphs` (named roots to zero-based paint indices), `paints` (shared
nodes tagged by `kind`) and default-empty `clips` (named `[xMin,yMin,xMax,yMax]`
boxes). This coexists with v0 fallback `ColorTable.glyphs` and shares CPAL.
`Layers`, `Solid`, `Linear`, `Radial`, `Sweep`, `Glyph`, `ColrGlyph`, `Transform`
and `Composite` nodes are described in [the graph reference](../../../../docs/user/colrv1.md).
Named glyphs and palette entries are validated along with graph cycles, depth,
expansion, boundedness and binary precision. Graph source JSON is a UI draft;
only a structurally valid, kernel-validated graph enters the document.
Graph deletion sets `graph` null and retains palettes/v0 fallbacks; whole-color
deletion continues to use `font.color = null` with owned tables.

### SVG color documents

`font.svg` defaults to null. `SvgTable.documents` is an ordered array of
`{ source, glyphs }` records. `source` is UTF-8 SVG XML; `glyphs` maps persistent
font glyph names to XML element IDs. Multiple glyphs may share a document or
an element. Font glyph IDs are generated at binary export; document IDs need
not match current glyph positions. Renaming, duplicating and deleting glyphs
update the name bindings while preserving shared definitions in the source.

`font.svg_table_owned` defaults to false. When true, null `svg` explicitly
removes the `SVG ` table instead of restoring imported bytes. Unsupported or
malformed imports retain their original raw table and leave ownership false.
Undo captures both fields. UFO private metadata stores `svg_json` as JSON text
and `svg_table_owned` independently, including explicit deletions.

Editable XML is the bounded static subset documented in `docs/user/svg.md`.
Documents are limited to 2 MiB each, 32 MiB total, 100,000 XML nodes and 128
nesting levels. Cyclic references and expansion above 1,000,000 elements are
rejected. The binary writer applies the same 32 MiB output budget after
adapting shared documents to the compiled glyph order.

P5-16 stores the optional version-1 glyph database overlay as the JSON object
`font.info.user_data["com.fontcarver.glyphData"]`. Its `version` is `1`; `glyphs`
is a name-keyed object of partial `unicodes`, `category`, `sub_category`, `script`,
`production_name`, `anchors` and `composition` overrides. Omitted fields inherit
the bundled database. Empty arrays explicitly clear a list. Dot suffixes inherit
base classification and recipes while remaining unencoded unless the exact
suffixed overlay assigns Unicode. Update Glyph Info persists the resolved
encoding, classification and production name into existing Glyph fields; anchor
and composition lists are suggestions and do not change source geometry. Both
overlay replacement and metadata refresh use the ordinary undo history.


P5-89/91 stores non-exported layer editor metadata in
`layer.user_data["com.fontcarver.editor"]`, preserving other user-data keys and
unknown keys inside this object. `annotations` is an array of records with
`id`, `kind` (`Text`, `Arrow`, `Circle`, `Rectangle`, `Plus`, or `Minus`), finite `x`, `y`, `end_x`,
`end_y` coordinates in y-up font units, and plain `text`. IDs are unique and
nonblank within a layer (maximum 256 UTF-8 bytes); text is limited to 16,384
UTF-8 bytes per annotation and a layer to 10,000 annotations. Circle and
rectangle records use opposite bounding-box corners; arrows use endpoints.
Missing annotations mean an empty list.

`locked_paths` is an array of persistent `Contour.id` strings, with a missing
array meaning no path locks. Locking an unnamed contour assigns a UUID
identifier; an ambiguous existing ID is replaced with a fresh unique one.
Locks follow their contour IDs through list reordering. Commands must preserve
locked contours, their owning layer, and the lock list until an explicit
`EditorMetadata::PathLocks` unlock. The existing `Glyph.locked` boolean blocks
glyph-targeted editing, including annotation/path-lock changes; explicit
`EditorMetadata::GlyphLock` commands can always unlock it. These commands use
ordinary atomic batch and undo/redo storage. Native version-1 FCF persistence
retains annotations, contour IDs, path locks and glyph locks without changing
exported outlines. Non-object legacy user data is rejected by metadata edits
rather than overwritten.

P5 glyph inventory labels reuse existing metadata fields. Glyph `color_label`
uses twelve named indexes (`0` through `11`), while each layer independently
stores its label at `Layer.user_data["com.fontcarver.colorLabel"]`. Omission means
no layer label. Label commands preserve other object entries and reject
non-object vendor metadata rather than discarding it. Glyph `tags` remain
trimmed free-text strings; autocomplete is derived from the document's tag set.

P5 construction editing adds optional `Point.id` and `Guide.id` string identities,
and optional `Point.super_smooth` (default false). IDs identify nodes and guides
within their containing layer, so references survive contour/point reordering.
`NodeRef` is `{id: string}`. `Layer.node_links` and `Layer.linked_guides` default
to empty arrays and are omitted when empty. A node link is
`{node, first, second, axis: "X" | "Y", ratio, handles: [{node, ratio}]}`. It sets
the selected servant coordinate to `first + (second - first) * ratio`; associated
handle ratios follow the same boundary span. Ratios are finite and may lie
outside `[0,1]`. A linked guide is `{guide_id, nodes: [{node, offset}]}`; offsets
are finite signed distances along the guide's left normal. Moving the guide
retains each node's tangent position and normal offset. References must resolve
uniquely; missing nodes/guides, cyclic constraints and conflicting coordinate
owners are rejected. Native read, write and autosave validate these constraints.

`Layer.constructions` is an optional array, defaulting to empty and omitted when
empty. Each entry is `{id, source, kind, keep_carrier}`. `source` names a glyph
resolved in the carrier's master; `id` is a unique nonempty layer-local string
of at most 256 bytes. `keep_carrier` defaults to false. The externally tagged
`kind` is one of:

- `{Corner: {at: NodeRef, scale}}`: attach the source origin to an interior node
  and align source +x with the outgoing tangent.
- `{Cap: {at: NodeRef, scale}}`: attach to an open endpoint, aligning +x outward.
- `{Segment: {start: NodeRef, end: NodeRef, source_length}}`: map the source
  baseline onto the chord between distinct on-curve nodes on one contour.
- `{Brush: {at: NodeRef, spacing, scale}}`: repeat source origins along the
  carrier by arc length, starting at the selected node.

A layer permits at most 128 constructions. Finite scale is in `[0.001,1000]`,
source baseline length in `[0.01,32767]`, and brush spacing in `[0.1,32767]`.
Sources and carrier contours remain editable until an explicit bake. Construction
source references participate in dependency invalidation, cycle detection and
glyph rename history. Native read, write and autosave validate carrier references
and construction parameters while retaining unresolved source glyph names.

UFO stores point and guide IDs as native GLIF identifiers and preserves the
construction/link arrays plus super-smooth point positions in the glyph lib's
`com.fontcarver.constructionData` JSON string (maximum 1 MiB per layer). The GLIF
outline remains the source contour data. Other editable source writers reject
live constructions or node constraints before touching the destination. Existing
live effect/corner source-export restrictions still apply to UFO.

## Directory projects (`.fcfp`)

A `.fcfp` is a directory with `project.json`, `glyphs/`, and a zero-byte
`.fontcarver.lock` advisory lock file. It is an alternative native save target;
existing `.fcf`, `.fcf.gz`, and legacy readers retain their current semantics.
The manifest uses `format: "fontcarver-project"`, `version: 1`, a `font` object
with an empty `glyphs` map, and an ordered `glyph_files` array of `{ name, file }`.
All other `font` fields use the same serde shapes as single-file native documents.
The index preserves glyph display/export order independently of filenames.

Each `glyphs/<name-sha1>-<content-sha1>.json` stores one pretty-printed Glyph with
its final newline. Digests provide deterministic filesystem-safe revision names;
they are not authentication. Readers accept manual text edits without requiring
users to recalculate hashes; the next save emits filenames for the edited bytes.
Readers reject malformed filenames, duplicate names/files, missing or invalid
JSON, embedded glyph-name mismatches, and symlinked managed entries/directories.

A save publishes new, flushed glyph files before atomically replacing the
manifest. Unchanged glyph files remain byte-identical and keep their names. A
single glyph edit replaces only its revision file and manifest entry, keeping
Git changes local. Advisory file locks serialize cooperating writers and protect
readers from obsolete-file pruning. Unix saves also sync the relevant directory
entries. Existing-manifest publication is the commit point: interrupted writes
before it leave the previous snapshot readable; files not referenced by the
manifest are ignored. After publication, only obsolete regular files explicitly
listed in the previous manifest may be removed. Arbitrary files, `.git`, and
unknown orphan files are preserved. Post-commit cleanup failures are warnings.
A crash during the first save can leave a directory without a manifest; retrying
the save completes it, while opening reports the missing manifest.

Recovery remains the ordinary complete `.autosave.fcf` snapshot in the configured
recovery directory, with `document_path` pointing at the `.fcfp` directory. This
keeps a recoverable snapshot independent of damage to any project member. The
lock file and interrupted `.fontcarver-*.tmp` files may be ignored by Git. Native
serialization deliberately does not retain `extra.source.original_bytes`, just
as for the existing single-file formats.

## Virtual master source positions

`font.virtual_masters` is an optional array (default empty, omitted when empty)
containing `{name, location}` records. Names are unique nonempty strings of at
most 256 bytes; locations map four-byte axis tags to finite design coordinates.
There are at most 1024 positions. Completed coordinates must be within mapped
axis ranges and distinct from physical and other virtual master positions.
Virtual positions contain no global metrics, kerning or glyph layers. Creating
a glyph source copies one position into a normal brace layer associated with
the physical default master, so previews and variable export use the existing
sparse/intermediate-source path. Editing or deleting a named position does not
move or remove already-created sources. Axis renames migrate both presets and
source coordinates atomically. Other source formats do not currently retain
the named preset list; compiled fonts contain evaluated source geometry.

P5-20 adds `Image.locked` (boolean, default false) to a layer's optional
`background_image`. Its existing `file_name`, six-coefficient affine `transform`,
optional `color`, and optional PNG `data` bytes remain unchanged. An unlocked
image can be placed, replaced or removed; a locked image must first be explicitly
unlocked. Image placement uses y-up source-pixel coordinates, with the bottom-left
at `(0,0)`, before the affine transform. The image is source-only and does not
alter compiled outlines. Tracing writes ordinary contours while retaining this
source record; preview data and decode/link errors are derived, never persisted.

Linked image names resolve relative to the native document directory, or the
`images/` directory of an opened UFO. Import stores document-relative links and
requires a saved document for non-embedded sources. Save As rebases links in both
the document and undo/redo history only after a successful write, preserving the
same external source. Embedded bytes take precedence over `file_name`. The image
workflow validates static PNG data with 16 MiB compressed, 4096 per-dimension and
one-million-pixel limits; unsupported, missing and invalid sources remain
visible as repairable link errors. Placement commands reject non-finite or
singular matrices and transform coefficients above one million in magnitude.
