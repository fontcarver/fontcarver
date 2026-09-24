# Task 6 review — editor core (selection, hit testing, edit sessions, tools, Info box, undo wiring)

Reviewed `ab056b9..2433b2b` from the diff package in four passes: (1) `editor/` core —
selection, outlineGeometry, hitTest, snapping, editSession, pointCommands, hud; (2) controller,
frame, gestures and their tests; (3) the React surface — GlyphCanvas, GlyphView, Info box family,
ToolPalette, hooks, stores, StatusBar; (4) rendering, tokens, i18n, fixtures, tests, licensing.
Screenshots viewed. Checks outside the diff (each named with its risk) are listed inline.

### Spec Compliance

❌ Issues found.

- ✅ Hit radii and precedence are exactly §4.2 (`app/src/editor/hitTest.ts:31`, precedence at
  `:186`), measured in CSS pixels so DPR does not change them, with a DPR test
  (`app/src/editor/hitTest.test.ts:79`).
- ✅ Gesture protocol per dispatch ruling 3: one `edit_begin` on pointer-down, one `edit_apply` per
  animation frame carrying the step since the last apply, one `edit_end`
  (`app/src/editor/editSession.ts:83-131`, `app/src/editor/gestures/drag.ts:437-495`); a click that
  never moves sends nothing and records no undo entry (`drag.test.ts:206`, and confirmed against the
  kernel: `crates/fw-doc/src/document.rs:150-190` only creates an entry when a command is applied).
  Relative-delta deviation is documented and correct.
- ✅ Nudge coalescing: one token per run, 500 ms idle window, new entry when the selection changes
  (`app/src/editor/gestures/nudge.ts:88-123`, `nudge.test.ts`). **But** the run's end is not wired
  back to the view — see Important 3.
- ✅ Snapping distance/order/Ctrl suspension per §4.2 (`app/src/editor/snapping.ts:118-190`,
  suspension at `gestures/drag.ts:349`); grid target deliberately absent (deviation documented).
- ✅ Tool strip renders all twelve §5 tools with key, icon, label key and cursor
  (`app/src/editor/tools/registry.ts:57-190`), tap/hold borrow at 200 ms
  (`app/src/views/glyph/useToolShortcuts.ts:38-60`); Select/Hand/Zoom live.
- ✅ Info box rows and commit commands per §6.1 and ruling 5 (`InfoPointRows.tsx`,
  `InfoMultiRows.tsx`, `InfoObjectRows.tsx`, `InfoGlyphRows.tsx`), expressions/Enter/Esc per §7
  (`app/src/components/NumberField.tsx:47-104`, `app/src/editor/fieldValue.ts:13`).
- ✅ §4.1 node grammar sizes and the selection layer (`app/src/views/glyph/drawOutline.ts:48-56`,
  `:120-137`), §4.1 item 14 ghosts/halo/alignment lines (`drawEditing.ts:47-124`), §6.2 HUD pill
  (`drawHud.ts:28-56`), drawing order preserved by extending `renderGlyph.ts:118-127` (ruling 9).
- ✅ §2.1 `--cv-*` tokens added with light **and** dark values (`app/src/styles/tokens.css:75-97`,
  `:129-151`); no colour literals outside `tokens.css`/`glyphTheme.ts` (checked: `grep -- --color-glyph`
  returns nothing, CSS modules use tokens only).
- ✅ Accessibility: tool buttons carry `aria-label`/`aria-pressed` (`ToolPalette.tsx:52-53`), the
  canvas has an offscreen live region (`GlyphView.tsx:213-215`), Tab still leaves the canvas when
  nothing is selected (`GlyphCanvas.tsx:352`).
- ✅ Transport-only kernel access (every call goes through `call()` from `../api`); no Rust changes;
  THIRD_PARTY §3.2 re-surveyed for lucide-react (ISC), which the CONVENTIONS licence rule allows.
- ❌ **Six new commands are mis-keyed in i18n and one collides with a P0 command id** —
  `app/src/views/glyph/glyphCommands.ts:157-215` vs `app/src/i18n/en.json:96-101` and
  `app/src/views/font/fontCommands.ts:84`. See Important 1 and 2.
