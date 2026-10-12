# CJK editor stale state regressions

Task: p5f-w3-cjk-editing. Read AGENTS.md, CONVENTIONS.md and p5f-protocol.md.

## Scope and ownership

Own VerticalMetricsPanel.tsx and its tests, CjkTransformGrid.tsx and its tests
after coordination with auditor w3_audit_b (auditor is adding the deferred-settle
regression in that test file), and docs/p5f-w3-cjk-editing-acceptance.md.
Discover exact paths and append gap analysis before changes. No shared status,
audit, locale or API files without parent coordination.

## Required fixes and acceptance

Auditor w3_audit_a found BSB arithmetic uses render-time origin even after
a queued origin edit; e.g. origin 880 -> 910, bbox yMin -100 and BSB250 must
send advance1260/origin910, not1230. Rejected overrides also remain in
sent.current, so a later valid edit can inadvertently reuse an invalid advance.
Prove both with failing component tests, fix pending-value lifecycle, and retain
successful rapid-edit semantics plus inherited/null vertical metadata.

Auditor w3_audit_b identified an async stale CjkTransformGrid closure after
await activeEditor.settle(): an old indexed component can overwrite a replacement.
Coordinate auditor regression first, then fix using established editor identity/
revision patterns. Test no stale doc.apply, legitimate pending edits and correct
undo batch. Do not suppress all changes or introduce an unbounded retry.

Focused tests/check/lint only; parent owns broad validation and E2E owner may
add real-kernel regressions. Use own .scratch/p5f-w3-cjk-editing with absolute
TMPDIR, run artifact checker at start/batches/end. No commits/push/reset/clean,
no nested agents/native desktop automation. Preserve evidence, clean owned
scratch/processes and report actual results/remaining limitations under300words.

## Gap analysis

- Row 19: `app/src/views/glyph/panels/VerticalMetricsPanel.tsx` already edits
  inherited or explicit vertical metrics as one command per field. Its BSB
  callback uses the rendered origin instead of the latest successful origin,
  and its optimistic pair survives command rejection. Add regression tests
  for origin-to-BSB edits and rejected/queued metric edits, then serialize
  patches and read authoritative stored pairs before constructing each command.
- Row 112: `app/src/views/glyph/panels/CjkTransformGrid.tsx` invalidates displayed
  drafts after component-list changes but its asynchronous apply closure still
  holds old components while awaiting editor settlement. Coordinate the audit
  regression, recheck mounted editor/component identity before dispatch, and
  verify that unchanged pending edits still produce one batch command.
- Scope is component state and dispatch; parent integration owns broad tests
  and real-kernel acceptance. No locale/API/model changes are planned.

## Approved ownership extension

Parent approved narrow `gid` wiring in `MetricsPanel.tsx` and
`CjkConstructionPanel.tsx`, fresh detail/outline/master reads, and full component
identity in `cjkTransforms.ts`. Async regression tests live in
`CjkTransformGrid.races.test.tsx` to keep each test file below 400 lines.

## Review findings addressed

Independent review found reset-to-inherited-origin, acknowledged-then-undone,
and Undo-before-acknowledgment cases. Removing the optimistic metric pair
entirely and reading `glyphs.detail` after the serialized preceding command
handles all three; sidebearings read current bounds and inherited master origin.
Full component signatures include anchor/alignment/smart/point-match metadata;
fresh outline validation covers kernel edits not yet rendered by React.
