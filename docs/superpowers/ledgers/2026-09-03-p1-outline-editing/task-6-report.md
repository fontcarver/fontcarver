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

---

# Fix round 1

Review: `task-6-review.md` (Needs fixes — 8 Important, 3 spec gaps, 14 Minor).
Rulings: `task-6-fix1-dispatch.md` items 1–12; item 13 deferred and not touched.
Nine commits on `p1-t6` from `2433b2b`; not pushed.

| SHA | Subject |
| --- | --- |
| `72285f1` | fix(app): stop the Edit tab's select-all replacing the Font view's |
| `3dbfa76` | fix(app): make an arrow-key nudge visible and close its run |
| `2eeb85a` | fix(app): recover from kernel failures and honour the segment grammar |
| `237b1a9` | feat(app): proportional nudge and translated snap names |
| `1da36c2` | feat(app): insert a node at the nearest extremum or inflection |
| `74ac719` | feat(app): flash the last undo label in the status bar |
| `d0d18cc` | refactor(app): tidy the editor's small edges |
| `9a8c38c` | fix(app): live Info box numbers, complete selections and steadier tests |
| `b2ba655` | refactor(app): split the editor controller by responsibility |

## Important

**I1 — `edit.selectAll` collision.** The Edit tab's command is now
`edit.selectAllPoints`, gated on the tab kind alone so exactly one of the two
owns `Mod+A` at any moment (`app/src/views/glyph/glyphCommands.ts:157-170`); the
delete and deselect ids are likewise scoped (`:172-205`). Covered by
`app/src/shell/builtinCommands.test.ts:60-77` — both ids survive registration,
and `commandForEvent` routes `Mod+A` to the Font command in a Font tab and to
the glyph command in a glyph tab.

**I2 — shadowed title keys.** The six titles are nested under the existing
`cmd.edit` / `cmd.path` / `cmd.view` objects (`app/src/i18n/en.json`, same in
`zh-CN.json` with English placeholders). `builtinCommands.test.ts:39-49` now
resolves *every* registered command's title through the app's own i18next and
fails on any that comes back as its own key, which closes the class of bug.

**I3 — a nudge run that froze the canvas.** `NudgeCoalescer` reports the end of a
run, including from its own idle timer (`app/src/editor/gestures/nudge.ts:36-44`,
`:112-127`), and the run's optimistic preview lives with it in the new
`app/src/editor/nudgeRun.ts`. `editing` is now derived from the live gesture
rather than toggled by each caller (`editorController.ts:150-160`), so a drag
whose finish resolves late cannot clear the flag a key press just set. Tests:
`editorController.test.ts:337-395` — first press moves the outline, 500 ms later
`edit_end` has been sent, `editing` is false and a reload was requested.

**I4 — Escape while an apply was in flight.** The compensation decision happens
inside the chain, after the calls already on the wire have answered
(`app/src/editor/editSession.ts:167-187`). Test:
`gestures/drag.test.ts:299-338` holds an apply open, cancels, then releases it
and asserts `edit_begin · edit_apply · edit_end · doc.undo`.

**I5 — no error handling.** `EditSession` routes every failure to an injected
reporter and keeps the chain alive so the gesture still closes
(`editSession.ts:76-90`); the controller's gesture and command paths use
try/finally that always clear `editing` and the preview and request a reload
(`editorController.ts:352-375`, `:520-545`), and a session that failed forces the
reload even when nothing "changed". Info box commits go through
`views/glyph/commitField.ts`, which reports and reconciles. The reporter is the
shell's own (`shell/commands.ts:96-110` → toast). Tests:
`editorController.test.ts:430-470` and `views/glyph/InfoBox.test.tsx:150-173`.