- ❌ **§4.3 "drag segment … ⌥ keeps handle angles" is unreachable**: Alt on any segment is
  intercepted as convert-to-curve (`app/src/editor/editorController.ts:544-556`), including on
  segments that are already curves.
- ❌ **Untranslated user-visible strings** in the HUD snap readout (`app/src/editor/snapping.ts:79`
  `'node'`/`'handle'`, `:153` `'guide'`) — dispatch ruling 7 requires `t()` for every new
  user-visible string.
- ❌ §4.3 "Ctrl+⌥+drag [a node] = nudge (adjacent unselected handles scale proportionally)" is not
  implemented; `planDrag` ignores `mod` for node targets (`gestures/drag.ts:132-145`). The brief does
  not list it, the design spec does.
- ❌ §4.3 "⇧+click segment = insert node at nearest extremum/inflection" inserts at the hit
  parameter instead (`editorController.ts:557-570`). Undocumented deviation.
- Extra (justified, but unrequested): the P0 corner readout `GlyphHud.tsx` was deleted. §6.1 and §3
  do cover the same values, and the tests were updated, so this reads as a deliberate scope call
  rather than a loss — flagging it only because nothing asked for it.

⚠️ Cannot verify from this diff:
- The repaint measurements (mean 2.15 ms / p95 4.3 ms on a 329-point glyph) and the three green test
  runs. The instrumentation exists and is dev-only (`app/src/canvas/paintStats.ts:63-73`,
  `GlyphCanvas.tsx:95-113`); the numbers themselves are the implementer's.
- §3's Edit-tab sidebar (Layers list + compact glyph list) is absent from the diff and from the
  screenshots (the sidebar still shows the Font filters panel). Presumably T8; not claimed here.
- §10's "status bar flashes the last undo label for 1.5 s" is not implemented anywhere I can see;
  the Edit-menu label path is P0's (`shell/builtinCommands.ts:165`) and does carry the new labels.

### Screenshots

- `p1-select-drag.png` — two corner nodes selected and dragged, HUD pill `Δx 184 Δy 216.9 ⟂ 184`
  offset down-right of the pointer, snap halo on the vertical line, Info box "SELECTION" with
  X/Y/W/H, 9-point origin, proportional lock and "2 of 41 points". Matches §6.1, §6.2 and §4.1
  item 10/14; tool strip shows Select active with the accent bar (§3).
- `p1-select-handles.png` — Alt-drag of an off-curve handle; HUD `∠ 119.9° ℓ 123.7` per §6.2, Info
  box on its handle rows (ΔX/ΔY/Length/Angle) per §6.1. It also shows the Info-box lag: the panel
  reads Length 114.2 while the HUD reads 123.7 for the same handle at the same instant (Minor 4).
