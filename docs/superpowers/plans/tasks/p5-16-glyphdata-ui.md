# P5-16 Glyph database inspector UI

## Scope and ownership

Add `GlyphDatabasePanel.tsx`, its CSS module and focused component tests under
`app/src/views/glyph/panels/`. Append English `glyphDatabase` translations only.
Parent owns inspector/menu integration, backend methods and generated API types.

## Public interfaces

Accept `PanelContext`. Read effective metadata through `glyphdata.state` with
`{ doc, gid }`; use the returned generation for `glyphdata.set_overlay` and
`glyphdata.update`. Overlay editing uses JSON paste and an accessible local file
input with `File.text()`. No direct Tauri or filesystem calls.

## Behaviour and acceptance

- Show effective category, script, production name, default anchors and component
  composition, and distinguish custom overlay metadata.
- Update the current glyph with one generation-checked, undoable API call.
- Save document-specific JSON overlays; file import fills the draft for review.
- Preserve unsaved draft text across background refreshes and failed operations;
  explicitly reloading saved JSON is the only discard action.
- Show request failures and generation conflicts visibly; ignore late responses
  after changing document/glyph or unmounting.
- Focused tests cover lookup, updating, overlay save/import, invalid JSON,
  background refresh/draft retention, stale requests and transport failures.

## Verification

Run focused Vitest, ESLint and Prettier checks; parent coordinates generated API
types and the final `pnpm check && pnpm lint && pnpm test` batch. Parent/root owns
interactive browser verification of the integrated inspector. Run artifact
checks at start and completion and clean only this task's scratch files.

## Acceptance result

- `pnpm exec vitest run src/views/glyph/panels/GlyphDatabasePanel.test.tsx`:
  `Test Files 1 passed (1); Tests 8 passed (8)`.
- Scoped ESLint passed with exit 0. Owned TSX/CSS files formatted with Prettier.
- File imports reject files larger than 4 MiB before reading. The inspector also
  documents version 1's name-keyed glyph mapping with an expandable example.
- Final artifact check passed: build 6.07 GiB / 13,162 files, temporary 0.05 GiB /
  4,502 files, free 669.16 GiB. No task-owned processes or scratch data remain.
- Generated-type validation, full frontend checks, localization and integrated
  browser review remain assigned to parent/root, as coordinated.
