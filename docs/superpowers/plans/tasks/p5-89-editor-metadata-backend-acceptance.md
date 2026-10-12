# P5-89/91 editor metadata backend acceptance

Scope: the backend portion of `p5-89-editor-metadata.md`.

Implemented `Command::EditorMetadata` with typed glyph locking, selected-path
locking, and layer annotation replacement. Annotation kinds cover text, arrows,
circles, rectangles, plus and minus markers. Data lives under
`Layer.user_data["com.fontcarver.editor"]`; unrelated and future metadata remain
intact. Locks assign persistent contour IDs and follow contour reordering.
Central guards reject locked glyph edits, locked contour modification/removal,
duplicate protected identities, discarded lock metadata, and layer-effect
changes. Explicit unlock operations and undo/redo remain available. Component
lock preflight is integrated from the component-lock owner. Linked-node
resolution runs before final path protection validation.

Ten focused integration tests cover annotation validation, unrelated metadata,
FCF round-trip including glyph/path locks, undo/redo, independent path edits,
reordering, duplicate imported IDs, rejected geometry/layer removal, effect
bypass, and nested-batch/rename rollback.

Verification:

- `rustfmt --edition 2021 crates/fc-doc/src/command/editor_metadata.rs crates/fc-doc/tests/editor_metadata.rs`: passed.
- `git diff --check` for owned files and narrow shared edits: passed.
- `cargo test -p fc-doc --test editor_metadata`: first attempt blocked by a
  concurrent `fc-model::Layer.metrics_keys` definition/default mismatch.
  The next attempt remained queued on the shared Cargo lock and was cancelled
  at the root's request (exit 143). Tests are **pending**, not passed.
- `/root/rust_integration` was notified to run the focused test and regenerate
  API types with the final integration check. No generated files were edited.
- Final artifact check: build 6.11 GiB / 13,180 files; temporary 0.22 GiB /
  7,960 files; workspace free 668.66 GiB; `PASS`.

No dependencies or model fields were added. Source compilation and focused
test results remain the integration owner's acceptance requirement. The
task's queued process was stopped and its empty test scratch directory removed.
The parent UI task's `.scratch/p5-editor-metadata/en.json` was preserved.