- `p1-select-infobox.png` — marquee result: 17 points, bbox 303/98/627/951, origin picker, "17 of
  41 points", status bar "17 points selected · snap on" (§6.1, §3). Selected nodes render as filled
  markers with the inner ring; at 5 px the selection blue (#2B5FA8) is hard to tell from the corner
  blue (#3B6FB6), but both are the spec's own tokens.
- `p1-select-cubic.png` — Source Serif `o`: smooth green circles, grey handle lines and 3.5 px
  off-curve dots per §4.1/§4.3, Ctrl+Alt mirror visible on the opposite handle, HUD `∠ 55.8° ℓ 125.5`.
  Same Info-box lag (panel shows the pre-drag 166/90).

### Strengths

- The module layout follows the dispatch's file structure exactly, and the seams are real: the
  controller never touches the DOM, the frame assembly is a pure function of gesture state
  (`editorFrame.ts:174-190`), and every i18n string enters through one bundle
  (`useEditorLabels.ts:17-48`), which is why the editor is testable with plain objects.
- `outlineGeometry.ts` maps resolved segments back onto stored indexes by object identity
  (`:87-95`), which is exactly right for quadratic contours with implied on-curve points — the hard
  part of editing a TrueType outline — and it handles contour wrap-around in
  `adjacentIndexes`/`nodeOfHandle`/`oppositeHandle`.
- Tests assert real protocol behaviour through the mocked transport, not implementation details:
  exact method sequences and command payloads (`drag.test.ts:171-205`), the no-move case, Escape →
  `edit_end` + `doc.undo`, per-frame coalescing with an injected scheduler, and the deferral
  behaviour of the data hooks (`useLayerOutline.test.tsx:81-109`).
- The four defects found interactively are genuine and the fixes are the right ones — in particular
  keeping the previous outline on screen across a reload (`useLayerOutline.ts:106-108`,
  `useGlyphDetail.ts:139-141`) instead of blanking it, which is what was tearing down live gestures.
- Design-token work is careful: the `--cv-*` palette matches §2.1 value for value in both themes,
  the old `--color-glyph-*` names are fully retired, and node/handle sizes are constant screen pixels.

### Issues

#### Critical (Must Fix)

None.

#### Important (Should Fix)

1. **`edit.selectAll` collides with the Font view's command id and silently replaces it.**
   `app/src/views/glyph/glyphCommands.ts:157` registers `id: 'edit.selectAll'`;
   `app/src/views/font/fontCommands.ts:84` already uses that id for "Select All Glyphs" with the same
   `Mod+A` shortcut. The registry is a `Map` keyed by id (`app/src/shell/commands.ts:109`) and
   `registerFontViewCommands()` runs before `registerGlyphViewCommands()`
   (`app/src/shell/builtinCommands.ts:66-67`), so the glyph command wins. Shortcut dispatch scans
   `listCommands()` and skips a command whose `when()` is false
   (`app/src/shell/shortcuts.ts:167-177`), so in a Font tab `Mod+A` now does nothing and the Edit
   menu shows Select All disabled with "The active tab is not a Glyph tab". That is a regression in
   P0 behaviour caused by this task. Fix: give the glyph command its own id
   (`edit.selectAllPoints`).

2. **Four of the six new command titles do not resolve; a fifth resolves to the wrong string.**
   The titles are `cmd.edit.deselect`, `cmd.edit.delete`, `cmd.edit.deleteBreak`,
   `cmd.path.toggleSmooth`, `cmd.view.toggleInfoBox`, `cmd.edit.selectAll`
   (`glyphCommands.ts:158-211`), but the strings were added as *flat dotted keys* under `cmd`
   (`app/src/i18n/en.json:96-101`, same in `zh-CN.json`) while `cmd.edit` and `cmd.view` already
   exist as nested objects. i18next resolves the shortest path first, so the flat keys are shadowed.
   Verified by running the app's own i18next against the committed `en.json`:
   `cmd.edit.selectAll → "Select All Glyphs"` (the P0 string, not "Select All Points"),
   `cmd.edit.deselect / cmd.edit.delete / cmd.edit.deleteBreak / cmd.view.toggleInfoBox` → the raw
   key text, which is what the Edit/View menus, the command palette and the shortcuts help will
   display. Only `cmd.path.toggleSmooth` works, and only because `cmd.path` does not already exist.
   Fix: nest the six keys under `cmd.edit` / `cmd.path` / `cmd.view` with names that do not clash.

3. **A nudge run that ends on its own timer leaves the editor in `editing: true`, so the canvas
   never shows the nudge.** `nudgeSelection` sets `editing` true on every arrow press
   (`editorController.ts:449`), and only `endNudge()` clears it (`:458-462`) — but the coalescer
   closes the run from its own 500 ms timer without telling the controller
   (`app/src/editor/gestures/nudge.ts:118-124`), after which `this.nudge.active` is false and
   `endNudge()` early-returns forever. `editing` is written nowhere else
   (`onEditingChange(false)` exists only at `editorController.ts:461`, `:472`, `:652`). While it is
   true, both data hooks defer every document event (`useLayerOutline.ts:96-101`,
   `useGlyphDetail.ts:118-124`), and there is no optimistic preview for nudges — the controller only
   queues the command. Net effect: press an arrow key and nothing moves on screen (and no later
   kernel change, including undo/redo, repaints) until the next drag or click-on-node gesture
   finishes and flips `editing` back to false. No test covers the controller-level nudge lifecycle.
   Fix: give `NudgeCoalescer` an `onEnd` callback (or have the controller own the timer) so the
   timeout path runs `onEditingChange(false)` + `onReload()`.

4. **Escape during a drag can fail to cancel, leaving the partial move and its undo entry.**
   `EditSession.cancel()` decides whether to compensate by reading `this.applied`
   (`app/src/editor/editSession.ts:144`), but `applied` is only set *after* the apply round trip
   resolves (`:114`). If Escape arrives while an `edit_apply` is in flight — the normal case when the
   pointer is still moving, since applies are queued every animation frame — `wasDirty` is false, no
   `doc.undo` is issued, and the gesture stays in the document (and the deferred reload then paints
   it back). The test avoids this by flushing first (`drag.test.ts:289-297`). Fix: set `applied`
   synchronously in `flush()` when a command is produced, or `await this.chain` before deciding.

5. **No error handling anywhere on the interaction path.** Every kernel call in a gesture is fired
   with `void` and no `catch`: `void this.finishDrag()` (`editorController.ts:337`),
   `void drag.cancel().then(...)` (`:471`), `void commit(...).then(...)` (`:546`, `:559`, and every
   Info box row, e.g. `InfoPointRows.tsx:37`), `void this.flush()` (`editSession.ts:97`). A stale
   token or any kernel error therefore produces an unhandled rejection with no toast — the house
   style routes command failures to the error reporter (`app/src/shell/commands.ts:154-159`) — and
   `finishDrag` never reaches `onEditingChange(false)` / `preview = null`, so the canvas keeps
   painting the dead preview and all reloads stay deferred (same stuck state as issue 3). Fix: wrap
   the gesture lifecycle in try/finally that always clears `editing` and the preview, and report the
   error through the existing reporter.

6. **Alt on a segment always converts to a curve, even when it already is one.**
   `handleSegmentShortcut` (`editorController.ts:544-556`) fires on pointer-down for any segment hit
   with Alt held, before a drag can start. Consequences: (a) §4.3's "drag segment = reshape; ⌥ keeps
   handle angles" can never happen — `gestures/drag.ts` has no keep-angle branch for
   `plan.kind === 'segment'` either; (b) Alt-clicking an existing curve records an empty undo entry
   and dirties the document — the kernel no-ops the conversion
   (`crates/fw-doc/src/command/outline/segments.rs:135-139`) but `apply_command` pushes the undo
   entry and bumps the dirty flag unconditionally (`crates/fw-doc/src/document.rs:339-357`). Fix:
   only take the shortcut when the hit segment is a line (the segment kind is already in
   `EditOutline.segments`), and let Alt reach the drag otherwise.

7. **Untranslated user-visible strings in the HUD (plan-mandated).** The snap readout is fed the
   literals `'node'` / `'handle'` (`app/src/editor/snapping.ts:79`) and the fallback `'guide'`
   (`:153`), and those reach the HUD text (`hud.ts:80`, `:87`). Dispatch ruling 7 and §12 require
   `t()` for every new user-visible string; the label bundle to hang them on already exists
   (`useEditorLabels.ts`, which already injects the metric names the same way).

8. **A newly mounted glyph tab inherits the previous tab's selection in the store.**
   `setSource` clears the selection through `setSelection`
   (`editorController.ts:181-190`), which early-returns when the two selections are equal — and a
   fresh controller's selection is already empty, so `onSelectionChange` never fires and
   `editorStore.selection` keeps whatever the previous tab left. Tabs are keyed and unmounted
   (`app/src/shell/TabContent.tsx:15`), so this is the ordinary "switch tabs" path. The canvas draws
   nothing selected, but the Info box, the status bar and the live region all read the store: the
   panel renders multi-selection rows computed from the *new* glyph's contours at the *old* glyph's
   indexes, and committing a field there would transform points the user never selected. Fix: push
   the empty selection to the callbacks unconditionally when the glyph changes.

#### Minor (Nice to Have)

1. `editorFrame.ts:136-143` finds a handle's owning node with
   `Math.abs(candidate.ref.point - handle.point) === 1`, duplicating — and weakening —
   `nodeOfHandle()` (`outlineGeometry.ts:243-252`), which already handles contour wrap-around. A
   handle whose node is across the wrap (or inside a quadratic run) silently falls back to the
   Δx/Δy HUD instead of the ∠/ℓ readout §6.2 asks for. Use `nodeOfHandle`.
2. `gestures/drag.ts:143` labels a move by the *node* count, so dragging a handles-only selection
   produces the undo label "Move 0 points" (and a mixed marquee selection under-counts). Count
   `refs.length` when no on-curve node is in the set.
3. `editorStore.snapping` is read by the status bar (`app/src/shell/StatusBar.tsx:27`, `:87`) but
   `setSnapping` has no caller and the controller never consults the flag (`editorController.ts:592`
   snaps unconditionally unless `mod` is held), so the readout is a constant "snap on". Either wire
   the flag or drop it until T7.
4. The Info box reads the fetched outline while reloads are deferred, so during a drag it shows
   pre-gesture numbers next to a live HUD showing different ones — visible in two of the four
   screenshots. Disclosed in the report; feeding the preview into the box would close it.
5. `InfoObjectRows.tsx` (guide branch) returns `undefined` for a master-scope guide, so selecting a
   master guide renders an empty Info box even though the hit test can select one
   (`hitTest.ts:151-160` passes the scope through).
6. The Alt-marquee reads the modifier at pointer-*up* (`editorController.ts:328`) while the additive
   flag is captured at pointer-down (`:266`); the two halves of one gesture should sample the same
   moment.
7. `app/src/editor/tools/hand.ts:17` exports `pan()`, which nothing calls (the controller pans the
   viewport directly at `editorController.ts:283`). Dead code.
8. Files created above CONVENTIONS' ~400-line guideline for TS/React (all within the dispatch's 800):
   `editorController.ts` 656, `gestures/drag.ts` 516, `GlyphCanvas.tsx` 418. The controller in
   particular now carries pointer grammar, tool routing, the keyboard command surface, the nudge
   lifecycle and snap-input assembly; the keyboard/command methods would split out cleanly.
9. `app/vitest.config.ts:12` raises the global timeout to 20 s *and* `GlyphView.test.tsx` adds
   explicit 5 s waits at five call sites. Two sequential mocked round trips should not take seconds
   in jsdom; the timeouts hide whatever actually is slow rather than fixing it.
10. `infoBox.module.css:12` uses `box-shadow: 0 8px 24px rgb(0 0 0 / 12%)` — §2.3 specifies .18
    light / .45 dark, and this has no dark variant.
11. `InfoMultiRows.tsx:63-68` computes the bounding box from selected *points* only, so a selection
    that includes anchors or components reports a box that excludes them while the transform moves
    them.
12. `InfoObjectRows.tsx` anchor name uses `defaultValue`, so the field goes stale if the anchor is
    renamed elsewhere.
13. `drawOutline.ts:196-217` rebuilds the duplicate-node map and (in `drawNodes`) a per-contour
    `Map<Point, number>` on every paint. Fine at the measured sizes, but it is per-frame work that
    only changes when the outline changes.
14. Custom SVG cursors (§2.4) stand in as `copy`/`crosshair` (`tools/select.ts:48-53`) and the §4.1
    metric-diamond / locked-component markers are absent — both disclosed in the report and
    reasonable to defer, listed here so they are not lost.

### Assessment

**Task quality:** Needs fixes

**Reasoning:** The architecture, the gesture protocol, the hit-test/snapping numbers and the
rendering work are all solid and well-tested, but three of the wired-up surfaces are broken in ways
a user meets immediately — `Mod+A` in the Font view is regressed by an id collision, four new menu
entries display raw i18n keys, and an arrow-key nudge freezes canvas refreshes until the next drag —
alongside an Escape-cancel race and a complete absence of transport error handling in the
interaction path.
