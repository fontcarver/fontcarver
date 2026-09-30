# P5f-b acceptance: glyph inventory, search, filters and glyph data

Area `b-inventory`, rows 10–18 of [the feature audit](p5-feature-audit.md).
Protocol: `superpowers/plans/tasks/p5f-protocol.md`. The gap analysis this work
started from is appended to `superpowers/plans/tasks/p5f-b-inventory.md`.

## Scope

What this task added to the code that was already there, per row:

| Row | Added                                                                                                                                                                                                              |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 10  | `Category`, `Script` and `Status` grid orders; a column-visibility menu and a Subcategory column; persisted view mode, cell size, label mode and column set                                                        |
| 11  | `has_custom_glyph_info` redefined as divergence from the database; a `locked` filter predicate; a sidebar Statuses section with one toggle per badge                                                               |
| 12  | `layer_color_label` on the grid cell and its corner mark; colour-label commands over a grid selection in one undo step; a swatch in the layer colour field                                                         |
| 13  | Inline reporting of a rejected search or smart expression beside the control that owns it                                                                                                                          |
| 14  | `inventory.coverage` and `inventory.missing`; present/total badges on categories, subcategories, scripts and blocks; a Generate button on every row that lacks members; match counts on saved filters              |
| 15  | `locked` in both the kernel vocabulary and the visual builder                                                                                                                                                      |
| 16  | `Update Glyph Info` over a font-view selection; database compositions and anchors put to use                                                                                                                       |
| 17  | `glyphdata.generate_glyphs`: composites built from the database with anchor-placed marks, in one undo step; regular-expression batch rename; renames now follow metrics keys, feature source and saved glyph lists |
| 18  | A persisted switch for the empty-cell reference character                                                                                                                                                          |

## Decisions recorded

- **What "present / total" means.** The denominator is a repertoire: the
  assigned Unicode scalars of that category, subcategory, script or block; the
  numerator is how many of them the font encodes. `present + missing = total`,
  so "generate the missing ones" has a definite set. The glyph count the row's
  filter selects answers a different question and is in the row's tooltip. A
  saved smart filter has no repertoire, so its badge is matches / glyphs in the
  font.
- **Where coverage is computed.** `doc.sidebar` belongs to task a, so coverage is
  a separate pair of methods in this task's `handlers/inventory.rs`, built on new
  repertoire tables in `fc-glyphdata`.
- **Composition placement.** A member with an entry anchor (`_top`) matching an
  anchor already on the layer is a mark: it is offset so the two coincide and
  does not move the pen. Every other member is placed at the pen and advances it.
  That composes `Aacute` as a base plus an attached accent and `f_f` as a
  ligature, with no invented coordinates — the database names anchors but holds
  none.
- **One classification everywhere.** `doc.sidebar`, `matches_filter`, the cell
  and the smart-filter evaluator all use the glyph's effective category,
  subcategory and script, so an explicit override moves a glyph between sidebar
  sections and its count and its listing agree. Coverage denominators stay
  Unicode repertoires, which is what "total" means.
- **Ordering is ascending only**, which matches the reference application. A
  descending toggle would need `GlyphListParams` and the document store.

## Commands

### Kernel

`TMPDIR` was an absolute path under `.scratch/p5f-b-inventory/tmp` and every
cargo command ran under `flock .scratch/p5f/cargo.lock`.

- `cargo test -p fc-glyphdata` — **27 passed** (4 repertoire unit tests, 13
  database, 6 names, 4 Unihan), 0 failed.
- `cargo test -p fc-doc` — full crate, **338 passed, 0 failed**, including the
  new `--test glyph_generation`: **11 passed** (mark attachment, per-master
  composition that still interpolates, the ligature pen rule, a missing member,
  an overlay composition, staged members, and the rename that follows metrics
  keys, feature source and glyph lists through undo and redo).
- `cargo test -p fc-api --lib` — **236 passed**, 0 failed, including the
  rewritten `glyph_status` unit tests and the new `locked` /
  `has_custom_glyph_info` predicate tests.
- `cargo test -p fc-api --test inventory_coverage` — **3 passed**: coverage
  numerators, denominators and glyph counts; bounded, ordered missing members
  and their recipe; coverage following an edit and a secondary encoding.
- `cargo test -p fc-api --test glyph_generation_api` — **8 passed**: dry run and
  apply as one undo step with undo/redo, skips, the 8,192 bound with staged
  members, a count-only request that builds no cells, the three new orders,
  `layer_color_label` with the `locked` filter, and task i's placeholder badge
  with its three filter spellings.
