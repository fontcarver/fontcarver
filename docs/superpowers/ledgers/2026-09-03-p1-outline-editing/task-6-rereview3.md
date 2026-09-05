# Task 6 — fix round 3 re-review (3007a90..63efbe6)

Read the 71 KB diff once, in six passes; `editSession.ts`, `editorController.ts`, `nudgeRun.ts`,
`gestures/nudge.ts`, `gestures/drag.ts`, `mockTransport.ts`, `commitField.ts`, `GlyphCanvas.tsx`,
`drawComponents.ts` and `crates/fw-doc/src/document.rs` were re-opened in the worktree because the
hunks for the controller, the session and the runner stop mid-class. Suite not re-run; three probes
were run against the real modules from a scratchpad file with its own vitest config (worktree
untouched — every file I wrote is under the scratchpad).

### Finding Verdicts

**1 — a failing `edit_begin` reported twice; `failed` false after a begin failure** — ADDRESSED. The
constructor's catch now goes through `report()` (`app/src/editor/editSession.ts:100`), which sets
`errored` before calling `onError` (`:124-129`), so `failed` (`:148`) is true after a begin failure
and every later step, re-awaiting the same rejected token, is deduped by identity (`:135`); a
*different* later failure gets exactly one `console.warn` (`:136-138`). Tests, all on the token
transport or a per-method mock: `editSession.test.ts:28` "reports a failing edit_begin exactly once
for the whole gesture" (built with `createEditingTransport({ failBegin })`, asserts
`errors.toHaveLength(1)` at `:46` and `failed === true`), `:52` "warns rather than reports a second,
unrelated failure", `:82` "reports nothing when every call succeeds". The two pre-existing
rejecting-apply tests now assert the count too (`editorController.test.ts:499`, `:511`,
`gestures/drag.test.ts:690`), which the round-2 report had claimed and they did not.

**2 — a late close cleared a newer same-selection run's preview** — ADDRESSED. `stale` now also
fires when the previous coalescer is no longer active, not only on a signature change
(`app/src/editor/nudgeRun.ts:95-97`), so a coalescer holds exactly one run and every run takes its
own generation (`:113-115`); the guard in `close` (`:165`) therefore covers the same-selection case.
Test: `editorController.test.ts:938` "does not let a late close clear the run that replaced it" —
`createEditingTransport({ hold: { on: 'doc.edit_end', until } })`, second press previews x = 220,
release, preview still 220 and `recorded.reloads` unchanged.

**3 — a new nudge run's `edit_begin` was not gated behind the closing run** — ADDRESSED for the
ruled scenario. `NudgeContext.after` (`gestures/nudge.ts:53-59`) is handed to the run's `EditSession`
(`:101`), and the runner passes the promise of the close it just started (`nudgeRun.ts:109`, `:122`).
Test: `editorController.test.ts:874` "keeps a second nudge run behind the first one it closes" (PROBE
B, Tab-then-arrow) on `createEditingTransport`, asserting the full
`edit_begin · edit_apply · edit_end · edit_begin · edit_apply · edit_end` order and no errors; my
probe P3 reproduces that same clean order against the real controller. The gate is *not* extended to
a run that starts behind a close the runner did not perform itself — see New Breakage 1, which is
the same defect on a different entry point.

**4 — a drag and the hit test based on the pre-nudge outline** — ADDRESSED. `editOutline()` indexes
`contours()`, i.e. the live preview, and rebuilds only on an identity change
(`editorController.ts:501-512`); it feeds the hit test (`:534`), `selectAll` (`:390`), `cycleNodes`
(`:395`), the segment shortcuts (`:574`), the marquee (`:298`) and the snap input (`:614`), and
`startDrag` takes `contours: this.contours()` / `anchors: this.anchors()` /
`components: this.components()` (`:593-595`). No double application: the drag computes its preview
from the context captured at `beginDrag` (`gestures/drag.ts:461`) and `setPreview` replaces the
controller's preview wholesale, so the run's delta is in the base exactly once. Test:
`editorController.test.ts:898` "keeps a drag behind the nudge run it interrupts" — the pointer goes
down at x = 210, which only hits the node because the hit test uses the nudged geometry (on the old
base it would open a marquee and the asserted second `edit_begin` would never appear).

**5 — the Info box commit bypassed `settle()`** — ADDRESSED. `commitField` closes the open run
through the mounted editor before `doc.apply` (`app/src/views/glyph/commitField.ts:26-28`), and the
production path really has a registered editor (`views/glyph/GlyphCanvas.tsx:183`). Test:
`editorController.test.ts:991` "closes the run before an Info box commit reaches the kernel" (PROBE
E2) — `registerEditor` + `commitField` on the token transport, asserting
`edit_begin · edit_apply · edit_end · apply` and no errors.

