# P5 proof preview controls

Owner: proofing/preview_controls. Scope: `PreviewStrip.tsx`, `PreviewCanvas.tsx`,
`previewPaint.ts`, `PreviewStrip.module.css`, optional `PreviewStripProps`, and
focused tests. Parent owns TextView, UI-store persistence, English strings, and
browser acceptance. No kernel or generated API changes.

Implement horizontal/vertical reflection, the existing independent inversion
theme control, bounded blur, validated custom waterfall sizes, single selected
glyph preview, optional detach action, and a pointer/keyboard resize divider.
Existing callers remain compatible. New public props are optional `activeGlyph`,
`onDetach`, `height`, and `onHeightChange`.

Acceptance: component tests cover controls and callback routing; recording-canvas
tests verify reflected transforms, blur, single-glyph positioning and custom
size scaling. Invalid sizes retain the current preview and explain correction.
Resize supports pointer input and keyboard bounds. Run focused Vitest and ESLint;
parent runs final full frontend checks after integration. Check disk artifacts at
start and completion and remove this task's temporary files.
