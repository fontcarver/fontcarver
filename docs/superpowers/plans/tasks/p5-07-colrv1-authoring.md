# P5-07: static COLRv1 paint graphs

Extend existing atomic COLR/CPAL ownership with validated static paint graphs:
layers, solid colors, linear/radial/sweep gradients, outline and color-glyph
references, affine/translate/scale/rotate/skew transforms, composites and clip
boxes. Preserve COLR v0 fallbacks alongside v1 roots and share CPAL palettes.
Variable paints remain opaque until a complete variation-store editing path
exists; never decode only the v0 fallback and discard the rest of a v1 table.

Use named glyph references and graph node indices, bounded graph parsing,
cycle/expansion checks, safe packed-offset writing, actual export order and
explicit ownership. Renames, duplicates, exclusions and undo must maintain the
graph or clearly reject/drop affected exported roots with a warning.

Provide a graph source editor alongside existing palette/layer controls, with
validated transactional apply/save/undo and a static preview. Tests cover all
static formats, malformed offsets and cycles, palette references, ownership,
persistence and four export containers. Independently verify exported paint
semantics and renderings with FontTools/a second renderer, then browser-check
English/Chinese and both themes. Use the OpenType COLR/CPAL specifications;
no new dependency or FontForge implementation.

Verified within the static boundary on 2026-09-28; see
[acceptance](../../../p5-colrv1-acceptance.md). Next: geometry/construction.
