# P5 native foundation: labelled gestures and crash recovery

## Scope and ownership

Complete the bounded foundation audit for labelled one-gesture history and
recoverable native snapshots. Own `crates/fc-doc/src/autosave.rs`, new
`crates/fc-doc/src/autosave_tests.rs` and isolated
`crates/fc-doc/tests/history_recovery.rs`; do not modify native format, session,
document state or UI modules owned by other agents. No public API changes.

## Existing behavior

`commands_undo.rs` already verifies labelled coalescing, undo/redo dirty-state,
gesture finalization, history branching, budget eviction and atomic batches.
`edit_sessions.rs` covers 50-preview exact undo, preview/apply equivalence and
stale tokens. Frontend `drag.test.ts` covers Escape end+undo, an in-flight apply
and cancellation before any successful apply. `session.rs` covers dirty-unsaved
autosave, worker lock scope, metadata listing performance and legacy snapshots;
`feature_drafts.rs` verifies recovery retains an invalid feature draft.

## Implementation and acceptance

1. Add regression cases before the fix: recovery excludes syntactically valid
   envelopes whose font cannot be read, including semantically invalid bitmap
   payloads. Preserve valid/legacy snapshots, freshness filtering and ordering.
2. Validate eligible recovery fonts through the native reader before presenting
   them in the recovery list. Keep the cheap original freshness filter first.
3. Verify gesture event labels, exact undo/redo, failed previews preserving a
   pending redo, failed previews inside a live gesture, empty cancellation and
   end+undo cancellation restoring the saved state.
4. Run focused integration tests under the shared Cargo lock, then the relevant
   existing history/session/draft tests. Parent owns final workspace formatting,
   Clippy and crate regression; use narrow rustfmt for owned files.

Use `.scratch/p5-history-recovery/` with an absolute TMPDIR and the workspace's
space-saving profiles. Check artifact budgets before tests and at completion,
remove only this task's temporary output, and report evidence to the parent.

## Acceptance handoff — 2026-09-30

Implemented native-read validation before listing an eligible recovery snapshot.
Malformed font payloads and semantic errors can no longer pass by virtue of a
valid metadata envelope. Existing original freshness filtering runs first.
Project freshness compares the committed `project.json` manifest instead of
directory mtime, so interrupted writes cannot hide a newer recovery snapshot.
Added six integration tests for corruption, provenance/freshness/order/legacy,
project manifest freshness,
gesture event labels and failed previews, preserved redo after empty or failed
gestures, and end+undo cancellation restoring the saved state. Three deterministic
unit tests cover last-good snapshot preservation, continued saves for other
documents, recovery after repair, clean-document skipping, provenance, retained
dirty state and prompt worker shutdown. No dependencies or public APIs changed.

Verification completed:

```text
rustfmt --edition 2021 --check crates/fc-doc/src/autosave.rs crates/fc-doc/src/autosave_tests.rs crates/fc-doc/tests/history_recovery.rs
exit 0
git diff --check -- crates/fc-doc/src/autosave.rs crates/fc-doc/src/autosave_tests.rs crates/fc-doc/tests/history_recovery.rs docs/superpowers/plans/tasks/p5-08-history-recovery.md
exit 0
python3 scripts/check_artifacts.py
build: 6.11 GiB, 13,180 files
temporary: 0.21 GiB, 7,037 files
workspace free: 668.73 GiB
PASS
```

The focused Cargo command was cancelled while still waiting for the shared lock
(exit 143, no compiler/test process). Parent owns its replacement combined
`fc-doc --lib` plus `directory_project`, `fcf_roundtrip`, `history_recovery`,
`session`, `edit_sessions`, `commands_undo` and `feature_drafts` tests, followed by
final workspace checks. Rust runtime acceptance remains pending that batch;
no pre-fix failure or passing tests are claimed here. The existing optimized
recovery-listing performance assertion remains unchanged and was not measured.
