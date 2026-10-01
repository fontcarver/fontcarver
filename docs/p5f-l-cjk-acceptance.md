# p5f-l: CJK completion acceptance

Area `l-cjk` of [completion wave 2](superpowers/plans/2026-09-30-p5-completion.md),
covering feature-audit rows **19**, **110**, **111** (the Edit View and `VORG`
part) and **112**. Task file and gap analysis:
[`p5f-l-cjk.md`](superpowers/plans/tasks/p5f-l-cjk.md). Working rules:
[`p5f-protocol.md`](superpowers/plans/tasks/p5f-protocol.md). Ports: API 8812,
UI 1512.

The gap analysis in the task file was written before any code changed; this
document records what was built and what was verified against the running
kernel and against independently decoded binaries.

## Scope

| Row | Requirement                                                                   |
| --- | ----------------------------------------------------------------------------- |
| 19  | Vertical metrics in the inspector: TSB/BSB, vertical origin, vertical advance |
| 110 | CID mapping, CID-ordered sort, CID-keyed export                               |
| 111 | Vertical writing mode in the **Edit View**, with `VORG` and origin editing    |
| 112 | Radical library, ideographic description sequence input, transform grid       |

Row 111's proofing surfaces belong to task `h`; this record covers the Edit View
and `VORG` only.

## What was built

### Row 19 — the inspector's vertical metrics

`VerticalMetricsPanel.tsx` already edited the vertical advance, the vertical
origin, TSB and BSB from the kernel's resolved ink bounds, so no control was
added. What was missing was proof that it obeys the same guards as every other
editing command, which is what wave 1's reviews found missing elsewhere.
`crates/fc-doc/tests/vertical.rs` now carries
`vertical_metric_guards_hold_and_edited_bearings_survive_fcf`: six out-of-range
or non-finite values are each refused with the document byte-identical
afterwards, a locked glyph refuses the edit, a read-only document refuses it
without adding a history entry, the two overrides survive a `.fcf` write and
read, and clearing both is one undoable step back to master inheritance.

### Row 110 — CID

The mapping editor, the `GlyphSort::Cid` order and the CID-keyed CFF writer all
existed. Two things were added:

- **A two-FontDICT export test.** `crates/fc-compile/tests/cid.rs` gains
  `cid_keyed_export_writes_an_fdarray_and_fdselect_for_two_font_dicts`, which
  gives the master a second CFF private dictionary with `LanguageGroup 1`,
  points two glyphs at it, exports, and reads the result back through
  `read_fonts::ps::cff::CffFontRef`: the font is CID-keyed, `FDSelect` routes
  glyphs 0–1 to FontDICT 0 and glyphs 2–3 to FontDICT 1, both subfonts parse,
  and reopening restores both dictionaries and the per-glyph selection.
- **An unmapped-glyph count in the dialog.** `cidSession.unmappedGlyphs`
  reports the glyphs with no CID that the writer actually orders — the ones with
  `export` set, `.notdef` excluded because the writer always gives it CID 0 —
  and `CidEditor` shows the count and the first names. `cid.state` returns that
  `exported` list beside the full glyph order. CID-keyed export refuses a font
  with an incomplete mapping, and that failure was previously invisible until
  export time.

### Row 111 — vertical writing mode and the canvas handles

- **`overlays.verticalMode`** is a new overlay with the usual View-menu and
  command-palette entry (`view.glyph.verticalMode`, "Vertical Writing Mode").
  In that mode `drawMetrics` promotes the vertical origin and vertical advance
  from faint dashed reference lines to solid, always-labelled metric lines and
  outlines the vertical advance box around the glyph.
