# P1 Task 8 — Inspector panels, Edit-tab sidebar, metrics editing, Path/Glyph/Layer menus

Branch `p1-t8` in `.worktrees/p1-t8`, base `449a390`. Ten commits, nothing pushed.
Ports used: kernel 8793, Vite 1426.

## Commits

| SHA | Subject |
| --- | --- |
| `49bf98e` | `feat(app): command builders for the editor panels and the Path menu` |
| `a3d0d32` | `feat(app): the Edit tab's inspector panels` |
| `eb4b15f` | `feat(app): background and other-master underlays behind the edited layer` |
| `2583976` | `feat(app): the Edit tab's sidebar — layers over a compact glyph list` |
| `7f04e29` | `feat(app): draggable sidebearing handles and guides pulled off the rulers` |
| `c1a5cd9` | `feat(app): the canvas markers deferred from Task 6` |
| `628fd6e` | `feat(app): the Path, Glyph and Layer menus` |
| `e30aa3b` | `test(e2e): cover the editor panels, the Path menu and the Edit sidebar` |
| `25630d3` | `fix(app): five defects the interactive pass found, and the screenshots` |
| `e1e2c6d` | `refactor(app): split the sidebearing and ruler gestures out of the controller` |

The brief's single `feat(app): editor panels, metrics editing and path operations` was split
across seven feature commits so each one builds and tests on its own, as CONVENTIONS asks.

## What was built

### The inspector (`app/src/views/glyph/panels/`)

`EditInspector.tsx:30` renders the seven sections of design spec §7 in that order, from the one
record the mounted glyph view publishes (`stores/editorStore.ts:23` — `ActiveGlyph` now carries
the editable layer, its master, that master's guides and the view's own `reload`). No panel
issues a request for data the canvas already has, and no panel keeps a copy of document state.

`PanelSection.tsx:25` is the accordion row: a count badge where a count means something, and the
open/closed state persisted per section in `uiStore` (`stores/uiStore.ts:93`, `:313`) beside the
panel sizes P0 already persists.

| Panel | File | What each control emits |
| --- | --- | --- |
| Glyph §7.1 | `GlyphPanel.tsx` (219) | name → `RenameGlyph`; Unicode chips and the add field → `SetUnicodes`; export toggle, the twelve colour labels, tags and note → `SetGlyphMeta`. Category, sub-category and script are read-only, from `Glyph` falling back to the `GlyphCell` the kernel derives through fw-glyphdata. |
| Metrics §7.2 | `MetricsPanel.tsx` (173) | width, LSB and RSB → `SetMetrics`; the three formula fields → `SetMetricsKeys`; the live resolved triple from `doc.metrics_resolve` (`useResolvedMetrics.ts`), an "out of sync" badge when it differs from the stored sidebearings, and "Apply resolved" → one `SetMetrics` with the resolved values. |
| Layers §7.3 | `LayersPanel.tsx` (165), `LayerList.tsx`, `layerRows.ts` | add background → `AddLayer`; copy to background and add backup → `CopyLayer`; swap → `SwapWithBackground`; delete → `RemoveLayer` (never offered for a master layer, which the kernel refuses). Rows are grouped masters → intermediate → backgrounds → backups, newest first. Selecting a row puts that layer on the canvas; the two eye toggles drive the new overlays. |
| Components §7.4 | `ComponentsPanel.tsx` (158), `ComponentRow.tsx` (214), `GlyphPicker.tsx` (128) | offset, scale, auto-align and use-my-metrics → `SetComponent`; the search popover (`glyphs.search`, with a thumbnail per hit) → `AddComponent`; decompose → `DecomposeComponents`; delete → `RemoveComponents`; reorder by drag or the ↑↓ buttons → one `Batch` of `SetComponent`, one write per row that actually moved (`panelCommands.ts:287`). "Open base" resolves the name through `glyphs.search` and opens its Edit tab (`openBase.ts`). |
| Anchors §7.5 | `AnchorsPanel.tsx` (158) | add → `AddAnchor` with the first free conventional name; name, X and Y → `SetAnchor`; delete → `RemoveAnchors`. Clicking a row selects the anchor on the canvas. |
| Guides §7.6 | `GuidesPanel.tsx` (91), `GuideRows.tsx` (147) | add → `AddGuide` in the layer's scope or the master's; position, angle, name and lock → `SetGuide`; delete → `RemoveGuide`; the measurement toggle is session state (see deviation 3). |
| Transform §7.7 | `TransformPanel.tsx` (254), `transformMath.ts` (264), `transformTargets.ts` (139) | Apply composes one affine about the nine-point origin and sends one `TransformSelection`; Flip H/V are the same command with a mirroring affine; align and distribute compute their deltas locally and commit one `Batch` of `MovePoints` (points, grouped per contour) and `TransformSelection` (anchors, components). Nothing selected transforms the whole layer. |

