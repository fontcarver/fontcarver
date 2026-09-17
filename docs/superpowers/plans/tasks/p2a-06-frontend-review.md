# P2a independent frontend review

## Scope

Read-only review of text kerning controls, shaped gap identity and rendering,
edit sessions, group inspector commands, and editor/document handover. Exclude
unrelated existing workspace changes. Product fixes remain with the feature
authors; this task owns only this task file and `docs/p2a-frontend-review.md`.

## Interfaces

- `useTextKerningControls` / `useTextKerning` / `TextKerningPanel`.
- `KerningEditRunner`, `EditSession`, and document/edit-controller barriers.
- `textGaps`, `TextGapPicker`, `drawTextGaps`, and `GlyphCanvas`.
- Group panel creation, queries, mutation barriers, and explicit deletion.

## Acceptance

Inspect stale-response identity, captured edit targets, zero/removal semantics,
keyboard routing, cancellation, and bounded accessibility. Report concrete
defects to the root and responsible authors. Run the focused existing tests
without altering product files. Record findings and verification limits; this
review does not replace the integrated headless workflow or localization QA.
