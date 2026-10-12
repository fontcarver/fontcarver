# Pointer and canvas usability review

2026-10-02. Scope: [p5ux-pointer](superpowers/plans/tasks/p5ux-pointer.md).
Initial read-only product review against `docs/design/editor-ui.md`, followed
by the coordinator-authorized outline repair below. The coordinator owns the
actual headless browser walkthrough. Findings in the first table describe the
pre-repair behavior; line numbers refer to that inspected version.

## Initial repair priorities

| Priority | Finding and reproduction                                                                                                                                                                                                                                                                                       | Evidence and likely repair ownership                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| High     | **Right dragging an outline node edits it.** In Select mode, press the secondary mouse button on a node, move, and release. Selection changes and a `doc.edit_apply` is dispatched. A secondary click must not start a primary-button editing gesture, regardless of whether a custom context menu is offered. | Reproduced with the real `EditorController` and editing transport fixture. `app/src/editor/editorController.ts:372` handles middle-button pan but otherwise never rejects secondary/auxiliary buttons before selection and dragging. `app/src/views/glyph/GlyphCanvas.tsx:653` also focuses and captures every button. A narrow outline-pointer repair should gate accepted buttons before settling/capturing/starting a gesture, while preserving middle-button pan. |
| High     | **A bitmap stroke cannot be abandoned with Escape.** Start a pencil or shape drag, press Escape while still holding the pointer, then move and release: editing continues and commits.                                                                                                                         | Reproduced with `BitmapCanvas` callbacks. `BitmapCanvas.tsx:181` handles only Hand-tool arrows; `BitmapEditor.tsx:156` handles clipboard/nudging but no Escape. Add explicit gesture cancellation that restores the pre-gesture pixels/selection, clears the gesture and does not flush a new undo entry. Avoid global undo, which could target someone else's command.                                                                                               |
| Medium   | **Interrupted pointers are treated as successful releases.** Bitmap `pointercancel` calls `onCommit`; outline `pointercancel` reaches `controller.pointerUp` and the smart/image handle `up` paths. Unexpected capture loss is unhandled by the outline canvas.                                                | Bitmap cancellation commit reproduced. Source: `BitmapCanvas.tsx:326` binds up/cancel/lost-capture to the same `finish`; `GlyphCanvas.tsx:919` binds cancel to normal `endPointer` and has no lost-capture handler. Separate successful release from abandonment, using the existing token-scoped cancellation path for outline editing. A normal release's subsequent lost-capture event must remain harmless.                                                       |
| Medium   | **Space-pan state can survive lost focus.** Focus the outline canvas, hold Space, move focus to a numeric/text field or another window, release Space there, then return and drag. The canvas has no blur/window-blur reset for its own held state.                                                            | Source-confirmed, pending coordinator browser reproduction. `GlyphCanvas.tsx:843` sets `spaceRef` and controller state; only its own `onKeyUp` at line 894 clears them. `useToolShortcuts` has a window blur handler, but it resets borrowed tools, not the canvas's Space state. Clear temporary pan/preview state on focus loss without toggling fill.                                                                                                              |
| Medium   | **Bitmap navigation breaks the outline gesture grammar.** Middle drag is ignored; Space drag cannot temporarily borrow pan; Ctrl-wheel/pinch has no bitmap zoom handling and can reach browser zoom. Users must switch explicitly to Hand and use the zoom dropdown.                                           | Middle-drag non-panning reproduced; other missing handlers confirmed in `BitmapCanvas.tsx` and its caller. This is a useful parity improvement, not evidence of corrupt bitmap data. The bitmap owner can add middle/Space pan and anchored wheel zoom with a caller `onZoom` contract, retaining ordinary scroll and current Hand-tool keyboard panning.                                                                                                             |
| Medium   | **Font-grid marquee has no cancellation lifecycle.** Begin a band on blank grid space and lose pointer capture/cancel: neither event clears its `band`. A later hover can keep updating that old band.                                                                                                         | Source-confirmed, pending browser reproduction. `useGridPointer.ts` owns `band`, while `GlyphGrid.tsx:127` binds down/move/up only. Add cancel/lost-capture cleanup and ensure Escape/focus loss clears a live band without applying a new selection. This can be a separate disjoint repair.                                                                                                                                                                         |

