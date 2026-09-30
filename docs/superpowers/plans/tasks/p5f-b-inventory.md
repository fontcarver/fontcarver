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

## Gap analysis

Read at `449e753`; code inspected before any change. Initial resource check:
build 6.00 GiB / 12,708 files, temporary 0.00 GiB, 666.53 GiB free — PASS.

### Row 10 — grid, cell size, grid/list, sortable and rearrangeable columns

Works: virtualised Canvas2D grid with a bounded page cache (`GlyphPager`,
`useVirtualGrid`), cell-size slider (`GridToolbar`), grid/list toggle, list mode
with a virtualised row window (`useVirtualGlyphTable`), column drag reorder and
keyboard left/right reorder with `aria-label`s (`GlyphTableHeader`).

Missing: only `name`, `unicode` and `width` headers sort — `category`, `script`
and `status` have no sort order in the kernel; there is no way to choose _which_
columns appear; nothing about the list presentation survives a reload.

Doing: add `Category`, `Script` and `Status` orders to `GlyphSort` and
`sort_candidates`; add a column-visibility menu to the table header; persist
view mode, cell size, label mode and the column set/order in
`app/src/views/font/fontViewPrefs.ts` (the `localStorage` pattern already used by
`uiStore`, `workspaceStore` and `keymapStore`), because `app/src/stores/documentStore.ts`
is not in this task's file list. Record the 65,535-glyph frame reading in the
acceptance report.

Limit: ordering is ascending only, which matches the reference application; a
descending toggle would need `GlyphListParams` and the document store, and is
recorded as a bound rather than a placeholder control.

### Row 11 — cell badges

Works: all six semantics have a kernel data source (`glyph_status.rs`:
`metrics_out_of_sync`, `has_annotations`, `has_custom_glyph_info`,
`incompatible_masters`, plus `export` and `locked` on the glyph), are carried on
`GlyphCell`, are drawn with tooltips and overflow (`GlyphStatusBadges`), and are
in the cell's `aria-label` and in the list's Status column.

Missing: `has_custom_glyph_info` is true whenever _any_ explicit classification
field is set, so a normal "Update Glyph Info" — which writes those fields from
the database — lights the badge on every glyph it touched. `locked` is the only
badge with no filter predicate. No one-click way to filter by a badge.

Doing: redefine `has_custom_glyph_info` as "explicit metadata differs from what
the shipped database resolves, or format-specific `user_data` is present", which
is the same rule `glyphdata.state` already uses for its `custom` flag; add a
`locked` smart-filter field; add a Status panel to the sidebar with one toggle
per badge writing the matching smart expression.

### Row 12 — colour labels, layer colour, tags

Works: twelve fixed labels (`colorLabels.ts`) with swatches in the cell stripe
and the sidebar, an independent layer label at
`Layer.user_data["com.fontcarver.colorLabel"]` with its own command
(`SetLayerColor`) and field, free-text tags with combobox autocomplete over the
document's tags, and `color_label`, `layer_color_label` and `tags` filter
predicates.

Missing: the layer colour is not visible anywhere in the grid; the glyph colour
label can only be set one glyph at a time from the Edit view, so the ordinary
"colour-code a selection" gesture does not exist in the font view; the layer
colour field shows no swatch.

Doing: carry `layer_color_label` on `GlyphCell` and paint it as a second marker
in the cell; add colour-label commands for the grid selection (twelve labels
plus None) in one undo unit, reachable from the context menu, the Glyph menu and
the palette; add the swatch to `LayerColorField`.

### Row 13 — search

Works: All/Name/Unicode/Note scopes, case toggle, regex toggle, bounded regex
compiled in the kernel (`glyph_query.rs`), CJK reading/radical query, ranked
matches, 4 KiB query cap, and unit tests for every scope and both toggles.

Missing: an invalid regular expression is reported only as a global toast from
`reportError`, so the field itself never says what is wrong and the grid stays
empty with no explanation.

Doing: route the failing page request's error to the search field through
`app/src/views/font/queryErrors.ts` and show it inline with `role="alert"`,
leaving the toast for genuine transport failures.

### Row 14 — sidebar sections, present/total, generate missing

Works: Overview/All, Unencoded, Categories with subcategories, Scripts, Unicode
blocks, Colours, Languages (with present/total and a generate button), Lists
(with present/total and generate-missing) and Filters.

Missing: present/total on categories, subcategories, scripts, blocks and saved
filters — those rows show a bare present count; generate-missing exists only for
languages and lists; every generate path creates _empty_ glyphs even when the
glyph database defines a composition.

