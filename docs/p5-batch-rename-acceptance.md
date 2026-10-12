# P5 batch rename acceptance

Implemented selected-glyph literal find/replace with before/after pagination,
whole-font name collision validation, invalid-name feedback and a single atomic
`doc.apply` Batch. All sources move through fresh UUID-prefixed temporary names
before receiving their final names; the planner also supports cycles and chains.
The dialog reloads the inventory before applying and requires another review if
names changed. Failure retains the dialog and its preview. The shell registers
`glyph.renameBatch`; the grid menu integrates the same command.

English UI keys: `dialog.batchRename`. Chinese translation is independently
owned. No dependencies, model schema, API methods or Rust command variants added.

Focused tests:

- `pnpm --dir app exec vitest run src/dialogs/batchRename.test.ts
  src/dialogs/BatchRenameDialog.test.tsx`: initial 12/12 passed. A final rerun
  under heavy concurrent load hit one 5-second test timeout. Final serial rerun
  with `--testTimeout=20000 --maxWorkers=1`: **12/12 passed**, 2 files, 6.42s
  (dialog tests 719ms; planner tests 8ms).
- Focused ESLint of both implementation files and both test files: passed.
- `rustfmt --edition 2021 crates/fc-doc/tests/batch_rename.rs
  crates/fc-api/tests/glyph_recipes.rs`: passed.
- `cargo test -p fc-doc --test batch_rename -p fc-api --test glyph_recipes`:
  delegated to the centralized Rust integration owner. The local lock waiter
  was terminated before compilation at the parent's request; it is not a pass.

Kernel tests cover swap identity, component/kerning references, FCF roundtrip,
undo/redo, atomic rollback and mixed Add Glyphs recipes containing names,
characters, Unicode codepoints and ranges. The production Add Glyphs parser was
already present; this task adds explicit acceptance coverage.

Boundaries: regex replacement is outside scope. The workflow inherits existing
RenameGlyph reference semantics; tests specifically assert component/kerning
references, not every format-specific record. UUID temporary names avoid
predictable collisions with dangling references, while the kernel still rejects
unresolved kerning/MATH collisions. Browser, translation and broad frontend/Rust
acceptance belong to parent integration and are not claimed here.
