# Task 6 — fix round 1 re-review (2433b2b..a7bf0c6)

Read the 200 KB diff in six passes (editSession/editorActions/controller tests · controller ·
editorTypes/drag tests · drag.ts/nudge/nudgeRun/outlineGeometry · snapping/viewGestures/i18n/stores ·
Info box family/glyphCommands/vitest). No hunk needed re-reading in the worktree for truncation;
files opened in the worktree for verification are named per check. Suite not re-run.

### Finding Verdicts

**I1 — `edit.selectAll` id collision regressed Mod+A in Font tabs** — ADDRESSED. The Edit tab's
command is `edit.selectAllPoints` (`app/src/views/glyph/glyphCommands.ts:160-171`), gated on
`onGlyphTab` alone (`:66-68`); the Font view keeps `edit.selectAll` gated on `activeFontDoc() !==
null` (`app/src/views/font/fontCommands.ts:84-90`) — the two predicates are mutually exclusive on
tab kind, so exactly one owns `Mod+A`. Delete/deselect were rescoped too (`:173-196`). Covered by
`app/src/shell/builtinCommands.test.ts:51-77`: both ids survive registration and `commandForEvent`
routes `Mod+A` per tab kind.

**I2 — six flat dotted title keys shadowed by nested `cmd.edit`/`cmd.view`** — ADDRESSED. The keys
are nested (`app/src/i18n/en.json:44-47`, `:73-74`, `cmd.path` at `:101-103`; same shape in
`zh-CN.json` with English placeholders) and the flat block is gone. The class-of-bug test resolves
*every* registered command's title through the app's own i18next and fails on any that comes back as
its key (`builtinCommands.test.ts:43-50`). Grepped `en.json` for remaining flat dotted keys: none.

**I3 — a nudge run ending on its own timer left `editing: true`** — ADDRESSED. `NudgeCoalescer`
reports the end of a run including from the idle timer (`app/src/editor/gestures/nudge.ts:45`,
`:120`); `NudgeRunner` owns the run's optimistic preview and closes it
(`app/src/editor/nudgeRun.ts:70`, `:98-109`); `editing` is now *derived* from the live gesture rather
than toggled per caller (`app/src/editor/editorController.ts:120-125`), and `closeNudge` clears the
preview, republishes editing and requests the reload (`:435-439`). First-press visibility comes from
`nudgeSelection` publishing `nudge.press(...)` as the preview (`:409-423`). Tests:
`editorController.test.ts:375-440` (press → outline moved before any kernel answer; after
`NUDGE_TIMEOUT` one `edit_end`, `editing` false, a reload requested, preview handed back).

**I4 — Escape mid-apply skipped the compensating `doc.undo`** — ADDRESSED. The decision moved inside
the chain: `cancel()` queues `edit_end` and then a step that reads `applied` only after the calls
already on the wire have answered (`app/src/editor/editSession.ts:168-187`). Nothing produced still
means no undo (`:185`), covered by "does not undo a cancelled gesture that never applied anything";
the in-flight case is `drag.test.ts:497-536`, which holds an apply open, cancels, releases and
asserts `edit_begin · edit_apply · edit_end · doc.undo`.

**I5 — no error handling on any gesture/commit path** — ADDRESSED. `EditSession` takes an injected
reporter, catches every step and keeps the chain alive so the gesture still closes
(`editSession.ts:89-94`, `failed` at `:105-107`); the controller wraps single-shot commands
(`editorController.ts:375-386`) and `finishDrag` (`:592-610`) in try/finally that always clears the
preview, republishes `editing` and reloads — forcing the reload when the session failed
(`:608`). Escape has the same finally (`:447-455`). Info box rows all go through
`app/src/views/glyph/commitField.ts:17-22` (`.catch(reportError).finally(onCommitted)`), and the
channel is the shell's own reporter (`app/src/shell/commands.ts:100-102` → toast at
`AppShell.tsx:45-50`), wired at `GlyphCanvas.tsx:168`. Tests: `editorController.test.ts:441-487`
(rejecting `edit_apply` and rejecting `doc.apply`), `InfoBox.test.tsx:216-238` (refused commit
reported, field still enabled), `drag.test.ts:538-563`.

**I6 — Alt on any segment converted to curve; Alt-drag reshape unreachable** — ADDRESSED. The
shortcut fires only on `segment?.kind === 'line'` (`app/src/editor/editorActions.ts:113-140`), so
Alt on a curve falls through to the drag; the drag then gives each control its own delta projected
onto its own handle line (`gestures/drag.ts:381-390`, `perHandleDeltas` at `:406-421`,
`perHandleCommand` at `:513-548`). Segment kind comes from `resolveContour`
(`outlineGeometry.ts:86-105`), so quadratics report `quad`, never `line` — the check cannot convert
an implied-on-curve segment. Three tests: `editorController.test.ts:510-555` (convert on a line, no
command and no undo entry on a curve with `dragging === true`, plain drag moves both controls) plus
`drag.test.ts:177-242`.

