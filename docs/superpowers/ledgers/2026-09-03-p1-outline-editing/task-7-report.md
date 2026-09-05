# P1 Task 7 — Drawing and transform tools: report

Branch `p1-t7` in `.worktrees/p1-t7`, base `b811ed2`. Not pushed.

## Commits

| SHA | Subject |
| --- | --- |
| `d66ae05` | refactor(app): split the drag gesture by responsibility |
| `327e989` | fix(app): gate every gesture close, whichever gesture closed it |
| `b7fc879` | feat(app): draw each component from its own path so a preview moves it |
| `cfb9739` | feat(app): the pen tool, path geometry and the custom SVG cursors |
| `b1962f7` | feat(app): drawing and transform tools with tap/hold switching and snapping |
| `fb64182` | test(app): cover every tool state machine, tap/hold and the snap order |
| `343e534` | test(e2e): pen-draw a closed triangle into a new font |
| `cbb16b1` | feat(app): pref-driven grid, component ghost nodes and the tool cursors |

## What was built

### The tool contract

`app/src/editor/tools/toolTypes.ts` (206) defines what every tool is: a state
machine taking `ToolPointer` / `ToolKey` plus a `ToolContext` and returning a
`ToolOutcome` (a command to apply, a preview, a dialog request, a closing
promise) and a `ToolOverlay` (contours, lines, marks, span labels, a transform
origin, a HUD line). Nothing in it touches the kernel, React or the DOM, so
every grammar below is driven from plain objects in its test.

`app/src/editor/toolHost.ts` (248) builds the tool for a `ToolId`
(`createTool`, `:44`), assembles the context fresh for every event (`:115`),
converts a `PointerInput` into the two spaces a tool works in (`:137`) and
turns outcomes into the editor's own vocabulary (`:236`). The controller only
routes: `editorController.ts:310` (pointer-down), `:351` (move), `:376` (up),
`:417` (`toolKey`), `:107` (where the host is built).

### Tools

| Tool | Key | File | What one gesture emits |
| --- | --- | --- | --- |
| Pen | P | `tools/pen.ts` (352), `tools/penPath.ts` (146) | one `AddContour` on close/finish (`pen.ts:274`), or `Batch([DeleteContours, AddContour])` labelled "Extend path" when an open contour's end node was picked up (`pen.ts:84`, `:311`) |
| Pencil | B | `tools/pencil.ts` (191) | `Batch([AddContour(polyline), PathOp::Simplify])` labelled "Draw" (`pencil.ts:178`) |
| Knife | E | `tools/knife.ts` (120), `cutPath.ts` (137) | `Batch([DeleteContours, AddContour…])` labelled "Cut" (`knife.ts:66`, `:101`) |
| Rectangle / Ellipse | F / ⇧F | `tools/shapes.ts` (152), `shapeContours.ts` (122) | one `AddContour` (`shapes.ts:136`), or a `ShapeDialogRequest` for a click that never dragged (`:102`) |
| Measure | L | `tools/measure.ts` (130) | nothing at all; overlay and HUD only |
| Rotate / Scale | R / S | `tools/transformTool.ts` (257), `rotate.ts` (42), `scale.ts` (50), `gestures/transform.ts` (233) | `edit_begin` → per-frame `TransformSelection` steps → `edit_end`, one undo entry (`transform.ts:211`) |
| Zoom / Hand | Z / H | unchanged from Task 6 | already §5-conformant: click in, ⌥-click out, drag = marquee zoom (`viewGestures.ts:115`); Hand pans |
| Text | T | `tools/registry.ts` | registers its key, icon and cursor; inert until Task 9 |

**Pen** (§5, ruling 2). Nodes accumulate locally with a rubber band to the
pointer (`pen.ts:238`); a press-drag sets the outgoing handle and mirrors the
incoming one unless ⌥ is held (`:147`), ⇧ constrains the handle to 45°
(`:135`), Space repositions the node the press is placing (`:114`), and a click
on the first node closes — the closing press may still drag, which shapes that
node's incoming handle (`:95`, `:140`). Enter finishes, Escape finishes with
≥ 2 nodes and cancels below that (`:168`), and a tool switch commits through
`finish()`. `penPath.ts:47` builds cubic segments on a PostScript layer and one
control per segment on a quadratic one.