**6 — the stored gate never reset** — ADDRESSED. `settle()` stores a closing promise only while a
run is actually closing and drops it when that promise settles, guarding against a newer gate having
replaced it (`editorController.ts:441-452`); `nudge.end()` can never reject (`nudgeRun.ts:153-161`
catches), so the `finally` always runs and `void this.settle()` at `:229`/`:150` cannot leave an
unhandled rejection. Test: `editorController.test.ts:970` "drops the gate once the run it was
waiting for has closed" — after a completed run, a pointer-down issues `doc.edit_begin`
synchronously.

**7 — components moved by `TransformSelection` were never previewed** — ADDRESSED in the preview
model. Offsets and bbox carry the delta under exactly the transform regime
(`gestures/drag.ts:493-524`: `this.deltas !== null || this.plan.components.length === 0` → return
unchanged), which is the same condition under which `command()` emits `TransformSelection`
(`:628`, `:646-656`); the `TransformSelection` payload is untouched by the diff
(`points: []`, `components: this.plan.components`, `anchors: this.plan.anchors`,
`affine: [1,0,0,1,step.x,step.y]`). It reaches the Info box through `EditorPreview`
(`editor/editorTypes.ts:23-27`) → store → `GlyphView.tsx:244` → `InfoObjectRows.tsx:117`. Tests:
`gestures/drag.test.ts:416` "carries a selected component, which the command moves too" (transform
offset `[70, 510]`, bbox `[70,510,170,610]`, a `TransformSelection` sent) and `:443` "leaves an
unselected component where it is". The canvas half of the ruling is not delivered — New Breakage 2.

**Cleanup — unreachable signature branch removed, signature built in one place** — ADDRESSED.
`NudgeCoalescer` no longer has a `signature` field or an end-on-change branch (`gestures/nudge.ts`
has no `signature` symbol at all); the string is built only in `nudgeRun.ts:89-91`. Contract test
rewritten: `gestures/nudge.test.ts` "holds exactly one run, whatever it is handed" now asserts one
`edit_begin` and one `edit_end` when the ref set changes under a single coalescer.

### New Breakage in the Fix Diff

**Minor — the new gate is only real for a close the runner performs itself; a nudge press one round
trip after any other close still overtakes it.** In `press`, the gate is `this.closing = this.end()`
(`nudgeRun.ts:109`), and `end()` returns immediately when the coalescer is already inactive
(`:155-157`), so a run closed by `settle()`/`endNudge()`/`dispose()` — or a drag closing, which
never registers in `editorController.closing` at all (`editorController.ts:637-655` sets no gate) —
leaves the replacement with a resolved gate; `nudgeSelection` (`:405-424`) never consults the
controller's own `this.closing` either. Probe P1 (real controller, `createEditingTransport`, arrow
→ `void endNudge()` → arrow in the same tick):
`edit_begin#101 · edit_begin#102 · edit_apply#101 · edit_apply#102 · edit_end#101 · edit_end#102`
with one `ApiCallError: InvalidEditToken` reported — the first run's only apply is dropped, and
`carried` (`nudgeRun.ts:100-107`) then bases the second run on a move the kernel never made, so the
preview stands 10 units ahead of the document until the post-run reload. Probe P2 (pointer-up then
an arrow in the same tick) gives the same rejected apply for drag → nudge. Probe P3 (the ruled
Tab-then-arrow path) is clean, so this is the residual half of finding 3, not a regression of it;
the window is one kernel round trip after a tool click, a Delete, an Info box commit or a pointer-up
that closed a gesture. The 500 ms idle-timer path is benign — its apply has already been flushed, so
the overtaking `edit_begin` only finalises an edit that `end_edit` then no-ops. Fix is small: thread
`after: this.closing` into `NudgePress`, and set `this.closing` in `finishDrag` as `settle()` does.

**Minor — `EditorFrame.components` has no consumer, so a dragged component moves in the Info box but
nowhere on the canvas.** `buildFrame` now publishes `components: state.preview?.components ?? null`
(`editor/editorFrame.ts:168`), but `GlyphCanvas`'s draw reads only `editing.contours` and
`editing.anchors` (`views/glyph/GlyphCanvas.tsx:133-145`); component geometry comes from
`scene.components`, which `GlyphView` builds from the *fetched* outline
(`views/glyph/GlyphView.tsx:96`) and `drawComponents` uses for both the filled path and the origin
markers and labels (`views/glyph/drawComponents.ts:87-119`). The report's concern 1 claims the
preview moves "a component's origin marker, its bounds and the Info box numbers but not the filled
ghost" — the marker and the bounds do not move either; only the Info box does.

