# Virtual master coordinates — resumed implementation

Date: 2026-09-30. Scope: audit row 97 and named sparse-source authoring.

Implemented named `Font.virtual_masters` positions without allocating full-font
layers, metrics or kerning. Font Info stages these coordinates in the same
revision-checked transaction as axes, masters and instances. The glyph source
editor can copy a virtual position into a brace layer. Dedicated API commands
also support replacing positions and creating a source at one position.

Validation rejects absent axes, invalid/out-of-range/unknown coordinates,
duplicate names, duplicate physical/virtual positions and more than 1024
positions. Axis renames migrate positions. Existing source geometry is a copy:
editing/deleting a preset does not silently move or remove glyph sources.
Static instantiation removes virtual metadata with the variable axes.

Native FCF serialization and schema documentation include the positions.
Source-format writers currently do not preserve the named preset list; sparse
brace geometry uses the existing source/export pipeline. This is disclosed in
the schema. No FontForge sources or new dependencies were used.

Verification performed:

```text
pnpm exec vitest run src/views/fontinfo/VirtualMastersEditor.test.tsx \
  src/views/fontinfo/designSpaceSession.test.ts --maxWorkers=1 --minWorkers=1
Test Files  2 passed (2)
Tests       7 passed (7)

Focused ESLint on those components, DesignSpaceEditor and session: exit 0
Owned Rust files: rustfmt applied
Owned diffs: git diff --check passed
```

Two new document tests cover metadata-only addition, sparse source duplication,
invalid-operation rollback, undo/redo, native roundtrip and axis renames. A new
compiler regression constructs sparse virtual sources and checks actual glyph
coordinates in variable TTF and CFF2, plus static-instance metadata cleanup.
The first focused Cargo invocation failed in concurrently incomplete shared
`node_links`/glyph-database modules before these tests ran. Per root integration
instructions no retry was queued; final workspace tests and generated API
validation belong to the root integration run. Until that run passes, row 97
is implemented but not accepted as complete.

No browser verification is claimed. Other variable/CJK agents recorded attempts
blocked by shared entry-module/API regeneration; no extra browser was opened.
Task scratch contained only its empty TMPDIR and was removed. No owned servers
or test processes remain. Final artifact checker passed (build 6.09 GiB,
temporary 0.22 GiB, free 668.66 GiB; subsequent integration may change totals).

The first consolidated Rust run exposed two issues: the compiler fixture removed
its deliberately last-ordered default master, and design-space undo snapshots
omitted the new virtual-position field. The fixture now chooses the non-default
master by identity. A named `DesignSpaceSnapshot` captures/restores all four
metadata collections and includes virtual positions in its memory estimate.
The document tests now also verify preset-only undo/redo, axis-rename undo/redo,
and rollback when a later command in the same batch fails (three tests total).
These corrective patches are ready for the coordinating agent's focused retry;
no local Cargo retry was queued while the workspace run held the shared lock.
