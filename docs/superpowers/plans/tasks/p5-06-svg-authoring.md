# P5-06: static SVG color documents

Implement named glyph-to-element bindings for shared UTF-8 SVG documents,
version-0 OpenType SVG import/export with bounded gzip decoding, and explicit
raw-table ownership. Follow the [OpenType SVG specification](https://learn.microsoft.com/en-us/typography/opentype/spec/svg).
Unsupported XML and malformed tables remain opaque and preserve their original
bytes under the existing export preservation policy.

Scope: static paths, basic shapes, groups, local use references, gradients,
clipping, masks and patterns; presentation attributes and simple inline CSS.
Embedded raster images, filters, stylesheets, animations, external references,
DTD/entity declarations and foreign content are outside this editable subset.
The subset is explicit; it does not complete COLRv1 or general SVG file import.

Model: `Font.svg: Option<SvgTable>` and `svg_table_owned`. Documents store XML
and named bindings, preserving IDs independently of glyph ordering. Export
rewrites IDs and local references to the actual compiled glyph order, including
shared discontinuous ranges and duplicate bindings. Name changes, deletion,
exclusions and production names must not redirect artwork to another glyph.

UI/API: revision-checked `svg.state`, `svg.apply`, `svg.decode_imported`, native
undo/redo/save/reopen; source and binding editor in Font Info with isolated
static image previews. Invalid drafts must survive attempted close/save.

Verification: malformed/truncated/gzip input, XML ID/reference limits, bounded
large glyph order, raw fallback, atomic failure, ownership, native/UFO
persistence, four font containers, independent FontTools/XML comparison,
bilingual browser workflow and formatting/regression checks. New dependency:
roxmltree (MIT OR Apache-2.0) for robust bounded XML parsing, licensed in
THIRD_PARTY.md. No FontForge code.
