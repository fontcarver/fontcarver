# P5-09: non-destructive layer effects

Status: implemented and verified on 2026-09-28 within the documented uniform-pen
and stored-mask boundaries. See [acceptance](../../../p5-layer-effects-acceptance.md).
Continue with smart geometry and the remaining P5 sequence.

Add an ordered, persistent layer-effect stack with enable/disable, reorder,
remove and bake actions. Initial operations are uniform round/elliptical or
rectangular stroke (editable width/height/angle/caps/joins), overlap removal,
and filled boolean operations against a stored mask outline. Source contours,
components and anchors remain editable. A mask can be captured from another
layer/glyph or supplied as bounded SVG path data; the UI must make its stored-copy
semantics clear. Variable pen-point interpolation and smart construction are the
next block; do not claim audit row 87 complete from uniform strokes alone.

## Architecture

Keep persistent data and validation in fc-model and evaluation in fc-geom. Extend
model outline traversal with an explicit post-resolution layer callback, applied
recursively to component/smart sources before the parent component transform.
The existing model-only path query remains a source-geometry query. Expose
fc-geom evaluated queries and route rendering, document path caches, metrics,
bitmap rasterization, color previews and compilation through them. Do not add a
model-to-geometry dependency cycle or a global mutable evaluator registration.

Compiled output evaluates effects on an export copy. Native source save retains
the stack and original geometry. Other editable formats must preserve the stack
and source in metadata or explicitly reject it until their representation is
implemented; no silent flattening or omission. Evaluate effects after local smart
source interpolation, requiring compatible stack definitions where needed.

Add serde defaults, FCF schema documentation, bounded validation, history memory
accounting, and cache/dependent invalidation. Unrecoverable evaluation errors are
visible. Curve-refitting fallbacks are disclosed before applying a draft/bake.
A revision-checked draft preview/apply API must not mutate source geometry.

## Editing and verification

Provide an actual active-layer editor with operation controls, a visual before /
after preview and explicit bake. Display evaluated geometry in the editor while
keeping source nodes selectable. Grid, text, component and export geometry must
agree. Capture the current document/layer and revision; reject stale writes.

Write model/kernel/document/API regression tests first. Cover old-file defaults,
invalid/excessive effects, stack ordering, enabled flags, components, source edits
under active effects, cache invalidation, bake/undo, native reopen and independent
TTF/OTF/WOFF/WOFF2 decoding or rendering. Verify the bilingual UI in an isolated
headless browser; no native/global desktop input automation.