### Metrics editing on the canvas

`views/glyph/metricsHandles.ts` (193) hit-tests the metric verticals at the design's 4 px radius
and runs the drag through the ordinary `edit_begin`/`edit_apply`/`edit_end` protocol, so a whole
drag is one undo entry. `SetMetrics` is absolute, so each frame sends the *value*, not a delta —
resending the same value is a no-op, which is why this gesture needs none of the delta
bookkeeping `MovePoints` does. The canvas draws the metrics the drag is making
(`editorFrame.ts:39`, `GlyphCanvas.tsx:150`) and the HUD reads the handle and its value.

### The Edit-tab sidebar (§3, deferred from Task 6)

`EditSidebar.tsx` (80) puts the Layers panel's own list over `EditGlyphList.tsx` (119) — the Font
view's grid at a 36 px cell, under the Font view's current filter, mounted under a key derived
from that query. Clicking a cell replaces the glyph of *this* tab rather than opening another;
the list scrolls to whichever glyph the tab shows.

### The Path, Glyph and Layer menus (§11)

`views/glyph/pathCommands.ts` (294) registers the Path menu, `pathOps.ts` (187) builds its
commands. Remove Overlap (`Mod+⇧+O`), Correct Direction (`Mod+⇧+R`), Add Extremes (`Mod+⇧+E`),
Add Inflections, Round to Grid (the grid size from the preferences), Convert to Cubic/Quadratic,
Simplify… (`Mod+⇧+M`) and Expand Stroke… go through `PathOp`; Tidy (`Mod+⇧+T`) is
`CanonicalStart` and `CanonicalOrder` in one `Batch`; Reverse, Set Start Point, Join (`Mod+J`)
and Break take their targets from the canvas selection, and Join orders its pair the way
`JoinContours` requires. `layerCommands.ts` (223) mirrors the Layers panel in the Layer menu and
adds Duplicate and Decompose All to the Glyph menu. Simplify and Expand Stroke get small dialogs
(`dialogs/SimplifyDialog.tsx`, `dialogs/ExpandStrokeDialog.tsx`) for the options the kernel takes.

### The canvas markers deferred from Task 6 (§4.1)

- A node sitting on a vertical metric wears a 7 px diamond in `--cv-zone` (`drawOutline.ts:343`).
- A hovered locked component is hatched (`drawComponents.ts:126`); "locked" is `auto_align`, see
  deviation 4. The origin marker and the collision-avoided name label were already in place.
- A selected guide carries its angle knob and a measurement guide labels the distance between
  successive crossings of the outline (`drawGuides.ts:76`, `guideMeasure.ts`).
- Design spec §4.1 items 4 and 5 came with the Layers panel's eye toggles: `drawUnderlays.ts`
  strokes the other masters with no nodes and the background layer with 3 px dots, both from the
  flattened paths `glyphs.detail` already returns, so an underlay costs no extra request.

## Tests

`pnpm --dir app test` → **77 files, 699 tests**, up from 674 at the base. New:

- `views/glyph/panels/panelCommands.test.ts` (15) — the `SetGlyphMeta` optionality (an absent
  `color_label` key leaves the label, an explicit `null` clears it), duplicate naming, metrics-key
  normalisation, layer construction, component reordering as one batch of only the moved rows,
  anchor-name suggestion, guide scoping.
