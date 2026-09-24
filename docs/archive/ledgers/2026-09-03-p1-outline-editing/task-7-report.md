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

---

# Fix round 1

Review: `task-7-review.md` (Needs fixes — 2 Important, 11 Minor). Eight commits
on `p1-t7` from `cbb16b1`, on top of a merge of `p1-outline-editing`
(Task 9a's `Dialog` primitive, dialogs and prefs store; Task 11's e2e config
with `FW_CONFIG_DIR`; Task 6/11 fixes). Not pushed.

| SHA | Subject |
| --- | --- |
| `7711ee1` | merge: p1-outline-editing (Task 9a, Task 11 config, Task 6/11 fixes) into p1-t7 |
| `5412b31` | fix(app): fit a pencil stroke locally instead of refitting the layer |
| `76eecdd` | fix(app): preview a component under the whole affine, not a slide |
| `efee248` | fix(app): free the measure's keys, clear a tool's halo and gate a live drag |
| `30dd7d8` | fix(app): carry a contour's name and id through a cut and an extend |
| `10719c4` | fix(app): build the shapes dialog on the shared primitive and read the curve flavour honestly |
| `100fe9b` | chore(app): let createTool say which tools are inert, not a flag |
| `d2f30ed` | test(e2e): draw into a font of the spec's own, through the New Font dialog |
| `9917ab3` | fix(app): split the fitting tolerance so a quadratic stroke stays in budget |
| `1dea9d7` | chore(app): keep the editor controller inside its line budget |

## Step 0 — the merge

Three conflicts, all resolved by keeping both sides:

- `app/src/stores/prefsStore.ts` — Task 9a's queued, kernel-confirmed store
  taken whole; this task's two readers (`snapSettingsOf`, `nudgeStepsOf`,
  `prefsStore.ts:290-306`) re-applied on top and re-typed for a non-null
  `Prefs`. `GlyphView` needed no change.
- `app/src/i18n/{en,zh-CN}.json` — Task 9a's 137 new keys plus this task's 20,
  merged programmatically and checked key-for-key: 683 keys, and the two
  bundles carry exactly the same set.
- Generated bindings were stale against Task 9a's Rust (`AddGlyphsParams`
  gained `dry_run`), so `pnpm --dir app gen:api` was re-run.

After the merge and `pnpm install --frozen-lockfile`: `tsc -b` silent,
**76 files / 660 tests** passing, eslint and Prettier clean — before any fix.

## Important

**I1 — the pencil no longer refits the layer.** `PathOp::Simplify` is gone from
the pencil entirely. A stroke is now fitted in the browser by
`app/src/editor/curveFit.ts` (Schneider's least-squares cubic fit, 1990:
chord-length parameterisation, a 2×2 least-squares solve for the two tangent
magnitudes at `:161`, Newton-Raphson reparameterisation at `:121`, and a split
at the worst sample at `:282`) behind a corner pass (`:357`) so a deliberate
corner survives. `PencilTool.commit` calls `fitContour` (`tools/pencil.ts:152`)
and `drawCommand` is one `AddContour` (`:169`).

Two things had to be got right that the textbook version gets wrong at a
font editor's tolerance. Schneider gates reparameterisation on `error²`, which
never fires at one unit, so the fit split far more than it needed to; it now
always tries (`curveFit.ts:305-315`). And acceptance is decided on the *true*
geometric distance to the curve (`distanceToCubic`, `:222`), not the parametric
distance at `u[i]`, which on a badly parameterised run reads far smaller than
the curve's real deviation. On a quadratic layer the tolerance is split between
the two fits — samples to cubics, cubics to quadratics — because their errors
add (`CUBIC_SHARE`, `:507`); the conversion takes the larger share because its
bound falls by eight per subdivision. That split alone took a real stroke from
96 stored points to 42.

Tests (`app/src/editor/curveFit.test.ts`, 9): **~200 noisy circle samples fit to
under twenty stored points and stay within the one-unit tolerance**; **a
polyline with a 90° corner keeps exactly one corner node, unsmoothed**; a
straight run comes back as a `Line`, not a flat curve; the joins inside a run
are smooth and the run's last node is not; a quadratic layer gets one control
per segment and no `Curve`; a cubic is subdivided until one quadratic is inside
the tolerance; a quadratic layer's *total* deviation stays inside the same
budget; fewer than two distinct samples produce nothing.
`tools/drawingTools.test.ts:38` asserts the command is **a single
`AddContour`** — `Object.keys(command)` equals `['AddContour']` — drawn into a
layer that already has a contour.

**I2 — a rotate or scale now previews components as transformed.**
`ComponentDraw` carries an affine instead of a `dx`/`dy` pair
(`views/glyph/drawComponents.ts:70-80`) and `paintPath` applies it with
`ctx.transform(...)` inside the font-space flip (`:112`), which takes the same
`[a b c d e f]` order the kernel does. The delta is computed in the new
`views/glyph/componentDraws.ts:48` as `previewed ∘ stored⁻¹` — the path the
kernel returned is already under the *stored* transform — with
`invertAffine`/`isInvertible` added to `editor/affine.ts:43`, `:54`, and a
translation fallback for a singular stored transform.

