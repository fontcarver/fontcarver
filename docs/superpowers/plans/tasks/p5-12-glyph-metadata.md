# P5-12 Glyph inventory metadata completion

Scope: audit rows 10–18 are split among bounded child tasks. This task owns
row 12: existing twelve named glyph labels are retained; add separate persistent
layer labels and free-text tag autocomplete from document tags. Also expose the
existing glyph lock field so row 11 lock status is actionable.

Files: layer metadata command and new tests; narrow `Command`/apply registry
additions; new `GlyphTagsField` and `LayerColorField` inspector components;
`GlyphPanel`, `LayersPanel`, English i18n keys and FCF schema documentation.
Existing glyph and layer model fields are reused; layer color is stored in the
namespaced `Layer.user_data` object.

Acceptance: validate labels 0–11, clear independently from glyph color, reject
invalid colors without mutation, preserve unrelated layer data, save/reopen,
undo/redo, lock/unlock through explicit metadata command, keyboard tag suggestion
selection and duplicate trimming. Focused document/API/component tests; parent
runs final workspace lint/regression. Browser evidence after integration.

No commits, no shared status/audit edits, no Chinese translation edits.

## Acceptance checkpoint

Implemented independent layer labels through `SetLayerColor` with validated
0–11 indexes, null clearing, and preservation of unrelated layer metadata.
Added inspector label editing and tag autocomplete (document suggestions,
keyboard selection, free-text entry, deduplication, Escape cancellation, undo
refresh). The glyph lock button uses the outline team's authoritative
`EditorMetadata::GlyphLock` command. Integrated the database panel.

Verification:

- `pnpm exec vitest run .../GlyphTagsField.test.tsx --maxWorkers=1 --minWorkers=1`: 3 passed.
- `pnpm exec vitest run .../LayerColorField.test.tsx --maxWorkers=1 --minWorkers=1`: 2 passed.
- `panelCommands.test.ts`: 15 passed.
- `pnpm exec vitest run .../EditInspector.test.tsx --maxWorkers=1 --minWorkers=1 --testTimeout=20000`: 28 passed in 5.9 seconds. An earlier concurrent run had nine 5-second timeouts; the complete rerun passed (slowest test 450 ms).
- Scoped ESLint and Prettier passed. New Rust test `layer_color.rs` covers
  independent glyph/layer labels, undo/redo, native FCF reopen, invalid index
  rollback and malformed vendor metadata preservation. Its initial compile
  stopped on in-progress shared model edits; final Rust execution is owned by
  root integration and is not claimed here.
- Browser verification, API generation, full lint/regression and independent
  translation are coordinated centrally. No new external dependency or
  FontForge source was used. English namespace: `glyphMetadata`.

Owned test processes exited; scratch directory removed. Final task-boundary
artifact check is recorded by the glyph-inventory coordinator.
