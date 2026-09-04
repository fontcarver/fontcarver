# P1 Task 6 — Editor core: report

Branch `p1-t6` in `.worktrees/p1-t6`, base `ab056b9`. Not pushed.

## Commits

| SHA | Subject |
| --- | --- |
| `2458fce` | feat(app): canvas design tokens and the design-spec node grammar |
| `8b8dd2b` | feat(app): editor core — selection, hit testing, snapping and edit sessions |
| `794e0cb` | feat(app): editable glyph canvas — selection, drag editing, info box, undo wiring |
| `a2d7a5f` | test(app): cover the editor's selection, gestures, snapping, tools and Info box |
| `af3a65c` | fix(app): keep the editor mounted across reloads and fix the modifier grammar |
| `fd0fd90` | docs(licensing): re-survey the npm dependency licenses after adding lucide-react |
| `c1f9402` | refactor(app): split frame assembly out of the editor controller |
| `2433b2b` | docs(screenshots): refresh the drag capture after the frame refactor |

## What was built

### `app/src/editor/` — the interaction core (plain TypeScript, no React, no DOM)

| Lines | File | Responsibility |
| ---: | --- | --- |
| 176 | `selection.ts` | `EditTarget` union, immutable keyed `Selection`, click/extend/toggle, `PointRef` extraction, counts |
| 304 | `outlineGeometry.ts` | editable index over a layer: stored points, kernel-compatible geometric segments, nearest-point-on-segment, adjacency/handle/opposite-handle lookups, local translate/set-position |
| 197 | `hitTest.ts` | design spec §4.2 radii and precedence |
| 202 | `snapping.ts` | §4.2 snap distance and ordering, alignment lines |
| 167 | `editSession.ts` | `edit_begin` → rAF-throttled `edit_apply` → `edit_end`, cancel-by-undo, `doc.apply` helper |
| 516 | `gestures/drag.ts` | what a pointer-down drags, modifier constraints, mirror handle, optimistic preview, per-frame command |
| 95 | `gestures/marquee.ts` | normalised band, enclosed nodes/handles/anchors/components, Alt ignores handles |
| 135 | `gestures/nudge.ts` | 1/10/100 steps, 500 ms coalescing window |
| 250 | `pointCommands.ts` | every single-shot command builder (`SetSmooth`, `DeletePoints`, `InsertPointOnSegment`, `ConvertSegment`, `SetPointPosition`, `MovePoints`, `TransformSelection`, `SetAnchor`, `SetComponent`, `SetGuide`, `SetMetrics`) |
| 94 | `hud.ts` | §6.2 HUD text, handle polar form, stem width |
| 38 | `fieldValue.ts` | §7 field expressions (`+10`, `*2`, `/2`, `+-10`) and unit formatting |
| 656 | `editorController.ts` | pointer/keyboard grammar, selection, gesture lifecycle, reconciliation |
| 190 | `editorFrame.ts` | pure assembly of the paint frame: marquee, snap halo, alignment lines, ghosts, HUD |
| 34 | `activeEditor.ts` | the mounted editor, reachable from the command registry |
| 203 | `tools/registry.ts` | the §5 table (key, icon, label key, cursor, group, live flag), tap/hold constant |
| 58 / 18 / 34 | `tools/{select,hand,zoom}.ts` | click grammar and cursors; pan; click/Alt-click/marquee zoom |

### `app/src/views/glyph/` — the React surface

| Lines | File |
| ---: | --- |
| 418 | `GlyphCanvas.tsx` (rewritten: owns the element, feeds the controller, paints) |
| 245 | `GlyphView.tsx` (tool strip + canvas + Info box + live region) |
| 66 + 71 | `ToolPalette.tsx`, `toolPalette.module.css` |
| 77 / 67 / 148 / 166 / 273 / 22 / 148 | `InfoBox.tsx`, `InfoGlyphRows.tsx`, `InfoPointRows.tsx`, `InfoMultiRows.tsx`, `InfoObjectRows.tsx`, `infoBoxProps.ts`, `infoBox.module.css` |
| 125 / 73 | `drawEditing.ts` (§4.1 items 10 + 14), `drawHud.ts` (§6.2 pill) |
| 396 | `drawOutline.ts` (§4.1 node grammar + selection layer) |
| 112 / 49 / 69 | `useLayerOutline.ts`, `useEditorLabels.ts`, `useToolShortcuts.ts` |
| 107 | `../components/NumberField.tsx` |
| 74 | `../canvas/paintStats.ts` (dev-only frame timer) |

