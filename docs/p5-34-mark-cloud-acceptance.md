# P5 anchor mark-cloud acceptance

Implemented an opt-in preview in the Anchors panel. Each card draws the current
base and an available combining mark aligned by an exact `top`/`_top` (or other
named) attachment pair. Geometry is selected from the exact master and includes
flattened components. The preview refuses a base outline from another master.

The Mark-category inventory is paginated in groups of 24. Four shared detail
slots bound requests across stale generations and remounts; closing a preview
abandons queued work. Loading, empty, partial failure, failure/retry, page shrink,
document changes and local base-anchor movement are handled explicitly.

Verification on 2026-09-30:

```text
pnpm --dir app exec vitest run src/views/glyph/panels/MarkCloudPreview.test.tsx --maxWorkers=1 --minWorkers=1
Test Files  1 passed (1)
Tests       11 passed (11)
```

Earlier combined focused run passed 52 tests: inspector 28, geometry 18 and the
initial six preview tests. The final preview run above includes five additional
regressions for cancellation, local movement, selected-master mismatch, page
shrink and overlapping request generations. Targeted ESLint passed for all six
TypeScript files (exit 0); owned files were formatted with Prettier. Final
frontend integration and shared browser verification belong to the
parent task. No browser profile or server was started by this task.

Owned scratch cleanup removed 854 temporary files (7.3 MiB). Final artifact
checker passed: build 6.11 GiB / 13,177 files; temporary 0.19 GiB / 6,360 files;
workspace free 668.73 GiB. Git status was checked after cleanup. Two earlier
duplicate test launches were stopped before results and are recorded as
interrupted, not passed; the serial focused runs above completed successfully.

Limits: discovery follows the font's Mark category and conventional underscore
anchors; incorrectly categorized glyphs do not appear. The UI previews one mark
attachment at a time, rather than contextual shaping or multi-mark stacking.
Pending transport calls finish in their bounded slots because the current
transport has no cancellation API. No source adaptation, dependencies, schema
change or persistent font mutation was introduced. No commits were made.
