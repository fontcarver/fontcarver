# P5-11 variable pen points

## Scope and ownership

Kernel: persistent `VariableStroke` layer effect, validated normalized pen points,
bounded geometry, existing preview/apply/bake/export integration and schema.
Frontend is separately owned by `variable_pen_ui`; existing effect endpoints
remain the public API. No FontForge source or new dependency is used.

## Contract

`kind: variable_stroke` has `nib`, `cap`, `join`, `miter_limit`, `progression`
(`linear` or `smooth`) and `points: [{position,width,height,angle}]`.
2–64 strictly ordered positions span exactly 0 to 1 and refer to normalized
centerline arc length independently for every contour. Dimensions/angles/miter
limits match fixed strokes. Smooth progression is cubic smoothstep; angle follows
the shortest angular interval. Closed paths require the endpoint nibs to match
(angles modulo a full turn). Source topology remains editable; points resample
against the current source length. Components resolve before local effects.

## Geometry and boundaries

Sample centerlines and changing nib support with explicit work bounds, assemble
filled span/cap/join polygons, resolve their union, and fit the resulting outline.
Treat open and closed paths, degenerate spans, both nibs, all caps/joins and
limited miters. Return approximation information through the existing effect
outcome. Reject excessive work or incompatible closed seams transactionally.

## Acceptance

- Model round trip and rejection of nonfinite, unsorted, duplicate, excessive and
  out-of-range pen parameters, including disabled entries.
- Geometry verifies taper/progression/rotation, bounds, cap/join behavior, closed
  counters and thick closed centers, no source mutation, and deterministic bake.
- Document tests cover native persistence, undo/redo, source editing, dependent
  invalidation and failed edits without mutation.
- Compiler tests compare exported evaluated geometry and untouched native source.
- Focused Cargo tests serialize through `.scratch/p5-resume/cargo.lock`;
  parent runs final workspace format/Clippy/regression checks.