Also: `stores/editorStore.ts` (tool, borrowed tool, selection, info-box, snapping, editing flags),
`styles/tokens.css` (the `--cv-*` palette of §2.1), `views/glyph/glyphTheme.ts`,
`views/glyph/glyphCommands.ts` (five new commands), `shell/StatusBar.tsx` (§3 Edit-tab readout),
`views/glyph/useGlyphDetail.ts`, `renderGlyph.ts`, `glyphScene.ts`.

The P0 corner readout (`GlyphHud.tsx`) was deleted: §6.1 gives the Info box the glyph identity and
metrics when nothing is selected, and §3 gives the status bar the same values, so keeping a third
copy in the canvas corner would have been redundant chrome over the glyph.

## How each brief behaviour maps to code

| Behaviour | Where |
| --- | --- |
| Selection of nodes/handles/segments/components/anchors/guides | `editor/selection.ts:20` (`EditTarget`), `editor/hitTest.ts:96` (`hitTest`) |
| Click / Shift-click / Ctrl-click | `editor/tools/select.ts:21` (`selectionForClick`), `editorController.ts:235` |
| Marquee, Alt ignores handles, Shift adds | `gestures/marquee.ts:57`, `editorController.ts:308` |
| Drag: `edit_begin` on pointer-down, `MovePoints` throttled to frames, `edit_end` on pointer-up, one undo entry | `editSession.ts:52`, `gestures/drag.ts:257`, `editorController.ts:577` |
| Optimistic local update | `gestures/drag.ts:361` (`preview`), `editorController.ts:276` |
| Shift constrains 0/45/90° | `gestures/drag.ts:224` (`constrainDelta`) |
| Alt-drag a handle keeps its angle | `gestures/drag.ts:331` (projection + snapping suspended) |
| Ctrl+Alt on a smooth node's handle mirrors | `gestures/drag.ts:296` (mirror set-up), `:452` (`SetPointPosition` in the batch) |
| Alt-drag a node = node only | `gestures/drag.ts:122` (`planDrag`, `handles: PointsOnly`) |
| Double-click or Enter toggles smooth; Alt keeps the outline | `editorController.ts:348`, `:370`; `Enter` is the `path.toggleSmooth` command |
| Delete = KeepShape, Shift+Delete = break | `editorController.ts:384`, commands `edit.delete` / `edit.deleteBreak` |
| Arrows nudge 1/10/100, coalesced within 500 ms | `gestures/nudge.ts:38`, `editorController.ts:438` |
| Tab / Shift+Tab cycle points | `editorController.ts:416`, `GlyphCanvas.tsx` key handler |
| Alt-click a line segment converts to a curve | `editorController.ts:544` (`handleSegmentShortcut`) |
| Shift-click a segment inserts a node | same, `InsertPointOnSegment` at the hit parameter |
| Drag a segment reshapes | `gestures/drag.ts:186` (`segmentRefs`) |
| Escape deselects / cancels a drag | `editorController.ts:466` |
| `Mod+A` selects all points | `editorController.ts:402`, command `edit.selectAll` |
| Info box rows and commits | `InfoPointRows.tsx`, `InfoMultiRows.tsx`, `InfoObjectRows.tsx`, `InfoGlyphRows.tsx` |
| Cursor HUD (Δ, ∠/ℓ, w×h, ⟂, snap name) | `editor/hud.ts:60`, `editorFrame.ts:118`, `views/glyph/drawHud.ts` |
| Hover cursors and ghosts | `tools/select.ts:41`, `editorFrame.ts:76` |
| Tool strip, tap/hold borrow | `tools/registry.ts`, `ToolPalette.tsx`, `useToolShortcuts.ts` |

## Verification

```
pnpm --dir app gen:api
pnpm --dir app check && pnpm --dir app test && pnpm --dir app lint && pnpm --dir app exec prettier --check .
```

All clean: **49 test files, 386 tests passing** (run three times), `tsc -b` silent, ESLint silent
(including the React Compiler rules), Prettier reports no style issues.

New tests:

- `editor/hitTest.test.ts` (10) — the §4.2 radii as published constants, a 5 px node hit and its
  miss, tolerance independent of the device pixel ratio, tolerance scaling with zoom, node over
  handle, selected node over node, anchor over guide over segment, segment `t`, `ignoreHandles`,
  empty canvas.
- `editor/gestures/drag.test.ts` (13) — `planDrag` for selection/target/Alt/handle/segment,
  `constrainDelta`, **exactly one `edit_begin`, one `edit_apply` per frame, one `edit_end`** with the
  undo label `Move 1 points`, a moved-then-not-moved gesture sending nothing, coalescing of several
  moves inside one frame, the optimistic preview, handles carried with a node, Shift axis lock,
  Escape → `edit_end` + `doc.undo`, and no undo for a cancelled gesture that applied nothing.