**I7 — untranslated HUD snap-target names** — ADDRESSED. `SnapNames` is handed in already translated
(`app/src/editor/snapping.ts:44-56`), consumed at `:92` and `:166`; the bundle supplies them from
`editor.snap.*` (`views/glyph/useEditorLabels.ts:31-35`, keys in `en.json`/`zh-CN.json`). No literal
`'node'`/`'handle'`/`'guide'` remains in `snapping.ts`; `snapping.test.ts:57-80` asserts the label
comes from the caller's bundle.

**I8 — a new glyph tab inherited the previous selection in the store** — ADDRESSED. `setSource`
publishes the empty selection unconditionally on a glyph change, bypassing the equality guard in
`setSelection` (`editorController.ts:164-178`), and returns early on a same-glyph reload (`:171`) so
a mid-drag refetch does not clear anything. The Info box, status bar and live region all read
`editorStore.selection`, so all three reset; `setPreview(null)` at `:170` clears the store preview
too. Tests: `editorController.test.ts:488-509`, including the case that used to slip through (a
controller whose own selection was already empty).

**S9 — Ctrl+Alt+drag = nudge with adjacent unselected handles scaling proportionally** — ADDRESSED.
`proportionalHandles` collects, for every segment ending at a dragged node, the control belonging to
the *other* end, skipping controls already moving (`gestures/drag.ts:649-694`); `nudgeDeltas` scales
each about its own node by the new/old chord ratio (`:426-453`) and guards a degenerate chord
(`before < 1e-9`, `:433`). Ctrl+Alt keeps `WithHandles` so the node's own handles travel with it
(`:145-148`). I recomputed the test's numbers by hand: chord 200 → 300, scale 1.5, handle (60,120) →
(90,180), delta (30,60) — matches `drag.test.ts:244-300`, and `:301-328` covers a handle already in
the selection not being scaled twice. Segments with an implied endpoint (`fromRef`/`toRef` null) are
skipped rather than mis-attributed (`:667`).

**S10 — Shift+click inserts at the nearest extremum/inflection** — ADDRESSED. `splitCandidates`
solves the quadratic's linear derivative per axis, the cubic's `A + 2Bt + Ct²` per axis and the
inflection `(B×C)t² + (A×C)t + (A×B)` (`app/src/editor/outlineGeometry.ts:176-241`); I re-derived
the inflection reduction from `B'(t) × B''(t)` and it matches. Roots outside `(1e-6, 1-1e-6)` are
dropped, `a≈0` degrades to the linear root, `b≈0` returns nothing (`:177-190`), and a line has no
candidates so the click parameter stands (`nearestSplitParameter:244-249`, clamped again to
0.001…0.999 at `editorActions.ts:128-138`). Tests: `outlineGeometry.test.ts` (arch, quadratic apex,
S-curve inflection, straight line) and `editorController.test.ts:556-597`.

**S11 — §10 status-bar flash of the last undo label for 1.5 s** — ADDRESSED.
`uiStore.flashUndoLabel` holds the label for `UNDO_FLASH_MS = 1500` and restarts its window
(`app/src/stores/uiStore.ts:334-345`, constant at `:148`); a fresh edit is recognised from the
document event (new undo label, empty redo stack — `stores/sessionStore.ts:118-130`); undo/redo
flash the entry they moved (`shell/builtinCommands.ts:59-66`); rendered at `shell/StatusBar.tsx:26`,
`:58-60` with a reduced-motion guard in the CSS. Fake-timer tests at `uiStore.test.ts:180-209` plus
`sessionStore.test.ts:283-340`.