- `views/glyph/panels/transformMath.test.ts` (14) — the affine about each of the nine origins,
  rotation sense in y-up space, flip, shear, translation order, composition order, and align /
  distribute including "three items or nothing" and ordering by position.
- `views/glyph/panels/EditInspector.test.tsx` (24) — one case per panel control asserting the exact
  `doc.apply` through the mock transport, plus the accordion order, the persisted open state, the
  count badges and a failing command reaching the error reporter.
- `views/glyph/pathOps.test.ts` (10) and `views/glyph/pathCommands.test.ts` (20) — the Path menu
  wiring: every whole-layer op as one `PathOp`, Tidy as one batch, the selection-scoped four with
  their gates, the design-spec shortcuts through `commandForEvent`, and the Layer/Glyph items.
- `views/glyph/metricsHandles.test.ts` (13) — handle positions and the grab radius, the RSB's
  opposite sense, rounding (⇧ = ten units), the protocol (`edit_begin`, one apply per changed
  value, `edit_end`), the preview, a gesture that never moved, cancellation, and the ruler-guide
  commands in both scopes.
- `views/glyph/markers.test.ts` (13) — the metric diamond, the locked hatch, the angle knob, the
  measurement labels, and the crossing geometry (stem width, counter, a slanted guide, a vertex
  the guide only touches, a guide lying along an edge).
- `views/glyph/EditSidebar.test.tsx` (3), `editor/editorController.test.ts` +6 (the sidebearing
  drag, a node outranking a metric line, ruler guides in both scopes, rulers hidden, and the
  anchor-only drag ruling 6 asked about), `components/NumberField.test.tsx` (3),
  `views/font/GlyphCell.test.tsx` +1, `views/glyph/drawLayers.test.ts` +3.

## Verification

```
$ pnpm --dir app gen:api                       # exit 0
$ pnpm --dir app check                         # tsc -b, silent
$ pnpm --dir app test
  Test Files  77 passed (77)
       Tests  699 passed (699)
$ pnpm --dir app lint                          # eslint, silent
$ pnpm --dir app exec prettier --check .
  All matched files use Prettier code style!
$ FW_API_PORT=8793 FW_UI_PORT=1426 pnpm --dir app e2e
  1 skipped
  15 passed (12.7s)                            # 13 pre-existing + 2 new
```

`app/e2e/panels.spec.ts` is the new spec the dispatch asks for. It opens DejaVu `A`, sets the LSB
from the Metrics panel (checking the Info box agrees without a reload), runs Remove Overlap from
the Path menu (checking the Edit menu now offers to undo it), adds an anchor, and undoes three
times — the anchor row goes, the outline operation goes, and the LSB and advance are back at the
values the font shipped, with the Edit menu reporting nothing left to undo. A second case switches
glyph from the compact list and layer from the layer list, and undoes the added background.

**The suite is load-sensitive** — see concern 1.

## Interactive pass (agent-browser, ports 8793 / 1426, both stopped afterwards)

Kernel `fw-devserver --port 8793 --open fixtures/fonts/DejaVuSans.ttf` under a scratch
`FW_CONFIG_DIR` so nothing reached the developer's own configuration; UI `vite --port 1426` with
`FW_DEV_PROXY`. The document was left clean (every exploratory edit undone, `dirty false`,
`undo_label null`) and the one autosave snapshot this session produced under the real config
directory — from before the kernel was moved to the scratch one — was discarded.

Log: opened `A` from the grid; walked every panel; added and removed background and backup
layers; opened `Aacute` and exercised the Components panel on its two components; added two
anchors and moved one with its panel fields; added local and master guides and marked one for
measurement; selected all 11 points for the Transform panel; and dragged the RSB handle on the
canvas. Five defects came out of it and are fixed in `25630d3` — see below.