**Knife** (ruling 4). `cutPath.ts` solves the crossings on the exact Béziers
(16 samples per curved segment bracketing each sign change, then bisection —
`pathGeometry.ts:254`), splits the contour there (`extractSegments`, `:201`)
and rebuilds the pieces: an open contour splits at every crossing
(`cutPath.ts:42`), a closed one with an even number of crossings is cut into
closed pieces by pairing the cut points *along the cut line* and joining each
pair with a chord (`cutPath.ts:67`) — the Jordan-curve pairing, which is what
makes a rectangle cut vertically come back as two closed four-point contours.
An odd number of crossings is left alone.

**Shapes** (ruling 5). `shapeContours.ts:38` is the rectangle, `:50` the
ellipse — four cubic quarters at κ = 0.5522847, or eight quadratic eighths
whose controls sit sec(22.5°) out on the bisector — and `:107` gives the result
the layer's winding (counter-clockwise for PostScript, clockwise for
TrueType). ⇧ squares the box and ⌥ grows it from the centre (`shapes.ts:31`).
A click opens the inline numeric dialog (`views/glyph/ShapeDialog.tsx`, 24 px
padding, title 16/600, Enter creates).

**Measure** (ruling 6). Purely local: the crossings come from the same
`lineHits`, sorted along the line, and each span between two of them is
labelled at its midpoint; the HUD reads `d <length>  ∠ <angle>`
(`measure.ts:118`). ⇧ constrains.

**Rotate / Scale** (ruling 7). A click names the origin, a drag transforms
about it (`transformTool.ts:81`, `:103`); the origin defaults to the centre of
the selection, or of the layer when nothing is selected (`:214`). The gesture
streams the *step* since the last apply, never the total
(`gestures/transform.ts:77`, `:211`), and with nothing selected the command
names no points, no anchors and no components, which is how
`points::transform_selection` is told to transform the whole layer. ⇧ snaps
rotation to 15° and keeps the two scale axes equal.

### Snapping completed (ruling 10)

`snapping.ts` now takes a `SnapSettings` (`:64`) and a list of component ghost
points, and resolves node > handle > anchor > guide > metric > grid >
alignment line. Grid snapping is `prefs.grid_size` when `snap.grid` is on
(`:225`); component ghosts come from `glyphs.flattened_components`
(`views/glyph/useComponentPoints.ts`, drawn by `editorFrame.ts:82`) and rank
just behind the layer's own features. Each family is gated by its pref. `editorSnapping.ts` (116) holds the
standing preference, the settings and the last match, and serves both callers —
a drag snapping a delta and a tool snapping the point it is about to place.
Preferences are read on every glyph-tab mount (`stores/prefsStore.ts`), which
also drives the arrow-key steps (`prefs.nudge`) and the grid the canvas draws
(`views/glyph/drawTools.ts:30`, §4.1 item 1, skipped below 4 px on screen).

### Cursors (ruling 11)

`app/src/editor/cursors.ts` (145) generates ten 24 px SVG cursors as inline
`data:` URIs — pen with its `+` / `×` / `○` badges, knife, measure, rotate,
scale, insert-node, break-handle — each drawn as a wide dark stroke under a
narrow white one and each ending in a standard cursor name, which is the CSS
fallback the platform uses when it refuses the image. The Select tool now uses
the insert-node and break-handle cursors it had been approximating with `copy`
and `crosshair` (`tools/select.ts:48`, `:59`).

### Carried from Task 6

**A — the gate now covers every close.** `EditorController.gate()`
(`editorController.ts:553`) records whatever is closing; `settle()` (`:576`)
closes a live *drag* as well as a nudge run, `pointerUp` (`:397`) and
`pointerDown` (`:311`) both go through it, `escape()` gates the cancel
(`:601`), and `nudgeSelection` hands the controller's own gate to the run
(`:534`), which `NudgeRunner` combines with the close it started itself
(`nudgeRun.ts:61`, `:139`). Four regression tests on `createEditingTransport`
cover the probes: settle-close → arrow, pointer-up → arrow, drag → drag, tool
switch mid-drag (`editorController.test.ts:1036`, `:1060`, `:1083`, `:1107`). All four fail on the pre-fix code.

**B — components are previewed on the canvas.** `drawComponents` takes one
`ComponentDraw` per component — its own flattened path plus the preview offset
— instead of the single whole-layer path (`views/glyph/drawComponents.ts:66`,
`:118`); `GlyphCanvas` builds that list, caching each parsed `Path2D`
(`GlyphCanvas.tsx:491`, `:509`). `hitInput()` and `frame()` now take
`this.components()`, i.e. the previewed ones (`editorController.ts:690`,
`:289`).

**C — `gestures/drag.ts` split.** `gestures/drag/{plan,deltas,preview,commands}.ts`
(224 / 159 / 121 / 116) plus `index.ts` (307) holding the class; the existing
`gestures/drag.test.ts` passes unchanged.

