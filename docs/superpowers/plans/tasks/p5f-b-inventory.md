# p5f-b: Glyph inventory, search, filters and glyph data

Area id: `b-inventory`. Ports: API 8802, UI 1502. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                                                          | State at start                                                                                                        |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| 10  | Virtualised GPU grid, cell-size slider, grid/list toggle, sortable & rearrangeable list columns                                                      | Virtual Canvas2D grid and size controls exist; list mode (`GlyphTable*`) and column rearrangement need acceptance.    |
| 11  | Cell badges: metrics out of sync, annotation, custom glyph info, incompatible masters, non-exporting, locked                                         | `GlyphStatusBadges`, `glyphStatuses`; all six semantics need verification.                                            |
| 12  | 12 named colour labels + separate layer colour; free-text tags with autocomplete                                                                     | `colorLabels`, `LayerColorField`, `GlyphTagsField`; layer colour and tag autocomplete need acceptance.                |
| 13  | Search: name / Unicode / note, case-sensitivity toggle, regex                                                                                        | `GlyphSearch`; notes, case control and regex need acceptance.                                                         |
| 14  | Sidebar: Categories & subcategories, Scripts/Languages, Filters, Lists, with present/total badges and "generate missing glyphs"                      | `SidebarFilters`, `LanguageFilters`, `GlyphListsPanel`, `languageRepertoires`; counts and generate-missing to verify. |
| 15  | Smart filters with the full predicate set, saved in the project, editable as text DSL and visual builder                                             | `SmartFilterBuilder/Editor`, `smart_filter_parser/eval`; see `docs/p5-smart-filters*-acceptance.md`.                  |
| 16  | Glyph database (name ↔ Unicode ↔ category ↔ production name ↔ anchors ↔ composition) as a shippable data file with user overlay; "Update Glyph Info" | `fc-glyphdata`, `GlyphDatabasePanel`, `glyph_data` handlers; overlay and Update Glyph Info to verify.                 |
| 17  | Add Glyphs by name / character / codepoint / range / recipe; batch rename with find-and-replace                                                      | `AddGlyphsDialog`, `BatchRenameDialog`; see `docs/p5-batch-rename-acceptance.md`; reference updates to verify.        |
| 18  | CJK cells rendered from a fallback system face when empty; radical / stroke-count / Unicode-block sidebar sections                                   | Filters exist; fallback-face rendering of empty cells lacks acceptance.                                               |

Existing records: `docs/p5-smart-filters-acceptance.md`,
`docs/p5-smart-filters-ui-acceptance.md`, `docs/p5-batch-rename-acceptance.md`,
`docs/p5-font-metadata-acceptance.md`, `docs/superpowers/plans/tasks/p5-glyph-lists-api.md`,
`docs/p0-acceptance.md`, `docs/p4-acceptance.md` (CJK filters).

## Scope notes

- Row 10: "GPU" is satisfied by a virtualised, canvas-accelerated grid that
  meets the P0 performance budget on a 65,535-glyph font; record that reading.
  List mode: sortable by every column, columns reorderable by drag and
  keyboard, column set persisted with the workspace/preferences.
- Row 11: each badge has a real data source (metrics keys out of sync,
  annotations present, custom glyph info overlay present, master
  incompatibility, non-exporting flag, lock) and a tooltip; badges filterable.
- Row 12: layer colour separate from glyph colour label, shown in cell and
  layers panel; tags with autocomplete over existing tags; both persist and are
  usable as filter predicates.
- Row 13: search fields name/Unicode/note with case toggle and regex toggle;
  invalid regex reported inline, not thrown.
- Row 14: present/total badges per category, subcategory, script, language,
  filter and list; "generate missing glyphs" creates the missing members with
  the glyph database recipe (composition) where one exists, empty glyphs
  otherwise, in one undo unit.
- Row 15: DSL text and visual builder are two views of the same predicate
  tree; round trip both ways; the full predicate set of `smart_filter_parser`
  is reachable from the builder; filters persist in the `.fcf`.
- Row 16: the shipped data file plus a per-project user overlay (edits to
  name/Unicode/category/production name/anchors/composition); "Update Glyph
  Info" re-applies database data to selected glyphs (Unicode, category,
  production name) in one undo unit; export uses production names.
- Row 17: all five Add Glyphs modes; batch rename with find/replace and regex
  that updates components, kerning groups, feature classes/rules, metrics keys
  and glyph lists, with a preview and one undo unit.
- Row 18: empty CJK cells render the character from a fallback face (system
  font through Canvas2D) in a distinct muted style; toggleable in preferences.

## Owned files

`app/src/views/font/**` except `Optical*`, `opticalSpacing*`, `opticalMetrics*` (task j);
`app/src/dialogs/{AddGlyphsDialog.tsx,addGlyphs.ts,BatchRename*,batchRename*,NameList.tsx}` and tests;
`crates/fc-glyphdata/**`; `crates/fc-api/src/handlers/{glyph_data.rs,glyph_lists.rs,glyph_query.rs,glyph_status.rs,inventory.rs,smart_filter_eval.rs,smart_filter_eval_tests.rs,smart_filter_parser.rs,smart_filters.rs,recipe.rs}`;
`crates/fc-api/src/types/{glyph_data.rs,glyph_lists.rs,smart_filters.rs}`;
`crates/fc-doc/src/command/{glyph_data.rs,glyphs.rs,structure.rs}`; `crates/fc-model/src/query/**`;
`app/src/views/glyph/panels/{GlyphDatabasePanel*,GlyphTagsField.tsx,glyphTags.ts,LayerColorField.tsx}`;
`app/e2e/fontview.spec.ts`; `docs/p5f-b-inventory-acceptance.md`; user docs for the inventory.

Shared files (narrow patches only): `crates/fc-api/src/handlers/glyphs.rs`,
`crates/fc-api/src/types/glyphs.rs`, `crates/fc-model/src/glyph.rs`,
`crates/fc-model/src/extra.rs`, `crates/fc-doc/src/fcf/schema.md`,
`app/src/shell/builtinCommands.ts`, `app/src/dialogs/DialogHost.tsx`,
`app/src/i18n/en.json`, `crates/fc-api/src/handlers/mod.rs`,
`crates/fc-api/src/types/mod.rs`.
