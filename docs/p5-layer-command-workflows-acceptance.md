# P5 layer command workflows acceptance

Implemented `RenameLayer` and `CopySelectionToBackground` with one-command undo
history, layer invalidation and atomic validation. Selected full contours and
components append to an existing background or create one with source metrics.
Copied contour/node identities are cleared to avoid collisions with destination
locks and links. Existing background metadata and source geometry are preserved.

Seven integration tests in `crates/fc-doc/tests/layer_workflows.rs` cover trimmed
names, invalid input without mutation, selection ordering/deduplication,
creation/append, component-only copy, self-copy rejection, independent identities
beside a locked background path, existing add/backup behavior, exact undo/redo,
and semantic plain/gzip `.fcf` persistence. Tests were written before implementation.
An independent read-only review found the copied-ID collision; the implementation
and regression test address it.

Verification completed:

```text
rustfmt --check --edition 2021 --config skip_children=true \
  crates/fc-doc/src/command/layers.rs crates/fc-doc/tests/layer_workflows.rs
exit 0
```

Runtime verification is **pending**, not passed. The serialized command
`TMPDIR=<workspace>/.scratch/p5-layer-command-workflows/tmp flock
.scratch/p5-resume/cargo.lock cargo test -p fc-doc --test layer_workflows` waited
for the shared Cargo lock, began compiling `fc-geom`, and was terminated with exit
143 on root's instruction to stop queued builds for a single integration owner.
No test result was produced. The root integration owner must run this target and
final workspace checks; this task claims no Clippy, full crate, or UI acceptance.

No model/schema/dependency/generated-file changes were needed. No commits were
made. Owned process group was checked empty; the empty task scratch directory
was removed. Resource checks passed throughout (start: 5.51 GiB build; final
check: 6.10 GiB build / 0.21 GiB temporary / 668.76 GiB free).
