# P5 shell command palette search routes

Owner: shell workspaces / palette routes; audit row 5.

## Scope

Extend the existing command palette with scoped search over commands, current
font filters, discovered local scripts and glyph names. Filter selection opens
the font overview after settling edits; glyph selection uses the existing glyph
tab navigation barrier. Script selection opens its source in the macro console,
with execution remaining an explicit console action. Reuse live registries and
stores from their owning agents; do not edit their implementation files.

Owned files: `app/src/shell/CommandPalette*`, new `paletteSearch*` helpers/tests,
`usePaletteGlyphSearch.ts`, `usePaletteEntries.ts`, `paletteSearch` English i18n keys, this task file and
palette acceptance report. Do not edit builtin command registration or generated
API types. Saved-filter and script APIs are coordinated with their owners.

## Acceptance

Write failing tests first for scoped search and async glyph navigation, verify
bounded metadata-only queries, stale request rejection, filter replacement and
edit-settlement cancellation. Existing command enablement, focus restoration and
keyboard operation must continue working. Run focused Vitest and ESLint checks;
parent runs shared broad app check/lint/tests and browser acceptance. Run artifact
budget checks before/after verification and before reporting completion.
