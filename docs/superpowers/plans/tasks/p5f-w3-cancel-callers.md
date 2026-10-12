# Atomic cancellation caller regression migration

Owner: delegated follow-up agent. Read CONVENTIONS.md and p5f-protocol.md.
Own only cancellation-related test updates in
app/src/editor/editorController.test.ts (find exact existing path),
app/src/editor/gestures/drag.test.ts and
app/src/views/glyph/metricsHandles.test.ts, plus this task file and
docs/p5f-w3-cancel-callers-acceptance.md.

Coordinate w3_fix_all: shared EditSession.cancel now uses doc.edit_cancel(token)
after pending applies, without doc.edit_end or global doc.undo. Already ended
sessions do not cancel; stale tokens are rejected without touching newer edits.
Update obsolete test transport behavior/assertions to preserve meaningful
Escape/cancel geometry/history checks, not merely accept any call. Do not edit
shared mockTransport/EditSession/direct tests owned by w3_fix_all, or product
code. Report uncovered caller defect to parent for scope. Run focused tests and
lint; no full suite or Cargo. Own scratch with absolute TMPDIR, artifact checks,
no commits/push/reset/clean, preserve evidence and clean owned scratch afterward.

## Gap analysis

The three caller suites still assert end-plus-global-undo cancellation and use
custom transports without the new token cancellation handler. Retain ordinary
finish/end and nudge Escape behavior; migrate only actual cancelled drag paths.
Strengthen the assertions with the originating document/token and unchanged
geometry/next-edit baselines, including in-flight cancellation ordering and
failure reporting. The shared EditSession and token-model transport belong to
the Fix All owner. Run these three files against the new implementation before
editing to capture the obsolete expectations, then rerun with focused lint.

## Authorized scope extension: width-drag preview

Parent explicitly authorized `app/src/views/glyph/metricsHandles.ts` and its
existing owned test file after this migration exposed a stale RSB preview.
For a width-only drag, ink stays fixed, so RSB must change by the advance delta.
The current preview changes width only, moving the displayed right ink edge
despite unchanged contours. Add regression cases for larger/smaller advances,
negative finite advances and coarse rounding; retain the kernel's existing
`SetMetrics` semantics (finite negative widths are accepted, no new clamping).
Only the width-preview branch changes; other handles keep their behavior.

## Integration follow-up: remaining cancellation callers

Parent extended ownership to cancellation-related assertions/transports in
`app/src/editor/tools/transformTool.test.ts` and
`app/src/views/glyph/GlyphView.test.tsx`. The current integrated Vitest run
reports 2 failed / 2,834 passed: these two tests still require edit-end/global
undo. Migrate the expectations to token cancellation while retaining actual
scale geometry and canvas Escape/next-command ordering checks. Change no
production code and start no additional task; focused verification of these
two files precedes the parent's final gate rerun.