- `editor/gestures/marquee.test.ts` (6) — normalisation, enclosed nodes/handles/anchors, Alt,
  components only when fully enclosed, Shift union.
- `editor/gestures/nudge.test.ts` (5) — step sizes, one undo entry for a run, a new entry after
  500 ms of silence, a new entry when the selection changes, per-apply steps summing to the run.
- `editor/snapping.test.ts` (7), `editor/fieldValue.test.ts` (4), `editor/tools/registry.test.ts` (5).
- `editor/editorController.test.ts` (13) — click/Shift/Ctrl grammar, marquee, select-all, Tab,
  **`SetSmooth` from the smooth toggle and from a double-click**, `DeletePoints`, `ConvertSegment`
  from Alt-click, `InsertPointOnSegment` from Shift-click, the full protocol through the controller,
  the preview and HUD, and the Ctrl+Alt / Alt modifier grammar.
- `views/glyph/InfoBox.test.tsx` (8) — **`SetPointPosition` from the X field**, the `+40` expression,
  an invalid entry writing nothing, `SetSmooth` from the type toggle, `SetMetrics` with nothing
  selected, `TransformSelection` with the scale affine about the chosen origin, `SetAnchor`, collapse.
- `views/glyph/ToolPalette.test.tsx` (3), `views/glyph/useLayerOutline.test.tsx` (3).

### Interactive session (agent-browser)

Kernel `fw-devserver --port 8789` with `fixtures/fonts/DejaVuSans.ttf` and
`fixtures/fonts/SourceSerif4Min.otf`; UI `vite --port 1422` with `FW_DEV_PROXY`. Both stopped
afterwards; both documents left clean (every exploratory edit undone), nothing saved to disk.

Log:

1. Opened DejaVu `g` (gid 74) in an Edit tab: tool strip, canvas, Info box, §3 status bar all render.
2. Clicked node `1:27`, Shift-clicked `1:28` → `2 points selected`, Info box switches to the
   multi-selection rows (`2 of 41 points`).
3. Dragged: HUD reads `Δx 184  Δy 216.9  ⟂ 184`, an alignment line appears, the outline follows the
   pointer, and pointer-up leaves **one** undo entry `Move 2 points`.
4. `Mod+Z` → nodes back at (930, 1120) and (1114, 1120), document clean, redo label `Move 2 points`.
5. Alt-dragged handle `1:25`: HUD `∠ 119.9° ℓ 123.7` — the angle is exactly the pre-drag angle, so
   Alt held the handle on its line; undo entry `Move handle`.
6. Marquee over the counter → `17 points selected`, Info box `X 303 / Y 98 / W 627 / H 951`,
   9-point origin, proportional lock, `17 of 41 points`.
7. Five `ArrowRight` presses → one undo entry `Nudge 1 point`, node x 930 → 935.
8. Escape mid-drag: the point travelled to (1009.4, 1169.6) and returned to (935, 1120) with the undo
   label unchanged — the cancelled gesture left nothing behind.
9. Tools: Hand cursor `grab`, pans exactly by the drag delta; Zoom cursor `zoom-in`, click
   0.3228 → 0.3712 (×1.15), Alt-click back; holding `H` for 200 ms borrows the Hand (`aria-pressed`)
   and releasing restores Select.
10. Source Serif `o` (CFF, cubic): Ctrl+Alt-drag on a handle mirrors the opposite one; HUD
    `∠ 55.8° ℓ 125.5`.

### Repaint measurements (dev build, `performance.now()` around the paint)

`window.__fwPaint` in a dev build records every frame. On **DejaVu `uni2704`, 329 points**, driving
120 pointer moves at one move per animation frame:

| Case | Frames | Mean | p95 | Max |
| --- | ---: | ---: | ---: | ---: |
| Drag all 329 points | 120 | 2.15 ms | 4.3 ms | 8.2 ms |
| Drag all 329 points (second run) | 120 | 0.74 ms | 1.1 ms | 1.6 ms |
| Drag one node | 120 | 2.6 ms | 5.0 ms | 19.7 ms |
| Hover, no gesture | 60 | 1.2 ms | 2.0 ms | 2.0 ms |
| Drag 2 nodes on `g` (41 points) | 46 | 1.7 ms | 2.5 ms | 2.9 ms |

