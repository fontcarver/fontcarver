# P5-82 Smart component controls

Owner: `variable_cjk/smart_components`; resumed 2026-09-30.

## Scope

Complete the selected component inspector's per-instance local-axis editing with
bounded range sliders, numeric fields, resetting to axis defaults, and a two-axis
parameter surface with a draggable smart handle. Each gesture commits one normal
`SetComponent` command and therefore uses the existing undo, persistence,
validation and outline refresh path. Handle keyboard operations are accessible.
The parameter surface is an inspector control; direct handles on the main glyph
canvas remain a distinct integration boundary and must not be claimed here.

## Files and interfaces

- New `SmartComponentControls.tsx`, `smartComponentControls.module.css`, pure
  coordinate helpers and focused tests under `app/src/views/glyph/panels/`.
- Narrow import/replacement of only the smart-axis map in `ComponentRow.tsx`,
  coordinated with `outline_editor/layers_anchors_audit` (locking owner).
- Add English `panel.smartComponent` strings only; independent translation owns
  Chinese strings.
- A separate `fc-api` integration test may cover range failure, undo/redo and
  native persistence through existing APIs. No model/API schema changes.

## Acceptance

Write failing interaction tests first for per-axis defaults and bounds,
per-gesture commits, cancellation, reset, independent stored axes, 2D handle
pointer/keyboard editing and locked controls. Run focused Vitest and API tests,
format/lint only owned frontend files, record actual results. Use isolated
agent-browser verification when a running app can be reused; otherwise report
that boundary explicitly. Parent owns broad integration checks and feature-audit
updates. Check artifact budgets before completion and remove owned temporary data.
