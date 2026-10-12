# P5 rows 26–27: extrema and path operations

## Scope

Add a distinct Force Extremes operation that splits interior extrema even when
they lie within the normal 0.01-unit endpoint tolerance. Preserve closed-path
geometry when breaking at a node, and reconnect any pair of open endpoints,
including the ends of one contour. Add all-master tidy/direction commands as one
undoable batch; direction also canonicalizes contour starts and order.

## Files and interfaces

- `crates/fc-geom/src/edit.rs`, re-export in `lib.rs`, focused extrema tests.
- `crates/fc-doc/src/command/outline/segments.rs`, new focused path-operation tests.
- Coordinated additive `PathOp::ForceExtrema` in `command/options.rs`.
- `app/src/views/glyph/pathOps.ts`, `pathCommands.ts`, and their focused tests.
- English keys and generated API types are owned by the integration parent.

Keep existing command shapes for split/join. Newly duplicated split nodes must
not duplicate stable IDs. Preserve undo snapshots and validate malformed input.
The spec's triangular-overlap Open Corner is separate from Break Contour and
must not be claimed complete by relabeling the latter.

## Acceptance

Write regressions before implementation: near-endpoint forced extrema;
shape-preserving closed split and same-contour reconnect; start-start endpoint
join; invalid endpoint rejection with unchanged document; undo/redo and native
FCF persistence. Frontend tests assert one command/undo batch and master-only
scoping. Run focused tests under the shared Cargo lock and report exact results;
the parent owns final workspace Clippy/check/lint/regressions. Check artifact
budgets at start/end and remove owned scratch outputs.