**Trivial — the component half of finding 4 was left on the stale geometry.** `hitInput()` takes
`anchors: this.anchors()` but `components: this.source?.outline.components ?? []`
(`editorController.ts:532-540`), and `frame()` passes the source components as the ghost bounds
(`:212`), so while a component preview stands its hit target and bbox lag behind its own preview.
`componentGhosts` bails out during a drag (`editorFrame.ts:70`), which is what keeps this cosmetic.

**Trivial — `editOutline()` rebuilds the whole index twice per pointer move during a drag.**
`contours()` returns a freshly allocated array on every `translatePoints`, so the identity check at
`editorController.ts:503` misses on every move, and both the snap callback (`:614`) and `frame()`
(`:210`) call it — two `buildEditOutline` passes (a `resolveContour` plus a `Map` per contour,
`outlineGeometry.ts:62-100`) per pointer move where the previous code built the index once per
`setSource`. Fine at the sizes tested, but it is new work on the 60 fps path.

**Trivial — the token transport's `doc.undo` does not finalise the active edit**
(`app/src/test/mockTransport.ts:167-168`), where the kernel's `undo` calls `finalize_active_edit`
(`crates/fw-doc/src/document.rs:194`). No current test depends on it; a future cancel-ordering test
would not see a stale token.

Checks run for the scope items: (a) gate set only inside `if (this.nudge.active)` and cleared in a
`finally` with an identity guard (`editorController.ts:441-452`); `nudge.end()` cannot reject
(`nudgeRun.ts:153-161`), and a rejected gate still begins — both arms of `gate.then` call
`edit_begin` (`editSession.ts:85-91`); a second `settle()` during a close returns the stored promise
because `nudge.active` goes false synchronously inside `NudgeCoalescer.end` (`gestures/nudge.ts:116-122`),
so gates cannot chain unboundedly. (b) `generation` advances only where a coalescer is constructed
(`nudgeRun.ts:113-115`), which is every new run and only a new run, so a run's own `close` always
matches and hands over; a dropped late close leaves `run`/`carried` alone, which is what the newer
run needs. (c) transport fidelity verified line by line against the kernel — `begin_edit` finalises
and mints a new token (`document.rs:150-160`), `apply_in_edit` rejects a missing or mismatched token
with `InvalidEditToken` (`:163-169`), `end_edit` on a stale token is a silent no-op (`:182-190`),
`apply` finalises (`:140-143`) — and `createEditingTransport` (`mockTransport.ts:103-168`) is the
transport actually installed by all six new controller tests and by `editSession.test.ts:28`.
(d) covered under finding 4. (e) `commitField` still returns `void` and still calls
`.catch(reportError).finally(onCommitted)` (`commitField.ts:26-29`), so the reload and the field's
error path are unchanged; it cannot deadlock, because `settle()` resolves on the run's own
`edit_end` and nothing in that chain waits on the commit, and a missing editor yields
`Promise.resolve(undefined)`. (f) covered under finding 7.

### Out-of-Scope Observations

- A run that closes on its idle timer clears `carried` (`nudgeRun.ts:166-167`) while the reload it
  triggers is still in flight, so a press landing in that window re-bases on the pre-nudge outline
  and the preview drops back by the previous run's delta for the length of the new run. Identical
  before and after this round.
- `closeNudge` (`editorController.ts:458-463`) does not check for a live drag, so a nudge close that
  lands mid-drag clears the drag's preview for one frame and requests a reload; introduced in round
  2, not this one, and self-healing on the next pointer move.
- Report accuracy: every citation I checked resolves (`editSession.ts:100`, `:148`,
  `nudgeRun.ts:95-105`, `:122`, `:163-170`, `editorController.ts:441-452`, `:501-512`, `:593`,
  `commitField.ts:26-28`, `drag.ts:493-524`, and the six test line numbers 874/898/920/938/970/991),
  a clear improvement on round 2; the only overstatement is the canvas claim in finding 7 and
  concern 1, above. Concern 4 ("a drag that never sees its pointer-up still leaks its session") is
  correct and is the drag→drag half of New Breakage 1: `pointerDown` overwrites a live drag whose
  `edit_end` never runs, and that path has no gate either.
- The verification block is pasted verbatim this round; I did not re-run the suite. Independent
  count: `grep -c "it("` over `app/src` gives 449 cases across 51 test files, matching the pasted
  summary.

### Verdict

**Fix round:** All findings addressed, no new Critical/Important breakage — findings 1-7 and the
signature cleanup each land with a named covering test on the token-checking transport, and the
kernel token rules the transport models were verified against `document.rs`. Four items are open at
Minor or below, of which one is worth doing now: the nudge gate covers only the close the runner
itself starts, so an arrow press one round trip after a settle-driven close or a pointer-up still
raises `InvalidEditToken` and drops that run's last apply (probes P1/P2).
