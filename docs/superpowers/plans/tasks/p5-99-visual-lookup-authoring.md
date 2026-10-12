# P5-99 visual lookup authoring

## Scope and ownership

Add editable source-backed lookup forms for GSUB single, multiple, alternate,
ligature and contextual substitutions, and GPOS single, pair, mark-to-base and
mark-to-mark positioning. Keep imported binary layout on the existing proven
conversion path. Supported feature bodies round-trip through a bounded typed IR;
unsupported source remains accessible in the source editor without rewriting.

Own new `app/src/views/features/VisualLookup*` UI, the `visualLookup` helper,
focused tests, and a narrow `FeaturesView` integration. Reuse
`FeatureDraftSession` so edits persist as drafts, compilation activates only a
valid candidate, and document history/save barriers remain authoritative. No
new dependencies, API methods, or model schema changes for the visual editor.
Backend feature preprocessing and editor diagnostics are separately delegated.

## Implementation and acceptance

1. Write parser/serializer and component behavior tests before implementation.
2. Add nine typed forms, feature identity selection, add/remove, source preview,
   validation and source-editor navigation. Persist intermediate edits as FEA
   drafts; keep unsupported and generated blocks protected.
3. Verify source-to-form round-trip, invalid source protection, draft persistence,
   checked apply, and external history reload. Backend acceptance compiles and
   shapes all nine source patterns and tests undo/native persistence.
4. Run focused Vitest/Rust acceptance, typecheck changed integration, and isolated
   headless browser verification when the shared devserver is available. Parent
   owns final full regression, lint and Clippy. Check artifact budget at start,
   around builds and before completion; clean owned temporary outputs.

## Boundaries

The forms cover bounded individual rules and mark definitions. Arbitrary lookup
flags, language systems, named contextual lookup graphs, device/variation value
records, and feature bodies containing mixed unsupported syntax remain editable
as FEA. Do not label this a lossless parser of all OpenType feature syntax.
