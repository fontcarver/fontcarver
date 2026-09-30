# P5 outline nodes and modifier acceptance

2026-09-30. Scope: audit rows 22–27; task
`superpowers/plans/tasks/p5-outline-node-semantics.md` and its G2/path subtasks.
New scope stopped when the user requested finishing underway work and resting.

## Implemented and checked

Physical Control is now distinct from the platform command modifier in browser
input. On macOS Command selects the 100-unit nudge, Control suspends snapping
and selects Control+Alt proportional/mirrored handle behavior. Shift retains
10-unit nudging and drag constraints. Existing drawing tools use physical
Control for snap suspension. Pressing/releasing a modifier also reevaluates the
stationary pointer, with tool key handling retained.

Changing Alt during a segment drag now preserves cumulative per-point command
positions: saved geometry matches the preview across multiple flushes and
modifier changes without pointer movement. Alt+Return now invokes the existing
outline-preserving smooth toggle, while a pen tool owning Return retains it.

Persistent cubic G2 nodes and one-shot Harmonize have geometry/document
commands, selection-gated menu entries, and visible ring markers. Preview
projection uses the same signed-curvature algorithm as the backend. G2 drags
send absolute projected outline snapshots in the existing edit session so
nonlinear handle projection cannot accumulate drift. Impossible samples leave
the most recent valid sample displayed and queued. The delegated nudge task
provides the corresponding arrow-run behavior.

Path operations add Force Extremes, all-master tidy/direction batches and
shape-preserving contour breaks/reconnections. See
`p5-path-operations-acceptance.md` for exact shape/metadata and open-corner bounds.

## Actual validation

Two regression failures were observed first: Alt segment preview/save mismatch
and conflation of Command with physical Control. After fixes:

```text
pnpm --dir app exec vitest run src/editor/gestures/drag.test.ts \
  src/editor/modifiers.test.ts src/editor/gestures/nudge.test.ts \
  src/editor/editorController.test.ts src/editor/tools/drawingTools.test.ts \
  --maxWorkers=1 --minWorkers=1
Test Files  5 passed (5)
Tests       135 passed (135)

pnpm --dir app exec vitest run src/views/glyph/smoothShortcut.test.ts \
  src/views/glyph/glyphCommands.test.ts --maxWorkers=1 --minWorkers=1
Test Files  2 passed (2)
Tests       12 passed (12)
```

Targeted ESLint passed for modified modifier/drag/tool files and the shortcut
registry/tests. `git diff --check` passed for the owned editor/glyph files.
G2 UI delegation passed 34 tests; path delegation passed 35 tests. Its reports
record exact suites, without treating overlapping suite totals as distinct tests.

Required Rust targets are pending centralized integration:
`cargo test -p fc-geom --test harmonize --test extrema` and
`cargo test -p fc-doc --test super_smooth --test path_operations`.
An early geometry build was blocked by other in-flight Rust work; later queued
Cargo waits were canceled on root instruction and are not reported as passed.
Root owns API generation, global TypeScript/lint/regression checks, and browser
acceptance. Generated types and translations were not hand-edited here.

## Audit boundaries

- Row 22: double-click/Return and Alt variants implemented; focused shortcut and
  controller evidence passes; backend persistence/undo tests await central run.
- Row 23: cubic-to-cubic G2 and Harmonize implemented; degenerate or opposite
  signed-curvature joins fail atomically. General quadratic G2 is not provided.
- Rows 24–25: modifier mapping, proportional drag and undo-session regression
  evidence passes; native macOS/Windows input was not exercised on Linux.
- Row 26: existing Shift-segment extremum/inflection tests pass in the controller
  suite; Force Extremes integration awaits central Rust/API checks.
- Row 27 remains partial: triangular-overlap Open Corner / Reconnect Nodes
  behavior is distinct from implemented Break/Join and remains unimplemented.

No FontForge source was read/adapted and no dependencies were added. Owned
processes and scratch are removed at handoff; shared build caches are retained.

Final nudge/projection delegation: `harmonize.test.ts`, `nudgeRun.test.ts`, and
`gestures/nudge.test.ts` passed 26 tests in three files with one worker; targeted
ESLint, Prettier and diff checks passed. The final parent task artifact check
passed at 6.17 GiB build / 0.21 GiB temporary / 668.71 GiB free.
