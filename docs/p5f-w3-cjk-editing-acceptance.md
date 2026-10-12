# CJK editor stale-state acceptance

Date: 2026-10-02. Rows 19 and 112; delegated task
[p5f-w3-cjk-editing](superpowers/plans/tasks/p5f-w3-cjk-editing.md).

## Changes and evidence

Vertical metric edits now serialize and read the stored glyph after preceding
edits settle. Rejected values cannot contaminate the next command. BSB arithmetic
uses the current origin and current kernel ink bounds; clearing an explicit
origin correctly restores master inheritance. No optimistic pair survives an
external Undo. Identical Enter/blur requests create no additional apply. Each
changed field still dispatches one command, and target changes cancel queued work.

Transform Apply rechecks mounted editor/target identity after settlement and
the document edit barrier, reads current kernel components, and validates the
complete component identity including metadata. A matching draft remains one
batch. Source changes, navigation and edits during the query cancel stale applies.
Rejected kernel commands retain retryable drafts.

Test-first evidence: vertical regressions initially produced **8 passed / 2
failed** (1230 instead of 1260 advance; rejected 70000 reused). The independent
audit's deferred component replacement regression produced **9 passed / 1
failed** before repair and is retained in `CjkTransformGrid.races.test.tsx`.

## Validation

From the repository root, using task-local absolute `TMPDIR`:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/glyph/panels/VerticalMetricsPanel.test.tsx \
  src/views/glyph/panels/CjkTransformGrid.test.tsx \
  src/views/glyph/panels/CjkTransformGrid.races.test.tsx \
  src/views/glyph/panels/cjkTransforms.test.ts \
  src/views/glyph/panels/MetricsPanel.test.tsx \
  src/views/glyph/panels/CjkConstructionPanel.test.tsx
Test Files  6 passed (6)
     Tests  55 passed (55)
  Duration  7.76s
```

Scoped ESLint over eight changed TypeScript files: exit 0, no warnings.
Scoped Prettier: `All matched files use Prettier code style!`
Final `pnpm --dir app check`: exit 0 (`tsc -b`). An earlier run reached
unrelated concurrent E2E edits:
`outline-audit.spec.ts` null `TtIndex` and DOM globals imported by the Node
TypeScript project; those were resolved by their owners before the final run.
Integration owns broad validation and real-kernel E2E evidence.

## Review round and limits

Independent review additions are pinned by tests: inherited reset followed by
BSB, Undo both before and after model acknowledgement, current kernel bounds,
metadata-only component changes, document barriers, and source-query races.
The independent editing reviewer reran all six files: **55/55 passed**, 7.26s,
and found no remaining blocker in this scoped implementation. Component tests
do not replace real-kernel browser or native-platform acceptance.
Neither `SetComponent` nor `SetVerticalMetrics` gains a new kernel atomic
compare-and-set API; validation guards known frontend edits and document events.
This is not a claim of atomic isolation from another external writer.

Recommended status: rows 19 and 112 retain **Complete** for the bounded editing
workflows after integration acceptance. No new strings, dependencies, model/API
types, FontForge adaptations, or native desktop automation.

Resource checkpoint before cleanup: build 15.88 GiB / 19,638 files; temporary
0.03 GiB / 1,507 files; free 606.00 GiB; PASS with the shared build warning.
Task scratch and owned processes are removed before handoff.