Mean and p95 are inside the 8 ms budget in every case; single-frame outliers (8.2 ms, 19.7 ms) come
from frames that coincide with a React commit, and the run-to-run spread is machine load. Before the
fix described below, a pointer move painted twice and the same drag measured mean 6.6 ms / p95 29 ms.

### Screenshots

| File | Caption |
| --- | --- |
| `docs/screenshots/p1-select-drag.png` | DejaVu `g` mid-drag: two corner nodes selected and pulled up-right, the cursor HUD reading `Δx 184 Δy 216.9 ⟂ 184`, the alignment line the snap latched onto, and the Info box showing the selection's bounding box, 9-point origin and `2 of 41 points`. |
| `docs/screenshots/p1-select-handles.png` | Alt-dragging an off-curve handle of `g`: the HUD reads `∠ 119.9° ℓ 123.7` — the same angle the handle started at, because Alt locks the angle and suspends snapping — with the Info box on its handle rows (ΔX, ΔY, length, angle). |
| `docs/screenshots/p1-select-infobox.png` | A marquee over the counter of `g` selecting 17 nodes and handles; the Info box shows X/Y/W/H, the 9-point origin picker, the proportional lock and `17 of 41 points`, and the status bar mirrors `17 points selected · snap on`. |
| `docs/screenshots/p1-select-cubic.png` | Source Serif `o` (CFF, cubic): a Ctrl+Alt handle drag mirroring length and angle onto the opposite handle, HUD `∠ 55.8° ℓ 125.5`, with cubic handle lines and smooth (green) nodes throughout. |

## Defects found by the interactive session and fixed (`af3a65c`)

1. **The canvas unmounted on the first frame of every drag.** Each `edit_apply` emits a document
   event; both data hooks blanked their state while the refetch was in flight, `GlyphView` fell back
   to its loading message, `GlyphCanvas` unmounted and `dispose()` cancelled and undid the live
   gesture. The hooks now keep the data for the same glyph on screen across a reload and defer
   reloads entirely while a gesture is live (`useLayerOutline.ts:96`, `useGlyphDetail.ts:120`), and
   the controller keeps its selection across a reload (`editorController.ts:181`).
2. **Alt hid handles from the hit test**, so Alt-dragging a handle started a marquee instead of the
   angle-locked drag §4.3 promises. Ignoring handles is a marquee behaviour and the marquee applies
   it itself.
3. **Ctrl+Alt toggled the selection** as if Ctrl had been pressed alone, dropping the handle the
   mirror drag was about to move.
