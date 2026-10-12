# P5-08: batch style effects

Implement actual selected-glyph style effects: anisotropic weight expansion or
contraction, condense/extend, oblique, small caps and sub/superscript transforms.
Use the existing permissive geometry kernel, not FontForge. Presets remain
editable numeric settings. Preview the captured selection at one master or all
masters and apply the same settings as one revision-checked undo transaction.

Flatten each selected source from the original document before applying any
mutation, transform anchors, offer advance scaling or CJK cell preservation
and centering, and clearly describe component decomposition. Optional backup
layers contain resolved original geometry. Bound per-layer/batch work; reject
locked glyphs, bad inputs, stale previews and unrecoverable booleans atomically.
Report the existing polygon/curve-refitting fallback in preview, with every
affected glyph/master named; independently check the final filled result.

Deliver kernel tests (bounds, counters, contraction, invalid parameters), API
preview/apply/stale/undo/native/export tests, a working bilingual dialog from
font selection and the command menu, browser checks and acceptance notes.
This block does not claim to implement the later non-destructive effect stack,
smart geometry, autotracing or optical spacing work.
