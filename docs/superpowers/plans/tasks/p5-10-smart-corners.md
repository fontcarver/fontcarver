# P5-10: live smart corners

Follow P5-09 with persistent per-node smart-corner radii. A positive radius rounds
the join; a negative radius constructs an ink trap. Keep the original nodes and
handles editable and evaluate the derived outline for preview, components,
metrics, rasterization and compilation. This addresses audit row 84, not the
separate corner/cap/segment/brush-component system in row 83.

## Data and geometry

Store corner intent with its source on-curve point so insertion, deletion and
reordering do not leave stale numeric indices in a separate side table. Add
backward-compatible serialization defaults and document the native schema.
Validate finite radius limits, point roles and geometry budgets. Reject
unsupported or degenerate joins explicitly rather than silently removing the
effect. Define the treatment of oversized radii and neighboring corners before
exposing a numeric control.

Keep evaluated geometry in fc-geom; model-only source queries retain the original
outline. Resolve corner geometry before the layer-effect stack and before a
component's placement transform. Local smart interpolation must handle radius
metadata consistently, and global interpolation must report incompatible output
topology. Do not reuse original point-bound hints on baked geometry.

## Editing and persistence

Provide numeric radius editing for selected on-curve nodes with one undoable
command per committed edit. Zero removes the live corner. Include a visible
distinction between round and trap behavior and an explicit bake path. Maintain
FCF/recovery persistence, dependency invalidation and revision-safe UI behavior.
Other source writers must preserve the metadata or reject before writing.

## Verification

Start with analytic line-line corners, convex and concave joins, both windings,
open/closed contours, radius bounds and adjacent-corner collisions. Include
curve-join behavior or explicit rejection coverage. Verify node moves, insertion,
deletion, undo, save/reopen, nested components and compiled outputs against
independent geometric expectations. Use real outlines and isolated bilingual
headless-browser editing; native/global desktop input remains prohibited.

Do not mark the full audit row complete solely from a geometry helper or a new
control. Record supported corner constructions and remaining limitations in the
acceptance report, then continue the rest of P5 without stopping at this boundary.
