# P5-130 — Bounded font metadata editor

## Scope and ownership

Add `FontMetadataEditor.tsx`, `fontMetadataSession.ts`, a scoped CSS module and
focused tests under `app/src/views/fontinfo/`. Add only the `fontMetadata` English
namespace. Parent owns FontInfoPanel integration and independent localization.
No model, command, generated API, master metrics or imported name-record editing.

## Public interface

`FontMetadataEditor({ doc, onClose })` stages family/style, UPM, version and
existing typed identity/legal strings. `FontMetadataSession` registers as a
document edit participant and persists one `SetFontInfo` with existing
`doc.edit_begin/apply/end`. UPM edits explicitly do not scale geometry.

Before replacing metadata, re-read within the edit token and merge only changed
fields. Reject conflicting edits to those fields while preserving the draft;
preserve all fresh untouched metadata, including imported names and user_data.
Close applies; Discard explicitly abandons local edits; errors keep the editor
open. Save/history/navigation drain the registered participant.

## Acceptance

Write failing session/component tests first. Verify validation, untouched-field
preservation, concurrent change/conflict detection, failed apply/end recovery,
single transaction, participant flush/suspension and UI close/discard behavior.
Run focused Vitest, ESLint and Prettier checks; parent runs final TypeScript and
full app regression after integration. Backend SetFontInfo undo/persistence
behavior is reused, not expanded. Record absent native/backend/visual evidence
honestly. Check artifact budget at start and completion, remove owned scratch.
