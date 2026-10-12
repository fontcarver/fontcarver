# P5 proof-library native persistence acceptance

## Scope and ownership

Add only `crates/fc-doc/tests/proof_library.rs` and the corresponding acceptance
report. Verify the proof-library frontend's existing persistence contract:
`FontInfo.user_data["com.fontcarver.proofLibrary"]` contains version 1 and entries
with id, name, group, script, language, direction, and text. No model, schema,
production API, dependencies, or frontend changes are in scope.

## Acceptance

- Plain `.fcf` and compressed `.fcf.gz` saves preserve the complete library,
  Unicode text, unrelated user metadata, and font data semantically.
- `SetFontInfo` library addition, editing, and clearing each have independent
  undo/redo behavior and preserve unrelated metadata and font fields.
- Existing APIs satisfy the new regression tests without implementation changes.

Run only the focused Rust integration test under the shared Cargo lock with
absolute TMPDIR `.scratch/p5-proof-library-rust`, and format only the new file.
The parent owns final broad Clippy and regression checks. Check artifact budgets
at task start and completion, and remove only this task's scratch directory.
