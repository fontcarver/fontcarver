# P2b 02 — Feature drafts, snapshots and persistence

Date: 2026-09-17. Status: implemented, reviewed and integrated; see final P2b acceptance.
Dependencies: 01.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`fc-model` feature/draft types; `fc-doc/src/command*`, `undo*`, `document.rs`,
`fcf/schema.md` and FCF/autosave tests.

## Interfaces and work

Persist optional draft source and generation settings separately from active
`Font.features`. Draft updates carry expected revision, cannot change layout
authority, and coalesce through the document history. Add feature snapshots and
source-change signals; do not treat existing kerning snapshots as complete.

Define an effective compile revision that excludes draft-only changes but
retains all existing shaping invalidation causes. Provide a typed trusted
commit path for validated source/conversion changes, not a deserializable
unchecked authority command. Only APIs in task 03/08 may invoke it after
validation. Capture active source, draft and conversion/lift metadata together.
FCF schema defaults preserve old documents; native autosave retains invalid text.

## Acceptance tests

Invalid drafts survive FCF/gzip/autosave/reopen; opening a source view stays
clean. Test coalescing, no-op, stale draft revision, undo/redo and aborted batch.
Draft changes do not touch outlines or active shaping; apply restores exact
snapshots. Test the compile revision for metrics, glyph, master and layout
changes to prevent stale previews.

## Verification and handoff

`cargo test -p fc-model -p fc-doc`; scoped Clippy; FCF schema review.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.

## Implementation evidence

`fc-model/src/feature_draft.rs` persists draft and minimal conversion backups;
`Font.feature_settings` remains separate from candidate settings.
`fc-doc/src/command/features.rs` provides guarded set/discard mutations.
`fc-doc/src/document/features.rs` provides the trusted, non-deserializable
`PreparedFeatureChange` and atomic commit after document/compile revision checks.
Feature and per-layer anchor snapshots preserve active source, draft, conversion
metadata and affected model data independently from outlines. Draft undo/redo
advances the persistent editor revision; compile revision excludes draft-only
changes. The FCF schema documents each defaulted field.

Verification on 2026-09-17:

- `cargo test -p fc-model -p fc-doc`: 185 passed, including 19 new feature-draft
  tests covering native/gzip/autosave persistence, old defaults, stale/no-op and
  coalesced transactions, aborted batches, all compile invalidation categories,
  trusted activation, conversion anchor/kerning undo/reopen, exhausted revisions,
  invalid partial lifts, and wire bypass rejection. Two additional kerning tests
  enforce the active writer-policy gate in both pure eligibility and commands.
- `cargo clippy -p fc-model -p fc-doc --all-targets -- -D warnings`: passed.
- `cargo fmt -p fc-model -p fc-doc --check` and `git diff --check`: passed.

Compiler/proof validation, API orchestration, source-export enforcement and UI
remain assigned to tasks 03–11; the trusted commit constructor documents this
validation obligation for direct Rust consumers.