**D — trivia.** `editOutline()` caches per contour array in a `WeakMap`, so the
two callers inside one pointer move share one build (`editorController.ts:646`);
the token transport's `doc.undo` finalises the active edit like the kernel
(`test/mockTransport.ts:141`); a run that closes on its idle timer carries its
result forward until the reload lands (`nudgeRun.ts:192`, cleared by
`reloaded()` at `:185` from `setSource`); `closeNudge` leaves a live drag's
preview alone (`editorController.ts:586`).

## Verification

```
pnpm --dir app gen:api          → clean
pnpm --dir app check            → tsc -b, silent
pnpm --dir app test             → 62 files, 561 tests passed (6.8 s)
pnpm --dir app lint             → eslint, silent
pnpm --dir app exec prettier --check .  → all matched files use Prettier style
FW_API_PORT=8791 FW_UI_PORT=1424 pnpm --dir app e2e
                                → 9 passed, 1 skipped (the CJK spec, no fixture)
```

Tests added this task: 561 − 476 = **85**.

- `editor/pathGeometry.test.ts` (13) — contour ↔ path round trips, de Casteljau
  splitting, crossings that land on the curve rather than on its polygon,
  crossings beyond the ends of the line, arc extraction with wrap-around,
  winding and its reversal.
- `editor/cutPath.test.ts` (6) — **a rectangle cut vertically → two closed
  four-point contours**, **an open path cut once → two open paths**, a stroke
  that misses, a stroke that only grazes (odd crossings), a curved contour cut
  on the curve, and the reported cut points.
- `editor/tools/pen.test.ts` (14) — **three clicks and a click on the first node
  → one `AddContour`, closed, three points**; nothing emitted until the path
  finishes; a press-drag's mirrored handles; ⌥ leaving the incoming handle
  alone; ⇧ constraining to 45°; Space repositioning; extending an existing open
  contour as `DeleteContours` + `AddContour`; a path picked up at its first node
  being reversed; Escape cancelling below two nodes and finishing at two; a
  tool switch committing; quadratic segments on a quadratic layer; the rubber
  band; the three cursor variants.
- `editor/tools/drawingTools.test.ts` (18) — pencil (the `AddContour` +
  `Simplify` batch, the 2 px sample spacing, ⇧ straight spans, a stroke that
  closes on itself, a one-sample stroke sending nothing), knife (the batch, a
  miss, the crossings marked mid-drag, ⇧), shapes (the dragged rectangle, the
  dialog request, ⇧/⌥ boxes, four cubic quarters, eight quadratic eighths, the
  `w × h` HUD), measure (the readout, the marks, the span labels, no command).
- `editor/tools/transformTool.test.ts` (10) — a rotation about a clicked
  origin as one `edit_begin`/`apply`/`edit_end` with the quarter-turn affine,
  the live preview, ⇧ snapping to 15°, the default origin (selection, then
  layer), the whole-layer command naming nothing, per-axis scale ratios,
  ⇧ uniform scale, **each apply carrying the step and not the total**, and
  Escape closing and undoing.
- `editor/cursors.test.ts` (4) — a 24 px inline SVG with a hotspot and a
  standard fallback for every cursor, escaped so the URI carries no raw markup.
- `editor/snapping.test.ts` (+7) — component ghosts on and off, both grid axes,
  a zero grid step, the full resolution order, and nodes off silencing the
  alignment lines.
- `views/glyph/useToolShortcuts.test.tsx` (4) — a tap switching for **every key
  of the §5 table**, the 200 ms borrow and its release, ⇧F staying on the
  ellipse when Shift is released first, and a modified key left to the commands.
- `editor/editorController.test.ts` (+9) — the four gate probes plus the pen
  routed through the controller (one `doc.apply`, closed, three points), the
  tool overlay reaching the frame, Enter and Escape claimed before the command
  registry, the shape dialog request, and a tool switch committing an open path.
- `views/glyph/drawLayers.test.ts` (+1) — a component drawn from its own path,
  offset by the preview.

### e2e

`app/e2e/draw.spec.ts` creates a document with `Mod+N`, adds `A`, opens it from
the grid, presses `P`, clicks three corners and clicks back on the first, then
asserts what the *kernel* stored: one contour, closed, three points, with the
status bar flashing `Draw path`. It uses a dev-only `globalThis.__fwCall`
bridge (`api/client.ts:37`) because there is no Add Glyphs surface yet; it
compiles out of a production build, like Task 6's `__fwEditor`.