| File | Caption |
| --- | --- |
| `docs/screenshots/p1-edit-sidebar.png` | The Edit tab of DejaVu `A`: the sidebar's layer list over the compact glyph list at 36 px under the Font view's filter, the seven inspector sections open on the right, and the docked Info box. |
| `docs/screenshots/p1-panels-layers.png` | The Layers section with the master layer, a background copied from it and a timestamped backup, the four actions and the two overlay toggles; the sidebar's list shows the same three rows. |
| `docs/screenshots/p1-panels-components.png` | `Aacute`'s two components with the selected row expanded onto its own line — offset, scale and the two alignment flags — and the Info box on the same component. |
| `docs/screenshots/p1-panels-anchors.png` | Two anchors on `Aacute`, the selected one expanded with its name and position, the suggestion picker offering the next free conventional name, and both anchors drawn on the canvas. |
| `docs/screenshots/p1-panels-guides.png` | A master guide across `A` at y = 700, selected (its angle knob at the left end) and marked as a measurement guide: the canvas labels the crossings **198 · 439 · 197** — left stem, counter, right stem. |
| `docs/screenshots/p1-panels-transform.png` | The Transform section with all 11 points selected: move, scale, rotate and skew, the proportional lock, the nine-point origin, Apply and the flips, and the align/distribute row. |
| `docs/screenshots/p1-metrics-handles.png` | Mid-drag of the RSB handle on the canvas: the HUD reads `RSB 136`, the advance line has followed it, the Metrics panel holds the layer's own numbers and the status bar flashes `Set metrics`. |

### Defects the interactive pass found (all fixed in `25630d3`)

1. **A grid cell whose page was invalidated stayed blank for good.** The detaching canvas's ref
   cleanup could run after the replacement had attached and null the handle the paint effect
   reads, leaving a 0 × 0 backing store. `views/font/GlyphCell.tsx:65` now counts attachments, so
   the paint always runs after a node exists, and a cleanup only clears its own handle. It
   reproduces in Chrome and not in jsdom, so the regression test documents the behaviour rather
   than the ordering (concern 4).
2. **One numeric field edit produced two undo entries.** Enter commits, and the blur that follows
   commits again against a `value` the round trip has not yet updated.
   `components/NumberField.tsx:44` remembers what it asked for until the model catches up; the
   test fails without the guard.
3. **The compact glyph list kept the previous filter's page cache**, because only the Font view
   remounted its grid on a new query. `EditSidebar.tsx:42` keys the list the same way.
4. **The expanded fields of a component, anchor or guide row overflowed their row** instead of
   taking a line of their own (`panels.module.css`).
5. **A sidebearing drag flashed "out of sync" for its whole length**, because it applies once per
   frame while the fetched layer deliberately lags. `useResolvedMetrics.ts:71` defers, the way
   every other reader of document events already does.

## Deviations and rulings applied

1. **`PathOp` is a whole-layer operation.** Ruling 9 says "on the selected contours (or all)", but
   `fw-doc` hands the *layer's* contours to `fw-geom` and takes the result back
   (`crates/fw-doc/src/command/outline/segments.rs:298`). Overlap removal, extrema, simplification,
   rounding, conversion and stroke expansion therefore always act on the layer; only Reverse, Set
   Start Point, Join and Break are selection-scoped, and `pathOps.ts:108` is where the selection
   becomes their targets.
2. **"Global" guides are the master's, not `SetFontGuides`.** The model has three guide lists —
   layer, master and font-wide (`crates/fw-doc/src/command/structure.rs:184`). The canvas draws
   layer and master guides and paints the master ones in the global colour, so the panel's two
   lists are those two. `SetFontGuides` replaces a third list the editor never draws; exposing it
   would create guides the user cannot see.
3. **The measurement flag is session-only.** `fw_model::Guide` is position, angle, name, lock and
   colour — there is no measurement field, and this task makes no kernel change, so the marks live
   in `editorStore.measurementGuides` and are deliberately not persisted.
4. **"Locked component" reads as `auto_align`.** `Component` has no lock; an automatically aligned
   component takes its position from the base glyph's anchors and so cannot be dragged, which is
   what §4.1 item 6's hatch is warning about (`drawComponents.ts:91`).
5. **The background and other-master overlay flags did not exist.** Ruling 4 says to reuse them;
   `GlyphOverlays` had six flags and no drawing for either layer. Both were added, along with
   `drawUnderlays.ts` and two View-menu commands.
6. **Vertical width and origin are read-only** in the Metrics panel: `SetMetrics` carries width,
   LSB and RSB only, and no command writes `Layer.vertical_width` / `vertical_origin`.