One smaller Space case belongs with the same outline lifecycle fix: pressing
Space during an already active pen press supports repositioning, but `dragged`
is only set during pointer-down. Moving after that late Space press can still
be interpreted as a tap on keyup and toggle fill. Track actual Space use during
an existing gesture, and preserve intentional tap-to-toggle behavior.

## Existing interaction support worth preserving

The outline editor already has primary click/modifier selection, empty-space
marquee, node/handle/segment dragging, double-click smooth/corner toggle,
Space/middle/Hand panning, Zoom click/Alt-click/marquee and anchored wheel zoom.
The wheel listener is non-passive and normalizes pixel/line/page deltas; plain
two-axis wheel pans, Ctrl/Meta-wheel zooms at the cursor. Trackpad pinch carried
as Ctrl-wheel therefore has an implemented route. Physical trackpad, touch and
native platform input were not exercised in this review.

The existing `editorController.test.ts` suite exercises middle-button and
Space pan, selection modifiers, smooth toggles, gesture cancellation and next
gesture restoration. `Viewport.test.ts` checks coordinate/zoom behavior.
`BitmapCanvas.test.tsx` checks pencil toggle consistency, selection move/copy,
eyedropper, Hand pan and pointer readouts. These tests do not cover the four
reproductions above. The font grid already has a real context menu that
preserves multi-selection when right-clicking inside it; this should be kept.

Text and single-glyph editing share `GlyphCanvas`. Text mode intentionally
gives primary clicks to glyph activation, kerning gaps and the measurement
beam before outline editing. A lifecycle fix must also release those auxiliary
states rather than fixing only plain node dragging. Two-finger touchscreen
pinch is not an implemented desktop requirement and is not proposed as a
blocking addition. A new outline context menu is optional; preventing right
button edits is required independently.

The design document's 1%–6400% zoom range differs from `Viewport`'s current
0.05%–4000% bounds. This is a documentation/product-range decision, not a reason
to change established zoom limits during the pointer safety repair.

## Verification

Temporary probes reused the existing controller/bitmap fixtures with absolute
source imports under `.scratch/p5ux-pointer`. They deliberately asserted the
current undesired outcomes, establishing reproducibility without editing
production files or committed tests:

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5ux-pointer/tmp \
  pnpm --dir app exec vitest run \
  --config ../.scratch/p5ux-pointer/vitest.config.mts
Test Files  2 passed (2)
     Tests  4 passed (4)
