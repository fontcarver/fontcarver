# Bitmap gesture safety and navigation

Date: 2026-10-02. User authorized actual UX evaluation and improvement.
Read CONVENTIONS.md. Audit probes reproduced bitmap pointercancel committing
strokes, Escape leaving strokes active, and middle dragging not panning.

Own bitmap interaction/controller/canvas files and their meaningful regressions;
coordinate exact files with parent and w3_review_editing. Repair cancellation and
expected middle/Space navigation consistently with outline behavior, preserving
right-click eyedropper if intentional and existing tools/undo/locks. Avoid native
desktop automation and broad builds. New English text needs separate GPT-6.1 Sol
translation. Write concise acceptance in docs/p5ux-bitmap-input-acceptance.md,
with actual commands/results; cleanup owned scratch, check resources, no commits.

## Gap analysis and owned implementation

The reviewer reproduced `pointercancel` committing a stroke, Escape leaving its
gesture alive, and middle-button drag being ignored. Own `BitmapCanvas.tsx`,
`BitmapCanvas.test.tsx`, `BitmapEditor.tsx`, `bitmapSession.ts` and
`bitmapSession.test.ts` under `app/src/views/bitmap/`.

Cancellation must restore both pixels and selection plus the pre-gesture dirty
state, otherwise a later Save persists a cancelled stroke. Preserve any earlier
failed-write draft. Normal pointer-up commits once; capture loss afterwards must
not undo it. Space and middle-button navigation must retain the chosen tool and
work while editing is disabled. No existing right-button eyedropper binding was
present to preserve.

Independent review additionally reproduced load/cancel checkpoint races. Seal
the canvas gesture synchronously when a save barrier accepts its draft, and
defer document-event reload while a selection-only checkpoint exists, refreshing
after completion/cancellation. This keeps pixels, generation and busy state
coherent.
