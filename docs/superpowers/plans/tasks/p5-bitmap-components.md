# P5 bitmap reference editing and integer grid presets

Scope: preserve bitmap component references in editable sessions, expose the
resolved preview separately, add reference add/offset/remove/flatten controls,
and offer ppem choices with integer font units per pixel for the current UPM.

Owned files: `app/src/views/bitmap/BitmapReferences.tsx`,
`BitmapReferences.test.tsx`, `BitmapGridPreset.tsx`, `BitmapGridPreset.test.tsx`,
`bitmapSession.ts`, `bitmapSession.test.ts`, and the `bitmapReferences` /
`bitmapGrid` groups in `app/src/i18n/en.json`.

Public interfaces:

- `BitmapReferences({ session, value, disabled })` edits the raw `BitmapGlyph`
  through its `BitmapSession` and participates in document draft flushing.
- `BitmapDraft.resolved` exposes the kernel's composed bitmap independently of
  the editable `BitmapDraft.value`, which retains component references.
- `BitmapGridPreset({ unitsPerEm, value, onSelect, disabled? })` emits a ppem;
  `integerGridPresets(unitsPerEm)` exposes exact-divisor presets.

Acceptance: reference edits preserve original pixels/metrics and references;
focused offset drafts flush on document Save; flatten uses resolved pixels;
failed saves retain the draft; 1000/1024/2048 UPM presets always have integer
units per pixel; custom ppem remains visible. Run targeted Vitest first, then
coordinate the broad app check/lint/test with the owning bitmap integration
agent. Browser acceptance belongs to that integrated editor task.
