# P5 metrics key UI and synchronization

Scope: `MetricsPanel.tsx`, a dedicated metrics command helper and tests,
`useResolvedMetrics.ts` and tests, narrow existing inspector test adjustments,
and the English `panel.metrics` messages. Backend/model/API work belongs to the
metrics-keys parent; generated API files and Chinese translations are excluded.

Behavior: display effective per-field glyph/layer formulas; `==` edits the
current layer override and `=` edits the glyph key, removing only that field's
override while preserving unrelated fields. Resolve the explicitly selected
layer. Show pending, synchronized, stale, and failed resolution. Update Metrics
commits the current master or all master layers through one undoable command.

Acceptance: component tests cover both scopes, override removal, preservation,
current/all-master commands, and refusal feedback. Hook tests prove selecting a
different layer invalidates a previous result. Run only focused Vitest/ESLint/
Prettier checks; root owns generated API refresh, frontend-wide checks and
integrated browser verification. Run artifact checker at task boundaries.
