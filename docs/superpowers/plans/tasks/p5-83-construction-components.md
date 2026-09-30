# P5-83 reusable construction glyphs

Scope: persistent, undoable layer constructions referencing a source glyph and stable
layer-local nodes. Corner attachments align to the outgoing node tangent; caps align
outward at open endpoints; segments map source baseline `(0,0)..(source_length,0)`
to two nodes; brushes repeat the source at explicit arc-length spacing along the
contour beginning at the selected node. These are affine glyph placements, not
curve-deforming or automatic intersection/trim constructions. `keep_carrier=false`
hides the referenced carrier contour while retaining it as editable source.

Public model: `GlyphConstruction { id, source, kind, keep_carrier }`,
`ConstructionKind::{Corner,Cap,Segment,Brush}`, `Layer.constructions`.
Use the sibling stable `NodeRef { id }` model, resolving missing or ambiguous nodes
as explicit errors. Bound constructions, stamp count, coordinates, recursion and
geometry work. Existing flatten recursion resolves source glyphs before geometry
placement, sharing cycle and work limits with components. Local constructions run
before effects. Compile/export evaluates and bakes a copy; FCF retains intent.

Owned files: new fc-model/fc-geom construction modules; narrowly scoped query,
Layer/effects, dependency/rename/source metadata, document command and API hooks.
Parent owns editor UI and acceptance integration tests. Commands/API and source
persistence delegated to child agents with disjoint implementation modules.

Acceptance: durable references after reorder; dangling node and source diagnostics;
corner/cap orientation and endpoint restrictions; segment placement; brush spacing
and bounds; mixed component/construction cycle rejection; source changes invalidate
consumers; undo/redo; source persistence and compile baking. Native/reference source
contours stay unchanged during evaluation. Run workspace formatting, appropriate
Rust tests and clippy through the shared Cargo lock. Check artifact budgets at start,
before/after builds and completion. No commits or shared roadmap status edits.
