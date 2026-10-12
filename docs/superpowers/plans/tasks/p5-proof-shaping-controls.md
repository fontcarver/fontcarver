# P5 proof shaping controls

## Scope and ownership

Implement `app/src/views/text/TextShapingControls.tsx`, its CSS Module and
component tests. Parent proofing agent owns TextView, TextBar integration,
presentation types, stored settings and English translation registration.
Do not edit those files, API types or translations. No axis/instance controls.

## Interface

Controlled props: `features: [string, boolean][]`, `script: string | null`,
`language: string | null`, `glyphs: readonly PositionedGlyph[]`, and
`onChange({ features?, script?, language? })`. Missing feature overrides mean
font/shaper defaults. Built-in feature selectors and all supplied override tags
offer Default/On/Off. Additional OpenType tags can be entered explicitly.
Script accepts four ASCII letters; language accepts nonempty ASCII subtags
separated by hyphens. Empty overrides restore automatic inference.

## Acceptance

Write failing component tests first for omitted/default feature semantics,
preserving independent overrides, custom feature validation, valid and invalid
script/language edits, Escape cancellation, and glyph cluster/advance output.
Use compact disclosure panels and token-based CSS with accessible labels.
Run focused Vitest, ESLint and Prettier checks; parent performs the final app
check/lint/test and browser verification after integrating the component.
Run artifact-budget checks at task start and completion; keep temporary outputs
under `.scratch/p5-proof-shaping-controls` and clean them at completion.
