# P5-39 bounded spacing shortcuts

## Scope and ownership

Own `app/src/views/text/textMetrics.ts`, `textMetrics.test.ts`, and the extracted
shortcut mapper tests in `textMetricShortcuts.test.ts` (to keep test files under
400 lines). The parent
owns hook and Text view integration; do not edit `useTextMetrics.ts`, TextBar,
TextView, shared progress files, or translations.

Approved extension: own `SpacingControls.tsx`, its CSS Module and focused test,
and only the `spacingControls` English translation namespace. Controlled props
are `side`, `onSideChange`, and optional `disabled`; show accessible selected-side
control and bracket modifier help. The parent feeds the configured preference
step to the shortcut helper and disables the controls outside edit mode or when
kerning is locked. No Chinese translations are owned here.

## Public contract

- Extend `MetricSide` with `both`; one runner press adjusts both sidebearings
  by the signed delta in one absolute metrics command and undo session.
- Add optional `MetricShortcutOptions` to `metricShortcut`: selected `side`
  and optional positive `step`. With options, `[` and `]` decrease/increase
  the selected side by the configured step. Shift multiplies by 10, platform
  Mod multiplies by 100, Shift+Mod forces exactly one font unit.
- Keep the existing Mod+arrow mappings unchanged. Alt combinations remain
  available for kerning. Brackets are claimed only when options are supplied;
  the parent limits them to the active metrics-editing context.
- Missing or invalid step falls back to one. Shifted bracket key values and
  physical key codes are recognized so the full modifier grammar works.

## Acceptance

Write failing focused tests first for selected sides, both signs, fractional
step multipliers, true-one override, platform modifiers, shifted brackets,
legacy compatibility and invalid step. Verify one atomic preview/command for
both sides, empty-glyph advance handling, coalescing and undo-session closure.
Run only focused Vitest, ESLint and Prettier checks for these owned files; the
parent performs frontend-wide checks after integration. Check artifact budget
at start and finish and remove owned scratch files.
