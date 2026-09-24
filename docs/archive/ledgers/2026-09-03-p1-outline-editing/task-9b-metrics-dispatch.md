# T9b metrics runner — Sol

Implement only `app/src/views/text/textMetrics.ts` and its tests in a fresh
`p1-text-metrics` worktree. Read CONVENTIONS, EditSession and T9 integration
rulings. Coordinate the TypeScript interface with t9_ui; that agent owns all
controller/store/TextView integration.

Export `metricShortcut(event, isMac)` returning `{side:'lsb'|'rsb',delta}` or
null. Mod+Left reduces LSB, Mod+Right increases RSB; step 1, Shift 10,
macOS Meta+Ctrl 100. Linux Control-as-Mod alone stays 1. Alt is reserved for
P2 kerning and must return null.

Export TextMetricsRunner constructed with doc, glyph, layer, initial width/LSB/
RSB, undo label, optional after/scheduler/timeout, and preview/end/error callbacks.
Expose press(side,delta), end(), active. Coalesce a run through one 500 ms
EditSession, accumulate absolute SetMetrics from initial values despite async
responses, and preview width/LSB/RSB/lsbDelta. LSB moves geometry and width;
RSB changes width. Alternating sides must preserve accumulated changes.

Test delayed begins/repeats, alternating sides, one begin/end undo boundary,
timeout/explicit end/errors and platform modifiers. No dependencies, Rust,
i18n or UI changes. Run scoped tests, check/lint/prettier; commit and report
under 300 words in root task-9b-metrics-report.md. Root review precedes merge.
