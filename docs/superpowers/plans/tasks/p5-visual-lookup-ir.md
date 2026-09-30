# Visual lookup authoring IR

Scope: typed, bounded frontend FEA authoring helpers for GSUB single, multiple,
alternate, ligature and single-input contextual substitution; GPOS single, pair,
mark-to-base and mark-to-mark positioning.

Owned files: `app/src/views/features/visualLookup.ts`,
`visualLookupParser.ts`, and `visualLookup.test.ts`.

Public interface: `VisualLookup`, `VisualLookupKind`, `VisualValueRecord`,
`VisualAnchor`, `VISUAL_LOOKUP_KINDS`, `MAX_VISUAL_GLYPHS`,
`defaultVisualLookup(kind)`, `validateVisualLookup(rule)`,
`serializeVisualLookup(rule)`, `parseVisualLookup(code)`,
and `isValidFeatureTag(tag)`.

Serialization emits one supported rule (and its mark class declaration when
needed), inside an existing feature body. Rendering remains available for
invalid intermediate form values; validation is separate. Parsing returns null
unless the entire source is a supported valid form. No API or component edits.

Acceptance: tests first for all nine round trips, comments/whitespace, numeric
limits, glyph/name/list bounds, feature tags, unsupported syntax and trailing
statements. Run focused Vitest, TypeScript, ESLint, then the app test suite.
Backend compilation acceptance belongs to the separate lookup acceptance task.