**I6 — Alt on a segment.** Alt converts only a *line*; on a curve it reaches the
drag, which projects each control onto its own handle line and sends one
`MovePoints` per control (`editor/editorActions.ts:104-140`,
`gestures/drag.ts:370-380`, `:520-560`). Tests: the three cases in
`editorController.test.ts:496-540` (command emitted, no command and no empty undo
entry on a curve, plain reshape) and `gestures/drag.test.ts:129-200`.

**I7 — untranslated HUD strings.** Snap-target names arrive through the label
bundle (`editor/snapping.ts:47-66`, `views/glyph/useEditorLabels.ts:31-41`);
`snapping.test.ts:66-80` asserts the names come from the caller, never a literal.

**I8 — stale selection across tabs.** The empty selection is published
unconditionally when a different glyph arrives (`editorController.ts:222-234`),
with a test for the case that used to slip through: a freshly mounted controller
whose own selection was already empty (`editorController.test.ts:474-494`).

## Spec gaps

**9 — Ctrl+⌥+drag nudge.** The node keeps its own handles and each neighbouring
node's handle scales by the ratio its segment's chord changed by
(`gestures/drag.ts:600-660` collects them, `:395-420` scales them). Tests:
`gestures/drag.test.ts:203-280`, including that a handle already in the selection
travels with it instead of being scaled twice.

**10 — ⇧+click inserts at an extremum or inflection.** Candidates are solved from
the segment's own Bézier — the roots of each axis of `B'(t)`, plus the root of
`B'(t) × B''(t)` for a cubic (`editor/outlineGeometry.ts:196-266`); a segment
with none splits where it was clicked. Tests:
`editor/outlineGeometry.test.ts` (arch, quadratic apex, S-curve inflection,
straight line) and `editorController.test.ts:544-578`.

**11 — 1.5 s undo-label flash.** `uiStore.flashUndoLabel` holds the label for
`UNDO_FLASH_MS` (`stores/uiStore.ts:340-355`); a fresh edit is recognised from
the document event (a new entry clears the redo stack,
`stores/sessionStore.ts:118-128`) and undo/redo flash from their own commands,
which know which entry they moved (`shell/builtinCommands.ts:60-72`). Rendered by
`shell/StatusBar.tsx:73-75`. Tests: `stores/uiStore.test.ts` (fake timers, window
restart, empty label) and `stores/sessionStore.test.ts`.

## Minors

M1 `editorFrame.ts:130-136` uses `nodeOfHandle`, which walks the contour and so
finds a node across a wrap-around · M2 a handles-only move is labelled
`Move N handles` (`gestures/drag.ts:140-152`, test at `drag.test.ts:96-118`) ·
M3 the controller consults `editorStore.snapping` and a `view.toggleSnapping`
command sets it (`editorController.ts:127-133`, test at
`editorController.test.ts:582-602`) · M4 the gesture publishes its optimistic
geometry and the Info box reads it, so its numbers track the drag
(`editorController.ts:162-170`, `views/glyph/GlyphView.tsx:228-242`) · M5
master-scope guides have rows (`InfoObjectRows.tsx`, test in `InfoBox.test.tsx`) ·
M6 the marquee samples Alt at pointer-down · M7 the dead `pan()` is gone · M8 the
controller is split into `editorTypes` (89), `editorActions` (178), `nudgeRun`
(110) and `viewGestures` (146), leaving 612 lines of orchestration · M9 below ·
M10 the Info box uses a new `--shadow-popover` token with the §2.3 light and dark
values · M11 the multi-selection box includes selected anchors and components ·
M12 the anchor name follows the model · M13 the duplicate-node map is cached per
outline in a `WeakMap`.

**M9 — why the component tests were slow.** They are not: `GlyphView`'s slowest
test measures 233 ms on its own and 1 579 ms under a full-speed run. The cause is
worker oversubscription — Vitest defaults to one fork per core, each with its own
jsdom and Vite transform pipeline, and on this 20-core box that made the whole
suite *slower* (44 s) as well as flaky. The 20 s global timeout and the five 5 s
`findBy` waits are reverted; `vitest.config.ts` caps the pool at 50 %, which runs
the suite in 13-26 s and stops it tripping its own timeouts. The measurements are
recorded in the config comment. The controller tests additionally let each
gesture's promise chain finish against its own transport before the next test
installs another one (`editorController.test.ts:118-125`), which was a real
cross-test leak.

