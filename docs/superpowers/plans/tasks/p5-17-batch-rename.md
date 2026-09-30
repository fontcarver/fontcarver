# P5 glyph inventory: batch rename

Scope: provide find-and-replace for the grid's selected glyph names with a
before/after preview, explicit collision/invalid-name feedback and one undo step.
Reuse the public `doc.apply` API and `Command::Batch` with collision-free temporary
names so swaps and rename chains preserve stable handles and references.

Owned files: new `app/src/dialogs/BatchRenameDialog*`,
`app/src/dialogs/batchRename*`, `crates/fc-doc/tests/batch_rename.rs`,
`crates/fc-api/tests/glyph_recipes.rs`, and the glyph context menu. Coordinate
narrow DialogState/DialogHost integration patches with the parent. English keys
use `dialog.batchRename`; an independent agent owns Chinese translation.

Acceptance: literal find/replace preview; unchanged names omitted; invalid names,
occupied destinations and duplicate destinations rejected; rename chains and
cycles planned without clobbering glyphs; atomic command rollback, undo/redo,
stable handles and component/kerning references covered by kernel tests; dialog
success/error behavior covered by frontend tests. Verify existing Add Glyphs
supports literal names, characters, Unicode codepoints and ranges in mixed
recipes. Focused tests use the shared Cargo lock; parent runs broad integration
checks and browser acceptance after combined grid changes.

No model/schema changes, new dependency, regex replacement or automatic glyph
production-name changes are included.