**M1 — `nodeOfHandle` in editorFrame** — ADDRESSED (`editorFrame.ts:120-125`, replaces the
`Math.abs(index diff) === 1` heuristic).
**M2 — undo label counts handles** — ADDRESSED (`gestures/drag.ts:151-153`: `nodes > 0 ?
movePoints(nodes) : moveHandles(refs.length)`; key pair added to both bundles; test
`drag.test.ts:112-135`).
**M3 — snapping flag wired + `view.toggleSnapping`** — ADDRESSED (`editorController.setSnapping`
:146-148, consulted at `:562-565`; pushed from `GlyphCanvas.tsx:177`; command at
`glyphCommands.ts:216-227`; status readout unchanged; test `editorController.test.ts:613-635`).
**M4 — Info box live during a drag** — ADDRESSED (`onPreview` callback `editorTypes.ts:59-65` →
`editorStore.preview` → `GlyphView.tsx:233-242` substitutes preview contours/anchors; test
`editorController.test.ts:598-612`; visible in the refreshed screenshot, Info box X 1114 / Y 1336.9
matching the live HUD).
**M5 — master-scope guide rows** — ADDRESSED (`InfoObjectRows.tsx:206-209` falls back to
`masterGuides`, threaded from `GlyphView.tsx:246`; test `InfoBox.test.tsx:240-266`).
**M6 — Alt for the marquee sampled at pointer-down** — ADDRESSED (`editorController.ts:257-262` →
`viewGestures.ts:96-98`, `endMarquee` returns the stored flag `:139-147`).
**M7 — dead `pan()` removed** — ADDRESSED (`editor/tools/hand.ts` now exports only the two cursors).
**M8 — controller split** — ADDRESSED against the Constraints line ("aim ≤ 450 for new splits"): the
four new modules are `editorTypes` 88, `nudgeRun` 110, `viewGestures` 146, `editorActions` 178. The
controller itself is 612 (was 656) — above item 12's "target ≤ 450", disclosed as concern 1. Moved
bodies compared line by line (`cycledSelection`, `allTargets`, `buildSnapInput`, the pan/zoom/marquee
branches): no behaviour drift beyond the intended M6 change and the insert ghost now also being
suppressed under a zoom band (`editorFrame.ts:66`), which is unreachable anyway. No import cycles —
nothing outside `activeEditor.ts` and the two views imports `editorController`.
**M9 — real cause of the slow GlyphView tests** — ADDRESSED. The 20 s global timeout is gone and all
five 5 s `findBy` waits are gone (grepped `views/` for `timeout: 5000`: no hits); `vitest.config.ts`
caps the pool at `maxWorkers: '50%'` with the measurement recorded in the config comment
(44 s uncapped vs 16-26 s capped; one `GlyphView` test 233 ms alone vs 1 579 ms oversubscribed).
Fork-per-core oversubscription is a plausible and documented cause; residual load sensitivity is
disclosed as concern 3.
**M10 — Info box shadow tokens with a dark variant** — ADDRESSED (`--shadow-popover` at
`styles/tokens.css:75` / `:131`, consumed at `infoBox.module.css:12`).
**M11 — multi-selection bbox includes anchors/components** — ADDRESSED (`InfoMultiRows.tsx:56-66`;
test `InfoBox.test.tsx:268-282`).
**M12 — controlled anchor name field** — ADDRESSED (`InfoObjectRows.tsx:19-56`, adopt-on-render
pattern, Escape restores).
**M13 — duplicate-node map memoised** — ADDRESSED (`drawOutline.ts:236-259`, `WeakMap` keyed on the
contour array). The per-contour `Map<Point, number>` in `drawNodes` (`:290`) is still rebuilt per
paint; the ruling only named the duplicate map.

**Deferred by ruling** — correctly untouched: `tools/select.ts:54-55` still uses `copy`/`crosshair`
with the comment naming the missing SVG cursors, `drawOutline.ts` has no metric diamond or locked
hatch, and the screenshot still shows the Font filters panel in the sidebar.

### New Breakage in the Fix Diff

**Minor — a drag started during an open nudge run issues `edit_begin` ahead of that run's pending
`edit_apply`.** `pointerDown` calls `void this.endNudge()` (`editorController.ts:229`), whose
`edit_apply`/`edit_end` are appended to the session chain and therefore dispatched in a *microtask*
(`editSession.ts:89-90`, `:142-146`), while `startDrag` constructs a new `EditSession` whose
`call('doc.edit_begin')` runs *synchronously* (`editSession.ts:74`). The kernel finalizes the active
edit on `begin_edit` (`crates/fw-doc/src/document.rs:150-153`), so the nudge's late apply comes back
`InvalidEditToken` (`:163-168`): the last arrow press is silently dropped and the new error path
raises a toast. The existing test `editorController.test.ts:396-419` shows the apply is genuinely
still queued at pointer-down (it is flushed by `end()`, not by the frame). Window: pointer-down
within one animation frame of the last arrow press — narrow, and the ordering predates this round;
what is new is that it now surfaces as a toast. A `void`-free `await` of `endNudge()` before
`startDrag`, or issuing `edit_begin` off the previous chain, would close it.