Duration   2.90s
```

The four confirmed outcomes were secondary-button outline mutation, bitmap
cancel committing, bitmap Escape leaving the stroke live, and bitmap middle
drag leaving the scroll position unchanged. **Passing reproduction probes do
not mean those interactions are acceptable.** Focus/capture-loss findings above
are source-traced and clearly distinguished from executed probes.

No broad build, native/global desktop automation, dependency change, commit or
publication was performed. Bitmap repairs belong to the separate bitmap task;
the font-grid lifecycle and zoom-range documentation observations were passed
to the coordinator rather than silently included in the outline scope.

## Authorized outline repair

`GlyphCanvas` now owns one primary pointer, accepting left editing and middle
panning before focus/capture. Right and auxiliary buttons cannot select or
begin an edit. `canvasPointerSession.ts` separates successful release from
Escape, pointer cancellation, unexpected capture loss, canvas/window blur and
hidden-document cancellation. Ownership is cleared before capture release, so
the browser's subsequent lost-capture event cannot cancel a completed edit.
Releasing an initiating left/middle button while another button stays held
cancels the gesture; the remaining right button cannot continue editing.

Cancellation clears smart-component, image-placement and measurement-beam
states, then uses the controller's token-scoped cancellation for node, metric
and transform edits. Pending Pen/shape drafts are discarded when an active
press is interrupted. Between Pen clicks, focus changes preserve the draft
and the existing Escape/tool-switch finish behavior. Cancelling temporary
Space/middle pan also preserves an existing Pen draft. Space blur clears the
temporary pan state without toggling fill; Space used inside an existing Pen
press is no longer mistaken for a preview-toggle tap.

Regressions were written before implementation: three controller assertions
failed and the new lifecycle suite could not load its not-yet-created helper.
A subsequent mouse-chord regression independently reproduced an open
`doc.edit_begin`/`doc.edit_apply` session without `doc.edit_end`, then passed
after the release fix. `TextView`'s two-Escape integration case initially
failed because its initial DOM pointer-down had no matching pointer-up; its
owner added the missing release and retained both original Escape assertions.

Validation and independent review are recorded at handoff below. No UI text,
dependency or kernel API was added by this repair.

## Repair validation

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5ux-pointer/tmp \
  pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/editor/editorController.test.ts \
  src/views/glyph/canvasPointerSession.test.ts \
  src/editor/tools/transformTool.test.ts src/editor/tools/pen.test.ts \
  src/views/text/TextView.test.tsx
Test Files  5 passed (5)
     Tests  135 passed (135)
Duration   7.82s

pnpm --dir app check
> tsc -b
exit 0

pnpm --dir app exec eslint \
  src/views/glyph/canvasPointerSession.ts \
  src/views/glyph/canvasPointerSession.test.ts \
  src/views/glyph/GlyphCanvas.tsx src/editor/editorController.ts \
  src/editor/editorController.test.ts src/editor/toolHost.ts \
  src/editor/tools/toolTypes.ts src/editor/tools/transformTool.ts
exit 0
```

Independent reviewer repeated the 128 outline tests and six additional probes
covering cancellation while node/metric/transform applies are pending, mouse
chords, inert hover after cancellation and hidden-document interruption:
[independent outline review](p5ux-outline-review.md). Browser owner separately
reported real-kernel right-button safety and Escape/pointercancel/blur
cancellation with unchanged outline/history, followed by a successful drag and
undo. Final browser evidence belongs to the coordinator's UX acceptance report.
Physical trackpad and native macOS/Windows input remain untested locally.

### Full-suite fixture follow-up

The coordinator's full Vitest run found three `GlyphView.test.tsx` failures.
Its Space-pan and metrics-drag events omitted `buttons`, whose default `0`
correctly means the initiating button is no longer held. Both fixtures now
send `buttons: 1` while dragging and `0` on successful release; the metric
gesture also identifies the same pointer explicitly. Original assertions are
unchanged. The layer-switch case passed alone: its failure was a cascade from
the metric fixture's unreleased cancellation promise after an early assertion
failure. `onTestFinished(release)` now prevents that test-only barrier leak.
No product source changed during this follow-up, preserving the browser freeze.

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5ux-pointer/tmp \
  pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/glyph/GlyphView.test.tsx \
  src/views/glyph/canvasPointerSession.test.ts \
  src/editor/editorController.test.ts
Test Files  3 passed (3)
     Tests  113 passed (113)
Duration   4.90s

pnpm --dir app exec eslint src/views/glyph/GlyphView.test.tsx
exit 0
```

Prettier and the scoped whitespace check passed. The recreated empty scratch
was removed; resource checker remained within budget.

## Resources

Start checker PASS: build 5.33 GiB / 12,257 files; temporary 0.19 GiB / 2,316
files; free 616.30 GiB. Owned 28 KiB scratch, probe files and its dependency
symlink were removed after validation; no process remains owned by this task.
Final checker PASS: build 5.33 GiB / 12,257 files; shared temporary 0.13 GiB /
5,022 files; free 616.32 GiB. The remaining shared scratch belongs to concurrent
tasks. Prettier and the scoped Git whitespace check also passed.
