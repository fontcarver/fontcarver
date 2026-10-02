# Independent review of wave 3 editing fixes

Date: 2026-10-02. Task: [review scope](superpowers/plans/tasks/p5f-w3-review-editing.md).

Final scoped verdict: **PASS** after the CJK and width-cancellation review
rounds below. This approves the inspected source and focused regression
evidence; the coordinator owns the full integrated checks and native platform
acceptance.

## Spacing shortcuts

Source review finds no blocking defect in the added Alt-bracket ×100 and
Alt+Shift-bracket exact-one steps. The mapper accepts physical bracket codes
(including macOS Option-produced punctuation), excludes both Ctrl and Meta
brackets, and preserves the existing side selection and edit-session path.
Fixed gestures participate in customized keymap conflicts. Locked spacing
consumes the gestures without issuing a command. The mapper and dispatcher
tests distinguish macOS and non-macOS modifier mapping; this is not evidence
of native keyboard delivery.

Independent focused verification, with absolute TMPDIR under
`.scratch/p5f-w3-review-editing/tmp`:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/text/textMetricShortcuts.test.ts \
  src/views/text/useTextMetrics.test.tsx \
  src/views/text/SpacingControls.test.tsx \
  src/shell/keymapIntegration.test.ts
Test Files  4 passed (4)
     Tests  40 passed (40)
Duration   4.27s
```

The owner subsequently added two explicit navigation dispatch tests. The
reviewer reran the updated keymap file with the bitmap tests below: all 13
keymap tests passed. The E2E owner reports `spacing-shortcuts.spec.ts`
passing three repeated real-kernel runs (21.3 s), checking the actual metric
amounts, one undo entry per settled gesture, redo, locked spacing, and
navigation preserving every glyph's metrics. The specimen was reviewed.
Independent Chinese translation is recorded below.

## Bitmap saving and vertical waterfall

Source and focused tests find no blocking defect. Prepared PNG bytes use the
existing `saveArtifact` native-dialog/filesystem or browser path; native
cancellation and write failure retain the prepared preview for retry. Tests
mock the native dialog and filesystem, so this does not assert an executed
native save dialog.

The vertical layout uses both origin coordinates and actual advances, retains
zero advances, accounts for negative bearings/advances, and bounds the canvas
with a visible clipping warning. Backend preview inheritance respects explicit
strike metrics first, then the default-master glyph layer, then master defaults
or em/ascender fallbacks. It operates on flattened preview clones and does not
write inherited values into stored bitmaps. User documentation accurately
states that vertical substitutions and punctuation rotation are not performed.

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/bitmap/BitmapSheet.test.tsx \
  src/views/bitmap/BitmapWaterfall.test.tsx \
  src/views/bitmap/bitmapWaterfallLayout.test.ts \
  src/shell/keymapIntegration.test.ts
Test Files  4 passed (4)
     Tests  28 passed (28)
Duration   5.39s
```

The owner reports bitmap API tests passing 4 + 20 cases. The reviewer inspected
the new inheritance/non-mutation regression without duplicating the Cargo run.
The E2E owner subsequently reports the actual PNG save/import round trip passing
(5.2 s) and the explicit-origin vertical proof passing (2.1 s): a 10×44 canvas
contains two 3×4 ink rectangles at (2,15) and (0,29), exactly 24 pixels. The
reviewer inspected the updated browser assertions. Inherited metrics are covered
by the API regression; the final integrated E2E run must use the rebuilt kernel.
Independent Chinese translation is recorded below.

## Width preview and cancellation callers

The parent extended this review to the width-only preview fix and affected
gesture caller tests. Source review approves the RSB delta: changing advance
without moving ink changes RSB by exactly the advance delta, including finite
negative widths accepted by the kernel.

The reviewer requested one controller-level assertion of exact live width/RSB
followed by Escape and a restored next-drag baseline. The owner added it: live
680/0/480 metrics leave the right ink edge at 200; cancellation removes the
preview and addresses token 101; the next drag starts from width 600 and
produces 620/0/420. Geometry stays unchanged. Independent verification:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/glyph/metricsHandles.test.ts \
  src/editor/editorController.test.ts src/editor/gestures/drag.test.ts
Test Files  3 passed (3)
     Tests  141 passed (141)
Duration   12.00s
```

No blocking defect remains in this narrow extension. The independent kernel
review covers atomic cancellation and shared `EditSession` lifecycle semantics.

## CJK review round

The original findings were stale-origin BSB arithmetic, rejected vertical
overrides contaminating subsequent edits, and a deferred component transform
overwriting a replacement. The reviewer additionally required metadata-only
component replacement, reset to inherited origin followed by BSB before reload,
and Undo both before and after acknowledgement of the preceding field edit.

These cases are resolved. Vertical field edits serialize after previous field
outcomes and editor/document barriers, then read authoritative stored glyph
metrics. The old sent-value cache is removed. TSB/BSB read current resolved ink
bounds; inherited origin comes from the actual master rather than a stale
rendered explicit value. The dispatch rechecks target/editor identity,
document revision/generation and spacing lock. Identical values do not create
a redundant command. Existing backend lock/read-only validation is unchanged.

The transform grid checks a signature of the full component records, including
metadata, after settlement and a fresh kernel outline read. Mounted target,
editor and edit revision/generation guards reject stale commands. An unchanged
pending edit still creates exactly one batch, and kernel rejection retains
drafts for retry. The narrow `gid` wiring in both calling panels was reviewed.

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
Duration   7.26s
```

No blocking defect remains in the inspected CJK fixes. These are component and
dispatch regressions using mocked transport, not a new native CJK run.

## Localization and limits

The independent [GPT-6.1 Sol delta review](p5f-w3-translation-acceptance.md)
records all twelve wave-3 English changes translated, with 3,943 matching
catalogue leaves and matching placeholders. This reviewer checked the nine
spacing and bitmap values in context and found no discrepancy. CJK state and
width-preview repairs add no visible strings.

No FontForge adaptation or dependency was introduced by these edits. This
review ran no native macOS/Windows keyboard or save-dialog session, no broad
integration suite, and no publishing action. The coordinator must retain those
distinctions when reporting project completion.

## Resources

Review start: build 15.59 GiB / 19,162 files; temporary 0.32 GiB / 2,060
files; workspace free 606.07 GiB; checker PASS with the existing build-cache
warning above 10 GiB. Final resource and cleanup evidence is recorded below.

Final checker PASS: build 15.88 GiB / 19,640 files; temporary 0.01 GiB / 553
files; workspace free 606.06 GiB, with the same build-cache warning. No owned
server/browser process was started; all focused test processes exited. The
owned empty scratch directory was removed. Only the review report and its task
scope were edited. No implementation file, commit or external state was changed.