Tests (`views/glyph/componentDraws.test.ts`, 5): an unpreviewed component sits
at the identity; a translation stays a translation; **a quarter turn puts a
component whose origin is (100, 0) at (0, 100) — exactly where
`points::transform_selection`'s `affine * component.transform` will put it — and
the delta's linear part is the turn, which no `dx`/`dy` could express**; a
singular stored transform degrades to a translation; each path is parsed once.
`drawLayers.test.ts:288` asserts the canvas issues `transform` with the quarter
turn rather than a `translate`.

## Minor

**3 — `MeasureTool` frees the keyboard.** `busy` is now true only while the
button is down (`tools/measure.ts:67`) and `claimsKey` returns `false`
outright (`:73`), so a measurement left on screen no longer disables
`path.toggleSmooth`.

**4 — the snap halo dies with the gesture.** `ToolHost.route` notices a tool
that has stopped being busy and reports it (`toolHost.ts:233`, `:238`); the
controller clears the snap match there (`editorController.ts:147`). Test:
`editorController.test.ts` "clears the snap halo when a tool gesture ends" — a
pen click on the square's corner shows the halo and the pointer-up removes it.

**5 — a cut and an extend keep the contour's identity.** `pathToContour` takes
a `ContourIdentity` (`pathGeometry.ts:105`); `cutContours` gives it to the
largest piece, by area for a closed cut and by traced length for an open one
(`cutPath.ts:49`, `:141`); the pen's extend carries the picked-up contour's
name and id (`tools/pen.ts:89`, `:285`, `penPath.ts:107`). `tt_index` stays
`Unnumbered` by design, and the reason is written where the points are built
(`pathGeometry.ts:87-95`): the points did not exist in the source, so there is
no TrueType number to carry and inventing one would break the hinting that
numbers them. Tests: `cutPath.test.ts` (the larger piece inherits `name`/`id`,
the other gets `null`; every rebuilt point is `Unnumbered`) and `pen.test.ts`
(an extended path keeps `stem`/`c-7`; a fresh path borrows nothing).

**6 — an arrow press closes a live drag first.** `nudgeSelection` calls
`settle()` (`editorController.ts:525`), which closes a drag that is merely
*open* — one that registers no gate of its own. Test:
`editorController.test.ts` "closes a drag that is still open before the arrow
press begins" on the token transport: pointer down, move, arrow, and the calls
are the clean `begin·apply·end · begin·apply·end` with no `InvalidEditToken`.

**7 — controller size:** accepted as-is. The fixes that touched
`editorController.ts` are four lines of wiring, which took it to 801; two
comment blocks were tightened to bring it back to **799**, inside the cap.

**8 — the grid floor is documented.** `MIN_GRID_SPACING`
(`views/glyph/drawTools.ts:29`) now carries the §4.1 citation and the reason:
at "≥ 1 px spacing" a 1-unit grid paints a 0.5 px line every device pixel,
which is a grey wash rather than a grid, so it disappears below four CSS pixels
instead.

**9 — the shapes dialog is built on the shared primitive.** `ShapeDialog.tsx`
is now a `Dialog` (focus trap, Escape, Enter, the §9 chrome and the modal
count) with a `committed` ref so the confirm and the focused button's own click
cannot commit twice. Tests (`views/glyph/ShapeDialog.test.tsx`, 4): the created
contour is the one a drag of the same box would have made; **Enter from the
Create button applies exactly once**; Tab stays inside the dialog through eight
presses; Escape cancels and applies nothing.

**10 — `inferCurves` reads the outline first, the format last.**
`pathGeometry.ts:406` now scans the whole layer and lets quadratic win a
mixture (a `glyf` outline cannot hold a cubic), and only a format that names
the outline table outright decides for a layer with no curves at all
(`GLYF_FORMATS`, `:421`). WOFF and WOFF2 wrap either table, so they say nothing
and the cubic default stands. Five tests in `pathGeometry.test.ts`.

**11 — dead code.** `transformsWholeLayer` is gone; `ToolDefinition.live` is
gone, and `createTool` is the single answer to "does this tool do anything"
(`registry.test.ts` asserts it directly); the two `void` no-ops went with the
merge and the pencil rewrite.

**12 — `ToolHost.consume` repaints only for a consumed event**
(`toolHost.ts:250-259`). The canvas offers every key to the active tool, so an
unclaimed key press no longer repaints the editor. Test:
`editorController.test.ts` "repaints for an event a tool consumed, not for one
it ignored" counts repaints across an unclaimed `q` (zero) and a pen click
(more than zero).

**13 — the degenerate one-sided cubic:** accepted as-is.

**Not in the review — `draw.spec.ts` had to be rewritten.** After the merge
`Mod+N` opens the New Font dialog instead of creating a document, so the old
spec's `glyphs.add` landed on the *fixture* font and broke two other specs. It
now creates a document of its own through the dialog (unique family, no starter
set), adds `A` through the shared `api()` helper from `e2e/support.ts`, and
closes its document in a `finally`. The dev-only `globalThis.__fwCall` bridge
was removed with it: `support.ts` covers what it existed for.

