# P5 export preflight panel

Implement a reusable `ExportPreflight({ doc, busy, onNavigate })` panel with an
explicit Check before export action against the existing `doc.find_problems`
API. Settle pending document edits before scanning. Show bounded diagnostics,
counts, compiler errors, stale-result notices, and keyboard-accessible glyph
links. Reuse `prepareTabTransition` and verify the originating document and
generation before navigation. Cancel responses after retargeting or unmount.

Own `app/src/dialogs/ExportPreflight.tsx`, its focused test file and private CSS
if needed, plus the narrow `exportPreflight` English namespace. Reuse existing
Find Problems styles and translations where possible. No generated API changes,
translations, export dialog integration, or broad checks; parent owns those.

Write failing behavior tests, implement, run focused Vitest and ESLint, record
acceptance evidence, and check artifact budgets before reporting completion.