- **Two new metric handles.** `metricsHandles.ts` gains `vorigin` and
  `vadvance`: `metricLineY` puts them exactly where `drawMetrics` paints them,
  `metricsHit` offers them only in vertical writing mode, and then only when no
  sidebearing or advance line was grabbed (so no press that worked before
  behaves differently), `metricCursor` gives
  them `ns-resize`, and `metricValue` reads the advance in the opposite sense —
  pulling the lower line down lengthens the advance — and clamps both to the
  ranges the kernel accepts, so a long drag stops at the limit instead of being
  refused. The gesture reuses `MetricsDrag`, so a whole drag is one
  `edit_begin`/`edit_apply`/`edit_end` run and one history entry, Escape
  abandons it and the HUD shows the live value.
- **Inherited metrics stay inherited.** `SetVerticalMetrics` writes both
  fields, so a drag sends the layer's **stored** value for the field it does not
  own — `null` included. Dragging the origin of a glyph that inherits its
  advance from the master therefore does not freeze a copy of that advance.
  `EditorSource.verticalOverrides` carries those stored values from
  `GlyphView.tsx`.

`VORG` already recorded only the origins that differ from the default;
`otf_vertical.rs` now pins that with
`vorg_records_only_the_origins_that_differ_from_the_default`, which reads the
raw table: one exceptional glyph writes a 12-byte table with one record, and a
uniform font writes the 8-byte header with `numVertOriginYMetrics = 0` while
every glyph still resolves to the right origin.

### Row 112 — radical library, IDS, transform grid

- **The radical library is now derived from the shipped Unihan data.**
  `fc-glyphdata` gains `kangxi_radicals()`: for each of the 214 radicals it
  holds the Kangxi Radicals block character and every Basic Multilingual Plane
  ideograph whose `kRSUnicode` assigns that radical with no residual strokes —
  which is how the source spells "this character _is_ the radical" — with each
  form's own smallest `kTotalStrokes`. Compatibility ideographs are excluded
  because they only repeat characters already listed. That derivation produces
  人 with 亻, 心 with 忄, 水 with 氵 and 氺, 言 with 訁 and 讠, 金 with 釒 and 钅,
  糸 with 糹 and 纟, 手 with 扌, 才 and 龵, from data rather than from the
  hand-typed string the panel used before.
- **A new `cjk.radicals` kernel method** returns, for one document, the 214
  radicals with their forms, each form's stroke count, the document glyph that
  encodes it, and how many of the document's glyphs use that radical, together
  with the glyphs the document already uses as components, most used first.
- **`CjkRadicalLibrary` was rebuilt on it**: a _Radicals_ tab and a _Font
  components_ tab, search by radical number, stroke count, character, glyph name
  or `U+XXXX`, an "Only forms this font has" switch, outline previews fetched
  per page by exact glyph name, and a picked entry appended to the IDS input by
  glyph name when the font has the glyph and by character when it does not. It
  refetches both on the panel's document generation and on the kernel's own
  document event, so appending components is reflected without reopening.
- **`CjkTransformGrid` was rebuilt as a staged editor**: position, scale and the
  two shear coefficients for every component of the layer, a live preview above
  the fields that follows the draft with no kernel round trip, **Apply
  transforms** writing every changed component as one `Batch` and one history
  entry, and **Revert transforms**. A value typed back to its stored value stops
  counting as a change, a non-numeric entry is refused by the field, a scale
  that would collapse a component is refused with the reason, and a reload that
  changed the component list drops the staged numbers instead of aiming them at
  different components.

## Commands

All runs used `TMPDIR=…/.scratch/p5f-l-cjk/tmp` and the shared cargo lock.

