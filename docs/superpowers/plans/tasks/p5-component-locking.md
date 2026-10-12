# P5 component locking

Scope: persist a default-false `Component.locked` flag, expose it through the existing
`SetComponent` payload, and prevent edit commands from transforming, deleting,
decomposing, replacing, or reordering a locked component. A locked component can
be unlocked only by a payload that otherwise preserves its current values.

Owned files: `fc-model/src/component.rs`, component schema paragraph, focused
`fc-doc/tests/component_locks.rs`, `fc-doc/src/command/component_locks.rs`, and
the required model-test and `fc-sfnt` component initializers. The annotation lock
owner wires the preflight guard into command dispatch. Parent owns component UI;
the Rust integration owner regenerates API types. The TypeScript lock property
is optional so older component payloads retain their default-unlocked meaning.

Acceptance: omitted native lock defaults to false; true persists; strict unlock
works with undo/redo; direct and bulk mutation rejection is atomic; unrelated
unlocked edits remain possible. Run focused Rust tests with the shared Cargo
lock, report limitations, and check artifact budgets at start and finish.

## Acceptance report — 2026-09-30

Implemented the default-false native component flag and optional generated TS
property. The command preflight protects direct component edits, transforms,
decomposition/removal, bulk outline/glyph replacement, layer deletion/background
overwrite/swap, destructive master deletion, and translating metric edits.
Strict unlock uses `SetComponent` with otherwise identical values. Four focused
tests cover native persistence/defaults, atomic rejection including a batch,
undo/redo unlock with independent unlocked edits, and destructive layer/glyph
operations. Required exhaustive component constructors were updated.

`rustfmt --edition 2021 crates/fc-doc/src/command/component_locks.rs
crates/fc-doc/tests/component_locks.rs crates/fc-model/src/component.rs` passed.
`cargo test -p fc-doc --test component_locks` was queued behind the shared Cargo
lock, then cancelled on the root's explicit integration instruction before
Cargo began; session exited 143 (`Terminated`). Tests are **pending**, not passed.
The root/Rust integration owner owns the solitary final generation/build/test
run and remaining shared-file formatting. No commit, generated-file edit, or
browser work was performed by this task.

Final artifact check passed: build 6.10 GiB / 13,178 files; temporary 0.15 GiB /
5,185 files; workspace free 668.72 GiB. The task's empty queued-test log and
scratch directory were removed; no task-owned process remains.
