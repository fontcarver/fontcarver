# P5 path-operation acceptance

Date: 2026-09-30. Scope: task `p5-26-path-operations.md`, audit rows 26–27.

## Implemented

- Distinct `ForceExtrema` API/Path-menu operation (`Mod+Alt+Shift+E`) includes
  extrema inside the ordinary 0.01-unit endpoint exclusion. Existing Add Extremes
  retains its tolerant behavior. Reconstructed unchanged on-curve nodes preserve
  stable identity and G2 metadata, matching each original node at most once.
- Direction correction also canonicalizes starts/order. Tidy and direction now
  have all-master variants (`Mod+Alt+Shift+T/R`), each applied as one document
  batch. Background/custom layers are excluded.
- Break Contour preserves the closing curve and its handles by adding a fresh
  coincident endpoint. Splitting a live smart-corner node fails atomically with
  a repair instruction. Split duplicates do not reuse point/contour identities.
- Join Contours accepts either end of two contours and closes one contour's two
  selected endpoints. Coincident ends merge without dropping their closing
  segment. Set Start Point is disabled for open contours.

## Verification

Frontend regressions were written first: six baseline failures covered the
missing force/master operations and endpoint join cases. After implementation:

```text
pnpm --dir app test src/views/glyph/pathOps.test.ts \
  src/views/glyph/pathCommands.test.ts --maxWorkers=1 --minWorkers=1
Test Files  2 passed (2)
Tests       35 passed (35)

pnpm --dir app exec eslint src/views/glyph/pathOps.ts \
  src/views/glyph/pathCommands.ts src/views/glyph/pathOps.test.ts \
  src/views/glyph/pathCommands.test.ts
exit 0
```

Owned Rust files were formatted, and `git diff --check` passed. Added geometry
regressions cover near-endpoint splitting, bounds/idempotence and metadata.
Four document tests cover exact closing-curve retention, same-contour reconnect,
start-start joining, malformed input rollback, smart-corner rejection, grouped
master undo/redo, forced extrema undo and FCF write/read.

The initial focused Rust attempt was blocked by other tasks' incomplete
`ttx` module and `VariableStroke` match. A later attempt waited on the shared
Cargo lock and was **interrupted**, exit 143, at the integration parent's request
to serialize final verification centrally. It did not run or pass. Pending root
command:

```text
cargo test -p fc-geom --test extrema -p fc-doc --test path_operations
```

API generation, workspace checks and browser visual verification remain with
the integration parent; no generated API files were hand-edited.

## Boundaries

Do not mark row 27 complete: triangular-overlap Open Corner / Reconnect Nodes
and its invisible-overlap evaluation are distinct from Break/Join and remain
unimplemented. Row 26 also needs the separate segment-gesture owner's evidence.
No FontForge code was read/adapted; no dependencies were added. New feature scope
was frozen when the user requested finishing underway work and stopping.

The final task resource check passed (6.11 GiB build artifacts, 0.23 GiB temporary
artifacts, 668.64 GiB free). The only owned scratch directory was empty and was
removed. Other tasks' artifacts and processes were left untouched.
