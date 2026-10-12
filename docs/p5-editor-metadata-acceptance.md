# P5 path focus, locking, and annotations

Implemented source scope: feature-audit rows 89 and 91.

The glyph inspector provides path focus, persistent path locking/unlocking, and
per-layer review annotations. Focus retains source contour indexes, dims other
paths, and filters click, marquee, keyboard, lasso, and knife eligibility. It is
session-local and clears on glyph/layer handover. Glyph locks are exposed by the
Glyph panel. Locked component and guide hits are omitted without renumbering
editable component indexes.

The Annotation tool draws Text, Arrow, Circle, Rectangle, Plus, and Minus marks.
The inspector chooses the new mark's shape/text, selects existing marks, edits
text and font-space coordinates, and removes marks. Canvas previews and stored
marks use the same renderer. Each creation/edit/removal is one document command.
A gesture cannot apply to a different layer after a source handover. Metadata
uses `Layer.user_data["com.fontcarver.editor"]`; native outlines stay unchanged.

`Command::EditorMetadata` implements glyph locks, path locks, and typed
annotation replacement. Path locks assign persistent contour IDs and follow
reordering. Backend guards reject modifications/removals and preserve atomic
rollback, including layer effects and linked-node resolution. The backend
acceptance record is
[`p5-89-editor-metadata-backend-acceptance.md`](superpowers/plans/tasks/p5-89-editor-metadata-backend-acceptance.md).

## Verification

Focused frontend verification used `pnpm --dir app exec vitest run ...
--maxWorkers=1 --minWorkers=1` with task-owned `TMPDIR`:

- `editorMetadata.test.ts`: 3 passed (namespace validation, stable contour
  references, selection collection).
- `editorGeometryMetadata.test.ts`: 3 passed (locked paths, owner handover,
  locked glyphs, overlapping locked/unlocked components).
- `EditorMetadataPanel.test.tsx`: 3 passed (focus, lock command, annotation
  editing/removal, disabled mutation controls).
- `knifeScope.test.ts`: 2 passed (scoped intersections/cuts, locked glyph).
- `annotation.test.ts`: 8 passed (all six mark types, cancellation, locks,
  refreshed metadata, and source handover). Total: **19 focused tests passed**.

ESLint passed for the new helpers/store/tool/panel/tests. Prettier was applied
only to owned files; shared integration patches remain narrow. No dependencies
were added and no generated API files were hand-edited.

## Acceptance boundary

The 10 backend tests include undo/redo, FCF reopen, invalid-input handling, and
atomic lock enforcement. Their first execution was blocked by another task's
in-flight model edit; the queued retry was cancelled at the root coordinator's
request. The integration owner must execute these tests and regenerate API
types before final TypeScript and workspace checks. No native/browser visual
acceptance is claimed here: the final integration pass owns those screenshots.
Rows 89/91 should remain pending verification until those checks are recorded.