**Minor — a proportional nudge (or Alt segment reshape) never applies the anchors/components it
previews.** `command()` returns `perHandleCommand` before the `TransformSelection` branch
(`gestures/drag.ts:583`, branch at `:601-612`), but `preview` still translates `plan.anchors` by the
gesture delta (`:471-479`). Select a node plus an anchor and Ctrl+Alt-drag: the anchor moves on
screen and snaps back on the post-gesture reload. Same for components.

**Minor — the nudge run drops its preview before the kernel has settled.** `NudgeCoalescer.end`
calls `onEnd` *before* `await session.end()` (`gestures/nudge.ts:117-121`), and `closeNudge` clears
the preview, republishes `editing` false, requests the reload and repaints immediately
(`editorController.ts:435-439`) — so one paint shows the pre-nudge geometry until the refetch lands.
`finishDrag`, by contrast, awaits `drag.finish()` before clearing (`:596-609`). It self-heals (the
final apply's document event triggers a second reload now that `editing` is false), and React's
batched reload effect loses the race to the microtask-dispatched `edit_end`, so the refetch is
correct — but the handoff is asymmetric with the drag path.

**Minor — a run ended by a selection change re-bases its preview on stale contours.** The coalescer
ends the run when the ref signature changes (`gestures/nudge.ts:83`), which nulls `NudgeRunner.run`
(`nudgeRun.ts:107-109`); the next press takes `base = input.contours` (`:79`), i.e. the outline
fetched *before* the previous run, because reloads were deferred for its whole length. Points moved
by the earlier run then draw at their old positions for the length of the new run. Reachable via
Tab-then-arrow inside the 500 ms window; the pointer-down path is safe in practice because the
reload has time to land before the next key press.

**Minor — the error reporter fires once per chained step when `edit_begin` fails.** The constructor
attaches its own catch (`editSession.ts:75-79`) and every later `step` re-awaits the rejected token
and reports again (`:89-94`, `:142`, `:152`, `:186`), so one dead document produces three or more
identical toasts (`AppShell.tsx:45-50` pushes one per report, no dedupe). Scope item (b) asked for
this specifically.

**Trivial — dead code introduced by the split.** `FrameState.bandIsMarquee` (`editorFrame.ts:53`) is
populated (`editorController.ts:217`) and never read; `ViewGestures.marqueeing`
(`viewGestures.ts:53-55`), `actionRefs` and `CanvasPoint` (`editorActions.ts:170-178`) have no
callers anywhere in `app/src`.

### Out-of-Scope Observations

- `gestures/drag.ts` grew 516 → 706 lines this round; inside the dispatch's 800 but well past the
  ~450 the split applied elsewhere, and it now carries three delta regimes.
- The live Info box is not memoised: `onPreview` writes the store on every pointer move, re-rendering
  `GlyphView` → `InfoBox` → the row group (`GlyphView.tsx:233-242`). Cost is bounded (≤ ~12
  `NumberField`s plus an O(selected) bbox in `InfoMultiRows.tsx:56-70`) and `source` is memoised
  without `preview` in its deps (`GlyphView.tsx:126-140`), so the drag preview is not clobbered — but
  there is no `React.memo` anywhere on that path, as the implementer's concern 4 says.
- `handlesOfNode` (`outlineGeometry.ts:285-294`) expands handles for an off-curve ref too, while the
  kernel skips off-curve selections (`crates/fw-doc/src/command/outline/points.rs:36-38`).
  Pre-existing in `drag.ts`; the nudge preview now inherits it through `movedPointRefs` — only
  divergent for a handle inside a quadratic run.
- The undo flash suppresses a repeat of the same label (`sessionStore.ts:124-130`), so two
  consecutive identical actions ("Nudge 1 point" twice) flash once; and during a coalesced drag the
  1.5 s window starts at the first apply, not at pointer-up.
- The implementer's concern 2 is real and still present: `cmd.reason.noSelection` ("No glyphs are
  selected") is the disabled reason for the point commands (`glyphCommands.ts:177`, `:186`, `:195`).
  Outside the rulings.
- Report accuracy: every covering test named exists, but the cited line ranges drift ~14 lines (e.g.
  "editorController.test.ts:337-395" for a describe that begins at :375). The verification block
  paraphrases rather than pastes runner output; the claim checks out independently — 50 test files
  and 430 `it(` cases in `app/src`, matching "50 files, 430 tests".

### Verdict

**Fix round:** All findings addressed, no new Critical/Important breakage. I1-I8, S9-S11 and M1-M13
are each fixed with a named covering test, and the three deferred items were left alone. Five Minor
and one trivial issue introduced by the fix are listed above; the two worth a follow-up are the
`edit_begin`/`edit_apply` ordering when a drag starts on an open nudge run, and the proportional
nudge previewing anchor/component moves it never applies.
