# P5-89/91 Path focus, locking, and annotations

## Scope

Complete the outline editor's per-layer annotations, session-local path focus,
and persistent glyph/path locking. Shared registries, controller, localization,
and inspector insertion are integrated by the outline-editor owner.

## Files and interfaces

- `fc-doc` adds `EditorMetadata` commands for glyph locking, path locks, and
  typed per-layer annotation replacement; metadata is namespaced inside
  existing `Layer.user_data`, with locked paths keyed by persistent contour ID.
- New editor modules manage focus/annotation state, pure metadata access and
  annotation drawing; a panel edits text, arrows, circles, rectangles and locks.
- A new annotation tool returns an annotation request after a canvas gesture;
  the panel supports precise numeric placement and editing.
- Existing contour, glyph, and layer structs remain compatible.

## Acceptance

First add focused tests covering metadata isolation, focus eligibility,
annotation geometry, validation, locking rejection and undo/redo. Verify
native persistence through FCF reopen. Run focused frontend/backend tests;
parent performs final integrated checks and UI screenshots. Reject locked
geometry mutations at the kernel boundary, preserve unrelated user metadata,
and clear focus when glyph/layer changes.