### Interactive pass (agent-browser, ports 8791 / 1424, both stopped afterwards)

Kernel with `DejaVuSans.ttf` and `SourceSerif4Min.otf`; a scratch “New Font”
was created for the drawing tools and closed at the end. Both fixture documents
were left clean (`dirty: false`, `undo_label: null`) and the preferences this
session changed were restored.

1. **Pen** — `P`, three clicks, then a press-drag on the fourth node: the
   mirrored handle pair, the curve preview and the HUD `∠ 29.2° ℓ 108.1`.
   Clicking the first node closed it: one contour, `closed: true`, 8 stored
   points (4 nodes + 4 controls), undo label `Draw path`.
2. **Shapes** — `⇧F`, drag: the live ellipse with the HUD `w 592 × h 434`;
   released → 4 `Curve` + 8 `OffCurve` points, undo label `Draw ellipse`.
3. **Knife** — `E`, a stroke across that ellipse: the dashed cut line with both
   crossings marked; released → **two closed contours of 10 points each**, undo
   label `Cut`.
4. **Measure** — DejaVu `o`, `L`, a horizontal drag: four crossings marked and
   the spans labelled **194 · 639 · 195** with the HUD `d 1425  ∠ 0.0°`.
   Nothing was applied.
5. **Rotate** — `Mod+A` (24 points), `R`, a click to set the origin and a drag:
   the outline turned live, HUD `∠ 25.8°`, the origin marker drawn, the Info box
   tracking the rotated bounding box, `24 of 24 points`. One `Mod+Z` restored
   DejaVu exactly (`undo_label: null`, `dirty: false`).
6. **Tap/hold** — a tap on `B` switched to Pencil; holding `H` for 320 ms
   borrowed the Hand (`aria-pressed`, cursor `grab`) and releasing gave the
   Pencil back.
7. **Pencil** — a 40-sample freehand stroke committed as one entry `Draw`, the
   simplifier returning a mixed line/curve contour.
8. **Grid and grid snapping** — `prefs.set { grid_size: 100, snap.grid: true }`
   then reopening the tab drew the grid; a pen click three pixels off the
   (300, 300) crossing landed on **exactly (300, 300)**.
9. **Cursors** — the canvas `cursor` while the pen is active is the inline
   `url("data:image/svg+xml,…")` value, not a standard name.

### Screenshots

| File | Caption |
| --- | --- |
| `docs/screenshots/p1-tools-pen.png` | Mid-draw with the pen: three placed nodes, the rubber band, and a press-drag on the fourth node showing the mirrored handle pair and the HUD `∠ 29.2° ℓ 108.1`. |
| `docs/screenshots/p1-tools-shapes.png` | The Ellipse tool mid-drag, the live shape following the pointer with the HUD reading `w 592 × h 434`, the tool strip showing Ellipse active. |
| `docs/screenshots/p1-tools-knife.png` | A knife stroke across that ellipse: the dashed cut line with a mark at each of the two crossings, and the ellipse's own nodes and handles. |
| `docs/screenshots/p1-tools-measure.png` | Measuring across DejaVu `o`: four crossings marked and the per-span distances 194 · 639 · 195 drawn on the line, HUD `d 1425 ∠ 0.0°`. |
| `docs/screenshots/p1-tools-rotate.png` | Rotating all 24 points of `o` about a clicked origin: the origin marker, the turned outline, HUD `∠ 25.8°`, and the Info box tracking the rotated bounding box. |

## New i18n keys (20) — for Task 13 to translate

`editor.undo.{drawPath,extendPath,draw,cut,drawRectangle,drawEllipse,rotate,scale}` ·
`editor.hud.{distance,percent}` ·
`editor.snap.{anchor,grid,component}` ·
`dialog.shape.{rectangleTitle,ellipseTitle,x,y,width,height,create}`

`zh-CN.json` carries all twenty with the English text as a placeholder; nothing
was translated here. `editor.hud.percent` (`⤢`) and `editor.hud.distance` (`d`)
are notation rather than prose, like the existing HUD symbols.

## Deviations and rulings applied

1. **`PathOp::Simplify` is a whole-layer operation.** Ruling 3 asks for
   `Batch([AddContour(polyline), PathOp::Simplify(that contour, …)])`, but the
   generated `PathOp` has no contour field and
   `fw_geom::simplify(&layer.contours, …)` refits every contour of the layer
   (`crates/fw-geom/src/edit.rs:76`); naming one contour needs a kernel change,
   which this task may not make. The batch is emitted exactly as ruled and the
   options are made as conservative as they can be — `ignore_extrema: true`, so
   the pass does not also add extrema to contours the user did not draw
   (`tools/pencil.ts:48`). A pencil stroke on a layer that already has outlines
   therefore refits those outlines too, in the same undo entry. See concern 1.