```text
cargo test -p fc-glyphdata
  unittests src/lib.rs   8 passed   (4 of them the new Kangxi radical tests)
  tests/database.rs     13 passed
  tests/names.rs         6 passed
  tests/unihan.rs        4 passed
  → 31 passed, 0 failed

cargo test -p fc-api --test cjk                      7 passed, 0 failed
cargo test -p fc-doc --test vertical                 3 passed, 0 failed
cargo test -p fc-doc --test cjk_construction         7 passed, 0 failed
cargo test -p fc-doc --test cid                      1 passed, 0 failed
cargo test -p fc-compile --test cid                  3 passed, 0 failed
cargo test -p fc-compile --test otf_vertical        10 passed, 0 failed
  → 62 passed, 0 failed across the focused Rust suites
```

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/glyph/panels/{CjkTransformGrid,CjkConstructionPanel,CjkRadicalLibrary}.test.tsx \
  src/views/glyph/panels/{cjkTransforms,cjkRadicalEntries,cjkConstruction,cjkResolve}.test.ts \
  src/views/glyph/panels/{VerticalMetricsPanel,EditInspector}.test.tsx \
  src/views/glyph/{metricsHandles,drawCollisionFixes}.test.ts \
  src/views/fontinfo/cidSession.test.ts src/views/font/{CjkFilters,CjkBatchDialog}.test.tsx \
  src/stores/editorStore.test.ts src/editor/editorController.test.ts \
  src/views/glyph/{GlyphView.test.tsx,glyphCommands.test.ts}

Test Files  18 passed (18)
     Tests  253 passed (253)