- `cargo test -p fc-api --test glyph_data --test glyph_list --test glyph_lists
--test sidebar_counts --test smart_filters --test smart_filter_search --test
glyph_recipes --test editing` — **30 passed**, 0 failed (the existing
  inventory suites, unchanged by this work).
- `cargo clippy -p fc-glyphdata -p fc-doc -p fc-api --all-targets -- -D
warnings` — clean.
- `rustfmt --edition 2021` over every Rust file this task changed — clean.

### Frontend

Focused Vitest runs used `pnpm --dir app exec vitest run --maxWorkers=1
--minWorkers=1` with the same absolute `TMPDIR`.

- `src/views/font/ src/dialogs/AddGlyphsDialog.test.tsx
src/dialogs/BatchRenameDialog.test.tsx src/dialogs/batchRename.test.ts
src/views/glyph/panels/{LayerColorField,GlyphDatabasePanel,GlyphTagsField}.test.tsx`
  — **38 files, 279 tests, 0 failed**, including the new files and cases:
  - `fontViewPrefs.test.ts` (4): column order, set and the one-column floor;
    reading back a stored presentation and repairing a corrupt one; seeding a
    document without overruling an explicit choice; storage that throws.
  - `queryErrors.test.ts` (2): which kernel rejections belong to which control,
    and per-document reporting and clearing.
  - `GlyphSearch.test.tsx` (+2): the inline report, its `aria-invalid` and
    description, isolation between documents, and clearing on a valid query.
  - `SidebarFilters.test.tsx` (+2): the present/total badge and its tooltip, the
    Generate button and its truncation notice, and filtering by a cell badge.
  - `GlyphTable.test.tsx` (+1) and `GlyphTableHeader.test.tsx` (+1): the column
    menu and the persisted set; every column sorts; a subcategory column shares
    the category order.
  - `cellRenderer.test.ts` (+3): the reference-character switch, the layer
    colour mark, and an out-of-range stored layer label.
  - `GlyphGrid.test.tsx` (+1): the context menu's twelve labels and None.
  - `fontCommands.test.ts` (+3): one batched undo step for a labelled selection,
    clearing it, `Update Glyph Info` at the revision the database reports, and
    the gates.
  - `AddGlyphsDialog.test.tsx` (+1): composites by default, the blank-glyph
    toggle and which method each one calls.
  - `batchRename.test.ts` (+4) and `BatchRenameDialog.test.tsx` (+3): capture
    groups, case handling in both modes, and an unfinished pattern reported
    without staging anything.
- `pnpm --dir app check` — no error in any file of this task. The run as a whole
  is red on other tasks' files (`automation/*`, `dialogs/Export*`,
  `views/features/*`, `views/crossdocument/*`, `views/glyph/panels/GlyphVariationPanel.tsx`).
- `pnpm --dir app exec eslint src/views/font src/dialogs/{AddGlyphsDialog.tsx,addGlyphs.ts,BatchRenameDialog.tsx,batchRename.ts} src/views/glyph/panels/LayerColorField.tsx`
  — clean.
- `pnpm --dir app exec prettier --write` over the same files and
  `docs/user/glyph-inventory.md` — clean.
- `pnpm --dir app gen:api` regenerated `app/src/api/generated/` after the new
  types; the generated files were not hand-edited.

### Browser

`app/e2e/fontview.spec.ts` runs against `fc-devserver --open
fixtures/fonts/DejaVuSans.ttf` plus Vite, on this task's ports
(`FC_API_PORT=8802 FC_UI_PORT=1502`), serially (`--workers=1`). Six cases were
added to the four this file already had.

The four cases this file already had still pass unchanged. The six added cases:

1. **List mode** — every column heading sorts and marks itself `aria-sort`,
   Width really reorders the kernel's answer (the narrowest glyph first), the
   column menu adds Subcategory, and the column set survives a reload.
2. **Search** — an unfinished regular expression is reported under the field with
   `aria-invalid`; completing it clears the report, and Match case narrows
   `^A$` from two matches to one.
3. **Sidebar** — Basic Latin badges `… / 128` with a tooltip that names the
   characters encoded and missing; a badge row filters the grid.
4. **Colour labels** — a labelled selection appears in the sidebar's colour
   section and one undo removes it.
5. **Add Glyphs** — `f_f` previews as one composite, is created, and appears in
   the grid; undo removes it.
6. **Batch rename** — a regular expression with a capture group renames
   `aacute` to `a.acute` through the dialog's preview; undo restores it.

### Independent inspection

