# P5 named glyph lists API

## Scope and owned files

Persist named sidebar glyph lists using the shared inventory metadata helper.
Own `crates/fc-api/src/types/glyph_lists.rs`,
`crates/fc-api/src/handlers/glyph_lists.rs`, and
`crates/fc-api/tests/glyph_lists.rs`, plus narrow registry additions.
Shared inventory storage is owned by the smart-filter persistence task.

## Public interfaces

- `inventory.get_lists({doc})` returns `{lists: GlyphListView[]}`.
- `inventory.set_lists({doc, lists: GlyphList[]})` atomically replaces lists and
  returns the same view shape.
- Entries contain stable `id`, display `name`, and ordered exact `glyphs` names.
  Views additionally contain ordered `present` and `missing` glyph-name arrays.
- Persist at `font.info.user_data["com.fontcarver.inventory"].glyph_lists`.
- Bound collection sizes, reject duplicate IDs and recipe syntax, trim display
  names, and deduplicate glyph names while retaining first occurrence order.
- Changes preserve unrelated metadata, participate in one undo step, and survive
  native FCF save/reopen. Reads classify presence using current document glyphs.

## Acceptance

Write focused integration tests before implementation: empty/default lists,
presence and exact names, normalized duplicates, atomic invalid inputs,
unrelated metadata preservation, no-op updates, undo/redo, and native persistence.
Parent owns the Cargo build/test slot and broad checks; this task does not start
Cargo processes independently. Format owned files and run artifact checks at
start and completion; report test commands for the parent's validation batch.