4. **Every pointer move painted twice** (once from the controller's repaint, once from the canvas),
   and a gesture that never saw its pointer-up could leak into the next one.

## New i18n keys (96) — for Task 13 to translate

`zh-CN.json` carries every key with the English text as a placeholder, as instructed; nothing was
translated by this task.

`tool.{palette,select,pen,pencil,knife,rectangle,ellipse,measure,rotate,scale,zoom,hand,text}` ·
`editor.undo.{movePoints_one,movePoints_other,moveHandle,reshapeSegment,moveAnchor,moveComponent,moveSelection,nudgePoints_one,nudgePoints_other,toggleSmooth,deletePoints,insertPoint,convertSegment,setPosition,setMetrics,setAnchor,setGuide,setComponent,transform}` ·
`editor.hud.{deltaX,deltaY,angle,length,width,height,stem}` ·
`editor.a11y.{selection_one,selection_other}` ·
`infoBox.{label,toggle,invalid,x,y,width,height,deltaX,deltaY,length,angle,type,corner,smooth,curve,onCurve,name,unicode,base,scaleX,scaleY,rotation,autoAlign,useMyMetrics,origin,proportional,locked,unlocked,count}` ·
`infoBox.title.{glyph,node,handle,multi,component,anchor,guide,segment}` ·
`infoBox.originPoint.{0..8}` ·
`cmd.{edit.selectAll,edit.deselect,edit.delete,edit.deleteBreak,path.toggleSmooth,view.toggleInfoBox}` ·
`status.{selectionPoints_one,selectionPoints_other,snapOn,snapOff}`

The HUD symbols (`Δx`, `∠`, `ℓ`, `w`, `h`, `⟂`) and the field labels `X`, `Y`, `W`, `H`, `ΔX`, `ΔY`
are notation rather than prose; Task 13 should probably leave them as they are.

## Deviations and rulings applied

1. **`MovePoints` is relative, so each apply carries the step since the previous one.** Dispatch
   ruling 3 says "the cumulative delta relative to the gesture start", but `apply_in_edit` mutates
   the current state (`fw-doc/src/document.rs:163`), so a cumulative delta would be applied on top of
   itself every frame. What the ruling asks for is preserved exactly — the document ends the gesture
   translated by the cumulative delta, in one undo entry — by sending the difference
   (`gestures/drag.ts:452`, `gestures/nudge.ts:127`), which is asserted in both tests.
2. **`doc.apply` takes a `label`.** The generated `ApplyParams` has `{ doc, command, label }`, not the
   dispatch's `{ doc, command }`.
3. **`MovePoints` carries `delta: [number, number]`**, not `dx`/`dy`; `SetSmooth` also needs
   `level_handles`. The generated types were followed throughout.
4. **Node glyph sizes read §4.1's "5 px circle" as a diameter** (radius 2.5), and off-curve
   "3.5 px circles" as radius 1.75, matching the unambiguous "5 px square". The start marker is now a
   7 px triangle pointing along the path, which replaced P0's ring-plus-arrow; `drawOutline.test.ts`
   was updated accordingly.
5. **`--color-glyph-*` was renamed to `--cv-*`** rather than duplicated: the old names had exactly one
   consumer (`glyphTheme.ts`). Four tokens beyond §2.1's table were needed and added:
   `--cv-selection-ink`, `--cv-hud-ink`, `--cv-ruler`, `--cv-sidebearing`, plus `--cv-node-implied`
   for the implied-point marker the P0 renderer already drew.
6. **A leading `-` in a numeric field is a negative coordinate, not a subtraction** (font space is
   full of negative numbers); `+-10` subtracts. `+`, `*` and `/` are the relative operators.
7. **`editorController.ts` is 656 lines**, inside the dispatch's 800-line limit but above
   CONVENTIONS' ~400-line guideline for TypeScript; `gestures/drag.ts` (516) and `GlyphCanvas.tsx`
   (418) are in the same position. The frame assembly was split out into `editorFrame.ts` to bring
   the controller down from 787.
8. **The Vitest timeout was raised to 20 s.** Two component tests (the glyph grid's pointer-selection
   test, which predates this task, and the `GlyphView` readout test, which now waits for two round
   trips) sat on the 5 s default under a fully parallel run and flaked. Both pass comfortably now;
   the suite ran clean three times in a row.
9. **`lucide-react` (ISC)** was added per ruling 8; `THIRD_PARTY.md` §3.2 was re-surveyed (258
   packages, no copyleft).
10. **Two dev-only hooks** exist behind `import.meta.env.DEV`: `globalThis.__fwPaint` (frame timings,
    `canvas/paintStats.ts`) and `globalThis.__fwEditor` (the mounted controller, `activeEditor.ts`).
    They are how the numbers above were measured and how a future interactive session can re-measure
    them; both compile out of a production build.

## Concerns / what a reviewer should look at

1. **Snapping has no grid target.** §4.2 lists `grid` between `metric` and `alignment line`, but the
   grid overlay itself is a preference that arrives with the drawing tools (T7), and snapping to
   something never drawn would be a mystery. `snapping.ts` says so in its module comment.
2. **Some §4.1 markers are still missing**: the diamond for a node sitting on a vertical metric
   (needs the metrics in `drawOutline`), the G2 star (P2), the locked-component hatch and the
   measurement-guide intersections. Duplicate-node rings, open-path ticks, the start triangle, the
   selection layer, the marquee, the snap halo, alignment lines and both ghost kinds are in.
3. **Custom SVG cursors are not implemented.** §2.4 asks for 24 px SVG cursors for insert-node and
   break-handle; `tools/select.ts` uses the nearest standard cursors (`copy`, `crosshair`) and says
   so. They want a designed asset, which felt like design work rather than a guess.
4. **The Info box lags during a drag.** Its fields read the fetched outline, and reloads are deferred
   while a gesture is live, so the numbers settle on pointer-up rather than tracking the pointer. The
   HUD carries the live numbers. Feeding the preview into the box would be a small follow-up.
5. **Component and anchor drags go through `TransformSelection`** with a translate affine; only
   nodes and handles use `MovePoints`. Both coalesce identically, but a reviewer may prefer a
   different command for anchors.
6. **The glyph name and Unicode are read-only in the Info box.** §6.1 lists them as editable, but
   ruling 5 enumerates the commit commands and `RenameGlyph`/`SetUnicodes` are not among them; they
   belong with the Glyph panel in T8.
7. **`GlyphInspector` was left as it was.** The Info box took over the selection-dependent rows per
   ruling 5; the inspector's glyph/metrics/component/anchor/layer panels are still read-only and are
   T8's subject.

Report written 2026-09-05.