The 65,535-glyph frame-time reading for row 10 is
[P0's](p0-acceptance.md): **16.8 ms p95 (< 20 ms)** while traversing all 65,535
cells of Source Han Sans SC, with a bounded page cache and cell 65,535 rendered.
Nothing in this task changes how the grid pages or paints; the frame meter
(`views/font/frameMeter.ts`) and the `__fwGridStats` probe are unchanged. The
fixture is present on this machine
(`../SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf`); a fresh
reading needs a release kernel, which this shared checkout does not build, so
the P0 figure stands as the record and no new number is claimed.

### Screenshot

`docs/screenshots/p5f-b-inventory/sidebar-coverage-and-statuses.png` — the font
view on DejaVu Sans, captured from the browser run. It shows the **Statuses**
section with one row per cell badge, including the placeholder badge; the
**Categories** section badging `Letter 3,277 / 145,672` and
`Lowercase 1,084 / 2,283` with a Generate button on each; the **Reference
characters** toggle in the toolbar; and the smart-filter and glyph-list panels.

### Emitted font, inspected independently

DejaVu Sans was opened in the kernel, `f_f` was generated with
`glyphdata.generate_glyphs`, and the document was exported as TTF with
`production_names: true` (script:
`.scratch/p5f-b-inventory/verify_export.py`, removed with the scratch
directory; its output is reproduced here):

```
generated: ['f_f']
composed: [{'name': 'f_f', 'components': ['f', 'f'], 'unavailable': []}]
exported: …/DejaVuWithComposite.ttf 606644 bytes
f_f components: [('f', 0, 0), ('f', 721, 0)]
advance: (1442, 47) f advance: (721, 47)
FreeType f_f outline points: 40
HarfBuzz 'ff': [(73, 721), (73, 721)]
OK
```

- **FontTools** reads `f_f` from `glyf` as a composite of two `f` components at
  x = 0 and x = 721 — exactly `f`'s advance — and `hmtx` gives it 1442, the sum
  of the two. The composition the database describes survives to the binary.
- **FreeType** finds `f_f` by name and loads 40 outline points for it, so the
  composite resolves in an independent rasteriser.
- **HarfBuzz** shapes `ff` against the exported font and returns two glyphs with
  721 advance each, so the font is valid to an independent shaper.
- Exporting with and without `production_names` produced the same 6,254-glyph
  font for this fixture, whose names already match their production names; the
  option is wired and the output is valid, which is all that is claimed here.

### Browser runs

After the dialog-height fix and the corrected menu selector,
`fontview.spec.ts` passes in full:

```
10 passed (27.6s)
```

All ten cases — the four this file already had plus list mode, search, the
sidebar badge, colour labels, Add Glyphs and batch rename — run green on this
shared checkout, each in a few seconds. The two editing cases that previously
timed out did so because the Add Glyphs dialog grew past a 720 px window and
took its confirm button with it; the earlier report blamed concurrent
`pnpm gen:api` runs wiping `app/src/api/generated/`, which did also happen and
did break page loads, but was not the cause of those two failures.

One thing diagnosing them fixed: the Add Glyphs character-set counts fetched a
full cell for every glyph of a repertoire, which cost **52.9 s** for
`@charset:gb18030level1` on a debug kernel. They now use a count-only request,
measured at **0.19 s** for the same repertoire.

## Review fixes

A review returned ACCEPT WITH FIXES. What changed:

1. **Update Glyph Info no longer destroys encodings** (blocking). It wrote
   `resolve_info`'s suggestion over `glyph.unicodes` unconditionally, and that
   suggestion is empty for any suffixed name and falls back to AGLFN for a name
   the database does not list. A suffixed private-use glyph therefore lost its
   `cmap` entry, and `Omega` was re-encoded U+03A9 → U+2126. The refresh now
   writes encodings only from `fc_glyphdata::explicit_unicodes`, which answers
   only for a record naming that exact glyph, and never clears one. Covered by
   `update_glyph_info_preserves_encodings_it_does_not_own` (suffixed PUA glyph,
   `Omega`, an unlisted name, a listed name that is refreshed, and undo) and
   `an_overlay_record_for_the_exact_name_re_encodes_a_glyph`.
   **`aglfn.txt` was deliberately not edited.** Its `2126;Omega;OHM SIGN` is not
   a transcription error: AGLFN 1.7's own revision history (lines 74–76)
   reverted Delta, Omega and mu to the 1.4 mappings, which is why line 89's 1.5
   entry disagrees with line 210. The file is verbatim upstream and its sha256
   is recorded in `THIRD_PARTY.md`; editing it would diverge from upstream and
   invalidate that hash. Treating AGLFN as authority for what a glyph _is
   encoded as_ was the bug, and that is what the fix removes.
2. **Dialogs never exceed the window** (blocking). `.dialog` in
   `app/src/shell/dialog.module.css` had no height cap, so the Add Glyphs form
   grew past a 720 px window and took its confirm button with it. It is now a
   flex column capped at `calc(100vh - 12vh - var(--space-4))` — the overlay
   reserves 12vh — with a `min-height: 0` scrolling body and a fixed header and
   footer. The e2e case sets a 1280×720 viewport to hold the fix in place.
3. **The batch-rename e2e selector was wrong**: it matched `/^Batch rename/`
   while the menu item reads "Batch Rename Glyphs…", so that case could never
   pass. Both selectors are corrected. The earlier claim that the two editing
   cases "pass when run on their own" was mistaken for the rename case and is
   withdrawn.
4. **`markClass` no longer blocks a rename.** It was in `FEA_NAMING_KEYWORDS`,
   but its next token is a glyph, so `markClass acutecomb <anchor…>` was skipped.
   Removed, with the rename test now asserting a `markClass` line follows.
5. **The custom-info badge reads the document overlay.** `has_custom_glyph_info`
   compared against the bundled data alone, so a refresh driven by a project
   overlay relit it. The overlay is now read once per request in
   `document_overlay` and threaded through `build_cell`, the badge weight and
   the evaluator. `glyphdata.state.custom` keeps its own, wider meaning — "the
   database carries a project record for this glyph" — which is now documented
   on the field and asserted in `custom_glyph_info_follows_the_document_overlay`.
6. **Sidebar wording and number formatting.** `sidebar.generateMissing` said
   "missing letters" on Symbol, Number, Script and Block rows and printed a bare
   count; it now says "characters" and formats every count through the locale.
   A rejected query is wrapped in `glyphSearch.invalidQuery` and collapsed to one
   line instead of printing a raw multi-line kernel message.
7. **One vocabulary, generated.** `SMART_FILTER_FIELDS` now lives in
   `crates/fc-api/src/types/smart_filters.rs`, drives `field_kind` (which keeps
   only the alias table), and is emitted to
   `app/src/api/generated/smartFilterFields.ts`, which the builder re-exports.
   The builder test asserts identity with the generated list and that every
   field's default rule uses one of its own operators, so a hand-written copy
   cannot drift again.
8. **Dead state removed.** `tableColumns` and `moveTableColumn` left
   `documentStore`; the column set lives only in the font view's preferences.

Also done, as the reviewer suggested: **`compute_sidebar` and `matches_filter`
now classify by the effective category, subcategory and script**, so a glyph
with an explicit override is counted in the section its cell shows and that
section's filter selects it. `sidebar_counts.rs` asserts the counts and the
filters agree for overridden and underived glyphs alike. This retires the
consistency item previously recorded as a limit.

## Limits recorded

1. **Ordering is ascending only.** Every column sorts, but not in reverse. A
   descending flag belongs on `GlyphListParams` and in the document store.
2. **The Status order resolves badges per request.** For `GlyphSort::Status`
   the metrics and interpolation checks run over the whole matched set on each
   page request; on a single-master font both are O(1) per glyph, and on a large
   multi-master font this costs a geometry compare per glyph per page. Caching
   the weights per document generation on `Api`, next to `sidebar_cache`, is the
   fix if it is ever felt.
3. **Coverage is recomputed per request.** `inventory.coverage` takes 0.4–0.8 s
   on DejaVu Sans (6,253 glyphs) with a debug kernel under load, behind a 400 ms
   debounce; it is not cached. The repertoire totals themselves are built once
   per process.
4. **A rename cannot follow a glyph name out of the document.** Feature source
   is text, so the rewrite is token-level: it skips comments, strings, `@`class
   names, feature/lookup/script/table names and the syntax keywords, and it
   leaves a glyph whose name is itself a feature keyword alone. A name that only
   appears in an `include()`d file is not rewritten. Saved smart-filter
   expressions are queries the user wrote, not references, and are left alone.
5. **Composition uses the anchors that exist.** The database names anchors but
   carries no coordinates, so a mark whose entry anchor has no match on the base
   is placed at the pen rather than somewhere invented, and a composite built
   from glyphs that carry no anchors gets no attachment. The database ships 154
   compositions; a name outside it is created empty.
6. **Generation is bounded** at 8,192 glyphs per request and 4,096 preloaded
   missing characters per Generate click; both report the remainder rather than
   truncating silently.
7. **Production names in export stay opt-in** (`ExportOptions.production_names`,
   default false), which is task f's surface; this task did not change it.
8. **The empty-cell switch lives in the font-view toolbar**, not in Preferences,
   because `app/src/dialogs/PreferencesDialog.tsx` is not in this task's file
   list.

## Cross-task notes

- **Task i's placeholders are badged.** A cross-document transfer writes an
  empty, non-exporting glyph tagged `missing-component-base` for a component
  base the destination lacks (contract in `crates/fc-doc/src/fcf/schema.md`).
  `GlyphCell.placeholder` now reports it, the grid and the list show a
  Placeholder badge with a tooltip, the sidebar's Statuses section filters by
  it, and `is_placeholder` is a smart-filter predicate in both the kernel
  vocabulary and the visual builder. Covered by
  `a_cross_document_placeholder_is_badged_and_filterable` and by the component
  and sidebar tests.
- **Left for the owners of those files:**
  `app/src/views/glyph/panels/LayerList.tsx` could show the layer colour on each
  row, not only in the field, and `app/src/dialogs/PreferencesDialog.tsx` is
  where the empty-cell switch would belong. The sidebar/filter classification
  split is gone: `compute_sidebar` (task a) and `matches_filter` now use the
  effective classification together.
- **Shared files touched, narrowly:** `crates/fc-api/src/api.rs` (three method
  registrations and their dispatch lines), `crates/fc-api/src/bin/gen_ts.rs`
  (nine type exports), `crates/fc-api/src/types/mod.rs` (one module),
  `crates/fc-api/src/types/glyphs.rs` (three `GlyphSort` variants, two
  `GlyphCell` fields), `crates/fc-api/src/handlers/glyphs.rs` (the new orders,
  the layer colour and the placeholder flag), `crates/fc-api/src/handlers/editing.rs`
  (one field in `preview_cell`), `crates/fc-doc/src/command.rs` (two exports),
  `crates/fc-doc/src/document.rs` (one call, so undo and redo of a rename
  restore the references this task added), `crates/fc-glyphdata/src/lib.rs`
  (one module and its exports), `app/src/i18n/en.json` (new keys under
  `fontView`, `sidebar`, `smartFilters`, `inspector.sortMode`, `cmd.glyph`,
  `toast`, `dialog.addGlyphs` and `dialog.batchRename`), and
  `app/src/views/glyph/panels/panels.module.css` (one rule).
- **Blocked on other tasks:** `cargo clippy -p fc-api --all-targets -- -D
warnings` cannot pass while `handlers/variation.rs` has three
  `unnecessary if let` errors; `pnpm --dir app check` cannot pass while the
  automation, export, features and cross-document test files disagree with their
  regenerated types.

## Recommended status per row

| Row | Status                                                                                                                                                                                                                                                                                                                                                                                                  |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 10  | **Complete** within limit 1 (ascending only): virtualised grid with the P0 frame reading, cell size, grid/list, every column sortable, drag and keyboard reorder, a column set that is chosen and remembered                                                                                                                                                                                            |
| 11  | **Complete**: six badges with real sources, tooltips, a seventh for task i's placeholders, all of them filterable, and a sidebar section of one-click filters                                                                                                                                                                                                                                           |
| 12  | **Complete**: twelve labels set over a selection in one undo step, an independent layer colour shown in the cell and the layers panel, tags with autocomplete, all three filterable                                                                                                                                                                                                                     |
| 13  | **Complete**: name/Unicode/note scopes, case and regex toggles, and an invalid pattern reported inline while the document is untouched                                                                                                                                                                                                                                                                  |
| 14  | **Complete**: present/total on categories, subcategories, scripts, blocks, languages and lists, matches on filters, and Generate on every row that lacks members, creating them with the database's compositions in one undo step                                                                                                                                                                       |
| 15  | **Complete**: one predicate tree behind the text and the builder, 35 fields in both, project persistence, canonical round trip                                                                                                                                                                                                                                                                          |
| 16  | **Complete**: a shipped database with a per-project overlay covering name, Unicode, category, production name, anchors and composition; Update Glyph Info over a selection in one undo step; composition and anchors now used, not only displayed, and verified in the exported binary with FontTools, FreeType and HarfBuzz. Export uses production names through the existing export option (limit 7) |
| 17  | **Complete**: all five Add Glyphs forms with a preview, composites built from the database, batch rename with find/replace and regular expressions, a preview, one undo step, and references followed into metrics keys, feature source and glyph lists (limit 4). Both browser cases pass in the full run                                                                                              |
| 18  | **Complete**: empty cells draw their character from a system face in a muted style, with a remembered switch; the radical, stroke-count and Unicode-block sections are in place (`CjkFilters`, now task l's file, plus the Blocks panel)                                                                                                                                                                |