Decision recorded here: "total" is the size of a defined repertoire —
assigned Unicode scalars of that category/subcategory/script/block, the bounded
language repertoire, the list length — and for a saved smart filter it is the
number of glyphs in the font, since a predicate has no repertoire. New
`inventory.coverage` and `inventory.missing` methods compute this in
`crates/fc-api/src/handlers/inventory.rs` (this task's file) rather than in
`doc.sidebar`, which belongs to task a. Generate is offered when at most 4,096
members are missing; a larger row states its count and sends the user to Add
Glyphs, which accepts the same repertoire recipes.

Doing: the two new methods, present/total on every sectioned row, a generate
button on each row with a bounded missing set, match counts on saved filters,
and composition-aware creation (below).

### Row 15 — smart filters

Works: one parser and evaluator for both views, 33 predicate fields, `and/or/not`
with parentheses, eight operators, bounded regex/literal/depth/node limits,
project persistence under `font.info.user_data["com.fontcarver.inventory"].smart_filters`
with ordinary undo, canonical-text round trip through `glyph.smart_filters.parse`,
and a builder whose field list matches `field_kind` exactly.

Missing: nothing structural. The builder vocabulary and the kernel vocabulary are
two hand-maintained lists that can drift, and `locked` (row 11) is absent from
both.

Doing: add `locked`; add a test that pins the builder vocabulary against the
canonical list so drift fails a test rather than silently losing a predicate;
keep the saved-filter rows (row 14) as the persistent entry point.

### Row 16 — glyph database, overlay, Update Glyph Info

Works: a shipped `data/glyphdata.json` (226 curated records, 154 with a
composition, `_top`/`_bottom` entry anchors on marks) resolved against Unicode
properties with suffix inheritance; a validated per-project overlay at
`font.info.user_data["com.fontcarver.glyphData"]` editable as JSON or imported
from a file; `Command::UpdateGlyphInfo` applying Unicode, category, subcategory,
script and production name in one undo unit; a panel showing the effective
record including anchors and composition.

Missing: "Update Glyph Info" is reachable only for the one glyph open in the Edit
view, not for a font-view selection; the anchors and composition the database
defines are shown but never used to build anything; the badge interaction in
row 11.

Doing: a font-view `glyph.updateInfo` command over the whole selection in one
undo unit; database-driven composition (row 17); keep production names in export
as the existing opt-in export option and verify it with FontTools rather than
changing task f's export code.

### Row 17 — Add Glyphs and batch rename

Works: one recipe grammar covering names, single characters, `U+XXXX`/`uniXXXX`,
inclusive ranges and `@charset:` repertoires, with a dry run that previews
exactly what would be created; batch rename with literal find/replace, a
paginated before/after preview, whole-font collision validation and one atomic
`Batch` that moves every source through a fresh temporary name so chains and
cycles work.

Missing: created glyphs are always empty, so adding `Aacute` gives no
composition; rename has no regular-expression mode
(`docs/p5-batch-rename-acceptance.md` records that as out of scope); and
`Font::rename_glyph` updates components, constructions, kerning pairs and
groups, bitmaps, CID, MATH, colour and SVG but _not_ metrics keys, feature
classes/feature code or saved glyph lists, so a rename silently breaks those.

Doing: `glyphdata.generate_glyphs` builds each missing glyph from the database
composition, placing marks from real `_x`/`x` anchor pairs per master layer and
carrying the base advance and anchors, in one undo unit, with a dry run; the Add
Glyphs dialog gets a composition toggle and every generate path uses it. Rename
gains the missing reference updates inside
`crates/fc-doc/src/command/glyphs.rs` (this task's file) instead of
`crates/fc-model/src/font.rs`, which this task does not own. Batch rename gains a
regex mode with capture groups, a case-insensitive option and inline invalid-regex
feedback.

Limit: feature source is stored as FEA text, so its rename is a token-level
rewrite that skips comments, strings, tags, class names and keywords; it is
tested, but a glyph name that only appears inside an `include()`d file or a
generated table cannot be rewritten.

### Row 18 — CJK fallback cells and CJK sidebar sections

Works: an empty cell whose glyph has a usable scalar already paints that
character from the UI font stack — the system face — clipped to the cell in the
muted colour (`cellRenderer.ts`: `emptyGlyphCharacter`, `paintReferenceCharacter`),
and the CJK sidebar has radical, stroke-count and Unicode-block sections
(`CjkFilters.tsx`, now owned by task l, plus the Blocks panel).

Missing: the fallback has no switch, so a user who wants truly empty cells cannot
get them; no acceptance evidence that the fallback and the CJK sections work
together on a CJK font.

Doing: a persisted "reference character in empty cells" toggle in the font-view
toolbar (`app/src/dialogs/PreferencesDialog.tsx` is not in this task's file
list, so the switch lives with the other font-view presentation settings and is
recorded as such), and browser evidence on a CJK fixture.

### Cross-task items (named, not silently worked around)

- `doc.sidebar` (task a, `crates/fc-api/src/handlers/doc.rs`) counts
  categories/scripts/blocks from Unicode properties only, while cells and smart
  filters honour explicit `Glyph.category`/`script` overrides, and `matches_filter`
  in the shared `handlers/glyphs.rs` follows `doc.sidebar`. Leaving all three
  aligned on Unicode properties is the consistent choice; moving them to the
  effective classification needs one change in `compute_sidebar` and one in
  `matches_filter`, together.
- `app/src/stores/documentStore.ts` (task i's shared file) has no
  `setTableColumns`; the column set is restored by replaying `moveTableColumn`.
- `app/src/views/glyph/panels/LayerList.tsx` (not in this task's list) could show
  the layer-colour swatch on each row; the swatch is added to `LayerColorField`,
  which the Layers panel renders for the selected layer.
- `app/src/dialogs/PreferencesDialog.tsx` (not in this task's list) is where the
  empty-cell reference-character switch would belong in the long run.
