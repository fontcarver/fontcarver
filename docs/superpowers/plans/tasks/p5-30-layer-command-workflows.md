# P5 layer command workflows

Scope: provide undoable layer naming and copying selected paths to a background
for the outline-editor layer panel. Follow `p5-resume-coordination.md`.

## Files and interfaces

- `crates/fc-doc/src/command/layers.rs`: implement `RenameLayer` and
  `CopySelectionToBackground`; preserve existing add/copy semantics.
- Narrow enum and dispatch additions in `command.rs` and `command/apply.rs`.
- `crates/fc-doc/tests/layer_workflows.rs`: behavior and persistence coverage.

`RenameLayer { glyph, layer, name }` stores the trimmed nonempty display name.
`CopySelectionToBackground { glyph, from, master, contours, components }` copies
the explicitly selected full contours/component indices in source order, once
per index, into the master's background. It appends to an existing background,
preserving identity/metadata, or creates one carrying source metrics. Empty or
invalid selections and missing sources/masters fail without mutation. Source
content is retained; copying a background to itself is rejected. Copied contour
and point IDs are cleared so repeated copies remain independent of existing
layer-local identities and locks. Each command is one undo unit.

## Acceptance

1. Tests precede implementation and exercise the new command variants.
2. Rename trims and rejects blank names; errors leave history/font unchanged.
3. Selection creates/appends background, preserves source and unrelated content,
   handles duplicate indices deterministically, and rejects stale indices.
4. Existing add/full-copy behavior retains IDs and whole-layer metadata as
   applicable, with exact undo/redo.
5. Edited names/backgrounds survive semantic `.fcf` round-trip.
6. Run focused layer tests under the shared Cargo lock with task TMPDIR. Parent
   runs final broad checks after integration. Record disk checks and evidence.

No new model fields, schema changes, dependencies, generated types, commits,
translations, status/audit edits, or unrelated command refactors.