```

```text
pnpm --dir app exec eslint <every file this task touched>      clean (exit 0)
pnpm --dir app exec prettier --check <same files, en.json>     clean
rustfmt --edition 2021 <every Rust file this task touched>     clean
cargo clippy -p fc-glyphdata --all-targets -- -D warnings      clean
cargo clippy -p fc-api --lib --test cjk                        no fc-api finding
cargo clippy -p fc-doc --test vertical --test cjk_construction no fc-doc finding
cargo clippy -p fc-compile --test cid --test otf_vertical      no fc-compile finding
```

`pnpm --dir app check` reported no error in any file this task touched. While
the work was being done it did not pass overall — `src/views/bitmap/**`,
`src/views/text/**`, `src/dialogs/Export*` and the `PsHints`/`ExportOptions`
fixtures were mid-edit in tasks `g`, `h`, `f` and `k`, and
`cargo clippy -p fc-api -- -D warnings` stopped on
`crates/fc-formats/src/type1/hints.rs:617` (`type_complexity`), task `k`'s file.
Both pass in full at the end of the review round (`tsc -b` exits 0). The one
expected failure is `src/i18n/i18n.test.ts`, because this task adds English keys
the separate translation pass has not reached yet.

## Browser verification

Session `p5f-l-cjk`, headless Chrome at 1680×1050, `target/debug/fc-devserver
--port 8812` with Vite on 1512. The fixture is a 46-glyph subset of Source Han
Sans SC (`pyftsubset`, kept in task scratch and deleted afterwards): a real
CID-keyed CFF with `ROS Adobe-Identity-0`, **three** FontDICTs, `vhea`, `vmtx`
and a `VORG` whose default origin is 880 with no records.

**Row 19.** Opened 林 (`uni6797`). The inspector showed vertical advance 1000,
vertical origin 880, TSB 39, BSB 42. Typing TSB 59 moved the origin to 900 and
left the advance at 1000 (BSB followed to 22); typing BSB 62 moved the advance
to 1040 and left the origin at 900 and TSB at 59. The horizontal advance never
moved. One Edit-menu undo reverted exactly the BSB edit and one redo restored
it, so each field edit is one history entry.
([vertical-inspector.png](screenshots/p5f-l-cjk/vertical-inspector.png))

**Row 110.** Font Info → CID mapping showed the imported 46-line map with real
non-identity CIDs (`34 uni0041`, `9774 uni4EBA`, `42642 uni94C1`). Deleting the
`uni4EBA` line produced "1 glyph has no CID: uni4EBA. CID-keyed CFF export needs
one for every exported glyph."; re-adding it as `65000 uni4EBA` cleared the
notice, and Apply changes committed it.
([cid-mapping.png](screenshots/p5f-l-cjk/cid-mapping.png)) With the grid's
**CID** sort, `uni4EBA` moved from position 6 to position 46 of 46, so the order
reads the live mapping. ([cid-sort.png](screenshots/p5f-l-cjk/cid-sort.png))

**Row 111.** View → Vertical Writing Mode drew the vertical origin and advance
as solid labelled lines with the advance box around the glyph
([vertical-mode-canvas.png](screenshots/p5f-l-cjk/vertical-mode-canvas.png)).
The canvas transform was derived from the application's own pointer readout, so
every coordinate below is in font units.

- The cursor over the origin line and over the advance line is `ns-resize`, over
  the horizontal advance line `ew-resize`, and `default` between them.
- Dragging the origin line from 900 upwards set the origin to **959** with the
  advance unchanged at 1040 (TSB 59→118, BSB 62→3). The HUD read
  `vert. origin 959` during the drag.
  ([vertical-origin-drag.png](screenshots/p5f-l-cjk/vertical-origin-drag.png))
- Dragging the advance line down set the advance to **1187** with the origin
  unchanged at 959 (BSB 3→150). The HUD read `vert. advance 1187`.
  ([vertical-advance-drag.png](screenshots/p5f-l-cjk/vertical-advance-drag.png))
- From origin 758, the identical press-and-move on the origin line left it at
  **758** when Escape was pressed before the release, and set it to **611** when
  released — the same gesture, abandoned and completed.
- The Edit menu named the entry "Undo Set vertical metrics" after each drag.

**Row 112.** The CJK construction section's Radicals tab listed 21 entries with
"Only forms this font has" on and **525** with it off, which is exactly the 214
Kangxi block characters plus the 311 Basic Multilingual Plane forms the data
yields. Searching `85` returned the block character U+2F54 and all three written
forms with their own stroke counts — 水 U+6C34 (4), 氵 U+6C35 (3), 氺 U+6C3A (5) —
each matched to this font's glyph, with "5 glyphs in this font" from
`kRSUnicode`. Picking 木 twice built `[uni6728][uni6728]`; with ⿰ in front,
**Preview IDS** reported "2 components will be appended" and **Append
components** added them as one entry labelled "Construct CJK components".
([cjk-construction.png](screenshots/p5f-l-cjk/cjk-construction.png)) The
transform grid then showed the two components at x 0 and x 500 with scale
0.5/1.0. Editing Y to 20 and Scale X to 0.52 on the first and X to 480 on the
second added **no** history entry while the preview followed each change;
**Apply transforms** wrote all three as one entry labelled "Transform CJK
components". ([transform-grid.png](screenshots/p5f-l-cjk/transform-grid.png))
The Font components tab, empty before, then listed
`U+6728 · uni6728 · 2 component uses`.

Two environment notes, neither a product defect. The dev server was reloaded
several times by other agents' `pnpm gen:api` runs and by hot updates to
`GlyphCanvas.tsx`; the steps above were re-run after each reload against the
same document. The interface language was switched to Chinese by the concurrent
localization pass, and was switched back to English through View → English
Interface for this session. The twelve description operators render as tofu in
the headless container, which has no font covering U+2FF0–U+2FFB.

## Independent verification of the emitted fonts

The document was saved as `.fcf` and reopened through the kernel. The reopened
document still held vertical origin 611 and advance 1187 on 林, both components
with the applied transforms `[0.52, 0, 0, 1, 0, 20]` and `[0.5, 0, 0, 1, 480,
0]`, and the CID map with the edited `65000 → uni4EBA`. It was then exported.

FontTools on the exported OTF:

```text
tables   BASE CFF  GPOS GSUB OS/2 VORG cmap head hhea hmtx maxp name post vhea vmtx
林 = cid20926   vmtx (advance 1187, tsb -248)        # 611 - ymax 859 = -248
VORG  defaultVertOriginY 880   records 1   { cid20926: 611 }
vhea  advanceHeightMax 1187  minTopSideBearing -248  minBottomSideBearing 8
      yMaxExtent 992
CFF   ROS ('Adobe', 'Identity', 0)   CIDCount 65001   FDArray 3
      FD 0 LanguageGroup 1 StdVW 40 StdHW 40
      FD 1 LanguageGroup 1 StdVW 63 StdHW 58
      FD 2 LanguageGroup 0 StdVW 85 StdHW 69 BlueValues [-13,0,543,557,733,747]
      FDSelect gid 0..5 → [0, 2, 2, 2, 2, 1]
      charset   .notdef cid00001 cid00034 cid00055 cid00080 cid65000
      人 gid 5 → charset cid65000, FontDICT 1
```

So the edited vertical advance and origin reach `vmtx`, `VORG` carries exactly
the one glyph that differs from the default, the edited CID reaches the charset,
and a real CID-keyed font round-trips through FontCarver with three FontDICTs
and a working FDSelect.

FreeType and HarfBuzz on the same file:

```text
FreeType  units/em 1000, has_vertical True
          林 vertAdvance 1187   人 1000   木 1000   (horiAdvance 1000)
          林 rendered at 48 ppem: 47×47, 1005 inked pixels
HarfBuzz  ltr  [(21, 1000, 0), (5, 1000, 0), (20, 1000, 0)]
          ttb  [(21, 0, -1187), (5, 0, -1000), (20, 0, -1000)]
          get_glyph_v_origin  林 (500, 611)   人 (500, 880)   木 (500, 880)
```

HarfBuzz reads the vertical origin as 611 for the edited glyph and 880 — the
`VORG` default — for the rest, which is the row's requirement stated from the
consumer's side.

Row 112's export option was verified on a component-only glyph, built through
the API by clearing 休's contours and adding 亻 and 木 as components:

```text
Keep.ttf         休 composite  [(uni4EBB, 0, 0), (uni6728, 300, 0)]
Decomposed.ttf   休 simple     contours=2        # decompose_components: true
```

## Remaining limits

1. **No bundled ideographic-description database.** The pinned Unicode 17.0
   Unihan subset in `crates/fc-glyphdata/data` carries `kRSUnicode` and
   `kTotalStrokes` but no IDS property, and adding one would mean a new licensed
   data file, which is outside this task. A sequence is therefore typed, or
   assembled by picking from the radical library; the panel does not propose a
   decomposition for a character. The layouts the twelve operators produce are
   geometric starting positions for manual refinement, not optical
   decomposition — the boundary the earlier CJK construction record already
   stated.
2. **The library reports each form's own stroke count, not a radical stroke
   table.** 水 is 4 strokes and 氵 is 3 in the pinned source, and that is what
   the entries show. A canonical "radical _n_ has _k_ strokes" column would be
   data the shipped source does not contain, so it is not claimed.
3. **Vertical writing mode is an Edit View canvas mode.** It makes the vertical
   metrics primary and draws the vertical advance box; it does not stack
   neighbouring glyphs vertically. Vertical text layout belongs to the proofing
   surfaces, which are task `h`'s rows.
4. **The CID mapping is edited as text.** Registry, ordering, supplement and
   `CID<tab>glyph` lines, with Identity auto-fill, the numeric ordering switch
   and now the unmapped-glyph count. There is no per-glyph CID cell in the glyph
   grid; the grid's column set lives in task `b`'s files and was not touched.
5. **A locked component refuses the whole transform batch.** The grid sends one
   `SetComponent` per changed row inside one `Batch`, and the kernel's component
   lock refuses the batch rather than applying the unlocked rows; the staged
   numbers are kept so the edit can be retried after unlocking. Pinned by
   `a_locked_component_refuses_the_whole_transform_batch`.
6. **`FT_FACE_FLAG_CID_KEYED` is not set** for a CID-keyed CFF inside OpenType;
   FreeType reserves that flag for bare CID-keyed fonts. The ROS, FDArray,
   FDSelect and charset evidence above is FontTools'.
7. **A glyph that mixes contours and components is always flattened** by the
   TrueType compiler, because TrueType has no mixed glyph. The decompose option
   is therefore observable on component-only glyphs, which is what the check
   above uses.
8. **The canvas's vertical handles need vertical writing mode.** They are drawn
   whenever the font has vertical metrics but take a press only in that mode,
   because a metric line runs the whole width of the canvas and outranks
   segments and components. The inspector's fields are always available.
9. **No macOS or Windows execution**, and no native or global desktop input
   automation was used anywhere in this task.

## Files

Owned: `crates/fc-glyphdata/src/{unihan.rs,lib.rs}` (Kangxi radical derivation),
`crates/fc-api/src/handlers/cjk.rs`, `crates/fc-api/src/types/cjk.rs`,
`crates/fc-api/tests/cjk.rs`, `crates/fc-doc/tests/{vertical.rs,cjk_construction.rs}`,
`app/src/views/glyph/panels/{CjkRadicalLibrary.tsx,CjkRadicalLibrary.test.tsx,cjkRadicalEntries.ts,cjkRadicalEntries.test.ts,CjkTransformGrid.tsx,CjkTransformGrid.test.tsx,cjkTransforms.ts,cjkTransforms.test.ts,CjkConstructionPanel.tsx,CjkConstructionPanel.test.tsx}`,
`app/src/views/fontinfo/{CidEditor.tsx,cidSession.ts,cidSession.test.ts}`,
`docs/user/variable-and-cjk.md`, this document and the screenshots it cites.

Shared files the task file allows, patched additively:
`app/src/views/glyph/{metricsHandles.ts,metricsHandles.test.ts,drawMetrics.ts,GlyphCanvas.tsx}`,
`app/src/i18n/en.json`, `crates/fc-api/src/api.rs`,
`crates/fc-api/src/bin/gen_ts.rs`, `crates/fc-compile/tests/{cid.rs,otf_vertical.rs}`.

Files outside both lists that the canvas handles and the new overlay required,
none of them owned by another wave-2 task, all additive:
`app/src/editor/asideGestures.ts` (route the font point, pick the cursor and the
undo label per handle, pass the stored overrides),
`app/src/editor/editorFrame.ts` (widen the previewed `aside.metrics`),
`app/src/editor/editorTypes.ts` (two more HUD handles, one more undo label,
`EditorSource.verticalOverrides`), `app/src/views/glyph/useEditorLabels.ts`
(translate them), `app/src/views/glyph/GlyphView.tsx` (publish the stored
overrides), `app/src/views/glyph/renderGlyph.ts` and
`app/src/views/glyph/glyphCommands.ts` (the `verticalMode` overlay and its
command), `app/src/editor/editorController.test.ts` and
`app/src/stores/editorStore.test.ts` and
`app/src/views/glyph/drawCollisionFixes.test.ts` (one line each for the new
label, the new overlay key and the vertical-mode drawing case).

New English key namespaces: `cmd.view.glyph.verticalMode`, `cid.unmapped*`, and
`cjkConstruction.{componentsHint,librarySource,tabs.*,radicalLabel,strokesLabel*,membersLabel*,usesLabel*,presentOnly,noComponentBases,transformPreview,transformHint,applyTransforms,revertTransforms,undoTransforms,errors.transformInvalid,errors.transformCollapsed}`.
No key was removed. No new dependency, no new data file, no FontForge code.

## Recommended status

| Row | Recommendation                                                                                                                                                                                                                                                                                                                                                         |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 19  | **Complete**: TSB/BSB, origin and advance edited from resolved ink in the inspector, one history entry per field, range/lock/read-only refusals pinned by tests, `.fcf` round trip, and the edited values decoded out of `vmtx`/`VORG`/`vhea` by FontTools, FreeType and HarfBuzz.                                                                                     |
| 110 | **Complete** within limit 4: mapping editor with identity fill, numeric ordering and an unmapped-glyph count; CID sort verified live against an edited mapping; CID-keyed CFF export decoded by FontTools with ROS, charset CIDs, a three-entry FDArray and a working FDSelect, plus a synthetic two-FD regression.                                                    |
| 111 | **Complete for the Edit View and `VORG`**: a vertical writing mode, vertical origin and advance handles that are grabbable only in that mode, with the right arithmetic, inheritance preserved, Escape and one-entry undo, and `VORG` proven to record only the origins that differ from the default. The proofing surfaces are task `h`'s half of this row.           |
| 112 | **Complete** within limit 1: a radical library derived from the shipped Unihan data and from the font's own components, IDS input resolving to a component layout applied in one history entry, and a staged position/scale transform grid with a live preview applied in one history entry; export keeps or decomposes components per option, verified in the binary. |

## Review round

The wave-2 review returned **accept with fixes**. Each item below names what
changed and the test that holds it.

1. **The vertical handles were live outside vertical mode (medium, row 111).**
   `metricsHit` offered `vorigin`/`vadvance` whenever the metrics carried a
   vertical origin, which in a CJK font with a master default is every glyph; a
   metric line outranks segments and components, so ordinary horizontal editing
   had two full-width 4 px bands in front of the outline. `metricsHit` now takes
   a `verticalMode` flag that defaults to off, `EditorController.setVerticalMode`
   publishes the overlay the way `setRulersVisible` already does,
   `AsideContext.verticalMode` carries it into the hit test, and `GlyphCanvas`
   sets it from `overlays.verticalMode`. Held by
   `metricsHandles.test.ts` ("offers the horizontal lines only in vertical
   writing mode": the same points hit in the mode, miss out of it, and the
   sidebearing handles are unaffected either way) and by
   `editorController.test.ts` ("offers the vertical metric handles only in
   vertical writing mode": the identical press-and-drag sends nothing out of the
   mode and one `SetVerticalMetrics` in it). The user doc's "in or out of
   vertical mode" sentence is replaced with the real rule and the reason.

2. **The unmapped-glyph warning counted non-exporting glyphs (low-medium, row
   110).** `otf::compile` orders `.notdef` plus the glyphs with `export`, so a
   font with non-exporting helpers carried a permanent false warning.
   `CidState` gained an `exported` list and `unmappedGlyphs` filters on it.
   Held by `cidSession.test.ts` (a glyph outside `exported` is not reported) and
   by `fc-api/tests/cjk.rs`
   (`cid_state_separates_the_exported_glyphs_from_the_whole_order`: the two
   lists agree until a glyph stops exporting, after which it stays in `glyphs`
   and leaves `exported`, order preserved). The acceptance wording above and the
   user doc now say "every glyph it orders" rather than "every exported glyph".

3. **The radical library refetched per document event (low).** `cjk.radicals`
   scans every layer for component bases, so a continuous gesture with the
   section open paid for that scan per frame. The refetch now waits the same
   100 ms the outline request uses. Held by `CjkRadicalLibrary.test.tsx`
   ("refetches on a document event, and collapses a burst into one scan": eight
   events produce one rebuild, and a settle window proves no late second one).

4. **The vertical guard test compared the model, not bytes (low).**
   `vertical_metric_guards_hold_and_edited_bearings_survive_fcf` now asserts,
   inside the out-of-range loop, that the serialized font is byte-identical and
   that `undo_len()` is unchanged, as well as the model equality it had.

5. **Nits.** `cjkConstruction.libraryHint` and `componentsHint` now say that a
   pick appends the glyph name when the font has the glyph and the character
   when it does not; `cjkConstruction.search` is "Search radicals and
   components". The tab strip follows the full ARIA pattern — `aria-controls`
   on each tab, a `role="tabpanel"` with `aria-labelledby`, one tab stop with
   roving `tabIndex`, and Left/Right/Home/End — held by a new case in
   `CjkRadicalLibrary.test.tsx`. The dead `radicalCodepoints()` and its test are
   removed. `app/e2e/cjk.spec.ts` gains two cases that need no CJK corpus and
   run against the committed DejaVu document the default configuration opens:
   one edits the vertical metrics, checks that TSB moves the origin by the same
   amount with the advance untouched, then proves the canvas gesture does
   nothing out of vertical mode and moves the origin by the dragged distance in
   it, with one "Undo Set vertical metrics" entry; the other opens the radical
   library on a font with no ideographs, where it still lists the 525 pinned
   entries, searches radical 85 down to 水/氵/氺, and walks the tab strip with the
   arrow keys.

6. **`commandHelp`.** Task m had already added
   `'view.glyph.verticalMode': { topic: 'glyph', summary:
'quickHelp.view.glyph.verticalMode' }`, so no entry was missing;
   `commandHelp.test.ts` passes 8/8. `view.glyph.verticalMode` is the only
   command this task registered.

**A defect the new e2e case found and fixed.** Writing the spec exposed a race
in `VerticalMetricsPanel`: `SetVerticalMetrics` writes both fields, and each
field read the _other_ one from the `glyph` prop, which still holds the record
from before the previous command's reload. Typing a vertical advance and then an
origin therefore sent the stale advance and silently reverted the first edit —
the e2e run saw the advance fall back to the 2048 upem default. The panel now
remembers the override pair it last sent and starts the next command from it
until the model reports that pair back (or the glyph or layer changes). The
existing unit case
`retains the other override when changing advance or origin directly` had
pinned the _old_ behaviour; it is now
`carries the other override through, including one still in flight` and asserts
the advance the user set, across three consecutive edits.

### Review round commands

```text
cargo test -p fc-glyphdata                            31 passed, 0 failed
cargo test -p fc-api --test cjk                        8 passed, 0 failed
cargo test -p fc-doc --test vertical                   3 passed, 0 failed
cargo test -p fc-doc --test cjk_construction           7 passed, 0 failed
cargo test -p fc-doc --test cid                        1 passed, 0 failed
cargo test -p fc-compile --test cid                    3 passed, 0 failed
cargo test -p fc-compile --test otf_vertical          10 passed, 0 failed
  → 63 passed, 0 failed

pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 <19 files>
Test Files  19 passed (19)
     Tests  263 passed (263)

FC_API_PORT=8812 FC_UI_PORT=1512 pnpm --dir app exec playwright test e2e/cjk.spec.ts
  2 passed, 1 skipped          (the Source Han Sans case skips without a
                                release kernel and the external corpus)

cargo clippy -p fc-glyphdata --all-targets -- -D warnings   clean
cargo clippy -p fc-api --lib --test cjk                     clean
cargo clippy -p fc-doc --test vertical --test cjk_construction  clean
rustfmt --edition 2021 --check <every Rust file touched>    clean
pnpm --dir app exec eslint <every file touched>             clean (exit 0)
pnpm --dir app exec prettier --check <same files>           clean
pnpm --dir app check                                        clean (exit 0)
```

The review round changed one more file outside the task's lists:
`app/src/editor/editorController.ts` (the `verticalMode` flag, its setter and
the `AsideContext` entry — the same three-line shape `rulersVisible` already
has), plus the new case in `app/src/editor/editorController.test.ts`.
`cid.state` gaining a field required `pnpm gen:api` under the cargo flock;
`app/src/api/generated/CidState.ts` is the only regenerated file that changed.
No English key was removed, and three were reworded —
`cjkConstruction.libraryHint`, `cjkConstruction.componentsHint` and
`cjkConstruction.search` — so the translation pass needs to revisit those three
zh-CN entries as well as the keys this task added.