## Verification

```
pnpm --dir app check   → tsc -b, silent
pnpm --dir app test    → 50 files, 430 tests passed (12.8 s)
pnpm --dir app lint    → eslint, silent
pnpm --dir app exec prettier --check .  → all matched files use Prettier style
```

25 tests added this round (386 → 430).

### Interactive pass (agent-browser, ports 8789 / 1422, both stopped afterwards)

1. **Mod+A in a Font tab** — status bar went from `0 selected` to
   `6,253 selected`. Before the fix this did nothing.
2. **Edit menu titles** — `Select All Glyphs`, `Select All Points`,
   `Deselect Points`, `Delete Selected Points`, `Delete and Open Path`; no raw
   keys anywhere.
3. **Arrow-key nudge** — the first press moved the node 930 → 931 in the frame
   *before* any kernel answer; four Shift+presses took it to 971; 500 ms later
   the preview was handed back, the kernel agreed at 971, and there was one undo
   entry `Nudge 1 point`, flashed in the status bar.
4. **Escape mid-drag** — with applies in flight: document clean afterwards,
   `undo_label` null, the node back at (930, 1120), no live gesture, no preview.
5. **Alt-click on a curve vs a line** — on the curve a drag started and the
   document stayed clean (no empty undo entry); on the line the undo label read
   `Convert segment`.
6. **Switching glyph tabs** — opening Source Serif `o` from a two-node selection
   in DejaVu `g` left `0 points selected` and the Info box on `o`'s
   nothing-selected rows.

`docs/screenshots/p1-select-drag.png` refreshed: the Info box now reads
X 1114 / Y 1336.9 — the live dragged position — instead of the pre-gesture
numbers the review saw next to a live HUD.

## New i18n keys (10 added, 3 renamed) — for Task 13

Added: `cmd.edit.selectAllPoints` · `cmd.edit.deselectPoints` ·
`cmd.edit.deletePoints` · `cmd.edit.deletePointsBreak` ·
`cmd.view.toggleSnapping` · `editor.snap.node` · `editor.snap.handle` ·
`editor.snap.guide` · `editor.undo.moveHandles_one` ·
`editor.undo.moveHandles_other`. The flat `cmd.edit.delete`,
`cmd.edit.deleteBreak` and `cmd.edit.deselect` keys are gone — they were the
shadowed ones. `zh-CN.json` carries all of them with English placeholders;
nothing was translated here.

## Concerns

1. **The controller is 612 lines**, still above CONVENTIONS' ~400 guideline
   though well inside the dispatch's 800. Four modules came out of it and each
   new one is under 180; what is left is pointer routing plus the drag
   lifecycle, and a further cut would be a facade over the same state rather
   than a real seam. Flagging it rather than forcing it.
2. **The disabled reasons for the point commands read "No glyphs are
   selected"** — `cmd.reason.noSelection` is shared with the Font view. Correct
   in spirit, wrong noun for points; a one-key fix that belongs with T8's
   inspector strings.
3. **Suite timing is still environment-sensitive.** With the worker cap the
   suite is green and fast on an idle machine, but this box also hosts other
   worktrees' builds (load average 20-40 during this round), and under that load
   a userEvent-driven component test can still exceed 5 s. Every reported run
   above was green; two individual runs during development failed on the grid
   and `GlyphView` tests while a parallel build was saturating the CPU.
4. **M4's live Info box re-renders the panel each animation frame** during a
   drag. Measured paint stayed inside budget in the interactive pass, but it is
   new per-frame React work that a future performance pass may want to throttle.
