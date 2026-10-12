# P5-14 Sidebar categories and repertoires

## Scope

Complete the glyph inventory sidebar with Unicode subcategories, explicit
language alphabet repertoires, present/total coverage, persistent user glyph
lists, and previewed creation of missing list entries. Language alphabet counts
describe the listed scalar repertoire, not shaping or complete language support.

## Ownership and interfaces

- Parent owns `SidebarFilters.tsx`, `FilterRow.tsx`, new language repertoire UI
  and helpers, and optional initial recipe wiring in Add Glyphs.
- `sidebar_backend` owns `SidebarCounts` and sidebar handler enhancement:
  `sub_categories: [category, subcategory, count][]`, `unicodes: number[]`.
- `custom_lists` owns named list UI/helpers and coordinates shared inventory
  persistence with the saved smart filters team.
- Existing `GlyphFilter.unicode_ranges` selects alphabet members; existing
  `glyphs.add` previews/applies missing scalar or named recipes.

## Acceptance

- Existing category counts refresh without regressions; subcategory selection
  uses the existing category OR semantics.
- Alphabet coverage deduplicates all encoded scalars, including secondary
  assignments; partial coverage generates only missing Unicode recipes.
- Selecting another sidebar filter updates active language/list indication.
- Named glyph lists survive save/reopen through undoable font metadata.
- Missing generation opens Add Glyphs with preview, does not mutate immediately.
- Focused Vitest tests first; consolidated app check/lint/test and browser
  verification coordinated by the glyph inventory parent. Resource checks at
  start and completion.

## Implementation checkpoint

Implemented qualified category/subcategory selection; sorted valid scalar
coverage including secondary Unicode assignments; nine CLDR 48 Latin locale
letter repertoires; missing-only recipes passed to the existing Add Glyphs
preview; persistent named glyph lists with present/missing coverage; sidebar
integration for saved smart filters. Other language/script repertoires remain
outside this bounded checkpoint, and the UI makes no complete-language claim.

Focused frontend verification: 4 repertoire-helper tests, 3 language component
tests, 5 sidebar tests, 11 Add Glyphs tests, and 8 named-list tests passed.
Sidebar and named-list suites used one worker and a 20-second test deadline
after host contention caused a default-deadline timeout. Targeted ESLint,
Prettier and diff whitespace checks passed. API bindings, full app checks,
Rust integration tests (`sidebar_counts`, `glyph_lists`) and browser review
are assigned to the parent integration task; they are not claimed passed here.
