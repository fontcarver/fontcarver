# P5-40 kerning modes and spacing protection

## Scope and ownership

Implement per-text-tab `off`, `on` (legacy/default), and `locked` kerning modes.
Off shapes with `kern`/`vkrn` disabled and blocks kerning mutations. Locked keeps
kerning available while disabling spacing keyboard edits, metric-handle drags,
and inspector metric mutations. A subordinate bounded task extends spacing
shortcut targets and increments (row 39).

Owned: mode helper/component/tests, TextBar integration, narrow TextTabState
field, useTextKerningControls gating, useTextTyping punctuation reservation,
useTextMetrics gating, and AsideGestures spacing guard. Proofing owner integrates
TextView; metrics-keys owner integrates MetricsPanel; workspace owner persists
new fields. Kerning class agent owns useTextKerning/kerningModel.

## Interfaces

`KerningMode = 'off' | 'on' | 'locked'`; optional tab `kerningMode` defaults to
`on`. `kerningFeatures(features, mode)` returns a shaping-only override while
retaining stored feature choices. `spacingLockedForDocument(doc)` and
`useSpacingLocked(doc)` guard editor entry points. TextBar optional mode props
render a labelled selector. useTextKerningControls receives optional mode.

## Acceptance

Unit tests cover feature overrides, mode persistence/default semantics, disabled
kerning controls and key handling, metric guard behavior, and selector changes.
Run focused Vitest suites and scoped lint/format checks. Parent owns broad
frontend acceptance; proofing owner provides integrated browser verification.
Run artifact checker before and after and remove owned scratch outputs.
