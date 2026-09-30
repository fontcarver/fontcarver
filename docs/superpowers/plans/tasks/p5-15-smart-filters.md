# P5-15 Smart filters and search

## Scope
Implement project-persisted named smart filters with a bounded boolean DSL and
matching visual rule builder, covering the vocabulary in competitor-ux §Smart
Filters (including nested groups, metrics, colors, Hangul and custom expressions).
Add name / Unicode / note query scopes, case sensitivity, and bounded regular
expressions to existing glyph list filtering. Use existing Font.lib persistence;
no structural model changes. Parent owns integration of FontView and registries.

## Owned files and interfaces
- `fc-api/src/handlers/glyphs.rs`: query/filter helpers only.
- `fc-api/src/types/glyphs.rs`: GlyphFilter additions only.
- New smart-filter handlers, parser, public types, and tests.
- New font-view smart-filter/search components and supporting helpers/tests.
- New narrowly scoped document command for project filter lib persistence if needed.
- English `smartFilters` keys; translations delegated separately.

## Acceptance
Focused tests demonstrate query scope/case/regex errors, nesting and all predicate
families, parser limits and malformed-input errors, builder round-trip, undo/redo
and .fcf persistence. Serialize Cargo through p5-resume/cargo.lock. Run scoped
formatting and focused suites; parent owns final integrated broad checks and UI
visual acceptance. Resource checks before/after batches and final cleanup.

## Dependency justification
`regex` (MIT OR Apache-2.0, already in the lockfile transitively) supplies a
linear-time, size-limited expression engine for glyph search and filter text.
Project collections use namespaced Font.info.user_data with SetFontInfo because
that existing command and snapshot path preserve undo/redo and .fcf persistence.
