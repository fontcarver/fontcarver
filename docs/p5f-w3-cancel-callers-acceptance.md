# Atomic cancellation caller regression acceptance

2026-10-02; [task and authorized extension](superpowers/plans/tasks/p5f-w3-cancel-callers.md).

Migrated cancellation assertions/transports in `editorController.test.ts`,
`gestures/drag.test.ts` and `metricsHandles.test.ts` to token-scoped
`doc.edit_cancel`. Tests retain in-flight apply ordering, disposal/source
handover barriers, failure reporting, geometry previews and restored next-drag
baselines. They assert originating document/token and prohibit an intervening
global undo. Ordinary finish and nudge Escape still use `doc.edit_end`.

The parent authorized a discovered width-preview repair: width-only dragging
now changes RSB by the advance delta, preserving the stationary right ink edge.
Four regressions cover larger, smaller, negative finite and coarse-rounded
advances, unchanged contours/anchors, and exact `SetMetrics` commands. Kernel
`metrics::set` accepts finite negative horizontal advances, so no clamp was added.
Other handles are unchanged. Review added a controller regression asserting
exact width/RSB, stationary ink, Escape preview removal, token cancellation and
a second width drag starting from the restored baseline.

Verification used absolute task-local `TMPDIR`:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 src/editor/editorController.test.ts src/editor/gestures/drag.test.ts src/views/glyph/metricsHandles.test.ts
Baseline: 11 failed | 125 passed (obsolete cancellation expectations)
Width regressions before repair: 4 failed | 24 passed
Final: Test Files 3 passed (3); Tests 141 passed (141)
Duration 4.01s

pnpm --dir app exec eslint src/editor/editorController.test.ts src/editor/gestures/drag.test.ts src/views/glyph/metricsHandles.ts src/views/glyph/metricsHandles.test.ts
exit 0

pnpm --dir app exec prettier --check src/editor/editorController.test.ts src/editor/gestures/drag.test.ts src/views/glyph/metricsHandles.ts src/views/glyph/metricsHandles.test.ts
All matched files use Prettier code style!
```

`git diff --check` passed. No Cargo/global suite, dependency, translation or
FontForge changes. Shared cancellation implementation and kernel-history tests
belong to the Fix All task; integrated checking belongs to the parent.

Resource checks passed before/after tests (build 15.59→15.88 GiB). No persistent
process was started. Task scratch is removed after preserving this evidence.

## Integration follow-up

The parent's integrated run passed 2,834 tests and found two remaining obsolete
cancel expectations. Ownership was extended to `transformTool.test.ts` and
`GlyphView.test.tsx`; no production code changed in this follow-up.

Scale Escape now asserts real ×2 selected-point geometry before cancellation,
token 101, cleared preview/HUD and unchanged source. Canvas Escape holds
`doc.edit_cancel` for token 5, proves a subsequent field command waits for it,
and checks exact live metrics plus restored geometry. End/global-undo calls
remain forbidden by the complete call-order assertions.

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 src/editor/tools/transformTool.test.ts src/views/glyph/GlyphView.test.tsx
Test Files 2 passed (2); Tests 21 passed (21); Duration 15.68s

pnpm --dir app exec eslint src/editor/tools/transformTool.test.ts src/views/glyph/GlyphView.test.tsx
exit 0

pnpm --dir app exec prettier --check src/editor/tools/transformTool.test.ts src/views/glyph/GlyphView.test.tsx
All matched files use Prettier code style!
```

Task-local absolute `TMPDIR` was used; `git diff --check` passed. The parent
owns the final integrated rerun. No additional task or production change began.