## Verification

```
pnpm --dir app gen:api          → clean, no generated diff
pnpm --dir app check            → tsc -b, silent
pnpm --dir app test             → 77 files, 690 tests passed
pnpm --dir app lint             → eslint, silent
pnpm --dir app exec prettier --check .  → all matched files use Prettier style
FW_API_PORT=8791 FW_UI_PORT=1424 pnpm --dir app e2e
                                → 14 passed, 1 skipped (CJK, no fixture)
```

The e2e suite was run four times. Two of those runs, at a load average in the
thirties with other checkouts building, timed out one to three of the
*existing* Font-view and dialog specs on `toBeVisible` waits; each of them
passes on its own and in the clean runs, and `draw.spec.ts` passed in every
run. See concern 6.

Tests: 660 after the merge → **690**, thirty added this round (9 curve fit, 5
component draws, 4 shapes dialog, 5 `inferCurves`, 3 controller, 2 cut
identity, 2 pen identity — less the pencil test that was rewritten rather than
added).

### Interactive checks (agent-browser, ports 8791 / 1424, both stopped afterwards)

1. **Pencil into DejaVu `o`** — a 45-sample freehand stroke through the counter
   committed as one entry `Draw`, and `glyphs.outline` shows the glyph's two
   original contours **byte-identical to before the stroke**
   (`JSON.stringify` equality), with the stroke added as a third contour. Under
   the old whole-layer `Simplify` both of them would have been refitted. The
   stroke is quadratic, because `inferCurves` reads DejaVu's own `QCurve`
   points. `Mod+Z` left the document clean (`dirty: false`,
   `undo_label: null`).
2. **The tolerance split, measured in the running app** — the same stroke
   before and after `9917ab3`: 96 stored points → **42** (23 on-curve), with the
   originals still untouched.
3. **Rotate on the composite `Eacute`** — nothing selected, so the whole layer
   turns: both components (`E` and `Acute`) are drawn **rotated** by 25.8°, not
   slid, with the origin marker at the clicked point and the HUD reading
   `∠ 25.8°`. Escape closed and undid the gesture; the component transforms are
   back at `[1,0,0,1,0,0]` and `[1,0,0,1,1182,373]` and the document is clean.

### Screenshots

`p1-tools-{pen,knife,shapes,measure,rotate}.png` are unchanged — the chrome
they show did not move. Two were added:

| File | Caption |
| --- | --- |
| `docs/screenshots/p1-tools-pencil.png` | A freehand stroke fitted into the counter of DejaVu `o`: smooth fitted nodes along the stroke, and the glyph's own two contours exactly as they were — one `AddContour`, undo label "Draw". |
| `docs/screenshots/p1-tools-rotate-composite.png` | Rotating the composite `Eacute` with nothing selected: both components drawn turned by 25.8° rather than slid, the origin marker at the clicked point and the HUD reading `∠ 25.8°`. |

## i18n

No keys were added or changed this round; the twenty from the first round are
unchanged and both bundles hold the same 683 keys after the merge.

## Concerns

1. **A quadratic layer still stores more points than a PostScript one for the
   same stroke** — 42 against roughly a third of that — because the conversion
   subdivides to hold the tolerance and because `pathToContour` writes an
   explicit on-curve point between every pair of controls rather than relying
   on TrueType's implied midpoints. Emitting consecutive off-curve points where
   the on-curve one is their midpoint would roughly halve it; that is a change
   to the shared point-list builder and felt like more than a fix round should
   take on.
2. **The corner threshold is fixed at 60°** (`CORNER_ANGLE`), measured across a
   two-sample window. It is right for a pointer sampled every 2 CSS pixels; a
   stylus at a much higher rate would want the window widened rather than the
   angle changed. `Prefs` has nowhere to put either.
3. **`fitRun` recurses with a depth cap of 24** and accepts the best fit it has
   at the cap rather than splitting further, so a pathological stroke degrades
   in accuracy rather than in stack depth. No stroke in testing came near it.
4. **The e2e suite is order-sensitive on a shared kernel.** `draw.spec.ts` now
   closes the document it creates, which is what the other specs need, but the
   session is still global: a spec that leaves a document open changes what
   `session.list` and the tab bar hold for everything running beside it.
6. **The e2e suite is load-sensitive**, as the Task 6 unit suite was: under a
   heavy parallel build the shared Vite server serves modules slowly enough
   that a `toBeVisible` wait in an unrelated spec can expire. Both contended
   runs failed only pre-existing specs, and both sets passed in isolation
   immediately afterwards.
7. **Vite would not start twice during the interactive pass** with `EMFILE`,
   because other checkouts on this machine had exhausted the inotify instance
   budget; `CHOKIDAR_USEPOLLING=1` worked around it. Nothing to do with this
   change, but worth knowing before blaming the app.

Fix round written 2026-09-05.