7. **A `metrics` hit is a separate probe, not a member of `EditTarget`.** Widening the selection
   union would have rippled into the Info box, the marquee and every count for a target that is
   never *selected*, only dragged. `editorController.ts:288` runs the probe after the ordinary hit
   test and lets nodes, handles, anchors and guides outrank it, with segments and components
   behind it (§4.2 precedence).
8. **`metricsHandles.ts` and `rulerGuides.ts` live under `views/glyph/`** as the brief's file list
   says, even though they are plain gesture code; `editor/editorTypes.ts` already imports from
   that directory, so the direction is not new.
9. **The P0 `GlyphInspector` is gone**, replaced by these panels — Task 6's report item 7 flagged
   it as this task's subject. The "Bounds" row the glyph-view e2e spec asserts on moved to the
   Metrics panel.
10. **Ruling 6's anchor check needed no fix.** `editorController.test.ts:205` drives an
    anchor-only drag through the controller and asserts the committed
    `TransformSelection { anchors: [0], affine: [1,0,0,1,30,0] }` under the label `Move anchor`.

## New i18n keys (161) — for Task 13 to translate

`zh-CN.json` carries every one with the English text as a placeholder; nothing was translated here.

`panel.glyph.*` (4) · `panel.metrics.*` (4) · `panel.layers.*` (7) · `panel.components.*` (8) ·
`panel.anchors.*` (4) · `panel.guides.*` (11) · `panel.transform.*` (10 + `align.*` 8) ·
`panel.sidebar.*` (3) · `editor.undo.*` (43 + `align.*` 8) · `cmd.path.*` (14) · `cmd.layer.*` (4) ·
`cmd.glyph.{duplicate,decomposeAll}` · `cmd.view.glyph.{background,otherMasters}` ·
`cmd.reason.{noGlyphLayer,noStartPoint,noJoin,noBreak}` · `dialog.simplify.*` (10) ·
`dialog.expandStroke.*` (16) · `a11y.{inspector,sidebar}` · `sidebar.glyphTitle`.

## Concerns

1. **The e2e suite is flaky under parallel workers on a loaded machine.** All specs share one
   kernel *and* one session, so a document another worker's spec creates appears in every other
   spec's tab bar and can become the active tab; the failures are always "the grid had not loaded"
   or a 30 s test timeout, and they rotate across `fontview`, `glyphview` and `panels`. Measured on
   this box (20 cores, load 10–38 from other worktrees' builds): `--workers=1` is green every time
   (15 passed, 1 skipped, 40 s); the default parallel run was green at load 10 and 12 and failed
   1–4 specs at load 16–32. **It reproduces with my changes stashed**, so it predates this task —
   it is the harness Task 11 built, and the fix is one kernel per worker (or `fullyParallel: false`),
   which is that task's file. I added the wait `glyphview.spec.ts:35` was missing and which its own
   sibling already had.
2. **`editorController.ts` is 703 lines** after the split — inside the dispatch's 800 but above
   CONVENTIONS' ~400 guideline, as Task 6 already recorded. `EditInspector.test.tsx` is 643 lines
   of test.
3. **The Components panel's drag reorder has no keyboard equivalent beyond the ↑↓ buttons**, and
   the drag uses the HTML5 drag events rather than the pointer protocol the canvas uses. It is the
   one place in this task where the interaction is not the editor's own.
4. **The `GlyphCell` regression test does not fail without its fix.** The ref-cleanup ordering that
   caused it happens in Chrome and not in jsdom; the test pins the behaviour ("a cell that came
   back is painted"), and the evidence for the bug and the fix is the interactive pass.
5. **The measurement guide re-measures on every paint.** `guideMeasurements` flattens the outline
   each frame it is drawn; on a 300-point glyph with a measurement guide that is real per-frame
   work, unmemoised. Nothing measurable came out of the interactive pass, but it is the obvious
   thing to cache next.
6. **`doc.metrics_resolve` is issued once per glyph, master and formula change.** A font where every
   glyph carries a metrics key would make that one request per glyph opened; it is not batched.

Report written 2026-09-05.