2. **`Prefs` has no simplify tolerance and no hit/snap distances.** Ruling 3's
   "error from prefs or 1.0" resolves to 1.0 (`PENCIL_SIMPLIFY_ERROR`); §4.2's
   6 px snap and the §4.2 hit radii stay the published constants. What *is*
   prefs-driven is `grid_size`, the five `snap.*` flags and `nudge`.
3. **The curve flavour is inferred, then the format decides.** `LayerOutline`
   does not carry the font's declared `CurveKind`, so `inferCurves`
   (`pathGeometry.ts:381`) reads a `QCurve` as quadratic and a `Curve` as
   cubic, and falls back to the document's `format` string — quadratic for
   TrueType-family sources, cubic otherwise — for a layer with no curves at all.
4. **The knife pairs cut points along the cut line, per contour.** That is the
   Jordan-curve pairing and is exact for any single simple contour. A stroke
   that crosses *two* contours (the outer and inner of an `o`) cuts each of
   them into its own closed pieces rather than joining outer to inner; Glyphs
   joins across contours there. Named in concern 2.
5. **Shift keeps the drag's length** when it constrains a knife, measure or pen
   direction, exactly as `constrainDelta` does for a Select drag, rather than
   projecting onto the axis. Consistency with the existing gesture won.
6. **Pen cursor variants.** §2.4 lists "×" for closing a path and "○" for the
   start node; read as: `×` while hovering the first node of the path being
   drawn (the click closes it) and `○` while hovering an end node of an existing
   open contour (the click picks it up). `+` is the segment hover.
7. **A pencil stroke closes itself** when its last sample comes within 10 px of
   its first, which is what a freehand shape almost always wants; the spec does
   not say either way.
8. **`editorController.ts` is 796 lines**, inside the dispatch's 800 cap but
   still large: the tool context and pointer conversion moved into
   `toolHost.ts`, snapping into `editorSnapping.ts` and the drag set-up into
   `editorActions.startDragGesture`. Everything new is well under 450 except
   `tools/pen.ts` (352) and `pathGeometry.ts` (391), which are also under it.
9. **A dev-only `globalThis.__fwCall`** was added (`api/client.ts:37`) so the
   e2e suite and interactive verification can create a font and add a glyph;
   there is no Add Glyphs surface yet. It sits behind `import.meta.env.DEV`
   alongside Task 6's `__fwEditor` and `__fwPaint`.

## Concerns / what a reviewer should look at

1. **The pencil's simplify pass touches the whole layer** (deviation 1). It is
   one undo entry, so it is recoverable, but drawing a stroke into a glyph that
   already has outlines refits those outlines. The honest fix is a
   contour-scoped `PathOp::Simplify` in `fw-doc`, which is a kernel change.
2. **A knife stroke across two contours cuts them independently.** For the
   common cases — a bar across a stem, a cut through a bowl — the result is
   right; for a stroke straight through an `o` the user gets four closed pieces
   where Glyphs would give two. Joining across contours needs a planar
   arrangement, which is a `fw-geom` job rather than a browser one.
3. **The grid has no user-facing switch.** It is drawn and snapped to exactly
   when `prefs.grid_size > 0`, and P1 has no Preferences dialog (§9), so today
   only `prefs.set` reaches it. A View-menu toggle was deliberately not added
   because §11 does not list one; the Preferences task should own it.
4. **Preferences are read on tab mount only.** There is no prefs-changed event
   in the kernel, so a change made in one tab reaches another when that tab
   remounts. `usePrefsStore` is ready for an event the moment one exists.
5. **The pen's one-sided cubic puts the far control on the node.** A segment
   where only one endpoint has a handle emits the other control at the node's
   own coordinates, which is a valid degenerate cubic but not what a tidier
   editor would store; converting those to a line when the handle is short
   would be a small follow-up.
6. **`transformsWholeLayer` is exported but unused** by production code — it
   documents the "no points named" contract that `TransformGesture` relies on
   and is covered by the whole-layer test. Left in deliberately; a reviewer may
   prefer it inlined.
7. **The Info box has no rows for a shape or a transform origin.** Ruling 27
   made them optional and they were left out; the HUD carries the live numbers
   and the shape dialog carries the numeric entry.

Report written 2026-09-05.
